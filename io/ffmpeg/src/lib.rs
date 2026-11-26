//! (a) The ffmpeg CLI: the baseline every other backend is measured against. Decoding keeps one rawvideo pipe open
//! while requests move forward and restarts it with an accurate seek otherwise; frame times come from ffprobe.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use raea_io_codec::{
    Audio, Backend, EncodeOpts, EncodeStats, Frame, Info, R, VideoDecoder, VideoEncoder, as_of,
};

pub struct Ffmpeg;
pub struct Nvenc;

/// The ffmpeg or ffprobe to run: the PATH's, else raea's own (raea_io_assets::ffmpeg_tool). Opening a video (`fetch`)
/// fetches raea's own on Windows the first time one needs it; sound, encoding and asking what FFmpeg can do (says)
/// use FFmpeg when it's there and never fetch it (a silent clip's sound is tried on every backend).
fn cli(name: &str, fetch: bool) -> R<Command> {
    raea_io_assets::ffmpeg_tool(name, fetch).map(Command::new)
}

/// A file named as a video: only these fetch FFmpeg when it isn't there (an audio file or an image never does)
fn named_video(path: &Path) -> bool {
    const VIDEO: &[&str] = &["mp4", "mov", "m4v", "mkv", "webm", "avi", "mts", "m2ts", "ts", "3gp", "wmv", "flv", "mpg", "mpeg", "mxf", "ogv"];
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    VIDEO.iter().any(|v| *v == ext)
}

/// Every decoded frame, once: -fps_mode from FFmpeg 5.1 on, -vsync before it (FFmpeg 8 took -vsync out, and the decoder
/// read nothing: #113).
pub fn passthrough() -> [&'static str; 2] {
    static FLAG: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    let flag = FLAG.get_or_init(|| {
        let head = says("-version");
        let version = head.split("version ").nth(1).and_then(|v| {
            let mut parts = v.trim_start_matches('n').split(|c: char| !c.is_ascii_digit());
            Some((parts.next()?.parse::<u32>().ok()?, parts.next()?.parse::<u32>().ok()?))
        });
        if version.is_some_and(|v| v < (5, 1)) { "-vsync" } else { "-fps_mode" }
    });
    [flag, "passthrough"]
}

/// What `ffmpeg -hide_banner OPTION` prints (nothing without ffmpeg).
fn says(option: &str) -> String {
    let out = cli("ffmpeg", false).ok().and_then(|mut c| c.args(["-hide_banner", option]).output().ok()).map(|o| o.stdout).unwrap_or_default();
    String::from_utf8_lossy(&out).into_owned()
}

fn encoders() -> &'static str {
    static ENCODERS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ENCODERS.get_or_init(|| says("-encoders"))
}

fn has_encoder(list: &str, name: &str) -> bool {
    list.split_whitespace().any(|w| w == name)
}

fn has_nvidia_device() -> bool {
    Path::new("/dev/nvidia0").exists() || Path::new("/dev/nvidiactl").exists()
}

/// Probe whether `h264_nvenc` actually initializes on this machine (checks libnvidia-encode + GPU capability,
/// not just FFmpeg compile flags).
fn probe_nvenc() -> bool {
    let Ok(mut cmd) = cli("ffmpeg", false) else {
        return false;
    };
    let Ok(mut child) = cmd
        .args([
            "-v",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "nv12",
            "-s",
            "256x256",
            "-framerate",
            "30",
            "-i",
            "-",
            "-frames:v",
            "1",
            "-c:v",
            "h264_nvenc",
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let ok = if let Some(mut stdin) = child.stdin.take() {
        let mut nv12 = vec![16u8; 256 * 256 * 3 / 2];
        nv12[256 * 256..].fill(128);
        stdin.write_all(&nv12).is_ok()
    } else {
        false
    };
    child.wait().is_ok_and(|s| s.success() && ok)
}

pub(crate) fn detect_nvenc(is_linux: bool, has_device: bool, enc_list: &str, probe: impl FnOnce() -> bool) -> bool {
    is_linux && has_device && has_encoder(enc_list, "h264_nvenc") && probe()
}

/// Whether Linux NVENC H.264 hardware encoding (`h264_nvenc` via FFmpeg) is available at runtime.
/// `RAEA_NVENC=0` disables auto-detection.
pub fn nvenc_available() -> bool {
    if std::env::var("RAEA_NVENC").as_deref() == Ok("0") {
        return false;
    }
    static AVAIL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAIL.get_or_init(|| detect_nvenc(cfg!(target_os = "linux"), has_nvidia_device(), encoders(), probe_nvenc))
}

/// The H.264 encoder this machine's ffmpeg has, as its arguments, for a clip a person plays (in a browser or on a
/// phone): libx264 where ffmpeg was built with it (Homebrew's, Linux distributions'), else the system's own (Windows'
/// Media Foundation, h264_mf, in the LGPL build raea ships there; macOS's VideoToolbox; Linux NVENC) at 8 Mb/s; None without one.
pub fn h264() -> Option<Vec<&'static str>> {
    let have = encoders();
    let enc = ["libx264", "h264_mf", "h264_videotoolbox"]
        .into_iter()
        .find(|c| has_encoder(have, c))
        .or_else(|| nvenc_available().then_some("h264_nvenc"))?;
    let rate: &[&'static str] = if enc == "libx264" { &[] } else { &["-b:v", "8M"] };
    Some([&["-c:v", enc][..], rate, &["-pix_fmt", "yuv420p"]].concat())
}

/// The facts, the presentation times, whether the colour matrix is untagged, and the decoder a VP8 or VP9 stream with
/// alpha needs (libvpx: ffmpeg's own VP9 decoder drops the alpha WebM keeps beside the picture, #113).
fn probe(path: &Path) -> R<(Info, Vec<f64>, bool, Option<&'static str>)> {
    let out = cli("ffprobe", named_video(path))?
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,width,height,r_frame_rate,color_space:stream_tags=alpha_mode:stream_side_data=rotation:format=duration",
            "-show_entries",
            "packet=pts_time",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .map_err(|e| format!("ffprobe: {e}"))?;
    let j: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("ffprobe json: {e}"))?;
    let s = &j["streams"][0];
    let (mut w, mut h) = (
        s["width"].as_u64().unwrap_or(0) as u32,
        s["height"].as_u64().unwrap_or(0) as u32,
    );
    let rot = s["side_data_list"]
        .as_array()
        .and_then(|l| l.iter().find_map(|d| d["rotation"].as_i64()))
        .unwrap_or(0);
    if rot.abs() == 90 || rot.abs() == 270 {
        std::mem::swap(&mut w, &mut h);
    }
    let fps = s["r_frame_rate"]
        .as_str()
        .and_then(|r| r.split_once('/'))
        .map(|(n, d)| n.parse::<f64>().unwrap_or(30.0) / d.parse::<f64>().unwrap_or(1.0))
        .unwrap_or(30.0);
    let mut pts: Vec<f64> = j["packets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["pts_time"].as_str().and_then(|t| t.parse().ok()))
        .collect();
    pts.sort_by(f64::total_cmp);
    let duration = j["format"]["duration"]
        .as_str()
        .and_then(|d| d.parse().ok())
        .unwrap_or(0.0);
    let untagged = !matches!(s["color_space"].as_str(), Some(c) if c != "unknown");
    // the tag in either case: ffmpeg's muxer writes ALPHA_MODE when it copies a stream that has alpha, and a file
    // remuxed with -c copy drew opaque (#224)
    let tagged = s["tags"].as_object().and_then(|t| t.iter().find_map(|(k, v)| k.eq_ignore_ascii_case("alpha_mode").then(|| v.as_str()).flatten()));
    let alpha = match (s["codec_name"].as_str(), tagged) {
        (Some("vp8"), Some("1")) => Some("libvpx"),
        (Some("vp9"), Some("1")) => Some("libvpx-vp9"),
        _ => None,
    };
    // an ffmpeg built without libvpx decodes the picture alone, as before
    static DECODERS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let have = DECODERS.get_or_init(|| says("-decoders"));
    let alpha = alpha.filter(|c| have.split_whitespace().any(|w| w == *c));
    Ok((
        Info {
            codec: s["codec_name"].as_str().unwrap_or("?").into(),
            width: w,
            height: h,
            fps,
            duration,
            frames: pts.len(),
        },
        pts,
        untagged,
        alpha,
    ))
}

struct Decoder {
    path: PathBuf,
    info: Info,
    pts: Vec<f64>,
    pipe: Option<(Child, usize)>, // the child, and the index of the next frame it will give
    /// untagged HD (720 lines or more) is read as BT.709, as Apple, browsers and our own decoders read it (swscale
    /// would assume BT.601)
    bt709: bool,
    /// the decoder named before the input (libvpx for VP8/VP9 with alpha), else ffmpeg's choice
    vpx: Option<&'static str>,
}

impl Decoder {
    fn start(&mut self, at: usize) -> R<()> {
        if let Some((mut c, _)) = self.pipe.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let start = self.pts[at] - self.pts[0];
        let mut cmd = cli("ffmpeg", false)?;
        cmd.args(["-v", "error"]).args(self.vpx.map(|c| ["-c:v", c]).into_iter().flatten()).args(["-ss", &format!("{start:.6}"), "-i"]).arg(&self.path);
        if self.bt709 {
            cmd.args(["-vf", "scale=in_color_matrix=bt709:in_range=tv"]);
        }
        let child = cmd
            .args([
                "-an",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
            ])
            .args(passthrough())
            .arg("-")
            .stdout(Stdio::piped())
            // kept for a read that fails (and out of the run's output: the pipe closed under it is "Broken pipe")
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("ffmpeg: {e}"))?;
        self.pipe = Some((child, at));
        Ok(())
    }
}

impl VideoDecoder for Decoder {
    fn info(&self) -> &Info {
        &self.info
    }
    fn index_at(&mut self, t: f64) -> Option<usize> {
        (!self.pts.is_empty()).then(|| raea_io_codec::as_of(&self.pts, t + self.pts[0]))
    }
    fn frame_at(&mut self, t: f64) -> R<Frame> {
        let want = as_of(&self.pts, t + self.pts[0]);
        match &self.pipe {
            Some((_, next)) if *next <= want && want - *next < 60 => {}
            _ => self.start(want)?,
        }
        let size = (self.info.width * self.info.height * 4) as usize;
        let mut buf = vec![0u8; size];
        loop {
            let (child, next) = self.pipe.as_mut().unwrap();
            if let Err(e) = child.stdout.as_mut().unwrap().read_exact(&mut buf) {
                let mut said = String::new();
                let _ = child.stderr.as_mut().map(|s| s.read_to_string(&mut said));
                return Err(format!("ffmpeg read: {e} {}", said.trim()));
            }
            *next += 1;
            if *next - 1 == want {
                return Ok(Frame {
                    pts: self.pts[want] - self.pts[0],
                    width: self.info.width,
                    height: self.info.height,
                    rgba: buf,
                });
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VideoEnc {
    Libx264,
    H264Nvenc,
}

impl VideoEnc {
    fn label(self) -> &'static str {
        match self {
            Self::Libx264 => "h264 (libx264)",
            Self::H264Nvenc => "h264 (h264_nvenc)",
        }
    }
}

pub(crate) fn encoder_args(enc: VideoEnc, o: &EncodeOpts) -> Vec<String> {
    let mut args = vec![
        "-v".into(),
        "error".into(),
        "-y".into(),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        if o.nv12 { "nv12" } else { "rgba" }.into(),
        "-s".into(),
        format!("{}x{}", o.width, o.height),
        "-framerate".into(),
        o.fps.to_string(),
        "-i".into(),
        "-".into(),
    ];
    match enc {
        VideoEnc::Libx264 => {
            args.extend(["-c:v", "libx264", "-preset", "medium"].map(String::from));
            // constant quality as CRF (0.85 -> 13, 0.95 -> 11, 0.6 -> 18), else the average bitrate
            match o.quality {
                Some(q) => args.extend(["-crf".into(), format!("{:.0}", 30.0 - 20.0 * q.clamp(0.0, 1.0))]),
                None => args.extend([
                    "-b:v".into(),
                    format!("{}k", o.kbps),
                    "-maxrate".into(),
                    format!("{}k", o.kbps * 3 / 2),
                    "-bufsize".into(),
                    format!("{}k", o.kbps * 2),
                ]),
            }
            args.extend(["-pix_fmt", "yuv420p", "-movflags", "+faststart"].map(String::from));
        }
        VideoEnc::H264Nvenc => {
            args.extend(["-c:v", "h264_nvenc", "-preset", "p4", "-bf", "0"].map(String::from));
            match o.quality {
                Some(q) => args.extend([
                    "-rc".into(),
                    "vbr".into(),
                    "-cq".into(),
                    format!("{:.0}", 30.0 - 20.0 * q.clamp(0.0, 1.0)),
                    "-b:v".into(),
                    "0".into(),
                ]),
                None => args.extend([
                    "-rc".into(),
                    "vbr".into(),
                    "-b:v".into(),
                    format!("{}k", o.kbps),
                    "-maxrate".into(),
                    format!("{}k", o.kbps * 3 / 2),
                    "-bufsize".into(),
                    format!("{}k", o.kbps * 2),
                ]),
            }
            args.extend([
                "-pix_fmt",
                if o.nv12 { "nv12" } else { "yuv420p" },
                "-colorspace",
                "bt709",
                "-color_primaries",
                "bt709",
                "-color_trc",
                "bt709",
                "-color_range",
                "tv",
                "-movflags",
                "+faststart",
            ].map(String::from));
        }
    }
    args
}

fn spawn_encoder(out: &Path, o: &EncodeOpts, enc: VideoEnc) -> R<Box<dyn VideoEncoder>> {
    if o.width == 0 || o.height == 0 || !o.width.is_multiple_of(2) || !o.height.is_multiple_of(2) {
        return Err(format!("even dimensions required for H.264, got {}x{}", o.width, o.height));
    }
    let video = out.with_extension("video.mp4");
    let mut child = cli("ffmpeg", false)?
        .args(encoder_args(enc, o))
        .arg(&video)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    let stderr = child.stderr.take().map(|mut s| {
        std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = s.read_to_string(&mut buf);
            buf
        })
    });
    Ok(Box::new(Encoder {
        child,
        stderr,
        video,
        out: out.to_path_buf(),
        width: o.width,
        height: o.height,
        nv12: o.nv12,
        video_codec: enc.label(),
        frames: 0,
    }))
}

struct Encoder {
    child: Child,
    stderr: Option<std::thread::JoinHandle<String>>,
    video: PathBuf,
    out: PathBuf,
    width: u32,
    height: u32,
    nv12: bool,
    video_codec: &'static str,
    frames: usize,
}

impl Encoder {
    fn write_frame(&mut self, bytes: &[u8]) -> R<()> {
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(bytes)
            .map_err(|e| format!("ffmpeg stdin: {e}"))?;
        self.frames += 1;
        Ok(())
    }
}

impl VideoEncoder for Encoder {
    fn push(&mut self, rgba: &[u8]) -> R<()> {
        let (w, h) = (self.width as usize, self.height as usize);
        let need = raea_io_codec::rgba_size(w, h);
        if rgba.len() < need {
            return Err(format!("rgba frame too short: got {} bytes, need {need} ({w}x{h})", rgba.len()));
        }
        if self.nv12 {
            let nv12 = raea_io_codec::yuv::rgba_to_nv12(&rgba[..need], w, h);
            return self.write_frame(&nv12);
        }
        self.write_frame(&rgba[..need])
    }
    fn push_nv12(&mut self, nv12: &[u8], w: usize, h: usize) -> R<()> {
        if w != self.width as usize || h != self.height as usize {
            return Err(format!("nv12 frame size {w}x{h} does not match encoder {}x{}", self.width, self.height));
        }
        let need = raea_io_codec::nv12_size(w, h);
        if nv12.len() < need {
            return Err(format!("nv12 frame too short: got {} bytes, need {need} ({w}x{h})", nv12.len()));
        }
        if self.nv12 {
            return self.write_frame(&nv12[..need]);
        }
        let (y, u, v) = raea_io_codec::yuv::nv12_to_i420(&nv12[..need], w, h);
        let mut rgba = Vec::new();
        raea_io_codec::yuv::i420_to_rgba(&y, &u, &v, w, w / 2, w / 2, w, h, &mut rgba);
        self.write_frame(&rgba)
    }
    fn finish(mut self: Box<Self>, audio: Option<&Audio>) -> R<EncodeStats> {
        drop(self.child.stdin.take());
        let st = self.child.wait().map_err(|e| e.to_string())?;
        let err_msg = self.stderr.take().and_then(|h| h.join().ok()).unwrap_or_default();
        if !st.success() {
            let detail = err_msg.trim();
            return Err(if detail.is_empty() {
                format!("ffmpeg encode exited {st}")
            } else {
                format!("ffmpeg encode exited {st}: {detail}")
            });
        }
        let mut audio_codec = String::new();
        if let Some(a) = audio {
            // mux: the video stream as is, the audio columns as AAC
            let raw = self.video.with_extension("f32");
            let mut bytes = Vec::with_capacity(a.len() * a.channels.len() * 4);
            for i in 0..a.len() {
                for ch in &a.channels {
                    bytes.extend_from_slice(&ch[i].to_le_bytes());
                }
            }
            std::fs::write(&raw, bytes).map_err(|e| e.to_string())?;
            let st = cli("ffmpeg", false)?
                .args(["-v", "error", "-y", "-i"])
                .arg(&self.video)
                .args([
                    "-f",
                    "f32le",
                    "-ar",
                    &a.rate.to_string(),
                    "-ac",
                    &a.channels.len().to_string(),
                    "-i",
                ])
                .arg(&raw)
                .args([
                    "-map",
                    "0:v",
                    "-map",
                    "1:a",
                    "-c:v",
                    "copy",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "128k",
                    "-movflags",
                    "+faststart",
                    "-shortest",
                ])
                .arg(&self.out)
                .status()
                .map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&raw);
            if !st.success() {
                return Err("ffmpeg mux failed".into());
            }
            audio_codec = "aac".into();
        } else {
            std::fs::rename(&self.video, &self.out).map_err(|e| e.to_string())?;
        }
        let _ = std::fs::remove_file(&self.video);
        Ok(EncodeStats {
            frames: self.frames,
            bytes: std::fs::metadata(&self.out).map(|m| m.len()).unwrap_or(0),
            video_codec: self.video_codec.into(),
            audio_codec,
        })
    }
}

impl Backend for Ffmpeg {
    fn name(&self) -> &'static str {
        "ffmpeg"
    }
    fn open_video(&self, path: &Path) -> R<Box<dyn VideoDecoder>> {
        let (info, pts, untagged, vpx) = probe(path)?;
        let bt709 = untagged && info.width.min(info.height) >= 720;
        if pts.is_empty() {
            return Err(format!("{}: no video frames", path.display()));
        }
        Ok(Box::new(Decoder {
            path: path.to_path_buf(),
            info,
            pts,
            pipe: None,
            bt709,
            vpx,
        }))
    }
    fn decode_audio(&self, path: &Path) -> R<Audio> {
        let ch = cli("ffprobe", false)?
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=channels",
                "-of",
                "csv=p=0",
            ])
            .arg(path)
            .output()
            .map_err(|e| e.to_string())?;
        let channels: usize = String::from_utf8_lossy(&ch.stdout)
            .trim()
            .parse()
            .unwrap_or(1);
        let out = cli("ffmpeg", false)?
            .args(["-v", "error", "-i"])
            .arg(path)
            .args(["-vn", "-f", "f32le", "-ar", "48000", "-"])
            .output()
            .map_err(|e| format!("ffmpeg: {e}"))?;
        let samples: Vec<f32> = out
            .stdout
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        let mut cols = vec![Vec::with_capacity(samples.len() / channels.max(1)); channels.max(1)];
        for (i, s) in samples.into_iter().enumerate() {
            cols[i % channels.max(1)].push(s);
        }
        Ok(Audio {
            rate: 48000,
            channels: cols,
        })
    }
    fn encoder(&self, out: &Path, o: &EncodeOpts) -> R<Box<dyn VideoEncoder>> {
        spawn_encoder(out, o, VideoEnc::Libx264)
    }
}

impl Backend for Nvenc {
    fn name(&self) -> &'static str {
        "nvenc"
    }
    fn open_video(&self, path: &Path) -> R<Box<dyn VideoDecoder>> {
        Ffmpeg.open_video(path)
    }
    fn decode_audio(&self, path: &Path) -> R<Audio> {
        Ffmpeg.decode_audio(path)
    }
    fn encoder(&self, out: &Path, o: &EncodeOpts) -> R<Box<dyn VideoEncoder>> {
        if !cfg!(target_os = "linux") {
            return Err("nvenc backend is only supported on Linux".into());
        }
        if !has_encoder(encoders(), "h264_nvenc") {
            return Err("ffmpeg has no h264_nvenc encoder".into());
        }
        spawn_encoder(out, o, VideoEnc::H264Nvenc)
    }
}

#[cfg(test)]
mod tests;

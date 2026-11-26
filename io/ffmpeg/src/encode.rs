use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};

use raea_io_codec::{Audio, EncodeOpts, EncodeStats, R, VideoEncoder};

use crate::probe::cli;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VideoEnc {
    Libx264,
    H264Nvenc,
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

pub(crate) fn spawn_encoder(out: &Path, o: &EncodeOpts, enc: VideoEnc) -> R<Box<dyn VideoEncoder>> {
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
    let video_codec = match enc {
        VideoEnc::Libx264 => "h264 (libx264)",
        VideoEnc::H264Nvenc => "h264 (h264_nvenc)",
    };
    Ok(Box::new(Encoder {
        child,
        stderr,
        video,
        out: out.to_path_buf(),
        width: o.width,
        height: o.height,
        nv12: o.nv12,
        video_codec,
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

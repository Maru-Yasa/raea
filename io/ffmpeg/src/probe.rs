use std::path::Path;
use std::process::Command;

use raea_io_codec::{Info, R};

/// The ffmpeg or ffprobe to run: the PATH's, else raea's own (raea_io_assets::ffmpeg_tool). Opening a video (`fetch`)
/// fetches raea's own on Windows the first time one needs it; sound, encoding and asking what FFmpeg can do (says)
/// use FFmpeg when it's there and never fetch it (a silent clip's sound is tried on every backend).
pub(crate) fn cli(name: &str, fetch: bool) -> R<Command> {
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
pub(crate) fn says(option: &str) -> String {
    let out = cli("ffmpeg", false).ok().and_then(|mut c| c.args(["-hide_banner", option]).output().ok()).map(|o| o.stdout).unwrap_or_default();
    String::from_utf8_lossy(&out).into_owned()
}

pub(crate) fn encoders() -> &'static str {
    static ENCODERS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ENCODERS.get_or_init(|| says("-encoders"))
}

pub(crate) fn has_encoder(list: &str, name: &str) -> bool {
    list.split_whitespace().any(|w| w == name)
}

/// The H.264 encoder this machine's ffmpeg has, as its arguments, for a clip a person plays (in a browser or on a
/// phone): libx264 where ffmpeg was built with it (Homebrew's, Linux distributions'), else the system's own (Windows'
/// Media Foundation, h264_mf, in the LGPL build raea ships there; macOS's VideoToolbox; Linux NVENC) at 8 Mb/s; None without one.
pub fn h264() -> Option<Vec<&'static str>> {
    let have = encoders();
    let enc = ["libx264", "h264_mf", "h264_videotoolbox"]
        .into_iter()
        .find(|c| has_encoder(have, c))
        .or_else(|| crate::nvenc_available().then_some("h264_nvenc"))?;
    let rate: &[&'static str] = if enc == "libx264" { &[] } else { &["-b:v", "8M"] };
    Some([&["-c:v", enc][..], rate, &["-pix_fmt", "yuv420p"]].concat())
}

/// The facts, the presentation times, whether the colour matrix is untagged, and the decoder a VP8 or VP9 stream with
/// alpha needs (libvpx: ffmpeg's own VP9 decoder drops the alpha WebM keeps beside the picture, #113).
pub(crate) fn probe(path: &Path) -> R<(Info, Vec<f64>, bool, Option<&'static str>)> {
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

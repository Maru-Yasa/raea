use std::io::Write;
use std::path::Path;
use std::process::Stdio;

use raea_io_codec::{Audio, Backend, EncodeOpts, R, VideoDecoder, VideoEncoder};

use crate::Ffmpeg;
use crate::encode::{VideoEnc, spawn_encoder};
use crate::probe::{cli, encoders, has_encoder};

pub struct Nvenc;

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

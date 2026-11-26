//! (a) The ffmpeg CLI: the baseline every other backend is measured against. Decoding keeps one rawvideo pipe open
//! while requests move forward and restarts it with an accurate seek otherwise; frame times come from ffprobe.

mod encode;
mod nvenc;
mod probe;

pub use nvenc::{Nvenc, nvenc_available};
pub use probe::{h264, passthrough};

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
#[cfg(test)]
use std::process::Command;

use raea_io_codec::{
    Audio, Backend, EncodeOpts, Frame, Info, R, VideoDecoder, VideoEncoder, as_of,
};

#[cfg(test)]
use encode::{VideoEnc, encoder_args};
#[cfg(test)]
use nvenc::detect_nvenc;
#[cfg(test)]
use probe::{encoders, has_encoder};
use probe::{cli, probe};

pub struct Ffmpeg;

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
        encode::spawn_encoder(out, o, encode::VideoEnc::Libx264)
    }
}

#[cfg(test)]
mod tests;

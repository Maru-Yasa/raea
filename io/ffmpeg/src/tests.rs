use super::*;

/// A 64x64 VP9 WebM with alpha, a red square on nothing (#113), made by this machine's ffmpeg; None without ffmpeg or
/// libvpx (the test then has nothing to read).
fn alpha_webm(dir: &Path) -> Option<PathBuf> {
    let p = dir.join("square.webm");
    let made = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "color=c=red@0.0:s=64x64:d=0.4,format=rgba,geq=r=255:g=0:b=0:a='if(lt(abs(X-32),16)*lt(abs(Y-32),16),255,0)'"])
        .args(["-c:v", "libvpx-vp9", "-pix_fmt", "yuva420p", "-auto-alt-ref", "0"])
        .arg(&p)
        .status()
        .is_ok_and(|s| s.success());
    made.then_some(p)
}

#[test]
fn vp9_webm_keeps_its_alpha() {
    let dir = std::env::temp_dir().join(format!("raea-ffmpeg-alpha-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let Some(webm) = alpha_webm(&dir) else { return eprintln!("skipped: no ffmpeg with libvpx-vp9 here") };
    // as made, and copied by ffmpeg's muxer, which writes the tag as ALPHA_MODE (#224)
    let copy = dir.join("copy.webm");
    let copied = Command::new("ffmpeg").args(["-v", "error", "-y", "-i"]).arg(&webm).args(["-c", "copy"]).arg(&copy).status().is_ok_and(|s| s.success());
    assert!(copied, "ffmpeg copies the WebM");
    for file in [&webm, &copy] {
        let mut d = Ffmpeg.open_video(file).expect("the ffmpeg backend opens a VP9 WebM");
        for t in [0.0, 0.2] {
            let f = d.frame_at(t).expect("a frame");
            let at = |x: usize, y: usize| f.rgba[(y * f.width as usize + x) * 4 + 3];
            assert_eq!((at(2, 2), at(32, 32)), (0, 255), "{} at {t} s: outside the square is transparent, inside opaque", file.display());
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nvenc_args_cover_nv12_rgba_quality_and_bitrate() {
    let q_nv12 = encoder_args(
        VideoEnc::H264Nvenc,
        &EncodeOpts { width: 1920, height: 1080, fps: 30.0, kbps: 20_000, quality: Some(0.85), nv12: true, realtime: false },
    );
    assert!(q_nv12.windows(2).any(|w| w == ["-pix_fmt", "nv12"]));
    assert!(q_nv12.windows(2).any(|w| w == ["-c:v", "h264_nvenc"]));
    assert!(q_nv12.windows(2).any(|w| w == ["-bf", "0"]));
    assert!(q_nv12.windows(2).any(|w| w == ["-rc", "vbr"]));
    assert!(q_nv12.windows(2).any(|w| w == ["-cq", "13"]));
    assert!(q_nv12.windows(2).any(|w| w == ["-colorspace", "bt709"]));
    assert!(q_nv12.windows(2).any(|w| w == ["-movflags", "+faststart"]));

    let br_rgba = encoder_args(
        VideoEnc::H264Nvenc,
        &EncodeOpts { width: 1280, height: 720, fps: 60.0, kbps: 10_000, quality: None, nv12: false, realtime: false },
    );
    assert!(br_rgba.windows(2).any(|w| w == ["-pix_fmt", "rgba"]));
    assert!(br_rgba.windows(2).any(|w| w == ["-pix_fmt", "yuv420p"]));
    assert!(br_rgba.windows(2).any(|w| w == ["-b:v", "10000k"]));
    assert!(br_rgba.windows(2).any(|w| w == ["-maxrate", "15000k"]));
    assert!(br_rgba.windows(2).any(|w| w == ["-bufsize", "20000k"]));
}

#[test]
fn detect_nvenc_requires_linux_device_encoder_and_working_probe() {
    let encs = " V..... libx264\n V..... h264_nvenc\n";
    assert!(!detect_nvenc(false, true, encs, || true));
    assert!(!detect_nvenc(true, false, encs, || true));
    assert!(!detect_nvenc(true, true, " V..... libx264\n", || true));
    assert!(!detect_nvenc(true, true, encs, || false));
    assert!(detect_nvenc(true, true, encs, || true));
}

#[test]
fn ffmpeg_pipes_nv12_frames_into_playable_mp4() {
    if !has_encoder(encoders(), "libx264") {
        eprintln!("skipped: no ffmpeg with libx264 here");
        return;
    }
    let dir = std::env::temp_dir().join(format!("raea-ffmpeg-nv12-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("nv12.mp4");
    let mut enc = Ffmpeg
        .encoder(&out, &EncodeOpts { width: 64, height: 64, fps: 30.0, kbps: 1000, quality: Some(0.8), nv12: true, realtime: false })
        .expect("spawn ffmpeg nv12 encoder");
    let mut nv12 = vec![128u8; 64 * 64 * 3 / 2];
    nv12[64 * 64..].fill(128);
    enc.push_nv12(&nv12, 64, 64).expect("push nv12 frame 1");
    enc.push_nv12(&nv12, 64, 64).expect("push nv12 frame 2");
    let stats = enc.finish(None).expect("finish nv12 encode");
    assert_eq!(stats.frames, 2);
    assert_eq!(stats.video_codec, "h264 (libx264)");
    assert!(stats.bytes > 0);
    let mut dec = Ffmpeg.open_video(&out).expect("open encoded nv12 mp4");
    assert_eq!(dec.info().width, 64);
    assert_eq!(dec.info().height, 64);
    assert!(dec.frame_at(0.0).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(not(target_os = "linux"))]
#[test]
fn nvenc_encoder_refuses_non_linux() {
    let err = match Nvenc.encoder(Path::new("/tmp/nvenc-unavail.mp4"), &EncodeOpts { width: 64, height: 64, fps: 30.0, kbps: 1000, quality: None, nv12: true, realtime: false }) {
        Ok(_) => panic!("expected nvenc refusal off Linux"),
        Err(e) => e,
    };
    assert!(err.contains("Linux"), "{err}");
}

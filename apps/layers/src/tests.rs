use serde_json::json;

use crate::model::{self, Clip, Layer, Layers};
use crate::plan::*;
use crate::srt;

fn said(frames: &[&[(&str, bool)]]) -> serde_json::Value {
    let frames: Vec<_> = frames.iter().enumerate().map(|(i, t)| json!({"frame": i, "videos": [], "text": t.iter().map(|(w, cap)| if *cap { json!({"text": w, "caption": true, "opacity": 1.0}) } else { json!({"text": w, "opacity": 1.0}) }).collect::<Vec<_>>()})).collect();
    json!({"frames": frames})
}

#[test]
fn a_frames_captions_are_its_caption_words_alone() {
    // docs/layers.feature LA2b: words marked caption, in the page's order; a title's words aren't
    let m = said(&[&[("Jane Doe", false), ("Hello", true), ("  there ", true)], &[], &[("Jane Doe", false)]]);
    assert_eq!(srt::said(&m), ["Hello there", "", ""]);
}

#[test]
fn a_line_lasts_while_its_words_do_and_a_highlight_starts_none() {
    let s = |v: &[&str]| v.iter().map(|w| w.to_string()).collect::<Vec<_>>();
    // the same words over frames 0-2 (a word's colour changing isn't in the words), a gap, then a line said word by word
    let lines = srt::lines(&s(&["Hello there", "Hello there", "Hello there", "", "This", "This is", "This is raea", "Next"]));
    assert_eq!(lines, [srt::Line { text: "Hello there".into(), from: 0, to: 3 }, srt::Line { text: "This is raea".into(), from: 4, to: 7 }, srt::Line { text: "Next".into(), from: 7, to: 8 }]);
    assert_eq!(srt::write(&lines[..2], 30.0), "1\n00:00:00,000 --> 00:00:00,100\nHello there\n\n2\n00:00:00,133 --> 00:00:00,233\nThis is raea\n");
    // timecodes past a minute and an hour
    assert_eq!(srt::write(&[srt::Line { text: "x".into(), from: 108_000, to: 108_015 }], 30.0), "1\n01:00:00,000 --> 01:00:00,500\nx\n");
}

#[test]
fn clips_go_on_as_few_tracks_as_hold_them_listed_top_down() {
    let c = |id: &str, start, frames| Clip::new(id.into(), 0, start, frames, 0.0);
    let t = model::lay(Layer::Footage, "footage", vec![c("v1", 0, 30), c("v2", 10, 30), c("v3", 30, 10), c("v4", 41, 5)]);
    // v2 overlaps v1, so it goes on a second track, above the first
    assert_eq!(t.iter().map(|t| (t.name.as_str(), t.clips.iter().map(|c| c.id.as_str()).collect::<Vec<_>>())).collect::<Vec<_>>(), [("footage 2", vec!["v2"]), ("footage", vec!["v1", "v3", "v4"])]);
}

#[test]
fn layers_json_reads_back_as_it_was_written() {
    let mut v = Clip { r#box: Some([140.0, 160.0, 800.0, 470.0]), fit: Some("cover".into()), link: Some("a1".into()), ..Clip::new("v1".into(), 0, 0, 90, 1.5) };
    v.speed = 2.0;
    let a = Clip { gain_db: -6.0, fade_in: 0.5, link: Some("v1".into()), ..Clip::new("a1".into(), 0, 0, 90, 1.5) };
    let l = Layers {
        version: model::VERSION,
        name: "edit".into(),
        width: 1080,
        height: 1920,
        fps: 30.0,
        frames: 90,
        reference: "reference.mp4".into(),
        srt: Some("edit.srt".into()),
        files: vec![model::File { path: "/m/clip.mp4".into(), kind: model::FileKind::Video, seconds: 3.0, width: 640, height: 360, has_audio: true, channels: 2, sample_rate: 48_000, rotation: 0 }],
        tracks: vec![model::Track { layer: Layer::Footage, name: "footage".into(), clips: vec![v] }, model::Track { layer: Layer::Sound, name: "sound".into(), clips: vec![a] }],
        notes: vec![],
    };
    let text = serde_json::to_string(&l).unwrap();
    assert_eq!(serde_json::from_str::<Layers>(&text).unwrap(), l);
    // what a clip doesn't have isn't written: a sound has no box, footage no gain
    let j: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(j["tracks"][0]["layer"], "footage");
    assert!(j["tracks"][0]["clips"][0].get("gain_db").is_none() && j["tracks"][1]["clips"][0].get("box").is_none(), "{j}");
}

// the manifest read as runs, sounds and spans (moved from apps/capcut, docs/capcut.feature CC2a-c)

/// 90 frames at 30 fps: one take cut into two pieces (a jump in its source time at frame 45), another file in a
/// corner from frame 60, moving one pixel at frame 70. Source times in whole milliseconds, as the manifest has them.
fn manifest() -> serde_json::Value {
    let ms = |t: f64| (t * 1000.0).round() / 1000.0;
    let frames: Vec<_> = (0..90)
        .map(|f| {
            let mut v = vec![json!({"src": "media/a.mp4", "at": ms(if f < 45 { 0.5 + f as f64 / 30.0 } else { 3.0 + (f - 45) as f64 / 30.0 }), "box": [0.0, 0.0, 1080.0, 1920.0], "fit": "cover", "opacity": 1.0})];
            if f >= 60 {
                v.push(json!({"src": "media/pip.mp4", "at": ms((f - 60) as f64 / 15.0), "box": [if f >= 70 { 662.0 } else { 660.0 }, 200.0, 360.0, 360.0], "opacity": 1.0}));
            }
            json!({"frame": f, "videos": v})
        })
        .collect();
    json!({"width": 1080, "height": 1920, "fps": 30.0, "frames": frames, "sounds": {"clips": [{"src": "/m/music.wav", "from": 0, "to": 90, "at": 0.0, "gainDb": -6.0, "fadeOut": 0.5}]}})
}

#[test]
fn footage_runs_break_where_the_source_jumps() {
    let r = runs(&manifest(), 30.0);
    assert_eq!(r.iter().map(|r| (r.src.as_str(), r.start, r.end)).collect::<Vec<_>>(), [("media/a.mp4", 0, 45), ("media/a.mp4", 45, 90), ("media/pip.mp4", 60, 90)]);
    assert!((r[1].at - 3.0).abs() < 1e-9);
    assert_eq!((r[0].rate, r[1].rate), (Some(1.0), Some(1.0)), "a take at its own speed, though each step is 33 or 34 ms");
    assert!((r[2].rate.unwrap() - 2.0).abs() < 1e-6, "the corner plays at twice its speed");
    assert!(!r[2].keys.is_empty() && r[0].keys.is_empty(), "the corner moves; the take holds");
    assert_eq!(r[2].fit, "fill");
}

#[test]
fn sounds_keep_gain_and_fades() {
    let s = sounds(&manifest());
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].from, s[0].to, s[0].gain_db, s[0].fade_out), (0, 90, -6.0, 0.5));
}

#[test]
fn overlay_spans_are_runs_of_drawn_frames() {
    assert_eq!(spans(&[false, true, true, false, false, true]), [(1, 3), (5, 6)]);
    assert!(spans(&[false; 4]).is_empty());
}

/// staticFile("x.mp4") is "/media/x.mp4" in the manifest: the export finds it where the render did, in media/ beside
/// the edit, as it finds a relative path; an absolute path stays as it is
#[test]
fn a_static_file_is_found_in_the_edits_media_folder() {
    let dir = std::env::temp_dir().join(format!("raea-layers-resolve-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("media")).unwrap();
    std::fs::write(dir.join("media/a.mp4"), "").unwrap();
    let want = dunce::canonicalize(dir.join("media/a.mp4")).unwrap();
    assert_eq!(resolve(&dir, "/media/a.mp4"), want);
    assert_eq!(resolve(&dir, "media/a.mp4"), want);
    assert_eq!(resolve(&dir, want.to_str().unwrap()), want);
    std::fs::remove_dir_all(&dir).unwrap();
}

// the FCP7 XML (docs/layers.feature LA5a)

/// motion's (scale, centre, crop) for a picture in `bx` on a `w` x `h` frame, in `editor`'s terms
fn placed(editor: crate::xml::Editor, bx: [f64; 4], clip: Option<[f64; 4]>, fit: &str, position: Option<[f64; 4]>, media: (f64, f64), frame: (f64, f64)) -> (f64, [f64; 2], [f64; 4]) {
    crate::xml::motion(editor, &picture(&bx, &clip, fit, &position, media, frame).0, media, false, frame)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn a_picture_in_picture_sits_as_raea_drew_it_in_basic_motions_terms() {
    use crate::xml::Editor::*;
    // C (1280x720) contained in a 380x225 box at (850, 50) on a 1280x720 sequence: scaled to 380/1280, its centre
    // right of and above the frame's, nothing cut; the media the frame's size, so both editors alike
    for e in [Premiere, Resolve] {
        let (scale, centre, crop) = placed(e, [850.0, 50.0, 380.0, 225.0], None, "contain", None, (1280.0, 720.0), (1280.0, 720.0));
        assert!(near(scale, 380.0 / 1280.0 * 100.0) && near(centre[0], (1040.0 - 640.0) / 1280.0) && near(centre[1], (162.5 - 360.0) / 720.0));
        assert_eq!(crop, [0.0; 4]);
    }
    // stretched in raea: fitted inside, and said
    assert!(picture(&[0.0, 0.0, 500.0, 100.0], &None, "fill", &None, (640.0, 640.0), (1080.0, 1920.0)).1.is_some());
}

/// a horizontal (landscape) take covering a vertical frame: scaled to fill its height, uncut (the frame cuts its
/// sides, so an editor reframing it finds the rest); Premiere's Scale of its pixels, Resolve's of it fitted (whole) to
/// the frame
#[test]
fn a_horizontal_take_covers_a_vertical_frame_uncut() {
    use crate::xml::Editor::*;
    let (scale, centre, crop) = placed(Premiere, [0.0, 0.0, 1080.0, 1920.0], None, "cover", None, (3840.0, 2160.0), (1080.0, 1920.0));
    assert!(near(scale, 1920.0 / 2160.0 * 100.0) && centre == [0.0, 0.0] && crop == [0.0; 4]);
    let (scale, _, _) = placed(Resolve, [0.0, 0.0, 1080.0, 1920.0], None, "cover", None, (3840.0, 2160.0), (1080.0, 1920.0));
    assert!(near(scale, (1920.0 / 2160.0) / (1080.0 / 3840.0) * 100.0));
    // framed on its left third (object-position: 0% 50%): the picture's left edge on the frame's, its centre right of
    // the frame's by half its overhang, in each editor's units
    let dw = 3840.0 * 1920.0 / 2160.0;
    let (_, centre, crop) = placed(Premiere, [0.0, 0.0, 1080.0, 1920.0], None, "cover", Some([0.0, 0.0, 0.0, 0.5]), (3840.0, 2160.0), (1080.0, 1920.0));
    assert!(near(centre[0], (dw / 2.0 - 540.0) / 3840.0) && centre[1] == 0.0 && crop == [0.0; 4]);
    let (_, centre, _) = placed(Resolve, [0.0, 0.0, 1080.0, 1920.0], None, "cover", Some([0.0, 0.0, 0.0, 0.5]), (3840.0, 2160.0), (1080.0, 1920.0));
    assert!(near(centre[0], (dw / 2.0 - 540.0) / 1080.0));
    // contained in a picture in picture above the frame's middle: Premiere's Center in the take's own pixels, Resolve's in
    // the take fitted to the frame (1080 x 607.5), across and down alike (Resolve drew 1080's units 238 px low, 11-17)
    let (_, centre, _) = placed(Premiere, [80.0, 300.0, 620.0, 620.0], None, "contain", None, (3840.0, 2160.0), (1080.0, 1920.0));
    assert!(near(centre[0], -150.0 / 3840.0) && near(centre[1], -350.0 / 2160.0), "{centre:?}");
    let (_, centre, _) = placed(Resolve, [80.0, 300.0, 620.0, 620.0], None, "contain", None, (3840.0, 2160.0), (1080.0, 1920.0));
    assert!(near(centre[0], -150.0 / 1080.0) && near(centre[1], -350.0 / 607.5), "{centre:?}");
    // in a box inside the frame it's cut to the box, each side by what overhangs it
    let (_, _, crop) = placed(Premiere, [100.0, 300.0, 880.0, 880.0], None, "cover", None, (3840.0, 2160.0), (1080.0, 1920.0));
    let over = (3840.0 * 880.0 / 2160.0 - 880.0) / 2.0 / (3840.0 * 880.0 / 2160.0) * 100.0;
    assert!(near(crop[0], over) && near(crop[1], over) && crop[2] == 0.0 && crop[3] == 0.0, "{crop:?}");
}

/// composed at the footage's own size (a 4K edit of 4K takes): 100 in both editors, wherever it's placed; a 1080 edit of
/// the same takes is Premiere's 50 and Resolve's 100, its centre in each one's units
#[test]
fn footage_at_the_frames_size_is_at_100_in_both_editors() {
    use crate::xml::Editor::*;
    for e in [Premiere, Resolve] {
        let (scale, centre, crop) = placed(e, [0.0, 0.0, 2160.0, 3840.0], None, "cover", None, (2160.0, 3840.0), (2160.0, 3840.0));
        assert!(near(scale, 100.0) && centre == [0.0, 0.0] && crop == [0.0; 4], "{e:?}");
        // risen 520 px (the 1080 edit's 260): Premiere's 520 / 3840 of the take, Resolve's 520 / 3840 of the frame
        let (_, centre, _) = placed(e, [0.0, -520.0, 2160.0, 3840.0], None, "cover", None, (2160.0, 3840.0), (2160.0, 3840.0));
        assert!(near(centre[1], -520.0 / 3840.0));
    }
    let (scale, centre, _) = placed(Premiere, [0.0, -260.0, 1080.0, 1920.0], None, "cover", None, (2160.0, 3840.0), (1080.0, 1920.0));
    assert!(near(scale, 50.0) && near(centre[1], -260.0 / 3840.0));
    let (scale, centre, _) = placed(Resolve, [0.0, -260.0, 1080.0, 1920.0], None, "cover", None, (2160.0, 3840.0), (1080.0, 1920.0));
    assert!(near(scale, 100.0) && near(centre[1], -260.0 / 1920.0));
    // across, in each one's units: a picture in picture 300 px right of centre
    let (_, centre, _) = placed(Premiere, [690.0, 760.0, 300.0, 400.0], None, "cover", None, (2160.0, 3840.0), (1080.0, 1920.0));
    assert!(near(centre[0], 300.0 / 2160.0));
    let (_, centre, _) = placed(Resolve, [690.0, 760.0, 300.0, 400.0], None, "cover", None, (2160.0, 3840.0), (1080.0, 1920.0));
    assert!(near(centre[0], 300.0 / 1080.0));
}

#[test]
fn the_xml_has_the_tracks_bottom_first_and_footage_linked_to_its_sound() {
    use crate::model::{File, FileKind, Track};
    let file = |path: &str, kind, w, h, a| File { path: path.into(), kind, seconds: 10.0, width: w, height: h, has_audio: a, channels: 2, sample_rate: 48_000, rotation: 0 };
    let c = |id: &str, f, start, frames, at| Clip::new(id.into(), f, start, frames, at);
    let v1 = Clip { r#box: Some([0.0, 0.0, 1280.0, 720.0]), fit: Some("cover".into()), link: Some("a1".into()), ..c("v1", 0, 0, 90, 1.0) };
    let v2 = Clip { r#box: Some([0.0, 0.0, 1280.0, 720.0]), fit: Some("cover".into()), ..c("v2", 0, 90, 30, 5.0) };
    let a1 = Clip { link: Some("v1".into()), ..c("a1", 0, 0, 90, 1.0) };
    let l = Layers {
        version: model::VERSION,
        name: "edit & co".into(),
        width: 1280,
        height: 720,
        fps: 29.97,
        frames: 120,
        reference: "reference.mp4".into(),
        srt: None,
        files: vec![file("/m/my clip.mp4", FileKind::Video, 1280, 720, true), file("graphics.prores.mov", FileKind::Video, 1280, 720, false), file("/m/music.wav", FileKind::Audio, 0, 0, true)],
        tracks: vec![
            Track { layer: Layer::Graphics, name: "graphics".into(), clips: vec![c("g1", 1, 30, 60, 1.0)] },
            Track { layer: Layer::Footage, name: "footage".into(), clips: vec![v1, v2] },
            Track { layer: Layer::Sound, name: "sound".into(), clips: vec![a1, Clip { gain_db: -12.0, ..c("a2", 2, 0, 120, 0.0) }] },
        ],
        notes: vec![],
    };
    let (x, notes) = crate::xml::write(&l, std::path::Path::new("/out/xml/edit"), crate::xml::Editor::Premiere);
    assert!(notes.is_empty(), "{notes:?}");
    // 29.97 is a timebase of 30, NTSC; the name escaped
    assert!(x.contains("<name>edit &amp; co</name><duration>120</duration><rate><timebase>30</timebase><ntsc>TRUE</ntsc></rate>"), "{x}");
    // video tracks bottom first: the footage (v1 from 30 source frames in, v2 from 150), then the graphics
    let footage = x.find("<clipitem id=\"v1\"").unwrap();
    assert!(footage < x.find("<clipitem id=\"g1\"").unwrap());
    assert!(x.contains("<start>0</start><end>90</end><in>30</in><out>120</out>") && x.contains("<start>90</start><end>120</end><in>150</in><out>180</out>"), "{x}");
    // a file in full once (its path percent-encoded), by id after
    assert_eq!(x.matches("<pathurl>file://localhost/m/my%20clip.mp4</pathurl>").count(), 1);
    assert!(x.contains("<file id=\"file-1\"/>"));
    // the graphics' movie found beside the XML, its alpha straight
    assert!(x.contains("<pathurl>file://localhost/out/xml/edit/graphics.prores.mov</pathurl>") && x.contains("<alphatype>straight</alphatype>"));
    // v1 and its sound's two channels (each a mono clipitem on a track of its own, as Premiere writes a stereo clip)
    // linked together, in each of the three; v2 has no sound placed
    let link = "<link><linkclipref>v1</linkclipref><mediatype>video</mediatype><trackindex>1</trackindex><clipindex>1</clipindex></link><link><linkclipref>a1</linkclipref><mediatype>audio</mediatype><trackindex>1</trackindex><clipindex>1</clipindex></link><link><linkclipref>a1-2</linkclipref><mediatype>audio</mediatype><trackindex>2</trackindex><clipindex>1</clipindex></link>";
    assert_eq!(x.matches(link).count(), 3, "{x}");
    assert!(x.contains("<clipitem id=\"a1-2\" premiereChannelType=\"mono\"><masterclipid>masterclip-1</masterclipid>"), "{x}");
    assert!(x.contains("<sourcetrack><mediatype>audio</mediatype><trackindex>2</trackindex></sourcetrack>"));
    // the file's sound as its two channels, left and right; the two tracks to output channels 1 and 2
    assert!(x.contains("<channelcount>1</channelcount><layout>stereo</layout><audiochannel><sourcechannel>2</sourcechannel><channellabel>right</channellabel></audiochannel>"));
    assert!(x.contains("<outputchannelindex>1</outputchannelindex></track>") && x.contains("<outputchannelindex>2</outputchannelindex></track>"));
    // each channel panned to its own side by an Audio Pan on the clip, as Resolve writes it; nothing on the track
    assert!(!x.contains("PannerCurrentValue"), "{x}");
    assert!(x.contains("<effectid>audiopan</effectid><effectcategory>audiopan</effectcategory><effecttype>audiopan</effecttype><mediatype>audio</mediatype><parameter><parameterid>pan</parameterid><name>Pan</name><valuemin>-1</valuemin><valuemax>1</valuemax><value>-1</value>"), "{x}");
    assert_eq!(x.matches("<parameterid>pan</parameterid>").count(), 4, "two stereo clips, two channels each");
    // the music at -12 dB as Audio Levels' linear gain
    assert!(x.contains("<parameterid>level</parameterid><name>Level</name><valuemin>0.00003162</valuemin><valuemax>31.6228</valuemax><value>0.251189</value>"), "{x}");
}

#[test]
fn a_sound_is_a_clip_as_long_as_its_file_plays() {
    let s = |from, to, at, rate| Sound { src: "tick.wav".into(), from, to, at, gain_db: 0.0, rate, fade_in: 0.0, fade_out: 0.0 };
    // a one-second tick in a 2.5 s Sequence at 24 fps: 24 frames, not 60
    assert_eq!(heard_frames(&s(100, 160, 0.0, 1.0), 1.0, 24.0), 24);
    // a long file in a short Sequence keeps the Sequence's length
    assert_eq!(heard_frames(&s(0, 48, 0.0, 1.0), 84.0, 24.0), 48);
    // from 0.5 s in, at double speed: 0.25 s left, 6 frames
    assert_eq!(heard_frames(&s(0, 60, 0.5, 2.0), 1.0, 24.0), 6);
    // started past its end: nothing heard
    assert_eq!(heard_frames(&s(0, 60, 2.0, 1.0), 1.0, 24.0), 0);
}

/// footage that rises in a box that shrinks over it (a split screen): keys where it holds and on either side of the
/// move, a clip from its box's cut; a run that holds still has none
#[test]
fn footage_that_moves_keeps_keys_where_it_changes() {
    let frame = |f: i64, top: f64, clip_h: Option<f64>| {
        let mut v = json!({"src": "a.mp4", "at": 1.0 + f as f64 / 24.0, "box": [0.0, top, 1080.0, 1920.0], "opacity": 1.0, "fit": "cover"});
        if let Some(h) = clip_h {
            v["clip"] = json!([0.0, 0.0, 1080.0, h]);
        }
        json!({"frame": f, "videos": [v]})
    };
    // frames 0-2 full, 3-5 rising and cut, 6-8 held split
    let tops = [0.0, 0.0, 0.0, -100.0, -200.0, -260.0, -260.0, -260.0, -260.0];
    let clips = [None, None, None, Some(1400.0), Some(1000.0), Some(860.0), Some(860.0), Some(860.0), Some(860.0)];
    let m = json!({"frames": (0..9).map(|f| frame(f, tops[f as usize], clips[f as usize])).collect::<Vec<_>>()});
    let r = &runs(&m, 24.0)[0];
    assert!(r.clip.is_none());
    assert_eq!(r.keys.iter().map(|k| k.frame).collect::<Vec<_>>(), [0, 2, 3, 4, 5, 8]);
    assert_eq!(r.keys[3].clip, Some([0.0, 0.0, 1080.0, 1000.0]));
    let still = json!({"frames": (0..9).map(|f| frame(f, 0.0, Some(860.0))).collect::<Vec<_>>()});
    let r = &runs(&still, 24.0)[0];
    assert!(r.keys.is_empty() && r.clip == Some([0.0, 0.0, 1080.0, 860.0]));
    // between two keys, a straight line: halfway from frame 3 (risen 100, cut at 1400) to 4 has none, but 3.. 5 does
    let moving = runs(&m, 24.0).remove(0);
    let k = moving.at(4);
    assert_eq!((k.r#box[1], k.clip), (-200.0, Some([0.0, 0.0, 1080.0, 1000.0])));
    assert_eq!(moving.at(7).r#box[1], -260.0);
    assert_eq!((moving.at(-5).r#box[1], moving.at(20).r#box[1]), (0.0, -260.0), "before its first key and after its last");
}

#[test]
fn a_clip_cuts_the_picture_as_crop() {
    use crate::xml::Editor::Premiere;
    // a 1080x1920 take filling a portrait frame, risen 260 px, its box cut at 860: placed by its box, all below 860
    // cut; its top (past the frame, which cuts it) not
    let (scale, centre, crop) = placed(Premiere, [0.0, -260.0, 1080.0, 1920.0], Some([0.0, 0.0, 1080.0, 860.0]), "cover", None, (1080.0, 1920.0), (1080.0, 1920.0));
    assert!(near(scale, 100.0) && near(centre[1], -260.0 / 1920.0));
    assert!(crop[0] == 0.0 && crop[1] == 0.0 && crop[2] == 0.0 && near(crop[3], (1920.0 - 1120.0) / 1920.0 * 100.0), "{crop:?}");
    // a picture in picture cut by a card's edges inside the frame: cut where the card cuts it
    let (_, _, crop) = placed(Premiere, [100.0, 100.0, 400.0, 400.0], Some([150.0, 100.0, 300.0, 400.0]), "cover", None, (400.0, 400.0), (1080.0, 1920.0));
    assert!(near(crop[0], 12.5) && near(crop[1], 12.5) && crop[2] == 0.0 && crop[3] == 0.0, "{crop:?}");
}

/// a steady move is its two ends in the XML, an eased one a key a frame; a clip at another speed is placed at its own
/// speed, as many source frames as it lasts, and said
#[test]
fn steady_moves_are_two_keys_and_other_speeds_are_said() {
    use crate::model::{File, FileKind, Key, Track};
    let k = |frame: i64, x: f64| Key { frame, r#box: [x, 500.0, 400.0, 400.0], clip: None, position: None, opacity: 1.0 };
    let steady = Clip { r#box: Some([100.0, 500.0, 400.0, 400.0]), fit: Some("cover".into()), keys: (0..24).map(|f| k(f, 100.0 + 10.0 * f as f64)).collect(), ..Clip::new("v1".into(), 0, 0, 24, 0.0) };
    let eased = Clip { r#box: Some([100.0, 500.0, 400.0, 400.0]), fit: Some("cover".into()), keys: (0..24).map(|f| k(f, 100.0 + (f * f) as f64)).collect(), speed: 2.0, ..Clip::new("v2".into(), 0, 24, 24, 0.0) };
    let l = Layers {
        version: model::VERSION,
        name: "e".into(),
        width: 1080,
        height: 1920,
        fps: 24.0,
        frames: 48,
        reference: "reference.mp4".into(),
        srt: None,
        files: vec![File { path: "/m/a.mov".into(), kind: FileKind::Video, seconds: 10.0, width: 400, height: 400, has_audio: false, channels: 2, sample_rate: 48_000, rotation: 0 }],
        tracks: vec![Track { layer: Layer::Footage, name: "footage".into(), clips: vec![steady, eased] }],
        notes: vec![],
    };
    let (x, notes) = crate::xml::write(&l, std::path::Path::new("/out"), crate::xml::Editor::Premiere);
    let v1 = &x[x.find("<clipitem id=\"v1\"").unwrap()..x.find("<clipitem id=\"v2\"").unwrap()];
    assert_eq!(v1.matches("<keyframe>").count(), 2, "{v1}");
    let v2 = &x[x.find("<clipitem id=\"v2\"").unwrap()..];
    assert!(v2.matches("<keyframe>").count() > 20 && v2.contains("<start>24</start><end>48</end><in>0</in><out>24</out>"), "{v2}");
    assert!(notes.iter().any(|n| n.contains("plays at 2.00x in raea")), "{notes:?}");
}

#[test]
fn keyed_footage_is_keyframed_in_the_xml() {
    use crate::model::{File, FileKind, Key, Track};
    let k = |frame, top: f64, h: f64, opacity| Key { frame, r#box: [0.0, top, 1080.0, 1920.0], clip: (h < 1920.0).then_some([0.0, 0.0, 1080.0, h]), position: None, opacity };
    let v = Clip { r#box: Some([0.0, 0.0, 1080.0, 1920.0]), fit: Some("cover".into()), keys: vec![k(0, 0.0, 1920.0, 1.0), k(12, -260.0, 860.0, 1.0), k(40, -260.0, 860.0, 0.5)], ..Clip::new("v1".into(), 0, 100, 41, 2.0) };
    let l = Layers {
        version: model::VERSION,
        name: "e".into(),
        width: 1080,
        height: 1920,
        fps: 24.0,
        frames: 141,
        reference: "reference.mp4".into(),
        srt: None,
        files: vec![File { path: "/m/a.mov".into(), kind: FileKind::Video, seconds: 10.0, width: 1080, height: 1920, has_audio: false, channels: 2, sample_rate: 48_000, rotation: 0 }],
        tracks: vec![Track { layer: Layer::Footage, name: "footage".into(), clips: vec![v] }],
        notes: vec![],
    };
    use crate::xml::Editor;
    let (x, _) = crate::xml::write(&l, std::path::Path::new("/out"), Editor::Premiere);
    // scale holds: a value, no keyframes; the centre, crop and opacity keyed, each key in the source's frames (from 48)
    assert!(x.contains("<parameterid>scale</parameterid><name>Scale</name><valuemin>0</valuemin><valuemax>1000</valuemax><value>100.0000</value></parameter>"), "{x}");
    assert!(x.contains("<parameterid>center</parameterid><name>Center</name><value><horiz>0.000000</horiz><vert>0.000000</vert></value><keyframe><when>48</when><value><horiz>0.000000</horiz><vert>0.000000</vert></value></keyframe><keyframe><when>60</when><value><horiz>0.000000</horiz><vert>-0.135417</vert></value></keyframe>"), "{x}");
    // Premiere's crop is Basic Motion's, as Premiere writes it: the bottom 41.7% cut at the split
    assert!(x.contains("<parameterid>bottomcrop</parameterid><name>Bottom</name><valuemin>0</valuemin><valuemax>100</valuemax><value>0.0000</value><keyframe><when>48</when><value>0.0000</value></keyframe><keyframe><when>60</when><value>41.6667</value></keyframe>"), "{x}");
    assert!(!x.contains("<effectid>crop</effectid>"), "{x}");
    assert!(x.contains("<parameterid>opacity</parameterid><name>opacity</name>") && x.contains("<keyframe><when>88</when><value>50.00</value></keyframe>"), "{x}");
    // Resolve's is FCP7's Crop filter, as it is for a file stored upright: 41.7% off its bottom (its top past the frame,
    // which cuts it)
    let (x, _) = crate::xml::write(&l, std::path::Path::new("/out"), Editor::Resolve);
    assert!(!x.contains("bottomcrop") && x.contains("<effectid>crop</effectid>"), "{x}");
    // a take twice the timeline's size, stored on its side (a phone's): Premiere's is described as stored (3840 x 2160,
    // as Premiere's own XML describes it) and its Center in that, 260 px up 260 / 2160; Resolve's in the frame's
    let mut phone = l.clone();
    (phone.files[0].width, phone.files[0].height, phone.files[0].rotation) = (2160, 3840, 90);
    let premiere = crate::xml::write(&phone, std::path::Path::new("/out"), Editor::Premiere).0;
    assert!(premiere.contains("<width>3840</width><height>2160</height>") && premiere.contains("<keyframe><when>60</when><value><horiz>0.000000</horiz><vert>-0.120370</vert></value></keyframe>"), "{premiere}");
    assert!(crate::xml::write(&phone, std::path::Path::new("/out"), Editor::Resolve).0.contains("<width>2160</width><height>3840</height>"));
    assert!(crate::xml::write(&phone, std::path::Path::new("/out"), Editor::Resolve).0.contains("<keyframe><when>60</when><value><horiz>0.000000</horiz><vert>-0.135417</vert></value></keyframe>"));
    // a take twice the timeline's size: Premiere's 50 (of its pixels) is Resolve's 100 (fitted to the timeline)
    let mut big = l.clone();
    big.files[0].width = 2160;
    big.files[0].height = 3840;
    assert!(crate::xml::write(&big, std::path::Path::new("/out"), Editor::Premiere).0.contains("<parameterid>scale</parameterid><name>Scale</name><valuemin>0</valuemin><valuemax>1000</valuemax><value>50.0000</value>"));
    assert!(crate::xml::write(&big, std::path::Path::new("/out"), Editor::Resolve).0.contains("<parameterid>scale</parameterid><name>Scale</name><valuemin>0</valuemin><valuemax>1000</valuemax><value>100.0000</value>"));
    assert!(x.contains("<parameterid>bottom</parameterid><name>bottom</name><valuemin>0</valuemin><valuemax>100</valuemax><value>0.0000</value><keyframe><when>48</when><value>0.0000</value></keyframe><keyframe><when>60</when><value>41.6667</value></keyframe>"), "{x}");
    // a phone's take stored turned 90: Resolve cuts it mirrored top to bottom, so its sides go swapped (11-17's render),
    // Premiere's as they are
    let mut l = l;
    l.files[0].rotation = 90;
    assert!(crate::xml::write(&l, std::path::Path::new("/out"), Editor::Premiere).0.contains("<parameterid>bottomcrop</parameterid><name>Bottom</name><valuemin>0</valuemin><valuemax>100</valuemax><value>0.0000</value><keyframe><when>48</when><value>0.0000</value></keyframe><keyframe><when>60</when><value>41.6667</value>"));
    let (x, _) = crate::xml::write(&l, std::path::Path::new("/out"), Editor::Resolve);
    assert!(x.contains("<parameterid>top</parameterid><name>top</name><valuemin>0</valuemin><valuemax>100</valuemax><value>0.0000</value><keyframe><when>48</when><value>0.0000</value></keyframe><keyframe><when>60</when><value>41.6667</value></keyframe>"), "{x}");
    assert!(x.contains("<parameterid>bottom</parameterid><name>bottom</name><valuemin>0</valuemin><valuemax>100</valuemax><value>0.0000</value></parameter>"), "{x}");
}

/// a pan inside a still box (object-position animated, a Ken Burns across a horizontal take): keyed like a move, the
/// picture's centre crossing the frame in the XML
#[test]
fn a_pan_inside_its_box_is_keyed() {
    let frame = |f: i64| json!({"frame": f, "videos": [{"src": "land.mov", "at": f as f64 / 24.0, "box": [0.0, 0.0, 1080.0, 1920.0], "opacity": 1.0, "fit": "cover", "position": [0.0, f as f64 / 10.0, 0.0, 0.5]}]});
    let r = &runs(&json!({"frames": (0..11).map(frame).collect::<Vec<_>>()}), 24.0)[0];
    assert!(!r.keys.is_empty() && r.at(5).position == Some([0.0, 0.5, 0.0, 0.5]));
    let (from, to) = (r.at(0), r.at(10));
    let centre = |k: &Key| crate::xml::motion(crate::xml::Editor::Resolve, &picture(&k.r#box, &k.clip, "cover", &k.position, (3840.0, 2160.0), (1080.0, 1920.0)).0, (3840.0, 2160.0), false, (1080.0, 1920.0)).1[0];
    let dw = 3840.0 * 1920.0 / 2160.0;
    assert!(near(centre(&from), (dw / 2.0 - 540.0) / 1080.0) && near(centre(&to), -(dw / 2.0 - 540.0) / 1080.0));
}

#[test]
fn a_windows_path_is_a_file_url_an_editor_opens() {
    // #195: 0.1.8-rc8 wrote file://localhostD%3A%5CWork%5C... for D:\Work\...; Resolve imported the same XML at once with
    // file:///D:/Work/...
    use crate::xml::pathurl;
    use std::path::Path;
    assert_eq!(pathurl(Path::new(r"D:\Work\_research\raea\xmltest\public\clip.mp4")), "file:///D:/Work/_research/raea/xmltest/public/clip.mp4");
    assert_eq!(pathurl(Path::new(r"\\?\C:\Users\Jürgen\my clip.mp4")), "file:///C:/Users/J%C3%BCrgen/my%20clip.mp4");
    assert_eq!(pathurl(Path::new("C:/Users/me/out/xml/edit/graphics.rle.mov")), "file:///C:/Users/me/out/xml/edit/graphics.rle.mov");
    assert_eq!(pathurl(Path::new(r"\\nas\clips\a.mp4")), "file://nas/clips/a.mp4");
    assert_eq!(pathurl(Path::new(r"\\?\UNC\nas\clips\a.mp4")), "file://nas/clips/a.mp4");
    // a Mac's or Linux's as before, a backslash there being part of a name
    assert_eq!(pathurl(Path::new("/Users/me/my clip.mp4")), "file://localhost/Users/me/my%20clip.mp4");
    assert_eq!(pathurl(Path::new(r"/tmp/a\b.mp4")), "file://localhost/tmp/a%5Cb.mp4");
}

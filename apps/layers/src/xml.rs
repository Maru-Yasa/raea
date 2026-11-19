//! layers.json as an FCP7 XML project (xmeml version 5, docs/layers.feature LA5a), which Premiere Pro (File, Import)
//! and DaVinci Resolve (File, Import, Timeline) both read: one sequence at the composition's rate and size, its video
//! tracks bottom first (under, the footage, what's behind a picture in picture, the pictures in picture, graphics,
//! captions), its audio tracks below. Footage points at its file where it is, placed as raea drew it (Basic Motion's
//! scale and centre, its crop where object-fit: cover or a box it's in cuts it, its Opacity, each keyframed where it
//! changes over the clip; the crop as each editor writes it, so an XML for each: see Editor); a footage clip and its own sound are linked
//! clipitems, which Premiere's Unlink and Resolve's Link Clips separate. Sound is laid out as Premiere writes it (read
//! from an XML Premiere Pro 2026 exported): a stereo file's channels each a mono clipitem on a track of its own, each
//! panned to its side by an Audio Pan on the clip, as Resolve and FCP7 write it (both editors import a channel centred
//! without it; Premiere's own track pan, PannerCurrentValue, was tried and neither read it). A layer's movie fills the frame, its alpha
//! straight. Pure: the export (lib.rs) makes the layers and writes this beside them.

use std::fmt::Write as _;
use std::path::Path;

use crate::model::{Clip, FileKind, Key, Layer, Layers};
use crate::plan::{self, Picture};

/// xmeml's rate: a whole timebase and whether it runs at 1000/1001 of it (29.97 is 30 NTSC).
fn rate(fps: f64) -> (u32, bool) {
    let base = fps.round().max(1.0);
    (base as u32, (fps - base).abs() > 0.001 && (fps * 1001.0 / 1000.0 - base).abs() < 0.01)
}

fn rate_xml(fps: f64) -> String {
    let (tb, ntsc) = rate(fps);
    format!("<rate><timebase>{tb}</timebase><ntsc>{}</ntsc></rate>", if ntsc { "TRUE" } else { "FALSE" })
}

/// A timecode from 0, as Premiere gives a sequence and every file.
fn timecode(fps: f64) -> String {
    let drop = rate(fps).1;
    format!("<timecode>{}<string>{}</string><frame>0</frame><displayformat>{}</displayformat></timecode>", rate_xml(fps), if drop { "00;00;00;00" } else { "00:00:00:00" }, if drop { "DF" } else { "NDF" })
}

/// A UUID-shaped id for the sequence, the same for the same name (FNV-1a over it, twice).
fn uuid(name: &str) -> String {
    let h = |seed: u64| name.bytes().fold(seed, |k, b| (k ^ b as u64).wrapping_mul(0x100_0000_01b3));
    let (a, b) = (h(0xcbf2_9ce4_8422_2325), h(0x8422_2325_cbf2_9ce4));
    format!("{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}", a >> 32, (a >> 16) & 0xffff, a & 0xfff, (b >> 52) & 0xfff, b & 0xffff_ffff_ffff)
}

/// Text as XML: its five reserved characters as entities.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// A file's path as xmeml's pathurl: file://localhost and the absolute path, each byte outside the unreserved set
/// percent-encoded. A Windows path is made a URL as a path, not as text (#195: 0.1.8-rc8 wrote
/// file://localhostD%3A%5CWork%5C..., which Resolve refused): a drive's is file:///D:/Work/clip.mp4, the form Resolve
/// opened, and a share's file://nas/clips/clip.mp4, each with its separators as slashes and Windows' \\?\ prefix off.
pub(crate) fn pathurl(p: &Path) -> String {
    let s = p.to_string_lossy();
    let s = match (s.strip_prefix(r"\\?\UNC\"), s.strip_prefix(r"\\?\")) {
        (Some(share), _) => format!(r"\\{share}"),
        (None, Some(local)) => local.to_string(),
        _ => s.into_owned(),
    };
    let b = s.as_bytes();
    let drive = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/');
    let (mut out, rest, windows) = if drive {
        (format!("file:///{}:", b[0] as char), &b[2..], true)
    } else if let Some(share) = s.strip_prefix(r"\\") {
        (String::from("file://"), share.as_bytes(), true)
    } else {
        (String::from("file://localhost"), b, false)
    };
    for &c in rest {
        if windows && c == b'\\' {
            out.push('/');
        } else if c.is_ascii_alphanumeric() || b"/-_.~".contains(&c) {
            out.push(c as char);
        } else {
            let _ = write!(out, "%{c:02X}");
        }
    }
    out
}

/// Basic Motion's scale and centre and the crop's sides ([left, right, top, bottom], percent of the picture) for a
/// picture drawn as `p` (plan::picture) from media of `mw` x `mh` (upright; `turned` when it's stored a quarter turn
/// from that, a phone's take) on a `w` x `h` sequence, in `editor`'s terms (see units).
pub fn motion(editor: Editor, p: &Picture, media: (f64, f64), turned: bool, frame: (f64, f64)) -> (f64, [f64; 2], [f64; 4]) {
    let [dx, dy, dw, dh] = p.drawn;
    let (cx, cy) = (dx + dw / 2.0 - frame.0 / 2.0, dy + dh / 2.0 - frame.1 / 2.0);
    let (whole, unit) = units(editor, media, turned, frame);
    let [l, t, r, b] = p.kept;
    (dw / whole * 100.0, [cx / unit.0, cy / unit.1], [l * 100.0, (1.0 - r) * 100.0, t * 100.0, (1.0 - b) * 100.0])
}

/// An editor's units on the canvas, in px: the picture's width at Scale 100, and Center's 1 across and down, each the
/// clip's own size at Scale 100 as that editor has it, read from what it did with what it was given (11-17).
/// Premiere: the media's own pixels, Center in its width and height as the XML describes it, which is as stored
/// (Premiere's own XML describes a phone's take 3840 x 2160 and wrote a 68 px move of it 68 / 2160; told 2160 x 3840 it
/// placed -0.120370 at 462.2 px, 0.120370 of 3840). Resolve: the media fitted to the frame (its default Input Scaling,
/// "scale entire image to fit": its own XML gave a 4K take on a smaller timeline Scale 100), Center in that fitted
/// size too (a horizontal take fitted to a 1080 x 1920 frame is 1080 x 607.5: it placed Center vert -0.182 at 110 px).
fn units(editor: Editor, (mw, mh): (f64, f64), turned: bool, (w, h): (f64, f64)) -> (f64, (f64, f64)) {
    match editor {
        Editor::Premiere if turned => (mw, (mh, mw)),
        Editor::Premiere => (mw, (mw, mh)),
        Editor::Resolve => {
            let fit = (w / mw).min(h / mh);
            (mw * fit, (mw * fit, mh * fit))
        }
    }
}


/// An FCP7 parameter: its id and name, its range (none for Center's), its value and its keyframes (none when it holds).
fn parameter(id: &str, name: &str, range: Option<(f64, f64)>, value: &str, keyframes: &str) -> String {
    let range = range.map_or(String::new(), |(a, b)| format!("<valuemin>{a}</valuemin><valuemax>{b}</valuemax>"));
    format!("<parameter><parameterid>{id}</parameterid><name>{name}</name>{range}<value>{value}</value>{keyframes}</parameter>")
}

/// A parameter that holds one value, or (its values differing) is keyframed: its first value, then a keyframe at each
/// of `at`'s whens, each value written by `value`. `range` is its (min, max), none for Center's.
fn keyed<const N: usize>(id: &str, name: &str, range: Option<(f64, f64)>, at: &[(i64, [f64; N])], value: impl Fn(&[f64; N]) -> String) -> String {
    let values: Vec<String> = at.iter().map(|(_, p)| value(p)).collect();
    let first = &values[0];
    let keys: String = if values.iter().all(|v| v == first) {
        String::new()
    } else {
        at.iter().zip(&values).map(|((when, _), v)| format!("<keyframe><when>{when}</when><value>{v}</value></keyframe>")).collect()
    };
    parameter(id, name, range, first, &keys)
}

/// A clip's filter of one FCP7 effect and its parameters (each effect here is its category's own type: motion, audio
/// levels, audio pan).
fn effect(name: &str, id: &str, category: &str, media: &str, params: &[String]) -> String {
    let mut out = format!("<filter><effect><name>{name}</name><effectid>{id}</effectid><effectcategory>{category}</effectcategory><effecttype>{category}</effecttype><mediatype>{media}</mediatype>");
    params.iter().for_each(|p| out.push_str(p));
    out.push_str("</effect></filter>");
    out
}

/// Which editor an XML is for: they read xmeml alike but for crop (read from an XML each exported of a clip it
/// keyframed, and from each one's render of a timeline cropping one picture stored four ways, 11-17). Premiere's crop
/// is four parameters of Basic Motion (leftcrop, topcrop, rightcrop, bottomcrop), cutting the upright picture whatever
/// its file's rotation. Resolve's is FCP7's Crop filter, which cuts the upright picture of a file stored upright, but
/// one stored turned (a phone's take) mirrored: its top and bottom swapped at 90, its left and right at 270, both at
/// 180 (Resolve 21.2.0's Cropping does the same in its Inspector). One file can't be both: an editor reading the other's
/// crop too cuts twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Editor {
    Premiere,
    Resolve,
}

impl Editor {
    pub fn name(self) -> &'static str {
        match self {
            Editor::Premiere => "premiere",
            Editor::Resolve => "resolve",
        }
    }
}

/// The XML for `editor`, and what it can't carry. `dir` is where layers.json is: its movies' paths are relative to it.
pub fn write(l: &Layers, dir: &Path, editor: Editor) -> (String, Vec<String>) {
    let mut notes = Vec::new();
    let fps = l.fps;
    let frames = |secs: f64| f64::round(secs * fps) as i64;
    let path = |k: usize| {
        let p = Path::new(&l.files[k].path);
        if p.is_absolute() { p.to_path_buf() } else { dir.join(p) }
    };
    let name_of = |k: usize| path(k).file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());

    // the tracks bottom first, xmeml's order; sounds apart, each sound track as one mono track a channel, as Premiere
    // writes a stereo clip: its left channel on one track, its right on the next, two clipitems linked
    let video: Vec<_> = l.tracks.iter().rev().filter(|t| t.layer != Layer::Sound).collect();
    let audio: Vec<_> = l.tracks.iter().filter(|t| t.layer == Layer::Sound).collect();
    let channels = |c: &Clip| l.files[c.file].channels.clamp(1, 2) as usize;
    let mut mono: Vec<Vec<(String, &Clip, usize)>> = Vec::new();
    for t in &audio {
        let base = mono.len();
        mono.resize_with(base + t.clips.iter().map(channels).max().unwrap_or(1), Vec::new);
        for c in &t.clips {
            for ch in 1..=channels(c) {
                mono[base + ch - 1].push((if ch == 1 { c.id.clone() } else { format!("{}-{ch}", c.id) }, c, ch));
            }
        }
    }
    // each clipitem's place: (video?, track index from 1, clip index from 1), for links
    let mut at: std::collections::HashMap<String, (bool, usize, usize)> = std::collections::HashMap::new();
    for (ti, t) in video.iter().enumerate() {
        for (ci, c) in t.clips.iter().enumerate() {
            at.insert(c.id.clone(), (true, ti + 1, ci + 1));
        }
    }
    for (ti, m) in mono.iter().enumerate() {
        for (ci, (id, _, _)) in m.iter().enumerate() {
            at.insert(id.clone(), (false, ti + 1, ci + 1));
        }
    }
    let by_id: std::collections::HashMap<&str, &Clip> = l.tracks.iter().flat_map(|t| t.clips.iter()).map(|c| (c.id.as_str(), c)).collect();
    // a clip's clipitems: footage's one, a sound's one a channel
    let items = |c: &Clip| -> Vec<String> { std::iter::once(c.id.clone()).chain((2..=2).map(|ch| format!("{}-{ch}", c.id))).filter(|id| at.contains_key(id)).collect() };
    // every clipitem of a clip and of the clip it's linked to, linked together (the video first), as Premiere links them
    let links = |c: &Clip| -> String {
        let mut all = items(c);
        if let Some(other) = c.link.as_deref().and_then(|o| by_id.get(o)) {
            all.extend(items(other));
        }
        if all.len() < 2 {
            return String::new();
        }
        all.sort_by_key(|id| (!at[id].0, at[id].1));
        all.iter()
            .map(|id| {
                let (v, ti, ci) = at[id];
                format!("<link><linkclipref>{id}</linkclipref><mediatype>{}</mediatype><trackindex>{ti}</trackindex><clipindex>{ci}</clipindex></link>", if v { "video" } else { "audio" })
            })
            .collect()
    };
    // a file is described in full the first time a clip names it, by its id after; its sound a channel at a time
    let mut described = std::collections::HashSet::new();
    let mut file = |k: usize| -> String {
        let id = format!("file-{}", k + 1);
        if !described.insert(k) {
            return format!("<file id=\"{id}\"/>");
        }
        let f = &l.files[k];
        let mut media = String::new();
        if f.kind == FileKind::Video {
            // its size as stored for Premiere (a phone's take on its side, as Premiere's own XML describes it), upright
            // for Resolve, as each editor's Center is read in (see units)
            let (w, h) = if editor == Editor::Premiere && f.rotation % 180 == 90 { (f.height, f.width) } else { (f.width, f.height) };
            let _ = write!(media, "<video><samplecharacteristics>{}<width>{w}</width><height>{h}</height></samplecharacteristics></video>", rate_xml(fps));
        }
        if f.has_audio {
            let n = f.channels.clamp(1, 2);
            for ch in 1..=n {
                let label = match (n, ch) {
                    (1, _) => "mono",
                    (_, 1) => "left",
                    _ => "right",
                };
                let _ = write!(media, "<audio><samplecharacteristics><depth>16</depth><samplerate>{}</samplerate></samplecharacteristics><channelcount>1</channelcount><layout>{}</layout><audiochannel><sourcechannel>{ch}</sourcechannel><channellabel>{label}</channellabel></audiochannel></audio>", f.sample_rate, if n == 1 { "mono" } else { "stereo" });
            }
        }
        format!("<file id=\"{id}\"><name>{}</name><pathurl>{}</pathurl>{}<duration>{}</duration>{}<media>{media}</media></file>", esc(&name_of(k)), pathurl(&path(k)), rate_xml(fps), frames(f.seconds), timecode(fps))
    };
    let clipitem = |id: &str, attrs: &str, c: &Clip, body: &str, file: String| -> String {
        let src_in = frames(c.source_in);
        format!(
            "<clipitem id=\"{id}\"{attrs}><masterclipid>masterclip-{}</masterclipid><name>{}</name><enabled>TRUE</enabled><duration>{}</duration>{}<start>{}</start><end>{}</end><in>{src_in}</in><out>{}</out>{file}{body}{}</clipitem>",
            c.file + 1,
            esc(&name_of(c.file)),
            frames(l.files[c.file].seconds),
            rate_xml(fps),
            c.start,
            c.end(),
            // its own speed (raea's isn't carried, said below): as many source frames as it lasts
            src_in + c.frames,
            links(c)
        )
    };

    // Resolve fits footage of another size to the timeline by its project's Input Scaling before its Scale applies
    let other_size = |f: &crate::model::File| f.kind == FileKind::Video && (f.width, f.height) != (l.width, l.height);
    if editor == Editor::Resolve && l.tracks.iter().any(|t| t.layer == Layer::Footage && t.clips.iter().any(|c| other_size(&l.files[c.file]))) {
        notes.push("Resolve: footage not the timeline's size is placed for its default Input Scaling, \"Scale entire image to fit\" (Project Settings, Image Scaling); with another, set it back or re-place that footage".to_string());
    }
    let mut v = String::new();
    for t in &video {
        let _ = write!(v, "<track>");
        for c in &t.clips {
            let mut body = String::new();
            if (c.speed - 1.0).abs() > 0.001 {
                notes.push(format!("{} (frames {}-{}): plays at {:.2}x in raea; placed at its own speed", name_of(c.file), c.start, c.end() - 1, c.speed));
            }
            match (t.layer, c.r#box) {
                (Layer::Footage, Some(bx)) => {
                    let f = &l.files[c.file];
                    let fit = c.fit.as_deref().unwrap_or("fill");
                    let turned = f.rotation % 180 == 90;
                    let (media, frame) = ((f.width.max(1) as f64, f.height.max(1) as f64), (l.width as f64, l.height as f64));
                    // its place at its first frame, or at each of its keys, each key's when in the source's frames (FCP7
                    // times a clip's filters from its media: its in point is its first frame)
                    let first = [Key { frame: 0, r#box: bx, clip: c.clip, position: c.position, opacity: c.opacity }];
                    let at = if c.keys.is_empty() { &first[..] } else { &c.keys[..] };
                    let src_in = frames(c.source_in);
                    let mut placed: Vec<(i64, [f64; 8])> = Vec::with_capacity(at.len());
                    let mut widest = [1.0f64, 1.0];
                    for k in at {
                        // (a picture object-fit: fill stretches is said with the layers: fitted inside its box here)
                        let (p, _) = plan::picture(&k.r#box, &k.clip, fit, &k.position, media, frame);
                        (widest[0], widest[1]) = (widest[0].max(p.drawn[2]), widest[1].max(p.drawn[3]));
                        let (scale, centre, crop) = motion(editor, &p, media, turned, frame);
                        placed.push((src_in + k.frame, [scale, centre[0], centre[1], crop[0], crop[1], crop[2], crop[3], k.opacity * 100.0]));
                    }
                    // a key a straight line between its neighbours gives to within a seventh of a pixel on the canvas
                    // (the manifest's boxes are to a tenth) and half a percent of opacity (its to a whole percent), in
                    // each value's own units
                    let (whole, unit) = units(editor, media, turned, frame);
                    let px = 0.15;
                    let (cx, cy) = (px / widest[0] * 100.0, px / widest[1] * 100.0);
                    let placed = plan::thin(&placed, &[px / whole * 100.0, px / unit.0, px / unit.1, cx, cx, cy, cy, 0.5]);
                    let scale = keyed("scale", "Scale", Some((0.0, 1000.0)), &placed, |p| format!("{:.4}", p[0]));
                    let centre = keyed("center", "Center", None, &placed, |p| format!("<horiz>{:.6}</horiz><vert>{:.6}</vert>", p[1], p[2]));
                    // the crop's sides as each editor writes them, from motion's [left, right, top, bottom]
                    let cut = placed.iter().any(|(_, p)| p[3..7].iter().any(|&k| k > 0.001));
                    let side = |id: &str, name: &str, i: usize| keyed(id, name, Some((0.0, 100.0)), &placed, |p| format!("{:.4}", p[3 + i]));
                    let mut motion = vec![scale, centre];
                    if editor == Editor::Premiere && cut {
                        motion.extend([("leftcrop", "Left", 0), ("topcrop", "Top", 2), ("rightcrop", "Right", 1), ("bottomcrop", "Bottom", 3)].iter().map(|&(id, n, i)| side(id, n, i)));
                    }
                    body.push_str(&effect("Basic Motion", "basic", "motion", "video", &motion));
                    if editor == Editor::Resolve && cut {
                        // Resolve's sides, mirrored as it mirrors a file stored turned (see Editor)
                        let (lr, tb) = (matches!(f.rotation, 180 | 270), matches!(f.rotation, 90 | 180));
                        let (l, r) = if lr { (1, 0) } else { (0, 1) };
                        let (t, b) = if tb { (3, 2) } else { (2, 3) };
                        let sides: Vec<String> = [("left", l), ("right", r), ("top", t), ("bottom", b)].iter().map(|&(n, i)| side(n, n, i)).collect();
                        body.push_str(&effect("Crop", "crop", "motion", "video", &sides));
                    }
                    if placed.iter().any(|(_, p)| p[7] < 99.9) {
                        body.push_str(&effect("Opacity", "opacity", "motion", "video", &[keyed("opacity", "opacity", Some((0.0, 100.0)), &placed, |p| format!("{:.2}", p[7]))]));
                    }
                }
                // a layer's movie fills the frame, its alpha straight
                _ => body.push_str("<alphatype>straight</alphatype>"),
            }
            let _ = write!(v, "{}", clipitem(&c.id, "", c, &body, file(c.file)));
        }
        let _ = write!(v, "<enabled>TRUE</enabled><locked>FALSE</locked></track>");
    }
    let mut a = String::new();
    for (ti, m) in mono.iter().enumerate() {
        // a stereo pair's left track goes to output channel 1, its right to 2
        // which side of a stereo pair the track is: 1 left, 2 right; none for a mono file's own track
        let side = if m.iter().any(|(_, _, ch)| *ch == 2) { Some(2) } else if mono.get(ti + 1).is_some_and(|n| n.iter().any(|(_, _, ch)| *ch == 2)) { Some(1) } else { None };
        let out = side.map_or(String::new(), |k| format!("<outputchannelindex>{k}</outputchannelindex>"));
        let _ = write!(a, "<track currentExplodedTrackIndex=\"0\" totalExplodedTrackCount=\"1\" premiereTrackType=\"Mono\">");
        for (id, c, ch) in m {
            let gain = 10f64.powf(c.gain_db / 20.0);
            let mut body = format!("<sourcetrack><mediatype>audio</mediatype><trackindex>{ch}</trackindex></sourcetrack>");
            if (gain - 1.0).abs() > 1e-6 {
                body.push_str(&effect("Audio Levels", "audiolevels", "audiolevels", "audio", &[parameter("level", "Level", Some((0.00003162, 31.6228)), &format!("{gain:.6}"), "")]));
            }
            // a stereo file's channel panned to its own side, as FCP7's Audio Pan on the clip (-1 left, 1 right): Premiere
            // and Resolve both import each channel centred without it, and both read it (heard 11-16); a mono file's
            // stays centred
            if channels(c) == 2 {
                body.push_str(&effect("Audio Pan", "audiopan", "audiopan", "audio", &[parameter("pan", "Pan", Some((-1.0, 1.0)), if *ch == 1 { "-1" } else { "1" }, "")]));
            }
            if *ch == 1 && (c.speed - 1.0).abs() > 0.001 {
                notes.push(format!("{} (frames {}-{}): plays at {:.2}x in raea; placed at its own speed", name_of(c.file), c.start, c.end() - 1, c.speed));
            }
            if *ch == 1 && (c.fade_in > 0.0 || c.fade_out > 0.0) {
                notes.push(format!("{} (frames {}-{}): its fades ({:.2} s in, {:.2} s out) aren't carried; set them in the editor", name_of(c.file), c.start, c.end() - 1, c.fade_in, c.fade_out));
            }
            let _ = write!(a, "{}", clipitem(id, " premiereChannelType=\"mono\"", c, &body, file(c.file)));
        }
        let _ = write!(a, "<enabled>TRUE</enabled><locked>FALSE</locked>{out}</track>");
    }
    let format = format!("<format><samplecharacteristics>{}<width>{}</width><height>{}</height><pixelaspectratio>square</pixelaspectratio><anamorphic>FALSE</anamorphic><fielddominance>none</fielddominance></samplecharacteristics></format>", rate_xml(fps), l.width, l.height);
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE xmeml>\n<xmeml version=\"5\"><sequence id=\"sequence-1\" explodedTracks=\"true\"><uuid>{}</uuid><name>{}</name><duration>{}</duration>{}{}<media><video>{format}{v}</video><audio><numOutputChannels>2</numOutputChannels><format><samplecharacteristics><depth>16</depth><samplerate>48000</samplerate></samplecharacteristics></format><outputs><group><index>1</index><numchannels>1</numchannels><downmix>0</downmix><channel><index>1</index></channel></group><group><index>2</index><numchannels>1</numchannels><downmix>0</downmix><channel><index>2</index></channel></group></outputs>{a}</audio></media></sequence></xmeml>\n",
        uuid(&l.name),
        esc(&l.name),
        l.frames,
        rate_xml(fps),
        timecode(fps),
    );
    (xml, notes)
}

//! Screens (`<Screen>` in raea/3d): each frame, every screen's nested composition is laid out in a bevy_ui root
//! of its own at the screen's size (the bevy_ui page's `sync_sub`, src/page.rs), drawn into its own picture, and that
//! picture is copied into the screen's image in raea-render's engine (Engine::copy_screen on the GPU in live play, or handed
//! as RGBA in a render: Engine::set_screen_picture), which draws it as a CRT or a flat panel
//! (engine/render/src/engine/screen.rs). A render's screen equals its play's, and a game on a screen plays live with the
//! player's input (the nested game reads the same useInput).
//!
//! An app screen (`<Screen app="chat.tsx">`, docs/blitz-bevy.feature H12) shows a page of its own instead: the module's
//! component on @raea/ui's react-dom in a Blitz document of its own (raea-runtime-appscreen), stepped once a frame
//! (`step_all`, from the page's sync) and drawn into its picture, which goes to the 3D as a nested screen's does. In
//! live play the pointer reaches it through the 3D (`route`: the camera's ray to the screen, Engine::screen_at): a
//! press on an app screen gives it the keys, which the game then doesn't get, until a press anywhere else or Escape.
//! What reached the app screens each frame goes in the input log (`logged`), and a render of the log gives them the
//! same (`APP_LOG`), so the render draws what was played.

use std::collections::{BTreeMap, HashMap};

use raea_runtime_appscreen::{AppScreen, Input};
use serde_json::{Value, json};

/// What an app's prebuilt bundle is called beside its module (`chat.tsx` -> `chat.tsx.surface.js`, its CSS
/// `chat.tsx.surface.css`): an exported game's (game.rs gather).
pub use raea_runtime_appscreen::{BUNDLED, BUNDLED_CSS, build as build_app};

/// A render's input log, by frame: what reached the app screens in the play it logged (its `apps`), given them again
/// in place of live play's routing. Set by `raea render --input`; None in play.
pub static APP_LOG: std::sync::Mutex<Option<BTreeMap<i64, Value>>> = std::sync::Mutex::new(None);

/// The page's app screens: their documents, and the input live play routed to them for their next step.
#[derive(Default)]
pub(crate) struct Apps {
    /// each app screen's document, by name
    docs: HashMap<String, AppScreen>,
    /// the frame and fps the next step is at (BevyPage::at_frame, set_app_frame)
    pub(crate) frame: Option<(i64, f64)>,
    /// screens whose app didn't open, with the module (said once)
    pub(crate) failed: HashMap<String, String>,
    /// the app screen the keys go to (the one last pressed on), and the keys held for it
    focus: Option<String>,
    keys: Vec<String>,
    /// where the pointer is on an app screen (its picture's pixels) and whether it's down
    pointer: Option<(String, f64, f64, bool)>,
    down: bool,
    /// the wheel's turns this frame (`Wheel:Y DY`), for the app screen under the pointer
    wheel: Vec<String>,
}

/// Live play's input split between the page and its app screens: the pointer where it lands on an app screen (the
/// camera's ray through it, Engine::screen_at; `size` is the frame's, the 3D filling it) is that screen's; a press on
/// one gives it the keys until a press anywhere else or Escape, and while it has them the page gets none. The keys
/// the page gets.
pub fn route(page: &mut crate::BevyPage, engine: &mut raea_render::engine::Engine, keys: &[String], pointer: Option<(f64, f64, bool)>, size: (u32, u32)) -> Vec<String> {
    let apps = page.app_screens();
    let a = page.apps();
    if apps.is_empty() {
        a.focus = None;
        a.pointer = None;
        return keys.to_vec();
    }
    let hit = pointer.and_then(|(x, y, _)| engine.screen_at(x / size.0.max(1) as f64, y / size.1.max(1) as f64));
    let on = hit.and_then(|(name, u, v)| apps.iter().find(|s| s.0 == name).map(|s| (name, u * s.1 as f64, v * s.2 as f64)));
    let down = pointer.is_some_and(|p| p.2);
    if down && !a.down {
        a.focus = on.as_ref().map(|o| o.0.clone());
    }
    a.down = down;
    a.pointer = on.map(|(name, x, y)| (name, x, y, down));
    // the wheel over an app screen scrolls it, and the page doesn't get it
    a.wheel.clear();
    let keys: Vec<String> = match &a.pointer {
        Some(_) => keys.iter().filter(|k| !(k.starts_with("Wheel:Y ") && { a.wheel.push(k.to_string()); true })).cloned().collect(),
        None => keys.to_vec(),
    };
    let keys = &keys[..];
    let (mine, page_keys): (Vec<String>, Vec<String>) = keys.iter().cloned().partition(|k| !(k.starts_with("feed:") || k.starts_with("action:") || k.starts_with("space:")));
    match &a.focus {
        Some(_) if mine.iter().any(|k| k == "Escape") => {
            a.focus = None;
            a.keys.clear();
            page_keys
        }
        Some(_) => {
            a.keys = mine;
            page_keys
        }
        None => {
            a.keys.clear();
            keys.to_vec()
        }
    }
}

/// The app screens that went let go of: their focus, pointer and failures forgotten, so a screen back again opens
/// fresh. `screens`: each app screen now, its name, size and module.
pub(crate) fn retire(a: &mut Apps, screens: &[(String, u32, u32, String)]) {
    let now = |name: &str, app: &str| screens.iter().any(|s| s.0 == name && s.3 == app);
    a.failed.retain(|name, app| now(name, app));
    // a document goes with its screen, or when the screen shows another module or size (opened again fresh)
    a.docs.retain(|name, d| screens.iter().any(|s| &s.0 == name && s.3 == d.module && (s.1, s.2) == d.size));
    let shown = |name: &str| screens.iter().any(|s| s.0 == name);
    if a.focus.as_deref().is_some_and(|f| !shown(f)) {
        a.focus = None;
        a.keys.clear();
    }
    if a.pointer.as_ref().is_some_and(|p| !shown(&p.0)) {
        a.pointer = None;
    }
}

/// Each app screen stepped to the page's frame with its input (`dir`: the composition's folder, its modules' from),
/// its document opened the first frame it's seen; one that won't open is said once and left blank.
pub(crate) fn step_all(a: &mut Apps, screens: &[(String, u32, u32, String)], dir: &std::path::Path) {
    let (f, fps) = a.frame.unwrap_or((0, 30.0));
    if let Some(log) = APP_LOG.lock().unwrap().as_ref() {
        set_logged(a, log.get(&f));
    }
    for (name, w, h, module) in screens {
        if a.failed.contains_key(name) || *w == 0 || *h == 0 {
            continue;
        }
        if !a.docs.contains_key(name) {
            match AppScreen::open(dir, module, *w, *h, f, fps, Some(raea_runtime_view::render_creation())) {
                Ok(d) => {
                    a.docs.insert(name.clone(), d);
                }
                Err(e) => {
                    eprintln!("raea: screen {name:?}: its app {module:?} didn't open: {e}");
                    a.failed.insert(name.clone(), module.clone());
                    continue;
                }
            }
        }
        let input = input_of(a, name);
        if let Some(d) = a.docs.get_mut(name) {
            d.step(f, fps, &input);
        }
    }
}

/// App screen `name`'s input this frame: the pointer when it's on it (with the wheel turned there), the keys while it
/// has the focus (a DOM key's name; the live wire's marked keys, "Mouse:Right" or "Pad0:South", aren't keys to it).
fn input_of(a: &Apps, name: &str) -> Input {
    let on = a.pointer.as_ref().filter(|p| p.0 == name);
    let key = |k: &&String| k.chars().count() == 1 || !k.contains(':');
    Input {
        pointer: on.map(|p| (p.1, p.2, p.3)),
        keys: if a.focus.as_deref() == Some(name) { a.keys.iter().filter(key).cloned().collect() } else { Vec::new() },
        wheel: if on.is_some() { a.wheel.iter().filter_map(|w| w.strip_prefix("Wheel:Y ")?.trim().parse().ok()).collect() } else { Vec::new() },
    }
}

/// What reached the app screens this frame, as the input log keeps it (`apps`): the screen with the focus and its
/// keys, the pointer on a screen (its name, x, y and whether it's down) and the wheel there. None when nothing did.
pub(crate) fn logged(a: &Apps) -> Option<Value> {
    if a.focus.is_none() && a.pointer.is_none() && a.wheel.is_empty() {
        return None;
    }
    Some(json!({ "focus": a.focus, "keys": a.keys, "pointer": a.pointer.as_ref().map(|p| json!([p.0, p.1, p.2, p.3])), "wheel": a.wheel }))
}

/// The app screens' input set from a log's entry for the frame (`logged`'s); none when there's none.
fn set_logged(a: &mut Apps, v: Option<&Value>) {
    let strings = |v: Option<&Value>| v.and_then(Value::as_array).map(|l| l.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default();
    a.focus = v.and_then(|v| v["focus"].as_str()).map(String::from);
    a.keys = strings(v.map(|v| &v["keys"]));
    a.wheel = strings(v.map(|v| &v["wheel"]));
    a.pointer = v.and_then(|v| v["pointer"].as_array()).and_then(|p| Some((p.first()?.as_str()?.to_string(), p.get(1)?.as_f64()?, p.get(2)?.as_f64()?, p.get(3)?.as_bool()?)));
}

/// Each app screen's picture as last drawn, read back: its name, size and RGBA.
pub(crate) fn pictures(a: &mut Apps) -> Result<Vec<(String, u32, u32, Vec<u8>)>, String> {
    a.docs.iter_mut().map(|(name, d)| Ok((name.clone(), d.size.0, d.size.1, d.rgba()?))).collect()
}

/// Each app screen's picture as last drawn: its texture on the shared device.
pub(crate) fn textures(a: &Apps) -> Vec<(String, wgpu::Texture)> {
    a.docs.iter().filter_map(|(name, d)| Some((name.clone(), d.texture()?))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logged_input_comes_back_the_same() {
        let mut a = Apps { focus: Some("kit".into()), keys: vec!["a".into(), "Pad0:South".into()], pointer: Some(("kit".into(), 12.5, 40.0, true)), wheel: vec!["Wheel:Y 120".into()], ..Default::default() };
        let v = logged(&a).unwrap();
        let mut b = Apps::default();
        set_logged(&mut b, Some(&v));
        assert_eq!(logged(&b), Some(v));
        let i = input_of(&b, "kit");
        assert_eq!(i, Input { pointer: Some((12.5, 40.0, true)), keys: vec!["a".into()], wheel: vec![120.0] });
        assert_eq!(input_of(&b, "other"), Input::default());
        set_logged(&mut a, None);
        assert_eq!(logged(&a), None);
    }
}


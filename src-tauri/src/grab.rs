use serde_json::json;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

// CGEventFlags modifier masks (CoreGraphics/CGEventTypes.h)
pub const MASK_SHIFT: u64 = 1 << 17;
pub const MASK_CTRL: u64 = 1 << 18;
pub const MASK_ALT: u64 = 1 << 19;
pub const MASK_CMD: u64 = 1 << 20;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventSourceFlagsState(state_id: u32) -> u64;
}
const COMBINED_SESSION_STATE: u32 = 0;

pub fn combo_mask(names: &[String]) -> u64 {
    names.iter().fold(0, |m, n| {
        m | match n.as_str() {
            "cmd" => MASK_CMD,
            "alt" => MASK_ALT,
            "ctrl" => MASK_CTRL,
            "shift" => MASK_SHIFT,
            _ => 0,
        }
    })
}

pub fn combo_active(flags: u64, mask: u64) -> bool {
    mask != 0 && flags & mask == mask
}

#[cfg(target_os = "macos")]
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_interactive: Option<bool> = None;
        loop {
            let Ok(path) = crate::oryx::config_path(&app) else {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                continue;
            };
            let cfg = crate::config::load(&path);
            let mask = combo_mask(&cfg.grab_combo);
            let flags = unsafe { CGEventSourceFlagsState(COMBINED_SESSION_STATE) };
            let grabbed = combo_active(flags, mask);
            let pinned = app
                .state::<crate::state::HudState>()
                .pinned
                .load(std::sync::atomic::Ordering::SeqCst);
            // The tray "pin" handler and this loop both want a say in whether the
            // overlay accepts mouse events; recomputing from both inputs every
            // tick (rather than only reacting to combo transitions) keeps them
            // from fighting over set_ignore_cursor_events.
            let interactive = grabbed || pinned;
            if last_interactive != Some(interactive) {
                last_interactive = Some(interactive);
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.set_ignore_cursor_events(!interactive);
                }
                let _ = app.emit("grab-mode", json!({ "on": interactive }));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_map_to_cg_flags() {
        assert_eq!(combo_mask(&["cmd".into()]), MASK_CMD);
        assert_eq!(combo_mask(&["cmd".into(), "alt".into()]), MASK_CMD | MASK_ALT);
        assert_eq!(combo_mask(&["bogus".into()]), 0);
    }

    #[test]
    fn combo_requires_all_and_nonempty() {
        let m = MASK_CMD | MASK_ALT;
        assert!(combo_active(MASK_CMD | MASK_ALT | MASK_SHIFT, m));
        assert!(!combo_active(MASK_CMD, m));
        assert!(!combo_active(MASK_CMD, 0));
    }
}

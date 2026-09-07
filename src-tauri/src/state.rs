use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use crate::config::WindowRect;

/// Shared HUD state so the tray "pin" handler and the combo-grab poll loop
/// agree on whether the overlay should currently accept mouse events, and so
/// every config.json read-modify-write (settings, window-drag persistence,
/// layout refresh, position reset) goes through the same lock instead of
/// racing each other.
pub struct HudState {
    pub pinned: AtomicBool,
    pub config_lock: Mutex<()>,
    /// grab_combo only, kept in sync by oryx::update_config on every write,
    /// so grab.rs's 10Hz poll loop (runs forever regardless of keyboard
    /// connectivity) doesn't need to read and fully deserialize config.json
    /// from disk on every tick just to check a value that rarely changes.
    pub grab_combo: Mutex<Vec<String>>,
    pub keyboard_online: AtomicBool,
    pub overlay_hidden: AtomicBool,
    pub hidden_rect: Mutex<Option<WindowRect>>,
}

impl HudState {
    pub fn new() -> Self {
        HudState {
            pinned: AtomicBool::new(false),
            config_lock: Mutex::new(()),
            grab_combo: Mutex::new(Vec::new()),
            keyboard_online: AtomicBool::new(true),
            overlay_hidden: AtomicBool::new(false),
            hidden_rect: Mutex::new(None),
        }
    }
}

impl Default for HudState {
    fn default() -> Self {
        Self::new()
    }
}

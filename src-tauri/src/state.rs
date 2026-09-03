use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

/// Shared HUD state so the tray "pin" handler and the combo-grab poll loop
/// agree on whether the overlay should currently accept mouse events, and so
/// every config.json read-modify-write (settings, window-drag persistence,
/// layout refresh, position reset) goes through the same lock instead of
/// racing each other.
pub struct HudState {
    pub pinned: AtomicBool,
    pub config_lock: Mutex<()>,
}

impl HudState {
    pub fn new() -> Self {
        HudState { pinned: AtomicBool::new(false), config_lock: Mutex::new(()) }
    }
}

impl Default for HudState {
    fn default() -> Self {
        Self::new()
    }
}

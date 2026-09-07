use crate::config::WindowRect;
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU8};
use std::sync::Mutex;
use std::time::Instant;

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
    /// -1 follows HID, 0 forces offline, 1 forces online (debug tray only).
    pub connection_override: AtomicI8,
    pub active_layer: AtomicU8,
    pub macro_recording: AtomicBool,
    pub macro_buffer: Mutex<Vec<u8>>,
    pub macro_last_event: Mutex<Option<Instant>>,
    pub macro_last_down: Mutex<Option<(u8, Instant)>>,
    pub layout_identity: Mutex<Option<crate::layout::LayoutIdentity>>,
    pub layout_lock: tokio::sync::Mutex<()>,
    pub visibility_lock: tokio::sync::Mutex<()>,
    pub overlay_hidden: AtomicBool,
    pub overlay_fully_hidden: AtomicBool,
    pub hidden_rect: Mutex<Option<WindowRect>>,
}

impl HudState {
    pub fn new() -> Self {
        HudState {
            pinned: AtomicBool::new(false),
            config_lock: Mutex::new(()),
            grab_combo: Mutex::new(Vec::new()),
            keyboard_online: AtomicBool::new(false),
            connection_override: AtomicI8::new(-1),
            active_layer: AtomicU8::new(0),
            macro_recording: AtomicBool::new(false),
            macro_buffer: Mutex::new(Vec::new()),
            macro_last_event: Mutex::new(None),
            macro_last_down: Mutex::new(None),
            layout_identity: Mutex::new(None),
            layout_lock: tokio::sync::Mutex::new(()),
            visibility_lock: tokio::sync::Mutex::new(()),
            overlay_hidden: AtomicBool::new(false),
            overlay_fully_hidden: AtomicBool::new(false),
            hidden_rect: Mutex::new(None),
        }
    }
}

impl Default for HudState {
    fn default() -> Self {
        Self::new()
    }
}

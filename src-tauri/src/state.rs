use std::sync::atomic::AtomicBool;

/// Shared HUD state so the tray "pin" handler and the combo-grab poll loop
/// agree on whether the overlay should currently accept mouse events.
pub struct HudState {
    pub pinned: AtomicBool,
}

impl HudState {
    pub fn new() -> Self {
        HudState { pinned: AtomicBool::new(false) }
    }
}

impl Default for HudState {
    fn default() -> Self {
        Self::new()
    }
}

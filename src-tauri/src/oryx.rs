use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

pub fn parse_layout_hash(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').collect();
        let idx = parts.iter().position(|p| *p == "layouts")?;
        let hash = parts.get(idx + 1)?;
        return if hash.chars().all(|c| c.is_ascii_alphanumeric()) && !hash.is_empty() {
            Some((*hash).to_string())
        } else {
            None
        };
    }
    if s.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(s.to_string())
    } else {
        None
    }
}

pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("config.json"))
        .map_err(|e| e.to_string())
}

/// Stable-enough key for "which physical display is this" across launches —
/// the monitor's name (e.g. "Built-in Retina Display") when macOS reports
/// one, falling back to its resolution if not. Two identical external
/// monitor models sharing a name is an accepted, rare edge case for a
/// personal utility app.
pub fn monitor_key(mon: &tauri::Monitor) -> String {
    match mon.name() {
        Some(name) if !name.is_empty() => name.clone(),
        _ => {
            let size = mon.size();
            format!("{}x{}", size.width, size.height)
        }
    }
}

/// A window occupying about 75% of the monitor width, centered on it — the starting
/// point for a monitor the overlay has never been positioned on before.
pub fn default_rect_for_monitor(mon: &tauri::Monitor) -> crate::config::WindowRect {
    let scale = mon.scale_factor();
    let pos = mon.position().to_logical::<f64>(scale);
    let size = mon.size().to_logical::<f64>(scale);
    let ratio = (12.0 + crate::config::Config::default().keyboard_halves_distance) / 6.0;
    let mut w = size.width * 0.75;
    let mut h = w / ratio;
    if h > size.height * 0.65 {
        h = size.height * 0.65;
        w = h * ratio;
    }
    crate::config::WindowRect {
        x: pos.x + (size.width - w) / 2.0,
        y: pos.y + (size.height - h) / 2.0,
        w,
        h,
    }
}

/// Whether a saved rect's origin still falls within a monitor's *current*
/// bounds. The same monitor (same monitor_key) can still go stale — a
/// resolution/scaling change or a rearranged multi-monitor layout moves its
/// live position()/size() without changing its name — so a rect that was
/// valid when saved isn't automatically still valid now.
pub fn rect_fits_monitor(rect: &crate::config::WindowRect, mon: &tauri::Monitor) -> bool {
    let scale = mon.scale_factor();
    let pos = mon.position().to_logical::<f64>(scale);
    let size = mon.size().to_logical::<f64>(scale);
    rect.x >= pos.x
        && rect.x < pos.x + size.width
        && rect.y >= pos.y
        && rect.y < pos.y + size.height
}

/// Position and size a window from a saved rect — the one place both the
/// startup restore and "reset position" apply a WindowRect, so a future
/// change to how that's done (e.g. clamping) only needs to happen once.
pub fn apply_rect(window: &tauri::WebviewWindow, rect: &crate::config::WindowRect) {
    let _ = window.set_position(tauri::LogicalPosition::new(rect.x, rect.y));
    let _ = window.set_size(tauri::LogicalSize::new(rect.w, rect.h));
}

/// Load, mutate and save config.json as one atomic step. config.json is
/// written from several independent places (this module's commands, the
/// window-drag handler in main.rs) with no other coordination between them;
/// routing every read-modify-write through the same app-wide lock is what
/// stops two concurrent writers from silently discarding each other's change.
pub fn update_config<F>(app: &AppHandle, f: F) -> Result<crate::config::Config, String>
where
    F: FnOnce(&mut crate::config::Config),
{
    let state = app.state::<crate::state::HudState>();
    // The lock only ever guards a file read+write, never leaving any
    // in-memory state inconsistent — so a poisoned lock (some other holder
    // panicked mid-critical-section) is safe to recover from rather than
    // treating one panic as "config persistence is now broken forever".
    let _guard = state
        .config_lock
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = config_path(app)?;
    let mut cfg = crate::config::load(&path);
    f(&mut cfg);
    crate::config::save(&path, &cfg).map_err(|e| e.to_string())?;
    // Most update_config callers (e.g. the window-drag handler, which fires
    // rapidly during a drag) never touch grab_combo — skip the lock/clone
    // entirely unless it actually changed, rather than rewriting an
    // identical Vec<String> on every unrelated write.
    let mut cached_combo = state
        .grab_combo
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if *cached_combo != cfg.grab_combo {
        *cached_combo = cfg.grab_combo.clone();
    }
    drop(cached_combo);
    state
        .pinned
        .store(cfg.overlay_pinned, std::sync::atomic::Ordering::SeqCst);
    Ok(cfg)
}

/// One-time migration for the io.jonathanlaf.voyagerhud -> io.jonathanlaf.layerhud
/// identifier rename: if this machine has a config saved under the old
/// identifier's app-support directory and none yet under the new one, copy it
/// over so existing settings aren't silently reset to defaults on upgrade.
pub fn migrate_legacy_identifier(app: &AppHandle) -> Result<(), String> {
    let new_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let Some(home) = std::env::var_os("HOME") else {
        return Ok(());
    };
    let old_dir = PathBuf::from(home).join("Library/Application Support/io.jonathanlaf.voyagerhud");
    if !old_dir.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(&new_dir).map_err(|e| e.to_string())?;
    // Route config.json through load()/save() rather than a raw fs::copy, so
    // the migrated file gets the same clamping and atomic write-then-rename
    // every other writer gets — this runs before grab::spawn/tray::build
    // start so nothing else touches config.json yet, but there's no reason
    // for this to be the one path that doesn't go through the shared helpers.
    if !new_dir.join("config.json").exists() && old_dir.join("config.json").exists() {
        let legacy_cfg = crate::config::load(&old_dir.join("config.json"));
        crate::config::save(&new_dir.join("config.json"), &legacy_cfg)
            .map_err(|e| e.to_string())?;
    }
    if !new_dir.join("layout.json").exists() && old_dir.join("layout.json").exists() {
        let _ = std::fs::copy(old_dir.join("layout.json"), new_dir.join("layout.json"));
    }
    Ok(())
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> Result<crate::config::Config, String> {
    let cfg_path = config_path(&app)?;
    Ok(crate::config::load(&cfg_path))
}

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn set_settings_title(app: AppHandle, title: String) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "Settings window is not open".to_string())?;
    window
        .set_title(&format!("KeyAura Settings — {title}"))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_config(app: AppHandle, mut config: crate::config::Config) -> Result<(), String> {
    config.clamp();
    let merged = update_config(&app, move |cfg| cfg.apply_preferences(config))?;
    app.emit("config-changed", &merged)
        .map_err(|e| e.to_string())
}

fn clear_window_position(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("overlay") {
        if let Ok(Some(mon)) = w.current_monitor() {
            let key = monitor_key(&mon);
            update_config(app, {
                let key = key.clone();
                move |cfg| {
                    cfg.window_by_monitor.remove(&key);
                    cfg.last_monitor = Some(key.clone());
                }
            })?;
            // Reset means "start over on this monitor" — the same 30%-
            // centered rect a monitor gets the first time it's ever seen.
            apply_rect(&w, &default_rect_for_monitor(&mon));
        } else {
            let _ = w.center();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn align_window(app: AppHandle, axis: String) -> Result<(), String> {
    let state = app.state::<crate::state::HudState>();
    let _guard = state.visibility_lock.lock().await;
    restore_overlay(&app)?;
    let Some(w) = app.get_webview_window("overlay") else {
        return Err("overlay window is not available".into());
    };
    let Some(mon) = w.current_monitor().map_err(|e| e.to_string())? else {
        return Err("no monitor found for overlay window".into());
    };
    let scale = mon.scale_factor();
    let mon_pos = mon.position().to_logical::<f64>(scale);
    let mon_size = mon.size().to_logical::<f64>(scale);
    let pos = w
        .outer_position()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let size = w
        .inner_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let mut x = pos.x;
    let mut y = pos.y;
    if axis == "horizontal" {
        x = mon_pos.x + (mon_size.width - size.width) / 2.0;
    }
    if axis == "vertical" {
        y = mon_pos.y + (mon_size.height - size.height) / 2.0;
    }
    if axis == "top" {
        y = mon_pos.y;
    }
    if axis == "bottom" {
        y = mon_pos.y + mon_size.height - size.height;
    }
    if !matches!(axis.as_str(), "horizontal" | "vertical" | "top" | "bottom") {
        return Err(format!("unknown alignment: {axis}"));
    }
    let rect = crate::config::WindowRect {
        x,
        y,
        w: size.width,
        h: size.height,
    };
    apply_rect(&w, &rect);
    let key = monitor_key(&mon);
    update_config(&app, move |cfg| {
        cfg.window_by_monitor.insert(key.clone(), rect);
        cfg.last_monitor = Some(key);
    })?;
    Ok(())
}

#[tauri::command]
pub async fn reset_window_positions(app: AppHandle) -> Result<(), String> {
    let state = app.state::<crate::state::HudState>();
    let _guard = state.visibility_lock.lock().await;
    restore_overlay(&app)?;
    update_config(&app, |cfg| {
        cfg.window_by_monitor.clear();
        cfg.last_monitor = None;
        cfg.keyboard_halves_distance = crate::config::Config::default().keyboard_halves_distance;
        cfg.keyboard_halves_rotation = crate::config::Config::default().keyboard_halves_rotation;
    })?;
    clear_window_position(&app)?;
    let cfg = crate::config::load(&config_path(&app)?);
    app.emit("config-changed", cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn recalculate_window_geometry(app: AppHandle) -> Result<(), String> {
    let state = app.state::<crate::state::HudState>();
    let _guard = state.visibility_lock.lock().await;
    restore_overlay(&app)?;
    let Some(w) = app.get_webview_window("overlay") else {
        return Ok(());
    };
    let scale = w.scale_factor().map_err(|e| e.to_string())?;
    let pos = w
        .outer_position()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let size = w
        .inner_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let path = config_path(&app)?;
    let distance = crate::config::load(&path).keyboard_halves_distance;
    let ratio = (12.0 + distance) / 6.0;
    let new_height = (size.width / ratio).max(120.0);
    let center_y = pos.y + size.height / 2.0;
    let new_y = center_y - new_height / 2.0;
    let rect = crate::config::WindowRect {
        x: pos.x,
        y: new_y,
        w: size.width,
        h: new_height,
    };
    apply_rect(&w, &rect);
    Ok(())
}

#[tauri::command]
pub fn is_overlay_pinned(app: AppHandle) -> bool {
    app.state::<crate::state::HudState>()
        .pinned
        .load(std::sync::atomic::Ordering::SeqCst)
}

// Caller holds visibility_lock. Keep the saved rect until native restoration succeeds.
fn restore_overlay(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<crate::state::HudState>();
    let rect = state
        .hidden_rect
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    if let Some(rect) = rect {
        let window = app
            .get_webview_window("overlay")
            .ok_or("Overlay window is unavailable")?;
        window
            .set_position(tauri::LogicalPosition::new(rect.x, rect.y))
            .map_err(|e| e.to_string())?;
        window
            .set_size(tauri::LogicalSize::new(rect.w, rect.h))
            .map_err(|e| e.to_string())?;
        *state.hidden_rect.lock().unwrap_or_else(|p| p.into_inner()) = None;
        state
            .overlay_fully_hidden
            .store(false, std::sync::atomic::Ordering::SeqCst);
        state
            .overlay_hidden
            .store(false, std::sync::atomic::Ordering::SeqCst);
        app.emit("overlay-visibility", serde_json::json!({ "hidden": false }))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn animate_position(
    window: &tauri::WebviewWindow,
    from: (f64, f64),
    to: (f64, f64),
    duration_ms: f64,
    progress: impl Fn(f64),
) -> Result<(), String> {
    if duration_ms <= 0.0 {
        return window
            .set_position(tauri::LogicalPosition::new(to.0, to.1))
            .map_err(|e| e.to_string());
    }
    let steps = ((duration_ms / 16.0).round() as u32).max(1);
    for step in 1..=steps {
        let t = step as f64 / steps as f64;
        let eased = 1.0 - (1.0 - t).powi(3);
        let x = from.0 + (to.0 - from.0) * eased;
        let y = from.1 + (to.1 - from.1) * eased;
        progress(eased);
        window
            .set_position(tauri::LogicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn toggle_overlay_visibility(app: AppHandle) -> Result<(), String> {
    let state = app.state::<crate::state::HudState>();
    let Some(window) = app.get_webview_window("overlay") else {
        return Ok(());
    };
    let _guard = state.visibility_lock.lock().await;
    let currently_hidden = state
        .overlay_hidden
        .load(std::sync::atomic::Ordering::SeqCst);
    if currently_hidden {
        let Some(rect) = state
            .hidden_rect
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
        else {
            return Ok(());
        };
        let cfg = crate::config::load(&config_path(&app)?);
        let pos = window.outer_position().map_err(|e| e.to_string())?;
        let scale = window.scale_factor().map_err(|e| e.to_string())?;
        let from = pos.to_logical::<f64>(scale);
        animate_position(
            &window,
            (from.x, from.y),
            (rect.x, rect.y),
            cfg.hide_animation_ms,
            {
                let app = app.clone();
                let side = cfg.hide_side.clone();
                move |movement| {
                    let _ = app.emit(
                        "overlay-visibility",
                        serde_json::json!({
                            "hidden": true, "side": side, "reveal": cfg.hide_reveal,
                            "progress": 1.0 - movement,
                        }),
                    );
                }
            },
        )
        .await?;
        restore_overlay(&app)?;
        return Ok(());
    }
    let Some(mon) = window.current_monitor().map_err(|e| e.to_string())? else {
        return Err("no monitor found".into());
    };
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let pos = window
        .outer_position()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let size = window
        .inner_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let rect = crate::config::WindowRect {
        x: pos.x,
        y: pos.y,
        w: size.width,
        h: size.height,
    };
    let cfg = crate::config::load(&config_path(&app)?);
    let mon_pos = mon.position().to_logical::<f64>(scale);
    let mon_size = mon.size().to_logical::<f64>(scale);
    let (target_x, target_y) = match cfg.hide_side.as_str() {
        "left" => (mon_pos.x, pos.y),
        "top" => (pos.x, mon_pos.y),
        "bottom" => (pos.x, mon_pos.y + mon_size.height - size.height),
        _ => (mon_pos.x + mon_size.width - size.width, pos.y),
    };
    *state.hidden_rect.lock().unwrap_or_else(|p| p.into_inner()) = Some(rect);
    state
        .overlay_hidden
        .store(true, std::sync::atomic::Ordering::SeqCst);
    state
        .overlay_fully_hidden
        .store(cfg.hide_reveal == 0.0, std::sync::atomic::Ordering::SeqCst);
    let _ = app.emit(
        "overlay-visibility",
        serde_json::json!({
            "hidden": true,
            "side": cfg.hide_side,
            "reveal": cfg.hide_reveal,
        }),
    );
    if let Err(error) = animate_position(
        &window,
        (pos.x, pos.y),
        (target_x, target_y),
        cfg.hide_animation_ms,
        {
            let app = app.clone();
            let side = cfg.hide_side.clone();
            move |movement| {
                let _ = app.emit(
                    "overlay-visibility",
                    serde_json::json!({
                        "hidden": true, "side": side, "reveal": cfg.hide_reveal,
                        "progress": movement,
                    }),
                );
            }
        },
    )
    .await
    {
        restore_overlay(&app)?;
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub fn export_config(app: AppHandle) -> Result<String, String> {
    let cfg = crate::config::load(&config_path(&app)?);
    let dir = app.path().download_dir().map_err(|e| e.to_string())?;
    crate::config::export_to_dir(&dir, &cfg)
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_config(
    app: AppHandle,
    contents: String,
) -> Result<crate::config::Config, String> {
    let mut imported: crate::config::Config =
        serde_json::from_str(&contents).map_err(|e| format!("Invalid settings JSON: {e}"))?;
    imported.clamp();
    let state = app.state::<crate::state::HudState>();
    let _guard = state.visibility_lock.lock().await;
    restore_overlay(&app)?;
    let result = update_config(&app, |cfg| *cfg = imported.clone())?;
    let _ = app.emit("config-changed", result.clone());
    Ok(result)
}

#[tauri::command]
pub async fn reset_config(app: AppHandle) -> Result<crate::config::Config, String> {
    let state = app.state::<crate::state::HudState>();
    let _guard = state.visibility_lock.lock().await;
    restore_overlay(&app)?;
    let result = update_config(&app, |cfg| *cfg = crate::config::Config::default())?;
    if let Some(window) = app.get_webview_window("overlay") {
        if let Ok(Some(mon)) = window.current_monitor() {
            apply_rect(&window, &default_rect_for_monitor(&mon));
        }
    }
    let _ = app.emit("config-changed", result.clone());
    Ok(result)
}

#[tauri::command]
pub fn get_keyboard_status(app: AppHandle) -> serde_json::Value {
    let state = app.state::<crate::state::HudState>();
    let override_state = state
        .connection_override
        .load(std::sync::atomic::Ordering::SeqCst);
    let online = match override_state {
        0 => false,
        1 => true,
        _ => state
            .keyboard_online
            .load(std::sync::atomic::Ordering::SeqCst),
    };
    serde_json::json!({
        "online": online,
        "layer": state.active_layer.load(std::sync::atomic::Ordering::SeqCst),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_layout_hash;

    #[test]
    fn parses_full_url() {
        assert_eq!(
            parse_layout_hash("https://configure.zsa.io/voyager/layouts/Br3gO/latest/0"),
            Some("Br3gO".into())
        );
    }

    #[test]
    fn parses_url_without_revision() {
        assert_eq!(
            parse_layout_hash("configure.zsa.io/voyager/layouts/gLwvw"),
            Some("gLwvw".into())
        );
    }

    #[test]
    fn parses_bare_hash() {
        assert_eq!(parse_layout_hash("Br3gO"), Some("Br3gO".into()));
        assert_eq!(parse_layout_hash("  Br3gO "), Some("Br3gO".into()));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_layout_hash(""), None);
        assert_eq!(parse_layout_hash("https://example.com/foo"), None);
        assert_eq!(parse_layout_hash("has spaces"), None);
    }
}

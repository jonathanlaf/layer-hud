use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

const GRAPHQL_URL: &str = "https://oryx.zsa.io/graphql";
const QUERY: &str = "query getLayout($hashId: String!, $revisionId: String!, $geometry: String) { layout(hashId: $hashId, revisionId: $revisionId, geometry: $geometry) { title revision { title config layers { position title keys } } } }";

#[derive(Clone)]
enum FetchError {
    NotFound(String),
    Transport(String),
}

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

fn cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("layout.json"))
        .map_err(|e| e.to_string())
}

fn legacy_cache_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support/io.jonathanlaf.voyagerhud/layout.json"))
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

/// A window occupying 30% of the monitor, centered on it — the starting
/// point for a monitor the overlay has never been positioned on before.
pub fn default_rect_for_monitor(mon: &tauri::Monitor) -> crate::config::WindowRect {
    let scale = mon.scale_factor();
    let pos = mon.position().to_logical::<f64>(scale);
    let size = mon.size().to_logical::<f64>(scale);
    let w = size.width * 0.3;
    let h = size.height * 0.3;
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
    rect.x >= pos.x && rect.x < pos.x + size.width && rect.y >= pos.y && rect.y < pos.y + size.height
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
    let _guard = state.config_lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = config_path(app)?;
    let mut cfg = crate::config::load(&path);
    f(&mut cfg);
    crate::config::save(&path, &cfg).map_err(|e| e.to_string())?;
    // Most update_config callers (e.g. the window-drag handler, which fires
    // rapidly during a drag) never touch grab_combo — skip the lock/clone
    // entirely unless it actually changed, rather than rewriting an
    // identical Vec<String> on every unrelated write.
    let mut cached_combo = state.grab_combo.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if *cached_combo != cfg.grab_combo {
        *cached_combo = cfg.grab_combo.clone();
    }
    drop(cached_combo);
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
        crate::config::save(&new_dir.join("config.json"), &legacy_cfg).map_err(|e| e.to_string())?;
    }
    if !new_dir.join("layout.json").exists() && old_dir.join("layout.json").exists() {
        let _ = std::fs::copy(old_dir.join("layout.json"), new_dir.join("layout.json"));
    }
    Ok(())
}

async fn fetch_from_oryx(hash: &str) -> Result<Value, FetchError> {
    let body = json!({
        "query": QUERY,
        "variables": { "hashId": hash, "revisionId": "latest", "geometry": "voyager" }
    });
    let resp: Value = reqwest::Client::new()
        .post(GRAPHQL_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| FetchError::Transport(format!("network: {e}")))?
        .json()
        .await
        .map_err(|e| FetchError::Transport(format!("bad response: {e}")))?;
    if resp.pointer("/data/layout").map(|v| v.is_null()).unwrap_or(true) {
        return Err(FetchError::NotFound(format!("layout '{hash}' not found on Oryx")));
    }
    Ok(resp)
}

#[tauri::command]
pub async fn refresh_layout(app: AppHandle, url: String) -> Result<Value, String> {
    let hash = parse_layout_hash(&url).ok_or("not a valid Oryx layout URL or hash")?;
    let cache = cache_path(&app)?;
    match fetch_from_oryx(&hash).await {
        Ok(v) => {
            update_config(&app, |c| {
                c.oryx_url = hash;
                c.last_refresh = Some(chrono_free_now());
            })?;
            let json_str = serde_json::to_string(&v).map_err(|e| e.to_string())?;
            std::fs::write(&cache, json_str).map_err(|e| e.to_string())?;
            let _ = { use tauri::Emitter; app.emit("layout-refreshed", serde_json::json!({})) };
            Ok(v)
        }
        Err(FetchError::NotFound(e)) => {
            Err(e)
        }
        Err(FetchError::Transport(e)) => {
            let cached = std::fs::read_to_string(&cache)
                .or_else(|_| legacy_cache_path().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no legacy cache")).and_then(std::fs::read_to_string))
                .map_err(|_| e.clone())?;
            let mut v: Value = serde_json::from_str(&cached).map_err(|_| e)?;
            if let Value::Object(ref mut obj) = v {
                obj.insert("stale".to_string(), json!(true));
            }
            Ok(v)
        }
    }
}

#[tauri::command]
pub async fn load_layout(app: AppHandle) -> Result<Value, String> {
    let cache = cache_path(&app)?;
    let cached = std::fs::read_to_string(&cache).or_else(|_| {
        legacy_cache_path().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no legacy cache")).and_then(std::fs::read_to_string)
    });
    if let Ok(s) = cached {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            return Ok(v);
        }
    }
    let hash = crate::config::load(&config_path(&app)?).oryx_url;
    refresh_layout(app, hash).await
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> Result<crate::config::Config, String> {
    let cfg_path = config_path(&app)?;
    Ok(crate::config::load(&cfg_path))
}

#[tauri::command]
pub fn set_config(app: AppHandle, mut config: crate::config::Config) -> Result<(), String> {
    // Tauri binds this arg by parameter name to the JSON key the frontend
    // sends (`invoke('set_config', { config: cfg })` in settings.js) — it
    // must stay named `config`, not renamed for internal clarity, or every
    // call is rejected before the body ever runs.
    config.clamp();
    // window/oryx_url/last_refresh are owned by other writers (the overlay's
    // drag/resize handler, and refresh_layout — which the tray's "Refresh
    // layout" item can trigger with no Settings window open at all), not by
    // this command; keep whatever's already on disk for them rather than
    // whatever the Settings page happened to have in memory when it sent this.
    // NOTE: this means these 3 fields can never be changed via set_config —
    // if you add a settings.js `bind()` for e.g. `oryx_url`, it will send
    // fine and silently do nothing forever. Route new oryx_url changes
    // through refresh_layout instead, the way the "Fetch" button already does.
    let merged = update_config(&app, move |cfg| {
        let window_by_monitor = std::mem::take(&mut cfg.window_by_monitor);
        let oryx_url = std::mem::take(&mut cfg.oryx_url);
        let last_refresh = cfg.last_refresh.take();
        *cfg = config;
        cfg.window_by_monitor = window_by_monitor;
        cfg.oryx_url = oryx_url;
        cfg.last_refresh = last_refresh;
    })?;
    use tauri::Emitter;
    app.emit("config-changed", &merged).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_window_position(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("overlay") {
        if let Ok(Some(mon)) = w.current_monitor() {
            let key = monitor_key(&mon);
            update_config(&app, {
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
pub fn align_window(app: AppHandle, axis: String) -> Result<(), String> {
    let Some(w) = app.get_webview_window("overlay") else {
        return Err("overlay window is not available".into());
    };
    let Some(mon) = w.current_monitor().map_err(|e| e.to_string())? else {
        return Err("no monitor found for overlay window".into());
    };
    let scale = mon.scale_factor();
    let mon_pos = mon.position().to_logical::<f64>(scale);
    let mon_size = mon.size().to_logical::<f64>(scale);
    let pos = w.outer_position().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    let size = w.inner_size().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    let mut x = pos.x;
    let mut y = pos.y;
    if axis == "horizontal" || axis == "both" {
        x = mon_pos.x + (mon_size.width - size.width) / 2.0;
    }
    if axis == "vertical" || axis == "both" {
        y = mon_pos.y + (mon_size.height - size.height) / 2.0;
    }
    if !matches!(axis.as_str(), "horizontal" | "vertical" | "both") {
        return Err(format!("unknown alignment: {axis}"));
    }
    let rect = crate::config::WindowRect { x, y, w: size.width, h: size.height };
    apply_rect(&w, &rect);
    let key = monitor_key(&mon);
    update_config(&app, move |cfg| {
        cfg.window_by_monitor.insert(key.clone(), rect);
        cfg.last_monitor = Some(key);
    })?;
    Ok(())
}

#[tauri::command]
pub fn reset_window_positions(app: AppHandle) -> Result<(), String> {
    update_config(&app, |cfg| {
        cfg.window_by_monitor.clear();
        cfg.last_monitor = None;
    })?;
    clear_window_position(app)
}

#[tauri::command]
pub fn recalculate_window_geometry(app: AppHandle) -> Result<(), String> {
    let Some(w) = app.get_webview_window("overlay") else {
        return Ok(());
    };
    let scale = w.scale_factor().map_err(|e| e.to_string())?;
    let pos = w.outer_position().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    let size = w.inner_size().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    let path = config_path(&app)?;
    let distance = crate::config::load(&path).keyboard_halves_distance;
    let ratio = (12.0 + distance) / 6.0;
    let new_height = (size.width / ratio).max(120.0);
    let center_y = pos.y + size.height / 2.0;
    let new_y = center_y - new_height / 2.0;
    let rect = crate::config::WindowRect { x: pos.x, y: new_y, w: size.width, h: new_height };
    apply_rect(&w, &rect);
    Ok(())
}

#[tauri::command]
pub fn is_overlay_pinned(app: AppHandle) -> bool {
    app.state::<crate::state::HudState>()
        .pinned
        .load(std::sync::atomic::Ordering::SeqCst)
}

fn animate_position(window: &tauri::WebviewWindow, from: (f64, f64), to: (f64, f64), duration_ms: f64) {
    let steps = ((duration_ms / 16.0).round() as u32).max(1);
    for step in 1..=steps {
        let t = step as f64 / steps as f64;
        let eased = 1.0 - (1.0 - t).powi(3);
        let x = from.0 + (to.0 - from.0) * eased;
        let y = from.1 + (to.1 - from.1) * eased;
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

#[tauri::command]
pub fn toggle_overlay_visibility(app: AppHandle) -> Result<(), String> {
    let state = app.state::<crate::state::HudState>();
    let Some(window) = app.get_webview_window("overlay") else { return Ok(()); };
    let currently_hidden = state.overlay_hidden.load(std::sync::atomic::Ordering::SeqCst);
    if currently_hidden {
        let Some(rect) = state.hidden_rect.lock().unwrap_or_else(|p| p.into_inner()).take() else { return Ok(()); };
        let cfg = crate::config::load(&config_path(&app)?);
        let pos = window.outer_position().map_err(|e| e.to_string())?;
        let scale = window.scale_factor().map_err(|e| e.to_string())?;
        let from = pos.to_logical::<f64>(scale);
        state.overlay_hidden.store(true, std::sync::atomic::Ordering::SeqCst);
        animate_position(&window, (from.x, from.y), (rect.x, rect.y), cfg.hide_animation_ms);
        state.overlay_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = app.emit("overlay-visibility", serde_json::json!({ "hidden": false }));
        return Ok(());
    }
    let Some(mon) = window.current_monitor().map_err(|e| e.to_string())? else { return Err("no monitor found".into()); };
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let pos = window.outer_position().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    let size = window.inner_size().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    let rect = crate::config::WindowRect { x: pos.x, y: pos.y, w: size.width, h: size.height };
    let cfg = crate::config::load(&config_path(&app)?);
    let mon_pos = mon.position().to_logical::<f64>(scale);
    let mon_size = mon.size().to_logical::<f64>(scale);
    let minimum_peek = 64.0;
    let reveal_x = if cfg.hide_reveal == 0.0 { 0.0 } else { (size.width * cfg.hide_reveal).max(minimum_peek) };
    let reveal_y = if cfg.hide_reveal == 0.0 { 0.0 } else { (size.height * cfg.hide_reveal).max(minimum_peek) };
    let (target_x, target_y) = match cfg.hide_side.as_str() {
        "left" => (mon_pos.x, pos.y),
        "top" => (pos.x, mon_pos.y),
        "bottom" => (pos.x, mon_pos.y + mon_size.height - size.height),
        _ => (mon_pos.x + mon_size.width - size.width, pos.y),
    };
    *state.hidden_rect.lock().unwrap_or_else(|p| p.into_inner()) = Some(rect);
    state.overlay_hidden.store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = app.emit("overlay-visibility", serde_json::json!({
        "hidden": true,
        "side": cfg.hide_side,
        "reveal": (reveal_x / size.width).max(reveal_y / size.height),
    }));
    animate_position(&window, (pos.x, pos.y), (target_x, target_y), cfg.hide_animation_ms);
    Ok(())
}

#[tauri::command]
pub fn export_config(app: AppHandle) -> Result<String, String> {
    let cfg = crate::config::load(&config_path(&app)?);
    serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_config(app: AppHandle, contents: String) -> Result<crate::config::Config, String> {
    let mut imported: crate::config::Config = serde_json::from_str(&contents)
        .map_err(|e| format!("Invalid settings JSON: {e}"))?;
    imported.clamp();
    let result = update_config(&app, |cfg| *cfg = imported.clone())?;
    let _ = app.emit("config-changed", result.clone());
    Ok(result)
}

#[tauri::command]
pub fn reset_config(app: AppHandle) -> Result<crate::config::Config, String> {
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
pub fn is_keymapp_online(app: AppHandle) -> bool {
    app.state::<crate::state::HudState>().keymapp_online.load(std::sync::atomic::Ordering::SeqCst)
}

fn chrono_free_now() -> String {
    // ISO-ish timestamp without a chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{now}")
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

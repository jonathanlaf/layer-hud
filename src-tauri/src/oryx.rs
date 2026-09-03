use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

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
    Ok(cfg)
}

/// One-time migration for the io.jonathanlaf.voyagerhud -> io.jonathanlaf.layerhud
/// identifier rename: if this machine has a config saved under the old
/// identifier's app-support directory and none yet under the new one, copy it
/// over so existing settings aren't silently reset to defaults on upgrade.
pub fn migrate_legacy_identifier(app: &AppHandle) -> Result<(), String> {
    let new_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    if new_dir.join("config.json").exists() {
        return Ok(());
    }
    let Some(home) = std::env::var_os("HOME") else {
        return Ok(());
    };
    let old_dir = PathBuf::from(home).join("Library/Application Support/io.jonathanlaf.voyagerhud");
    if !old_dir.join("config.json").exists() {
        return Ok(());
    }
    std::fs::create_dir_all(&new_dir).map_err(|e| e.to_string())?;
    // Route config.json through load()/save() rather than a raw fs::copy, so
    // the migrated file gets the same clamping and atomic write-then-rename
    // every other writer gets — this runs before grab::spawn/tray::build
    // start so nothing else touches config.json yet, but there's no reason
    // for this to be the one path that doesn't go through the shared helpers.
    let legacy_cfg = crate::config::load(&old_dir.join("config.json"));
    crate::config::save(&new_dir.join("config.json"), &legacy_cfg).map_err(|e| e.to_string())?;
    if old_dir.join("layout.json").exists() {
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
            let cached = std::fs::read_to_string(&cache).map_err(|_| e.clone())?;
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
    if let Ok(s) = std::fs::read_to_string(&cache) {
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
        let window = cfg.window.take();
        let oryx_url = std::mem::take(&mut cfg.oryx_url);
        let last_refresh = cfg.last_refresh.take();
        *cfg = config;
        cfg.window = window;
        cfg.oryx_url = oryx_url;
        cfg.last_refresh = last_refresh;
    })?;
    use tauri::Emitter;
    app.emit("config-changed", &merged).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_window_position(app: AppHandle) -> Result<(), String> {
    update_config(&app, |cfg| cfg.window = None)?;
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.center();
    }
    Ok(())
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

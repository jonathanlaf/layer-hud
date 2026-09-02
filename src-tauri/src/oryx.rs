use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const GRAPHQL_URL: &str = "https://oryx.zsa.io/graphql";
const QUERY: &str = "query getLayout($hashId: String!, $revisionId: String!, $geometry: String) { layout(hashId: $hashId, revisionId: $revisionId, geometry: $geometry) { title revision { title config layers { position title keys } } } }";

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

pub fn config_path(app: &AppHandle) -> PathBuf {
    app.path().app_config_dir().expect("config dir").join("config.json")
}

fn cache_path(app: &AppHandle) -> PathBuf {
    app.path().app_config_dir().expect("config dir").join("layout.json")
}

async fn fetch_from_oryx(hash: &str) -> Result<Value, String> {
    let body = json!({
        "query": QUERY,
        "variables": { "hashId": hash, "revisionId": "latest", "geometry": "voyager" }
    });
    let resp: Value = reqwest::Client::new()
        .post(GRAPHQL_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?
        .json()
        .await
        .map_err(|e| format!("bad response: {e}"))?;
    if resp.pointer("/data/layout").map(|v| v.is_null()).unwrap_or(true) {
        return Err(format!("layout '{hash}' not found on Oryx"));
    }
    Ok(resp)
}

#[tauri::command]
pub async fn refresh_layout(app: AppHandle, url: String) -> Result<Value, String> {
    let hash = parse_layout_hash(&url).ok_or("not a valid Oryx layout URL or hash")?;
    let cache = cache_path(&app);
    match fetch_from_oryx(&hash).await {
        Ok(v) => {
            let mut c = crate::config::load(&config_path(&app));
            c.oryx_url = hash;
            c.last_refresh = Some(chrono_free_now());
            crate::config::save(&config_path(&app), &c).map_err(|e| e.to_string())?;
            std::fs::write(&cache, serde_json::to_string(&v).unwrap()).map_err(|e| e.to_string())?;
            Ok(v)
        }
        Err(e) => {
            let cached = std::fs::read_to_string(&cache).map_err(|_| e.clone())?;
            let mut v: Value = serde_json::from_str(&cached).map_err(|_| e)?;
            v["stale"] = json!(true);
            Ok(v)
        }
    }
}

#[tauri::command]
pub async fn load_layout(app: AppHandle) -> Result<Value, String> {
    let cache = cache_path(&app);
    if let Ok(s) = std::fs::read_to_string(&cache) {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            return Ok(v);
        }
    }
    let hash = crate::config::load(&config_path(&app)).oryx_url;
    refresh_layout(app, hash).await
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> crate::config::Config {
    crate::config::load(&config_path(&app))
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

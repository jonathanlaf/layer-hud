//! Oryx layout retrieval. HID identifies the flashed revision; Oryx supplies its labels.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutIdentity {
    pub layout: String,
    pub revision: String,
}

impl LayoutIdentity {
    pub fn new(layout: &str, revision: &str) -> Option<Self> {
        let valid = |s: &str| {
            !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric())
        };
        (valid(layout) && valid(revision)).then(|| Self {
            layout: layout.into(),
            revision: revision.into(),
        })
    }

    pub fn from_serial(serial: &str) -> Option<Self> {
        let (layout, revision) = serial.split_once('/')?;
        Self::new(layout, revision)
    }
}

const GRAPHQL_URL: &str = "https://oryx.zsa.io/graphql";
const QUERY: &str = "query getLayout($hashId: String!, $revisionId: String!, $geometry: String) { layout(hashId: $hashId, revisionId: $revisionId, geometry: $geometry) { title revision { title config layers { position title keys } } } }";

fn request_body(identity: &LayoutIdentity) -> Value {
    json!({ "query": QUERY, "variables": {
        "hashId": identity.layout, "revisionId": identity.revision, "geometry": "voyager"
    } })
}

fn validate_layout(value: &Value) -> Result<(), String> {
    let layers = value
        .pointer("/data/layout/revision/layers")
        .and_then(Value::as_array)
        .filter(|layers| !layers.is_empty())
        .ok_or("Layout has no layers")?;
    let mut positions = std::collections::HashSet::new();
    for layer in layers {
        let position = layer
            .get("position")
            .and_then(Value::as_u64)
            .ok_or("Invalid layer position")?;
        if !positions.insert(position) {
            return Err("Duplicate layer position".into());
        }
        let keys = layer
            .get("keys")
            .and_then(Value::as_array)
            .ok_or("Layer has no keys")?;
        if keys.len() != 52 || !keys.iter().all(Value::is_object) {
            return Err("Only 52-key Voyager layouts are supported".into());
        }
    }
    Ok(())
}

fn identity(app: &AppHandle) -> Result<LayoutIdentity, String> {
    if let Some(identity) = app
        .state::<crate::state::HudState>()
        .layout_identity
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
    {
        return Ok(identity);
    }
    if app
        .state::<crate::state::HudState>()
        .keyboard_online
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        return Err("Connected keyboard has not reported an Oryx layout/revision identity".into());
    }
    let cfg = crate::config::load(&crate::oryx::config_path(app)?);
    let hash = crate::oryx::parse_layout_hash(&cfg.oryx_url).unwrap_or_default();
    LayoutIdentity::new(&hash, &cfg.oryx_revision)
        .ok_or("No keyboard layout detected; connect your Voyager".into())
}

fn cached_layout(
    path: &Path,
    expected: Option<&LayoutIdentity>,
    legacy: Option<&LayoutIdentity>,
) -> Result<Value, String> {
    let mut value: Value =
        serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    validate_layout(&value)?;
    let stored = value
        .get("_layer_hud")
        .cloned()
        .map(serde_json::from_value::<LayoutIdentity>)
        .transpose()
        .map_err(|e| format!("Invalid cache identity: {e}"))?;
    if let Some(expected) = expected {
        if stored.as_ref().or(legacy) != Some(expected) {
            return Err("Cached layout belongs to a different layout or firmware revision".into());
        }
    }
    value["stale"] = json!(true);
    Ok(value)
}

fn read_cache(app: &AppHandle, expected: Option<&LayoutIdentity>) -> Result<Value, String> {
    let cfg = crate::config::load(&crate::oryx::config_path(app)?);
    let hash = crate::oryx::parse_layout_hash(&cfg.oryx_url).unwrap_or_default();
    let legacy = LayoutIdentity::new(&hash, &cfg.oryx_revision);
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("layout.json");
    cached_layout(&path, expected, legacy.as_ref())
}

async fn fetch(identity: &LayoutIdentity) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let mut value: Value = client
        .post(GRAPHQL_URL)
        .json(&request_body(identity))
        .send()
        .await
        .map_err(|e| format!("Oryx connection failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Oryx request failed: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    validate_layout(&value)?;
    value["_layer_hud"] = serde_json::to_value(identity).map_err(|e| e.to_string())?;
    Ok(value)
}

#[tauri::command]
pub async fn refresh_layout(app: AppHandle) -> Result<Value, String> {
    let state = app.state::<crate::state::HudState>();
    let _guard = state.layout_lock.lock().await;
    let requested = identity(&app)?;
    let value = match fetch(&requested).await {
        Ok(value) => value,
        Err(error) => read_cache(&app, Some(&requested)).map_err(|_| error)?,
    };
    // A device may have been changed while a network request was in flight.
    if identity(&app)? != requested {
        return Err("Keyboard changed during layout refresh".into());
    }
    if value.get("stale") != Some(&json!(true)) {
        let path = app
            .path()
            .app_config_dir()
            .map_err(|e| e.to_string())?
            .join("layout.json");
        crate::config::save_json(&path, &value).map_err(|e| e.to_string())?;
        crate::oryx::update_config(&app, |cfg| {
            cfg.oryx_url = requested.layout;
            cfg.oryx_revision = requested.revision;
        })?;
    }
    let _ = app.emit("layout-refreshed", &value);
    Ok(value)
}

#[tauri::command]
pub async fn load_layout(app: AppHandle) -> Result<Value, String> {
    let expected = match identity(&app) {
        Ok(identity) => Some(identity),
        Err(error)
            if app
                .state::<crate::state::HudState>()
                .keyboard_online
                .load(std::sync::atomic::Ordering::SeqCst) =>
        {
            return Err(error)
        }
        Err(_) => None,
    };
    if let Ok(value) = read_cache(&app, expected.as_ref()) {
        return Ok(value);
    }
    refresh_layout(app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Value {
        json!({"data":{"layout":{"revision":{"layers":[{"position":0,"keys":vec![json!({});52]}]}}}})
    }

    #[test]
    fn flashed_revision_is_used_in_request() {
        let id = LayoutIdentity::from_serial("Br3gO/OadOzq").unwrap();
        assert_eq!(request_body(&id)["variables"]["revisionId"], "OadOzq");
        assert!(LayoutIdentity::from_serial("Br3gO/").is_none());
        assert!(LayoutIdentity::from_serial("../other").is_none());
    }

    #[test]
    fn validates_geometry_and_rejects_error_responses() {
        assert!(validate_layout(&layout()).is_ok());
        assert!(validate_layout(&json!({"errors":[{"message":"unavailable"}]})).is_err());
        let mut invalid = layout();
        invalid["data"]["layout"]["revision"]["layers"][0]["keys"] = json!([{}]);
        assert!(validate_layout(&invalid).is_err());
    }

    #[test]
    fn cache_never_substitutes_a_different_revision() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("layout.json");
        let id = LayoutIdentity::new("abc", "revision1").unwrap();
        let other = LayoutIdentity::new("abc", "revision2").unwrap();
        let mut value = layout();
        value["_layer_hud"] = serde_json::to_value(&id).unwrap();
        crate::config::save_json(&path, &value).unwrap();
        assert_eq!(
            cached_layout(&path, Some(&id), None).unwrap()["stale"],
            true
        );
        assert!(cached_layout(&path, Some(&other), Some(&other)).is_err());
        value.as_object_mut().unwrap().remove("_layer_hud");
        crate::config::save_json(&path, &value).unwrap();
        assert!(cached_layout(&path, Some(&other), Some(&id)).is_err());
        assert!(cached_layout(&path, Some(&id), Some(&id)).is_ok());
        value["_layer_hud"] = json!({"revision": "missing-layout"});
        crate::config::save_json(&path, &value).unwrap();
        assert!(cached_layout(&path, Some(&id), Some(&id)).is_err());
    }
}

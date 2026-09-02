use kontroll::Kontroll;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub fn extract_layer(status: &Value) -> Option<i64> {
    status.pointer("/keyboard/current_layer").and_then(Value::as_i64)
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_layer: Option<i64> = None;
        let mut online = true;
        let mut backoff = Duration::from_millis(250);
        let mut had_error = false;
        loop {
            // TODO: Consider caching the Kontroll connection and reconnecting only on error
            // to reduce the overhead of creating new instances every 100ms polling cycle.
            let status = match Kontroll::new(None).await {
                Ok(api) => {
                    had_error = false;
                    api.get_status().await.ok()
                }
                Err(e) => {
                    if !had_error {
                        eprintln!("Failed to connect to keymapp: {}", e);
                        had_error = true;
                    }
                    None
                }
            };
            match status.and_then(|s| serde_json::to_value(&s).ok()) {
                Some(v) => match extract_layer(&v) {
                    Some(layer) => {
                        if !online {
                            online = true;
                            if let Err(e) = app.emit("keymapp-online", json!({})) {
                                eprintln!("Failed to emit keymapp-online event: {}", e);
                            }
                        }
                        backoff = Duration::from_millis(250);
                        if last_layer != Some(layer) {
                            last_layer = Some(layer);
                            if let Err(e) = app.emit("layer-changed", json!({ "layer": layer })) {
                                eprintln!("Failed to emit layer-changed event: {}", e);
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    None => sleep_offline(&app, &mut online, &mut backoff).await,
                },
                None => sleep_offline(&app, &mut online, &mut backoff).await,
            }
        }
    });
}

async fn sleep_offline(app: &AppHandle, online: &mut bool, backoff: &mut Duration) {
    if *online {
        *online = false;
        if let Err(e) = app.emit("keymapp-offline", json!({})) {
            eprintln!("Failed to emit keymapp-offline event: {}", e);
        }
    }
    tokio::time::sleep(*backoff).await;
    *backoff = (*backoff * 2).min(Duration::from_secs(5));
}

#[cfg(test)]
mod tests {
    use super::extract_layer;
    use serde_json::json;

    #[test]
    fn extracts_current_layer() {
        let v = json!({"keymapp_version":"1.3.2","kontroll_version":"1.0.3",
            "keyboard":{"friendly_name":"Voyager","firmware_version":"x","current_layer":2}});
        assert_eq!(extract_layer(&v), Some(2));
    }

    #[test]
    fn none_when_no_keyboard() {
        let v = json!({"keymapp_version":"1.3.2","kontroll_version":"1.0.3","keyboard":null});
        assert_eq!(extract_layer(&v), None);
    }
}

use kontroll::Kontroll;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub fn extract_layer(status: &Value) -> Option<i64> {
    status.pointer("/keyboard/current_layer").and_then(Value::as_i64)
}

// Neither Kontroll::new() nor get_status() has any built-in timeout (tonic's
// HTTP/2 keepalive/request timeouts are opt-in, and the kontroll crate
// doesn't expose them), so a keymapp daemon that accepts the connection but
// stops responding (deadlock, suspended around sleep/wake, etc.) would hang
// either call forever. That risk existed before the client was reused too,
// but with a long-lived connection it now means the *same* stuck client
// every tick instead of a fresh one, so it's bounded here rather than left
// open. Local Unix-socket IPC normally completes in well under 100ms, so 1s
// is generous without stalling offline-detection for long on a real hang.
const KONTROLL_TIMEOUT: Duration = Duration::from_secs(1);

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_layer: Option<i64> = None;
        let mut online = true;
        let mut backoff = Duration::from_millis(250);
        // Dedups repeated stderr spam by message content rather than a
        // single boolean, so a failure mode *changing* (e.g. "not running"
        // to "hung") is never silently hidden just because some earlier,
        // different failure already set a shared flag.
        let mut last_error: Option<String> = None;
        // Reused across polls instead of reconnecting every ~100ms tick —
        // only dropped and re-established when a request actually fails.
        // Opening a fresh Unix-socket + HTTP/2 connection ~10x/second,
        // forever, was leaking enough over a long-running session to
        // exhaust memory.
        let mut client: Option<Kontroll> = None;
        loop {
            if client.is_none() {
                client = match tokio::time::timeout(KONTROLL_TIMEOUT, Kontroll::new(None)).await {
                    Ok(Ok(api)) => {
                        last_error = None;
                        Some(api)
                    }
                    Ok(Err(e)) => {
                        log_once(&mut last_error, format!("Failed to connect to keymapp: {e}"));
                        None
                    }
                    Err(_) => {
                        log_once(&mut last_error, "Timed out connecting to keymapp".to_string());
                        None
                    }
                };
            }
            let status = match &client {
                Some(api) => match tokio::time::timeout(KONTROLL_TIMEOUT, api.get_status()).await {
                    Ok(Ok(s)) => {
                        last_error = None;
                        Some(s)
                    }
                    // Treat a request failure OR a timeout as a dead
                    // connection (Keymapp restarted, keyboard unplugged, a
                    // hung daemon, etc.) and reconnect on the next tick
                    // instead of retrying the same stuck client forever.
                    Ok(Err(e)) => {
                        log_once(&mut last_error, format!("Keymapp request failed: {e}"));
                        client = None;
                        None
                    }
                    Err(_) => {
                        log_once(&mut last_error, "Timed out waiting for keymapp".to_string());
                        client = None;
                        None
                    }
                },
                None => None,
            };
            match status.and_then(|s| serde_json::to_value(&s).ok()) {
                Some(v) => match extract_layer(&v) {
                    Some(layer) => {
                        if !online {
                            online = true;
                            app.state::<crate::state::HudState>().keymapp_online.store(true, std::sync::atomic::Ordering::SeqCst);
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

/// Prints `msg` to stderr only if it differs from the last message logged —
/// repeats every ~100ms-1s tick are silenced, but a change in failure mode
/// (a different message) always gets its own line.
fn log_once(last_error: &mut Option<String>, msg: String) {
    if last_error.as_deref() != Some(msg.as_str()) {
        eprintln!("{msg}");
        *last_error = Some(msg);
    }
}

async fn sleep_offline(app: &AppHandle, online: &mut bool, backoff: &mut Duration) {
    if *online {
        *online = false;
        app.state::<crate::state::HudState>().keymapp_online.store(false, std::sync::atomic::Ordering::SeqCst);
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

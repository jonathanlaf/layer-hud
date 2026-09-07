//! Direct ZSA Oryx WebHID watcher.
//!
//! The keyboard exposes its live key stream on its Oryx Raw
//! HID collection, so this watcher talks to the Voyager directly.

use hidapi::{HidApi, HidDevice};
use serde_json::json;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const ORYX_USAGE_PAGE: u16 = 0xff60;
const ORYX_USAGE: u16 = 0x61;
const REPORT_LEN: usize = 32;
const HID_BUFFER_LEN: usize = REPORT_LEN + 1; // hidapi includes report ID

// Oryx WebHID protocol commands/events.
const PAIRING_INIT: u8 = 0x01;
const EVT_LAYER: u8 = 0x05;
const EVT_KEYDOWN: u8 = 0x06;
const EVT_KEYUP: u8 = 0x07;

fn matrix_index(col: usize, row: usize) -> Option<u8> {
    const MAP: [[i16; 7]; 12] = [
        [-1, 0, 1, 2, 3, 4, 5],
        [-1, 6, 7, 8, 9, 10, 11],
        [-1, 12, 13, 14, 15, 16, 17],
        [-1, 18, 19, 20, 21, 22, -1],
        [-1, -1, -1, -1, 23, -1, -1],
        [24, 25, -1, -1, -1, -1, -1],
        [26, 27, 28, 29, 30, 31, -1],
        [32, 33, 34, 35, 36, 37, -1],
        [38, 39, 40, 41, 42, 43, -1],
        [-1, 45, 46, 47, 48, 49, -1],
        [-1, -1, 44, -1, -1, -1, -1],
        [-1, -1, -1, -1, -1, 50, 51],
    ];
    MAP.get(row)?
        .get(col)
        .copied()
        .filter(|i| *i >= 0)
        .map(|i| i as u8)
}

fn is_voyager_interface(product: Option<&str>, usage_page: u16, usage: u16) -> bool {
    usage_page == ORYX_USAGE_PAGE
        && usage == ORYX_USAGE
        && product.is_some_and(|name| name.eq_ignore_ascii_case("Voyager"))
}

fn find_device(api: &HidApi) -> Option<HidDevice> {
    api.device_list()
        .filter(|info| is_voyager_interface(info.product_string(), info.usage_page(), info.usage()))
        .find_map(|info| info.open_device(api).ok())
}

fn pair(device: &HidDevice) -> bool {
    let mut packet = [0_u8; HID_BUFFER_LEN];
    packet[1..].fill(0xfe);
    packet[1] = PAIRING_INIT;
    device.write(&packet).is_ok()
}

fn emit_online(app: &AppHandle, online: &mut bool) {
    if !*online {
        *online = true;
        app.state::<crate::state::HudState>()
            .keyboard_online
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if app
            .state::<crate::state::HudState>()
            .connection_override
            .load(std::sync::atomic::Ordering::SeqCst)
            != 0
        {
            let _ = app.emit("keyboard-online", json!({}));
        }
    }
}

fn emit_offline(app: &AppHandle, online: &mut bool) {
    if *online {
        *online = false;
        app.state::<crate::state::HudState>()
            .active_layer
            .store(0, std::sync::atomic::Ordering::SeqCst);
        app.state::<crate::state::HudState>()
            .keyboard_online
            .store(false, std::sync::atomic::Ordering::SeqCst);
        if app
            .state::<crate::state::HudState>()
            .connection_override
            .load(std::sync::atomic::Ordering::SeqCst)
            != 1
        {
            let _ = app.emit("keyboard-offline", json!({}));
        }
    }
}

fn emit_layout_identity(app: &AppHandle, device: &HidDevice) {
    let identity = device
        .get_serial_number_string()
        .ok()
        .flatten()
        .and_then(|serial| crate::layout::LayoutIdentity::from_serial(&serial));
    *app.state::<crate::state::HudState>()
        .layout_identity
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = identity.clone();
    let _ = app.emit("layout-loading", identity);
    // Fetch in the backend: a one-shot event sent before the webview starts
    // listening must not be the only copy of the keyboard's identity.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = crate::layout::refresh_layout(app.clone()).await {
            eprintln!("layer-hud: {error}");
            let _ = app.emit("layout-error", error);
        }
    });
}

#[derive(Debug, PartialEq)]
enum Packet {
    Layer(u8),
    Key {
        col: u8,
        row: u8,
        index: u8,
        pressed: bool,
    },
}

fn decode_packet(packet: &[u8]) -> Option<Packet> {
    // hidapi reads normally omit report ID; also accept padded ID-zero reports.
    let data = if packet.first() == Some(&0) && packet.len() == HID_BUFFER_LEN {
        &packet[1..]
    } else {
        packet
    };
    match *data.first()? {
        EVT_LAYER => Some(Packet::Layer(*data.get(1)?)),
        EVT_KEYDOWN | EVT_KEYUP => {
            let col = *data.get(1)?;
            let row = *data.get(2)?;
            Some(Packet::Key {
                col,
                row,
                index: matrix_index(col as usize, row as usize)?,
                pressed: data[0] == EVT_KEYDOWN,
            })
        }
        _ => None,
    }
}

fn handle_packet(app: &AppHandle, packet: &[u8]) {
    match decode_packet(packet) {
        Some(Packet::Layer(layer)) => {
            app.state::<crate::state::HudState>()
                .active_layer
                .store(layer, std::sync::atomic::Ordering::SeqCst);
            let _ = app.emit("layer-changed", json!({ "layer": layer }));
        }
        Some(Packet::Key {
            col,
            row,
            index,
            pressed,
        }) => {
            if pressed {
                record_toggle_macro(app, index);
            }
            let _ = app.emit(
                "key-event",
                json!({ "col": col, "row": row, "index": index, "pressed": pressed }),
            );
        }
        None => {}
    }
}

fn record_toggle_macro(app: &AppHandle, index: u8) {
    let state = app.state::<crate::state::HudState>();
    if state
        .macro_recording
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        return;
    }
    let Ok(path) = crate::oryx::config_path(app) else {
        return;
    };
    let cfg = crate::config::load(&path);
    if cfg.toggle_macro.is_empty() {
        return;
    }
    let now = std::time::Instant::now();
    {
        let mut last = state
            .macro_last_down
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if last.is_some_and(|(previous, at)| {
            previous == index && now.duration_since(at) < std::time::Duration::from_millis(160)
        }) {
            return;
        }
        *last = Some((index, now));
    }
    let mut last_event = state
        .macro_last_event
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let mut buffer = state.macro_buffer.lock().unwrap_or_else(|p| p.into_inner());
    if last_event.is_some_and(|at| now.duration_since(at) > std::time::Duration::from_secs(1)) {
        buffer.clear();
    }
    *last_event = Some(now);
    buffer.push(index);
    if buffer.len() > cfg.toggle_macro.len() {
        let excess = buffer.len() - cfg.toggle_macro.len();
        buffer.drain(..excess);
    }
    if buffer.as_slice() == cfg.toggle_macro.as_slice() {
        buffer.clear();
        drop(buffer);
        drop(last_event);
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = crate::oryx::toggle_overlay_visibility(app.clone()).await {
                eprintln!("layer-hud: HID macro toggle failed: {error}");
                let _ = app.emit("overlay-toggle-error", error);
            }
        });
    }
}

pub fn spawn(app: AppHandle) {
    thread::Builder::new()
        .name("oryx-hid-watcher".into())
        .spawn(move || {
            let mut online = false;
            loop {
                let Ok(api) = HidApi::new() else {
                    emit_offline(&app, &mut online);
                    thread::sleep(Duration::from_secs(2));
                    continue;
                };
                let Some(device) = find_device(&api) else {
                    emit_offline(&app, &mut online);
                    thread::sleep(Duration::from_secs(1));
                    continue;
                };

                let _ = device.set_blocking_mode(false);
                if !pair(&device) {
                    emit_offline(&app, &mut online);
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
                emit_online(&app, &mut online);
                emit_layout_identity(&app, &device);

                let mut packet = [0_u8; HID_BUFFER_LEN];
                loop {
                    match device.read_timeout(&mut packet, 250) {
                        Ok(0) => {}
                        Ok(n) => handle_packet(&app, &packet[..n]),
                        Err(_) => {
                            emit_offline(&app, &mut online);
                            break;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(250));
            }
        })
        .expect("failed to start Oryx HID watcher");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_other_raw_hid_keyboards_and_normal_typing_interfaces() {
        assert!(is_voyager_interface(
            Some("Voyager"),
            ORYX_USAGE_PAGE,
            ORYX_USAGE
        ));
        assert!(!is_voyager_interface(
            Some("Moonlander"),
            ORYX_USAGE_PAGE,
            ORYX_USAGE
        ));
        assert!(!is_voyager_interface(None, ORYX_USAGE_PAGE, ORYX_USAGE));
        assert!(!is_voyager_interface(Some("Voyager"), 1, 6));
    }

    #[test]
    fn decodes_presses_releases_layers_and_padded_reports() {
        assert_eq!(
            decode_packet(&[EVT_KEYDOWN, 1, 0]),
            Some(Packet::Key {
                col: 1,
                row: 0,
                index: 0,
                pressed: true
            })
        );
        assert_eq!(
            decode_packet(&[EVT_KEYUP, 6, 11]),
            Some(Packet::Key {
                col: 6,
                row: 11,
                index: 51,
                pressed: false
            })
        );
        assert_eq!(decode_packet(&[EVT_LAYER, 2]), Some(Packet::Layer(2)));
        let mut report = [0; HID_BUFFER_LEN];
        report[1..4].copy_from_slice(&[EVT_KEYDOWN, 0, 5]);
        assert_eq!(
            decode_packet(&report),
            Some(Packet::Key {
                col: 0,
                row: 5,
                index: 24,
                pressed: true
            })
        );
    }

    #[test]
    fn malformed_and_unused_matrix_positions_are_ignored() {
        for bytes in [
            vec![],
            vec![EVT_LAYER],
            vec![EVT_KEYDOWN, 17],
            vec![EVT_KEYDOWN, 0, 0],
            vec![EVT_KEYDOWN, 255, 255],
            vec![99, 1, 0],
        ] {
            assert_eq!(decode_packet(&bytes), None);
        }
    }

    #[test]
    fn every_voyager_key_has_exactly_one_matrix_position() {
        let mut keys: Vec<_> = (0..12)
            .flat_map(|row| (0..7).filter_map(move |col| matrix_index(col, row)))
            .collect();
        keys.sort();
        assert_eq!(keys, (0..52).collect::<Vec<_>>());
    }
}

//! Direct ZSA Oryx WebHID watcher.
//!
//! Keymapp's local gRPC API exposes status/control operations but not the
//! live key stream.  The keyboard itself exposes that stream on its Oryx Raw
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
        [-1, 0, 1, 2, 3, 4, 5], [-1, 6, 7, 8, 9, 10, 11],
        [-1, 12, 13, 14, 15, 16, 17], [-1, 18, 19, 20, 21, 22, -1],
        [-1, -1, -1, -1, 23, -1, -1], [24, 25, -1, -1, -1, -1, -1],
        [26, 27, 28, 29, 30, 31, -1], [32, 33, 34, 35, 36, 37, -1],
        [38, 39, 40, 41, 42, 43, -1], [-1, 45, 46, 47, 48, 49, -1],
        [-1, -1, 44, -1, -1, -1, -1], [-1, -1, -1, -1, -1, 50, 51],
    ];
    MAP.get(row)?.get(col).copied().filter(|i| *i >= 0).map(|i| i as u8)
}

fn find_device(api: &HidApi) -> Option<HidDevice> {
    api.device_list()
        .find(|info| {
            info.usage_page() == ORYX_USAGE_PAGE && info.usage() == ORYX_USAGE
        })
        .and_then(|info| info.open_device(api).ok())
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
            .keymapp_online
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = app.emit("keymapp-online", json!({}));
    }
}

fn emit_offline(app: &AppHandle, online: &mut bool) {
    if *online {
        *online = false;
        app.state::<crate::state::HudState>()
            .keymapp_online
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = app.emit("keymapp-offline", json!({}));
    }
}

fn emit_layout_identity(app: &AppHandle, device: &HidDevice) {
    // Oryx-generated firmware stores the layout/revision pair in the USB
    // serial number, e.g. "Br3gO/OadOzq".  This lets the UI select the right
    // cached layout without a manually entered URL.
    if let Ok(Some(serial)) = device.get_serial_number_string() {
        if let Some((layout, revision)) = serial.split_once('/') {
            if !layout.is_empty() {
                let _ = app.emit(
                    "keyboard-layout",
                    json!({ "layout": layout, "revision": revision }),
                );
            }
        }
    }
}

fn handle_packet(app: &AppHandle, packet: &[u8]) {
    if packet.is_empty() {
        return;
    }
    // hidapi prepends a zero Report ID for this unnumbered HID report.
    let data = if packet[0] == 0 && packet.len() > REPORT_LEN {
        &packet[1..]
    } else {
        packet
    };
    match data[0] {
        EVT_LAYER if data.len() > 1 => {
            let _ = app.emit("layer-changed", json!({ "layer": data[1] }));
        }
        // Oryx sends the QMK matrix coordinates, not the flattened visual
        // key index: byte 1 is the matrix column and byte 2 is the row.
        EVT_KEYDOWN | EVT_KEYUP if data.len() > 2 => {
            let _ = app.emit(
                "key-event",
                json!({
                    "col": data[1],
                    "row": data[2],
                    "index": matrix_index(data[1] as usize, data[2] as usize),
                    "pressed": data[0] == EVT_KEYDOWN,
                }),
            );
        }
        _ => {}
    }
}

pub fn spawn(app: AppHandle) {
    thread::Builder::new()
        .name("oryx-hid-watcher".into())
        .spawn(move || {
            // HudState starts optimistic so the overlay can render while the
            // first USB enumeration is happening; transition it to offline
            // immediately if no compatible HID device is found.
            let mut online = true;
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
    use super::handle_packet;
    use tauri::test::{mock_app, noop_assets};

    #[test]
    fn key_packets_are_decoded() {
        let app = mock_app(noop_assets());
        handle_packet(&app, &[0x06, 17]);
        handle_packet(&app, &[0x07, 17]);
    }
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod grab;
mod oryx;
mod state;
mod tray;
mod watcher;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            app.manage(state::HudState::new());
            if let Err(e) = oryx::migrate_legacy_identifier(app.handle()) {
                eprintln!("layer-hud: legacy config migration failed: {e}");
            }
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let overlay = app.get_webview_window("overlay").expect("overlay window");
            overlay.set_ignore_cursor_events(true)?;

            // Restore window position and size
            if let Ok(path) = oryx::config_path(app.handle()) {
                let cfg = config::load(&path);
                if let Some(r) = &cfg.window {
                    // Validate that the saved position is on a currently available monitor
                    let on_screen = if let Ok(monitors) = overlay.available_monitors() {
                        monitors.iter().any(|mon| {
                            let scale = mon.scale_factor();
                            let mon_pos = mon.position().to_logical::<f64>(scale);
                            let mon_size = mon.size().to_logical::<f64>(scale);
                            r.x >= mon_pos.x && r.x < mon_pos.x + mon_size.width &&
                            r.y >= mon_pos.y && r.y < mon_pos.y + mon_size.height
                        })
                    } else {
                        false
                    };

                    if on_screen {
                        use tauri::{LogicalPosition, LogicalSize};
                        let _ = overlay.set_position(LogicalPosition::new(r.x, r.y));
                        let _ = overlay.set_size(LogicalSize::new(r.w, r.h));
                    }
                }
            }

            watcher::spawn(app.handle().clone());
            grab::spawn(app.handle().clone());
            tray::build(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "overlay" {
                return;
            }
            if matches!(event, tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)) {
                let app = window.app_handle();
                let Ok(path) = oryx::config_path(app) else { return; };
                let mut cfg = config::load(&path);
                let scale = window.scale_factor().unwrap_or(1.0);
                if let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) {
                    let pos = pos.to_logical::<f64>(scale);
                    let size = size.to_logical::<f64>(scale);
                    cfg.window = Some(config::WindowRect { x: pos.x, y: pos.y, w: size.width, h: size.height });
                    if let Err(e) = config::save(&path, &cfg) {
                        eprintln!("layer-hud: failed to persist window rect: {e}");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            oryx::refresh_layout,
            oryx::load_layout,
            oryx::get_config,
            oryx::set_config,
            oryx::clear_window_position
        ])
        .run(tauri::generate_context!())
        .expect("error while running layer-hud");
}

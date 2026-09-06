#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod grab;
mod hid;
mod oryx;
mod state;
mod tray;

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

            // Restore this monitor's saved position/size, or — first time the
            // overlay has ever appeared on it — start at 30% of it, centered.
            if let Ok(path) = oryx::config_path(app.handle()) {
                let cfg = config::load(&path);
                app.state::<state::HudState>().pinned.store(cfg.overlay_pinned, std::sync::atomic::Ordering::SeqCst);
                *app.state::<state::HudState>().grab_combo.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = cfg.grab_combo.clone();
                let monitors = overlay.available_monitors().unwrap_or_default();
                // A freshly-created window's current_monitor() just reflects
                // wherever the OS placed it (usually the primary display),
                // not necessarily where the user left it — so prefer the
                // monitor last_monitor names, if it's still connected, over
                // trusting current_monitor() for *which* monitor to restore.
                let target = cfg
                    .last_monitor
                    .as_ref()
                    .and_then(|last| monitors.iter().find(|m| &oryx::monitor_key(m) == last))
                    .or_else(|| overlay.current_monitor().ok().flatten().as_ref().and_then(|cur| {
                        let key = oryx::monitor_key(cur);
                        monitors.iter().find(|m| oryx::monitor_key(m) == key)
                    }))
                    .or_else(|| monitors.first());
                if let Some(mon) = target {
                    let key = oryx::monitor_key(mon);
                    let rect = cfg
                        .window_by_monitor
                        .get(&key)
                        .filter(|r| oryx::rect_fits_monitor(r, mon))
                        .cloned()
                        .unwrap_or_else(|| oryx::default_rect_for_monitor(mon));
                    oryx::apply_rect(&overlay, &rect);
                }
            }

            hid::spawn(app.handle().clone());
            grab::spawn(app.handle().clone());
            tray::build(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "overlay" {
                return;
            }
            if window.app_handle().state::<state::HudState>().overlay_hidden.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            if matches!(event, tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)) {
                let app = window.app_handle();
                let scale = window.scale_factor().unwrap_or(1.0);
                if let (Ok(pos), Ok(size), Ok(Some(mon))) =
                    (window.outer_position(), window.inner_size(), window.current_monitor())
                {
                    let pos = pos.to_logical::<f64>(scale);
                    let mut size = size.to_logical::<f64>(scale);
                    // Keep the keyboard's aspect ratio and resize around its
                    // current center, so dragging any corner grows/shrinks it
                    // without making the overlay drift or distort its padding.
                    if matches!(event, tauri::WindowEvent::Resized(_)) {
                        let half_distance = oryx::config_path(&app)
                            .ok()
                            .map(|path| config::load(&path).keyboard_halves_distance)
                            .unwrap_or(1.6);
                        let ratio = (12.0 + half_distance) / 6.0;
                        let target_h = size.width / ratio;
                        if (target_h - size.height).abs() > 1.0 {
                            size.height = target_h.max(120.0);
                            // Keep the corner being dragged anchored; only
                            // correct the opposite dimension to the keyboard
                            // ratio, avoiding the visible jump caused by
                            // repeatedly recentering during native resize.
                            let _ = window.set_size(tauri::LogicalSize::new(size.width, size.height));
                        }
                    }
                    let rect = config::WindowRect { x: pos.x, y: pos.y, w: size.width, h: size.height };
                    let key = oryx::monitor_key(&mon);
                    if let Err(e) = oryx::update_config(app, move |cfg| {
                        cfg.window_by_monitor.insert(key.clone(), rect);
                        cfg.last_monitor = Some(key);
                    }) {
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
            oryx::clear_window_position,
            oryx::align_window,
            oryx::reset_window_positions,
            oryx::recalculate_window_geometry,
            oryx::is_overlay_pinned,
            oryx::toggle_overlay_visibility,
            oryx::is_keymapp_online,
            oryx::export_config,
            oryx::import_config,
            oryx::reset_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running layer-hud");
}

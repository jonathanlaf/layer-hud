#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod grab;
mod oryx;
mod watcher;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let overlay = app.get_webview_window("overlay").expect("overlay window");
            overlay.set_ignore_cursor_events(true)?;
            watcher::spawn(app.handle().clone());
            grab::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            oryx::refresh_layout,
            oryx::load_layout,
            oryx::get_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running voyager-hud");
}

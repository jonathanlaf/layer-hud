use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let refresh = MenuItem::with_id(app, "refresh", "Refresh layout", true, None::<&str>)?;
    let pin = CheckMenuItem::with_id(app, "pin", "Pin overlay (interactive)", true, false, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&refresh, &pin, &settings, &quit])?;
    let pin_handle = pin.clone();

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "refresh" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let Ok(path) = crate::oryx::config_path(&app) else { return; };
                    let hash = crate::config::load(&path).oryx_url;
                    let _ = crate::oryx::refresh_layout(app.clone(), hash).await;
                });
            }
            "pin" => {
                let pinned = pin_handle.is_checked().unwrap_or(false);
                // Only update the shared flag here. Applying the window flag
                // and emitting grab-mode is left entirely to grab::spawn's
                // poll loop, which recomputes `grabbed || pinned` every tick
                // against its own last-applied cache — if tray.rs also wrote
                // set_ignore_cursor_events/grab-mode directly, the two could
                // desync (e.g. unchecking pin while the combo is still held
                // would wrongly force the window non-interactive here, and
                // the loop's cache would then suppress the correction).
                app.state::<crate::state::HudState>()
                    .pinned
                    .store(pinned, std::sync::atomic::Ordering::SeqCst);
            }
            "settings" => {
                if app.get_webview_window("settings").is_none() {
                    let _ = tauri::WebviewWindowBuilder::new(
                        app,
                        "settings",
                        tauri::WebviewUrl::App("settings.html".into()),
                    )
                    .title("Voyager HUD Settings")
                    .inner_size(420.0, 380.0)
                    .build();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

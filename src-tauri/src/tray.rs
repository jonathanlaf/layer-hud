use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let refresh = MenuItem::with_id(app, "refresh", "Refresh layout", true, None::<&str>)?;
    let pin = CheckMenuItem::with_id(app, "pin", "Pin overlay (interactive)", true, false, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let legend = MenuItem::with_id(app, "legend", "Icon legend & layers…", true, None::<&str>)?;
    #[cfg(debug_assertions)]
    let devtools = MenuItem::with_id(app, "devtools", "Open DevTools", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    // The overlay's own right/ctrl-click context menu is disabled (see
    // hud.js) since a click during grab mode can hold ctrl; DevTools access
    // moves here instead, and only exists at all in debug builds.
    let mut items: Vec<&dyn IsMenuItem<tauri::Wry>> = vec![&refresh, &pin, &settings, &legend];
    #[cfg(debug_assertions)]
    items.push(&devtools);
    items.push(&quit);
    let menu = Menu::with_items(app, &items)?;
    let pin_handle = pin.clone();

    // Rasterized from icons/voyager.svg. Keep the transparent monochrome image
    // as a template so macOS supplies contrasting light/dark menu-bar colors.
    let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id("main")
        .icon(tray_icon)
        .icon_as_template(true)
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
                // Flip our own tracked flag rather than trusting the menu
                // item's post-click is_checked() — whether the native menu
                // auto-toggles its checkmark before or after firing this
                // event is platform/toolkit behavior we don't control, and
                // reading it produced the exact "one click behind" symptom
                // this replaced. Our AtomicBool is the single source of
                // truth; set_checked() below only syncs the checkmark to it.
                let state = app.state::<crate::state::HudState>();
                let pinned = !state.pinned.load(std::sync::atomic::Ordering::SeqCst);
                let _ = pin_handle.set_checked(pinned);
                // Only update the shared flag here. Applying the window flag
                // and emitting grab-mode is left entirely to grab::spawn's
                // poll loop, which recomputes `grabbed || pinned` every tick
                // against its own last-applied cache — if tray.rs also wrote
                // set_ignore_cursor_events/grab-mode directly, the two could
                // desync (e.g. unchecking pin while the combo is still held
                // would wrongly force the window non-interactive here, and
                // the loop's cache would then suppress the correction).
                state.pinned.store(pinned, std::sync::atomic::Ordering::SeqCst);
            }
            "legend" => {
                if let Some(w) = app.get_webview_window("legend") {
                    let _ = w.unminimize();
                    let _ = w.show();
                    let _ = w.set_focus();
                } else if let Err(e) = tauri::WebviewWindowBuilder::new(
                    app,
                    "legend",
                    tauri::WebviewUrl::App("legend.html".into()),
                )
                .title("Layer HUD — Icon legend & layers")
                .inner_size(620.0, 640.0)
                .min_inner_size(420.0, 320.0)
                .build() {
                    eprintln!("Failed to open icon legend: {e}");
                }
            }
            "settings" => {
                if let Some(w) = app.get_webview_window("settings") {
                    let _ = w.unminimize();
                    let _ = w.set_always_on_top(true);
                    let _ = w.set_size(tauri::LogicalSize::new(640.0, 720.0));
                    let _ = w.show();
                    let _ = w.set_focus();
                } else {
                    let _ = tauri::WebviewWindowBuilder::new(
                        app,
                        "settings",
                        tauri::WebviewUrl::App("settings.html".into()),
                    )
                    .title("Layer HUD Settings")
                    .inner_size(640.0, 720.0)
                    .min_inner_size(520.0, 520.0)
                    .always_on_top(true)
                    .build();
                }
            }
            #[cfg(debug_assertions)]
            "devtools" => {
                // Open both — the bug being chased is often a mismatch
                // between what Settings sends and what the overlay applies,
                // so one console alone tells half the story.
                if let Some(w) = app.get_webview_window("settings") {
                    w.open_devtools();
                }
                if let Some(w) = app.get_webview_window("overlay") {
                    w.open_devtools();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

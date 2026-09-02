# Voyager HUD

A macOS overlay application that displays your ZSA Voyager keyboard layer state in real-time. Voyager HUD connects to Keymapp's API to fetch your keyboard layout, renders layers on a transparent overlay, and mirrors the Oryx color scheme—keeping you informed of your current layer without interrupting your workflow.

## Prerequisites

- **Keymapp** ≥ 1.3.2 with API enabled (enable in Keymapp settings)
- **protoc** (Protocol Buffers compiler): `brew install protobuf`
- **Rust** and **Cargo**: Install from [rustup.rs](https://rustup.rs)
- **tauri-cli**: `cargo install tauri-cli`

## Build

From the project root:

```bash
cd src-tauri
cargo tauri build
```

The release `.app` bundle will be created at:
```
src-tauri/target/release/bundle/macos/voyager-hud.app
```

Launch it from Applications or Finder.

## Usage

- **Tray Menu:** Click the Voyager HUD icon in the macOS menu bar to show/hide or quit
- **Move Overlay:** Hold ⌘⌥ (Command+Option) and drag the overlay to reposition it
- **Settings:** Right-click the overlay or use the tray menu to access settings (Keymapp URL, opacity, grab combo, color scheme)
- **Layer Colors:** Layer background colors are mirrored from your Oryx layout if "Use Oryx Colors" is enabled in settings

## Troubleshooting

- **Offline Badge:** Ensure Keymapp's API is enabled in Keymapp settings (check the Keymapp UI for the API server status)
- **Multiple Clients:** Only one API client can connect to Keymapp at a time; close other applications using the API before launching Voyager HUD
- **Layer Colors:** Layer colors are sourced from the Oryx layout (https://configure.zsa.io/). If colors appear missing or wrong, verify your Keymapp URL in Voyager HUD settings matches your Oryx layout

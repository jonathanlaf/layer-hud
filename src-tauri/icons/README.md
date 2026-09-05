# Voyager icons

`voyager.svg` is the monochrome menu-bar source, showing the left keyboard half
centered and enlarged on a square canvas. Keep `icon_as_template(true)` for
native contrast.

`voyager-app.svg` adds a light tile for the bundled application icon.

Regenerate from the repository root:

```sh
icon_build_dir=$(mktemp -d /private/tmp/layer-hud-icons.XXXXXX)
cargo tauri icon src-tauri/icons/voyager.svg --output "$icon_build_dir/tray" --png 64
cp "$icon_build_dir/tray/64x64.png" src-tauri/icons/tray.png
cargo tauri icon src-tauri/icons/voyager-app.svg --output "$icon_build_dir/app" --png 512
cp "$icon_build_dir/app/512x512.png" src-tauri/icons/icon.png
```

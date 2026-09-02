# Voyager HUD — Design

Always-on-top transparent overlay for macOS showing the live keymap of a ZSA Voyager, with the active layer highlighted in real time. Fills the gap in Keymapp, whose visualization is a normal opaque window that cannot float over other apps.

## Goals

- Permanent translucent HUD rendering both Voyager halves with full legends (tap, hold, double-tap per key), showing characters as they actually output on macOS with the Canadian–CSA input source.
- Live active-layer highlight driven by the Keymapp gRPC API (via the `kontroll` Rust crate).
- Click-through by default; grabbable (move/resize) while a configurable modifier combo is held.
- Settings UI: Oryx layout URL, opacity, grab hotkey.

## Non-goals

- Per-key press visualization (the Keymapp API exposes no key events — verified against the full `keymapp.proto`).
- Reading the keymap from the compiled firmware `.bin` (no symbols; compiler-dependent offsets; less data than Oryx's API).
- Windows/Linux support, multiple keyboards, RGB control.

## Architecture

Tauri v2 menu-bar app (`voyager-hud`, no Dock icon), one overlay window + one settings window.

### Rust backend

- **Layer watcher** — `kontroll` crate → Keymapp Unix socket (`$CONFIG_DIR/.keymapp/keymapp.sock`). Polls `GetStatus` at 10 Hz; emits Tauri event `layer-changed { layer: u32 }` only on change. On failure (Keymapp closed, API disabled, socket busy with another client): emits `keymapp-offline`, retries with exponential backoff capped at 5 s, emits `keymapp-online` on recovery.
- **Grab manager** — polls modifier state (`device_query`) at ~10 Hz. While the configured combo (default ⌘⌥) is held: `set_ignore_cursor_events(false)` and emits `grab-mode { on: true }`; on release, back to click-through. Default is ⌘⌥, not plain ⌘, so ordinary ⌘-shortcuts never make the overlay swallow a click.
- **Layout fetcher** — Tauri command `refresh_layout(url_or_hash)`: parses the layout hash from any Oryx URL form, queries the Oryx GraphQL API (`https://oryx.zsa.io/graphql`, `layout(hashId, revisionId: "latest", geometry: "voyager")`), stores the raw reply as `layout.json` in the app config dir, returns it to the frontend. On network failure, returns the cached copy with a `stale: true` flag and the cache timestamp.
- **Config** — JSON in the Tauri app config dir: window position/size, opacity, grab combo, Oryx URL, last refresh timestamp. Loaded at startup, saved on change.

### Frontend (plain HTML/CSS/JS, no framework)

- **Keycode translator** — pure function: Oryx key object → display labels. Table-driven from the macOS Canadian–CSA mapping (verified by dumping the OS layout tables via `UCKeyTranslate`), e.g. `CSA_EGRV+Alt` → `\`, `KC_MINUS+Alt` → `|`, `CSA_ECUT+Alt` → `/`, `CSA_AGRV+Alt` → `` ` ``, `KC_COLN` → `:`, `CSA_QEST` → `?`, dead keys marked (`^ dead`). Unknown codes fall back to a cleaned keycode name.
- **Renderer** — draws both halves in physical Voyager geometry (column stagger + 2-key thumb clusters) using absolutely positioned divs; scales with window size. All layers rendered at startup; layer switch = CSS class flip. Per key: tap label centered, hold label small underneath, double-tap in the top-right corner. Layer-trigger keys highlighted while their layer is active. Non-base layers tint the frame edge for peripheral awareness. Layer-name badge in a corner.
- **States** — `keymapp-offline`: overlay dims to 40 %, small "Keymapp offline" badge, keeps base layer. Grab mode: brightened border + resize affordance. Unknown layer index: badge shows the number over an empty board.

### Settings window

Opened from the tray menu. Fields: Oryx layout URL (validated by fetching; inline error), opacity slider (live preview), grab-combo picker (⌘, ⌥, ⌃, ⌘⌥, ⌘⌃, ⌥⌃). Tray menu additionally: Refresh layout, Pin/unpin (manual click-through override), Settings…, Quit.

## Data flow

```
Oryx GraphQL ──refresh──▶ layout.json (cache) ──▶ translator ──▶ renderer (all layers)
Keymapp UDS ◀─poll 10Hz── layer watcher ──event──▶ CSS class flip
modifier poll ──▶ grab manager ──▶ ignore_cursor_events + grab styling
```

## Constraints and known trade-offs

- Keymapp must be running with its API enabled; it accepts one API client at a time. The HUD is a companion, not a replacement.
- Momentary layers (`MO`) held shorter than ~100 ms may be missed by 10 Hz polling; acceptable — a layer used that briefly doesn't need a cheat-sheet.
- The CSA translation table is macOS-specific and assumes the keyboard is detected as ANSI or uses only swap-immune combos (the user's layout was built to be swap-immune throughout).

## Testing

- Unit tests (JS) for the keycode translator, seeded with the hand-verified CSA cases above plus dead-key and unknown-code fallbacks.
- Unit tests (Rust) for Oryx URL/hash parsing and config round-trip.
- gRPC integration and window behavior verified manually via `tauri dev` with Keymapp running.

## Decisions log

- **Tauri over Swift/Electron/Hammerspoon** — official `kontroll` crate, first-class transparent/always-on-top/click-through windows, small binary.
- **Oryx URL over firmware `.bin` or source ZIP** — same data source Oryx itself uses; richer metadata; no parser to maintain.
- **Default grab combo ⌘⌥** — plain ⌘ would make the overlay momentarily clickable during every ordinary shortcut.
- **10 Hz polling** — cheap over UDS; catches held layers; no streaming API exists.

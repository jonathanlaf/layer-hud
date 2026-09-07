# Layer HUD

[![CI](https://github.com/jonathanlaf/layer-hud/actions/workflows/ci.yml/badge.svg)](https://github.com/jonathanlaf/layer-hud/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/jonathanlaf/layer-hud?include_prereleases)](https://github.com/jonathanlaf/layer-hud/releases/latest)

A macOS menu-bar utility that currently displays your ZSA Voyager keyboard layer state in real time. Layer HUD connects directly to the keyboard over its Oryx HID interface, reads the keyboard's layout identity and active layer/key events, and renders a click-through transparent overlay over your work area. It shows the two keyboard halves, layer-trigger and alternate-character hints, shift indicators, Oryx colors, and the current layer name without interrupting your workflow. Support for additional ZSA keyboards is planned.

When the layout service is unavailable, Layer HUD uses its local layout cache and clearly marks the overlay as offline. A tray menu opens the layer/icon legend and Settings, where the overlay, colors, fonts, icons, heatmap, resizing, and complete JSON settings backup/restore can be configured.

This project is fully vibe coded as an experiment in building a useful native utility with [Claude](https://www.anthropic.com/claude) and [OpenAI Codex](https://openai.com/codex/).

## Prerequisites

- A connected ZSA Voyager keyboard with its Oryx HID interface available
- **protoc** (Protocol Buffers compiler): `brew install protobuf`
- **Rust** and **Cargo**: Install from [rustup.rs](https://rustup.rs)
- **tauri-cli**: `cargo install tauri-cli --locked`

## Build

From the project root:

```bash
cd src-tauri
cargo tauri build
```

The release build produces both a `.app` and an installable `.dmg`:
```
src-tauri/target/release/bundle/macos/layer-hud.app
src-tauri/target/release/bundle/dmg/layer-hud_<version>_<arch>.dmg
```

Launch it from Applications or Finder.

## Usage

- **Tray Menu:** Click the Layer HUD icon in the macOS menu bar for Refresh layout, the icon/layer legend, Settings, Pin overlay, and Quit
- **Move and resize:** Hold the configured grab shortcut (⌘⌥ by default) and drag the overlay; resizing keeps the keyboard proportions and stores the window per monitor
- **Settings:** Configure appearance, colors, fonts, icons, heatmap visualization/counts, hotkeys, hidden-edge behavior, and startup behavior. Export or import the complete settings set as a JSON file, or reset everything to defaults
- **Heatmap:** Enable persistent per-key press-frequency coloring and optional per-key counts. Double-taps count as two physical presses; a held key counts once when pressed. Heatmap history can be reset from Settings
- **Offline use:** The layout identified by the keyboard is cached locally, so the last known keyboard layout remains available while the layout service is unavailable
- **Layer colors:** Layer colors are mirrored from your Oryx layout when “Use Oryx layer colors” is enabled

## Troubleshooting

- **Offline Badge:** Connect the Voyager directly and relaunch Layer HUD; the last cached layout remains available while the layout service is unavailable
- **Keyboard Not Detected:** Confirm the keyboard is connected and not claimed exclusively by another HID utility, then use Refresh layout from the tray menu
- **Layer Colors:** Layer colors are sourced from the cached Oryx layout (https://configure.zsa.io/)

## Security & Provenance

Every release is built by GitHub Actions directly from this repository's source — never on a personal machine — and attested with [GitHub build provenance](https://docs.github.com/en/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds): a signed, verifiable statement of exactly which commit and workflow produced the binary. Before running a downloaded release, verify it:

```bash
gh attestation verify <path-to-downloaded-file>.dmg --owner jonathanlaf
```

A successful verification means the file you downloaded was built from this repository's source by this repository's CI — not tampered with in transit or substituted upstream.

CI only builds from tags reachable from `main` (enforced in CI, not just convention) and creates each release as a **draft** for manual review before it goes public — the version badge above reflects the latest *published* release, not the draft. `main` requires a pull request with passing CI to merge, can't be force-pushed or deleted, and external contributions auto-request review from [CODEOWNERS](.github/CODEOWNERS).

## Footprint

Measured on this machine (Apple Silicon, macOS) over a few seconds idle — a sense of scale for a menu-bar overlay, not a rigorous benchmark:

- **Binary size:** ~16 MB (unstripped release build)
- **Idle memory:** ~99 MB RSS
- **Idle CPU:** <1%

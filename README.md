# Layer HUD

[![CI](https://github.com/jonathanlaf/layer-hud/actions/workflows/ci.yml/badge.svg)](https://github.com/jonathanlaf/layer-hud/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/jonathanlaf/layer-hud?include_prereleases)](https://github.com/jonathanlaf/layer-hud/releases/latest)

A macOS overlay application that displays your ZSA Voyager keyboard layer state in real-time. Layer HUD fetches your keyboard layout from Oryx's GraphQL API and tracks live layer state via Keymapp/kontroll, renders layers on a transparent overlay, and mirrors the Oryx color scheme—keeping you informed of your current layer without interrupting your workflow.

## Prerequisites

- **Keymapp** ≥ 1.3.2 with API enabled (enable in Keymapp settings)
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

- **Tray Menu:** Click the Layer HUD icon in the macOS menu bar for Refresh layout, Pin overlay, Settings…, and Quit
- **Move Overlay:** Hold ⌘⌥ (Command+Option) and drag the overlay to reposition it
- **Settings:** Use the tray menu's "Settings…" item to access settings (Oryx layout URL, opacity, grab combo, color scheme); the overlay itself is click-through and doesn't respond to clicks
- **Layer Colors:** Layer background colors are mirrored from your Oryx layout if "Use Oryx Colors" is enabled in settings

## Troubleshooting

- **Offline Badge:** Ensure Keymapp's API is enabled in Keymapp settings (check the Keymapp UI for the API server status)
- **Multiple Clients:** Only one API client can connect to Keymapp at a time; close other applications using the API before launching Layer HUD
- **Layer Colors:** Layer colors are sourced from the Oryx layout (https://configure.zsa.io/). If colors appear missing or wrong, verify the Oryx layout URL in Layer HUD settings matches your Oryx layout

## Security & Provenance

Every release is built by GitHub Actions directly from this repository's source — never on a personal machine — and attested with [GitHub build provenance](https://docs.github.com/en/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds): a signed, verifiable statement of exactly which commit and workflow produced the binary. Before running a downloaded release, verify it:

```bash
gh attestation verify <path-to-downloaded-file>.dmg --owner jonathanlaf
```

A successful verification means the file you downloaded was built from this repository's source by this repository's CI — not tampered with in transit or substituted upstream.

CI only builds from tags reachable from `main` (enforced in CI, not just convention) and creates each release as a **draft** for manual review before it goes public — the version badge above reflects the latest *published* release, not the draft. `main` itself blocks direct pushes and requires a passing CI run to merge; external contributions additionally auto-request review from [CODEOWNERS](.github/CODEOWNERS).

## Footprint

Measured on this machine (Apple Silicon, macOS) over a few seconds idle — a sense of scale for a menu-bar overlay, not a rigorous benchmark:

- **Binary size:** ~16 MB (unstripped release build)
- **Idle memory:** ~99 MB RSS
- **Idle CPU:** <1%

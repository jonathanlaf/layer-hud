# KeyAura

[![CI](https://github.com/jonathanlaf/layer-hud/actions/workflows/ci.yml/badge.svg)](https://github.com/jonathanlaf/layer-hud/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/jonathanlaf/layer-hud?include_prereleases)](https://github.com/jonathanlaf/layer-hud/releases/latest)

A macOS menu-bar app that keeps your **ZSA Voyager** layout visible while you work. Its transparent, click-through overlay follows the active layer, highlights physical key presses, and shows layer triggers, alternate characters, and Shift hints. Voyager is the only supported keyboard today; support for more ZSA keyboards is planned.

This project is fully vibe coded as an experiment in building a useful native utility with [Claude](https://www.anthropic.com/claude) and [OpenAI Codex](https://openai.com/codex/).

## Icon attribution

The layer-target glyph is adapted from Streamline's [Layers 1 icon](https://github.com/webalys-hq/streamline-vectors/blob/main/plump/remix/interface-essential/layers-1.svg) by Webalys. It is bundled locally under the Streamline vector license. The remaining UI glyphs are original project artwork.

## How it works

KeyAura connects directly to the Voyager's Oryx Raw HID interface. The keyboard supplies its layout ID, flashed revision, active layer, and physical key-down/up events. **The labels and layer definitions still come from Oryx's online service**, not from reading the firmware binary. The app requests the flashed revision, not the latest unflashed edits.

After a successful fetch, the layout is cached locally. A matching cached revision can be used if Oryx is unavailable. A different firmware/layout revision needs its own successful fetch; KeyAura will not substitute an unrelated cached layout.

The `OFFLINE` pill means the keyboard's HID connection is unavailable. Oryx being down does not stop live key/layer events when a matching cache exists.

## Getting started

Download the app from [Releases](https://github.com/jonathanlaf/layer-hud/releases), open the DMG, and move KeyAura to Applications. Rust and Node are not needed to run a downloaded app.

Connect a Voyager running compatible Oryx firmware, and **close Keymapp and any browser Oryx live-training session** so they do not compete for the HID interface. The app detects the Voyager automatically; there is no layout URL to enter. An internet connection is needed for the first layout fetch.

This is a macOS-only app. Older macOS/WebKit versions and other ZSA keyboards have not been validated. With multiple Voyagers connected, the first available compatible interface is used; there is no device picker yet.

## Usage

- **Menu bar:** Refresh layout, open the icon/layer legend or Settings, pin the overlay for interaction, or quit. Debug builds also offer DevTools.
- **Move and resize:** Hold the grab shortcut (⌘⌥ by default) or enable Pin overlay. Drag the board or its corner handles. The app constrains proportions and saves position and size per monitor.
- **Appearance:** Configure fills, borders, opacity, shadows, pressed-key feedback, icon visibility and sizes, heatmap coloring/counts, and key spacing. Fonts have their own tab; enter an installed font family or leave it blank for the system font. The global ligature checkbox controls font shaping; italic letterforms depend on the selected font.
- **Position:** Center horizontally, vertically, or both; reset saved positions; change the gap between halves; choose the hidden edge, visible amount, and animation duration.
- **Toggle shortcut:** Record physical presses, then Stop. Repeated taps work, as do different keys pressed in order. Matching uses a maximum one-second gap between presses. It is an ordered sequence, not a system-wide chord/hold recognizer, and only receives keys from the connected Voyager. Record then Stop without pressing a key to clear it.
- **Hide/show:** The same sequence hides and restores the overlay. The real key layers are translated and clipped at the selected monitor edge to avoid showing the hidden portion on a neighboring display. Zero visible amount hides it completely; nonzero amounts retain at least approximately one key-width of visibility. Alignment, position reset, and geometry changes restore a hidden overlay before repositioning it.
- **Legend:** A separate window lists icon meanings and textual routes to each layer, including configured tap, hold, double-tap, and tap-and-hold actions. These are configured actions, not a live interpretation of firmware gesture timing.

Character translation currently includes macOS Canadian-CSA mappings plus standard keycode labels. It does not detect or reproduce every operating-system input source.

## Heatmap and privacy

Counts accumulate while KeyAura is running, even when heatmap coloring is disabled. Enable **Show key heatmap** in Appearance to tint frequently pressed keys; the saturation setting controls how many presses reach full intensity. **Show heatmap counts on keys** replaces the labels with numbers independently of the coloring toggle.

Counts are per physical key, shared across all layers and Voyagers; they are not separate totals for each firmware action. A double tap contributes two presses; a held key contributes one. Operating-system key repeat does not add presses. Reset heatmap history separately in Appearance.

Heatmap totals are stored locally in the overlay's WebKit storage and survive normal restarts. Saves are throttled to 250 ms and flushed on page exit; a crash or forced quit can lose the latest unsaved interval. No typed text or timestamped key history is stored. Key events and counts are not sent to Oryx; network requests send only the layout identity needed to retrieve labels.

## Settings and backups

General contains Start at login, JSON import/export, and Reset. Export writes into your macOS Downloads directory and displays the **actual saved path**. Repeated exports get numbered filenames and never overwrite earlier exports.

Import replaces the app's preferences, including saved monitor positions (restored on the next launch); out-of-range numbers and invalid colors are sanitized. Missing fields in older backups receive defaults. Reset restores default app preferences and recenters the overlay.

The JSON backup covers `config.json`, not the layout cache or heatmap history. Start at login is managed separately by macOS through the autostart plugin; it is not included in that file and is not changed by Reset.

The app's settings and layout cache are normally stored in:

```text
~/Library/Application Support/io.jonathanlaf.layerhud/config.json
~/Library/Application Support/io.jonathanlaf.layerhud/layout.json
```

Settings/cache from the old `io.jonathanlaf.voyagerhud` identifier are migrated when no new copy exists. Old caches without revision metadata may need one online refresh to establish that they match the flashed revision.

## Troubleshooting

- **No layout:** Connect the Voyager, close competing HID apps, and choose Refresh layout. If there has never been a successful fetch for this revision, Oryx must be reachable first.
- **OFFLINE:** Check the USB connection and firmware, and close Keymapp or browser training sessions. Detection retries automatically.
- **Wrong-looking characters:** Check your OS input source. The translator's Canadian-CSA mappings are not a universal mapping for all language layouts.
- **Hidden overlay:** Trigger the recorded sequence again, or use a positioning control in Settings to bring it back.
- **Corrupt settings:** Use General → Reset. Invalid/missing JSON falls back to defaults; keep a JSON export if you want to restore your custom preferences.

## Development

Install Rust/Cargo, Xcode Command Line Tools, and the Tauri CLI. Node 22 or later is used only for tests; the frontend is plain HTML/CSS/JavaScript with no npm dependencies or bundler. Neither Keymapp nor `protoc` is a build dependency.

```sh
xcode-select --install
cargo install tauri-cli --locked
cargo tauri dev
```

From the repository root, run the checks and build:

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml
node --test ui/test/*.test.mjs
cargo tauri build
```

Bundles are written beneath `src-tauri/target/release/bundle/macos/` and `src-tauri/target/release/bundle/dmg/`. The source audit and remaining verification work are recorded in [the v0.0.3 audit](docs/audit-v0.0.3.md). Documents under `docs/superpowers/` are historical plans, not descriptions of the current architecture.

## Releases and provenance

Merging a PR into `main` runs CI; **it does not create a release by itself**. Update both `Cargo.toml` and `tauri.conf.json`, commit the updated lockfile, then push a matching `vX.Y.Z` tag after that commit reaches `main`.

The release workflow checks tag ancestry and version consistency, builds the app, creates a **draft release with generated notes**, and attests the DMG. A tag that is not reachable from `main` is rejected and deleted by the workflow. Drafts require manual review/publication; the version badge shows the latest published release.

Verify a downloaded DMG's GitHub build provenance with:

```sh
gh attestation verify <downloaded-file>.dmg --owner jonathanlaf
```

This verifies the build's repository/workflow provenance, not that the application is bug-free. GitHub attestation is separate from Apple Developer ID signing and notarization, which the current workflow does not configure; macOS may show a security warning.

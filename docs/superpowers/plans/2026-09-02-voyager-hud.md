# Voyager HUD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** macOS always-on-top transparent overlay showing the live ZSA Voyager keymap with the active layer highlighted via the Keymapp gRPC API.

**Architecture:** Tauri v2 menu-bar app. Rust backend: kontroll-based layer watcher (10 Hz poll → events), modifier-flags grab manager (click-through toggle), Oryx GraphQL layout fetcher with cache, JSON config. Frontend: plain HTML/CSS/JS rendering all layers once, layer switch = CSS flip.

**Tech Stack:** Rust, Tauri 2, kontroll 1.0.3 (tonic/gRPC), reqwest, plain JS (ES modules), `node --test` for JS units, `cargo test` for Rust units.

**Spec:** `docs/superpowers/specs/2026-09-02-voyager-hud-design.md`

## Global Constraints

- macOS only; transparency requires `"macOSPrivateApi": true` in `tauri.conf.json`.
- Build prerequisites: Rust toolchain, `protoc` (`brew install protobuf`) — required by kontroll's tonic-build; Tauri CLI (`cargo install tauri-cli --locked`).
- No JS framework, no npm dependencies. JS is ES modules; shared logic lives in `.mjs` files imported by both the webview and `node --test`.
- No Dock icon: `ActivationPolicy::Accessory`.
- Default config: opacity `0.85`, grab combo `["cmd","alt"]`, `use_oryx_colors: true`, Oryx layout hash `Br3gO`.
- kontroll `Status` fields are private — read values via `serde_json::to_value(&status)`.
- Frontend uses `withGlobalTauri: true` (`window.__TAURI__`), no bundler.
- User rule: run the `code-review` skill before every commit.

---

### Task 1: Tauri scaffold with transparent overlay window

**Files:**
- Create: `.gitignore`, `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/icons/icon.png` (any 512×512 PNG placeholder; generate with `tauri icon` later), `ui/index.html`, `ui/style.css`
- Test: manual (`cargo tauri dev`)

**Interfaces:**
- Produces: window label `"overlay"`; `ui/` as `frontendDist`; `run()` wiring in `main.rs` that later tasks extend with `.invoke_handler` / `.setup` additions.

- [ ] **Step 1: Prerequisites**

```bash
brew list protobuf >/dev/null || brew install protobuf
cargo install tauri-cli --locked
```

- [ ] **Step 2: Write scaffold files**

`.gitignore`:
```
/src-tauri/target
/src-tauri/gen
.DS_Store
```

`src-tauri/Cargo.toml`:
```toml
[package]
name = "voyager-hud"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
kontroll = "1.0.3"
reqwest = { version = "0.12", features = ["json"] }
```

`src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build()
}
```

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "voyager-hud",
  "version": "0.1.0",
  "identifier": "io.jonathanlaf.voyagerhud",
  "build": {
    "frontendDist": "../ui"
  },
  "app": {
    "macOSPrivateApi": true,
    "withGlobalTauri": true,
    "windows": [
      {
        "label": "overlay",
        "title": "Voyager HUD",
        "width": 940,
        "height": 380,
        "transparent": true,
        "decorations": false,
        "alwaysOnTop": true,
        "shadow": false,
        "resizable": true,
        "skipTaskbar": true,
        "acceptFirstMouse": true
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": ["app"],
    "icon": ["icons/icon.png"]
  }
}
```

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let overlay = app.get_webview_window("overlay").expect("overlay window");
            overlay.set_ignore_cursor_events(true)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running voyager-hud");
}
```

`ui/index.html`:
```html
<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <link rel="stylesheet" href="style.css" />
  <title>Voyager HUD</title>
</head>
<body>
  <div id="board">voyager-hud scaffold</div>
</body>
</html>
```

`ui/style.css`:
```css
html, body { margin: 0; background: transparent; overflow: hidden; }
#board { color: #fff; font: 13px -apple-system, sans-serif;
         background: rgba(20, 20, 24, var(--hud-opacity, 0.85));
         border-radius: 12px; padding: 10px; height: calc(100vh - 20px); }
```

For the icon: any 512×512 PNG (e.g. `python3 -c "..."` one-liner below).

```bash
python3 -c "
import zlib,struct
w=h=512
raw=b''.join(b'\x00'+b'\x28\x28\x30\xff'*w for _ in range(h))
def c(t,d):return struct.pack('>I',len(d))+t+d+struct.pack('>I',zlib.crc32(t+d))
png=b'\x89PNG\r\n\x1a\n'+c(b'IHDR',struct.pack('>IIBBBBB',w,h,8,6,0,0,0))+c(b'IDAT',zlib.compress(raw))+c(b'IEND',b'')
open('src-tauri/icons/icon.png','wb').write(png)"
```

- [ ] **Step 3: Verify it builds and shows the overlay**

Run: `cd src-tauri && cargo tauri dev`
Expected: a borderless translucent dark rounded rectangle floats above all windows; clicks pass through it (it starts click-through, quit via Ctrl-C in the terminal).

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: tauri scaffold with transparent click-through overlay"
```

---

### Task 2: Config module (Rust)

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/main.rs` (add `mod config;`)
- Test: inline `#[cfg(test)]` in `config.rs`

**Interfaces:**
- Produces:
  - `Config { oryx_url: String, opacity: f64, grab_combo: Vec<String>, use_oryx_colors: bool, window: Option<WindowRect>, last_refresh: Option<String> }` (all `pub`, derives `Serialize, Deserialize, Clone, Debug, PartialEq`)
  - `WindowRect { x: f64, y: f64, w: f64, h: f64 }` (same derives)
  - `impl Default for Config`
  - `pub fn load(path: &std::path::Path) -> Config` (missing/corrupt file → default)
  - `pub fn save(path: &std::path::Path, cfg: &Config) -> std::io::Result<()>` (creates parent dirs)

- [ ] **Step 1: Write the failing tests** (bottom of `src-tauri/src/config.rs`, module skeleton above them)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let c = Config::default();
        assert_eq!(c.oryx_url, "Br3gO");
        assert_eq!(c.opacity, 0.85);
        assert_eq!(c.grab_combo, vec!["cmd".to_string(), "alt".to_string()]);
        assert!(c.use_oryx_colors);
        assert!(c.window.is_none());
    }

    #[test]
    fn save_load_round_trip() {
        let dir = std::env::temp_dir().join("vhud-test-config");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("config.json");
        let mut c = Config::default();
        c.opacity = 0.5;
        c.window = Some(WindowRect { x: 10.0, y: 20.0, w: 800.0, h: 300.0 });
        save(&path, &c).unwrap();
        assert_eq!(load(&path), c);
    }

    #[test]
    fn load_missing_or_corrupt_returns_default() {
        assert_eq!(load(std::path::Path::new("/nonexistent/vhud.json")), Config::default());
        let dir = std::env::temp_dir().join("vhud-test-corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(load(&path), Config::default());
    }
}
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cd src-tauri && cargo test config`
Expected: compile error (types not defined).

- [ ] **Step 3: Implement**

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WindowRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Config {
    pub oryx_url: String,
    pub opacity: f64,
    pub grab_combo: Vec<String>,
    pub use_oryx_colors: bool,
    pub window: Option<WindowRect>,
    pub last_refresh: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            oryx_url: "Br3gO".into(),
            opacity: 0.85,
            grab_combo: vec!["cmd".into(), "alt".into()],
            use_oryx_colors: true,
            window: None,
            last_refresh: None,
        }
    }
}

pub fn load(path: &Path) -> Config {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, cfg: &Config) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(cfg).expect("serialize config"))
}
```

Add `mod config;` at the top of `main.rs`.

- [ ] **Step 4: Run tests, verify pass**

Run: `cd src-tauri && cargo test config`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: config module with defaults and round-trip persistence"
```

---

### Task 3: Oryx layout fetcher (Rust)

**Files:**
- Create: `src-tauri/src/oryx.rs`
- Modify: `src-tauri/src/main.rs` (add `mod oryx;`, register commands, state)
- Test: inline `#[cfg(test)]` in `oryx.rs`

**Interfaces:**
- Consumes: `config::{Config, load, save}`.
- Produces:
  - `pub fn parse_layout_hash(input: &str) -> Option<String>`
  - Tauri command `refresh_layout(app: AppHandle, url: String) -> Result<serde_json::Value, String>` — fetches from Oryx, caches to `<app_config_dir>/layout.json`, updates `config.oryx_url` + `last_refresh`; on network failure returns cache with `"stale": true` injected at the top level, or `Err` if no cache.
  - Tauri command `load_layout(app: AppHandle) -> Result<serde_json::Value, String>` — cache if present, else calls the fetch path with the configured hash.
  - Tauri command `get_config(app) -> Config` and helper `pub fn config_path(app: &AppHandle) -> PathBuf` (`app.path().app_config_dir().unwrap().join("config.json")`).

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::parse_layout_hash;

    #[test]
    fn parses_full_url() {
        assert_eq!(
            parse_layout_hash("https://configure.zsa.io/voyager/layouts/Br3gO/latest/0"),
            Some("Br3gO".into())
        );
    }

    #[test]
    fn parses_url_without_revision() {
        assert_eq!(
            parse_layout_hash("configure.zsa.io/voyager/layouts/gLwvw"),
            Some("gLwvw".into())
        );
    }

    #[test]
    fn parses_bare_hash() {
        assert_eq!(parse_layout_hash("Br3gO"), Some("Br3gO".into()));
        assert_eq!(parse_layout_hash("  Br3gO "), Some("Br3gO".into()));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_layout_hash(""), None);
        assert_eq!(parse_layout_hash("https://example.com/foo"), None);
        assert_eq!(parse_layout_hash("has spaces"), None);
    }
}
```

- [ ] **Step 2: Run, verify failure**

Run: `cd src-tauri && cargo test oryx`
Expected: compile error.

- [ ] **Step 3: Implement**

```rust
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const GRAPHQL_URL: &str = "https://oryx.zsa.io/graphql";
const QUERY: &str = "query getLayout($hashId: String!, $revisionId: String!, $geometry: String) { layout(hashId: $hashId, revisionId: $revisionId, geometry: $geometry) { title revision { title config layers { position title keys } } } }";

pub fn parse_layout_hash(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').collect();
        let idx = parts.iter().position(|p| *p == "layouts")?;
        let hash = parts.get(idx + 1)?;
        return if hash.chars().all(|c| c.is_ascii_alphanumeric()) && !hash.is_empty() {
            Some((*hash).to_string())
        } else {
            None
        };
    }
    if s.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(s.to_string())
    } else {
        None
    }
}

pub fn config_path(app: &AppHandle) -> PathBuf {
    app.path().app_config_dir().expect("config dir").join("config.json")
}

fn cache_path(app: &AppHandle) -> PathBuf {
    app.path().app_config_dir().expect("config dir").join("layout.json")
}

async fn fetch_from_oryx(hash: &str) -> Result<Value, String> {
    let body = json!({
        "query": QUERY,
        "variables": { "hashId": hash, "revisionId": "latest", "geometry": "voyager" }
    });
    let resp: Value = reqwest::Client::new()
        .post(GRAPHQL_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?
        .json()
        .await
        .map_err(|e| format!("bad response: {e}"))?;
    if resp.pointer("/data/layout").map(|v| v.is_null()).unwrap_or(true) {
        return Err(format!("layout '{hash}' not found on Oryx"));
    }
    Ok(resp)
}

#[tauri::command]
pub async fn refresh_layout(app: AppHandle, url: String) -> Result<Value, String> {
    let hash = parse_layout_hash(&url).ok_or("not a valid Oryx layout URL or hash")?;
    let cache = cache_path(&app);
    match fetch_from_oryx(&hash).await {
        Ok(v) => {
            let mut c = crate::config::load(&config_path(&app));
            c.oryx_url = hash;
            c.last_refresh = Some(chrono_free_now());
            crate::config::save(&config_path(&app), &c).map_err(|e| e.to_string())?;
            std::fs::write(&cache, serde_json::to_string(&v).unwrap()).map_err(|e| e.to_string())?;
            Ok(v)
        }
        Err(e) => {
            let cached = std::fs::read_to_string(&cache).map_err(|_| e.clone())?;
            let mut v: Value = serde_json::from_str(&cached).map_err(|_| e)?;
            v["stale"] = json!(true);
            Ok(v)
        }
    }
}

#[tauri::command]
pub async fn load_layout(app: AppHandle) -> Result<Value, String> {
    let cache = cache_path(&app);
    if let Ok(s) = std::fs::read_to_string(&cache) {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            return Ok(v);
        }
    }
    let hash = crate::config::load(&config_path(&app)).oryx_url;
    refresh_layout(app, hash).await
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> crate::config::Config {
    crate::config::load(&config_path(&app))
}

fn chrono_free_now() -> String {
    // ISO-ish timestamp without a chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{now}")
}
```

In `main.rs` add `mod oryx;` and register:

```rust
        .invoke_handler(tauri::generate_handler![
            oryx::refresh_layout,
            oryx::load_layout,
            oryx::get_config
        ])
```

- [ ] **Step 4: Run tests, verify pass**

Run: `cd src-tauri && cargo test oryx`
Expected: 4 passed. Also `cargo check` clean.

- [ ] **Step 5: Manual fetch check**

Run: `cd src-tauri && cargo tauri dev`, then in the overlay's devtools console (right-click → Inspect):
`await window.__TAURI__.core.invoke('load_layout')`
Expected: JSON with `data.layout.revision.layers` (4 layers).

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: oryx layout fetcher with cache and stale fallback"
```

---

### Task 4: Keycode translator (JS)

**Files:**
- Create: `ui/translator.mjs`, `ui/test/translator.test.mjs`
- Test: `node --test ui/test/`

**Interfaces:**
- Produces: `translateSlot(slot) -> string` where `slot` is an Oryx key-slot object `{ code, modifiers?, layer?, macro?, customLabel? }` (a key object's `tap`/`hold`/`doubleTap`/`tapHold` value plus the key-level `customLabel`). Also exports `BASE_TABLE`, `COMBO_TABLE` for the renderer's reuse.
- Label precedence: `customLabel` → layer ref (`MO`/`LT`/`TG` → `L<n>`) → macro (`⌘M` style name or `MACRO`) → modifier-combo table → base table → cleaned keycode.

- [ ] **Step 1: Write the failing tests**

```js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { translateSlot } from '../translator.mjs';

test('custom label wins', () => {
  assert.equal(translateSlot({ code: 'CSA_EGRV', customLabel: 'BS' }), 'BS');
});

test('verified CSA modifier combos', () => {
  const alt = { rightAlt: true };
  assert.equal(translateSlot({ code: 'CSA_EGRV', modifiers: alt }), '\\');
  assert.equal(translateSlot({ code: 'KC_MINUS', modifiers: alt }), '|');
  assert.equal(translateSlot({ code: 'CSA_ECUT', modifiers: alt }), '/');
  assert.equal(translateSlot({ code: 'CSA_AGRV', modifiers: alt }), '`');
});

test('base CSA and plain codes', () => {
  assert.equal(translateSlot({ code: 'KC_A' }), 'A');
  assert.equal(translateSlot({ code: 'KC_COLN' }), ':');
  assert.equal(translateSlot({ code: 'CSA_QEST' }), '?');
  assert.equal(translateSlot({ code: 'CSA_ECUT' }), 'é');
  assert.equal(translateSlot({ code: 'CSA_CCED' }), 'ç');
  assert.equal(translateSlot({ code: 'KC_NO' }), '');
});

test('dead keys are marked', () => {
  assert.equal(translateSlot({ code: 'CSA_DCRC' }), '^̲'); // combining low line marks dead
  assert.equal(translateSlot({ code: 'CSA_DGRV' }), '`̲');
});

test('layer references', () => {
  assert.equal(translateSlot({ code: 'MO', layer: 2 }), 'L2');
  assert.equal(translateSlot({ code: 'TG', layer: 1 }), 'L1');
});

test('macros show a name', () => {
  assert.equal(
    translateSlot({ code: 'KC_TRANSPARENT', macro: { keys: [{ code: 'KC_B', modifiers: { leftCtrl: true } }, { code: 'KC_1' }] } }),
    '⌃B 1'
  );
});

test('unknown code falls back to cleaned name', () => {
  assert.equal(translateSlot({ code: 'KC_MEDIA_PLAY_PAUSE' }), '⏯');
  assert.equal(translateSlot({ code: 'KC_SOMETHING_NEW' }), 'SOMETHING NEW');
});
```

- [ ] **Step 2: Run, verify failure**

Run: `node --test ui/test/`
Expected: FAIL, cannot find `../translator.mjs`.

- [ ] **Step 3: Implement `ui/translator.mjs`**

```js
// Base table: Oryx keycode -> what macOS Canadian-CSA actually outputs.
// Verified against the OS layout tables (UCKeyTranslate dump, 2026-09-01).
export const BASE_TABLE = {
  KC_NO: '', KC_TRANSPARENT: '',
  KC_ESCAPE: '⎋', KC_TAB: '⇥', KC_ENTER: '⏎', KC_SPACE: '␣', KC_BSPC: '⌫',
  KC_DELETE: '⌦', KC_CAPS_LOCK: '⇪',
  KC_LEFT_SHIFT: '⇧', KC_RIGHT_SHIFT: '⇧', KC_LEFT_CTRL: '⌃', KC_RIGHT_CTRL: '⌃',
  KC_LEFT_ALT: '⌥', KC_RIGHT_ALT: '⌥', KC_LEFT_GUI: '⌘', KC_RIGHT_GUI: '⌘',
  KC_LEFT: '←', KC_RIGHT: '→', KC_UP: '↑', KC_DOWN: '↓',
  KC_HOME: '⇱', KC_END: '⇲', KC_PAGE_UP: '⇞', KC_PGDN: '⇟',
  KC_MINUS: '-', KC_EQUAL: '=', KC_COMMA: ',', KC_DOT: '.',
  KC_COLN: ':', KC_SCLN: ';', KC_UNDS: '_',
  KC_LPRN: '(', KC_RPRN: ')',
  KC_KP_PLUS: '+', KC_KP_MINUS: '-', KC_KP_ASTERISK: '*', KC_KP_SLASH: '/',
  KC_KP_DOT: '.', KC_KP_EQUAL: '=',
  KC_MEDIA_PLAY_PAUSE: '⏯', KC_MEDIA_NEXT_TRACK: '⏭', KC_MEDIA_PREV_TRACK: '⏮',
  KC_MEDIA_STOP: '⏹', KC_AUDIO_VOL_UP: '🔊', KC_AUDIO_VOL_DOWN: '🔉', KC_AUDIO_MUTE: '🔇',
  MAC_SPOTLIGHT: '🔍',
  // CSA pack (Oryx CMS keycodes), values = macOS CSA output
  CSA_ECUT: 'é', CSA_EGRV: 'è', CSA_AGRV: 'à', CSA_CCED: 'ç', CSA_UGRV: 'ù',
  CSA_APOS: "'", CSA_DQOT: '"', CSA_QEST: '?',
  CSA_LGIL: '«', CSA_RGIL: '»', CSA_LESS: '<', CSA_GRTR: '>',
  CSA_LBRC: '[', CSA_RBRC: ']', CSA_LCBR: '{', CSA_RCBR: '}',
  CSA_DTLD: '~', CSA_BSLS: '\\', CSA_PIPE: '|', CSA_SLSH: '/',
};

const DEAD_MARK = '̲'; // combining low line under the char
const DEAD_TABLE = {
  CSA_DCRC: '^' + DEAD_MARK,
  CSA_DGRV: '`' + DEAD_MARK,
  CSA_DTRM: '¨' + DEAD_MARK,
};

// Combos verified on macOS Canadian-CSA (swap-immune set).
// Key = `${code}+${mods}` with mods letters in C,S,A,G order.
export const COMBO_TABLE = {
  'CSA_EGRV+A': '\\',
  'KC_MINUS+A': '|',
  'CSA_ECUT+A': '/',
  'CSA_AGRV+A': '`',
  'KC_6+S': '?',
  'KC_MINUS+S': '_',
  'KC_COMMA+S': "'",
  'KC_DOT+S': '"',
  'KC_EQUAL+S': '+',
};

const MOD_SYMBOL = { C: '⌃', S: '⇧', A: '⌥', G: '⌘' };

function modString(modifiers) {
  if (!modifiers) return '';
  let s = '';
  if (modifiers.leftCtrl || modifiers.rightCtrl) s += 'C';
  if (modifiers.leftShift || modifiers.rightShift) s += 'S';
  if (modifiers.leftAlt || modifiers.rightAlt) s += 'A';
  if (modifiers.leftGui || modifiers.rightGui) s += 'G';
  return s;
}

function cleanCode(code) {
  return code.replace(/^(KC|CSA|MAC)_/, '').replace(/_/g, ' ');
}

export function translateSlot(slot) {
  if (!slot) return '';
  if (slot.customLabel) return slot.customLabel;
  if (slot.layer !== null && slot.layer !== undefined) return `L${slot.layer}`;
  if (slot.macro && Array.isArray(slot.macro.keys)) {
    const parts = slot.macro.keys
      .filter((k) => k.code !== 'KC_TRANSPARENT')
      .map((k) => {
        const mods = modString(k.modifiers)
          .split('')
          .map((m) => MOD_SYMBOL[m])
          .join('');
        return mods + cleanCode(k.code);
      });
    return parts.length ? parts.join(' ') : 'MACRO';
  }
  const mods = modString(slot.modifiers);
  if (mods) {
    const combo = COMBO_TABLE[`${slot.code}+${mods}`];
    if (combo) return combo;
    const base = DEAD_TABLE[slot.code] ?? BASE_TABLE[slot.code] ?? cleanCode(slot.code);
    return mods.split('').map((m) => MOD_SYMBOL[m]).join('') + base;
  }
  if (slot.code in DEAD_TABLE) return DEAD_TABLE[slot.code];
  if (slot.code in BASE_TABLE) return BASE_TABLE[slot.code];
  const m = slot.code.match(/^KC_([A-Z0-9])$/);
  if (m) return m[1];
  if (/^KC_F\d+$/.test(slot.code)) return slot.code.slice(3);
  return cleanCode(slot.code);
}
```

- [ ] **Step 4: Run tests, verify pass**

Run: `node --test ui/test/`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: CSA-aware keycode translator with verified combo table"
```

---

### Task 5: Geometry + renderer (JS) — overlay shows all layers

**Files:**
- Create: `ui/geometry.mjs`, `ui/hud.js`, `ui/test/geometry.test.mjs`
- Modify: `ui/index.html`, `ui/style.css`
- Test: `node --test ui/test/` + manual `cargo tauri dev`

**Interfaces:**
- Consumes: `translateSlot` (Task 4); `load_layout`, `get_config` commands (Task 3).
- Produces: `keyRects() -> Array<{x,y,w,h}>` of length 52 in "key units" (Oryx key order: left half rows top-to-bottom 0–23 then 2 thumbs 24–25; right half likewise 26–49 then thumbs 50–51; every row listed visually left→right); `renderBoard(layoutJson, config)` builds the DOM; `setActiveLayer(n)`; `setOffline(flag)`. `hud.js` wires these to Tauri events on load.

- [ ] **Step 1: Write the failing geometry tests**

```js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { keyRects } from '../geometry.mjs';

test('52 keys', () => {
  assert.equal(keyRects().length, 52);
});

test('halves are separated by a split gap', () => {
  const r = keyRects();
  const leftMax = Math.max(...r.slice(0, 26).map((k) => k.x + k.w));
  const rightMin = Math.min(...r.slice(26, 52).map((k) => k.x));
  assert.ok(rightMin - leftMax >= 1.0, `gap ${rightMin - leftMax}`);
});

test('rows are top to bottom within a half', () => {
  const r = keyRects();
  assert.ok(r[0].y < r[6].y && r[6].y < r[12].y && r[12].y < r[18].y);
  assert.ok(r[24].y > r[18].y, 'thumbs below bottom row');
});

test('no overlapping keys', () => {
  const r = keyRects();
  for (let i = 0; i < r.length; i++)
    for (let j = i + 1; j < r.length; j++) {
      const a = r[i], b = r[j];
      const overlap = a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
      assert.ok(!overlap, `keys ${i} and ${j} overlap`);
    }
});
```

- [ ] **Step 2: Run, verify failure**

Run: `node --test ui/test/`
Expected: geometry tests FAIL (module missing).

- [ ] **Step 3: Implement `ui/geometry.mjs`**

Column stagger in key units, visual starting point (tuned by eye in Step 6):

```js
const KEY = 1.0;          // key cell (includes gap)
const SPLIT_X = 7.6;      // right half x origin
// vertical offset per column, outer pinky -> inner column
const LEFT_STAGGER = [0.45, 0.35, 0.12, 0.0, 0.12, 0.28];
const RIGHT_STAGGER = [0.28, 0.12, 0.0, 0.12, 0.35, 0.45];

export function keyRects() {
  const rects = [];
  for (let row = 0; row < 4; row++)
    for (let col = 0; col < 6; col++)
      rects.push({ x: col * KEY, y: row * KEY + LEFT_STAGGER[col], w: 0.94, h: 0.94 });
  rects.push({ x: 4.35, y: 4.35, w: 0.94, h: 0.94 });  // 24: left thumb inner
  rects.push({ x: 5.45, y: 4.65, w: 0.94, h: 1.1 });   // 25: left thumb outer
  for (let row = 0; row < 4; row++)
    for (let col = 0; col < 6; col++)
      rects.push({ x: SPLIT_X + col * KEY, y: row * KEY + RIGHT_STAGGER[col], w: 0.94, h: 0.94 });
  rects.push({ x: SPLIT_X - 0.25, y: 4.65, w: 0.94, h: 1.1 }); // 50: right thumb outer
  rects.push({ x: SPLIT_X + 0.85, y: 4.35, w: 0.94, h: 0.94 }); // 51: right thumb inner
  return rects;
}

export const BOARD_UNITS = { w: SPLIT_X + 6, h: 6.0 };
```

- [ ] **Step 4: Run tests, verify geometry passes**

Run: `node --test ui/test/`
Expected: all pass (adjust thumb coordinates if the overlap test complains — keep the split-gap and row-order invariants true).

- [ ] **Step 5: Implement renderer + wire-up**

`ui/hud.js`:

```js
import { keyRects, BOARD_UNITS } from './geometry.mjs';
import { translateSlot } from './translator.mjs';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

export function renderBoard(layoutJson, config) {
  const layers = layoutJson.data.layout.revision.layers;
  const board = document.getElementById('board');
  board.innerHTML = '';
  const rects = keyRects();
  const badge = document.createElement('div');
  badge.id = 'badge';
  board.appendChild(badge);
  for (const layer of layers) {
    const el = document.createElement('div');
    el.className = 'layer';
    el.dataset.layer = layer.position;
    el.dataset.name = layer.title || `Layer ${layer.position}`;
    layer.keys.forEach((key, i) => {
      const r = rects[i];
      const k = document.createElement('div');
      k.className = 'key';
      k.style.cssText = `left:${r.x * 62}px;top:${r.y * 62}px;width:${r.w * 62}px;height:${r.h * 62}px`;
      if (config.use_oryx_colors && key.glowColor) k.style.background = hexTint(key.glowColor);
      const custom = key.customLabel;
      const tap = document.createElement('span');
      tap.className = 'tap';
      tap.textContent = translateSlot(key.tap ? { ...key.tap, customLabel: custom } : key.tap);
      k.appendChild(tap);
      for (const [slot, cls] of [['hold', 'hold'], ['doubleTap', 'dtap'], ['tapHold', 'thold']]) {
        if (key[slot]) {
          const s = document.createElement('span');
          s.className = cls;
          s.textContent = translateSlot(key[slot]);
          k.appendChild(s);
          if (key[slot].layer !== null && key[slot].layer !== undefined)
            k.dataset.triggersLayer = key[slot].layer;
        }
      }
      el.appendChild(k);
    });
    board.appendChild(el);
  }
  setActiveLayer(0);
}

function hexTint(hex) {
  const n = parseInt(hex.replace('#', ''), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  return `rgba(${r},${g},${b},0.25)`;
}

export function setActiveLayer(n) {
  document.querySelectorAll('.layer').forEach((el) => {
    el.classList.toggle('active', Number(el.dataset.layer) === n);
  });
  document.querySelectorAll(`[data-triggers-layer]`).forEach((el) => {
    el.classList.toggle('trigger-active', Number(el.dataset.triggersLayer) === n);
  });
  const active = document.querySelector('.layer.active');
  document.getElementById('badge').textContent = active ? active.dataset.name : `Layer ${n}`;
  document.body.dataset.base = n === 0 ? '1' : '0';
}

export function setOffline(off) {
  document.body.classList.toggle('offline', off);
}

async function main() {
  const config = await invoke('get_config');
  document.documentElement.style.setProperty('--hud-opacity', config.opacity);
  const layout = await invoke('load_layout');
  renderBoard(layout, config);
  if (layout.stale) document.getElementById('badge').textContent += ' (cached)';
  await listen('layer-changed', (e) => setActiveLayer(e.payload.layer));
  await listen('keymapp-offline', () => setOffline(true));
  await listen('keymapp-online', () => setOffline(false));
  await listen('grab-mode', (e) => document.body.classList.toggle('grab', e.payload.on));
  await listen('config-changed', async (e) => {
    document.documentElement.style.setProperty('--hud-opacity', e.payload.opacity);
    renderBoard(await invoke('load_layout'), e.payload);
  });
  await listen('layout-refreshed', async () => {
    renderBoard(await invoke('load_layout'), await invoke('get_config'));
  });
}
main();
```

`ui/index.html` body becomes:

```html
<body>
  <div id="board"></div>
  <script type="module" src="hud.js"></script>
</body>
```

Append to `ui/style.css`:

```css
#board { position: relative; }
.layer { display: none; }
.layer.active { display: block; }
.key { position: absolute; border: 1px solid rgba(255,255,255,0.35); border-radius: 7px;
       box-sizing: border-box; text-align: center; }
.key .tap { position: absolute; inset: 0; display: flex; align-items: center;
            justify-content: center; font-size: 16px; }
.key .hold { position: absolute; bottom: 2px; left: 0; right: 0; font-size: 9px; opacity: 0.75; }
.key .dtap { position: absolute; top: 1px; right: 4px; font-size: 9px; opacity: 0.75; }
.key .thold { position: absolute; top: 1px; left: 4px; font-size: 9px; opacity: 0.75; }
.key.trigger-active { border-color: #ffd479; box-shadow: 0 0 6px #ffd479; }
#badge { position: absolute; top: 4px; right: 10px; font-size: 11px; opacity: 0.8; }
body[data-base="0"] #board { outline: 2px solid rgba(120,180,255,0.6); outline-offset: -2px; }
body.offline #board { opacity: 0.4; }
body.offline #badge::after { content: " — Keymapp offline"; color: #ff9c9c; }
body.grab #board { outline: 2px solid rgba(255,220,120,0.9); outline-offset: -2px; }
```

- [ ] **Step 6: Manual visual check**

Run: `cd src-tauri && cargo tauri dev`
Expected: both halves with your Colemak-DH base layer, correct legends (`é`, `?`, `:` etc.), Oryx colors tinting colored keys. Tune `geometry.mjs` constants if key positions look off (re-run `node --test ui/test/` after tuning).

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat: voyager board renderer with layers, legends and oryx colors"
```

---

### Task 6: Layer watcher (Rust) — live layer switching

**Files:**
- Create: `src-tauri/src/watcher.rs`
- Modify: `src-tauri/src/main.rs` (add `mod watcher;`, spawn in setup)
- Test: inline `#[cfg(test)]` for `extract_layer`; live behavior manual with Keymapp

**Interfaces:**
- Consumes: kontroll (`Kontroll::new(None).await`, `get_status()`); `Status` fields are private → serialize.
- Produces: `pub fn extract_layer(status: &serde_json::Value) -> Option<i64>`; `pub fn spawn(app: AppHandle)` — emits `layer-changed {layer}`, `keymapp-offline`, `keymapp-online` (payloads consumed by Task 5's listeners).

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::extract_layer;
    use serde_json::json;

    #[test]
    fn extracts_current_layer() {
        let v = json!({"keymapp_version":"1.3.2","kontroll_version":"1.0.3",
            "keyboard":{"friendly_name":"Voyager","firmware_version":"x","current_layer":2}});
        assert_eq!(extract_layer(&v), Some(2));
    }

    #[test]
    fn none_when_no_keyboard() {
        let v = json!({"keymapp_version":"1.3.2","kontroll_version":"1.0.3","keyboard":null});
        assert_eq!(extract_layer(&v), None);
    }
}
```

- [ ] **Step 2: Run, verify failure**

Run: `cd src-tauri && cargo test watcher`
Expected: compile error.

- [ ] **Step 3: Implement**

```rust
use kontroll::Kontroll;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub fn extract_layer(status: &Value) -> Option<i64> {
    status.pointer("/keyboard/current_layer").and_then(Value::as_i64)
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_layer: Option<i64> = None;
        let mut online = true;
        let mut backoff = Duration::from_millis(250);
        loop {
            let status = match Kontroll::new(None).await {
                Ok(api) => api.get_status().await.ok(),
                Err(_) => None,
            };
            match status.and_then(|s| serde_json::to_value(&s).ok()) {
                Some(v) => match extract_layer(&v) {
                    Some(layer) => {
                        if !online {
                            online = true;
                            let _ = app.emit("keymapp-online", json!({}));
                        }
                        backoff = Duration::from_millis(250);
                        if last_layer != Some(layer) {
                            last_layer = Some(layer);
                            let _ = app.emit("layer-changed", json!({ "layer": layer }));
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    None => sleep_offline(&app, &mut online, &mut backoff).await,
                },
                None => sleep_offline(&app, &mut online, &mut backoff).await,
            }
        }
    });
}

async fn sleep_offline(app: &AppHandle, online: &mut bool, backoff: &mut Duration) {
    if *online {
        *online = false;
        let _ = app.emit("keymapp-offline", json!({}));
    }
    tokio::time::sleep(*backoff).await;
    *backoff = (*backoff * 2).min(Duration::from_secs(5));
}
```

In `main.rs` setup closure, after the overlay lookup: `watcher::spawn(app.handle().clone());` (and `mod watcher;` on top).

Note: reconnecting `Kontroll::new` each cycle is deliberate — the socket may vanish between polls; a fresh UDS connect is sub-millisecond. If profiling ever shows churn, keep the client across iterations and rebuild only on error.

- [ ] **Step 4: Run tests, verify pass**

Run: `cd src-tauri && cargo test watcher`
Expected: 2 passed.

- [ ] **Step 5: Manual live check**

Keymapp running, API enabled (Keymapp → Settings → Enable API), Voyager connected.
Run: `cd src-tauri && cargo tauri dev`
Expected: holding your Enter-thumb (L1) switches the overlay to the symbols layer and back; quitting Keymapp dims the overlay with the offline badge; relaunching recovers.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: live layer watcher via keymapp api with offline backoff"
```

---

### Task 7: Grab manager (Rust) — ⌘⌥ makes the overlay movable

**Files:**
- Create: `src-tauri/src/grab.rs`
- Modify: `src-tauri/src/main.rs` (add `mod grab;`, spawn in setup), `docs/superpowers/specs/2026-09-02-voyager-hud-design.md` (replace the `device_query` mention: modifier state is read with a direct `CGEventSourceFlagsState` FFI call — zero extra crates and, unlike key-state polling, requires no Input Monitoring permission)
- Test: inline `#[cfg(test)]` for combo matching; grab behavior manual

**Interfaces:**
- Consumes: `config::Config.grab_combo` (`Vec<String>`, values from `{"cmd","alt","ctrl","shift"}`); overlay window handle.
- Produces: `pub fn combo_mask(names: &[String]) -> u64`, `pub fn combo_active(flags: u64, mask: u64) -> bool`, `pub fn spawn(app: AppHandle)` — toggles `set_ignore_cursor_events` and emits `grab-mode {on}`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_map_to_cg_flags() {
        assert_eq!(combo_mask(&["cmd".into()]), MASK_CMD);
        assert_eq!(combo_mask(&["cmd".into(), "alt".into()]), MASK_CMD | MASK_ALT);
        assert_eq!(combo_mask(&["bogus".into()]), 0);
    }

    #[test]
    fn combo_requires_all_and_nonempty() {
        let m = MASK_CMD | MASK_ALT;
        assert!(combo_active(MASK_CMD | MASK_ALT | MASK_SHIFT, m));
        assert!(!combo_active(MASK_CMD, m));
        assert!(!combo_active(MASK_CMD, 0));
    }
}
```

- [ ] **Step 2: Run, verify failure**

Run: `cd src-tauri && cargo test grab`
Expected: compile error.

- [ ] **Step 3: Implement**

```rust
use serde_json::json;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

// CGEventFlags modifier masks (CoreGraphics/CGEventTypes.h)
pub const MASK_SHIFT: u64 = 1 << 17;
pub const MASK_CTRL: u64 = 1 << 18;
pub const MASK_ALT: u64 = 1 << 19;
pub const MASK_CMD: u64 = 1 << 20;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventSourceFlagsState(state_id: u32) -> u64;
}
const COMBINED_SESSION_STATE: u32 = 0;

pub fn combo_mask(names: &[String]) -> u64 {
    names.iter().fold(0, |m, n| {
        m | match n.as_str() {
            "cmd" => MASK_CMD,
            "alt" => MASK_ALT,
            "ctrl" => MASK_CTRL,
            "shift" => MASK_SHIFT,
            _ => 0,
        }
    })
}

pub fn combo_active(flags: u64, mask: u64) -> bool {
    mask != 0 && flags & mask == mask
}

#[cfg(target_os = "macos")]
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut grabbed = false;
        loop {
            let cfg = crate::config::load(&crate::oryx::config_path(&app));
            let mask = combo_mask(&cfg.grab_combo);
            let flags = unsafe { CGEventSourceFlagsState(COMBINED_SESSION_STATE) };
            let active = combo_active(flags, mask);
            if active != grabbed {
                grabbed = active;
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.set_ignore_cursor_events(!grabbed);
                }
                let _ = app.emit("grab-mode", json!({ "on": grabbed }));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
}
```

In `main.rs` setup: `grab::spawn(app.handle().clone());`. Reload of config each tick is fine at 10 Hz and picks up settings changes with no plumbing.

- [ ] **Step 4: Run tests, verify pass**

Run: `cd src-tauri && cargo test grab`
Expected: 2 passed.

- [ ] **Step 5: Manual check**

Run: `cd src-tauri && cargo tauri dev`
First add drag support to `hud.js` `main()` (dragging must be started programmatically since the window is undecorated):

```js
document.getElementById('board').addEventListener('mousedown', (e) => {
  if (document.body.classList.contains('grab')) {
    window.__TAURI__.window.getCurrentWindow().startDragging();
    e.preventDefault();
  }
});
```

Run: `cd src-tauri && cargo tauri dev`
Expected: normally clicks pass through the overlay; while holding ⌘⌥ the yellow grab outline appears and dragging the board moves the window; releasing ⌘⌥ returns to click-through.

- [ ] **Step 6: Update the spec's grab-manager line** (as listed under Files) and commit

```bash
git add -A && git commit -m "feat: modifier-combo grab mode toggling click-through"
```

---

### Task 8: Tray menu, settings window, config commands

**Files:**
- Create: `src-tauri/src/tray.rs`, `ui/settings.html`, `ui/settings.js`
- Modify: `src-tauri/src/main.rs` (add `mod tray;`, call `tray::build` in setup, register `set_config`), `src-tauri/src/oryx.rs` (add `set_config` command; emit `layout-refreshed` at the end of a successful `refresh_layout`), `src-tauri/tauri.conf.json` (no new window entry — settings window is created on demand)
- Test: `cargo test` still green; behavior manual

**Interfaces:**
- Consumes: `get_config`/`refresh_layout` (Task 3), `config-changed`/`layout-refreshed` listeners (Task 5).
- Produces: command `set_config(app, config: Config) -> Result<(), String>` (saves + emits `config-changed` with the full new `Config`); tray with items `Refresh layout`, `Pin overlay` (checkable — when checked, forces click-through off), `Settings…`, `Quit`.

- [ ] **Step 1: Add `set_config` to `oryx.rs`**

```rust
#[tauri::command]
pub fn set_config(app: AppHandle, config: crate::config::Config) -> Result<(), String> {
    crate::config::save(&config_path(&app), &config).map_err(|e| e.to_string())?;
    use tauri::Emitter;
    app.emit("config-changed", &config).map_err(|e| e.to_string())
}
```

At the end of `refresh_layout`'s `Ok` arm, before returning: `let _ = { use tauri::Emitter; app.emit("layout-refreshed", serde_json::json!({})) };`
Register `oryx::set_config` in the `generate_handler!` list.

- [ ] **Step 2: Implement `src-tauri/src/tray.rs`**

```rust
use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

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
                    let hash = crate::config::load(&crate::oryx::config_path(&app)).oryx_url;
                    let _ = crate::oryx::refresh_layout(app.clone(), hash).await;
                });
            }
            "pin" => {
                let pinned = pin_handle.is_checked().unwrap_or(false);
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.set_ignore_cursor_events(!pinned);
                }
                let _ = app.emit("grab-mode", serde_json::json!({ "on": pinned }));
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
```

In `main.rs` setup: `tray::build(&app.handle().clone())?;`

- [ ] **Step 3: Implement settings UI**

`ui/settings.html`:
```html
<!doctype html>
<html>
<head><meta charset="utf-8"><title>Settings</title>
<style>
  body { font: 13px -apple-system, sans-serif; padding: 16px; }
  label { display: block; margin: 12px 0 4px; font-weight: 600; }
  input[type=text] { width: 100%; }
  .error { color: #c00; font-size: 12px; }
  .ok { color: #080; font-size: 12px; }
</style></head>
<body>
  <label>Oryx layout URL or hash</label>
  <input type="text" id="oryx" />
  <button id="apply-url">Fetch &amp; apply</button>
  <span id="url-status"></span>

  <label>Opacity <span id="opacity-val"></span></label>
  <input type="range" id="opacity" min="0.2" max="1" step="0.05" />

  <label><input type="checkbox" id="colors" /> Use Oryx layer colors</label>

  <label>Grab hotkey (hold to move/resize)</label>
  <select id="combo">
    <option value="cmd,alt">⌘⌥</option>
    <option value="cmd,ctrl">⌘⌃</option>
    <option value="alt,ctrl">⌥⌃</option>
    <option value="cmd">⌘</option>
    <option value="alt">⌥</option>
    <option value="ctrl">⌃</option>
  </select>
  <script type="module" src="settings.js"></script>
</body>
</html>
```

`ui/settings.js`:
```js
const { invoke } = window.__TAURI__.core;

let cfg = await invoke('get_config');
const $ = (id) => document.getElementById(id);

$('oryx').value = cfg.oryx_url;
$('opacity').value = cfg.opacity;
$('opacity-val').textContent = cfg.opacity;
$('colors').checked = cfg.use_oryx_colors;
$('combo').value = cfg.grab_combo.join(',');

async function push() {
  await invoke('set_config', { config: cfg });
}

$('opacity').addEventListener('input', async (e) => {
  cfg.opacity = Number(e.target.value);
  $('opacity-val').textContent = cfg.opacity;
  await push();
});
$('colors').addEventListener('change', async (e) => {
  cfg.use_oryx_colors = e.target.checked;
  await push();
});
$('combo').addEventListener('change', async (e) => {
  cfg.grab_combo = e.target.value.split(',');
  await push();
});
$('apply-url').addEventListener('click', async () => {
  $('url-status').textContent = '…';
  try {
    const res = await invoke('refresh_layout', { url: $('oryx').value });
    cfg = await invoke('get_config');
    $('url-status').textContent = res.stale ? 'offline — using cache' : 'applied';
    $('url-status').className = res.stale ? 'error' : 'ok';
  } catch (err) {
    $('url-status').textContent = String(err);
    $('url-status').className = 'error';
  }
});
```

- [ ] **Step 4: Verify**

Run: `cd src-tauri && cargo test && cargo tauri dev`
Expected: tray icon appears; Settings opens; opacity slider changes the overlay live; colors toggle re-renders; combo change takes effect within a tick; pasting the layout URL and Fetch re-renders; bogus URL shows inline error; Pin makes the overlay clickable until unchecked; Quit exits.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: tray menu and settings window with live config"
```

---

### Task 9: Window persistence, README, release build

**Files:**
- Modify: `src-tauri/src/main.rs` (window event handler)
- Create: `README.md`
- Test: `cargo test`, `node --test ui/test/`, manual

**Interfaces:**
- Consumes: `config::{load, save, WindowRect}`, `oryx::config_path`.

- [ ] **Step 1: Persist and restore overlay position/size**

In `main.rs` setup, after getting `overlay` — restore:

```rust
            let cfg = config::load(&oryx::config_path(&app.handle()));
            if let Some(r) = &cfg.window {
                use tauri::{LogicalPosition, LogicalSize};
                let _ = overlay.set_position(LogicalPosition::new(r.x, r.y));
                let _ = overlay.set_size(LogicalSize::new(r.w, r.h));
            }
```

And register (on the builder, before `.run`):

```rust
        .on_window_event(|window, event| {
            if window.label() != "overlay" {
                return;
            }
            if matches!(event, tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)) {
                let app = window.app_handle();
                let mut cfg = config::load(&oryx::config_path(app));
                let scale = window.scale_factor().unwrap_or(1.0);
                if let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) {
                    let pos = pos.to_logical::<f64>(scale);
                    let size = size.to_logical::<f64>(scale);
                    cfg.window = Some(config::WindowRect { x: pos.x, y: pos.y, w: size.width, h: size.height });
                    let _ = config::save(&oryx::config_path(app), &cfg);
                }
            }
        })
```

- [ ] **Step 2: Verify persistence**

Run `cargo tauri dev`, ⌘⌥-drag the overlay somewhere, quit, relaunch.
Expected: overlay reopens where you left it.

- [ ] **Step 3: Write `README.md`**

Content: what it is (one paragraph), prerequisites (Keymapp ≥1.3.2 with API enabled, protoc, Rust, tauri-cli), build (`cd src-tauri && cargo tauri build`, app lands in `src-tauri/target/release/bundle/macos/`), usage (tray menu, ⌘⌥ to move, settings), troubleshooting (offline badge → check Keymapp API setting; only one API client at a time; layer colors come from Oryx).

- [ ] **Step 4: Full test + release build**

```bash
node --test ui/test/ && (cd src-tauri && cargo test && cargo tauri build)
```
Expected: all tests pass; `.app` bundle produced; launching it shows the HUD with no Dock icon.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: window persistence, readme and release build"
```

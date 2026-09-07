# v0.0.3 source audit and cleanup

Date: 2026-09-06. Reviewed baseline: `6fd6a3f` on `release/v0.0.3`, including the two local cleanup commits following `origin/release/v0.0.3`. The fixes described below are additional working-tree changes. This audit does not merge, push, tag, or publish a release.

## Scope

Reviewed all application Rust modules; the frontend entry points, renderer, translation/geometry helpers, CSS, HTML and SVG references; settings persistence and import/export; HID decoding and device discovery; layout retrieval/cache; heatmap and shortcut handling; manifests/lockfile, Tauri capabilities/configuration, CI/release workflows, tests, and README. Historical planning documents were retained as history and identified as such in the README.

This is a source review with automated checks, not a penetration test, an exhaustive proof of correctness, or a completed hardware/visual acceptance test.

## Verified findings addressed

Priority describes the original impact: P1 = core functionality can be wrong or unavailable; P2 = user-visible correctness/persistence issue; P3 = maintainability or test coverage.

| Priority | Finding and correction | Main source |
| --- | --- | --- |
| P1 | HID layout identity existed only in a one-shot event, so startup could miss it. The backend now retains identity and active layer and owns layout retrieval; the UI can query connection/layer status after registering listeners. | [hid.rs](../src-tauri/src/hid.rs), [state.rs](../src-tauri/src/state.rs), [hud.js](../ui/hud.js) |
| P1 | The flashed revision was ignored and every fetch requested `latest`. Requests now use the firmware revision. Cache metadata binds the content to its layout/revision, rejects mismatches, and validates Voyager layer structure. Refreshes are serialized, requests time out, and cache writes use atomic rename. | [layout.rs](../src-tauri/src/layout.rs) |
| P2 | Discovery could select another Raw HID keyboard and stopped after the first open failure. It now filters the Voyager product/interface and tries the next matching device if opening fails. The connected device's `Voyager` product name was verified using the local USB registry; no layout ID or serial is hardcoded. | [hid.rs](../src-tauri/src/hid.rs) |
| P2 | A disconnect left keys highlighted; matrix mapping was duplicated in JavaScript. The backend now validates/maps packets once, and disconnect clears frontend presses/partial shortcuts. Invalid packets are ignored and duplicate press notifications do not add extra counts. | [hid.rs](../src-tauri/src/hid.rs), [hud.js](../ui/hud.js) |
| P2 | Debounced heatmap saving could be postponed indefinitely during continuous typing. Saving is now throttled, reset cancels pending writes, page exit flushes history, and corrupt/unavailable storage does not stop key tracking. The existing storage key is retained. | [heatmap.mjs](../ui/heatmap.mjs) |
| P2 | Heatmap color/peak updates read stale preferences, and heatmap/Oryx fills overrode live pressed-key fills. Theme updates now use current config and CSS has explicit fill precedence. Count mode replaces labels instead of overlapping them with tiny counts. | [hud.js](../ui/hud.js), [style.css](../ui/style.css) |
| P2 | Several font controls produced CSS variables that were never consumed, and family selectors contained only “System default.” Font variables now reach the labels; family inputs accept installed font names. Unused per-font ligature flags were removed while preserving the global setting and compatibility with old JSON. | [settings.html](../ui/settings.html), [style.css](../ui/style.css), [config.rs](../src-tauri/src/config.rs) |
| P2 | Whole-config Settings writes could overwrite newer tray/HID/window-owned state. Preference updates preserve those fields; queued saves capture snapshots and report failures. Import/reset synchronize the pin flag and tray checkmark. Duplicate reset dispatch was removed. | [config.rs](../src-tauri/src/config.rs), [oryx.rs](../src-tauri/src/oryx.rs), [settings.js](../ui/settings.js), [tray.rs](../src-tauri/src/tray.rs) |
| P2 | Export reported a guessed browser filename. It now allocates a real file in Downloads using `create_new`, returns the actual path, and never overwrites an earlier export. Export/import/reset wait for queued preference saves. | [config.rs](../src-tauri/src/config.rs), [oryx.rs](../src-tauri/src/oryx.rs), [settings.js](../ui/settings.js) |
| P2 | Hide/show slept synchronously and had no serialization; restore state could be consumed before restoration succeeded. Animation is asynchronous and serialized, restoration retains the saved rectangle until success, and positioning commands restore hidden state before changing geometry. Zero reveal also disables native interaction. Existing clipped-side border/handle CSS geometry was retained. | [oryx.rs](../src-tauri/src/oryx.rs), [grab.rs](../src-tauri/src/grab.rs), [hud.js](../ui/hud.js) |
| P2 | Recording a replacement toggle sequence could activate the existing shortcut. Matching is suspended during recording and reset on stop, Settings reload, and native window destruction. Sequence length and indices are bounded. | [toggle-sequence.mjs](../ui/toggle-sequence.mjs), [settings.js](../ui/settings.js), [main.rs](../src-tauri/src/main.rs) |
| P2 | Malformed colors and slot codes could break rendering, and a key retained only one of its layer targets. Config/slot validation is stronger, and trigger highlighting considers all configured targets, including tap actions. | [config.rs](../src-tauri/src/config.rs), [translator.mjs](../ui/translator.mjs), [hud.js](../ui/hud.js) |
| P3 | The previous HID test called the handler with invalid two-byte key packets and asserted nothing. Pure decoder tests now assert real presses/releases, padded reports, layers, malformed packets, and all 52 mappings. Test-only Tauri support and unused Tokio macro features were removed from production dependencies. | [hid.rs](../src-tauri/src/hid.rs), [Cargo.toml](../src-tauri/Cargo.toml) |
| P3 | Unused `last_refresh`, per-font flags, `BOARD_UNITS`, CSS variables and an unused exposed command were removed. Fixed-name temporary test directories were replaced with isolated temporary directories. Existing removal of the Keymapp watcher and `protoc` was verified. | [config.rs](../src-tauri/src/config.rs), [geometry.mjs](../ui/geometry.mjs), [main.rs](../src-tauri/src/main.rs) |
| P3 | CI did not parse the actual UI entry points or check bridge registrations, control bindings, numeric limits, formatting, or lint warnings. These checks were added, along with Cargo/Tauri version parity. Release builds also run the Rust tests. README behavior/release claims and stale footprint figures were corrected. | [ui/test](../ui/test), [ci.yml](../.github/workflows/ci.yml), [release.yml](../.github/workflows/release.yml), [README](../README.md) |

## Dependency and security review

All direct production/build dependencies have current source usage. There are no npm dependencies. `tempfile` was already in the lockfile transitively and is now an explicit development dependency for isolated tests. No broad dependency upgrade was performed.

`cargo-audit 0.22.2` scanned 479 locked dependencies against 1,239 RustSec advisories, database revision `5a0ebedfe8bdd2e295b171f4162f8c977bcad9a5`:

- No entries in the vulnerability category and no yanked-package warning were reported.
- 16 unmaintained warnings and one unsoundness warning remain; exit status 0 does **not** mean a warning-free scan.
- The GTK3/ATK/GDK family, `proc-macro-error`, and the [GLib unsoundness advisory](https://rustsec.org/advisories/RUSTSEC-2024-0429) belong to Tauri's Linux dependency tree and are absent from this machine's macOS build graph.
- Five `unic-*` warnings remain in the macOS dependency graph through `urlpattern → tauri-utils`, including [unic-ucd-ident](https://rustsec.org/advisories/RUSTSEC-2025-0100). These require upstream dependency work; they were neither silently ignored nor removed from the lockfile by hand.

The scanner was installed in a temporary directory, not added as an app dependency. To repeat with an installed scanner, run `cargo audit --file src-tauri/Cargo.lock` with network access.

Oryx labels are rendered as text, not HTML; the only network endpoint is a fixed HTTPS GraphQL URL. Color strings are validated before use. Exports use exclusive file creation, and settings/cache writes are serialized and atomically replaced. No key text/event history is transmitted to the layout service.

## Remaining review findings and limitations

These were not hidden by the cleanup:

- **P2 — Backup/reset scope:** Start at login lives in the OS autostart plugin, outside `config.json`, so JSON import/export and Reset do not include it. Heatmap history is deliberately separate. Making backup/reset cover OS login state needs explicit integration and rollback/error handling. The README now states the boundary.
- **P2 — Monitor identity collisions:** `monitor_key` uses a display name or resolution. Identically named displays can share a saved-position slot. A native stable display identifier remains a follow-up in [oryx.rs](../src-tauri/src/oryx.rs).
- **P2 — Interactive clipping:** CSS clipping does not shrink the native window's hit-test rectangle. Zero reveal is now click-through, but a partially hidden, pinned/grabbed overlay can still intercept input in its transparent area. Native hit-region behavior needs focused testing/design; the accepted visual clipping was not replaced with another speculative implementation.
- **P2 — Product compatibility:** Only a Voyager interface is supported. Multiple Voyagers have no picker, and custom firmware that renames the USB product is not detected. Physical presses are recorded, not QMK's resolved tap/hold actions; the toggle recorder is an ordered sequence, not an unordered chord recognizer. Character translation remains Canadian-CSA-specific in several mappings.
- **P3 — Defense in depth:** The Tauri configuration still has no explicit Content Security Policy. Current external labels are text-only and there is no remote webview navigation, but a restrictive CSP should be added and validated against native IPC and the inline/dynamic styling before extending the UI.
- **P3 — Distribution:** The workflow uses version-tagged actions, not immutable commit pins. It attests the DMG but does not configure Apple signing/notarization. Remote branch-protection settings and a new GitHub Actions run were not validated by this local audit.

## Automated validation

The following checks passed locally on macOS:

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --offline --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --offline --locked --manifest-path src-tauri/Cargo.toml
node --test ui/test/*.test.mjs
cargo build --offline --locked --release --manifest-path src-tauri/Cargo.toml
git diff --check
```

Results: **24 Rust tests and 33 JavaScript tests passed**, with a successful optimized release binary build. The JavaScript suite includes source/bridge contract checks, not browser rendering tests. The dependency scan completed with the warnings listed above. DMG packaging and publication were not performed.

## Required pre-release smoke test

The development process was stopped during the audit and was not relaunched. The following live checks remain before merge/tagging:

1. Launch with the Voyager connected before startup and after startup; disconnect while holding a key, then reconnect. Check every key, thumb positions, active layers and removal of stuck highlights.
2. Verify the flashed revision against Oryx, then test a matching cache with the network unavailable. Flash a different revision and confirm an old cache is not silently substituted.
3. Exercise heatmap color/peak/count toggles with Oryx colors on, restart after sustained typing, and reset history. Verify pressed feedback takes precedence.
4. Exercise all three font groups, icon visibility/size controls, and imported invalid colors. Export twice and check the reported real files. Change settings after toggling Pin, then test import/reset while hidden.
5. Test ordered/triple-tap toggles, rapid repeated toggles, recording over an existing shortcut, and closing Settings mid-recording.
6. On mixed-DPI/multi-monitor arrangements, test all four hidden sides, zero/nonzero reveal, handles/borders in pinned and unpinned modes, position restoration, and dragging/resizing after changing the halves distance. Include transparent-area mouse interaction.
7. Build/test the DMG and confirm GitHub CI, draft release notes, tag/version checks, and attestation on the actual release workflow.

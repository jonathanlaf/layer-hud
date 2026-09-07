use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub oryx_revision: String,
    pub opacity: f64,
    pub char_opacity: f64,
    pub border_opacity: f64,
    pub border_width: f64,
    pub grab_combo: Vec<String>,
    pub overlay_pinned: bool,
    pub toggle_macro: Vec<u8>,
    pub hide_side: String,
    pub hide_reveal: f64,
    pub hide_animation_ms: f64,
    pub use_oryx_colors: bool,
    pub show_layer_action_icons: bool,
    pub layer_indicator: String,
    pub show_shift_icons: bool,
    pub shift_icon_scale: f64,
    pub show_alternate_action_icons: bool,
    pub alternate_action_icon_scale: f64,
    pub show_heatmap: bool,
    pub show_heatmap_counts: bool,
    pub heatmap_color: String,
    pub heatmap_peak: f64,
    pub key_fill_color: String,
    pub key_fill_opacity: f64,
    pub padding: f64,
    pub bg_color: String,
    pub text_color: String,
    pub legend_color: String,
    pub shift_color: String,
    pub alternate_color: String,
    pub border_color: String,
    pub pressed_key_color: String,
    pub pressed_key_fill_opacity: f64,
    pub pressed_key_border_color: String,
    pub pressed_key_border_opacity: f64,
    pub pressed_key_border_width: f64,
    pub key_border_radius: f64,
    pub pill_border_radius: f64,
    pub show_key_shadows: bool,
    pub show_pressed_key_shadow: bool,
    pub key_shadow_color: String,
    pub pressed_key_shadow_color: String,
    pub key_shadow_opacity: f64,
    pub pressed_key_shadow_opacity: f64,
    pub key_spacing: f64,
    pub keyboard_halves_distance: f64,
    pub keyboard_halves_rotation: f64,
    pub layer_pill_horizontal: f64,
    pub layer_pill_vertical: f64,
    pub offline_pill_horizontal: f64,
    pub offline_pill_vertical: f64,
    pub base_outline_enabled: bool,
    pub base_outline_color: String,
    pub base_outline_opacity: f64,
    pub base_outline_width: f64,
    pub grab_outline_enabled: bool,
    pub grab_outline_color: String,
    pub grab_outline_opacity: f64,
    pub grab_outline_width: f64,
    pub key_font_family: String,
    pub key_font_size: f64,
    pub key_font_bold: bool,
    pub key_font_italic: bool,
    pub legend_font_family: String,
    pub legend_font_size: f64,
    pub legend_font_bold: bool,
    pub legend_font_italic: bool,
    pub layer_name_font_family: String,
    pub layer_name_font_size: f64,
    pub layer_name_font_bold: bool,
    pub layer_name_font_italic: bool,
    pub font_ligatures: bool,
    pub window_by_monitor: HashMap<String, WindowRect>,
    pub last_monitor: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            // Populated from the keyboard's Oryx HID identity; there is no
            // user-entered layout URL anymore.
            oryx_url: String::new(),
            oryx_revision: "latest".into(),
            opacity: 0.85,
            char_opacity: 1.0,
            border_opacity: 0.35,
            border_width: 1.0,
            grab_combo: vec!["cmd".into(), "alt".into()],
            overlay_pinned: false,
            toggle_macro: Vec::new(),
            hide_side: "right".into(),
            hide_reveal: 0.08,
            hide_animation_ms: 220.0,
            use_oryx_colors: true,
            show_layer_action_icons: true,
            layer_indicator: "icon".into(),
            show_shift_icons: true,
            shift_icon_scale: 1.0,
            show_alternate_action_icons: true,
            alternate_action_icon_scale: 1.0,
            show_heatmap: false,
            show_heatmap_counts: false,
            heatmap_color: "#ff5c5c".into(),
            heatmap_peak: 20.0,
            key_fill_color: "#ffffff".into(),
            key_fill_opacity: 0.0,
            padding: 10.0,
            bg_color: "#141418".into(),
            text_color: "#ffffff".into(),
            legend_color: "#ffffff".into(),
            shift_color: "#ffffff".into(),
            alternate_color: "#ffffff".into(),
            border_color: "#ffffff".into(),
            pressed_key_color: "#7ad7ff".into(),
            pressed_key_fill_opacity: 0.45,
            pressed_key_border_color: "#7ad7ff".into(),
            pressed_key_border_opacity: 0.85,
            pressed_key_border_width: 1.0,
            key_border_radius: 7.0,
            pill_border_radius: 999.0,
            show_key_shadows: false,
            show_pressed_key_shadow: true,
            key_shadow_color: "#ffffff".into(),
            pressed_key_shadow_color: "#7ad7ff".into(),
            key_shadow_opacity: 0.25,
            pressed_key_shadow_opacity: 0.85,
            key_spacing: 0.06,
            keyboard_halves_distance: 1.6,
            keyboard_halves_rotation: 0.0,
            layer_pill_horizontal: 50.0,
            layer_pill_vertical: 8.0,
            offline_pill_horizontal: 50.0,
            offline_pill_vertical: 50.0,
            base_outline_enabled: true,
            base_outline_color: "#78b4ff".into(),
            base_outline_opacity: 0.6,
            base_outline_width: 2.0,
            grab_outline_enabled: true,
            grab_outline_color: "#ffdc78".into(),
            grab_outline_opacity: 0.9,
            grab_outline_width: 2.0,
            key_font_family: "".into(),
            key_font_size: 1.0,
            key_font_bold: false,
            key_font_italic: false,
            legend_font_family: "".into(),
            legend_font_size: 1.0,
            legend_font_bold: false,
            legend_font_italic: false,
            layer_name_font_family: "".into(),
            layer_name_font_size: 11.0,
            layer_name_font_bold: false,
            layer_name_font_italic: false,
            font_ligatures: true,
            window_by_monitor: HashMap::new(),
            last_monitor: None,
        }
    }
}

impl Config {
    /// Preferences sent by Settings must not overwrite fields owned by HID,
    /// the tray, or native window movement while Settings was open.
    pub fn apply_preferences(&mut self, mut incoming: Self) {
        incoming.window_by_monitor = std::mem::take(&mut self.window_by_monitor);
        incoming.last_monitor = self.last_monitor.take();
        incoming.oryx_url = std::mem::take(&mut self.oryx_url);
        incoming.oryx_revision = std::mem::take(&mut self.oryx_revision);
        incoming.overlay_pinned = self.overlay_pinned;
        incoming.clamp();
        *self = incoming;
    }

    /// Clamp every user-editable numeric field to the range the Settings UI's
    /// sliders allow, so a value written by something other than the slider
    /// (a hand-edited config file, a future API) can't push the renderer an
    /// out-of-range opacity/width/padding.
    pub fn clamp(&mut self) {
        if !matches!(self.hide_side.as_str(), "left" | "right" | "top" | "bottom") {
            self.hide_side = "right".into();
        }
        if !matches!(self.layer_indicator.as_str(), "none" | "textual" | "icon") {
            self.layer_indicator = "icon".into();
        }
        self.toggle_macro.retain(|key| *key < 52);
        self.toggle_macro.truncate(64);
        self.grab_combo
            .retain(|key| matches!(key.as_str(), "cmd" | "alt" | "ctrl" | "shift"));
        self.window_by_monitor.retain(|_, r| {
            r.x.is_finite()
                && r.y.is_finite()
                && r.w.is_finite()
                && r.h.is_finite()
                && r.w > 0.0
                && r.h > 0.0
        });
        // Imported colors must be usable both by CSS and the color inputs.
        let defaults = Config::default();
        macro_rules! color {
            ($($field:ident),+ $(,)?) => { $(
                if self.$field.len() != 7 || !self.$field.starts_with('#') || !self.$field[1..].bytes().all(|b| b.is_ascii_hexdigit()) {
                    self.$field = defaults.$field;
                }
            )+ };
        }
        color!(
            bg_color,
            text_color,
            legend_color,
            shift_color,
            alternate_color,
            border_color,
            key_fill_color,
            pressed_key_color,
            pressed_key_border_color,
            key_shadow_color,
            pressed_key_shadow_color,
            base_outline_color,
            grab_outline_color,
            heatmap_color
        );
        self.opacity = self.opacity.clamp(0.0, 1.0);
        self.char_opacity = self.char_opacity.clamp(0.2, 1.0);
        self.border_opacity = self.border_opacity.clamp(0.0, 1.0);
        self.border_width = self.border_width.clamp(0.0, 5.0);
        self.key_fill_opacity = self.key_fill_opacity.clamp(0.0, 1.0);
        self.padding = self.padding.clamp(0.0, 60.0);
        self.shift_icon_scale = self.shift_icon_scale.clamp(0.5, 2.5);
        self.alternate_action_icon_scale = self.alternate_action_icon_scale.clamp(0.5, 2.5);
        self.heatmap_peak = self.heatmap_peak.clamp(1.0, 1000.0);
        self.base_outline_opacity = self.base_outline_opacity.clamp(0.0, 1.0);
        self.base_outline_width = self.base_outline_width.clamp(0.0, 5.0);
        self.grab_outline_opacity = self.grab_outline_opacity.clamp(0.0, 1.0);
        self.grab_outline_width = self.grab_outline_width.clamp(0.0, 5.0);
        self.pressed_key_fill_opacity = self.pressed_key_fill_opacity.clamp(0.0, 1.0);
        self.pressed_key_border_opacity = self.pressed_key_border_opacity.clamp(0.0, 1.0);
        self.pressed_key_border_width = self.pressed_key_border_width.clamp(0.0, 5.0);
        self.key_border_radius = self.key_border_radius.clamp(0.0, 30.0);
        self.pill_border_radius = self.pill_border_radius.clamp(0.0, 999.0);
        self.key_shadow_opacity = self.key_shadow_opacity.clamp(0.0, 1.0);
        self.pressed_key_shadow_opacity = self.pressed_key_shadow_opacity.clamp(0.0, 1.0);
        self.key_spacing = self.key_spacing.clamp(0.0, 0.25);
        self.keyboard_halves_distance = self.keyboard_halves_distance.clamp(0.25, 20.0);
        self.keyboard_halves_rotation = self.keyboard_halves_rotation.clamp(-15.0, 15.0);
        self.layer_pill_horizontal = self.layer_pill_horizontal.clamp(0.0, 100.0);
        self.layer_pill_vertical = self.layer_pill_vertical.clamp(0.0, 100.0);
        self.offline_pill_horizontal = self.offline_pill_horizontal.clamp(0.0, 100.0);
        self.offline_pill_vertical = self.offline_pill_vertical.clamp(0.0, 100.0);
        self.hide_reveal = self.hide_reveal.clamp(0.0, 1.0);
        self.hide_animation_ms = self.hide_animation_ms.clamp(0.0, 1000.0);
        self.key_font_size = self.key_font_size.clamp(0.5, 2.0);
        self.legend_font_size = self.legend_font_size.clamp(0.5, 2.0);
        self.layer_name_font_size = self.layer_name_font_size.clamp(8.0, 24.0);
    }
}

pub fn load(path: &Path) -> Config {
    let mut cfg: Config = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // Clamp on every read, not just set_config's write path, so a hand-edited
    // or otherwise out-of-range value on disk self-heals for every caller
    // (get_config, the window-restore read in main.rs, grab.rs's poll, etc.)
    // instead of only after the user next changes a setting via the UI.
    cfg.clamp();
    cfg
}

pub fn save(path: &Path, cfg: &Config) -> std::io::Result<()> {
    save_json(path, cfg)
}

/// Allocate the filename before reporting it: never overwrite an earlier export.
pub fn export_to_dir(dir: &Path, cfg: &Config) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    let contents = serde_json::to_vec_pretty(cfg)?;
    for suffix in 0.. {
        let name = if suffix == 0 {
            "layer-hud-settings.json".into()
        } else {
            format!("layer-hud-settings ({suffix}).json")
        };
        let path = dir.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(&contents)?;
                file.sync_all()?;
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    unreachable!()
}

/// Callers serialize writes to each path; rename keeps concurrent readers safe.
pub fn save_json(path: &Path, value: &impl Serialize) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Write to a sibling temp file and rename it into place, rather than
    // truncating and rewriting `path` directly. config_lock only serializes
    // this crate's own writers; several call sites still read config.json
    // unlocked (grab.rs's poll loop, tray.rs's refresh handler), and a
    // truncate-then-write leaves a window where such a reader can observe an
    // empty file. A rename is atomic on the filesystems this app targets, so
    // any concurrent reader always sees either the fully-old or fully-new
    // content, never a partial one — no reader-side locking required.
    let mut tmp_path = path.as_os_str().to_owned();
    tmp_path.push(".tmp");
    let tmp_path = std::path::PathBuf::from(tmp_path);
    std::fs::write(&tmp_path, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(&tmp_path, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let c = Config::default();
        assert!(c.oryx_url.is_empty());
        assert_eq!(c.opacity, 0.85);
        assert_eq!(c.grab_combo, vec!["cmd".to_string(), "alt".to_string()]);
        assert!(c.use_oryx_colors);
        assert!(c.window_by_monitor.is_empty());
        assert!(c.last_monitor.is_none());
        assert_eq!(c.char_opacity, 1.0);
        assert_eq!(c.border_opacity, 0.35);
        assert_eq!(c.border_width, 1.0);
        assert_eq!(c.bg_color, "#141418");
        assert_eq!(c.key_fill_opacity, 0.0);
        assert_eq!(c.padding, 10.0);
        assert_eq!(c.text_color, "#ffffff");
        assert_eq!(c.legend_color, "#ffffff");
        assert_eq!(c.border_color, "#ffffff");
        assert_eq!(c.shift_icon_scale, 1.0);
        assert_eq!(c.alternate_action_icon_scale, 1.0);
    }

    #[test]
    fn old_config_without_new_fields_gets_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"oryx_url":"abc","opacity":0.5}"#).unwrap();
        let c = load(&path);
        assert_eq!(c.oryx_url, "abc");
        assert_eq!(c.opacity, 0.5);
        assert_eq!(c.char_opacity, 1.0);
        assert_eq!(c.border_opacity, 0.35);
        assert_eq!(c.border_width, 1.0);
        assert_eq!(c.bg_color, "#141418");
    }

    #[test]
    fn clamp_bounds_every_numeric_field() {
        let mut c = Config {
            opacity: 5.0,
            char_opacity: -1.0,
            border_opacity: -1.0,
            border_width: 999.0,
            key_fill_opacity: 2.0,
            padding: -10.0,
            shift_icon_scale: 3.0,
            alternate_action_icon_scale: 0.1,
            ..Config::default()
        };
        c.clamp();
        assert_eq!(c.opacity, 1.0);
        assert_eq!(c.char_opacity, 0.2);
        assert_eq!(c.border_opacity, 0.0);
        assert_eq!(c.border_width, 5.0);
        assert_eq!(c.key_fill_opacity, 1.0);
        assert_eq!(c.padding, 0.0);
        assert_eq!(c.shift_icon_scale, 2.5);
        assert_eq!(c.alternate_action_icon_scale, 0.5);
    }

    #[test]
    fn load_clamps_out_of_range_values_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"opacity":5.0,"padding":-10.0}"#).unwrap();
        let c = load(&path);
        assert_eq!(c.opacity, 1.0);
        assert_eq!(c.padding, 0.0);
    }

    #[test]
    fn save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut c = Config {
            opacity: 0.5,
            legend_color: "#ffcc00".into(),
            shift_color: "#00ccff".into(),
            alternate_color: "#ff88cc".into(),
            show_layer_action_icons: true,
            show_shift_icons: false,
            show_alternate_action_icons: false,
            ..Config::default()
        };
        c.window_by_monitor.insert(
            "test".into(),
            WindowRect {
                x: 10.0,
                y: 20.0,
                w: 800.0,
                h: 300.0,
            },
        );
        c.last_monitor = Some("test".into());
        save(&path, &c).unwrap();
        assert_eq!(load(&path), c);
    }

    #[test]
    fn load_missing_or_corrupt_returns_default() {
        assert_eq!(
            load(std::path::Path::new("/nonexistent/vhud.json")),
            Config::default()
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(load(&path), Config::default());
    }

    #[test]
    fn validates_imported_colors_shortcuts_and_rectangles() {
        let mut cfg = Config {
            text_color: "#gggggg".into(),
            bg_color: "x".into(),
            heatmap_color: "#12Ab9F".into(),
            hide_side: "elsewhere".into(),
            grab_combo: vec!["cmd".into(), "unknown".into()],
            toggle_macro: vec![51; 80],
            ..Config::default()
        };
        cfg.toggle_macro[0] = 255;
        cfg.window_by_monitor.insert(
            "bad".into(),
            WindowRect {
                x: 0.0,
                y: 0.0,
                w: -1.0,
                h: 20.0,
            },
        );
        cfg.clamp();
        assert_eq!(cfg.text_color, Config::default().text_color);
        assert_eq!(cfg.bg_color, Config::default().bg_color);
        assert_eq!(cfg.heatmap_color, "#12Ab9F");
        assert_eq!(cfg.hide_side, "right");
        assert_eq!(cfg.grab_combo, ["cmd"]);
        assert_eq!(cfg.toggle_macro, vec![51; 64]);
        assert!(cfg.window_by_monitor.is_empty());
    }

    #[test]
    fn stale_settings_do_not_overwrite_runtime_owned_fields() {
        let mut live = Config {
            overlay_pinned: true,
            oryx_url: "abc".into(),
            oryx_revision: "rev2".into(),
            last_monitor: Some("display".into()),
            ..Config::default()
        };
        live.window_by_monitor.insert(
            "display".into(),
            WindowRect {
                x: 1.0,
                y: 2.0,
                w: 300.0,
                h: 200.0,
            },
        );
        let previous = live.clone();
        live.apply_preferences(Config {
            opacity: 0.2,
            ..Config::default()
        });
        assert_eq!(live.opacity, 0.2);
        assert_eq!(live.oryx_url, previous.oryx_url);
        assert_eq!(live.oryx_revision, previous.oryx_revision);
        assert_eq!(live.last_monitor, previous.last_monitor);
        assert_eq!(live.window_by_monitor, previous.window_by_monitor);
        assert!(live.overlay_pinned);
    }

    #[test]
    fn exports_allocate_real_unique_filenames_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let first = export_to_dir(dir.path(), &Config::default()).unwrap();
        let changed = Config {
            opacity: 0.2,
            ..Config::default()
        };
        let second = export_to_dir(dir.path(), &changed).unwrap();
        assert_eq!(first.file_name().unwrap(), "layer-hud-settings.json");
        assert_eq!(second.file_name().unwrap(), "layer-hud-settings (1).json");
        assert_eq!(load(&first), Config::default());
        assert_eq!(load(&second), changed);
        assert!(export_to_dir(&dir.path().join("missing"), &changed).is_err());
    }

    #[test]
    fn legacy_ligatures_fields_are_ignored_without_losing_preferences() {
        let cfg: Config = serde_json::from_str(
            r#"{"key_font_ligatures":false,"last_refresh":"old","key_font_italic":true}"#,
        )
        .unwrap();
        assert!(cfg.font_ligatures);
        assert!(cfg.key_font_italic);
        assert!(serde_json::to_value(cfg)
            .unwrap()
            .get("key_font_ligatures")
            .is_none());
    }

    #[test]
    fn cargo_and_tauri_versions_match() {
        let tauri: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(tauri["version"], env!("CARGO_PKG_VERSION"));
    }
}

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
    pub opacity: f64,
    pub char_opacity: f64,
    pub border_opacity: f64,
    pub border_width: f64,
    pub grab_combo: Vec<String>,
    pub use_oryx_colors: bool,
    pub show_layer_action_icons: bool,
    pub show_shift_icons: bool,
    pub shift_icon_scale: f64,
    pub show_alternate_action_icons: bool,
    pub alternate_action_icon_scale: f64,
    pub key_fill_color: String,
    pub key_fill_opacity: f64,
    pub padding: f64,
    pub bg_color: String,
    pub text_color: String,
    pub legend_color: String,
    pub shift_color: String,
    pub alternate_color: String,
    pub border_color: String,
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
    pub key_font_ligatures: bool,
    pub legend_font_family: String,
    pub legend_font_size: f64,
    pub legend_font_bold: bool,
    pub legend_font_italic: bool,
    pub legend_font_ligatures: bool,
    pub layer_name_font_family: String,
    pub layer_name_font_size: f64,
    pub layer_name_font_bold: bool,
    pub layer_name_font_italic: bool,
    pub layer_name_font_ligatures: bool,
    pub font_ligatures: bool,
    pub window_by_monitor: HashMap<String, WindowRect>,
    pub last_monitor: Option<String>,
    pub last_refresh: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            oryx_url: "Br3gO".into(),
            opacity: 0.85,
            char_opacity: 1.0,
            border_opacity: 0.35,
            border_width: 1.0,
            grab_combo: vec!["cmd".into(), "alt".into()],
            use_oryx_colors: true,
            show_layer_action_icons: true,
            show_shift_icons: true,
            shift_icon_scale: 1.0,
            show_alternate_action_icons: true,
            alternate_action_icon_scale: 1.0,
            key_fill_color: "#ffffff".into(),
            key_fill_opacity: 0.0,
            padding: 10.0,
            bg_color: "#141418".into(),
            text_color: "#ffffff".into(),
            legend_color: "#ffffff".into(),
            shift_color: "#ffffff".into(),
            alternate_color: "#ffffff".into(),
            border_color: "#ffffff".into(),
            base_outline_enabled: true, base_outline_color: "#78b4ff".into(), base_outline_opacity: 0.6, base_outline_width: 2.0,
            grab_outline_enabled: true, grab_outline_color: "#ffdc78".into(), grab_outline_opacity: 0.9, grab_outline_width: 2.0,
            key_font_family: "".into(), key_font_size: 1.0, key_font_bold: false, key_font_italic: false, key_font_ligatures: false,
            legend_font_family: "".into(), legend_font_size: 1.0, legend_font_bold: false, legend_font_italic: false, legend_font_ligatures: false,
            layer_name_font_family: "".into(), layer_name_font_size: 11.0, layer_name_font_bold: false, layer_name_font_italic: false, layer_name_font_ligatures: false,
            font_ligatures: true,
            window_by_monitor: HashMap::new(),
            last_monitor: None,
            last_refresh: None,
        }
    }
}

impl Config {
    /// Clamp every user-editable numeric field to the range the Settings UI's
    /// sliders allow, so a value written by something other than the slider
    /// (a hand-edited config file, a future API) can't push the renderer an
    /// out-of-range opacity/width/padding.
    pub fn clamp(&mut self) {
        self.opacity = self.opacity.clamp(0.0, 1.0);
        self.char_opacity = self.char_opacity.clamp(0.2, 1.0);
        self.border_opacity = self.border_opacity.clamp(0.0, 1.0);
        self.border_width = self.border_width.clamp(0.0, 5.0);
        self.key_fill_opacity = self.key_fill_opacity.clamp(0.0, 1.0);
        self.padding = self.padding.clamp(0.0, 60.0);
        self.shift_icon_scale = self.shift_icon_scale.clamp(0.5, 2.5);
        self.alternate_action_icon_scale = self.alternate_action_icon_scale.clamp(0.5, 2.5);
        self.base_outline_opacity = self.base_outline_opacity.clamp(0.0, 1.0);
        self.base_outline_width = self.base_outline_width.clamp(0.0, 5.0);
        self.grab_outline_opacity = self.grab_outline_opacity.clamp(0.0, 1.0);
        self.grab_outline_width = self.grab_outline_width.clamp(0.0, 5.0);
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
    std::fs::write(&tmp_path, serde_json::to_string_pretty(cfg).expect("serialize config"))?;
    std::fs::rename(&tmp_path, path)
}

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
        let dir = std::env::temp_dir().join("vhud-test-migrate");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
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
        let dir = std::env::temp_dir().join("vhud-test-load-clamp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, r#"{"opacity":5.0,"padding":-10.0}"#).unwrap();
        let c = load(&path);
        assert_eq!(c.opacity, 1.0);
        assert_eq!(c.padding, 0.0);
    }

    #[test]
    fn save_load_round_trip() {
        let dir = std::env::temp_dir().join("vhud-test-config");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("config.json");
        let mut c = Config::default();
        c.opacity = 0.5;
        c.legend_color = "#ffcc00".into();
        c.shift_color = "#00ccff".into();
        c.alternate_color = "#ff88cc".into();
        c.show_layer_action_icons = true;
        c.show_shift_icons = false;
        c.show_alternate_action_icons = false;
        c.window_by_monitor.insert("test".into(), WindowRect { x: 10.0, y: 20.0, w: 800.0, h: 300.0 });
        c.last_monitor = Some("test".into());
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

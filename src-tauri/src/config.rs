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
    pub char_opacity: f64,
    pub border_opacity: f64,
    pub border_width: f64,
    pub grab_combo: Vec<String>,
    pub use_oryx_colors: bool,
    pub key_fill_color: String,
    pub key_fill_opacity: f64,
    pub padding: f64,
    pub bg_color: String,
    pub text_color: String,
    pub legend_color: String,
    pub border_color: String,
    pub window: Option<WindowRect>,
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
            key_fill_color: "#ffffff".into(),
            key_fill_opacity: 0.0,
            padding: 10.0,
            bg_color: "#141418".into(),
            text_color: "#ffffff".into(),
            legend_color: "#ffffff".into(),
            border_color: "#ffffff".into(),
            window: None,
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
        assert_eq!(c.char_opacity, 1.0);
        assert_eq!(c.border_opacity, 0.35);
        assert_eq!(c.border_width, 1.0);
        assert_eq!(c.bg_color, "#141418");
        assert_eq!(c.key_fill_opacity, 0.0);
        assert_eq!(c.padding, 10.0);
        assert_eq!(c.text_color, "#ffffff");
        assert_eq!(c.legend_color, "#ffffff");
        assert_eq!(c.border_color, "#ffffff");
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
            ..Config::default()
        };
        c.clamp();
        assert_eq!(c.opacity, 1.0);
        assert_eq!(c.char_opacity, 0.2);
        assert_eq!(c.border_opacity, 0.0);
        assert_eq!(c.border_width, 5.0);
        assert_eq!(c.key_fill_opacity, 1.0);
        assert_eq!(c.padding, 0.0);
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

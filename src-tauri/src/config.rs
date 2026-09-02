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

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Volume,
    Brightness,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChordAction {
    SnippingTool,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Action assigned to Back button (BTN_SIDE / BTN_BACK) + scroll
    pub back_action: Action,
    /// Action assigned to Forward button (BTN_EXTRA / BTN_FORWARD) + scroll
    pub forward_action: Action,
    /// Action when pressing both side buttons together
    pub chord_action: ChordAction,
    /// Max time in ms between pressing both buttons to arm the chord (default 300ms)
    pub chord_window_ms: u64,
    /// Brightness step percentage per wheel tick (default 5%)
    pub brightness_step_pct: i32,
    /// Volume step percentage per wheel tick (default 5%)
    pub volume_step_pct: i32,
    /// Optional specific mouse device name or path to bind to (None = auto-detect)
    pub target_device: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            back_action: Action::Volume,
            forward_action: Action::Brightness,
            chord_action: ChordAction::SnippingTool,
            chord_window_ms: 300,
            brightness_step_pct: 5,
            volume_step_pct: 5,
            target_device: None,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let base = match std::env::var("XDG_CONFIG_HOME") {
            Ok(p) => PathBuf::from(p),
            Err(_) => {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".config")
            }
        };
        base.join("sidekickmc").join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str::<Config>(&content) {
                    log::info!("Loaded configuration from {}", path.display());
                    return cfg;
                }
            }
        }
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;
        fs::write(path, content)
    }
}

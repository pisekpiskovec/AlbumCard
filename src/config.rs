use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub default_void_size: u32,
    pub max_count_of_tracks_in_single: u32,
    pub skip_saving_track_position_in_single: bool,
    pub rename_pattern_ep: String,
    pub rename_pattern_single: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_void_size: 1,
            max_count_of_tracks_in_single: 1,
            skip_saving_track_position_in_single: true,
            rename_pattern_ep: "%{artist} - %{album} - %{title}".to_owned(),
            rename_pattern_single: "%{artist} - %{title}".to_owned(),
        }
    }
}

impl Config {
    fn get_config_path() -> Result<PathBuf, String> {
        let mut path = dirs::config_dir().unwrap_or_else(|| std::env::home_dir().unwrap());
        path.push("albumcard");
        fs::create_dir_all(&path).ok();
        path.push("config.toml");
        Ok(path)
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::get_config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {}", e))?;
        toml::from_str(&contents).map_err(|e| format!("Failed to parse config: {}", e))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let toml_string = toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&path, toml_string).map_err(|e| format!("Failed to write config: {}", e))
    }
}

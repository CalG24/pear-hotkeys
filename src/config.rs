use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
    pub hotkey_like: String,
    pub hotkey_dislike: String,
    pub debounce_ms: u64,
    pub sound_enabled: bool,
    pub show_notifications: bool,
    pub start_with_windows: bool,
    pub minimise_to_tray: bool,
    pub start_minimised: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:26538".to_string(),
            hotkey_like: "Alt+L".to_string(),
            hotkey_dislike: "Alt+D".to_string(),
            debounce_ms: 300,
            sound_enabled: true,
            show_notifications: true,
            start_with_windows: false,
            minimise_to_tray: true,
            start_minimised: false,
        }
    }
}

impl Config {
    pub fn dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("could not resolve %APPDATA%")?
            .join("PearDesktop");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn path() -> Result<PathBuf> {
        Ok(Self::dir()?.join("config.toml"))
    }

	#[allow(dead_code)] // reserved for future log-viewer / diagnostics UI
    pub fn log_path() -> Result<PathBuf> {
        Ok(Self::dir()?.join("pear.log"))
    }

    pub fn load() -> Self {
        match Self::try_load() {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!("Failed to load config, using defaults: {e:#}");
                Config::default()
            }
        }
    }

    fn try_load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            let cfg = Config::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, toml::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

const CONFIG_DIR: &str = "funcspec";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// Active profile name
    #[serde(default = "default_profile")]
    pub active_profile: String,

    /// Named profiles
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub host: String,
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_project: Option<String>,
}

fn default_profile() -> String {
    "default".into()
}

impl Config {
    /// Load config from disk, or return default if not found.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let config: Config =
            toml::from_str(&content).with_context(|| "Failed to parse config.toml")?;
        Ok(config)
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Get the active profile, with env var overrides.
    pub fn active_profile(&self) -> Option<Profile> {
        // Env vars override stored config
        let env_key = std::env::var("FUNCSPEC_API_KEY").ok();
        let env_host = std::env::var("FUNCSPEC_HOST").ok();

        if let Some(key) = env_key {
            return Some(Profile {
                host: env_host.unwrap_or_else(|| "https://funcspec.net".into()),
                api_key: key,
                default_project: None,
            });
        }

        self.profiles.get(&self.active_profile).cloned()
    }

    /// Get the config file path, respecting XDG_CONFIG_HOME.
    pub fn config_path() -> Result<PathBuf> {
        let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
        } else {
            dirs::config_dir().context("Cannot determine config directory")?
        };
        Ok(base.join(CONFIG_DIR).join(CONFIG_FILE))
    }
}

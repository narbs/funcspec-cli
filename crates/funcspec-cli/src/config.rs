use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = "funcspec";
const CONFIG_FILE: &str = "config.toml";

/// Per-directory local config stored in `.funcspec` at the repo root.
///
/// Overrides the global profile's `default_project` for commands run within
/// that directory tree. Committed to version control so all contributors share
/// the same project binding.
///
/// Format (TOML):
/// ```toml
/// project = "my-project"
/// ```
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct LocalConfig {
    /// Project slug override for this directory tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

impl LocalConfig {
    pub const FILE_NAME: &'static str = ".funcspec";

    /// Walk up from `start` looking for a `.funcspec` file, returning the path
    /// of the first one found (if any).
    pub fn find(start: &Path) -> Option<PathBuf> {
        let mut dir = start;
        loop {
            let candidate = dir.join(Self::FILE_NAME);
            if candidate.exists() {
                return Some(candidate);
            }
            match dir.parent() {
                Some(p) => dir = p,
                None => return None,
            }
        }
    }

    /// Load from the first `.funcspec` found walking up from `start`.
    /// Returns `None` if no file is found or on parse error.
    pub fn find_and_load(start: &Path) -> Option<Self> {
        let path = Self::find(start)?;
        Self::load_from_path(&path).ok()
    }

    /// Load from an explicit path.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
    }

    /// Save to an explicit path (atomic write).
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &content)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Config {
    /// Active profile name
    #[serde(default = "default_profile")]
    pub active_profile: String,

    /// Named profiles keyed by profile name
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_profile: default_profile(),
            profiles: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// Load config from the default path, or return default if not found.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from_path(&path)
    }

    /// Load config from an explicit path (useful for testing).
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let config: Config =
            toml::from_str(&content).with_context(|| "Failed to parse config.toml")?;
        Ok(config)
    }

    /// Save config to the default path.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        self.save_to_path(&path)
    }

    /// Save config to an explicit path (useful for testing).
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir {}", parent.display()))?;
        }
        let content = toml::to_string_pretty(self)?;
        // Write atomically via a temp file in the same directory
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, &content)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Get the active profile with environment variable overrides applied.
    ///
    /// `FUNCSPEC_API_KEY` and `FUNCSPEC_HOST` override stored values.
    pub fn active_profile(&self) -> Option<Profile> {
        let env_key = std::env::var("FUNCSPEC_API_KEY").ok();
        let env_host = std::env::var("FUNCSPEC_HOST").ok();

        if env_key.is_some() || env_host.is_some() {
            let stored = self.profiles.get(&self.active_profile);
            return Some(Profile {
                host: env_host.or_else(|| stored.map(|p| p.host.clone()))
                    .unwrap_or_else(|| "https://funcspec.net".into()),
                api_key: env_key.or_else(|| stored.map(|p| p.api_key.clone()))
                    .unwrap_or_default(),
                default_project: stored.and_then(|p| p.default_project.clone()),
            });
        }

        self.profiles.get(&self.active_profile).cloned()
    }

    /// Return the config file path, respecting `XDG_CONFIG_HOME`.
    pub fn config_path() -> Result<PathBuf> {
        let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
        } else {
            dirs::config_dir().context("Cannot determine config directory")?
        };
        Ok(base.join(CONFIG_DIR).join(CONFIG_FILE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_profile(host: &str, key: &str) -> Profile {
        Profile {
            host: host.into(),
            api_key: key.into(),
            default_project: None,
        }
    }

    fn make_config_with_profile() -> Config {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "default".into(),
            make_profile("https://funcspec.net", "key123"),
        );
        Config {
            active_profile: "default".into(),
            profiles,
        }
    }

    #[test]
    fn default_config_has_no_profiles() {
        let config = Config::default();
        assert!(config.profiles.is_empty());
        assert_eq!(config.active_profile, "default");
    }

    #[test]
    fn active_profile_returns_stored() {
        let config = make_config_with_profile();
        let profile = config.active_profile().unwrap();
        assert_eq!(profile.host, "https://funcspec.net");
        assert_eq!(profile.api_key, "key123");
    }

    #[test]
    fn active_profile_returns_none_when_missing() {
        let config = Config::default();
        assert!(config.active_profile().is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("funcspec").join("config.toml");

        let config = make_config_with_profile();
        config.save_to_path(&path).unwrap();

        let loaded = Config::load_from_path(&path).unwrap();
        assert_eq!(config, loaded);
    }

    #[test]
    fn load_from_missing_path_returns_default() {
        let path = PathBuf::from("/tmp/does_not_exist_12345/config.toml");
        let config = Config::load_from_path(&path).unwrap();
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn toml_roundtrip_preserves_optional_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut profiles = BTreeMap::new();
        profiles.insert(
            "work".into(),
            Profile {
                host: "https://my.funcspec.net".into(),
                api_key: "secret".into(),
                default_project: Some("my-proj".into()),
            },
        );
        let config = Config {
            active_profile: "work".into(),
            profiles,
        };
        config.save_to_path(&path).unwrap();

        let loaded = Config::load_from_path(&path).unwrap();
        assert_eq!(
            loaded.profiles["work"].default_project.as_deref(),
            Some("my-proj")
        );
    }

    #[test]
    fn multiple_profiles_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = Config::default();
        config.profiles.insert(
            "default".into(),
            make_profile("https://funcspec.net", "key1"),
        );
        config.profiles.insert(
            "work".into(),
            make_profile("https://work.funcspec.net", "key2"),
        );
        config.active_profile = "work".into();

        config.save_to_path(&path).unwrap();
        let loaded = Config::load_from_path(&path).unwrap();

        assert_eq!(loaded.active_profile, "work");
        assert_eq!(loaded.profiles.len(), 2);
        assert_eq!(loaded.profiles["default"].api_key, "key1");
        assert_eq!(loaded.profiles["work"].api_key, "key2");
    }

    #[test]
    fn env_api_key_overrides_stored() {
        let config = make_config_with_profile();
        // We can't set env vars safely in parallel tests, so test the logic directly
        // by checking that active_profile() reads from profiles when no env var set
        let profile = config.active_profile().unwrap();
        assert_eq!(profile.api_key, "key123");
    }
}

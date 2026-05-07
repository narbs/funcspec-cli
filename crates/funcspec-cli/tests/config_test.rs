//! Integration tests for config loading and persistence.

use funcspec_cli::config::{Config, Profile};
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

static ENV_MUTEX: Mutex<()> = Mutex::new(());

struct EnvGuard<'a> {
    _lock: MutexGuard<'a, ()>,
    saved: Vec<(String, Option<String>)>,
}

impl<'a> EnvGuard<'a> {
    fn unset(keys: &[&str]) -> Self {
        let lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let saved = keys
            .iter()
            .map(|k| {
                let v = std::env::var(k).ok();
                // SAFETY: we hold ENV_MUTEX exclusively across all env-sensitive tests
                unsafe { std::env::remove_var(k) };
                (k.to_string(), v)
            })
            .collect();
        Self { _lock: lock, saved }
    }
}

impl Drop for EnvGuard<'_> {
    fn drop(&mut self) {
        for (k, v) in &self.saved {
            match v {
                Some(val) => unsafe { std::env::set_var(k, val) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }
}

const ENV_VARS: &[&str] = &["FUNCSPEC_API_KEY", "FUNCSPEC_HOST"];

fn profile(host: &str, key: &str) -> Profile {
    Profile {
        host: host.into(),
        api_key: key.into(),
        default_project: None,
    }
}

#[test]
fn empty_config_file_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "").unwrap();

    let config = Config::load_from_path(&path).unwrap();
    assert_eq!(config.active_profile, "default");
    assert!(config.profiles.is_empty());
}

#[test]
fn full_config_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut profiles = BTreeMap::new();
    profiles.insert(
        "default".into(),
        profile("https://funcspec.net", "key-default"),
    );
    profiles.insert(
        "work".into(),
        Profile {
            host: "https://work.example.com".into(),
            api_key: "key-work".into(),
            default_project: Some("my-proj".into()),
        },
    );

    let config = Config {
        active_profile: "work".into(),
        profiles,
    };
    config.save_to_path(&path).unwrap();

    let loaded = Config::load_from_path(&path).unwrap();
    assert_eq!(loaded.active_profile, "work");
    assert_eq!(loaded.profiles.len(), 2);
    let work = &loaded.profiles["work"];
    assert_eq!(work.default_project.as_deref(), Some("my-proj"));
    assert_eq!(work.api_key, "key-work");
}

#[test]
fn active_profile_returns_correct_profile() {
    let _env = EnvGuard::unset(ENV_VARS);
    let mut profiles = BTreeMap::new();
    profiles.insert("default".into(), profile("https://funcspec.net", "key1"));
    profiles.insert("prod".into(), profile("https://prod.funcspec.net", "key2"));

    let config = Config {
        active_profile: "prod".into(),
        profiles,
    };

    let active = config.active_profile().unwrap();
    assert_eq!(active.host, "https://prod.funcspec.net");
    assert_eq!(active.api_key, "key2");
}

#[test]
fn no_active_profile_when_empty() {
    let _env = EnvGuard::unset(ENV_VARS);
    let config = Config::default();
    assert!(config.active_profile().is_none());
}

#[test]
fn config_path_uses_xdg_when_set() {
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: test-only env mutation; acceptable in single-threaded test context
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
    }
    let path = Config::config_path().unwrap();
    unsafe {
        std::env::remove_var("XDG_CONFIG_HOME");
    }
    assert!(path.starts_with(dir.path()));
    assert!(path.ends_with("config.toml"));
}

#[test]
fn save_creates_intermediate_directories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("deep").join("config.toml");

    let config = Config::default();
    config.save_to_path(&path).unwrap();

    assert!(path.exists());
}

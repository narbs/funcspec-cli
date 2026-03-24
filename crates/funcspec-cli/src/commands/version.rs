use anyhow::Result;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

pub fn run() -> Result<()> {
    println!("{PKG_NAME} {VERSION}");
    println!("Build: Rust {}", rustc_version());
    if let Some(host) = configured_host() {
        println!("Host:  {host}");
    } else {
        println!("Host:  (not configured — run `funcspec auth login`)");
    }
    Ok(())
}

fn configured_host() -> Option<String> {
    // Check env var first
    if let Ok(h) = std::env::var("FUNCSPEC_HOST") {
        return Some(h);
    }
    // Try loading from config
    crate::config::Config::load()
        .ok()
        .and_then(|c| c.active_profile())
        .map(|p| p.host)
}

fn rustc_version() -> String {
    // Captured at build time via RUSTC_VERSION env var, if set
    option_env!("RUSTC_VERSION")
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_string_is_semver() {
        // Just verify it's set and non-empty
        assert!(!VERSION.is_empty());
        // Should look like major.minor.patch
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn version_run_succeeds() {
        // Should not panic
        run().unwrap();
    }
}

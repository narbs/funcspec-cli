use anyhow::{Context, Result};
use rust_i18n::t;

use crate::config::LocalConfig;
use crate::context::{client_and_config, project_slug_override, resolve_project_slug};

/// Arguments for `funcspec view`.
pub struct ViewArgs {
    pub item_id: Option<String>,
    pub url: bool,
}

pub fn build_command() -> clap::Command {
    clap::Command::new("view")
        .about(t!("cmd.view.about").to_string())
        .long_about(t!("cmd.view.long_about").to_string())
        .arg(clap::Arg::new("item_id").help(t!("cmd.view.item_id").to_string()))
        .arg(
            clap::Arg::new("url")
                .long("url")
                .action(clap::ArgAction::SetTrue)
                .help(t!("cmd.view.url").to_string()),
        )
}

pub fn from_arg_matches(matches: &clap::ArgMatches) -> ViewArgs {
    ViewArgs {
        item_id: matches.get_one::<String>("item_id").cloned(),
        url: matches.get_flag("url"),
    }
}

pub async fn run(args: ViewArgs) -> Result<()> {
    let (client, config) = client_and_config()?;
    let profile = config
        .active_profile()
        .context("Not logged in. Run `funcspec auth login`.")?;

    let cwd = std::env::current_dir().unwrap_or_default();
    let local = LocalConfig::find_and_load(&cwd);
    let project_slug = resolve_project_slug(
        project_slug_override(),
        profile.default_project.as_deref(),
        local.as_ref(),
    )
    .context(
        "No project specified. Use --project <slug>, set FUNCSPEC_PROJECT, \
         add a .funcspec file, or run `funcspec onboard`.",
    )?;

    let project = client
        .get_project(&project_slug)
        .await
        .with_context(|| format!("Project '{}' not found", project_slug))?;

    let view_url = if let Some(ref item_id) = args.item_id {
        // Fetch the item and use its canonical URL
        let item = client
            .get_item(project.id, item_id)
            .await
            .with_context(|| format!("Item '{}' not found", item_id))?;
        item.attributes.url.clone()
    } else {
        // Construct the project URL: host/projects/:id
        let host = profile.host.trim_end_matches('/');
        format!("{}/projects/{}", host, project.id)
    };

    if args.url {
        println!("{view_url}");
    } else {
        eprintln!("Opening {view_url}");
        open::that(&view_url)
            .with_context(|| format!("Failed to open browser for URL: {view_url}"))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_url_construction() {
        let host = "https://app.funcspec.io";
        let project_id = 42u64;
        let url = format!("{}/projects/{}", host.trim_end_matches('/'), project_id);
        assert_eq!(url, "https://app.funcspec.io/projects/42");
    }

    #[test]
    fn project_url_host_trailing_slash() {
        let host = "https://app.funcspec.io/";
        let project_id = 7u64;
        let url = format!("{}/projects/{}", host.trim_end_matches('/'), project_id);
        assert_eq!(url, "https://app.funcspec.io/projects/7");
    }

    #[test]
    fn view_args_defaults() {
        let args = ViewArgs {
            item_id: None,
            url: false,
        };
        assert!(args.item_id.is_none());
        assert!(!args.url);
    }

    #[test]
    fn view_args_with_item_id() {
        let args = ViewArgs {
            item_id: Some("F-377".into()),
            url: false,
        };
        assert_eq!(args.item_id.as_deref(), Some("F-377"));
    }

    #[test]
    fn view_args_url_flag() {
        let args = ViewArgs {
            item_id: None,
            url: true,
        };
        assert!(args.url);
    }

    #[test]
    fn build_command_parses_url_flag() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["view", "--url"]).unwrap();
        assert!(m.get_flag("url"));
    }

    #[test]
    fn build_command_parses_item_id() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["view", "F-377"]).unwrap();
        assert_eq!(m.get_one::<String>("item_id").unwrap(), "F-377");
    }

    #[test]
    fn build_command_no_args() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["view"]).unwrap();
        assert!(m.get_one::<String>("item_id").is_none());
        assert!(!m.get_flag("url"));
    }
}

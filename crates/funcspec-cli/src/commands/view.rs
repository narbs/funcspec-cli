use anyhow::{Context, Result};
use clap::Args;

use crate::context::client_and_config;

/// Arguments for `funcspec view`.
#[derive(Debug, Args)]
#[command(
    about = "Open project spec (or a specific item) in the browser",
    long_about = "Open the current project's spec page in the default browser.\n\
                  Pass an item ID (e.g. F-377) to jump directly to that item.\n\
                  Use --url to print the URL without opening a browser."
)]
pub struct ViewArgs {
    /// Item ID to open (e.g. F-377); omit to view the full project spec
    pub item_id: Option<String>,

    /// Print the URL instead of opening the browser
    #[arg(long)]
    pub url: bool,
}

pub async fn run(args: ViewArgs) -> Result<()> {
    let (client, config) = client_and_config()?;
    let profile = config
        .active_profile()
        .context("Not logged in. Run `funcspec auth login`.")?;
    let project_slug = profile
        .default_project
        .as_deref()
        .context("No default project set. Run `funcspec projects set-default <slug>`.")?;
    let project = client
        .get_project(project_slug)
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
        // Construct the project spec view URL from the configured host
        let host = profile.host.trim_end_matches('/');
        format!("{}/projects/{}/spec", host, project.attributes.slug)
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
        let slug = "my-project";
        let url = format!("{}/projects/{}/spec", host.trim_end_matches('/'), slug);
        assert_eq!(url, "https://app.funcspec.io/projects/my-project/spec");
    }

    #[test]
    fn project_url_host_trailing_slash() {
        let host = "https://app.funcspec.io/";
        let slug = "demo";
        let url = format!("{}/projects/{}/spec", host.trim_end_matches('/'), slug);
        assert_eq!(url, "https://app.funcspec.io/projects/demo/spec");
    }

    #[test]
    fn view_args_defaults() {
        // item_id defaults to None, url defaults to false
        let args = ViewArgs { item_id: None, url: false };
        assert!(args.item_id.is_none());
        assert!(!args.url);
    }

    #[test]
    fn view_args_with_item_id() {
        let args = ViewArgs { item_id: Some("F-377".into()), url: false };
        assert_eq!(args.item_id.as_deref(), Some("F-377"));
    }

    #[test]
    fn view_args_url_flag() {
        let args = ViewArgs { item_id: None, url: true };
        assert!(args.url);
    }
}

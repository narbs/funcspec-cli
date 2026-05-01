use funcspec_cli::cli::build_cli;
use funcspec_cli::commands;
use funcspec_cli::context;
use funcspec_cli::output::OutputFormat;

use anyhow::Result;
use colored::Colorize;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() {
    // Initialise locale: FUNCSPEC_LANG > LANG > "en"
    let raw_locale = std::env::var("FUNCSPEC_LANG")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en".to_string());
    let locale = raw_locale
        .split(['.', '@'])
        .next()
        .unwrap_or("en")
        .replace('_', "-");
    rust_i18n::set_locale(&locale);

    let matches = build_cli().get_matches();

    // Initialise tracing based on verbosity flags
    let filter = if matches.get_flag("debug") {
        EnvFilter::new("debug")
    } else if matches.get_flag("verbose") {
        EnvFilter::new("info")
    } else {
        EnvFilter::from_default_env().add_directive(tracing::Level::WARN.into())
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(filter)
        .init();

    let format = matches
        .get_one::<OutputFormat>("format")
        .copied()
        .unwrap_or_default();

    let project = matches.get_one::<String>("project").cloned();
    context::set_project_override(project);

    if let Err(err) = run(matches, format).await {
        eprintln!("{} {err:#}", "error:".red().bold());
        std::process::exit(1);
    }
}

async fn run(matches: clap::ArgMatches, format: OutputFormat) -> Result<()> {
    match matches.subcommand() {
        Some(("ai", sub)) => commands::ai::dispatch(sub).await,
        Some(("auth", sub)) => commands::auth::dispatch(sub).await,
        Some(("config", sub)) => commands::config::dispatch(sub).await,
        Some(("projects", sub)) => commands::projects::dispatch(sub, format).await,
        Some(("items", sub)) => commands::items::dispatch(sub, format).await,
        Some(("edges", sub)) => commands::edges::dispatch(sub, format).await,
        Some(("instructions", sub)) => {
            let args = commands::instructions::from_arg_matches(sub);
            commands::instructions::run(args, format).await
        }
        Some(("onboard", sub)) => {
            let args = commands::onboard::from_arg_matches(sub);
            commands::onboard::run(args).await
        }
        Some(("doctor", sub)) => {
            let args = commands::doctor::from_arg_matches(sub);
            commands::doctor::run(args).await
        }
        Some(("search", sub)) => {
            let args = commands::search::from_arg_matches(sub);
            commands::search::run(args, format).await
        }
        Some(("stats" | "status", sub)) => {
            let args = commands::stats::from_arg_matches(sub);
            commands::stats::run(args, format).await
        }
        Some(("export", sub)) => {
            let args = commands::export::from_arg_matches(sub);
            commands::export::run(args).await
        }
        Some(("snapshots", sub)) => commands::snapshots::dispatch(sub, format).await,
        Some(("view", sub)) => {
            let args = commands::view::from_arg_matches(sub);
            commands::view::run(args).await
        }
        Some(("version", _)) => commands::version::run(),
        Some(("completion", sub)) => {
            let shell = sub
                .get_one::<clap_complete::Shell>("shell")
                .copied()
                .unwrap();
            let mut cmd = build_cli();
            clap_complete::generate(shell, &mut cmd, "funcspec", &mut std::io::stdout());
            Ok(())
        }
        _ => {
            build_cli().print_help().ok();
            Ok(())
        }
    }
}

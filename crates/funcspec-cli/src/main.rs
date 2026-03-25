use funcspec_cli::commands;
use funcspec_cli::context;
use funcspec_cli::output::OutputFormat;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use colored::Colorize;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(
    name = "funcspec",
    about = "Command-line interface for FuncSpec — AI-driven spec management",
    version,
    propagate_version = true,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output (HTTP requests/responses)
    #[arg(long, short = 'v', global = true)]
    verbose: bool,

    /// Enable debug output (full headers, bodies, timing)
    #[arg(long, global = true)]
    debug: bool,

    /// Output format (default: table when TTY, json when piped)
    #[arg(long, global = true, value_enum, default_value = "auto")]
    format: OutputFormat,

    /// Project ID, slug, or org/project slug (overrides default project)
    #[arg(long, short = 'p', global = true)]
    project: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// AI-powered review, improvement, generation, and audit operations
    #[command(subcommand)]
    Ai(commands::ai::AiCmd),

    /// Manage authentication and credentials
    #[command(subcommand)]
    Auth(commands::auth::AuthCmd),

    /// Manage configuration values
    #[command(subcommand)]
    Config(commands::config::ConfigCmd),

    /// List and manage projects
    #[command(subcommand)]
    Projects(commands::projects::ProjectsCmd),

    /// Manage spec items (functional and technical)
    #[command(subcommand)]
    Items(commands::items::ItemsCmd),

    /// Search spec items by full-text query
    Search(commands::search::SearchArgs),

    /// Show project stats and dashboard
    Stats(commands::stats::StatsArgs),

    /// Export project spec (markdown, JSON, CSV, HTML, PDF, DOCX)
    Export(commands::export::ExportArgs),

    /// Manage project snapshots (save/restore points)
    #[command(subcommand)]
    Snapshots(commands::snapshots::SnapshotsCmd),

    /// Open project spec in browser
    View(commands::view::ViewArgs),

    /// Show version information
    Version,

    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialise tracing based on verbosity flags
    let filter = if cli.debug {
        EnvFilter::new("debug")
    } else if cli.verbose {
        EnvFilter::new("info")
    } else {
        EnvFilter::from_default_env().add_directive(tracing::Level::WARN.into())
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(filter)
        .init();

    context::set_project_override(cli.project.clone());

    if let Err(err) = run(cli).await {
        eprintln!("{} {err:#}", "error:".red().bold());
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Ai(cmd) => commands::ai::run(cmd).await,
        Commands::Auth(cmd) => commands::auth::run(cmd).await,
        Commands::Config(cmd) => commands::config::run(cmd).await,
        Commands::Projects(cmd) => commands::projects::run(cmd, cli.format).await,
        Commands::Items(cmd) => commands::items::run(cmd, cli.format).await,
        Commands::Search(args) => commands::search::run(args, cli.format).await,
        Commands::Stats(args) => commands::stats::run(args, cli.format).await,
        Commands::Export(args) => commands::export::run(args).await,
        Commands::Snapshots(cmd) => commands::snapshots::run(cmd, cli.format).await,
        Commands::View(args) => commands::view::run(args).await,
        Commands::Version => commands::version::run(),
        Commands::Completion { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "funcspec", &mut std::io::stdout());
            Ok(())
        }
    }
}

mod commands;
mod config;
mod context;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "funcspec",
    about = "Command-line interface for FuncSpec — AI-driven spec management",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage authentication and credentials
    #[command(subcommand)]
    Auth(commands::auth::AuthCmd),

    /// List and manage projects
    #[command(subcommand)]
    Projects(commands::projects::ProjectsCmd),

    /// Manage spec items (functional and technical)
    #[command(subcommand)]
    Items(commands::items::ItemsCmd),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli).await {
        eprintln!("{} {err:#}", "error:".red().bold());
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Auth(cmd) => commands::auth::run(cmd).await,
        Commands::Projects(cmd) => commands::projects::run(cmd).await,
        Commands::Items(cmd) => commands::items::run(cmd).await,
    }
}

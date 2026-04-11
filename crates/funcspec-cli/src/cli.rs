use clap::{Arg, ArgAction, Command};
use clap::builder::EnumValueParser;
use rust_i18n::t;

use crate::commands;
use crate::output::OutputFormat;

pub fn build_cli() -> Command {
    Command::new("funcspec")
        .about(t!("cli.about").to_string())
        .version(env!("CARGO_PKG_VERSION"))
        .propagate_version(true)
        .arg_required_else_help(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .global(true)
                .action(ArgAction::Help)
                .help(t!("cli.help").to_string()),
        )
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .global(true)
                .action(ArgAction::Version)
                .help(t!("cli.version_flag").to_string()),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .global(true)
                .action(ArgAction::SetTrue)
                .help(t!("cli.verbose").to_string()),
        )
        .arg(
            Arg::new("debug")
                .long("debug")
                .global(true)
                .action(ArgAction::SetTrue)
                .help(t!("cli.debug").to_string()),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .global(true)
                .value_parser(EnumValueParser::<OutputFormat>::new())
                .default_value("auto")
                .help(t!("cli.format").to_string()),
        )
        .arg(
            Arg::new("project")
                .long("project")
                .short('p')
                .global(true)
                .help(t!("cli.project").to_string()),
        )
        .subcommand(commands::ai::build_command())
        .subcommand(commands::auth::build_command())
        .subcommand(commands::config::build_command())
        .subcommand(commands::projects::build_command())
        .subcommand(commands::items::build_command())
        .subcommand(commands::edges::build_command())
        .subcommand(commands::instructions::build_command())
        .subcommand(commands::onboard::build_command())
        .subcommand(commands::doctor::build_command())
        .subcommand(commands::search::build_command())
        .subcommand(commands::stats::build_command())
        .subcommand(commands::export::build_command())
        .subcommand(commands::snapshots::build_command())
        .subcommand(commands::view::build_command())
        .subcommand(commands::version::build_command())
        .subcommand(
            Command::new("completion")
                .about(t!("cmd.completion.about").to_string())
                .arg(
                    Arg::new("shell")
                        .value_parser(EnumValueParser::<clap_complete::Shell>::new())
                        .required(true)
                        .help(t!("cmd.completion.shell").to_string()),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_macro_resolves_en() {
        rust_i18n::set_locale("en");
        let s = t!("cli.about").to_string();
        assert_ne!(s, "cli.about", "t! returned raw key — locale data not embedded");
        assert!(s.contains("FuncSpec"), "unexpected: {s}");
    }
}

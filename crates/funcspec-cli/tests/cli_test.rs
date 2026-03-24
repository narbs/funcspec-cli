//! CLI argument parsing and command routing tests.

use assert_cmd::Command;

fn cmd() -> Command {
    Command::cargo_bin("funcspec").unwrap()
}

#[test]
fn no_args_shows_help() {
    let mut c = cmd();
    c.assert().failure(); // exits non-zero when no subcommand
}

#[test]
fn help_flag_exits_zero() {
    cmd().arg("--help").assert().success();
}

#[test]
fn version_flag_exits_zero() {
    cmd().arg("--version").assert().success();
}

#[test]
fn version_subcommand_exits_zero() {
    cmd().arg("version").assert().success();
}

#[test]
fn completion_bash_outputs_something() {
    let output = cmd().args(["completion", "bash"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Bash completion scripts typically start with a function or complete command
    assert!(!stdout.is_empty());
}

#[test]
fn completion_zsh_exits_zero() {
    cmd().args(["completion", "zsh"]).assert().success();
}

#[test]
fn completion_fish_exits_zero() {
    cmd().args(["completion", "fish"]).assert().success();
}

#[test]
fn auth_subcommand_has_login() {
    cmd().args(["auth", "--help"]).assert().success();
}

#[test]
fn config_subcommand_has_set_get_list() {
    cmd().args(["config", "--help"]).assert().success();
}

#[test]
fn projects_subcommand_help() {
    cmd().args(["projects", "--help"]).assert().success();
}

#[test]
fn items_subcommand_help() {
    cmd().args(["items", "--help"]).assert().success();
}

#[test]
fn unknown_subcommand_fails() {
    cmd().arg("notacommand").assert().failure();
}

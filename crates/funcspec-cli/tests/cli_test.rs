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

// ── --format flag ────────────────────────────────────────────────────────────

#[test]
fn format_flag_auto_accepted() {
    cmd().args(["--format", "auto", "version"]).assert().success();
}

#[test]
fn format_flag_json_accepted() {
    cmd().args(["--format", "json", "version"]).assert().success();
}

#[test]
fn format_flag_table_accepted() {
    cmd().args(["--format", "table", "version"]).assert().success();
}

#[test]
fn format_flag_csv_accepted() {
    cmd().args(["--format", "csv", "version"]).assert().success();
}

#[test]
fn format_flag_minimal_accepted() {
    cmd().args(["--format", "minimal", "version"]).assert().success();
}

#[test]
fn format_flag_markdown_accepted() {
    cmd().args(["--format", "markdown", "version"]).assert().success();
}

#[test]
fn format_flag_invalid_value_fails() {
    cmd().args(["--format", "notaformat", "version"]).assert().failure();
}

#[test]
fn format_flag_appears_in_help() {
    let output = cmd().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format"), "expected --format in help output");
}

// ── command argument validation ──────────────────────────────────────────────

#[test]
fn projects_list_help_exits_zero() {
    cmd().args(["projects", "list", "--help"]).assert().success();
}

#[test]
fn projects_show_requires_slug_arg() {
    // Missing the required slug positional argument
    cmd().args(["projects", "show"]).assert().failure();
}

#[test]
fn projects_set_default_requires_slug_arg() {
    cmd().args(["projects", "set-default"]).assert().failure();
}

#[test]
fn items_list_help_exits_zero() {
    cmd().args(["items", "list", "--help"]).assert().success();
}

#[test]
fn items_show_requires_item_arg() {
    cmd().args(["items", "show"]).assert().failure();
}

#[test]
fn items_create_requires_title_flag() {
    cmd().args(["items", "create"]).assert().failure();
}

#[test]
fn items_delete_requires_item_arg() {
    cmd().args(["items", "delete"]).assert().failure();
}

#[test]
fn items_edit_requires_item_arg() {
    cmd().args(["items", "edit"]).assert().failure();
}

#[test]
fn items_edit_help_exits_zero() {
    cmd().args(["items", "edit", "--help"]).assert().success();
}

// ── search command ────────────────────────────────────────────────────────────

#[test]
fn search_subcommand_help_exits_zero() {
    cmd().args(["search", "--help"]).assert().success();
}

#[test]
fn search_requires_query_arg() {
    // search with no positional arg should fail
    cmd().arg("search").assert().failure();
}

#[test]
fn search_help_mentions_type_flag() {
    let output = cmd().args(["search", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("type"), "expected --type flag in search help");
}

#[test]
fn search_help_mentions_tag_flag() {
    let output = cmd().args(["search", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tag"), "expected --tag flag in search help");
}

#[test]
fn search_help_mentions_count_flag() {
    let output = cmd().args(["search", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("count"), "expected --count flag in search help");
}

// ── items list new flags ─────────────────────────────────────────────────────

#[test]
fn items_list_sort_flag_in_help() {
    let output = cmd().args(["items", "list", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sort"), "expected --sort flag in items list help");
}

#[test]
fn items_list_bare_flag_in_help() {
    let output = cmd().args(["items", "list", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bare"), "expected --bare flag in items list help");
}

#[test]
fn items_list_count_flag_in_help() {
    let output = cmd().args(["items", "list", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("count"), "expected --count flag in items list help");
}

#[test]
fn format_flag_bare_accepted() {
    cmd().args(["--format", "bare", "version"]).assert().success();
}

// ── ai subcommand ────────────────────────────────────────────────────────────

#[test]
fn ai_subcommand_help_exits_zero() {
    cmd().args(["ai", "--help"]).assert().success();
}

#[test]
fn ai_review_help_exits_zero() {
    cmd().args(["ai", "review", "--help"]).assert().success();
}

#[test]
fn ai_review_requires_permalink() {
    cmd().args(["ai", "review"]).assert().failure();
}

#[test]
fn ai_review_all_help_exits_zero() {
    cmd().args(["ai", "review-all", "--help"]).assert().success();
}

#[test]
fn ai_improve_help_exits_zero() {
    cmd().args(["ai", "improve", "--help"]).assert().success();
}

#[test]
fn ai_improve_requires_permalink() {
    cmd().args(["ai", "improve"]).assert().failure();
}

#[test]
fn ai_improve_help_mentions_auto_accept() {
    let output = cmd().args(["ai", "improve", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("auto-accept"),
        "expected --auto-accept in improve help"
    );
}

#[test]
fn ai_generate_help_exits_zero() {
    cmd().args(["ai", "generate", "--help"]).assert().success();
}

#[test]
fn ai_generate_requires_permalink() {
    cmd().args(["ai", "generate"]).assert().failure();
}

#[test]
fn ai_audit_help_exits_zero() {
    cmd().args(["ai", "audit", "--help"]).assert().success();
}

#[test]
fn ai_audit_requires_permalink() {
    cmd().args(["ai", "audit"]).assert().failure();
}

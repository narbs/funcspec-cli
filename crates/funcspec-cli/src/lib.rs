// Library entry point — exposes modules for integration tests.
pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod output;

// Initialise the i18n translation registry. Locale files are loaded from
// `crates/funcspec-cli/locales/` at compile time and embedded in the binary.
rust_i18n::i18n!("locales");

=== BUILD & DISTRIBUTION SPECS ===

## F-11: Build & Distribution Pipeline

Cross-platform build and release pipeline for the CLI binary.

Targets: Linux x86_64 (glibc + musl static), Linux aarch64, macOS x86_64, macOS aarch64, Windows x86_64.

Distribution: cargo install, GitHub Releases with prebuilt binaries, Homebrew tap, shell installer script on funcspec.net.

CI/CD via GitHub Actions: cross-compile on tag push, run tests/clippy/fmt on PR. Use cross or cargo-zigbuild. Stripped and compressed binaries with SHA256 checksums.

Versioning: SemVer. funcspec --version shows version, git SHA, build date, target triple. Embedded at build time via built or vergen crate.

## T-83: GitHub Actions CI/CD workflow with cross-compilation matrix

Create .github/workflows/ci.yml and .github/workflows/release.yml. CI workflow: trigger on PR, run cargo fmt --check, cargo clippy --all-targets --all-features, cargo test --all-features across matrix of ubuntu-latest, macos-latest, windows-latest. Release workflow: trigger on tag push (v*.*.*), use cross-compilation matrix for targets linux-x86_64-gnu, linux-x86_64-musl, linux-aarch64-gnu, macos-x86_64, macos-aarch64, windows-x86_64. Use cross-rs/cross action or cargo-zigbuild for cross-compilation. Strip binaries with cargo-strip, compress with gzip/zip. Generate SHA256 checksums. Upload artifacts to GitHub Releases with release notes from CHANGELOG.md. Set up GitHub secrets for deployment keys. Handle musl static linking with musl-tools. Acceptance criteria: successful builds for all targets, artifacts uploaded with checksums, tests pass in CI.

**Rationale:** Core build and deployment infrastructure needs separate implementation from binary versioning logic

## T-84: Build-time version embedding with built crate integration

Add built crate to Cargo.toml dependencies. Create build.rs script that uses built::util::strptime_to_git_describe for git SHA extraction. Generate src/version.rs with VERSION constant from Cargo.toml, GIT_SHA from built::GIT_COMMIT_HASH, BUILD_DATE from built::BUILT_TIME_UTC, and TARGET from built::TARGET. Implement --version flag handler in main.rs that outputs formatted string: 'funcspec v{VERSION} ({GIT_SHA}) built on {BUILD_DATE} for {TARGET}'. Handle missing git info gracefully with 'unknown' fallback. Follow SemVer format validation. Store constants as static str references for binary size optimization. Acceptance criteria: --version flag works locally and in CI builds, shows correct information for each target platform.

**Rationale:** Version information embedding requires build-time code generation separate from CI/CD pipeline

## T-85: Cross-platform binary optimization and compression configuration

Configure Cargo.toml with release profile optimizations: opt-level = 'z', lto = true, codegen-units = 1, panic = 'abort', strip = true. Create target-specific configurations in .cargo/config.toml for musl static linking with RUSTFLAGS='-C target-feature=+crt-static'. Set up cross-compilation dependencies: cross.toml configuration for custom Docker images if needed. Configure binary naming convention: funcspec-{version}-{target}.{ext} where ext is empty for Unix, .exe for Windows. Implement compression in CI: use gzip for Unix targets, zip for Windows. Generate SHA256 checksums with shasum -a 256. Optimize binary size with wee_alloc or similar for embedded targets. Acceptance criteria: binaries under 10MB compressed, static musl builds work without libc dependencies, all targets boot and show version.

**Rationale:** Binary optimization and packaging logic is distinct from CI workflow orchestration

## T-86: Homebrew tap repository with formula generation

Create separate GitHub repository funcspec-tap with homebrew formula. Generate funcspec.rb formula template with class FuncSpec < Formula, desc, homepage, license fields. Implement url and sha256 hash extraction from GitHub releases API. Set up automated formula updates via GitHub Actions workflow triggered by repository_dispatch from main repo release workflow. Formula should support both x86_64 and arm64 macOS architectures using if Hardware::CPU.arm? conditional. Include test block with system bin/'funcspec', '--version' assertion. Configure tap discovery with brew tap funcspec/tap. Document installation process: brew install funcspec/tap/funcspec. Handle version bumps automatically by parsing latest GitHub release. Acceptance criteria: brew install works for both Intel and Apple Silicon Macs, formula validates with brew audit.

**Rationale:** Homebrew distribution requires separate repository and formula maintenance workflow

## T-87: Shell installer script with platform detection and verification

Create install.sh shell script hosted on funcspec.net. Implement platform detection logic: uname -m for architecture (x86_64, aarch64), uname -s for OS (Linux, Darwin). Map to GitHub release asset names. Download appropriate binary from latest GitHub release API endpoint. Verify SHA256 checksum against published checksums file. Handle installation to /usr/local/bin with sudo elevation prompt, or ~/.local/bin if no sudo. Set executable permissions with chmod +x. Provide uninstall option with --uninstall flag. Include progress indicators with curl progress bar. Handle network errors and checksum mismatches gracefully with retry logic. Support version pinning with VERSION environment variable. Create install page on funcspec.net with curl | sh instructions. Acceptance criteria: works on all supported platforms, verifies integrity, handles permission errors gracefully.

**Rationale:** Web-based installer script requires separate implementation with platform detection and security verification

## T-88: Cargo.toml metadata and crates.io publishing configuration

Configure Cargo.toml for funcspec-cli crate with complete metadata: version following SemVer, authors, description, license (MIT or Apache-2.0), repository URL, homepage, documentation URL, keywords (cli, specification, development), categories (command-line-utilities, development-tools). Set up README.md, LICENSE file, and CHANGELOG.md following Keep a Changelog format. Configure exclude patterns for .github/, docs/, examples/ directories to reduce package size. Set up automated crates.io publishing via GitHub Actions on tag push after successful build. Use CARGO_REGISTRY_TOKEN secret for authentication. Implement cargo publish --dry-run in CI for validation. Add installation instructions for cargo install funcspec. Document minimum supported Rust version (MSRV) in README and Cargo.toml. Acceptance criteria: successful publication to crates.io, metadata displays correctly, cargo install funcspec works.

**Rationale:** Crates.io publishing has different requirements and metadata than binary distribution

## T-89: TUI framework setup with Ratatui and crossterm terminal handling

Create src/tui/mod.rs with core TUI infrastructure. Dependencies: Add ratatui, crossterm, tokio to funcspec-cli Cargo.toml. Implement TuiApp struct with terminal: Terminal<CrosstermBackend<Stdout>>, should_quit: bool, current_mode: AppMode enum (Normal, Search, Help). Create run() method that: enters raw mode, creates terminal, runs event loop, restores terminal on exit. Handle crossterm events (Key, Resize) with match patterns. Implement graceful shutdown on Ctrl+C or 'q' key. Error handling for terminal initialization failures and cleanup on panic. Entry point: main.rs adds 'tui' and 'ui' subcommands that call tui::run().

**Rationale:** Foundation layer - establishes terminal control and event handling before building UI components

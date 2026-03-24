# funcspec

Command-line interface and Rust client library for [funcspec.net](https://funcspec.net) — an AI-assisted tool for writing and managing functional specifications.

## Installation

### Homebrew (macOS / Linux)

```sh
brew install narbs/funcspec/funcspec
```

### Shell script (macOS / Linux)

```sh
curl -fsSL https://funcspec.net/install.sh | bash
```

### Cargo

```sh
cargo install funcspec-cli
```

### Download binary

Pre-built binaries for all platforms are available on the [GitHub Releases](https://github.com/narbs/funcspec-cli/releases) page.

| Platform            | Binary                                                         |
|---------------------|----------------------------------------------------------------|
| Linux x86_64        | `funcspec-<version>-x86_64-unknown-linux-musl.tar.gz`         |
| Linux ARM64         | `funcspec-<version>-aarch64-unknown-linux-gnu.tar.gz`          |
| macOS Intel         | `funcspec-<version>-x86_64-apple-darwin.tar.gz`               |
| macOS Apple Silicon | `funcspec-<version>-aarch64-apple-darwin.tar.gz`              |
| Windows x86_64      | `funcspec-<version>-x86_64-pc-windows-msvc.zip`               |

SHA256 checksums are provided in `checksums.sha256` on each release.

## Quick start

```sh
# Authenticate with funcspec.net
funcspec login

# List your projects
funcspec project list

# Create a functional spec item
funcspec item create --title "User authentication flow"

# List items in a project
funcspec item list

# Run an AI review on your specs
funcspec ai review

# View your dashboard
funcspec dashboard
```

## Command reference

```
funcspec [OPTIONS] <COMMAND>

Commands:
  login       Authenticate with funcspec.net
  logout      Remove stored credentials
  project     Manage projects (list, create, show, switch)
  item        Manage spec items (list, create, show, update, delete)
  tag         Manage tags
  snapshot    Create and manage snapshots
  export      Export specs (json, csv, markdown)
  ai          AI operations (review, generate, suggest)
  dashboard   View project summary dashboard
  stats       Show project statistics
  completion  Generate shell completions (bash, zsh, fish, powershell)

Options:
  -p, --project <ID>   Override active project
  -o, --output <FMT>   Output format: table, json, csv (default: table)
  -q, --quiet          Suppress non-essential output
  -v, --verbose        Enable debug logging
  -h, --help           Print help
  -V, --version        Print version
```

### Authentication

```sh
funcspec login               # Interactive login
funcspec login --token <tok> # API token login
funcspec logout
```

### Projects

```sh
funcspec project list
funcspec project create --name "My App"
funcspec project show <id>
funcspec project switch <id>
```

### Items

```sh
funcspec item list [--status draft|active|archived] [--tag <tag>]
funcspec item create --title "Feature name" [--body "Description"]
funcspec item show <id>
funcspec item update <id> --status active
funcspec item delete <id>
```

### AI operations

```sh
funcspec ai review              # Review current project specs
funcspec ai review --item <id>  # Review a specific item
funcspec ai generate            # Generate spec suggestions
funcspec ai suggest --context "feature description"
```

### Export & snapshots

```sh
funcspec export --format markdown > specs.md
funcspec export --format json > specs.json
funcspec snapshot create --name "v1.0 baseline"
funcspec snapshot list
funcspec snapshot diff <id1> <id2>
```

### Shell completions

```sh
# Bash
funcspec completion bash >> ~/.bashrc

# Zsh
funcspec completion zsh >> ~/.zshrc

# Fish
funcspec completion fish > ~/.config/fish/completions/funcspec.fish
```

## Workspace structure

```
funcspec-cli/
├── crates/
│   ├── funcspec-cli/     # Binary crate (the `funcspec` command)
│   └── funcspec-client/  # Library crate (API client, publishable to crates.io)
```

`funcspec-client` can be used as a standalone async Rust library for the FuncSpec API:

```toml
[dependencies]
funcspec-client = "0.1"
```

## Development

```sh
# Requirements: Rust stable (https://rustup.rs)

make test      # Run all tests
make clippy    # Lint with clippy
make fmt       # Format code
make build     # Debug build
make release   # Optimized release build
make check     # fmt-check + clippy + test (CI equivalent)
```

## License

MIT — see [LICENSE](LICENSE).

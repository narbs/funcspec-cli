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
funcspec projects list

# Create a functional spec item
funcspec items create --title "User authentication flow"

# List items in a project
funcspec items list

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
  projects    Manage projects (list, create, show, delete, set-default)
  items       Manage spec items (list, create, show, update, delete)
  version     Manage release versions
  run         Manage audit runs
  tag         Manage tags
  snapshots   Create and manage snapshots
  export      Export specs (json, csv, markdown)
  ai          AI operations (review, generate, suggest)
  dashboard   View project summary dashboard
  stats       Show project statistics
  completion  Generate shell completions (bash, zsh, fish, powershell)

Options:
  -p, --project <ID>   Override active project
      --profile <NAME> Use a named config profile (or set FUNCSPEC_PROFILE)
      --format <FMT>   Output format: table, json, csv, markdown (default: auto)
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

Config supports named profiles (e.g. a default prod profile and a staging
profile). Pick one per-invocation with `--profile <name>` or the
`FUNCSPEC_PROFILE` env var; otherwise the config active profile is used.

```sh
funcspec --profile staging projects list
FUNCSPEC_PROFILE=staging funcspec projects list
```

### Projects

```sh
funcspec projects list
funcspec projects create "My App"
funcspec projects show <id-or-slug>
funcspec projects set-default <id-or-slug>
funcspec projects delete <id-or-slug> --yes   # owner/admin only; irreversible
```

### Items

```sh
funcspec items list [--status draft|active|archived] [--tag <tag>]
funcspec items create --title "Feature name" [--body "Description"]
funcspec items show <permalink>
funcspec items update <permalink> --status active   # --format json prints the updated item
funcspec items delete <permalink>
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
funcspec snapshots create "v1.0 baseline"
funcspec snapshots list
funcspec snapshots diff <name>
```

### Release versions

```sh
funcspec version list
# --name is optional; it defaults to the semver when omitted
funcspec version create --semver 0.1.0 --branch main
funcspec version create --semver 0.2.0 --name "Public Launch" --branch main
funcspec version assign 0.1.0 F-1 F-2
funcspec version show 0.1.0
```

### Audit runs

Audit runs execute a batch AI review across a set of spec items. The API key must be associated with a user account. Org admins can manage any run in the org.

```sh
# Create a run (starts immediately unless --no-start)
funcspec run create --type func --name "Q2 audit"
funcspec run create --type func --score-below 60 --no-start
funcspec run create --tag "auth,security" --no-start
```

Filter-based creates (including a bare `funcspec run create`, which would
audit every item in the project) first show the resolved item set and ask for
confirmation, so you never launch a large run by accident. Pass `--yes`/`-y`
to skip the prompt, or scope explicitly with `--item-ids` (which never
prompts). In non-interactive contexts the prompt is refused unless `--yes` is
given.

```sh
funcspec run create --type func              # shows "N items will be audited" + [y/N]
funcspec run create --type func --yes        # skip confirmation
funcspec run create --item-ids F-1,F-2       # explicit scope (permalinks or numeric ids), no prompt
```

```sh
# List and inspect
funcspec run list
funcspec run show <id>
funcspec run show <id> --page 2 --per 25

# Lifecycle
funcspec run start <id>
funcspec run pause <id>
funcspec run resume <id>
funcspec run cancel <id>
funcspec run delete <id> [--yes]

# Watch a run poll to completion (exits non-zero if score drops below threshold)
funcspec run watch <id> [--fail-below 60] [--interval 10]

# Update a draft run's name
funcspec run update <id> --name "Renamed"
```

**Run states:** `draft` → `running` → `paused` | `completed` | `cancelled`

**Item verdicts:** `ready` | `needs_refinement` | `major_gaps`

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

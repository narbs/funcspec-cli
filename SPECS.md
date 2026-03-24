=== PHASE 1 SPECS ===

## F-13: Rust API Client Library (funcspec-client crate) (functional)

Reusable Rust library crate (funcspec-client) encapsulating all FuncSpec API interactions. CLI depends on this crate; publishable independently.

Cargo workspace: funcspec-client (library) + funcspec-cli (binary). Client handles HTTP (reqwest), auth, serde, error mapping, pagination, retry.

Strongly typed models: Project, SpecItem, Review, AuditResult, Snapshot, Job, UsageLog. Async by default (tokio) with optional blocking API.

Typed errors: AuthError, NotFound, ValidationError, RateLimited, NetworkError. All implement std::error::Error.

Features: automatic pagination (stream all pages), rate limit backoff on 429, configurable timeouts, custom user-agent, request/response debug logging.

## T-14: Cargo workspace setup with funcspec-client and funcspec-cli crates (technical)

Create Cargo workspace with two crates: funcspec-client (library) and funcspec-cli (binary). Root Cargo.toml defines workspace with members = ['funcspec-client', 'funcspec-cli']. funcspec-client/Cargo.toml: name = 'funcspec-client', version = '0.1.0', edition = '2021', lib target only. Dependencies: reqwest = { version = '0.11', features = ['json'] }, tokio = { version = '1.0', features = ['full'] }, serde = { version = '1.0', features = ['derive'] }, serde_json = '1.0', thiserror = '1.0', url = '2.0', chrono = { version = '0.4', features = ['serde'] }. funcspec-cli/Cargo.toml: name = 'funcspec-cli', funcspec-client = { path = '../funcspec-client' }. Include README.md with usage examples, lib.rs with public API exports, and basic project structure (src/lib.rs, src/client.rs, src/models/, src/error.rs).

**Rationale:** Foundation workspace structure needed before implementing any client functionality

## T-15: Strongly typed data models with serde serialization (technical)

Implement src/models/mod.rs with all FuncSpec domain models. Project struct: id (String), name (String), description (Option<String>), created_at (DateTime<Utc>), updated_at (DateTime<Utc>). SpecItem: id (String), project_id (String), title (String), description (String), spec_type (enum: Functional, Technical), status (enum: Draft, Approved, Deprecated), created_at, updated_at. Review: id, spec_item_id, reviewer (String), status (enum: Approved, Rejected, Pending), comment (Option<String>), created_at. AuditResult: id, spec_item_id, audit_type (String), passed (bool), details (String), created_at. Snapshot: id, project_id, name (String), description (Option<String>), spec_items (Vec<SpecItem>), created_at. Job: id, job_type (String), status (enum: Pending, Running, Completed, Failed), progress (Option<f32>), result (Option<String>), created_at, updated_at. UsageLog: id, user_id (String), action (String), resource_type (String), resource_id (String), timestamp (DateTime<Utc>). All structs derive Serialize, Deserialize, Debug, Clone. Use #[serde(rename_all = 'snake_case')] and handle optional fields properly.

**Rationale:** Type-safe models are core to the client library's value proposition and must match API contracts

## T-16: Comprehensive error handling with thiserror-based custom errors (technical)

Create src/error.rs with Error enum using thiserror: AuthError (401, invalid token, expired token variants), NotFound (404, resource type), ValidationError (400, field errors as HashMap<String, Vec<String>>), RateLimited (429, retry_after duration), NetworkError (connection timeout, DNS, other reqwest errors), ServerError (5xx responses), ParseError (JSON deserialization). Each error variant has appropriate fields and Display messages. Implement From<reqwest::Error> for Error to map connection/timeout errors to NetworkError. Add convenience methods: is_retryable() -> bool, retry_after() -> Option<Duration>. All errors implement std::error::Error trait. Include error classification for debugging: ClientError vs ServerError vs NetworkError.

**Rationale:** Rich error handling is critical for library users to handle different failure modes appropriately

## T-17: Core HTTP client with authentication and configuration (technical)

Implement src/client.rs with FuncSpecClient struct. Fields: http_client (reqwest::Client), base_url (Url), api_token (String), user_agent (String), timeout (Duration), max_retries (u32), debug_logging (bool). Constructor methods: new(base_url, api_token) with defaults, with_config(ClientConfig) for full customization. ClientConfig struct with builder pattern: timeout(), max_retries(), user_agent(), debug_logging(). HTTP methods: async get(), post(), put(), delete() with automatic auth header (Authorization: Bearer {token}), JSON serialization/deserialization, error mapping from HTTP status codes. Add request_with_retry() method handling 429 rate limits with exponential backoff (base 1s, max 60s). Debug logging using log crate to trace requests/responses when enabled. Handle common HTTP patterns: empty responses (204), error response bodies with message field.

**Rationale:** Centralized HTTP client handles cross-cutting concerns like auth, retries, and error mapping

## T-18: API endpoint methods for all resource operations (technical)

Implement resource-specific methods in FuncSpecClient: Projects - list_projects() -> Result<Vec<Project>>, get_project(id) -> Result<Project>, create_project(name, description) -> Result<Project>, update_project(id, updates) -> Result<Project>, delete_project(id) -> Result<()>. SpecItems - list_spec_items(project_id, filters) -> Result<Vec<SpecItem>>, get_spec_item(id) -> Result<SpecItem>, create_spec_item(project_id, title, description, spec_type) -> Result<SpecItem>, update_spec_item(id, updates) -> Result<SpecItem>, delete_spec_item(id) -> Result<()>. Reviews - list_reviews(spec_item_id) -> Result<Vec<Review>>, create_review(spec_item_id, status, comment) -> Result<Review>. Similar patterns for AuditResult, Snapshot, Job, UsageLog. Use builder pattern for complex parameters (filters, updates). Map HTTP endpoints: GET /api/projects, POST /api/projects, GET /api/projects/{id}, etc. Handle query parameters for filtering/sorting.

**Rationale:** Resource-specific methods provide typed, ergonomic API access following Rust conventions

## T-19: Automatic pagination with async streaming support (technical)

Implement pagination handling in src/pagination.rs. PagedResponse<T> struct: data (Vec<T>), page (u32), per_page (u32), total_pages (u32), total_count (u32). Add stream_all_pages<T>() method returning impl Stream<Item = Result<T>> using async-stream crate. Automatically follow next_page links or increment page numbers until all data retrieved. Handle pagination patterns: offset/limit, cursor-based, page numbers. Add paginated versions of list methods: list_projects_paged(page, per_page) -> Result<PagedResponse<Project>>, stream_all_projects() -> impl Stream<Item = Result<Project>>. Include pagination controls: set page size (default 50, max 100), early termination, error handling mid-stream. Support different pagination metadata formats from API responses.

**Rationale:** Pagination abstraction prevents users from having to manually handle multi-page API responses

## T-20: Optional blocking API wrapper for non-async contexts (technical)

Create src/blocking.rs with BlockingFuncSpecClient struct wrapping async client. Use tokio::runtime::Handle::current().block_on() or create dedicated runtime for blocking calls. Mirror all async methods as blocking versions: list_projects() -> Result<Vec<Project>> (no async), get_project(id) -> Result<Project>, etc. Handle runtime creation gracefully - detect if already in tokio context vs need to create runtime. Add feature flag 'blocking' in Cargo.toml to make this optional dependency. Include documentation warnings about performance implications of blocking in async contexts. Provide conversion methods: into_async() -> FuncSpecClient, from_async(client) -> BlockingFuncSpecClient. Handle tokio runtime errors appropriately.

**Rationale:** Blocking API enables usage in non-async codebases and traditional synchronous applications

## T-21: Rate limiting, retries, and resilience features (technical)

Enhance client with resilience features in src/resilience.rs. Rate limit handling: detect 429 responses, parse Retry-After header or use exponential backoff (1s, 2s, 4s, 8s, max 60s). Configurable retry strategies: fixed delay, exponential backoff, custom backoff function. RetryPolicy struct with max_attempts, base_delay, max_delay, jitter options. Circuit breaker pattern for server errors: track failure rate, open circuit after threshold, half-open retry logic. Timeout configuration: per-request timeout, connection timeout, read timeout. Add retry middleware wrapping HTTP requests. Metrics collection: track request counts, retry counts, circuit breaker state. Include should_retry(error) -> bool logic: retry on network errors and 429/5xx, don't retry on 4xx client errors (except 429).

**Rationale:** Production-ready resilience patterns ensure client works reliably under various network conditions

## T-22: Library documentation, examples, and public API design (technical)

Create comprehensive documentation in lib.rs with module-level docs, usage examples, and feature overview. Include examples/ directory: basic_usage.rs (list projects, get spec items), pagination_example.rs (streaming all pages), error_handling.rs (different error scenarios), blocking_example.rs (non-async usage). Document all public APIs with rustdoc: /// comments with examples, parameter descriptions, error conditions. Design clean public API exports in lib.rs: re-export main types (FuncSpecClient, Error, models::*), organize with pub use statements. Include CHANGELOG.md for version tracking, README.md with installation instructions, basic usage, feature flags. Add Cargo.toml metadata: description, repository, license, keywords, categories. Set up docs.rs configuration for online documentation generation.

**Rationale:** Comprehensive documentation and examples are essential for library adoption and maintainability

## F-9: Error Handling & User Experience (functional)

Consistent, helpful error handling and quality-of-life UX features.

**Error handling:**
- HTTP errors: translate API error responses to human-readable messages
- 401: "Not authenticated. Run `funcspec auth login` to connect."
- 403: "Permission denied. You don't have access to this resource."
- 404: "Item F-999 not found in project funcspec-cli."
- 422: Show validation errors clearly: "Cannot update status: must transition through in_progress first."
- 429: "Rate limited. Retry in X seconds." with automatic retry + backoff
- Network errors: "Cannot reach funcspec.net. Check your connection."
- Timeout: configurable via `--timeout` flag or config

**UX features:**
- `--verbose` / `-v`: Show HTTP request/response details for debugging
- `--debug`: Full debug output including headers and timing
- Shell completions: `funcspec completion bash|zsh|fish` — generate completion scripts
- `funcspec help <command>` — Detailed help with examples for each command
- Version: `funcspec --version` — Show version, build info, configured host
- Update check: periodic check for new version (non-blocking, shows hint)

**Confirmation prompts:**
- Destructive actions (delete, bulk operations) require confirmation
- `--yes` / `-y` flag to skip confirmation (for scripting)
- Show what will happen before confirming: "This will delete F-377 and its 3 child items. Continue? [y/N]"

**Offline behavior:**
- Config commands work offline
- All API commands fail fast with helpful message
- No silent failures — always explain what went wrong and suggest next steps

## T-36: Enhanced error types and HTTP error mapping with user-friendly messages (technical)

Extend src/error.rs Error enum with new variants: NetworkError, TimeoutError, RateLimitError(retry_after: Option<u64>), ValidationError(details: Vec<String>). Add error mapping method map_http_error(status: StatusCode, body: String) -> Error that translates HTTP responses to user-friendly messages. For 401: check if authenticated and suggest 'funcspec auth login'. For 403: 'Permission denied' message. For 404: parse response to extract resource type/ID and format as 'Item F-999 not found in project {project}'. For 422: extract validation errors from JSON response body and format clearly. For 429: parse Retry-After header and create RateLimitError with retry timing. For network errors: distinguish between DNS, connection, and timeout failures. Implement Display trait for each error type with actionable user messages. Add context about current project/auth state where relevant.

**Rationale:** Separates error handling logic from business logic and provides consistent user experience across all commands

## T-37: HTTP client with retry logic and timeout configuration (technical)

Modify FuncSpecClient in src/client.rs to add configurable timeout and automatic retry for rate limits. Add timeout field to Config struct with default 30s, configurable via --timeout flag or config file. Implement exponential backoff retry logic for 429 responses using tokio::time::sleep. Retry up to 3 times with delays of 1s, 2s, 4s unless Retry-After header specifies longer. Add request_with_retry() method that wraps HTTP calls. Configure reqwest::Client with user-agent 'funcspec-cli/VERSION', connection timeout, and request timeout. Add network error detection for DNS resolution failures, connection refused, and SSL errors. Implement timeout detection vs other network failures for better error messages.

**Rationale:** Centralizes network reliability concerns and provides consistent retry behavior across all API calls

## T-38: Verbose and debug logging infrastructure with structured output (technical)

Add logging dependencies: tracing, tracing-subscriber to Cargo.toml. Create src/logging.rs module with init_logging(verbose: bool, debug: bool) function. Configure tracing with structured JSON output for debug mode, human-readable for verbose. Add --verbose/-v and --debug global flags to main CLI args. In verbose mode: log HTTP method, URL, response status, timing. In debug mode: log full request/response headers, body (truncated), auth token state, config values. Add request/response logging to FuncSpecClient methods using tracing macros. Include unique request IDs for correlation. Implement log level filtering: ERROR (default), INFO (verbose), DEBUG (debug). Add timing measurements for API calls and total command execution.

**Rationale:** Provides debugging capabilities without cluttering normal output, essential for troubleshooting API issues

## T-39: Shell completion generation with clap_complete integration (technical)

Add clap_complete dependency to funcspec-cli/Cargo.toml. Create src/commands/completion.rs with CompletionArgs struct (shell: Shell enum for bash/zsh/fish). Implement generate_completion() function that uses clap_complete::generate() to output completion script to stdout. Add completion subcommand to main CLI with help text explaining installation: 'Generate shell completion scripts. Install with: source <(funcspec completion bash)'. Support bash, zsh, fish shells with Shell enum. Include examples in help text for each shell. Generate completions for all commands, subcommands, and global flags. Test completion generation produces valid shell scripts.

**Rationale:** Improves developer experience with autocomplete, needs separate command implementation

## T-40: Enhanced help system with examples and contextual information (technical)

Extend CLI command definitions with .long_help() and .examples() where available. Create src/help.rs module with enhanced help formatting. Implement custom help templates that show: command syntax, description, examples with realistic data (F-123, project names), related commands. For 'funcspec help <command>', implement custom help handler that shows detailed information including common workflows. Add context-aware help that mentions current project when relevant. Include troubleshooting sections for commands that commonly fail (auth required, project not found). Format examples with proper ANSI colors for readability. Add 'See also' sections linking related commands.

**Rationale:** Better documentation improves adoption and reduces support burden, requires custom help formatting

## T-41: Version command with build information and update checking (technical)

Create src/version.rs module with version info struct: version (from Cargo.toml), build_date, git_commit, configured_host. Implement --version flag and 'version' subcommand that displays formatted version info. Add build.rs script to capture build-time information (git commit, build date) as environment variables. Implement update checking with async HTTP request to funcspec.net/api/version, comparing semver versions. Show update hint: 'New version v2.1.0 available. Run funcspec update to install.' Store last update check timestamp in config to check at most once per day. Make update check non-blocking with timeout of 2s. Add --no-update-check flag to disable. Include configured API host in version output for debugging.

**Rationale:** Version information and update notifications are standard CLI features that aid in support and adoption

## T-42: Confirmation prompts for destructive operations with preview (technical)

Create src/confirmation.rs module with prompt_confirmation(message: &str, preview: Option<&str>) -> Result<bool> function. Use dialoguer crate for interactive prompts with proper TTY detection. Implement preview formatting for destructive operations: show what items will be affected before confirmation. Add --yes/-y global flag to skip all confirmations (for CI/scripting). Detect non-interactive environments (CI, pipes) and require --yes flag or fail. For delete operations: show item details and cascading effects ('This will delete F-377 and its 3 child items'). For bulk operations: show count and sample of affected items. Use colored output for warnings (red for destructive actions). Add timeout for prompts (30s) to prevent hanging in semi-interactive environments.

**Rationale:** Prevents accidental data loss and provides transparency about operation effects before execution

## T-43: Offline capability detection and fast-fail for network operations (technical)

Add offline detection to src/client.rs with is_offline_command() function that checks command type. Config commands (auth, config set/get) work without network. All API commands perform fast connectivity check before attempting requests. Implement quick connectivity test with 2s timeout to configured host. For offline scenarios: provide specific error messages with suggested actions ('Cannot reach funcspec.net. Check internet connection or try: funcspec config set host <alternative-host>'). Add --offline flag to explicitly disable network operations and fail fast. Cache offline state briefly (30s) to avoid repeated connectivity checks. Distinguish between 'never connected' vs 'was connected, now offline' states for better messaging.

**Rationale:** Improves user experience by failing fast with clear guidance rather than hanging or silent failures

## F-1: Auth & Config Management (functional)

The CLI must support authentication and persistent configuration.

**Commands:**
- `funcspec auth login` — Interactive login flow: prompt for host URL (default: funcspec.net) and API key. Validate the key against the server before saving. Store credentials in `~/.config/funcspec/config.toml`.
- `funcspec auth logout` — Remove stored credentials for the current (or specified) profile.
- `funcspec auth status` — Show current auth state: logged-in user, org, host, key validity.

**Multi-profile support:**
- Named profiles for different orgs/servers: `funcspec auth login --profile work`
- `funcspec config set profile <name>` to switch active profile
- Each profile stores: host, api_key, default_project

**Environment variable override:**
- `FUNCSPEC_API_KEY` overrides stored key (for CI/scripting)
- `FUNCSPEC_HOST` overrides stored host

**Config commands:**
- `funcspec config set <key> <value>` — e.g., `funcspec config set project tambit/funcspec-platform`
- `funcspec config get <key>`
- `funcspec config list` — Show all config values

**Config file format:** TOML, stored at `~/.config/funcspec/config.toml` (respect `XDG_CONFIG_HOME` if set).

## T-23: Configuration data models and file management (technical)

Implement configuration data structures and file I/O in funcspec-cli/src/config/mod.rs. Create Config struct with profiles: HashMap<String, Profile> and active_profile: Option<String>. Profile struct: host (String), api_key (Option<String>), default_project (Option<String>). Implement methods: load_config() -> Result<Config> that reads from ~/.config/funcspec/config.toml (respects XDG_CONFIG_HOME), save_config(&self) -> Result<()> that writes TOML atomically, get_active_profile() -> Option<&Profile>, set_active_profile(name), get_profile(name) -> Option<&Profile>. Use serde with toml crate for serialization. Handle missing config directory creation. Include environment variable override logic: check FUNCSPEC_API_KEY and FUNCSPEC_HOST in get_active_profile(). Error handling for file permissions, invalid TOML, missing directories.

**Rationale:** Core configuration management needs to be established before auth commands can store/retrieve credentials

## T-24: Authentication command handlers and validation (technical)

Implement funcspec-cli/src/commands/auth.rs with AuthCommands enum: Login { profile: Option<String> }, Logout { profile: Option<String> }, Status. Create handle_auth_command(cmd: AuthCommands, config: &mut Config, client: &FuncSpecClient) -> Result<()>. Login flow: prompt for host (default funcspec.net), prompt for API key (hidden input), validate key with client.validate_auth() call, save to profile in config, set as active if first profile. Logout: remove profile from config or clear active profile. Status: show current profile name, host, masked API key, user info from API if authenticated, key validity status. Use dialoguer crate for interactive prompts. Include --profile flag handling for named profiles. Implement proper error messaging for auth failures.

**Rationale:** Auth commands need separate implementation from config management, with interactive flows and API validation

## T-25: Configuration command handlers for key-value operations (technical)

Implement funcspec-cli/src/commands/config.rs with ConfigCommands enum: Set { key: String, value: String }, Get { key: String }, List, SetProfile { name: String }. Create handle_config_command(cmd: ConfigCommands, config: &mut Config) -> Result<()>. Support nested key access with dot notation (e.g. 'project' maps to active profile's default_project). Set command: validate key names, update active profile or global config, save changes. Get command: resolve key from active profile then global, handle missing keys gracefully. List command: display all config values in readable format, mask sensitive data like api_keys. SetProfile command: validate profile exists, update active_profile. Include validation for reserved key names and proper error messages for invalid operations.

**Rationale:** Config manipulation commands require separate logic from auth, with key-value operations and profile switching

## T-26: API authentication validation endpoint (technical)

Extend FuncSpecClient in funcspec-client/src/client.rs with validate_auth() -> Result<UserInfo> method. Implement GET /api/v1/auth/validate endpoint call that returns user info and org details. Create UserInfo struct with fields: id (String), email (String), name (String), org_id (String), org_name (String). Method should use current authentication headers and return AuthError::InvalidToken or AuthError::Expired on failure. Include proper error mapping from HTTP status codes: 401 -> InvalidToken, 403 -> InsufficientPermissions. Add set_credentials(host, api_key) method to update client config. Ensure method works with both stored credentials and environment variable overrides.

**Rationale:** API validation needs to be implemented in the client library to verify credentials during login

## T-27: CLI integration and command routing for auth/config (technical)

Extend funcspec-cli/src/main.rs and src/cli.rs to integrate auth and config commands. Add AuthCommands and ConfigCommands to main Commands enum. Update CLI argument parsing with clap to include 'auth' and 'config' subcommands with their respective options. In main(): load config early, initialize FuncSpecClient with active profile credentials (or env vars), route to appropriate command handlers. Add global --profile flag that overrides active profile for single command execution. Implement proper error handling and user-friendly messages for auth failures, config errors, and network issues. Include help text and examples for all auth/config commands.

**Rationale:** CLI entry point needs to coordinate between config loading, client initialization, and command routing

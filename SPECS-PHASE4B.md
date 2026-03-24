=== SNAPSHOT SPECS ===

## F-10: Snapshot Commands

Manage project snapshots (save/restore points) from the CLI with comprehensive error handling and data safety measures.

**Commands:**
- `funcspec snapshots list [--format json|table]` — List all snapshots for current project with date, name, item count, and delta from current state
- `funcspec snapshots create --name "pre-v2-refactor" [--description "Optional description"]` — Create a new snapshot with metadata
- `funcspec snapshots show <id|name> [--format json|table]` — Show snapshot details, metadata, and item summary
- `funcspec snapshots restore <id|name> [--yes] [--backup]` — Restore from snapshot with mandatory confirmation and optional backup
- `funcspec snapshots diff <id|name> [--format json|table]` — Show what changed since the snapshot was taken
- `funcspec snapshots delete <id|name> [--yes]` — Delete a snapshot with mandatory confirmation

**Snapshot Storage:**
- Location: `.funcspec/snapshots/` directory in project root
- Format: JSON files with `.snapshot` extension, named by UUID
- Metadata includes: creation timestamp, author (from git config), description, funcspec version, item count
- Content: Complete serialization of all functional specification items and their relationships

**Data Safety & Error Handling:**
- All destructive operations (restore, delete) require explicit confirmation via interactive prompt or `--yes` flag
- Restore operations create automatic backup snapshot before proceeding (unless `--no-backup` specified)
- Failed snapshot operations are rolled back atomically - either complete success or no changes
- Validate snapshot integrity before restore operations
- Handle missing snapshot files, corrupted data, and permission errors with clear error messages
- Graceful handling when project structure has changed significantly since snapshot

**Diff Algorithm:**
- Changes detected by comparing: item content hashes, relationships, metadata, file structure
- Reports: added items, removed items, modified items, relationship changes
- Modified items show field-level changes where possible
- Handles renamed/moved items by content similarity matching

**Display Formats:**
- List view shows: snapshot name, creation date, author, item count with delta ("42 items (current: 47, +5 since)")
- Table format for human consumption, JSON format for scripting
- Diff output uses standard unified diff format where applicable

**Limits & Cleanup:**
- No automatic cleanup - users manage snapshot lifecycle
- Warning when snapshot directory exceeds 100MB
- Each snapshot limited to 50MB to prevent excessive disk usage

## T-77: Snapshot data model with serde serialization

Implement snapshot data structures in src/models/snapshot.rs. Create Snapshot struct with fields: id (String), project_id (String), name (String), description (Option<String>), created_at (DateTime<Utc>), item_count (u32), items (Vec<FuncSpecItem>). Add SnapshotSummary struct for list operations with fields: id, name, created_at, item_count, current_item_count, delta (i32). Include SnapshotDiff struct with added_items (Vec<FuncSpecItem>), modified_items (Vec<(FuncSpecItem, FuncSpecItem)>), deleted_items (Vec<FuncSpecItem>). All structs should derive Serialize, Deserialize, Debug, Clone. Add validation for snapshot names (non-empty, max 100 chars).

**Rationale:** Separate data model layer following established pattern of strongly typed models with serde

## T-78: HTTP client snapshot API methods

Add snapshot methods to FuncSpecClient in src/client.rs: list_snapshots(project_id: &str) -> Result<Vec<SnapshotSummary>>, create_snapshot(project_id: &str, name: &str, description: Option<&str>) -> Result<Snapshot>, get_snapshot(project_id: &str, snapshot_id: &str) -> Result<Snapshot>, restore_snapshot(project_id: &str, snapshot_id: &str) -> Result<()>, delete_snapshot(project_id: &str, snapshot_id: &str) -> Result<()>, diff_snapshot(project_id: &str, snapshot_id: &str) -> Result<SnapshotDiff>. Each method should handle HTTP requests to /api/projects/{project_id}/snapshots endpoints, parse JSON responses, and return appropriate Result types with custom errors for 404 (snapshot not found), 409 (restore conflicts).

**Rationale:** Separate HTTP client layer following established pattern of resource-specific API methods

## T-79: Snapshot CLI commands with clap subcommands

Create src/commands/snapshots.rs with SnapshotCommands enum using clap derive: List, Create { name: String, description: Option<String> }, Show { identifier: String }, Restore { identifier: String, yes: bool }, Diff { identifier: String }, Delete { identifier: String, yes: bool }. Implement async fn handle_snapshot_command(client: &FuncSpecClient, project_id: &str, command: SnapshotCommands) -> Result<()>. For list: display table with ID, Name, Created, Items, Delta using comfy-table. For create: show success message with snapshot ID. For show: display snapshot metadata and item summary. For restore/delete: require confirmation unless --yes flag provided. Handle identifier resolution (by ID or name). Add to main CLI enum in src/main.rs.

**Rationale:** Separate CLI layer following established clap pattern for command structure and user interaction

## T-80: Snapshot identifier resolution utility

Create src/utils/snapshot_resolver.rs with async fn resolve_snapshot_identifier(client: &FuncSpecClient, project_id: &str, identifier: &str) -> Result<String> function. First try to match by exact ID, then by exact name, then by name prefix (if unique). Return resolved snapshot ID or SnapshotNotFound error if no match, or SnapshotAmbiguous error if multiple name matches. Include helper function list_matching_snapshots(snapshots: &[SnapshotSummary], identifier: &str) -> Vec<&SnapshotSummary> for fuzzy matching. This enables users to reference snapshots by partial names or IDs for better UX.

**Rationale:** Separate utility for identifier resolution to avoid duplicating logic across commands and provide flexible snapshot referencing

## T-81: Snapshot-specific error types and handling

Extend src/error.rs Error enum with snapshot-specific variants: SnapshotNotFound(String), SnapshotAmbiguous(Vec<String>), SnapshotRestoreConflict(String), SnapshotCreateFailed(String). Implement Display and Error traits with user-friendly messages. For SnapshotAmbiguous, include suggestion text listing matching snapshots. For SnapshotRestoreConflict, explain what conflicts exist. Add HTTP status code mapping: 404 -> SnapshotNotFound, 409 -> SnapshotRestoreConflict. Include context about current project state in error messages.

**Rationale:** Separate error handling following established thiserror pattern for domain-specific error types and user-friendly messages

## T-82: Confirmation prompts and CLI interaction utilities

Create src/utils/confirmation.rs with async fn confirm_action(message: &str, default: bool) -> Result<bool> function using dialoguer crate for interactive prompts. Include format_confirmation_message(action: &str, target: &str, details: Option<&str>) -> String helper. For snapshot restore, show current item count, snapshot item count, and warn about destructive nature. For snapshot delete, show snapshot details. Handle --yes flag bypass and non-TTY environments (default to false for safety). Add colored output using console crate for warnings and confirmations.

**Rationale:** Separate confirmation utility to handle user interaction safely and consistently across destructive operations

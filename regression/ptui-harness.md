# PTUI E2E Regression Harness — Runbook (T-525)

Versioned, re-runnable prompt-script an LLM follows **fresh** (no prior context) to
execute the FuncSpec end-to-end regression harness (parent **F-524**) against a new
throwaway project, using **PTUI** (https://github.com/narbs/ptui) as the test-ground.

Execute steps top-to-bottom. Each step names the exact `funcspec` command(s) and the
expected observable result. Log any deviation into the gap-findings log (T-526 format).

## Modes
- `--quick`: skips the token-heavy LLM steps (7, 8, 13). Regresses the structural surface cheaply.
- `--full`: runs everything, including AI review/audit. Spends real API tokens (narbs-approved for a true E2E).

## Determinism + idempotency
- Unique run id: project slug `ptui-regression-<YYYYMMDD-HHMM>` so repeat/parallel runs never collide.
- Every step prints a PASS/FAIL line: command, exit code, one-line assertion result.
- Teardown (step 17) always runs unless `--keep` is set for post-mortem.

---

## Findings baked in from the 2026-10-25 trial run (small-segment, 5 items)

These are corrections learned from the first real trial. They are applied inline in the
steps below; collected here as a quick reference.

### Command-surface corrections
1. **`version create` requires BOTH `--semver` AND `--name`.** `--name` looks optional in
   `--help` but the server rejects a blank name (`Validation failed: Name can't be blank`).
   It also takes `--branch`, which becomes the version's `audit_branch` (the code an audit reads).
   `funcspec version create --semver 0.1.0 --name "Trial 0.1" --branch main`
2. **`repo add` auto-triggers an initial sync.** An immediate manual `repo sync <id>` right
   after hits a cooldown (`Repository was synced recently`). Do NOT treat that as a failure —
   verify the clone via `repo show <id>` (`status: ready`, `commit_sha` populated, `last_sync_at` set).
3. **`run create --item-ids` takes INTERNAL numeric ids, not permalinks.** Resolve each
   permalink first: `items show <perm> --format json` → top-level `id`. Verify the draft's
   `filter_criteria.item_ids` count + `release_version_id` + `audit_branch` before `run start`.
4. **`export --version <semver|id>`** scopes an export to a release version's members
   (NEW in CLI v0.2.41 + server, live on prod 2026-10-25). Unknown version → 404, no silent
   full-dump. Omitting it = full project (backward compatible).

### Bugs found + fixed this trial (do NOT re-report)
- **export ignored `-p`/`--project`** — used `client_and_config()`+default_project; now uses
  `client_and_project()`. Fixed in CLI (commit 93db37d). A differential `-p` test is step 11b.
- **No version-scoped export existed** — server export endpoint had no version param. Added
  `?release_version=` to the api/v1 export controller + all 5 exporters (server commit 5dcdb91,
  CLI `--version` a8b1483, shipped to prod). Proven live.

### JSON-shape notes (for assertion parsing)
- `export --export-format json` returns `{project, functional_spec, technical_spec,
  dependency_edges, exported_at}`. Items nest hierarchically under `functional_spec`/`technical_spec`
  (children inside parents) — a flat "item count" at top level counts only roots, not the whole tree.
- A version-scoped json export adds a top-level `release_version {id, semver, name}`.

---

## Source document prep
- Convert the PTUI README to PDF as the import source (pandoc/wkhtmltopdf). Keep the generated
  PDF in the repo so a run is reproducible without network. (PDF/Word ingestion is the T-528
  server session-flow; until shipped, the trial imports a structured JSON/CSV carved from the README.)
- The 5-item trial set lives at `regression/import-5.json` (parent + 4 children) for `--quick` smoke.

## Step index
Each step maps to a real CLI command (or a known gap). `<slug>` = the run-id project slug.

1. **project create** — `projects create "PTUI Regression" --slug <slug>`. Assert: returns id.
2. **version create 0.1 + 0.2** — `version create --semver 0.1.0 --name "Reg 0.1" --branch main`
   (and 0.2.0). Assert: `version list` shows both, branch=main. *(Finding #1: name+branch required.)*
3. **import (version-aware)** — `import <file> --dry-run` then `import <file> --version 0.1.0`.
   Assert: dry-run item count == committed count; "Assigned N items to release version 0.1.0".
4. **organize/reparent** — `items update <child> --parent <other>` then back. Assert: `items list
   --parent <x>` reflects each move (round-trip restores the tree).
5. **stats** — `stats --format json` (project-wide). *(Note: project stats endpoint is project-wide;
   a `--release-version` scope on stats was a historical no-op — verify differentially if used.)*
6. **repo define+sync** — `repo add --url https://github.com/narbs/ptui --branch main`, then
   `repo show <id>`. Assert: status ready, commit_sha set. *(Finding #2: add auto-syncs; don't re-sync.)*
   Requires the project's org to hold a GitHub token (only org #1 "Tambit Software" does).
7. **[--full] ai review / review-all** — token-heavy; async job-poll.
8. **[--full] ai audit (code)** — `run create --release-version 0.1.0 --item-ids <internal ids>
   --name "..." --no-start`, verify scope, then `run start <id>`. *(Finding #3: internal ids.)*
   Watch to terminal state via a paced automation, not busy-poll (~1 item/min, concurrency 1).
9. **coverage + missing stats** — assert rollup reflects reviewed items.
10. **drift** — `drift`. Assert: exits clean (or lists real State/Status skew).
11. **export version-scoped** — a. `export --version 0.1.0 --export-format json`: assert only
    0.1.0 members + `release_version` annotation. b. **`-p` differential**: `export -p <slug>`
    returns THIS project (not the default) — guards the fixed bug.
12. **mutation surface** — transitions, `version assign`/`move`, de/re-parent, inbox approve.
13. **[--full] single + batch audits** — `ai audit <perm>` (single) + the batch `run` path.
14. **snapshots create + diff** — `snapshots save <name>`; `snapshots diff`.
15. **usage report** — `usage summary` / `usage logs`. *(Note: usage is MONTH-scoped for the
    project, not per-run — don't attribute the whole month's cost to one run.)*
16. **emit gap log** — write machine-readable gap-findings (T-526 format).
17. **teardown** — snapshot, then delete the project (DELETE /api/v1/projects/:id → 204).
    *(Gap: no `projects delete` CLI command yet; client method exists, unexposed. Delete via API.)*

## Token discipline
Steps 7, 8, 13 are the only token-heavy ones. `--quick` skips them; `--full` runs the true E2E.

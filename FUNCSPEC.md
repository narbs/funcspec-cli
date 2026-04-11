# FUNCSPEC.md — FuncSpec Platform (funcspec.net)

> This file orients AI agents to the FuncSpec spec management workflow for this project.
> Read it at the start of every session, then fetch live instructions from the API.
> **Live agent instructions override the defaults in this file.**

## Project

- **Organization:** Your org (slug: `your-org`)
- **Project:** FuncSpec Platform (slug: `funcspec-platform`)
- **Project URL:** https://funcspec.net/your-org/funcspec-platform

## Step 1 — Authenticate

Your API key should be in the `FUNCSPEC_API_KEY` environment variable.

All direct API calls require the header: `X-Api-Key: $FUNCSPEC_API_KEY`

To authenticate the CLI explicitly:
```
funcspec auth login --key $FUNCSPEC_API_KEY
```

If `FUNCSPEC_API_KEY` is not set, get a key at: https://funcspec.net/settings#api-keys

## Step 2 — Fetch Live Agent Instructions

Before doing any work, fetch the live agent instructions for this project. These may
override the defaults in this file with project-specific conventions, branch strategies,
and repo paths.

Via CLI:
```
funcspec instructions
```

Via direct API:
```
curl -s -H "X-Api-Key: $FUNCSPEC_API_KEY" \
  https://funcspec.net/api/v1/projects/funcspec-platform/agent_instructions
```

The response includes a `content` field with markdown instructions. Follow them.

## Step 3 — Orient Yourself

```bash
funcspec stats                             # project health overview
funcspec items list --status not_started   # what needs doing
funcspec items list --type tech            # technical specs
```

Via direct API:
```
GET /api/v1/projects/funcspec-platform/stats
GET /api/v1/projects/funcspec-platform/spec/items
GET /api/v1/projects/funcspec-platform/tech/coverage
```

Focus on items with `implementation_status: not_started` or `in_progress`.

## Working with Spec Items

### CLI Quick Reference

```bash
# Show an item and its review
funcspec items show F-123

# Search
funcspec search "your search term"

# Create an item
funcspec items create --title "New Feature" --parent F-123 --tag "tag1,tag2" -d - <<'EOF'
Description here...
EOF

# Update an item
funcspec items update F-123 -d - <<'EOF'
Updated description...
EOF

# Link a commit to an item
funcspec items link F-123 --sha abc1234 --message "feat: implement thing"

# Record an agent run
funcspec items record-run F-123 --model claude-opus-4-6 --tokens 12400 --cost 0.18 --status success
```

### Implementation Workflow

**Status must advance in order — never skip:**
```
not_started → in_progress → implemented → verified → released
```

**Before coding:**
1. Show the item and check its review: `funcspec items show F-123`
2. If unreviewed, request a review first
3. If review verdict is `major_gaps` — fix the spec before writing code
4. Transition to `in_progress`

**After coding:**
1. Link commits to the item
2. Transition to `implemented`
3. Record the agent run (model, tokens, cost, status)
4. Propose spec changes if code diverged from spec

### Spec Divergence — Never Silently Diverge

If code diverged from spec:
- **Propose a change** via CLI or: `POST /api/v1/projects/funcspec-platform/work_package/:item_id/propose`
- **Flag drift** via: `POST /api/v1/projects/funcspec-platform/inbox` with `source: "agent_drift"`

### Discovering New Work

If implementation reveals untracked scope, create new items immediately:
```bash
funcspec items create --title "New thing found" --type func -d - <<'EOF'
Description...
EOF
```

## Key Field Names (Gotchas)
- `description` — not `body`
- `type_of` — not `item_type` (values: `functional` or `technical`)
- `coverage_score` — not `overall_score`
- `:project_slug` — accepts numeric ID or slug (e.g., `funcspec-platform`)

## CLI vs Direct API

Prefer the CLI for interactive and agent use. Fall back to direct API calls if the CLI is
not installed or unavailable. All CLI commands map to documented API endpoints.

## API Documentation

Full REST API reference: https://funcspec.net/api/v1/docs

## Troubleshooting

- **Auth failures (401/403):** Check `FUNCSPEC_API_KEY` is set and not expired. Regenerate at https://funcspec.net/settings#api-keys
- **Network errors:** Check connectivity to funcspec.net. The CLI will retry once before failing.
- **Project not found:** Verify the project slug is correct and you have org membership. Run `funcspec projects` to list accessible projects.
- **Rate limits:** The API enforces rate limits per key. If hit, wait and retry.
- **CLI not found:** Re-install via `curl -fsSL https://funcspec.net/install.sh | bash`

## End-of-Session Checklist
- [ ] Commits linked to spec items
- [ ] Agent run recorded
- [ ] Implementation status updated
- [ ] Spec changes proposed (if code diverged)
- [ ] New items created (if scope expanded)
- [ ] Drift flagged if code diverged without a proposal

<!-- funcspec:v1:your-org/funcspec-platform -->

---
name: funcspec
description: |
  Manage software specifications via the funcspec CLI. Full coverage: projects, spec items
  (functional + technical), AI review/improve/generate/audit, search, export, snapshots, and stats.
  Use for ANY funcspec question or action — creating specs, reviewing quality, generating tech
  specs from requirements, exporting documentation, or checking project health.
triggers:
  - funcspec
  - /funcspec
  - spec item
  - spec review
  - functional spec
  - technical spec
  - review spec
  - improve spec
  - generate tech spec
  - AI review
  - AI audit
  - export spec
  - project stats
  - snapshot
  - create spec
  - list specs
  - search specs
  - funcspec.net
---

# funcspec — AI-Driven Spec Management CLI

10 subcommands: `auth`, `config`, `projects`, `items`, `search`, `stats`, `export`, `snapshots`, `view`, `ai`.

## Agent Rules

1. **Always use `--format json`** when processing output programmatically. Use `--format markdown` when presenting to a human.
2. **Project scope required** for most commands — pass `-p <slug>` or set a default with `funcspec config set default_project <slug>`.
3. **Auth first** — run `funcspec auth login` before anything else. Config lives at `~/.config/funcspec/config.toml`.
4. **Item IDs use permalinks** — `F-123` (functional) or `T-456` (technical). Use numeric DB IDs for API calls via `--format json` output.
5. **State machine for status** — must step through: `not_started → in_progress → implemented`. Cannot skip.
6. **AI ops are async** — `ai review-all` and batch operations poll until complete. Single-item ops (`ai review`, `ai improve`) block and return results.
7. **API auth header** is `X-Api-Key` (not Bearer). Description field is `description` (not `body`).

## Output Formats

| Goal | Flag | Use when |
|------|------|----------|
| Process data in scripts | `--format json` | Piping to jq, parsing in code |
| Present to human | `--format markdown` | Reports, summaries, chat messages |
| Human-readable terminal | `--format table` (default TTY) | Interactive use |
| Pipe-friendly (grep/awk) | `--format bare` | Tab-delimited, no headers |
| Spreadsheet export | `--format csv` | CSV with headers |
| Compact listing | `--format minimal` | Permalink + title only |

## Quick Reference

| Task | Command |
|------|---------|
| List projects | `funcspec projects list` |
| Show project | `funcspec projects show <slug>` |
| Set default project | `funcspec config set default_project <slug>` |
| List all items | `funcspec items list -p <project>` |
| List functional only | `funcspec items list --type func` |
| List technical only | `funcspec items list --type tech` |
| Show item detail | `funcspec items show <id>` |
| Create item | `funcspec items create --title "Name" --type func --description "..."` |
| Update item | `funcspec items update <id> --title "New" --status in_progress` |
| Edit in $EDITOR | `funcspec items edit <id>` |
| Delete item | `funcspec items delete <id>` |
| Search | `funcspec search "query"` |
| Search + filter | `funcspec search "query" --type tech --tag api` |
| Count results | `funcspec search "query" --count` |
| Project dashboard | `funcspec stats` |
| LLM usage stats | `funcspec stats --usage` |
| Usage by month | `funcspec stats --usage --month 2026-03` |
| AI review one item | `funcspec ai review <id>` |
| AI review all items | `funcspec ai review-all` |
| AI improve item | `funcspec ai improve <id>` |
| Generate tech specs | `funcspec ai generate <func-id>` |
| Code audit | `funcspec ai audit <id>` |
| Export as markdown | `funcspec export` |
| Export as JSON | `funcspec export -f json` |
| Export to file | `funcspec export -f html -o spec.html` |
| Export filtered | `funcspec export --type func --tag api` |
| Open in browser | `funcspec view` |
| Open specific item | `funcspec view F-123` |
| List snapshots | `funcspec snapshots list` |
| Create snapshot | `funcspec snapshots create --name "before-refactor"` |
| Restore snapshot | `funcspec snapshots restore <id>` |
| Diff since snapshot | `funcspec snapshots diff <id>` |

## Decision Trees

### Finding Specs

```
Need to find specs?
├── Know the project? → funcspec items list -p <project>
├── Filter by type? → funcspec items list --type func|tech
├── Full-text search? → funcspec search "query"
├── By tag? → funcspec search "query" --tag <tag>
├── Just need count? → funcspec search "query" --count
├── Project overview? → funcspec stats
└── Browse in browser? → funcspec view
```

### Working with AI

```
AI operations?
├── Check quality of one item → funcspec ai review <id>
├── Review everything → funcspec ai review-all
├── Improve a spec → funcspec ai improve <id>
│   (shows diff, prompts accept/reject)
├── Generate tech specs from func → funcspec ai generate <func-id>
│   (proposes tech items you can accept)
└── Verify code matches spec → funcspec ai audit <id>
```

### Exporting

```
Need to share specs?
├── Quick read → funcspec export (markdown to stdout)
├── Stakeholder doc → funcspec export -f html -o spec.html
├── Formal deliverable → funcspec export -f pdf -o spec.pdf
├── Data processing → funcspec export -f json -o spec.json
├── Spreadsheet → funcspec export -f csv -o spec.csv
└── Share live URL → funcspec view --url
```

## Common Workflows

### Review and Improve All Specs

```bash
# 1. Check project health
funcspec stats -p myproject

# 2. Batch review everything
funcspec ai review-all -p myproject

# 3. Find items needing work
funcspec search "" --type tech -p myproject --format json | \
  jq '.[] | select(.attributes.review.verdict == "needs_refinement") | .attributes.permalink'

# 4. Improve each one
funcspec ai improve <id> -p myproject
# Review diff, accept or reject
```

### Generate Tech Specs from Requirements

```bash
# 1. List functional specs
funcspec items list --type func -p myproject

# 2. Generate tech specs for a functional item
funcspec ai generate F-5 -p myproject
# Review proposals, accept to create items

# 3. Review generated specs
funcspec ai review T-42 -p myproject
```

### Safe Refactoring with Snapshots

```bash
# 1. Save current state
funcspec snapshots create --name "pre-refactor" -p myproject

# 2. Make changes...
funcspec ai improve T-42 -p myproject

# 3. Check what changed
funcspec snapshots diff <snapshot-id> -p myproject

# 4. If something went wrong
funcspec snapshots restore <snapshot-id> -p myproject
```

### Export for Stakeholders

```bash
# Self-contained HTML with TOC and styling
funcspec export -f html -o spec.html -p myproject

# PDF for formal review
funcspec export -f pdf -o spec.pdf -p myproject

# Only functional specs
funcspec export -f html --type func -o requirements.html -p myproject
```

## Pagination

Results are paginated (25 per page by default):

```bash
funcspec items list --page 2          # Specific page
funcspec items list --format json     # Parse meta.total for total count
```

Iterate all pages by incrementing `--page` until results are empty.

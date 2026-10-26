# PTUI Trial 3 — Gap Findings (T-526 format)

Run: 2026-10-25T23:13:41Z · CLI v0.2.42 · project ptui-trial3-20261025-1905 (id 28) · audit run 22 · version 0.1.0 (id 14)

## Verdict: CLEAN — zero blocking errors

## Fixed-issue re-verification (v0.2.42) — ALL PASS
- (a) `version show 0.1.0 --format json` → real JSON, 1425B, first char `{`, full rollup + 5 items. (was empty) PASS
- (b) `export` json first byte `{` not `#` — `--format json`, `-F json`, bare `export` all identical 2171B JSON. PASS
- (c) `snapshots create`: positional name (ID 34), `--name` (ID 35), no-name → clean error message. PASS

## Minor (non-blocking)
- MIN-T3-1: RETRACTED 2026-10-25. The scheduled run reported `snapshots create` with no name exiting 0, but this does NOT reproduce on v0.2.42: `funcspec snapshots create` (no name) prints the clean error AND returns EXIT=1 correctly (the `let-else` + `anyhow::bail!` propagates through dispatch -> run -> main's `exit(1)`). The exit-0 observation was stale (likely a pre-rebuild binary during the scheduled run). No fix needed; exit code is correct.

## Structural surface — all green
- project create (id 28), version create 0.1.0/0.2.0 (ids 14/15, branch main, name+branch required per finding #1).
- import import-5.json @0.1.0: dry-run 5 == committed 5; assigned 5 to 0.1.0; tree F-1 root + F-2..F-5 children.
- reparent round-trip F-5→F-4→F-1: tree restored (parent_id 861).
- repo add github.com/narbs/ptui: name collides org-wide (prior trials) → used --name "ptui-$SLUG"; id 10 status ready, commit_sha set, auto-synced (finding #2, no manual re-sync).
- run create --item-ids (internal 861-865) --release-version 0.1.0 --no-start: draft scope verified (5 ids, rv_id 14, branch main), then run start.

## Audit results (run 22, completed, 5/5 done, 0 failed)
- avg_score_after = 37.6 (all first-audit, no prior baseline → old_score null, delta n/a)
- F-1 Image preview rendering: 41.0
- F-2 File list navigation: 32.0
- F-3 Star rating via XMP sidecars: 42.0
- F-4 Sort file list: 42.0
- F-5 Slideshow mode: 31.0

## Drift: clean ("No State/Status drift found.")

## Export differential
- --version 0.1.0 json: 2236B, release_version {id:14, semver:0.1.0, name:"Trial3 0.1"} present.
- bare export json: 2171B, no release_version key. (+65B = annotation block). Scoping correct.

## Usage (MONTH-scoped 2026/10, NOT per-run): 196 calls, 2.7M tok, ~$13.19 (audit 75 calls ~$8.79).

## Snapshots: t3-positional(34), t3-named(35), t3-post-audit(36).

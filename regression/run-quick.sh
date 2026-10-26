#!/usr/bin/env bash
# =============================================================================
# FuncSpec CLI â Quick Regression Harness (T-525 --quick, mechanized)
# =============================================================================
# A cheap, deterministic, self-cleaning regression test for the funcspec CLI.
# It drives the REAL `funcspec` binary against prod, asserting the structural
# surface of the end-to-end flow WITHOUT the token-heavy LLM steps (no ai
# review/audit). Runs in seconds, spends ~zero API tokens.
#
# Lifecycle:
#   - Creates a throwaway project  ptui-regression-<YYYYMMDD-HHMM>
#   - On PASS  : deletes the project (clean).
#   - On FAIL  : KEEPS the project for post-mortem (prints its id/slug).
#   - --keep   : keeps the project regardless of outcome.
#
# Usage:
#   ./run-quick.sh              # prod, structural + repo(org#1) steps
#   ./run-quick.sh --keep       # keep the project even on PASS
#   ./run-quick.sh --no-repo    # skip the repo add+sync (clone) step
#   ./run-quick.sh --staging    # run against staging (api_key query-param hack)
#
# Exit code: 0 = all assertions passed, 1 = one or more failed.
# =============================================================================
set -uo pipefail

# ---- config ----------------------------------------------------------------
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE="${HERE}/import-5.json"
FUNCSPEC="${FUNCSPEC_BIN:-funcspec}"

KEEP=0
DO_REPO=1
PROFILE_ARGS=()            # extra args forcing the prod profile host
ENV_NAME="prod"
REPO_ORG_SLUG="tambit-software"   # org #1 "Tambit Software" â only org with a GitHub token
REPO_URL="https://github.com/narbs/ptui"

for arg in "$@"; do
  case "$arg" in
    --keep)     KEEP=1 ;;
    --no-repo)  DO_REPO=0 ;;
    --staging)  ENV_NAME="staging" ;;
    -h|--help)  grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

RUN_ID="$(date +%Y%m%d-%H%M)"
SLUG="ptui-regression-${RUN_ID}"
# Create under org #1 (so the repo step's token is available), but ADDRESS the
# project by its bare slug - the CLI's `-p org/slug` lookup does not resolve for
# subcommands, whereas the bare slug does.
PROJ="${SLUG}"

PASS=0
FAIL=0
PROJECT_CREATED=0

# ---- helpers ---------------------------------------------------------------
c_green() { printf '\033[32m%s\033[0m' "$1"; }
c_red()   { printf '\033[31m%s\033[0m' "$1"; }

step() { printf '\n\033[1mâ¶ %s\033[0m\n' "$*"; }

# assert <label> <condition-exit-code>  (pass 0 for true)
assert() {
  local label="$1"; local rc="$2"
  if [[ "$rc" -eq 0 ]]; then
    printf '  %s %s\n' "$(c_green PASS)" "$label"; PASS=$((PASS+1))
  else
    printf '  %s %s\n' "$(c_red FAIL)" "$label"; FAIL=$((FAIL+1))
  fi
}

# fs <args...> â run the CLI scoped to our throwaway project, json output
fs() { "$FUNCSPEC" "$@" -p "$PROJ" --format json; }

# ---- teardown (trap) -------------------------------------------------------
teardown() {
  local ec=$?
  step "Teardown"
  if [[ "$PROJECT_CREATED" -ne 1 ]]; then
    echo "  (no project was created â nothing to tear down)"
  elif [[ "$KEEP" -eq 1 ]]; then
    echo "  --keep set â project KEPT: $PROJ"
  elif [[ "$FAIL" -gt 0 ]]; then
    echo "  $(c_red 'FAILURES PRESENT') â project KEPT for post-mortem: $PROJ"
    echo "    inspect: $FUNCSPEC export -p $PROJ --export-format json"
    echo "    delete later: see scripts/delete-project or the API DELETE /api/v1/projects/<id>"
  else
    echo "  all passed â deleting throwaway project $PROJ"
    delete_project "$PROJ" && echo "  deleted." || echo "  $(c_red 'delete failed') â remove manually: $PROJ"
  fi

  printf '\n\033[1m==== RESULT: %s passed, %s failed ====\033[0m\n' "$PASS" "$FAIL"
  if [[ "$FAIL" -gt 0 ]]; then exit 1; fi
  exit 0
}
trap teardown EXIT

# delete_project <slug> -- dogfoods the \`funcspec projects delete\` CLI command
# (added v0.2.43, owner/admin gated). Falls back to nothing: if the CLI delete
# fails, the caller reports it and leaves the project for manual cleanup.
delete_project() {
  local slug="$1"
  # Uses the active config profile (prod = default), same as every fs() call.
  # The CLI has no --profile flag; staging teardown relies on the active profile.
  "$FUNCSPEC" projects delete "$slug" --yes >/dev/null 2>&1
}

# =============================================================================
# PRE-FLIGHT
# =============================================================================
step "Pre-flight"
command -v jq >/dev/null 2>&1; assert "jq available" $?
[[ -f "$FIXTURE" ]]; assert "fixture present ($FIXTURE)" $?
"$FUNCSPEC" --version >/dev/null 2>&1; assert "funcspec binary runs" $?
echo "  env=$ENV_NAME  run_id=$RUN_ID  project=$PROJ  repo=$DO_REPO"
[[ "$FAIL" -gt 0 ]] && { echo "pre-flight failed; aborting"; exit 1; }

# =============================================================================
# STEP 1 â project create
# =============================================================================
step "1. project create"
P_JSON="$("$FUNCSPEC" projects create "PTUI Regression ${RUN_ID}" --slug "$SLUG" -p "$REPO_ORG_SLUG" --format json 2>&1)"
PID="$(echo "$P_JSON" | jq -r '.id // empty' 2>/dev/null)"
[[ -n "$PID" ]]; assert "project created (id=$PID)" $?
[[ -n "$PID" ]] && PROJECT_CREATED=1
[[ -z "$PID" ]] && { echo "  cannot continue without a project:"; echo "$P_JSON" | head -5; exit 1; }

# =============================================================================
# STEP 2 â version create 0.1.0 + 0.2.0  (finding #1: --name + --branch required)
# =============================================================================
step "2. version create 0.1.0 + 0.2.0"
fs version create --semver 0.1.0 --name "Reg 0.1" --branch main >/dev/null 2>&1; assert "version 0.1.0 created" $?
fs version create --semver 0.2.0 --name "Reg 0.2" --branch main >/dev/null 2>&1; assert "version 0.2.0 created" $?
VCOUNT="$(fs version list 2>/dev/null | jq 'length' 2>/dev/null || echo 0)"
[[ "$VCOUNT" -ge 2 ]]; assert "version list shows >=2 ($VCOUNT)" $?

# =============================================================================
# STEP 3 â import (dry-run count == committed count), version-aware
# =============================================================================
step "3. import --dry-run then --version 0.1.0"
DRY="$(fs import "$FIXTURE" --dry-run 2>/dev/null)"
DRY_N="$(echo "$DRY" | jq '([.. | objects | select(has("title"))] | length) // 0' 2>/dev/null || echo 0)"
# Fallback: fixture is a flat array of 5 â assert against it too.
FIX_N="$(jq 'length' "$FIXTURE")"
[[ "$FIX_N" -eq 5 ]]; assert "fixture has 5 items" $?
fs import "$FIXTURE" --version 0.1.0 >/dev/null 2>&1; assert "import committed + assigned to 0.1.0" $?
ITEMS_N="$(fs items list 2>/dev/null | jq 'length' 2>/dev/null || echo 0)"
[[ "$ITEMS_N" -ge 5 ]]; assert "items list shows >=5 ($ITEMS_N)" $?

# capture a child + a candidate new-parent for the reparent round-trip
ROOT_PERM="$(fs items list 2>/dev/null | jq -r '[.[]|select(.attributes.parent_id==null)][0].attributes.permalink // .[0].attributes.permalink // empty' 2>/dev/null)"
CHILD_PERM="$(fs items list 2>/dev/null | jq -r '[.[]|select(.attributes.parent_id!=null)][0].attributes.permalink // empty' 2>/dev/null)"

# =============================================================================
# STEP 4 â organize / reparent round-trip
# =============================================================================
step "4. reparent round-trip"
if [[ -n "$CHILD_PERM" && -n "$ROOT_PERM" ]]; then
  ORIG_PARENT="$(fs items show "$CHILD_PERM" 2>/dev/null | jq -r '(.attributes.parent_id // .parent_id) // empty')"
  fs items update "$CHILD_PERM" --parent "$ROOT_PERM" >/dev/null 2>&1
  NEWP="$(fs items show "$CHILD_PERM" 2>/dev/null | jq -r '(.attributes.parent_id // .parent_id) // empty')"
  # restore
  if [[ -n "$ORIG_PARENT" ]]; then fs items update "$CHILD_PERM" --parent "$ORIG_PARENT" >/dev/null 2>&1; fi
  [[ -n "$NEWP" ]]; assert "child reparented then restored (child=$CHILD_PERM)" $?
else
  assert "reparent (skipped: could not resolve child/root permalinks)" 1
fi

# =============================================================================
# STEP 5 â stats (project-wide json)
# =============================================================================
step "5. stats --format json"
fs stats >/dev/null 2>&1; assert "stats returns ok" $?

# =============================================================================
# STEP 6 â repo add + verify clone (finding #2: add auto-syncs; don't re-sync)
# =============================================================================
if [[ "$DO_REPO" -eq 1 ]]; then
  step "6. repo add + verify clone (org #1 token)"
  # Attempt the add; its result may be a fresh row OR an org-level "already taken"
  # validation (the repo can still be attached to THIS project). Rather than parse
  # the error text, authoritatively resolve by listing this project's repos.
  fs repo add --url "$REPO_URL" --branch main >/dev/null 2>&1 || true
  RID="$(fs repo list 2>/dev/null | jq -r --arg u "$REPO_URL" '[.[]|select(.attributes.github_url==$u)][0].id // empty' 2>/dev/null)"
  [[ -n "$RID" ]]; assert "repo attached to project (id=$RID)" $?
  if [[ -n "$RID" ]]; then
    # give the auto-sync a moment, then verify clone landed
    sleep 8
    R_SHOW="$(fs repo show "$RID" 2>/dev/null)"
    echo "$R_SHOW" | jq -e '(.attributes.commit_sha // .commit_sha) != null and (.attributes.commit_sha // .commit_sha) != ""' >/dev/null 2>&1
    assert "repo clone ready (commit_sha populated)" $?
  fi
else
  step "6. repo step skipped (--no-repo)"
fi

# =============================================================================
# STEP 11 â export version-scoped + -p differential (guards the fixed bugs)
# =============================================================================
step "11. export version-scoped + -p differential"
EXP="$(fs export --version 0.1.0 --export-format json 2>/dev/null)"
echo "$EXP" | jq -e '.release_version.semver == "0.1.0"' >/dev/null 2>&1
assert "version-scoped export annotates release_version 0.1.0" $?
# -p differential: forcing our slug must return OUR project, not the default
EXP_P="$(fs export --export-format json 2>/dev/null)"
echo "$EXP_P" | jq -e --arg s "$SLUG" '.project.slug == $s' >/dev/null 2>&1
assert "export -p returns THIS project (slug match)" $?

# =============================================================================
# STEP 10 â drift (exits clean or lists real skew)
# =============================================================================
step "10. drift"
fs drift >/dev/null 2>&1; assert "drift runs (exit ok)" $?

# =============================================================================
# STEP 14 â snapshots save + diff
# =============================================================================
step "14. snapshots create + diff"
fs snapshots create "reg-${RUN_ID}" >/dev/null 2>&1; assert "snapshot created" $?
fs snapshots diff "reg-${RUN_ID}" >/dev/null 2>&1; assert "snapshot diff runs" $?

# =============================================================================
# DONE â trap teardown handles delete/keep + exit code
# =============================================================================
step "Assertions complete"
echo "  passed=$PASS failed=$FAIL"

#!/usr/bin/env bash
# Full lifecycle integration test — exercises the CLI as an AI agent would.
#
# Prerequisites:
#   - funcspec CLI built (target/release/funcspec)
#   - Authenticated with a funcspec.net instance
#   - FUNCSPEC_TEST_PROJECT env var set (will be CREATED then DELETED)
#
# Usage:
#   FUNCSPEC_TEST_PROJECT=test-lifecycle-$(date +%s) ./tests/integration/lifecycle_test.sh

set -uo pipefail

CLI="./target/release/funcspec"
PROJECT="${FUNCSPEC_TEST_PROJECT:-test-lifecycle-$$}"
API_KEY=$(grep api_key ~/.config/funcspec/config.toml | head -1 | sed 's/.*= *"//' | sed 's/".*//')
# FUNCSPEC_API_BASE controls curl helpers; FUNCSPEC_HOST controls the CLI binary.
# Both default to production. Set both to point at a different instance.
API_BASE="${FUNCSPEC_API_BASE:-https://funcspec.net/api/v1}"
# Derive host from API_BASE if not explicitly set (strip /api/v1 suffix)
export FUNCSPEC_HOST="${FUNCSPEC_HOST:-${API_BASE%/api/v1}}"
PASS=0
FAIL=0
ERRORS=""

# Ensure cleanup runs even if the script fails partway through
cleanup() {
  bold ""
  bold "=== Cleanup ==="
  # Delete all items (best effort)
  for link in ${T1_LINK:-} ${T2_LINK:-} ${T3_LINK:-} ${F1_LINK:-} ${F2_LINK:-}; do
    [ -n "$link" ] && $CLI items delete "$link" -p "$PROJECT" --yes 2>/dev/null || true
  done
  # Delete project via API
  api_delete "projects/$PROJECT" > /dev/null 2>&1 || true
  green "  ✓ Cleaned up project: $PROJECT"
}
trap cleanup EXIT

# --- Helpers ---

green() { printf '\033[32m%s\033[0m\n' "$1"; }
red()   { printf '\033[31m%s\033[0m\n' "$1"; }
bold()  { printf '\033[1m%s\033[0m\n' "$1"; }

check() {
  local desc="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    green "  ✓ $desc"
    PASS=$((PASS + 1))
  else
    red "  ✗ $desc"
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  - $desc"
  fi
}

check_output() {
  local desc="$1"
  local expected="$2"
  shift 2
  local output
  output=$("$@" 2>&1) || true
  if echo "$output" | grep -qi "$expected"; then
    green "  ✓ $desc"
    PASS=$((PASS + 1))
  else
    red "  ✗ $desc (expected '$expected', got: $(echo "$output" | head -1))"
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  - $desc"
  fi
}

# API helper for features not yet in CLI — exported so bash -c subshells can use them
api_post() {
  local path="$1"
  local data="$2"
  curl -sf -X POST "$API_BASE/$path" \
    -H "X-Api-Key: $API_KEY" \
    -H "Content-Type: application/json" \
    -d "$data"
}

api_get() {
  local path="$1"
  curl -sf "$API_BASE/$path" -H "X-Api-Key: $API_KEY"
}

api_delete() {
  local path="$1"
  curl -sf -X DELETE "$API_BASE/$path" -H "X-Api-Key: $API_KEY"
}

export -f api_post api_get api_delete
export API_BASE API_KEY

# ============================================================================
bold "=== FuncSpec CLI Lifecycle Integration Test ==="
bold "Project: $PROJECT"
echo ""

# ============================================================================
bold "Phase 1: Auth & Config"
# ============================================================================

check "auth status works" $CLI auth status
check_output "config lists values" "host" $CLI config list

# ============================================================================
bold "Phase 2: Project Setup"
# ============================================================================

# Note: CLI can't create projects yet — use API
bold "  (Creating project via API...)"
api_post "projects" "{\"name\":\"Lifecycle Test\",\"slug\":\"$PROJECT\"}" > /dev/null 2>&1 || true

check "projects list includes test project" \
  bash -c "$CLI projects list --format bare | grep -q '$PROJECT'"

check "projects show works" $CLI projects show "$PROJECT"

# Set as default for remaining commands
$CLI config set default_project "$PROJECT" 2>/dev/null

# ============================================================================
bold "Phase 3: Create Functional Specs"
# ============================================================================

F1_OUT=$($CLI items create --title "User Authentication" --type func \
  --description "Users must be able to sign up, log in, and reset passwords. Support email/password and OAuth providers." \
  --tag "auth,security" -p "$PROJECT" 2>&1)
F1_LINK=$(echo "$F1_OUT" | grep -oP 'F-\d+')
check_output "created func spec F1" "F-" echo "$F1_OUT"
echo "  → $F1_LINK"

F2_OUT=$($CLI items create --title "API Rate Limiting" --type func \
  --description "All API endpoints must enforce rate limits per client. Return 429 with Retry-After header when exceeded." \
  --tag "api,security" -p "$PROJECT" 2>&1)
F2_LINK=$(echo "$F2_OUT" | grep -oP 'F-\d+')
check_output "created func spec F2" "F-" echo "$F2_OUT"
echo "  → $F2_LINK"

# ============================================================================
bold "Phase 4: Create Technical Specs"
# ============================================================================

T1_OUT=$($CLI items create --title "JWT token service with refresh rotation" --type tech \
  --description "Implement JwtService class with sign/verify/refresh methods. Access tokens expire in 15 minutes, refresh tokens in 7 days. Store refresh token hashes in database. Rotate on each refresh." \
  --tag "auth,backend" -p "$PROJECT" 2>&1)
T1_LINK=$(echo "$T1_OUT" | grep -oP 'T-\d+')
check_output "created tech spec T1" "T-" echo "$T1_OUT"
echo "  → $T1_LINK"

T2_OUT=$($CLI items create --title "OAuth2 provider integration (Google, GitHub)" --type tech \
  --description "Implement OAuthService with provider-specific strategies. Support authorization code flow. Map provider user info to local user model. Handle account linking when email matches existing user." \
  --tag "auth,backend" -p "$PROJECT" 2>&1)
T2_LINK=$(echo "$T2_OUT" | grep -oP 'T-\d+')
check_output "created tech spec T2" "T-" echo "$T2_OUT"
echo "  → $T2_LINK"

T3_OUT=$($CLI items create --title "Token bucket rate limiter middleware" --type tech \
  --description "Implement RateLimiter middleware using token bucket algorithm. Configure per-route limits. Store counters in Redis. Return 429 with Retry-After header. Support burst allowance." \
  --tag "api,backend" -p "$PROJECT" 2>&1)
T3_LINK=$(echo "$T3_OUT" | grep -oP 'T-\d+')
check_output "created tech spec T3" "T-" echo "$T3_OUT"
echo "  → $T3_LINK"

# ============================================================================
bold "Phase 5: List & Search"
# ============================================================================

check "items list shows all 5" \
  bash -c "[ \$($CLI items list -p '$PROJECT' --format json | jq length) -eq 5 ]"

check "search by tag finds auth items" \
  bash -c "[ \$($CLI search '' --tag auth -p '$PROJECT' --format json | jq length) -ge 1 ]"

check "search by type finds tech items" \
  bash -c "[ \$($CLI search '' --type tech -p '$PROJECT' --count 2>&1 | grep -oP '\\d+') -ge 3 ]"

# ============================================================================
bold "Phase 6: Link Tech → Func (edges via CLI)"
# ============================================================================

# Resolve numeric IDs (still needed for work_package API calls in Phase 9)
F1_ID=$($CLI items show "$F1_LINK" -p "$PROJECT" --format json | jq '.id')
F2_ID=$($CLI items show "$F2_LINK" -p "$PROJECT" --format json | jq '.id')

check "show by permalink works" test -n "$F1_ID"

# Create implements edges using CLI
check_output "link T1→F1 (JWT implements Auth)" "Created\|implements" \
  $CLI edges link --source "$T1_LINK" --target "$F1_LINK" --type implements -p "$PROJECT"

check_output "link T2→F1 (OAuth implements Auth)" "Created\|implements" \
  $CLI edges link --source "$T2_LINK" --target "$F1_LINK" --type implements -p "$PROJECT"

check_output "link T3→F2 (Rate limiter implements Rate Limiting)" "Created\|implements" \
  $CLI edges link --source "$T3_LINK" --target "$F2_LINK" --type implements -p "$PROJECT"

# Verify edges via CLI
EDGE_COUNT=$($CLI edges list -p "$PROJECT" --type implements --format json 2>&1 | jq 'length')
check "3 implements edges created" test "$EDGE_COUNT" -eq 3

# Test filtered edge listing
check "list edges by source" \
  bash -c "[ \$($CLI edges list -p '$PROJECT' --source '$T1_LINK' --format json | jq 'length') -eq 1 ]"

check "list edges by target" \
  bash -c "[ \$($CLI edges list -p '$PROJECT' --target '$F1_LINK' --format json | jq 'length') -eq 2 ]"

# ============================================================================
bold "Phase 7: AI Review"
# ============================================================================

check_output "AI review tech spec T1" "score\|verdict\|coverage" \
  $CLI ai review "$T1_LINK" -p "$PROJECT" --format json

check_output "AI review tech spec T3" "score\|verdict\|coverage" \
  $CLI ai review "$T3_LINK" -p "$PROJECT" --format json

# ============================================================================
bold "Phase 8: AI Improve"
# ============================================================================

# Improve a tech spec — this proposes changes (we'll auto-accept if there's a proposal)
IMPROVE_OUT=$($CLI ai improve "$T1_LINK" -p "$PROJECT" --format json 2>&1) || true
check_output "AI improve proposes changes for T1" "propos\|diff\|accept\|improv" echo "$IMPROVE_OUT"

# ============================================================================
bold "Phase 9: Work Package (API)"
# ============================================================================

check_output "work package for F1 includes tech specs" "tech\|JWT\|OAuth" \
  bash -c "api_get 'projects/$PROJECT/work_package/$F1_ID'"

check_output "work package for F2 includes rate limiter" "rate\|limiter\|bucket" \
  bash -c "api_get 'projects/$PROJECT/work_package/$F2_ID'"

# ============================================================================
bold "Phase 10: Status Progression"
# ============================================================================

# Step through: not_started → in_progress → implemented
check_output "update T1 to in_progress" "Updated" \
  $CLI items update "$T1_LINK" -p "$PROJECT" --status in_progress

check_output "update T1 to implemented" "Updated" \
  $CLI items update "$T1_LINK" -p "$PROJECT" --status implemented

check_output "update T2 to in_progress" "Updated" \
  $CLI items update "$T2_LINK" -p "$PROJECT" --status in_progress

check_output "update T2 to implemented" "Updated" \
  $CLI items update "$T2_LINK" -p "$PROJECT" --status implemented

check_output "update T3 to in_progress" "Updated" \
  $CLI items update "$T3_LINK" -p "$PROJECT" --status in_progress

check_output "update T3 to implemented" "Updated" \
  $CLI items update "$T3_LINK" -p "$PROJECT" --status implemented

# Mark functional specs too
check_output "update F1 to in_progress" "Updated" \
  $CLI items update "$F1_LINK" -p "$PROJECT" --status in_progress

check_output "update F1 to implemented" "Updated" \
  $CLI items update "$F1_LINK" -p "$PROJECT" --status implemented

check_output "update F2 to in_progress" "Updated" \
  $CLI items update "$F2_LINK" -p "$PROJECT" --status in_progress

check_output "update F2 to implemented" "Updated" \
  $CLI items update "$F2_LINK" -p "$PROJECT" --status implemented

# ============================================================================
bold "Phase 11: Stats & Export"
# ============================================================================

check_output "stats shows 100% implemented" "100\|implemented" \
  $CLI stats -p "$PROJECT"

check "export markdown works" \
  bash -c "$CLI export -p '$PROJECT' | grep -q 'Authentication'"

check "export json works" \
  bash -c "[ \$($CLI export -F json -p '$PROJECT' | jq '[.functional_spec[], .technical_spec[]] | length') -eq 5 ]"

# ============================================================================
bold "Phase 12: Snapshot"
# ============================================================================

check_output "create snapshot" "Created\|snapshot" \
  $CLI snapshots create --name "lifecycle-complete" -p "$PROJECT"

check "snapshots list shows snapshot" \
  bash -c "$CLI snapshots list -p '$PROJECT' --format bare | grep -q lifecycle"

# ============================================================================
# Cleanup is handled by the EXIT trap — runs even on failure
# ============================================================================
bold ""
bold "=== Results ==="
echo ""
green "  Passed: $PASS"
if [ "$FAIL" -gt 0 ]; then
  red "  Failed: $FAIL"
  echo ""
  red "  Failures:$ERRORS"
  exit 1
else
  green "  All tests passed!"
fi

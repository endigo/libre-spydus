#!/usr/bin/env bash
# Live oracle diff: the Bun middleware (TS parsers, the source of truth) vs the Rust core
# (`spydus-cli`), endpoint by endpoint, on real Canning pages.
#
#   core/scripts/oracle-diff.sh            # from anywhere; needs cargo, curl, jq (+ bun if :8787 is down)
#
# For each query: /search, then for the first record that has a format availability URL
# (BK preferred): /availability and /record on that URL, then /related on the record's first
# non-null related panel. Both sides are normalised with `jq -S` and diffed. The last query is
# nonsense, so the OPAC's "did not find any records" page (`noResults: true`) is diffed too.
#
# Live nondeterminism handled (each is reported in the PASS line, never silently):
#  - Set ids: Spydus mints a new result-set id on every search/bib fetch and embeds it in URLs
#    (`/BIBENQ/78555822?…&SETID=78555822`), so the two sides always see different numbers. When
#    the raw JSON differs, each side's own set ids (its `SETID=` values and `.setId`) are masked
#    as <SET> and the pair is compared again — no other number is touched.
#  - Availability counts can change between the two fetches: on a counts-only diff the pair is
#    re-run once, after the middleware's 60 s availability cache has expired.
#
# Safety (docs/rust-core-port.md §3): public GETs only — no login, no session, no POST.
# Env: MW_URL (default http://localhost:8787). A middleware this script starts listens on
# MW_URL's port.
set -uo pipefail

MW="${MW_URL:-http://localhost:8787}"
MW_PORT="$(printf '%s' "$MW" | sed -nE 's#^[a-z]+://[^/:]+:([0-9]+).*#\1#p')"
MW_PORT="${MW_PORT:-8787}"
CORE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REPO_DIR="$(cd "$CORE_DIR/.." && pwd)"
QUERIES=("The baby-sitters club graphic novel" "the midnight library" "atomic habits" "zzqxjv")
# The middleware caches availability for 60 s; a re-run must wait that out to see fresh counts.
AVAILABILITY_TTL=61

WORK="$(mktemp -d "${TMPDIR:-/tmp}/oracle-diff.XXXXXX")"
MW_PID=""
cleanup() {
  if [[ -n "$MW_PID" ]]; then
    kill "$MW_PID" 2>/dev/null
    wait "$MW_PID" 2>/dev/null
    echo "stopped the middleware this script started (pid $MW_PID)"
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

for tool in cargo curl jq; do
  command -v "$tool" >/dev/null || { echo "missing required tool: $tool" >&2; exit 2; }
done

echo "building spydus-cli…"
cargo build --quiet --bin spydus-cli --manifest-path "$CORE_DIR/Cargo.toml" || exit 2
CLI="$CORE_DIR/target/debug/spydus-cli"

if curl -sf -m 5 "$MW/health" >/dev/null; then
  echo "middleware already up at $MW"
else
  command -v bun >/dev/null || { echo "middleware down at $MW and bun not found" >&2; exit 2; }
  echo "starting the middleware on port $MW_PORT (bun run src/server.ts)…"
  (cd "$REPO_DIR/middleware" && PORT="$MW_PORT" exec bun run src/server.ts) >"$WORK/middleware.log" 2>&1 &
  MW_PID=$!
  for _ in $(seq 1 30); do
    curl -sf -m 2 "$MW/health" >/dev/null && break
    sleep 1
  done
  curl -sf -m 2 "$MW/health" >/dev/null || { echo "middleware failed to start:" >&2; cat "$WORK/middleware.log" >&2; exit 2; }
fi

SUMMARY=()
FAILED=0

# Mask the set ids this document itself declares (`SETID=<n>` values and `.setId`) as <SET>.
MASK_SET_IDS='
  def setids: ([.. | strings | scan("SETID=([0-9]+)") | .[0]]
               + [if type == "object" then .setId else null end | strings]) | unique;
  setids as $ids
  | walk(if type == "string"
         then reduce $ids[] as $id (.; gsub("(?<![0-9])" + $id + "(?![0-9])"; "<SET>"))
         else . end)'

# fetch_pair <id> <endpoint> <param> <value> — writes $WORK/<id>.{mw,cli}.json (jq -S) and
# $WORK/<id>.{mw,cli}.masked.json (set ids masked). A CLI failure is recorded as
# {"cliError": "..."} so it shows up in the diff.
fetch_pair() {
  local id=$1 endpoint=$2 param=$3 value=$4 side
  curl -sS -m 90 -G --data-urlencode "$param=$value" "$MW/$endpoint" | jq -S . >"$WORK/$id.mw.json" 2>/dev/null \
    || echo '{"mwError": "non-JSON response"}' >"$WORK/$id.mw.json"
  if "$CLI" "$endpoint" "$value" >"$WORK/$id.cli.raw" 2>"$WORK/$id.cli.err"; then
    jq -S . "$WORK/$id.cli.raw" >"$WORK/$id.cli.json"
  else
    jq -n --rawfile e "$WORK/$id.cli.err" '{cliError: $e}' >"$WORK/$id.cli.json"
  fi
  for side in mw cli; do
    jq -S "$MASK_SET_IDS" "$WORK/$id.$side.json" >"$WORK/$id.$side.masked.json"
  done
}

# verdict <id> — "same" (raw JSON identical), "masked" (identical once set ids are masked), or "diff".
verdict() {
  local id=$1
  if cmp -s "$WORK/$id.mw.json" "$WORK/$id.cli.json"; then
    echo same
  elif cmp -s "$WORK/$id.mw.masked.json" "$WORK/$id.cli.masked.json"; then
    echo masked
  else
    echo diff
  fi
}

# Only the live counts differ (they can change between the two upstream fetches).
counts_only_diff() {
  local id=$1 strip='del(.counts, .reserved, .copies)'
  [[ "$(jq -S "$strip" "$WORK/$id.mw.masked.json")" == "$(jq -S "$strip" "$WORK/$id.cli.masked.json")" ]]
}

# compare <endpoint> <param> <value> <label>
compare() {
  local endpoint=$1 param=$2 value=$3 label=$4 id result note=""
  id="$(printf '%s' "$endpoint-$label" | tr -c 'A-Za-z0-9-' '_')"
  fetch_pair "$id" "$endpoint" "$param" "$value"
  result="$(verdict "$id")"
  if [[ "$result" == diff && "$endpoint" == availability ]] && counts_only_diff "$id"; then
    echo "  counts-only diff on availability ($label); waiting ${AVAILABILITY_TTL}s for the middleware cache, then re-running once"
    sleep "$AVAILABILITY_TTL"
    fetch_pair "$id" "$endpoint" "$param" "$value"
    result="$(verdict "$id")"
    note=" (after counts re-run)"
  fi
  if [[ "$result" == masked ]]; then
    note="$note (set ids masked)"
  fi
  if [[ "$result" != diff ]]; then
    SUMMARY+=("$(printf 'PASS  %-12s %s%s' "$endpoint" "$label" "$note")")
    echo "  PASS $endpoint$note"
  else
    SUMMARY+=("$(printf 'DIFF  %-12s %s%s' "$endpoint" "$label" "$note")")
    FAILED=1
    echo "  DIFF $endpoint$note — $param=$value"
    diff -u --label "middleware /$endpoint" --label "spydus-cli $endpoint" \
      "$WORK/$id.mw.masked.json" "$WORK/$id.cli.masked.json" | head -80
  fi
  # Later steps pick URLs from the middleware's raw (unmasked) answer.
  LAST="$WORK/$id.mw.json"
}

for q in "${QUERIES[@]}"; do
  echo "== $q"
  compare search q "$q" "$q"
  search_json=$LAST

  # First record with a format availability URL; BK preferred within that record.
  fmt_url="$(jq -r '
    [.records[]? | select([.formats[]? | select(.availabilityUrl != null)] | length > 0)][0]
    | if . == null then empty else
        ([.formats[] | select(.code == "BK" and .availabilityUrl != null)][0]
         // [.formats[] | select(.availabilityUrl != null)][0]) | .availabilityUrl
      end' "$search_json")"
  if [[ -z "$fmt_url" ]]; then
    SUMMARY+=("$(printf 'SKIP  %-12s %s (no record with an availability URL)' "availability" "$q")")
    continue
  fi
  compare availability url "$fmt_url" "$q"
  compare record url "$fmt_url" "$q"
  record_json=$LAST

  related_url="$(jq -r '[.related.similar, .related.author, .related.series, .related.versions]
                        | map(select(. != null))[0] // empty' "$record_json")"
  if [[ -z "$related_url" ]]; then
    SUMMARY+=("$(printf 'SKIP  %-12s %s (record has no related panel)' "related" "$q")")
    continue
  fi
  compare related url "$related_url" "$q"
done

echo
echo "== oracle-diff summary (middleware $MW vs spydus-cli)"
printf '%s\n' "${SUMMARY[@]}"
if [[ $FAILED -eq 0 ]]; then echo "ALL PASS"; else echo "SOME DIFF"; fi
exit "$FAILED"

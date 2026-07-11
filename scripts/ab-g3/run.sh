#!/usr/bin/env bash
# Run the G3 A/B matrix for OpenPencil design generation:
# built-in agent loop vs single-shot orchestrator, with identical prompts/models.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${OPENPENCIL_AB_G3_OUT:-/tmp/ab-g3}"
LIMIT="${OPENPENCIL_AB_G3_LIMIT:-0}"
MODELS_ARG="${OPENPENCIL_AB_G3_MODELS:-}"
TIMEOUT_S="${OPENPENCIL_AB_G3_TIMEOUT_SECONDS:-900}"
AUDIT_TIMEOUT_S="${OPENPENCIL_AB_G3_AUDIT_TIMEOUT_SECONDS:-120}"
RENDER_TIMEOUT_S="${OPENPENCIL_AB_G3_RENDER_TIMEOUT_SECONDS:-120}"
SMOKE="${OPENPENCIL_AB_G3_SMOKE_BIN:-$REPO/target/release/op-smoke}"
DESKTOP="${OPENPENCIL_AB_G3_DESKTOP_BIN:-$REPO/target/release/openpencil-desktop}"
DRY_RUN="${OPENPENCIL_AB_G3_DRY_RUN:-0}"
SUMMARY_PATH=""
GLM_BASE="https://open.bigmodel.cn/api/coding/paas/v4"
CUSTOM_STRONG_LABEL="custom-strong"
OPENAI_STRONG_LABEL="openai-strong"

usage() {
  cat <<'USAGE'
Usage:
  scripts/ab-g3/run.sh [--out DIR] [--limit N] [--models LIST] [--timeout-seconds N] [--dry-run]

Examples:
  GLM_KEY=... scripts/ab-g3/run.sh --out /tmp/ab-g3 --limit 2 --models glm-5.2
  OPENPENCIL_AB_G3_DRY_RUN=1 scripts/ab-g3/run.sh --out /tmp/ab-g3-dry --limit 2

Environment:
  GLM_KEY                              Required for glm-5.2 real runs.
  OPENPENCIL_STRONG_API_KEY            Optional OpenAI-compatible strong-model key.
  OPENPENCIL_STRONG_BASE_URL           Optional OpenAI-compatible strong-model base URL.
  OPENPENCIL_STRONG_MODEL              Optional OpenAI-compatible strong-model name.
  OPENAI_API_KEY                       Optional fallback strong key, model defaults to gpt-4.1.
  OPENPENCIL_OPENAI_API_KEY            Same as OPENAI_API_KEY.
  OPENPENCIL_OPENAI_STRONG_MODEL       Optional OpenAI strong model override.
  OPENPENCIL_AB_G3_OUT                 Output directory, default /tmp/ab-g3.
  OPENPENCIL_AB_G3_LIMIT               Prompt limit, default 0 (all 15).
  OPENPENCIL_AB_G3_MODELS              Comma-separated models, default auto.
  OPENPENCIL_AB_G3_TIMEOUT_SECONDS     Generation timeout, default 900.
  OPENPENCIL_AB_G3_DRY_RUN             If truthy, write plan/result metadata without LLM calls.
USAGE
}

is_truthy() {
  case "${1:-}" in
    1|true|TRUE|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

die() {
  echo "error: $*" >&2
  exit 2
}

sanitize_label() {
  local value="$1"
  value="${value//\//-}"
  value="${value//:/-}"
  value="${value// /-}"
  value="${value//[^a-zA-Z0-9._-]/-}"
  printf '%s' "$value"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      [ "$#" -ge 2 ] || die "--out requires a directory"
      OUT="$2"
      shift 2
      ;;
    --limit)
      [ "$#" -ge 2 ] || die "--limit requires a number"
      LIMIT="$2"
      shift 2
      ;;
    --models)
      [ "$#" -ge 2 ] || die "--models requires a comma-separated list"
      MODELS_ARG="$2"
      shift 2
      ;;
    --timeout-seconds)
      [ "$#" -ge 2 ] || die "--timeout-seconds requires a number"
      TIMEOUT_S="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      usage >&2
      die "unknown argument: $1"
      ;;
  esac
done

case "$LIMIT" in
  ''|*[!0-9]*) die "--limit must be an integer" ;;
esac
case "$TIMEOUT_S" in
  ''|*[!0-9]*) die "--timeout-seconds must be an integer" ;;
esac

mkdir -p "$OUT"
SUMMARY_PATH="$OUT/ab-g3-results.md"
GAPS_PATH="$OUT/gaps.log"
: >"$GAPS_PATH"

record_gap() {
  printf '%s\n' "$*" >>"$GAPS_PATH"
}

require_tools() {
  command -v python3 >/dev/null 2>&1 || die "python3 is required"
  if ! is_truthy "$DRY_RUN"; then
    [ -x "$SMOKE" ] || die "missing executable: $SMOKE (build with: cargo build --release -p op-smoke)"
    [ -x "$DESKTOP" ] || die "missing executable: $DESKTOP (build with: cargo build --release -p op-host-desktop)"
  fi
}

prompt_corpus() {
  cat <<'PROMPTS'
p01	dashboard-generic	Utility operations dashboard	Design a dense operations dashboard for a regional utilities company. Include service health, outage tickets, crew dispatch status, demand charts, and clear alert priority.
p02	dashboard-generic	Analytics growth dashboard	Design a polished analytics dashboard for a B2B SaaS growth team. Include KPI cards, funnel conversion, cohort retention, campaign performance, and an executive summary panel.
p03	dashboard-generic	Admin control dashboard	Design an admin control dashboard for a multi-tenant product. Include sidebar navigation, user and role management, system health, billing status, and recent audit log rows.
p04	mobile-generic	Fitness tracker home	Design a health and fitness tracking mobile app homepage. Include a greeting header "Good morning, Alex", today's activity ring for steps, calories and distance, a heart rate card with mini chart, weekly workout bars, two upcoming workout cards, and bottom tabs.
p05	mobile-generic	Travel itinerary mobile	Design a travel itinerary mobile screen for an upcoming city trip. Include day tabs, hotel and flight summary cards, a timeline of activities, transit hints, weather, and a sticky bottom action.
p06	mobile-generic	Food delivery home	Design a premium food delivery mobile app homepage. Include delivery location, search, horizontal food categories, a featured discount card, restaurant cards with ratings and delivery times, cart state, and bottom tabs.
p07	mobile-generic	Finance wallet mobile	Design a personal finance mobile wallet home screen. Include balance, spending trend, budget progress, recent transactions, upcoming bills, savings goal, and secure bottom navigation.
p08	landing-generic	Climate SaaS landing	Design a modern landing page for a climate risk SaaS called TerraScope. Include nav, hero headline, email capture, product screenshot, three feature cards, customer logos, testimonial, and footer links.
p09	landing-generic	Camera product landing	Design a premium product landing page for a compact mirrorless camera. Include hero product image area, specs strip, color swatches, feature sections, reviews, comparison card, and buy CTA.
p10	web-app-generic	Luxury barbershop management console	Design a luxury web app for managing barbershop clients. Include sidebar navigation, client table, appointment calendar, revenue metrics, stylist performance, and an upscale dark visual style.
p11	real-this-week	Travel booking explore replica	Design a travel booking mobile app explore page. Include a search section with "Where to?" input, date picker chips, and guest count. Popular destinations as horizontal scrollable cards with destination photos, city names and starting prices. "Deals of the Week" section with 2 featured deal cards showing discount badges. Recently viewed section with 2 compact cards. Bottom tab bar (Explore, Wishlists, Trips, Messages, Profile). Warm, inviting design with orange accents.
p12	real-this-week	UtilityOps dashboard replica	Technical dashboard web app for a utilities company.
p13	real-this-week	Prague coffee shop replica	Dark bold website for a coffee shop in Prague.
p14	real-this-week	Barbershop luxury replica	Luxury webapp for managing barbershop clients.
p15	real-this-week	Fitness home replica	Design a health and fitness tracking mobile app homepage. Include a greeting header "Good morning, Alex", today's activity ring (steps, calories, distance), a heart rate card with mini chart, weekly workout summary bar chart, upcoming workout schedule with 2 cards (Morning Run, Yoga Session), and a bottom tab bar (Today, Workouts, Nutrition, Profile). Use a dark background with green accent.
PROMPTS
}

default_models() {
  local models="glm-5.2"
  if [ -n "${OPENPENCIL_STRONG_API_KEY:-}" ] &&
    [ -n "${OPENPENCIL_STRONG_BASE_URL:-}" ] &&
    [ -n "${OPENPENCIL_STRONG_MODEL:-}" ]; then
    models="$models,$CUSTOM_STRONG_LABEL"
  elif [ -n "${OPENAI_API_KEY:-${OPENPENCIL_OPENAI_API_KEY:-}}" ]; then
    models="$models,$OPENAI_STRONG_LABEL"
  elif [ -n "${ANTHROPIC_API_KEY:-${OPENPENCIL_ANTHROPIC_API_KEY:-}}" ]; then
    record_gap "ANTHROPIC_API_KEY is present but was not auto-added as a strong model because current op-smoke loop mode only supports OpenAI-compatible OPENPENCIL_LLM_* wiring."
  fi
  printf '%s' "$models"
}

MODEL_LABEL=""
MODEL_NAME=""
MODEL_BASE=""
MODEL_KEY=""
MODEL_KEY_ENV=""

configure_model() {
  MODEL_LABEL="$(sanitize_label "$1")"
  MODEL_NAME=""
  MODEL_BASE=""
  MODEL_KEY=""
  MODEL_KEY_ENV=""

  case "$1" in
    glm-5.2)
      MODEL_NAME="glm-5.2"
      MODEL_BASE="$GLM_BASE"
      MODEL_KEY_ENV="GLM_KEY"
      MODEL_KEY="${GLM_KEY:-}"
      ;;
    "$CUSTOM_STRONG_LABEL")
      MODEL_NAME="${OPENPENCIL_STRONG_MODEL:-}"
      MODEL_BASE="${OPENPENCIL_STRONG_BASE_URL:-}"
      MODEL_KEY_ENV="OPENPENCIL_STRONG_API_KEY"
      MODEL_KEY="${OPENPENCIL_STRONG_API_KEY:-}"
      ;;
    "$OPENAI_STRONG_LABEL")
      MODEL_NAME="${OPENPENCIL_OPENAI_STRONG_MODEL:-gpt-4.1}"
      MODEL_BASE="https://api.openai.com/v1"
      if [ -n "${OPENPENCIL_OPENAI_API_KEY:-}" ]; then
        MODEL_KEY_ENV="OPENPENCIL_OPENAI_API_KEY"
        MODEL_KEY="$OPENPENCIL_OPENAI_API_KEY"
      else
        MODEL_KEY_ENV="OPENAI_API_KEY"
        MODEL_KEY="${OPENAI_API_KEY:-}"
      fi
      ;;
    *)
      return 1
      ;;
  esac

  [ -n "$MODEL_NAME" ] && [ -n "$MODEL_BASE" ]
}

write_selected_prompts() {
  local prompts_path="$OUT/corpus.tsv"
  local count=0
  : >"$prompts_path"
  while IFS=$'\t' read -r prompt_id category title prompt; do
    [ -n "$prompt_id" ] || continue
    count=$((count + 1))
    if [ "$LIMIT" -gt 0 ] && [ "$count" -gt "$LIMIT" ]; then
      break
    fi
    printf '%s\t%s\t%s\t%s\n' "$prompt_id" "$category" "$title" "$prompt" >>"$prompts_path"
  done <<EOF
$(prompt_corpus)
EOF
}

write_selected_models() {
  local models_path="$OUT/models.tsv"
  local raw_models="${MODELS_ARG:-}"
  local missing=0
  local old_ifs
  : >"$models_path"

  if [ -z "$raw_models" ]; then
    raw_models="$(default_models)"
  fi

  old_ifs="$IFS"
  IFS=','
  set -- $raw_models
  IFS="$old_ifs"
  for requested in "$@"; do
    [ -n "$requested" ] || continue
    if ! configure_model "$requested"; then
      die "unknown model label: $requested (known: glm-5.2,$CUSTOM_STRONG_LABEL,$OPENAI_STRONG_LABEL)"
    fi
    if [ -z "$MODEL_KEY" ]; then
      record_gap "$MODEL_KEY_ENV is not set for $MODEL_LABEL ($MODEL_NAME)."
      missing=1
    fi
    printf '%s\t%s\t%s\t%s\t%s\n' \
      "$MODEL_LABEL" "$MODEL_NAME" "$MODEL_BASE" "$MODEL_KEY_ENV" \
      "$([ -n "$MODEL_KEY" ] && printf yes || printf no)" >>"$models_path"
  done

  if [ ! -s "$models_path" ]; then
    die "no models selected"
  fi

  if [ "$missing" -ne 0 ] && ! is_truthy "$DRY_RUN"; then
    echo "error: required model key(s) missing; see $GAPS_PATH" >&2
    echo "hint: rerun with --dry-run to validate orchestration without LLM calls" >&2
    exit 3
  fi
}

run_logged_timeout() {
  local timeout_s="$1"
  local log_path="$2"
  shift 2
  python3 - "$timeout_s" "$log_path" "$@" <<'PY'
import subprocess
import sys

timeout_s = int(sys.argv[1])
log_path = sys.argv[2]
cmd = sys.argv[3:]

with open(log_path, "w", encoding="utf-8", errors="replace") as log:
    try:
        completed = subprocess.run(
            cmd,
            stdout=log,
            stderr=subprocess.STDOUT,
            timeout=timeout_s,
            check=False,
        )
        sys.exit(completed.returncode)
    except subprocess.TimeoutExpired:
        log.write(f"\n[TIMEOUT] exceeded {timeout_s}s\n")
        sys.exit(124)
PY
}

run_split_timeout() {
  local timeout_s="$1"
  local stdout_path="$2"
  local log_path="$3"
  shift 3
  python3 - "$timeout_s" "$stdout_path" "$log_path" "$@" <<'PY'
import subprocess
import sys

timeout_s = int(sys.argv[1])
stdout_path = sys.argv[2]
log_path = sys.argv[3]
cmd = sys.argv[4:]

with open(stdout_path, "w", encoding="utf-8", errors="replace") as out, \
     open(log_path, "w", encoding="utf-8", errors="replace") as log:
    try:
        completed = subprocess.run(
            cmd,
            stdout=out,
            stderr=log,
            timeout=timeout_s,
            check=False,
        )
        sys.exit(completed.returncode)
    except subprocess.TimeoutExpired:
        log.write(f"\n[TIMEOUT] exceeded {timeout_s}s\n")
        sys.exit(124)
PY
}

has_png() {
  local shots_dir="$1"
  [ -n "$(find "$shots_dir" -type f -name '*.png' -print -quit 2>/dev/null)" ]
}

result_is_dry_run() {
  local result_path="$1"
  python3 - "$result_path" <<'PY'
import json
import sys

try:
    row = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:
    sys.exit(1)
sys.exit(0 if row.get("dryRun") else 1)
PY
}

write_result_json() {
  RESULT_PATH="$1" \
    OUT_ROOT="$OUT" \
    MODEL_LABEL="$2" \
    MODEL_NAME="$3" \
    MODEL_BASE="$4" \
    MODEL_KEY_ENV="$5" \
    MODE="$6" \
    PROMPT_ID="$7" \
    PROMPT_CATEGORY="$8" \
    PROMPT_TITLE="$9" \
    PROMPT_TEXT="${10}" \
    STATUS="${11}" \
    GEN_RC="${12}" \
    AUDIT_RC="${13}" \
    RENDER_RC="${14}" \
    GEN_SECONDS="${15}" \
    TOTAL_SECONDS="${16}" \
    OP_PATH="${17}" \
    AUDIT_PATH="${18}" \
    SHOTS_DIR="${19}" \
    GEN_LOG="${20}" \
    AUDIT_LOG="${21}" \
    RENDER_LOG="${22}" \
    DRY_RUN_FLAG="$DRY_RUN" \
    python3 - <<'PY'
import glob
import json
import os
from collections import Counter

out_root = os.environ["OUT_ROOT"]

def rel(path):
    if not path:
        return ""
    return os.path.relpath(path, out_root)

def int_env(name, default):
    value = os.environ.get(name, "")
    try:
        return int(value)
    except ValueError:
        return default

def excerpt(path):
    if not path or not os.path.exists(path):
        return ""
    text = open(path, encoding="utf-8", errors="replace").read()
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    if not lines:
        return ""
    return "\n".join(lines[-8:])[:900]

def classify_issue(line):
    lowered = line.lower()
    if "declared height fill_container" in lowered or "resolved to ~0px" in lowered:
        return "collapsed_fill_container"
    if "empty" in lowered and "stub" in lowered:
        return "empty_stub"
    if "navbar" in lowered or "nav links" in lowered:
        return "navbar_archetype"
    if "fixed column widths" in lowered or "columns cannot fit" in lowered:
        return "table_width"
    if "text resolved" in lowered or "text shreds" in lowered:
        return "text_overflow"
    if "spills out" in lowered:
        return "child_overflow"
    if "starved" in lowered:
        return "starved_fill"
    if "jam" in lowered:
        return "sibling_jam"
    return "other"

audit_path = os.environ["AUDIT_PATH"]
audit_issue_count = None
audit_kinds = {}
audit_parse_error = ""
rubric = {}
if audit_path and os.path.exists(audit_path) and os.path.getsize(audit_path) > 0:
    try:
        audit = json.load(open(audit_path, encoding="utf-8"))
        issues = audit.get("issues", [])
        audit_issue_count = audit.get("issueCount", len(issues) if isinstance(issues, list) else None)
        if isinstance(issues, list):
            audit_kinds = dict(Counter(classify_issue(str(issue)) for issue in issues))
        # Rubric block (op-smoke audit >= 0710): chrome completeness /
        # node-vocabulary richness / density counters — the dimensions the
        # 07-04 comparison missed. Absent on older binaries → fields None.
        rubric = audit.get("rubric") or {}
    except Exception as err:
        audit_parse_error = str(err)

def rubric_chrome(rubric):
    """True only when EVERY mobile screen ships complete chrome; None when
    no screen is judged (desktop-only design or missing rubric)."""
    screens = rubric.get("screens") or []
    judged = [s.get("chromeComplete") for s in screens if s.get("chromeComplete") is not None]
    if not judged:
        return None
    return all(judged)

shots_dir = os.environ["SHOTS_DIR"]
shots = []
if shots_dir and os.path.isdir(shots_dir):
    shots = [rel(path) for path in sorted(glob.glob(os.path.join(shots_dir, "*.png")))]

status = os.environ["STATUS"]
success = status == "pass"
log_path = os.environ["GEN_LOG"]
if status == "fail_audit":
    log_path = os.environ["AUDIT_LOG"]
elif status == "fail_render":
    log_path = os.environ["RENDER_LOG"]

row = {
    "model": os.environ["MODEL_LABEL"],
    "modelName": os.environ["MODEL_NAME"],
    "modelBaseUrl": os.environ["MODEL_BASE"],
    "keyEnv": os.environ["MODEL_KEY_ENV"],
    "mode": os.environ["MODE"],
    "promptId": os.environ["PROMPT_ID"],
    "category": os.environ["PROMPT_CATEGORY"],
    "title": os.environ["PROMPT_TITLE"],
    "prompt": os.environ["PROMPT_TEXT"],
    "status": status,
    "success": success,
    "dryRun": os.environ["DRY_RUN_FLAG"].lower() in ("1", "true", "yes", "on"),
    "generationReturnCode": int_env("GEN_RC", -1),
    "auditReturnCode": int_env("AUDIT_RC", -1),
    "renderReturnCode": int_env("RENDER_RC", -1),
    "generationSeconds": int_env("GEN_SECONDS", 0),
    "totalSeconds": int_env("TOTAL_SECONDS", 0),
    "op": rel(os.environ["OP_PATH"]) if os.path.exists(os.environ["OP_PATH"]) else "",
    "audit": rel(audit_path) if os.path.exists(audit_path) else "",
    "shots": shots,
    "issueCount": audit_issue_count,
    "chromeComplete": rubric_chrome(rubric),
    "vocabularyRichness": rubric.get("vocabularyRichness"),
    "iconNodes": rubric.get("iconNodes"),
    "imageFills": rubric.get("imageFills"),
    "textNodes": rubric.get("textNodes"),
    "auditKinds": audit_kinds,
    "auditParseError": audit_parse_error,
    "logs": {
        "generation": rel(os.environ["GEN_LOG"]) if os.path.exists(os.environ["GEN_LOG"]) else "",
        "audit": rel(os.environ["AUDIT_LOG"]) if os.path.exists(os.environ["AUDIT_LOG"]) else "",
        "render": rel(os.environ["RENDER_LOG"]) if os.path.exists(os.environ["RENDER_LOG"]) else "",
    },
    "errorExcerpt": "" if success else excerpt(log_path),
}

with open(os.environ["RESULT_PATH"], "w", encoding="utf-8") as f:
    json.dump(row, f, indent=2, ensure_ascii=False)
    f.write("\n")
PY
}

run_cell() {
  local model_label="$1"
  local model_name="$2"
  local model_base="$3"
  local model_key_env="$4"
  local model_key="$5"
  local mode="$6"
  local prompt_id="$7"
  local category="$8"
  local title="$9"
  local prompt="${10}"
  local cell_dir="$OUT/$model_label/$mode/$prompt_id"
  local result_path="$cell_dir/result.json"
  local op_path="$cell_dir/$prompt_id.op"
  local audit_path="$cell_dir/audit.json"
  local shots_dir="$cell_dir/shots"
  local gen_log="$cell_dir/generation.log"
  local audit_log="$cell_dir/audit.log"
  local render_log="$cell_dir/render.log"
  local gen_rc=-1
  local audit_rc=-1
  local render_rc=-1
  local status="pass"
  local gen_start=0
  local gen_seconds=0
  local cell_start=0
  local total_seconds=0

  if [ -s "$result_path" ] &&
    { is_truthy "$DRY_RUN" || ! result_is_dry_run "$result_path"; }; then
    echo "[skip] $model_label/$mode/$prompt_id" >&2
    return 0
  fi

  mkdir -p "$shots_dir"
  printf '%s\n' "$prompt" >"$cell_dir/prompt.txt"

  if is_truthy "$DRY_RUN"; then
    status="dry-run"
    write_result_json "$result_path" "$model_label" "$model_name" "$model_base" "$model_key_env" \
      "$mode" "$prompt_id" "$category" "$title" "$prompt" "$status" 0 -1 -1 0 0 \
      "$op_path" "$audit_path" "$shots_dir" "$gen_log" "$audit_log" "$render_log"
    echo "[dry-run] $model_label/$mode/$prompt_id" >&2
    return 0
  fi

  echo "[run] $model_label/$mode/$prompt_id" >&2
  cell_start="$(date +%s)"
  gen_start="$(date +%s)"
  if [ "$mode" = "loop" ]; then
    run_logged_timeout "$TIMEOUT_S" "$gen_log" \
      env \
      OPENPENCIL_SMOKE_LOOP=1 \
      OPENPENCIL_SMOKE_OUT="$op_path" \
      OPENPENCIL_LLM_PROVIDER=openai-compat \
      OPENPENCIL_LLM_API_KEY="$model_key" \
      OPENPENCIL_LLM_BASE_URL="$model_base" \
      OPENPENCIL_ORCHESTRATOR_MODEL="$model_name" \
      "$SMOKE" "$prompt" || gen_rc=$?
  else
    run_logged_timeout "$TIMEOUT_S" "$gen_log" \
      env \
      OPENPENCIL_SMOKE_DIRECT=1 \
      OPENPENCIL_SMOKE_OUT="$op_path" \
      OPENPENCIL_LLM_PROVIDER=openai-compat \
      OPENPENCIL_LLM_API_KEY="$model_key" \
      OPENPENCIL_LLM_BASE_URL="$model_base" \
      OPENPENCIL_ORCHESTRATOR_MODEL="$model_name" \
      "$SMOKE" "$prompt" || gen_rc=$?
  fi
  if [ "$gen_rc" -eq -1 ]; then
    gen_rc=0
  fi
  gen_seconds=$(($(date +%s) - gen_start))

  if [ "$gen_rc" -eq 124 ]; then
    status="timeout"
  elif [ "$gen_rc" -ne 0 ]; then
    status="fail_generation"
  elif [ ! -s "$op_path" ]; then
    status="fail_empty_output"
  else
    run_split_timeout "$AUDIT_TIMEOUT_S" "$audit_path" "$audit_log" \
      env \
      OPENPENCIL_SMOKE_AUDIT="$op_path" \
      OPENPENCIL_LLM_PROVIDER=openai-compat \
      OPENPENCIL_LLM_API_KEY="$model_key" \
      OPENPENCIL_LLM_BASE_URL="$model_base" \
      OPENPENCIL_ORCHESTRATOR_MODEL="$model_name" \
      "$SMOKE" audit || audit_rc=$?
    if [ "$audit_rc" -eq -1 ]; then
      audit_rc=0
    fi
    if { [ "$audit_rc" -ne 0 ] && [ "$audit_rc" -ne 1 ]; } || [ ! -s "$audit_path" ]; then
      status="fail_audit"
    else
      run_logged_timeout "$RENDER_TIMEOUT_S" "$render_log" \
        "$DESKTOP" --render-shots "$op_path" "$shots_dir" || render_rc=$?
      if [ "$render_rc" -eq -1 ]; then
        render_rc=0
      fi
      if [ "$render_rc" -ne 0 ] || ! has_png "$shots_dir"; then
        status="fail_render"
      fi
    fi
  fi

  total_seconds=$(($(date +%s) - cell_start))
  write_result_json "$result_path" "$model_label" "$model_name" "$model_base" "$model_key_env" \
    "$mode" "$prompt_id" "$category" "$title" "$prompt" "$status" "$gen_rc" "$audit_rc" \
    "$render_rc" "$gen_seconds" "$total_seconds" "$op_path" "$audit_path" "$shots_dir" \
    "$gen_log" "$audit_log" "$render_log"
}

write_summary() {
  SUMMARY_PATH="$SUMMARY_PATH" OUT_ROOT="$OUT" GAPS_PATH="$GAPS_PATH" python3 - <<'PY'
import glob
import json
import os
from collections import Counter
from datetime import datetime, timezone

out_root = os.environ["OUT_ROOT"]
summary_path = os.environ["SUMMARY_PATH"]
gaps_path = os.environ["GAPS_PATH"]

def md(text):
    if text is None:
        return ""
    text = str(text).replace("\\", "\\\\").replace("|", "\\|")
    text = text.replace("\n", "<br>")
    return text

def read_tsv(path):
    rows = []
    if not os.path.exists(path):
        return rows
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            rows.append(line.split("\t"))
    return rows

prompts = read_tsv(os.path.join(out_root, "corpus.tsv"))
models = read_tsv(os.path.join(out_root, "models.tsv"))
results = {}
all_results = []
for path in glob.glob(os.path.join(out_root, "*", "*", "p*", "result.json")):
    try:
        row = json.load(open(path, encoding="utf-8"))
    except Exception:
        continue
    key = (row.get("model", ""), row.get("mode", ""), row.get("promptId", ""))
    results[key] = row
    all_results.append(row)

def cell(row, field):
    if not row:
        return "missing"
    if field == "issue":
        value = row.get("issueCount")
        return "NA" if value is None else str(value)
    if field == "chrome":
        value = row.get("chromeComplete")
        return "NA" if value is None else ("ok" if value else "MISSING")
    if field == "rich":
        value = row.get("vocabularyRichness")
        return "NA" if value is None else str(value)
    if field == "time":
        value = row.get("generationSeconds")
        return f"{value}s" if value is not None else "NA"
    if field == "status":
        return "pass" if row.get("success") else row.get("status", "fail")
    if field == "shots":
        shots = row.get("shots") or []
        return "<br>".join(f"`{md(s)}`" for s in shots) if shots else ""
    return ""

def failure_text(*rows):
    parts = []
    for row in rows:
        if row and not row.get("success") and row.get("status") != "dry-run":
            excerpt = row.get("errorExcerpt") or row.get("status", "fail")
            parts.append(f"{row.get('mode')}: {excerpt}")
    return md("\n".join(parts))

lines = []
lines.append("# G3 A/B Results")
lines.append("")
lines.append(f"- Generated: {datetime.now(timezone.utc).isoformat()}")
lines.append(f"- Output root: `{out_root}`")
lines.append(f"- Summary path: `{summary_path}`")
lines.append(f"- Resume marker: each completed cell has `result.json` under `<out>/<model>/<mode>/<p##>/`.")
lines.append("")
lines.append("## op-smoke Env Contract")
lines.append("")
lines.append("- `glm-5.2`: `GLM_KEY` is mapped to `OPENPENCIL_LLM_API_KEY`; base URL is `https://open.bigmodel.cn/api/coding/paas/v4`; model is `OPENPENCIL_ORCHESTRATOR_MODEL=glm-5.2`.")
lines.append("- Both modes use `OPENPENCIL_LLM_PROVIDER=openai-compat`, `OPENPENCIL_LLM_BASE_URL`, `OPENPENCIL_LLM_API_KEY`, `OPENPENCIL_ORCHESTRATOR_MODEL`, and `OPENPENCIL_SMOKE_OUT`.")
lines.append("- Orchestrator mode adds `OPENPENCIL_SMOKE_DIRECT=1`; loop mode adds `OPENPENCIL_SMOKE_LOOP=1` and currently requires OpenAI-compatible provider wiring.")
lines.append("- Audit runs `OPENPENCIL_SMOKE_AUDIT=<file.op> op-smoke audit`; current `op-smoke` still constructs the provider before the audit branch, so the harness passes the same `OPENPENCIL_LLM_*` env into audit. Audit output has `issueCount` plus string `issues`, not structured categories.")
lines.append("- Render shots run `openpencil-desktop --render-shots <file.op> <outdir>`.")
lines.append("")
lines.append("## Models")
lines.append("")
lines.append("| label | actual model | base URL | key env | key present |")
lines.append("|---|---|---|---|---|")
for model in models:
    label, name, base, key_env, present = (model + [""] * 5)[:5]
    lines.append(f"| `{md(label)}` | `{md(name)}` | `{md(base)}` | `{md(key_env)}` | {md(present)} |")

gaps = []
if os.path.exists(gaps_path):
    gaps = [line.strip() for line in open(gaps_path, encoding="utf-8") if line.strip()]
if gaps:
    lines.append("")
    lines.append("## Gaps")
    lines.append("")
    for gap in gaps:
        lines.append(f"- {md(gap)}")

for model in models:
    label = model[0]
    name = model[1] if len(model) > 1 else label
    lines.append("")
    lines.append(f"## Results: {md(label)} (`{md(name)}`)")
    lines.append("")
    lines.append("| prompt | category | loop issues | loop chrome | loop vocab | loop time | loop status | loop shots | orch issues | orch chrome | orch vocab | orch time | orch status | orch shots | failures |")
    lines.append("|---|---|---:|---|---:|---:|---|---|---:|---|---:|---:|---|---|---|")
    for prompt in prompts:
        prompt_id, category, title, _prompt_text = (prompt + [""] * 4)[:4]
        loop = results.get((label, "loop", prompt_id))
        orch = results.get((label, "orchestrator", prompt_id))
        lines.append(
            "| "
            f"`{md(prompt_id)}` {md(title)} | {md(category)} | "
            f"{cell(loop, 'issue')} | {cell(loop, 'chrome')} | {cell(loop, 'rich')} | {cell(loop, 'time')} | {md(cell(loop, 'status'))} | {cell(loop, 'shots')} | "
            f"{cell(orch, 'issue')} | {cell(orch, 'chrome')} | {cell(orch, 'rich')} | {cell(orch, 'time')} | {md(cell(orch, 'status'))} | {cell(orch, 'shots')} | "
            f"{failure_text(loop, orch)} |"
        )

kind_counts = Counter()
for row in all_results:
    for kind, count in (row.get("auditKinds") or {}).items():
        kind_counts[(row.get("model", ""), row.get("mode", ""), kind)] += int(count)
if kind_counts:
    lines.append("")
    lines.append("## Audit Kind Counts")
    lines.append("")
    lines.append("| model | mode | kind | count |")
    lines.append("|---|---|---|---:|")
    for (model, mode, kind), count in sorted(kind_counts.items()):
        lines.append(f"| `{md(model)}` | {md(mode)} | `{md(kind)}` | {count} |")

with open(summary_path, "w", encoding="utf-8") as f:
    f.write("\n".join(lines) + "\n")
PY
}

main() {
  require_tools
  record_gap "OPENPENCIL_SMOKE_AUDIT has no structured issue category field; auditKinds is a best-effort histogram derived from issue strings."
  write_selected_prompts
  write_selected_models

  while IFS=$'\t' read -r model_label model_name model_base model_key_env _key_present; do
    configure_model "$model_label" || die "internal model config failed: $model_label"
    while IFS=$'\t' read -r prompt_id category title prompt; do
      run_cell "$model_label" "$model_name" "$model_base" "$model_key_env" "$MODEL_KEY" \
        loop "$prompt_id" "$category" "$title" "$prompt"
      run_cell "$model_label" "$model_name" "$model_base" "$model_key_env" "$MODEL_KEY" \
        orchestrator "$prompt_id" "$category" "$title" "$prompt"
    done <"$OUT/corpus.tsv"
  done <"$OUT/models.tsv"

  write_summary
  printf '%s\n' "$SUMMARY_PATH"
}

main "$@"

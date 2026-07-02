#!/usr/bin/env bash
# Self-contained develop→generate→render→audit loop, zero human in the loop.
#
#   scripts/self-loop.sh <run-name> <prompts.txt> [out-dir]
#
# prompts.txt: one prompt per line (# comments / blank lines skipped).
# For each prompt N:
#   1. GENERATE  op-smoke (orchestrator DIRECT by default; set
#                OPENPENCIL_SMOKE_LOOP=1 for the agentic loop)
#                → <out>/<run>/pNN.op
#   2. RENDER    openpencil-desktop --render-shots — the REAL jian-core
#                (taffy) layout + jian-skia paint, the exact desktop canvas
#                pipeline → <out>/<run>/pNN_shots/*.png
#   3. AUDIT     op-smoke OPENPENCIL_SMOKE_AUDIT — real-layout geometry
#                diagnostics (collapse / table overflow / text & frame
#                overflow / sibling jam), ALSO computed by jian, so
#                generation feedback, audit and final pixels share ONE
#                layout engine → <out>/<run>/pNN.audit.json
# Finally assembles <out>/<run>/scorecard.json.
#
# Vision scoring is the agent's job afterwards: Read each shot PNG and score
# it against openpencil-docs/self-loop/rubric.md (same rubric scores the
# Pencil reference anchors, giving the quantified gap).
#
# Model env (required): OPENPENCIL_LLM_PROVIDER / OPENPENCIL_LLM_BASE_URL /
# OPENPENCIL_LLM_API_KEY / OPENPENCIL_ORCHESTRATOR_MODEL.
set -uo pipefail
REPO="$(cd "$(dirname "$0")/.." && pwd)"
RUN="${1:?run name required}"
PROMPTS="${2:?prompts file required}"
OUT="${3:-/tmp/self-loop}/$RUN"
mkdir -p "$OUT"
SMOKE="$REPO/target/release/op-smoke"
DESKTOP="$REPO/target/release/openpencil-desktop"
[ -x "$SMOKE" ] && [ -x "$DESKTOP" ] || {
  echo "build first: cargo build --release -p op-smoke -p op-host-desktop" >&2
  exit 2
}

i=0
while IFS= read -r prompt; do
  case "$prompt" in ''|'#'*) continue ;; esac
  i=$((i + 1))
  tag=$(printf 'p%02d' "$i")
  op="$OUT/$tag.op"
  printf '%s' "$prompt" >"$OUT/$tag.prompt"
  echo "== [$tag] ${prompt:0:80}..." >&2

  if [ -n "${OPENPENCIL_SMOKE_LOOP:-}" ]; then
    OPENPENCIL_SMOKE_OUT="$op" "$SMOKE" "$prompt" \
      >"$OUT/$tag.stdout" 2>"$OUT/$tag.log"
  else
    OPENPENCIL_SMOKE_DIRECT=1 OPENPENCIL_SMOKE_OUT="$op" "$SMOKE" "$prompt" \
      >"$OUT/$tag.stdout" 2>"$OUT/$tag.log"
  fi
  gen_ok=false
  [ -s "$op" ] && gen_ok=true

  render_ok=false
  shots="$OUT/${tag}_shots"
  if [ "$gen_ok" = true ]; then
    mkdir -p "$shots"
    "$DESKTOP" --render-shots "$op" "$shots" 2 >/dev/null 2>&1 \
      && [ -n "$(ls -A "$shots" 2>/dev/null)" ] && render_ok=true
  fi

  if [ "$gen_ok" = true ]; then
    OPENPENCIL_SMOKE_AUDIT="$op" "$SMOKE" audit >"$OUT/$tag.audit.json" 2>/dev/null || true
  fi

  GEN_OK="$gen_ok" RENDER_OK="$render_ok" TAG="$tag" OUT_DIR="$OUT" python3 - <<'PY'
import json, os
out, tag = os.environ["OUT_DIR"], os.environ["TAG"]
row = {
    "tag": tag,
    "prompt": open(f"{out}/{tag}.prompt").read()[:120],
    "genOk": os.environ["GEN_OK"] == "true",
    "renderOk": os.environ["RENDER_OK"] == "true",
    "nodes": 0,
    "auditIssues": -1,
}
try:
    d = json.load(open(f"{out}/{tag}.op"))
    def cnt(n):
        return 1 + sum(cnt(c) for c in n.get("children", []))
    roots = d.get("children") or d.get("pages", [{}])[0].get("children", [])
    row["nodes"] = sum(cnt(x) for x in roots)
except Exception:
    pass
try:
    row["auditIssues"] = json.load(open(f"{out}/{tag}.audit.json"))["issueCount"]
except Exception:
    pass
json.dump(row, open(f"{out}/{tag}.row.json", "w"), ensure_ascii=False)
print(f"   gen={row['genOk']} render={row['renderOk']} nodes={row['nodes']} audit_issues={row['auditIssues']}")
PY
done <"$PROMPTS"

RUN_NAME="$RUN" OUT_DIR="$OUT" python3 - <<'PY'
import glob, json, os
out = os.environ["OUT_DIR"]
rows = [json.load(open(p)) for p in sorted(glob.glob(f"{out}/p*.row.json"))]
clean = sum(1 for r in rows if r["renderOk"] and r["auditIssues"] == 0)
card = {
    "run": os.environ["RUN_NAME"],
    "prompts": len(rows),
    "structurallyClean": clean,
    "rows": rows,
}
json.dump(card, open(f"{out}/scorecard.json", "w"), indent=1, ensure_ascii=False)
print(json.dumps(card, indent=1, ensure_ascii=False))
PY
echo "scorecard → $OUT/scorecard.json" >&2

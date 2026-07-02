#!/usr/bin/env bash
# prep_parity.sh <source.pen> <name> [scale]
#
# Deterministic (no-Pencil) half of the render-parity loop:
#   1. pen2op converts <source.pen> -> <name>.op   (preserves node ids)
#   2. openpencil-desktop --render-shots renders our side, one PNG per top node
#   3. prints the top-level node-id JSON array to paste into the Pencil MCP
#      export_nodes call that captures the baseline (Pencil's own render).
#
# After running this, capture the baseline via MCP:
#   export_nodes(filePath=<source.pen>, outputDir=<BASELINE_DIR>,
#                nodeIds=<NODE_IDS_JSON>, scale=<same scale>, format=png)
# then diff:
#   diff_nodes.py <BASELINE_DIR> <OURS_DIR> --out <report>
set -euo pipefail

PEN="$1"; NAME="$2"; SCALE="${3:-2}"
ROOT="${RPARITY_ROOT:?set RPARITY_ROOT to a working dir}"
BIN="${OPENPENCIL_BIN:-/Users/fini/workspace/openpencil/target/debug}"

OP="$ROOT/$NAME.op"
OURS="$ROOT/$NAME/ours"
BASE="$ROOT/$NAME/baseline"
mkdir -p "$OURS" "$BASE"

"$BIN/pen2op" "$PEN" "$OP" >&2
"$BIN/openpencil-desktop" --render-shots "$OP" "$OURS" "$SCALE" >&2

ids=$(ls "$OURS"/*.png | xargs -n1 basename | sed 's/\.png$//')
json="[$(echo "$ids" | sed 's/.*/"&"/' | paste -sd, -)]"

echo "NAME=$NAME"
echo "SOURCE_PEN=$PEN"
echo "OURS_DIR=$OURS"
echo "BASELINE_DIR=$BASE"
echo "SCALE=$SCALE"
echo "NODE_COUNT=$(echo "$ids" | grep -c .)"
echo "NODE_IDS_JSON=$json"

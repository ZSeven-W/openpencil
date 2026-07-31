#!/usr/bin/env bash
# 渲染场景模板的预览图：每帧一张（scale 2）+ 整页拼合总览（scale 1）
# 卡片缩略图由 scene_preview_cards.py 从这些渲染产物再烤一层。
set -euo pipefail
R=/Users/fini/workspace/openpencil
BIN=$R/target/release/openpencil-desktop
OUT=$R/templates/step0/previews
TMP=$(mktemp -d)
mkdir -p "$OUT"

for t in screenshot-tutorial knowledge-carousel before-after slide-deck; do
  # The documents themselves are shipped assets (embedded by the scene
  # template catalogue); this directory keeps only the generators and the
  # full-resolution renders they produce.
  op=$R/crates/op-editor-core/assets/scene_templates/$t.op
  rm -rf "$TMP/$t"; mkdir -p "$TMP/$t"
  OPENPENCIL_RENDER_MARGIN=0 "$BIN" --render-shots "$op" "$TMP/$t" 2 >/dev/null

  # 按 .op 里 children 的顺序把 <node-id>.png 重命名成 <模板>-NN.png
  python3 - "$op" "$TMP/$t" "$OUT" "$t" <<'PY'
import json, shutil, sys, pathlib
op, shots, out, name = sys.argv[1:5]
doc = json.load(open(op))
kids = doc.get("children") or doc["pages"][0]["children"]
for i, n in enumerate(kids, 1):
    src = pathlib.Path(shots) / f"{n['id']}.png"
    dst = pathlib.Path(out) / (f"{name}.png" if len(kids) == 1
                               else f"{name}-{i:02d}.png")
    shutil.copyfile(src, dst)
    print(f"  {dst.name}  <- {n['name']}")
PY

  # 多帧模板再导一张整页总览（export_item 把所有顶层 frame 合成一张画布）
  n=$(python3 -c "import json;d=json.load(open('$op'));print(len(d.get('children') or d['pages'][0]['children']))")
  if [ "$n" -gt 1 ]; then
    printf '%s\n' \
      '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
      '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"export_item","arguments":{"itemId":"page-1","format":"png","scale":1}}}' \
    | "$BIN" --mcp "$op" 2>/dev/null | tail -1 \
    | python3 -c "
import sys, json, base64
t = json.loads(json.load(sys.stdin)['result']['content'][0]['text'])
open('$OUT/$t-overview.png','wb').write(base64.b64decode(t['bytes_base64']))
print('  $t-overview.png  <- 五帧拼合')
"
  fi
done
rm -rf "$TMP"
echo "--- previews ---"
ls -la "$OUT"

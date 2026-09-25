#!/usr/bin/env bash
# gate.sh <file.op ...> — 模板终审双闸。
#
#   1. OPENPENCIL_SMOKE_AUDIT   几何 / 结构 / 对比度诊断
#   2. cjkcheck + cjkreal 交叉   CJK 断行（audit 覆盖不到的那一类）
#
# stubcheck 已摘除：`is_empty_decorated_stub` 同一谓词双喂修复与诊断
# （geometry_diagnostics_collect.rs:147 有诊断调用点），空壳会被 audit 直接
# 报出来，再跑一遍是冗余。
#
# **cjkcheck 必须过 cjkreal 这道交叉才算数**：它按保守宽度模拟折行，会把
# 根本没折行的节点也报出来（实测 58 套里 13 报 → 11 真，2 套纯误报）。
# cjkreal 用 snapshot_layout 的 resolved height 反算真实行数：
#   真实行数 == 硬换行数  ⇒  没发生自动折行  ⇒  该条报告作废。
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
# OP_BIN_DIR points at another profile (e.g. target/debug).
BIN_DIR="${OP_BIN_DIR:-$REPO/target/release}"
FAIL=0

for f in "$@"; do
  name=$(basename "$f" .op)
  issues=$(OPENPENCIL_ANTHROPIC_API_KEY=dummy OPENPENCIL_SMOKE_AUDIT="$f" \
    "$BIN_DIR/op-smoke" audit 2>/dev/null \
    | python3 -c "import sys,json;print(json.loads(sys.stdin.read())['issueCount'])")
  if [ "$issues" != "0" ]; then
    echo "✗ $name  audit=$issues"
    FAIL=1
    continue
  fi
  # cjkcheck 原始报告 → cjkreal 交叉过滤
  real=$(python3 "$HERE/cjkreal.py" "$f" 2>/dev/null | grep -cE "^\s+(孤字末行|行首标点|疑似超框)" || true)
  if [ "$real" != "0" ]; then
    echo "✗ $name  audit=0  cjk(实测)=$real"
    python3 "$HERE/cjkreal.py" "$f" 2>/dev/null | sed 's/^/    /'
    FAIL=1
    continue
  fi
  # CJK 负字距（audit 与 cjk 两道都看不见的一类）
  track=$(python3 "$HERE/trackcheck.py" "$f" 2>/dev/null | grep -c "^\s\+字距越界" || true)
  if [ "$track" != "0" ]; then
    echo "✗ $name  audit=0  cjk=0  字距=$track"
    python3 "$HERE/trackcheck.py" "$f" 2>/dev/null | sed 's/^/    /'
    FAIL=1
  else
    echo "✓ $name  audit=0  cjk=0  字距=0"
  fi
done
exit $FAIL

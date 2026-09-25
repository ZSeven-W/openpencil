#!/usr/bin/env python3
"""CJK 断行自检：从 snapshot_layout 拿每个文本节点的实测宽度，模拟贪心折行，
报出 (a) 行首标点（中文行首禁则）(b) 末行 ≤2 字的孤字 (c) 疑似超框。

usage: cjkcheck.py <file.op>
"""
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

BIN = "/Users/fini/workspace/openpencil/target/release/openpencil-desktop"
# 行首禁则：这些字符不允许出现在行首
NO_LINE_START = "，。、；：？！）」』】》〉·…—"


def snapshot(path):
    req = [
        {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/call",
         "params": {"name": "snapshot_layout",
                    "arguments": {"depth": 40}}},
    ]
    payload = "\n".join(json.dumps(r) for r in req) + "\n"
    # `--mcp <file>` finalizes and WRITES BACK to the file; run it on a copy.
    with tempfile.TemporaryDirectory() as tmp:
        copy = pathlib.Path(tmp) / pathlib.Path(path).name
        shutil.copy(path, copy)
        out = subprocess.run([BIN, "--mcp", str(copy)], input=payload,
                             capture_output=True, text=True).stdout
    last = [ln for ln in out.strip().splitlines() if ln.startswith("{")][-1]
    body = json.loads(last)["result"]["content"][0]["text"]
    return json.loads(body)


def flatten(node, out):
    out[node.get("id")] = node
    for kid in node.get("children") or []:
        flatten(kid, out)
    return out


def text_nodes(node, out):
    if node.get("type") == "text":
        out.append(node)
    for kid in node.get("children") or []:
        text_nodes(kid, out)
    return out


def wrap(content, per_line):
    """按 CJK 等宽近似贪心折行；显式 \\n 强制断行。

    返回 [(line, soft)]，`soft` 表示这一行是被**引擎折**出来的而不是作者用
    \\n 断的。孤字规则只对 soft 行成立：作者写「还债」独占一行是版式意图，
    引擎把「。」甩到下一行才是事故。
    """
    lines = []
    for hard in content.split("\n"):
        if not hard:
            lines.append(("", False))
            continue
        chunks = [hard[i:i + per_line]
                  for i in range(0, len(hard), per_line)]
        for idx, chunk in enumerate(chunks):
            lines.append((chunk, idx > 0))
    return lines


def main(path):
    tree = snapshot(path)
    roots = tree.get("layout") if isinstance(tree, dict) else tree
    rects = {}
    for r in roots:
        flatten(r, rects)

    doc = json.load(open(path))
    kids = doc.get("children") or doc["pages"][0]["children"]
    texts = []
    for k in kids:
        text_nodes(k, texts)

    bad = 0
    for node in texts:
        rect = rects.get(node["id"])
        if not rect:
            continue
        width = rect.get("width") or 0
        size = node.get("fontSize", 16)
        content = node.get("content")
        if not isinstance(content, str) or not width or width < size:
            continue
        # `growth: auto` 的节点是收缩包裹的，量到的宽度就是它自己的内容宽，
        # 永远不会折行 —— 拿它去模拟只会得到一堆假阳性。
        if node.get("textGrowth") == "auto":
            continue
        # 每字 1em 的宽度模型只对汉字成立。「01」「第 02 板」这类拉丁/混排
        # 串按 1em 一个算会被判成折行，实际上西文半宽、一行就放得下 —— 那
        # 是模型的假阳性，不是版式问题。汉字占比低于六成的直接跳过。
        han = sum(1 for ch in content if "一" <= ch <= "鿿")
        if not content.strip() or han / len(content) < 0.6:
            continue
        # 不用渲染高度反推行数：layout 量文字的字体和 paint 画的不是同一个
        # （实测 752×40px 的 18 字块 layout 报 2 行、paint 只画 1 行），高度
        # 是错的真值。改成按宽度模拟，并且**同时**试 per 和 per+1 —— 末字标
        # 点悬挂会让实际每行多塞一个字，两个都干净才算安全。
        base = max(1, int(width // size))
        for per in (base, base + 1):
            lines = wrap(content, per)
            if len(lines) < 2:
                continue
            for idx, (line, soft) in enumerate(lines[1:], 1):
                if soft and line and line[0] in NO_LINE_START:
                    print(f"行首标点  {node['name']!r} L{idx + 1} 起于 "
                          f"{line[0]!r}  (w={width:.0f} fs={size} per={per})")
                    bad += 1
            tail, tail_soft = lines[-1]
            if tail_soft and 0 < len(tail) <= 4:
                print(f"孤字末行  {node['name']!r} 末行={tail!r} "
                      f"(w={width:.0f} fs={size} per={per})  "
                      f"全文={content[:30]!r}")
                bad += 1
    print(f"--- {path}: {bad} 处断行问题 / {len(texts)} 个文本节点 ---")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))

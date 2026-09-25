#!/usr/bin/env python3
"""预览新鲜度审计：落盘的预览产物是否等于**用当前二进制**重渲的结果。

场景模板有三层预览产物，每一层都可能悄悄过期：

    分帧 PNG   templates/step0/previews/<id>.png 或 <id>-NN.png
    overview   templates/step0/previews/<id>-overview.png（整页拼合）
    卡片 JPEG  crates/op-editor-ui/assets/scene_template_previews/<id>.jpg

过期的来源有两个，第二个是这个工具存在的理由：

  1. **文档变了**没重渲 —— 改了 .op 忘了刷预览，这个还算好想到；
  2. **渲染器变了** —— .op 一个字节没动，但 openpencil-desktop 重新编译后
     文字栅格化的结果变了，落盘图就从此对不上了。2026-08-09 那次就是这样：
     minimal-keynote 的 01/03/08 帧文档内容与 HEAD 完全一致，渲染出来却和落
     盘 PNG 不一致，查到最后是二进制在 08-07 之后重建过。

第 2 类没有任何人为痕迹可查 —— git diff 是干净的，测试是绿的，gate 也是绿
的，只有把图重渲一遍比字节才看得见。所以判据只有一条：**重渲比字节**。

前提是渲染确定：同一文档连渲两次必须字节一致（`--selftest` 验这条）。这一点
实测成立，也是整套方法的地基；哪天它不成立了，这个工具的结论全部作废，先修
渲染而不是信这里的输出。

**不进 gate.sh**：审计全库要把 64 套模板逐帧重渲一遍（分钟级），gate 是每次
改模板都要跑的快闸，扛不住这个量级。这里是独立工具，渲染器一变手动跑一次。

usage:
    freshcheck.py                     # 审计全库，只读
    freshcheck.py <id> [<id> ...]     # 只审计指定模板
    freshcheck.py --fix               # 审计并把过期产物刷新落盘
    freshcheck.py --selftest          # 验渲染确定性（连渲两次比字节）
"""
from __future__ import annotations

import base64
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

# qa/ 比 _generators/ 深一级，所以是 parents[4] 而不是 scene_preview_cards.py
# 用的 parents[3]。算错一级不会报错，只会让 glob 落空、审计静默地零套通过。
REPO = pathlib.Path(__file__).resolve().parents[4]
# OP_BIN_DIR points at another profile (e.g. target/debug).
BIN = pathlib.Path(os.environ.get("OP_BIN_DIR", REPO / "target" / "release")) / "openpencil-desktop"
DOCS = REPO / "crates" / "op-editor-core" / "assets" / "scene_templates"
PREVIEWS = REPO / "templates" / "step0" / "previews"
CARDS_DIR = REPO / "crates" / "op-editor-ui" / "assets" / "scene_template_previews"

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))


def boards(doc_path: pathlib.Path) -> list[dict]:
    doc = json.load(open(doc_path))
    return doc.get("children") or doc["pages"][0]["children"]


def frame_names(template: str, count: int) -> list[str]:
    """分帧图的文件名 —— 与 previews.sh 的重命名规则必须一致。

    单板模板是 `<id>.png`（没有序号后缀），多板才是 `<id>-NN.png`。两边一旦
    对不上，审计会把所有单板模板报成「产物缺失」。
    """
    if count == 1:
        return [f"{template}.png"]
    return [f"{template}-{index:02d}.png" for index in range(1, count + 1)]


def render_frames(doc: pathlib.Path, out: pathlib.Path, template: str) -> list[pathlib.Path]:
    """把每个顶层 frame 渲成一张 PNG，按 children 顺序改成对外的文件名。"""
    shots = out / "_shots"
    shots.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [str(BIN), "--render-shots", str(doc), str(shots), "2"],
        capture_output=True, text=True,
        env={"PATH": "/usr/bin:/bin", "OPENPENCIL_RENDER_MARGIN": "0"},
    )
    kids = boards(doc)
    written = []
    for name, node in zip(frame_names(template, len(kids)), kids):
        src = shots / f"{node['id']}.png"
        if not src.exists():
            return []
        dst = out / name
        shutil.copyfile(src, dst)
        written.append(dst)
    return written


def render_overview(doc: pathlib.Path, dst: pathlib.Path) -> bool:
    """整页拼合导出（export_item page-1），多板模板才有。"""
    req = [
        {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/call",
         "params": {"name": "export_item",
                    "arguments": {"itemId": "page-1", "format": "png", "scale": 1}}},
    ]
    out = subprocess.run(
        [str(BIN), "--mcp", str(doc)],
        input="\n".join(json.dumps(r) for r in req) + "\n",
        capture_output=True, text=True,
    ).stdout
    lines = [ln for ln in out.strip().splitlines() if ln.startswith("{")]
    if not lines:
        return False
    try:
        body = json.loads(json.loads(lines[-1])["result"]["content"][0]["text"])
    except (KeyError, IndexError, ValueError):
        return False
    dst.write_bytes(base64.b64decode(body["bytes_base64"]))
    return True


def same(a: pathlib.Path, b: pathlib.Path) -> bool:
    return a.exists() and b.exists() and a.read_bytes() == b.read_bytes()


def bake_card(template: str, dst: pathlib.Path, src_dir: pathlib.Path) -> bool:
    """从 `src_dir` 里的源图重烤卡片。

    刻意 import scene_preview_cards 而不是抄一份 bake 逻辑：卡片长什么样只能
    有一个定义，抄第二遍就会有第二个版本。

    `src_dir` 指向**刚渲出来的**帧而不是 previews/，否则只读审计会拿旧源烤出
    一张和旧卡片一致的图，把「帧过期了、卡片当然也过期」这层漏报成干净。一张
    卡片的源只会来自它自己那套模板，所以整体换掉源目录是安全的。
    """
    import scene_preview_cards as spc
    src = dict(spc.CARDS).get(template)
    if src is None:
        return False
    names = [src.name] if isinstance(src, spc.Top) else (src if isinstance(src, list) else [src])
    if any(not (src_dir / n).exists() for n in names):
        return False
    original = spc.SRC
    try:
        spc.SRC = src_dir
        arg = src if isinstance(src, spc.Top) else (
            [src_dir / n for n in src] if isinstance(src, list) else src_dir / src)
        spc.bake(arg).save(dst, "JPEG", quality=spc.JPEG_QUALITY, optimize=True)
    finally:
        spc.SRC = original
    return True


def audit(templates: list[str], fix: bool) -> int:
    stale_total = 0
    rows = []
    with tempfile.TemporaryDirectory() as tmp_root:
        for template in templates:
            doc = DOCS / f"{template}.op"
            tmp = pathlib.Path(tmp_root) / template
            tmp.mkdir(parents=True)
            count = len(boards(doc))
            stale: list[str] = []

            # ---- 第一层：分帧图
            fresh = render_frames(doc, tmp, template)
            if not fresh:
                rows.append((template, "渲染失败", count, 0))
                stale_total += 1
                continue
            for shot in fresh:
                landed = PREVIEWS / shot.name
                if not landed.exists():
                    continue          # 该产物本就不存在，不凭空造
                if not same(shot, landed):
                    stale.append(shot.name)
                    if fix:
                        shutil.copyfile(shot, landed)

            # ---- 第二层：overview（只在已有该产物时才校，缺的不补）
            landed_overview = PREVIEWS / f"{template}-overview.png"
            if count > 1 and landed_overview.exists():
                candidate = tmp / landed_overview.name
                if render_overview(doc, candidate) and not same(candidate, landed_overview):
                    stale.append(landed_overview.name)
                    if fix:
                        shutil.copyfile(candidate, landed_overview)

            # ---- 第三层：卡片，从 tmp 里的新鲜帧烤（见 bake_card 的说明）
            landed_card = CARDS_DIR / f"{template}.jpg"
            if landed_card.exists():
                candidate = tmp / f"{template}.jpg"
                if bake_card(template, candidate, tmp) and not same(candidate, landed_card):
                    stale.append(landed_card.name)
                    if fix:
                        shutil.copyfile(candidate, landed_card)

            stale_total += len(stale)
            rows.append((template, stale, count, len(fresh)))

    width = max(len(r[0]) for r in rows)
    for template, stale, count, _ in rows:
        if isinstance(stale, str):
            print(f"  {template:<{width}}  {stale}")
        elif stale:
            verb = "已刷新" if fix else "过期"
            print(f"  {template:<{width}}  {verb} {len(stale):2d} / {count} 帧+产物"
                  f"   {', '.join(stale)}")
    clean = sum(1 for _, s, _, _ in rows if not isinstance(s, str) and not s)
    print(f"\n{'刷新' if fix else '过期'}产物 {stale_total} 个 / 模板 {len(rows)} 套"
          f"（干净 {clean} 套）")
    return 0 if stale_total == 0 or fix else 1


def selftest() -> int:
    """渲染确定性自检 —— 这套方法的地基，不成立则本工具的结论全部作废。"""
    sample = sorted(DOCS.glob("*.op"))[:3]
    # 空样本要当失败报 —— 路径算错一级时 glob 落空，比对零个文件同样「全过」，
    # 那是自检里最坏的一种绿。
    if not sample:
        print(f"没有找到任何模板（DOCS={DOCS}）", file=sys.stderr)
        return 2
    with tempfile.TemporaryDirectory() as tmp_root:
        for doc in sample:
            root = pathlib.Path(tmp_root) / doc.stem
            a = render_frames(doc, root / "a", doc.stem)
            b = render_frames(doc, root / "b", doc.stem)
            if not a or len(a) != len(b):
                print(f"  {doc.stem}: 渲染失败，无法自检", file=sys.stderr)
                return 2
            bad = [x.name for x, y in zip(a, b) if not same(x, y)]
            print(f"  {doc.stem}: {'确定' if not bad else '不确定 ' + str(bad)}")
            if bad:
                return 1
    print("\n渲染确定性成立（同一文档连渲两次字节一致）")
    return 0


def main(argv: list[str]) -> int:
    fix = "--fix" in argv
    args = [a for a in argv if not a.startswith("--")]
    if "--selftest" in argv:
        return selftest()
    if not BIN.exists():
        print(f"缺少渲染二进制 {BIN}；先 cargo build --release -p op-host-desktop",
              file=sys.stderr)
        return 2
    templates = args or sorted(p.stem for p in DOCS.glob("*.op"))
    return audit(templates, fix)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

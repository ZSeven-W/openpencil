#!/usr/bin/env python3
"""把空态提示烘焙成 assets/*.png —— 必须先于 tpl1.py / tpl3.py 运行。

为什么要烤：拖图时编辑器写的是 fill[0]
(`op-editor-core/src/image_fill_upload.rs`: `fills[0] = body`)。把提示做成
fill[0] 的内嵌 PNG，用户拖图就是"直接替换提示"，零手工、随框自适应，
不依赖任何 jian 布局修复，也不用写死浮层尺寸。

用真实渲染器烤，保证与矢量版逐像素一致（实测差异 0.138%，纯抗锯齿）。
"""
import json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
BIN = os.path.join(REPO, "target", "release", "openpencil-desktop")
ASSETS = os.path.join(HERE, "assets")
sys.path.insert(0, HERE)


def bake(tag, variables, groups, w, h):
    os.makedirs(ASSETS, exist_ok=True)
    for i, kids in enumerate(groups):
        # 透明底：白卡仍由 fill[1] 的 $c-surface 驱动
        box = {"type": "frame", "id": "bake", "name": "bake", "x": 0, "y": 0,
               "width": w, "height": h, "layout": "vertical",
               "justifyContent": "center", "alignItems": "center", "fill": [],
               "children": [{"type": "group", "id": "hint", "name": "hint",
                             "width": "fill_container", "height": "fit_content",
                             "layout": "vertical", "gap": 24,
                             "alignItems": "center", "fill": [],
                             "children": kids}]}
        with tempfile.TemporaryDirectory() as td:
            src = os.path.join(td, "bake.op")
            with open(src, "w", encoding="utf-8") as fh:
                json.dump({"version": "1.0.0", "variables": variables,
                           "children": [box]}, fh, ensure_ascii=False)
            env = dict(os.environ, OPENPENCIL_RENDER_MARGIN="0")
            subprocess.run([BIN, "--render-shots", src, td, "2"],
                           check=True, capture_output=True, env=env)
            png = [f for f in os.listdir(td) if f.endswith(".png")][0]
            dst = os.path.join(ASSETS, f"{tag}-{i}.png")
            os.replace(os.path.join(td, png), dst)
            print(f"  baked {os.path.basename(dst)}  "
                  f"{os.path.getsize(dst):,d} B")


def main():
    import tpl1, tpl3
    print("bake screenshot-tutorial:")
    bake("screenshot-tutorial", tpl1.VARS,
         [tpl1.hint_children(h, s) for h, s in tpl1.HINTS],
         tpl1.SLOT_W, tpl1.SLOT_H)
    print("bake before-after:")
    bake("before-after", tpl3.VARS,
         [tpl3.hint_children(h) for h, _ in tpl3.HINTS],
         tpl3.SLOT_W, tpl3.SLOT_H)


if __name__ == "__main__":
    main()

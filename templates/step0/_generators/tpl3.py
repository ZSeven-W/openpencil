#!/usr/bin/env python3
"""before-after.op — 16:9 单帧对比图（1600×900）"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from oplib import (Ids, frame, rect, text, solid, stroke, write_doc,
                   color_vars, group, upload_disc, asset_fill,
                   PLACEHOLDER_DISC, PLACEHOLDER_ICON,
                   PLACEHOLDER_TITLE, PLACEHOLDER_SPEC)

W, H = 1600, 900
PAD_X, PAD_Y = 64, 56

VARS = color_vars({
    "c-bg":         "#F6F7F9",
    "c-surface":    "#FFFFFF",
    "c-ink":        "#101828",
    "c-muted":      "#667085",
    "c-before":     "#98A2B3",
    "c-before-soft": "#EEF0F3",
    "c-after":      "#0E9F6E",
    "c-after-soft": "#E3F5EE",
    "c-border":     "#D3D8E0",
})

ids = Ids()


# 每条说明控制在 16 字以内：三栏各约 380px 宽，22px 中文一行约 17 字，
# 超出会把句号挤到第二行变成孤字。
DIFFS = [
    ("信息层级", "标题只留一个重点，其余弱化"),
    ("对比度", "正文换成深色，小字也读得清"),
    ("留白", "模块间距拉到 32px，不再拥挤"),
]


def block(name, children, gap=20, **extra):
    node = frame(ids, name, width="fill_container", height="fit_content",
                 layout="vertical", gap=gap, fill=[], alignItems="start")
    node["children"] = children
    node.update(extra)
    return node


def badge(label, *, fill_c, text_c, size=24):
    node = frame(ids, f"徽章 · {label}", width="fit_content", height="fit_content",
                 layout="horizontal", padding=[10, 22], gap=0, cornerRadius=999,
                 alignItems="center", justifyContent="center", fill=solid(fill_c))
    node["children"] = [
        text(ids, "徽章文字", label, size, 700, text_c, width="fit_content",
             growth="auto", line_height=1.4, family="Inter")
    ]
    return node


SLOT_W, SLOT_H = 716, 420          # 实测占位框布局尺寸，仅用于烘焙提示图
# 两栏的空态提示文案 —— bake_hints.py 用它烘焙 assets/*.png
HINTS = [("拖入改版前的截图", "支持 PNG / JPG，两侧建议用同一尺寸"),
         ("拖入改版后的截图", "支持 PNG / JPG，两侧建议用同一尺寸")]
HINT_ASSET = "before-after-{i}.png"


def hint_children(hint):
    """空态提示的矢量构造 —— 只被 bake_hints.py 用来烘焙成 PNG。

    两栏都用中性灰（不再分别取 before/after 的色）：占位是控件不是品牌表达，
    Before/After 的色彩区分由上方徽章承担。
    """
    return [
        upload_disc(ids, "上传图标", 88, PLACEHOLDER_DISC, 40,
                    PLACEHOLDER_ICON),
        text(ids, "占位提示", hint, 28, 600, PLACEHOLDER_TITLE, align="center",
             line_height=1.4),
        text(ids, "占位规格", "支持 PNG / JPG，两侧建议用同一尺寸", 22, 400,
             PLACEHOLDER_SPEC, align="center"),
    ]


def shot_slot(kind, idx):
    """截图占位框 —— 空态提示是 fill[0] 的内嵌 PNG，没有子节点。"""
    return frame(ids, f"截图占位框 · {kind}", width="fill_container",
                 height="fill_container", cornerRadius=20,
                 fill=[asset_fill(HINT_ASSET.format(i=idx), "fit"),
                       solid("$c-surface")[0]],
                 stroke=stroke("$c-border", 3), clipContent=True)


def column(label, kind, tint, accent, idx):
    head = frame(ids, f"{kind}标签行", width="fill_container",
                 height="fit_content", layout="horizontal", gap=16,
                 alignItems="center", fill=[])
    head["children"] = [
        badge(label, fill_c=tint, text_c=accent),
        text(ids, "标签说明", kind, 26, 500, "$c-muted", width="fit_content",
             growth="auto", line_height=1.4),
    ]
    node = frame(ids, f"对比栏 · {kind}", width="fill_container",
                 height="fill_container", layout="vertical", gap=18, fill=[],
                 alignItems="start")
    node["children"] = [head, shot_slot(kind, idx)]
    return node


def diff_item(no, title, desc):
    num = frame(ids, f"差异序号 {no}", width=44, height=44, layout="horizontal",
                alignItems="center", justifyContent="center", cornerRadius=22,
                fill=solid("$c-after-soft"))
    num["children"] = [
        text(ids, "序号", str(no), 24, 700, "$c-after", width="fit_content",
             growth="auto", line_height=1.4, family="Inter")
    ]
    body = block("差异文案", [
        text(ids, "差异标题", title, 26, 600, "$c-ink", line_height=1.4),
        text(ids, "差异说明", desc, 22, 400, "$c-muted", line_height=1.6),
    ], gap=6)
    node = frame(ids, f"差异点 {no}", width="fill_container",
                 height="fit_content", layout="horizontal", gap=18,
                 alignItems="start", fill=[])
    node["children"] = [num, body]
    return node


def build():
    header = frame(ids, "页头", width="fill_container", height="fit_content",
                   layout="horizontal", justifyContent="space_between",
                   alignItems="center", gap=24, fill=[])
    header["children"] = [
        block("页头文案", [
            text(ids, "页头标题", "改版前后对比", 40, 700, "$c-ink",
                 line_height=1.3),
            text(ids, "页头副标题", "同一个页面，只改了三件事。", 24, 400,
                 "$c-muted", line_height=1.5),
        ], gap=10, width="fit_content"),
        badge("BEFORE / AFTER", fill_c="$c-before-soft", text_c="$c-muted",
              size=22),
    ]

    compare = frame(ids, "对比区", width="fill_container",
                    height="fill_container", layout="horizontal", gap=40,
                    alignItems="start", fill=[])
    compare["children"] = [
        column("BEFORE", "改版前", "$c-before-soft", "$c-before", 0),
        column("AFTER", "改版后", "$c-after-soft", "$c-after", 1),
    ]

    diffs = frame(ids, "差异点卡", width="fill_container", height="fit_content",
                  layout="horizontal", padding=[28, 32], gap=40,
                  alignItems="start", cornerRadius=20,
                  fill=solid("$c-surface"))
    diffs["children"] = [diff_item(i + 1, t, d) for i, (t, d) in enumerate(DIFFS)]

    root = frame(ids, "改版前后对比图", x=0, y=0, width=W, height=H,
                 layout="vertical", padding=[PAD_Y, PAD_X], gap=32,
                 alignItems="start", fill=solid("$c-bg"), clipContent=True)
    root["children"] = [header, compare, diffs]

    write_doc(sys.argv[1], VARS, [root], "改版前后对比图 · 16:9 模板")


if __name__ == "__main__":
    build()

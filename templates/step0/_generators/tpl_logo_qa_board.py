#!/usr/bin/env python3
"""logo-qa-board.op — single-board structural fusion review template.

This board is a decision surface, not a logo presentation poster. The left rail
keeps the candidate, black/reverse variants, and native-size survival evidence
visible at once. The right side is a 2x2 matrix in which every fusion predicate
has its own check, question, and observable evidence.

The sample mark is generated from one original polygon. Its outer contour reads
as a G, while the same inward cross-stroke reads as an entry route. The sample is
only there to demonstrate how evidence belongs on the board; every label and
shape remains editable in OpenPencil.

Design constraints:
  - 1600x1000 single artboard, with 54 px outer margins.
  - Marks are evaluated only in black and reverse; accent color annotates review
    status and never participates in the mark's meaning.
  - The four predicate cards use one component path and equal visual weight.
  - The minimum-size row shows 64, 32, and 20 px from the same source geometry.
  - No gradients, shadows, app-icon mockups, decorative logos, or bitmap assets.
  - All colors are explicit, locally derived hex values exposed as variables.
  - CJK text uses Noto Sans SC with non-negative tracking below 48 px.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from oplib import Ids, color_vars, frame, path, rect, solid, text, write_doc


W, H = 1600, 1000
EDGE_X, EDGE_Y = 54, 42
BODY_GAP = 24
LEFT_W = 522

CJK = "Noto Sans SC"
LATIN = "Inter"

ids = Ids()

VARS = color_vars({
    "c-bg":          "#ECECE7",
    "c-paper":       "#FAFAF5",
    "c-panel":       "#F3F3ED",
    "c-ink":         "#171719",
    "c-dark":        "#242426",
    "c-muted":       "#595A5E",
    "c-faint":       "#74757A",
    "c-line":        "#C8C8C0",
    "c-white":       "#FFFFFF",
    "c-accent":      "#5A3FC0",
    "c-accent-soft": "#E6E0FA",
})


# Exact acceptance language stays separate from the sample evidence so users can
# replace the candidate without weakening the predicate itself.
TESTS = [
    (
        "01",
        "同一结构双职",
        "同一笔画是否同时承担主读法与第二层含义？",
        "G 的横笔补全字母，也形成进入路线；\n没有新增图标层。",
    ),
    (
        "02",
        "删除任一语义会改同一几何",
        "拿掉任一层含义时，\n是否必须重画同一处结构？",
        "去掉横笔，G 不再成立；\n抹去箭头端点，路线也同时消失。",
    ),
    (
        "03",
        "单一轮廓 / 节奏",
        "缩小或反白后，是否仍被读成一枚连续标记？",
        "外框、横笔与端点共用一套 18% 模数，20 px 仍保持整体。",
    ),
    (
        "04",
        "次义为发现而非贴图",
        "第二层含义是否在主读法之后自然浮现？",
        "先读到 G，再从横笔发现进入方向；\n旁边没有独立箭头。",
    ),
]


def col(name, children, *, gap=14, width="fill_container",
        height="fit_content", align="start", **props):
    node = frame(ids, name, width=width, height=height, layout="vertical",
                 gap=gap, alignItems=align, fill=[], **props)
    node["children"] = children
    return node


def row(name, children, *, gap=14, width="fill_container",
        height="fit_content", align="center", justify="start", **props):
    node = frame(ids, name, width=width, height=height, layout="horizontal",
                 gap=gap, alignItems=align, justifyContent=justify, fill=[],
                 **props)
    node["children"] = children
    return node


def label_chip(content, *, bg="$c-accent-soft", fg="$c-accent"):
    node = frame(ids, f"标签 · {content}", width="fit_content",
                 height="fit_content", layout="horizontal", padding=[7, 14],
                 alignItems="center", justifyContent="center", fill=solid(bg))
    node["children"] = [
        text(ids, "标签文字", content, 17, 700, fg, family=CJK,
             width="fit_content", growth="auto", line_height=1.35),
    ]
    return node


def g_mark(size, color, name):
    """Return one concave polygon: G contour and inward route are inseparable."""
    d = ("M 0 0 H 100 V 18 H 18 V 82 H 82 V 58 H 58 "
         "L 43 49 L 58 40 H 100 V 100 H 0 Z")
    return path(ids, name, d, width=size, height=size, fill=solid(color))


def lockup(mark_size, mark_color, word_color, *, compact=False):
    word_size = 42 if compact else 54
    sub_size = 17 if compact else 20
    copy = col("字标", [
        text(ids, "英文名称", "GOWAY", word_size, 750, word_color,
             family=LATIN, width="fit_content", growth="auto",
             line_height=1.0, spacing=-1.2),
        text(ids, "品牌说明", "路径协作工具 · 候选 02", sub_size, 500,
             "$c-faint" if mark_color == "$c-ink" else "$c-white",
             family=CJK, width="fit_content", growth="auto",
             line_height=1.45),
    ], gap=8, width="fit_content")
    return row("组合标", [g_mark(mark_size, mark_color, "G 路径标记"), copy],
               gap=22 if compact else 28, width="fit_content", align="center")


def header():
    title = col("标题组", [
        row("标题眉题", [
            label_chip("LOGO 结构融合 · 评审版"),
            text(ids, "版本", "BOARD 01 / 2026", 16, 650, "$c-muted",
                 family=LATIN, width="fit_content", growth="auto",
                 line_height=1.3),
        ], gap=14, width="fit_content"),
        text(ids, "板标题", "一张板，判定融合是否真的成立", 39, 700,
             "$c-ink", family=CJK, width="fit_content", growth="auto",
             line_height=1.25),
    ], gap=12, width="fit_content")

    meta = col("评审元数据", [
        text(ids, "阶段", "STRUCTURE ONLY · NOT SCREENED", 17, 750, "$c-ink",
             family=LATIN, width="fit_content", growth="auto",
             line_height=1.3),
        text(ids, "评审说明", "黑白先行 · 色彩仅作批注", 19, 500,
             "$c-muted", family=CJK, width="fit_content", growth="auto",
             line_height=1.45),
    ], gap=6, width="fit_content", align="end")

    return row("页头", [title, meta], gap=24, align="end",
               justify="space_between")


def main_specimen():
    mark_area = frame(ids, "主候选", width="fill_container", height=282,
                      layout="vertical", padding=[22, 24], gap=22,
                      justifyContent="space_between", fill=solid("$c-paper"))
    mark_area["stroke"] = {"thickness": 2, "fill": solid("$c-line")}
    mark_area["children"] = [
        row("主候选标签", [
            text(ids, "候选标签", "A / 主候选", 18, 650, "$c-muted",
                 family=CJK, width="fit_content", growth="auto",
                 line_height=1.4),
            label_chip("待评", bg="$c-panel", fg="$c-muted"),
        ], justify="space_between"),
        lockup(126, "$c-ink", "$c-ink"),
        text(ids, "替换提示", "示例只演示证据结构；请替换为你的候选方案。",
             18, 400, "$c-muted", family=CJK, line_height=1.55),
    ]
    return mark_area


def variant_card(title, *, reverse):
    bg = "$c-dark" if reverse else "$c-paper"
    mark = "$c-white" if reverse else "$c-ink"
    fg = "$c-white" if reverse else "$c-muted"
    card = frame(ids, f"{title}视图", width="fill_container", height=164,
                 layout="vertical", padding=[16, 18], gap=12,
                 justifyContent="space_between", alignItems="start",
                 fill=solid(bg))
    if not reverse:
        card["stroke"] = {"thickness": 2, "fill": solid("$c-line")}
    card["children"] = [
        text(ids, "视图名称", title, 17, 650, fg, family=CJK,
             width="fit_content", growth="auto", line_height=1.4),
        row("标记居中", [g_mark(78, mark, f"{title}标记")],
            justify="center", align="center"),
    ]
    return card


def size_sample(size):
    sample = frame(ids, f"{size}px 样本", width="fill_container", height=102,
                   layout="vertical", padding=[10, 6], gap=9,
                   justifyContent="end", alignItems="center", fill=[])
    sample["children"] = [
        g_mark(size, "$c-ink", f"{size}px 标记"),
        text(ids, "尺寸", f"{size} PX", 14, 650, "$c-muted",
             family=LATIN, width="fit_content", growth="auto",
             line_height=1.2),
    ]
    return sample


def minimum_panel():
    panel = frame(ids, "最小尺寸验证", width="fill_container",
                  height="fill_container", layout="vertical",
                  padding=[18, 20], gap=14, justifyContent="space_between",
                  fill=solid("$c-panel"))
    panel["children"] = [
        row("最小尺寸标题", [
            text(ids, "行标题", "最小尺寸存活", 21, 700, "$c-ink",
                 family=CJK, width="fit_content", growth="auto",
                 line_height=1.35),
            text(ids, "判定尺寸", "TARGET 20 PX", 14, 700, "$c-accent",
                 family=LATIN, width="fit_content", growth="auto",
                 line_height=1.3),
        ], justify="space_between"),
        row("尺寸序列", [size_sample(64), size_sample(32), size_sample(20)],
            gap=8, height=108, align="end"),
        text(ids, "尺寸结论", "20 px 仍读作 G，开口与箭头端点不黏连。",
             17, 500, "$c-muted", family=CJK, line_height=1.5),
    ]
    return panel


def evidence_rail():
    panel = frame(ids, "候选证据轨", width=LEFT_W, height="fill_container",
                  layout="vertical", padding=[22, 22], gap=18,
                  fill=solid("$c-bg"))
    panel["stroke"] = {"thickness": 2, "fill": solid("$c-line")}
    panel["children"] = [
        main_specimen(),
        row("黑白双视图", [
            variant_card("纯黑", reverse=False),
            variant_card("反白", reverse=True),
        ], gap=14, height=164, align="stretch"),
        minimum_panel(),
    ]
    return panel


def pending_mark():
    box = frame(ids, "待判框", width=38, height=38, layout="horizontal",
                alignItems="center", justifyContent="center",
                fill=solid("$c-paper"))
    box["stroke"] = {"thickness": 2, "fill": solid("$c-accent")}
    box["children"] = []
    return box


def test_card(no, title, question, evidence):
    card = frame(ids, f"融合测试 {no}", width="fill_container",
                 height="fill_container", layout="vertical",
                 padding=[24, 26], gap=16, fill=solid("$c-paper"))
    card["stroke"] = {"thickness": 2, "fill": solid("$c-line")}
    card["children"] = [
        row("测试状态", [
            pending_mark(),
            text(ids, "测试编号", f"TEST {no}", 15, 750, "$c-accent",
                 family=LATIN, width="fit_content", growth="auto",
                 line_height=1.3),
        ], justify="space_between"),
        text(ids, "测试标题", title, 29, 700, "$c-ink", family=CJK,
             line_height=1.35),
        text(ids, "测试提问", question, 20, 500, "$c-muted", family=CJK,
             line_height=1.55),
        rect(ids, "证据分隔", width="fill_container", height=2,
             fill=solid("$c-line")),
        col("证据说明", [
            text(ids, "证据眉题", "示例依据", 15, 700, "$c-accent",
                 family=CJK, width="fit_content", growth="auto",
                 line_height=1.35),
            text(ids, "证据正文", evidence, 19, 400, "$c-ink", family=CJK,
                 line_height=1.55),
        ], gap=8),
        row("测试结论", [
            rect(ids, "结论短线", width=26, height=4,
                 fill=solid("$c-accent")),
            text(ids, "结论文字", "待判 · 记录依据后勾选", 16, 650,
                 "$c-muted", family=CJK, width="fit_content",
                 growth="auto", line_height=1.4),
        ], gap=10, align="center"),
    ]
    return card


def test_matrix():
    cards = [test_card(*item) for item in TESTS]
    matrix = col("融合测试矩阵", [
        row("测试第一行", cards[:2], gap=18, height="fill_container",
            align="stretch"),
        row("测试第二行", cards[2:], gap=18, height="fill_container",
            align="stretch"),
    ], gap=18, height="fill_container")
    return matrix


def build():
    body = row("评审主体", [evidence_rail(), test_matrix()], gap=BODY_GAP,
               height="fill_container", align="stretch")

    root = frame(ids, "LOGO 融合测试评审板", x=0, y=0, width=W, height=H,
                 layout="vertical", padding=[EDGE_Y, EDGE_X], gap=24,
                 alignItems="start", fill=solid("$c-bg"), clipContent=True)
    root["children"] = [header(), body]
    write_doc(sys.argv[1], VARS, [root], "LOGO 融合测试 · 评审板",
              compact=True)


# Contrast ratios, computed from the literal palette:
#   c-ink on c-paper       17.10    c-muted on c-paper      6.58
#   c-white on c-dark     15.49    c-accent on c-paper     6.90
#   c-muted on c-bg        5.81    c-accent on accent-soft 5.64
# The lowest text pair is above AA body-text requirements. The mark itself is
# evaluated only at 17.10:1 in black and 15.49:1 in reverse.

if __name__ == "__main__":
    build()

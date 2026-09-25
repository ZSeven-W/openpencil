#!/usr/bin/env python3
"""blank-vs-example-contrast.op — 两种创作起点对比长图（1080×N 竖版）

Studio 首页「信息图 · 对比」的示例就是这一份：「对比『从空白开始』与『从示例
开始』两种创作方式。按起点、修改方式、自由度、适用场景排成对照表，最后给出各
自适合谁，不虚构效率数据。」空输入框点开始设计时它作为即时初稿载入，所以版面
与示例逐项对应：

    深色页头 → 两张定义卡 → 四行对照表（起点 / 修改方式 / 自由度 / 适用场景）
    → 各自适合谁 → 页脚

### 这张图的立场：没有赢家

示例要的是「选对起点」，不是「示例更好」。所以两侧**同权**：各有一个颜色
（石板灰 A、琥珀 B），明度相当、面积相同，表格里每格一句能验证的事实，
**不出现任何百分比、倍数、耗时** —— 示例明令不虚构效率数据。

### 表格的做法

同 concept-contrast-infographic：维度名横跨整宽，下面两栏各一句，中间一条
height: fill_container 的竖线靠外层 stretch 拿高度，哪一栏长线就跟着长。两
栏写死同宽（CELL_W），四行才对得齐。

硬契约同 tpl_infographic_data：内容距边缘 80、配色全走 color_vars、CJK 行
高 1.2/1.3/1.7、CJK 负字距不超过 -0.02em、根帧写 x/y、ROOT_H 量出来烤回。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from oplib import (Ids, color_vars, frame, icon_font, rect, solid, text,
                   write_doc)

ids = Ids()

VARS = color_vars({
    "c-bg":          "#F8F6F1",
    "c-surface":     "#FFFFFF",
    "c-band":        "#1F2430",
    "c-band-muted":  "#B3B8C4",
    "c-ink":         "#1C2029",
    "c-muted":       "#616775",
    # A 侧：从空白开始。石板灰，和 B 侧明度相当 —— 两边同权。
    "c-side-a":      "#3F4A5C",
    "c-side-a-soft": "#E6E9EF",
    # B 侧：从示例开始。琥珀，也是全图的强调色（短线、胶囊）。
    "c-accent":      "#B86E0B",
    "c-accent-deep": "#8A5207",
    "c-accent-soft": "#FBEBD3",
    "c-border":      "#E7E3DA",
    "c-rule":        "#CFC8BB",
})

CJK = "Noto Sans SC"
NUM = "Inter"

W = 1080
EDGE = 80
INNER = W - EDGE * 2
LH_DISPLAY, LH_HEAD, LH_BODY = 1.2, 1.3, 1.7

CARD_PAD = 34
RULE_W = 2
CELL_GAP = 26
CELL_W = (INNER - CARD_PAD * 2 - RULE_W - CELL_GAP * 2) // 2

# 量出来的根高（根设 fit_content 渲一次读 PNG 高度，再烤回来）。
ROOT_H = 3370

SIDE_A = ("从空白开始", "file-plus", "$c-side-a", "$c-side-a-soft",
          "画布上什么都没有，从第一笔开始，结构、配色和文字都由你来定。")
SIDE_B = ("从示例开始", "layout-template", "$c-accent-deep", "$c-accent-soft",
          "先载入一份做好的示例，在它的版式上换成你自己的内容。")

# (维度, A 侧一句话, B 侧一句话)
# Cells are 399px wide at 28px (14 CJK per line), so every two-line cell
# carries its own break: an automatic wrap here strands 「。」 or 「颜色。」
# on a line of its own.
DIMENSIONS = [
    ("起点", "一张空白画布", "一份完整的示例稿"),
    ("修改方式", "从零搭结构，\n一层层往上加内容。",
     "在现成的版式上，\n替换文字、图片和颜色。"),
    ("自由度", "最高：版式、节奏、\n配色都由你决定。",
     "先沿用示例的骨架，\n需要时也能再改结构。"),
    ("适用场景", "心里已有清楚的样子，\n或要做独一无二的版式。",
     "第一次做这类内容，\n或想先看到一版再改。"),
]

FITS = [
    ("适合从空白开始", SIDE_A, [
        "已经想好要做成什么样",
        "有固定的品牌规范要照着来",
        "享受从零搭起来的过程",
    ]),
    ("适合从示例开始", SIDE_B, [
        "第一次做这类内容",
        "想先有一版能看的，再慢慢改",
        "时间紧，想少做几个决定",
    ]),
]


def band(name, *, fill, pad, gap, children, align="start"):
    node = frame(ids, name, width="fill_container", height="fit_content",
                 layout="vertical", padding=pad, gap=gap, alignItems=align,
                 fill=fill)
    node["children"] = children
    return node


def col(name, children, *, gap=16, width="fill_container", align="start",
        height="fit_content", **props):
    node = frame(ids, name, width=width, height=height, layout="vertical",
                 gap=gap, alignItems=align, fill=[], **props)
    node["children"] = children
    return node


def row(name, children, *, gap=24, align="center", width="fill_container",
        **props):
    node = frame(ids, name, width=width, height="fit_content",
                 layout="horizontal", gap=gap, alignItems=align, fill=[],
                 **props)
    node["children"] = children
    return node


def label(name, content, size, weight, color, *, family=CJK, lh=1.4):
    return text(ids, name, content, size, weight, color, family=family,
                width="fit_content", growth="auto", line_height=lh)


def chip(content, *, bg, fg, size=24, icon=None):
    node = frame(ids, "胶囊", width="fit_content", height="fit_content",
                 layout="horizontal", padding=[10, 22], gap=10,
                 cornerRadius=999, alignItems="center",
                 justifyContent="center", fill=solid(bg))
    node["children"] = []
    if icon:
        node["children"].append(icon_font(ids, "胶囊图标", icon, size, fg))
    node["children"].append(label("胶囊文字", content, size, 600, fg))
    return node


def icon_tile(glyph, fg, bg, size=64):
    node = frame(ids, "图标底", width=size, height=size, layout="horizontal",
                 alignItems="center", justifyContent="center",
                 cornerRadius=18, fill=solid(bg))
    node["children"] = [icon_font(ids, "图标", glyph, round(size * 0.5), fg)]
    return node


def section_head(title, note):
    return col("区块头", [
        rect(ids, "强调短线", width=72, height=8, cornerRadius=999,
             fill=solid("$c-accent")),
        text(ids, "区块标题", title, 46, 700, "$c-ink", family=CJK,
             line_height=LH_HEAD),
        text(ids, "区块说明", note, 27, 400, "$c-muted", family=CJK,
             line_height=LH_BODY),
    ], gap=16)


def white_card(name, children, *, gap=22, pad=(CARD_PAD, CARD_PAD), **props):
    card = col(name, children, gap=gap, padding=list(pad), cornerRadius=24,
               **props)
    card["fill"] = solid("$c-surface")
    card["stroke"] = {"thickness": 2, "fill": solid("$c-border")}
    return card


# ------------------------------------------------------------------ 01 页头
def header():
    return band("01 页头", fill=solid("$c-band"), pad=[76, EDGE, 68, EDGE],
                gap=26, children=[
        chip("两种创作起点 · 对照看", bg="$c-accent", fg="$c-surface",
             icon="columns-2"),
        text(ids, "主标题", "选对起点，\n让创作更顺手", 76, 700, "$c-surface",
             family=CJK, line_height=LH_DISPLAY, spacing=-1.4),
        text(ids, "副标题",
             "从空白开始和从示例开始没有高下之分，只是适合的时候不一样。",
             28, 400, "$c-band-muted", family=CJK, line_height=LH_BODY),
    ])


# ------------------------------------------------------------------ 02 定义
def define_card(side, tag):
    name, glyph, fg, bg, principle = side
    head = row("定义卡头", [
        icon_tile(glyph, fg, bg),
        col("名称组", [
            label("方式标记", tag, 24, 600, fg, family=NUM),
            label("方式名", name, 38, 700, "$c-ink"),
        ], gap=2),
    ], gap=20)
    return white_card(f"定义卡 {name}", [
        head,
        text(ids, "做法", principle, 28, 400, "$c-muted", family=CJK,
             line_height=LH_BODY),
    ], gap=18, pad=(32, CARD_PAD))


def definitions():
    return band("02 定义", fill=[], pad=[64, EDGE, 0, EDGE], gap=32,
                children=[
        section_head("先说清是哪两种", "同样是做一张图，差别只在第一步。"),
        col("定义组", [define_card(SIDE_A, "A"), define_card(SIDE_B, "B")],
            gap=18),
    ])


# ------------------------------------------------------------------ 03 对照
def dimension_block(title, left, right, *, last=False):
    cells = row("两栏", [
        text(ids, "A 侧", left, 28, 400, "$c-ink", family=CJK, width=CELL_W,
             line_height=LH_BODY),
        rect(ids, "竖分隔", width=RULE_W, height="fill_container",
             fill=solid("$c-rule")),
        text(ids, "B 侧", right, 28, 400, "$c-ink", family=CJK, width=CELL_W,
             line_height=LH_BODY),
    ], gap=CELL_GAP, align="stretch")
    kids = [
        row("维度名行", [
            rect(ids, "维度点", width=10, height=10, cornerRadius=5,
                 fill=solid("$c-muted")),
            label("维度名", title, 26, 700, "$c-muted"),
        ], gap=12),
        cells,
    ]
    if not last:
        kids.append(rect(ids, "维度分割线", width="fill_container", height=2,
                         fill=solid("$c-border")))
    return col(f"维度 {title}", kids, gap=16)


def side_head(side, tag):
    name, glyph, fg, _bg, _ = side
    return row(f"表头 {name}", [
        icon_font(ids, "表头图标", glyph, 30, fg),
        label("表头名", f"{tag} · {name}", 30, 700, fg),
    ], gap=12, width=CELL_W)


def table():
    header_row = row("表头", [
        side_head(SIDE_A, "A"),
        rect(ids, "表头竖线占位", width=RULE_W, height=2, fill=[]),
        side_head(SIDE_B, "B"),
    ], gap=CELL_GAP)
    blocks = [header_row,
              rect(ids, "表头下线", width="fill_container", height=2,
                   fill=solid("$c-rule"))]
    for index, (title, left, right) in enumerate(DIMENSIONS):
        blocks.append(dimension_block(title, left, right,
                                      last=index == len(DIMENSIONS) - 1))
    return band("03 对照", fill=[], pad=[68, EDGE, 0, EDGE], gap=32,
                children=[
        section_head("四个维度，一格一句",
                     "左边是从空白开始，右边是从示例开始。只写做法，不比快慢。"),
        white_card("对照表", blocks, gap=22),
    ])


# ------------------------------------------------------------------ 04 适合谁
def fit_card(title, side, points):
    _name, glyph, fg, bg, _ = side
    items = [row("适合项", [
        icon_font(ids, "对勾", "check", 28, fg),
        text(ids, "适合描述", point, 28, 500, "$c-ink", family=CJK,
             line_height=1.5),
    ], gap=14, align="center") for point in points]
    card = col(f"适合 {title}", [
        row("适合头", [
            icon_font(ids, "适合图标", glyph, 34, fg),
            label("适合标题", title, 34, 700, fg),
        ], gap=14),
        *items,
    ], gap=20, padding=[34, CARD_PAD], cornerRadius=24)
    card["fill"] = solid(bg)
    return card


def fits():
    return band("04 适合谁", fill=[], pad=[68, EDGE, 72, EDGE], gap=32,
                children=[
        section_head("各自适合谁", "看看自己现在是哪一种情况，再决定从哪儿开始。"),
        col("适合组", [fit_card(*entry) for entry in FITS], gap=18),
    ])


# ------------------------------------------------------------------ 05 页脚
def footer():
    return band("05 页脚", fill=solid("$c-band"), pad=[48, EDGE], gap=16,
                children=[
        row("提示行", [
            icon_font(ids, "切换", "repeat", 28, "$c-band-muted"),
            text(ids, "提示", "两种方式随时能换：从示例开始，也可以清空了重来。",
                 26, 400, "$c-band-muted", family=CJK, line_height=1.6),
        ], gap=12),
        row("署名行", [
            label("账号名", "@ 你的账号名", 26, 600, "$c-surface"),
            label("更新说明", "每周一张讲清一个选择", 24, 400,
                  "$c-band-muted"),
        ], gap=16),
    ])


def build():
    page = frame(ids, "两种创作起点对比长图", width=W, height=ROOT_H,
                 layout="vertical", gap=0, fill=solid("$c-bg"),
                 clipContent=True)
    page["children"] = [header(), definitions(), table(), fits(), footer()]
    page["x"], page["y"] = 0, 0
    return [page]


# 对比度（WCAG 相对亮度比，op-design-lint 门槛 2.0）：
#   c-surface     on c-band        15.8    c-band-muted on c-band         8.0
#   c-ink         on c-bg          15.6    c-muted      on c-surface      5.6
#   c-side-a      on c-surface      9.1    c-side-a     on c-side-a-soft  7.4
#   c-accent-deep on c-surface      6.4    c-accent-deep on c-accent-soft 5.4
#   c-surface     on c-accent       3.6 —— 只承载 24px 粗体胶囊字（AA 大字 3.0）。

if __name__ == "__main__":
    write_doc(sys.argv[1], VARS, build(), "两种创作起点对比长图")

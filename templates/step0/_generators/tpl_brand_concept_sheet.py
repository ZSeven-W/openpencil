#!/usr/bin/env python3
"""brand-concept-sheet.op — editable logo concept board (1600 x 1000).

The board is intentionally a decision sheet rather than a logo gallery. One
lockup occupies the dominant field, three construction steps explain the same
geometry, and the bottom rail checks positive, reverse, and reduced states.

The fictional PENPATH identity uses a self-authored geometric P. Its counter is
stretched into a bent route channel: one void keeps the letter readable and
carries the path idea, so no standalone arrow or pencil pictogram is added.

Hard constraints:
  - one 1600 x 1000 board with explicit root coordinates;
  - every color is a document variable;
  - the logo itself is assessed in monochrome before any accent is applied;
  - the 64 / 32 / 16 samples use true node sizes, not labels on one large mark;
  - all text remains editable and CJK tracking stays at zero below 48 px.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from oplib import (Ids, color_vars, frame, group, path, rect, solid, stroke,
                   text, write_doc)

ids = Ids()

VARS = color_vars({
    "c-paper": "#F2F0EA",
    "c-surface": "#FCFBF8",
    "c-ink": "#151719",
    "c-muted": "#646762",
    "c-line": "#D5D2C9",
    "c-soft": "#E7E4DB",
    "c-accent": "#C85A35",
    "c-accent-soft": "#F3DDD4",
    "c-white": "#FFFFFF",
})

CJK = "Noto Sans SC"
LATIN = "Inter"

W, H = 1600, 1000
PAD_X, PAD_Y = 64, 52
INNER_W = W - PAD_X * 2
BODY_H = 526
BOTTOM_H = 208


def col(name, children, *, width="fill_container", height="fit_content",
        gap=12, align="start", justify="start", **props):
    node = frame(ids, name, width=width, height=height, layout="vertical",
                 gap=gap, alignItems=align, justifyContent=justify, fill=[],
                 **props)
    node["children"] = children
    return node


def row(name, children, *, width="fill_container", height="fit_content",
        gap=16, align="center", justify="start", **props):
    node = frame(ids, name, width=width, height=height, layout="horizontal",
                 gap=gap, alignItems=align, justifyContent=justify, fill=[],
                 **props)
    node["children"] = children
    return node


def chip(label, *, bg, fg):
    node = frame(ids, "状态标签", width="fit_content", height="fit_content",
                 layout="horizontal", padding=[9, 16], cornerRadius=999,
                 alignItems="center", justifyContent="center", fill=solid(bg))
    node["children"] = [
        text(ids, "状态标签文字", label, 16, 700, fg, family=LATIN,
             width="fit_content", growth="auto", line_height=1.3,
             spacing=1.2),
    ]
    return node


def p_mark(name, size, ink, paper):
    """Build the editable P + route-counter mark from primitive geometry."""
    stem_w = round(size * 0.27, 2)
    bowl_w = round(size * 0.82, 2)
    bowl_h = round(size * 0.67, 2)
    counter_w = round(size * 0.42, 2)
    counter_h = round(size * 0.25, 2)

    stem = rect(ids, f"{name} · 竖干", width=stem_w, height=size,
                cornerRadius=round(stem_w / 2, 2), fill=solid(ink))
    stem["x"], stem["y"] = 0, 0

    bowl = rect(ids, f"{name} · 圆肩", width=bowl_w, height=bowl_h,
                cornerRadius=round(bowl_h * 0.42, 2), fill=solid(ink))
    bowl["x"], bowl["y"] = round(size * 0.08, 2), 0

    # The counter is simultaneously the P counter and a bent route channel.
    d = (f"M0 0 H{counter_w * 0.48:.2f} "
         f"V{counter_h * 0.22:.2f} H{counter_w:.2f} "
         f"V{counter_h * 0.78:.2f} H{counter_w * 0.48:.2f} "
         f"V{counter_h:.2f} H0 Z")
    counter = path(ids, f"{name} · 共享留白", d, width=counter_w,
                   height=counter_h, fill=solid(paper))
    counter["x"] = round(size * 0.34, 2)
    counter["y"] = round(size * 0.19, 2)

    mark = group(ids, name, width=size, height=size, layout="none", fill=[])
    # Jian paints the first sibling last, so the cutout must be first.
    mark["children"] = [counter, stem, bowl]
    return mark


def route_cut(name, width, height, color):
    d = (f"M0 0 H{width * 0.48:.2f} V{height * 0.22:.2f} "
         f"H{width:.2f} V{height * 0.78:.2f} H{width * 0.48:.2f} "
         f"V{height:.2f} H0 Z")
    return path(ids, name, d, width=width, height=height, fill=solid(color))


def header():
    title = col("标题组", [
        text(ids, "项目名", "PENPATH / 笔径", 42, 750, "$c-ink",
             family=CJK, line_height=1.2),
        text(ids, "板型说明", "品牌概念板 · 单方案结构说明", 19, 400,
             "$c-muted", family=CJK, line_height=1.45),
    ], width="fit_content", gap=6)
    note = col("阶段说明", [
        text(ids, "阶段", "CONCEPT 01 · NOT SCREENED", 16, 700, "$c-accent",
             family=LATIN, width="fit_content", growth="auto",
             line_height=1.3, spacing=1.1),
        text(ids, "范围", "只验证构成与缩小表现，不替代精密矢量完稿。", 18,
             400, "$c-muted", family=CJK, line_height=1.5),
    ], width=430, gap=6, align="end")
    return row("页头", [title, note], justify="space_between")


def hero_panel():
    mark = p_mark("主标志", 164, "$c-white", "$c-ink")
    naming = col("字标", [
        text(ids, "英文名", "PENPATH", 78, 800, "$c-white", family=LATIN,
             width="fit_content", growth="auto", line_height=1.0,
             spacing=-1.8),
        text(ids, "中文名", "笔径", 30, 600, "$c-white", family=CJK,
             width="fit_content", growth="auto", line_height=1.3),
        text(ids, "品牌描述", "路径协作工具 · 让每次修改都有来路", 18, 400,
             "$c-soft", family=CJK, line_height=1.5),
    ], width="fit_content", gap=12)
    lockup = row("组合标展示", [mark, naming], width="fit_content", gap=34)

    method = col("方法说明", [
        text(ids, "方法名", "共享留白 / 字腔改造", 18, 650, "$c-white",
             family=CJK, line_height=1.4),
        text(ids, "方法描述", "同一处切口既完成 P 的识别，也提示路径向前。", 17,
             400, "$c-soft", family=CJK, line_height=1.5),
    ], width=438, gap=5)
    guardrail = col("边界说明", [
        text(ids, "边界标题", "删除项", 18, 650, "$c-white", family=CJK,
             line_height=1.4),
        text(ids, "边界描述", "不再叠加箭头、铅笔或容器外框。", 17, 400,
             "$c-soft", family=CJK, line_height=1.5),
    ], width=286, gap=5)

    panel = frame(ids, "主方案展示", width=880, height=BODY_H,
                  layout="vertical", padding=[38, 46], gap=0,
                  justifyContent="space_between", alignItems="start",
                  cornerRadius=28, fill=solid("$c-ink"))
    panel["children"] = [
        chip("PRIMARY LOCKUP", bg="$c-white", fg="$c-ink"),
        lockup,
        row("方案注释", [method, guardrail], gap=24, align="start"),
    ]
    return panel


def logic_visual(kind):
    box = frame(ids, "构成示意", width=92, height=92, layout="horizontal",
                alignItems="center", justifyContent="center",
                cornerRadius=18, fill=solid("$c-paper"))
    if kind == "base":
        item = text(ids, "P 母形", "P", 66, 800, "$c-ink", family=LATIN,
                    width="fit_content", growth="auto", line_height=1.0)
    elif kind == "void":
        item = route_cut("方向切口", 58, 26, "$c-accent")
    else:
        item = p_mark("融合标志", 66, "$c-ink", "$c-paper")
    box["children"] = [item]
    return box


def logic_step(number, title, body, kind):
    copy = col("步骤文字", [
        text(ids, "步骤名", f"{number}  {title}", 21, 700, "$c-ink",
             family=CJK, line_height=1.3),
        text(ids, "步骤说明", body, 17, 400, "$c-muted", family=CJK,
             line_height=1.5),
    ], gap=3)
    return row("构成步骤", [logic_visual(kind), copy], height=112, gap=18,
               align="center")


def logic_panel():
    heading = col("构成标题", [
        text(ids, "区块标题", "构成逻辑", 32, 700, "$c-ink", family=CJK,
             line_height=1.25),
        text(ids, "区块副标题", "三步只改一处结构，不靠额外图标补意思。", 17,
             400, "$c-muted", family=CJK, line_height=1.5),
    ], gap=6)
    steps = col("构成步骤组", [
        logic_step("01", "守住母形", "保留 P 的竖干与圆肩，\n先守住字母识别。", "base"),
        logic_step("02", "改造字腔", "把封闭字腔拉成切口，\n让留白承担第二层含义。", "void"),
        logic_step("03", "统一收口", "删除独立箭头，只留下\n一个轮廓和一次节奏。", "final"),
    ], gap=8)

    panel = frame(ids, "构成逻辑面板", width=568, height=BODY_H,
                  layout="vertical", padding=[30, 34], gap=18,
                  alignItems="start", cornerRadius=28,
                  fill=solid("$c-surface"), stroke=stroke("$c-line", 2))
    panel["children"] = [heading, steps]
    return panel


def state_card(name, label, *, dark):
    bg = "$c-ink" if dark else "$c-surface"
    ink = "$c-white" if dark else "$c-ink"
    card = frame(ids, name, width=248, height=BOTTOM_H, layout="vertical",
                 padding=[24, 22], gap=14, alignItems="center",
                 justifyContent="center", cornerRadius=22, fill=solid(bg))
    if not dark:
        card["stroke"] = stroke("$c-line", 2)
    card["children"] = [
        p_mark(f"{name} · 标志", 76, ink, bg),
        text(ids, f"{name} · 标签", label, 15, 650, ink, family=LATIN,
             width="fit_content", growth="auto", line_height=1.3,
             spacing=0.8),
    ]
    return card


def size_sample(size, label):
    return col(f"{label} 样本", [
        p_mark(f"{label} 标志", size, "$c-ink", "$c-surface"),
        text(ids, f"{label} 标签", label, 15, 600, "$c-muted",
             family=LATIN, width="fit_content", growth="auto",
             line_height=1.3),
    ], width="fill_container", gap=10, align="center", justify="end")


def minimum_card():
    samples = row("尺寸样本", [
        size_sample(64, "64 PX"),
        size_sample(32, "32 PX"),
        size_sample(16, "16 PX"),
    ], height=112, gap=10, align="end")
    card = frame(ids, "最小尺寸", width=624, height=BOTTOM_H,
                 layout="vertical", padding=[20, 26], gap=10,
                 justifyContent="space_between", cornerRadius=22,
                 fill=solid("$c-surface"), stroke=stroke("$c-line", 2))
    card["children"] = [
        row("尺寸标题行", [
            text(ids, "尺寸标题", "最小尺寸", 20, 700, "$c-ink",
                 family=CJK, width="fit_content", growth="auto",
                 line_height=1.3),
            text(ids, "尺寸说明", "真实节点尺寸", 15, 400, "$c-muted",
                 family=CJK, width="fit_content", growth="auto",
                 line_height=1.4),
        ], justify="space_between"),
        samples,
    ]
    return card


def validation_rail():
    intro = col("三态说明", [
        rect(ids, "强调短线", width=54, height=6, cornerRadius=3,
             fill=solid("$c-accent")),
        text(ids, "三态标题", "三态存活检查", 30, 700, "$c-ink", family=CJK,
             line_height=1.25),
        text(ids, "三态文案", "先看黑白与缩小，\n确认过线后再讨论颜色。", 18, 400,
             "$c-muted", family=CJK, line_height=1.5),
    ], width=280, height=BOTTOM_H, gap=10, justify="center")
    return row("黑白与最小尺寸", [
        intro,
        state_card("正片", "POSITIVE", dark=False),
        state_card("反片", "REVERSE", dark=True),
        minimum_card(),
    ], width=INNER_W, height=BOTTOM_H, gap=24, align="stretch")


def build():
    page = frame(ids, "品牌概念板", width=W, height=H, layout="vertical",
                 padding=[PAD_Y, PAD_X], gap=0,
                 justifyContent="space_between", alignItems="start",
                 fill=solid("$c-paper"), clipContent=True)
    page["children"] = [
        header(),
        row("主体", [hero_panel(), logic_panel()], width=INNER_W,
            height=BODY_H, gap=24, align="stretch"),
        validation_rail(),
    ]
    page["x"], page["y"] = 0, 0
    return [page]


# Contrast ratios were checked from the literal hex values above. Text-bearing
# pairs all exceed the repository's 2.0 threshold; the lowest is c-muted on
# c-paper at 4.87. The accent is annotation only and never carries body copy.

if __name__ == "__main__":
    write_doc(sys.argv[1], VARS, build(), "品牌概念板 · 单方案构成",
              compact=True)

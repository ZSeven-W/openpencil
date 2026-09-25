#!/usr/bin/env python3
"""openpencil-intro-deck-43.op — OpenPencil 产品介绍（1024×768 · 4:3 · 5 页）

Studio 首页「演示文稿 · 4:3」的即时初稿，与 16:9 版
（`tpl_deck_openpencil_intro.py`）讲同一件事：封面 → 产品价值 → 创作流程
→ 应用场景 → 结束页。文案与配色同源于 `openpencil_intro_copy.py`，矢量示意
部件同源于 `openpencil_intro_parts.py`（按 4:3 画板缩放）。

### 4:3 的重新构图（不是等比缩小）

  - **封面**：宽画幅的左右分栏在 4:3 上会把标题挤成三行，所以改成上下：
    标题与引语在上，产品窗口压成一条横向的「提示框 + 三张生成卡」放在下。
  - **价值页**：左标题右清单改成标题在上、三条价值纵向排满。
  - **流程页**：四张横排卡在 1024 宽里每张只剩 200px，改成 2×2 宫格，
    每张卡保留序号、AI 标签、说明与示意区。
  - **场景页**：2×2 宫格，每格左缩略右文字。
  - **结束页**：左标题右输入卡保留，行动路径改成竖排三行。

字阶沿用 deck43kit：封面 60 · 页标题 40 · 引语 22 · 条目标题 22–24 ·
正文 17–19 · 页脚 14。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import openpencil_intro_copy as C
from appkit import NUM, Kit
from deck43kit import (FS_BODY, FS_ITEM, FS_LEAD, FS_META, FS_TITLE, board,
                       footer, place)
from oplib import Ids, rect, solid, write_doc
from openpencil_intro_parts import Parts

ids = Ids()
k = Kit(ids)
P = Parts(k, ids, 0.6)
P_STEP = Parts(k, ids, 0.7)
P_MINI = Parts(k, ids, 0.62)

FS_COVER = 60


def foot(index, *, inverse=False):
    if inverse:
        return footer(k, ids, C.LABEL, index, C.TOTAL,
                      rule="$inverse-border", color="$inverse-muted")
    return footer(k, ids, C.LABEL, index, C.TOTAL)


def page(index, name, children, *, gap=28):
    body = k.col(f"{name} · 正文", children, gap=gap)
    return board(k, ids, f"{index:02d} {name}", body, foot(index))


def title_block(title, lead):
    return k.col("标题区", [
        rect(ids, "荧光标记", width=48, height=8, cornerRadius=4,
             fill=solid("$accent")),
        k.para(title, FS_TITLE, 700, lh=1.25, name="页标题"),
        k.para(lead, FS_LEAD, 400, "$muted-foreground", lh=1.5,
               name="引语"),
    ], gap=12)


def highlight(content, size, *, name="荧光色块"):
    return k.row(name, [
        k.label(content, size, 800, "$accent-foreground", lh=1.18),
    ], width="fit_content", fill=solid("$accent"), padding=[4, 14],
        cornerRadius=8)


# ---------------------------------------------------------------- 01 封面
def cover_strip():
    """产品窗口压成一条：提示框 + 三张生成卡。"""
    px = P.px
    thumbs = k.row("生成结果", [P.thumb(i, c, height=160)
                               for i, c in enumerate(C.MOCK_CARDS)],
                   gap=12, align="start")
    card = k.card("产品界面示意", [
        P.prompt_row(C.MOCK_PROMPT, size=28), thumbs,
    ], gap=16, pad=(18, 20), radius=18, border=None)
    card["effects"] = [{"type": "shadow", "offsetX": 0, "offsetY": px(24),
                        "blur": px(60), "spread": 0,
                        "color": "rgba(8,20,90,0.35)"}]
    return card


def cover():
    eyebrow = k.row("眉标", [
        k.tile("pen-tool", size=30, icon_size=16, fill="$accent",
               color="$accent-foreground", radius=8),
        k.label(C.COVER_EYEBROW, 16, 600, "$primary-foreground"),
    ], gap=10)
    heading = k.col("封面标题", [
        k.label(C.COVER_LINE_1, FS_COVER, 800, "$primary-foreground",
                lh=1.18),
        highlight(C.COVER_LINE_2, FS_COVER),
    ], gap=10)
    lead = k.para(C.COVER_LEAD, 19, 400, "$inverse-muted", lh=1.6,
                  width=640, name="封面副标题")
    meta = k.row("封面信息", [
        k.col(f"信息 {cap}", [
            k.label(cap, FS_META, 500, "$inverse-muted"),
            k.label(val, 17, 600, "$primary-foreground"),
        ], gap=4, width="fit_content") for cap, val in C.COVER_META
    ], gap=40)
    top = k.col("封面上栏", [eyebrow, heading, lead, meta], gap=24)
    body = k.col("封面 · 正文", [top, cover_strip()], gap=0,
                 justify="space_between")
    return board(k, ids, "01 封面", body, foot(1, inverse=True),
                 fill="$primary")


# ---------------------------------------------------------------- 02 价值
def value():
    rows = []
    for index, (glyph, title, desc) in enumerate(C.VALUES, 1):
        if index > 1:
            rows.append(k.divider())
        rows.append(k.row(f"价值 {title}", [
            k.para(f"0{index}", 36, 800, "$primary", family=NUM, lh=1.1,
                   width=60, name=f"序号 0{index}"),
            k.col(f"{title} 文字", [
                k.row(f"{title} 标题行", [
                    k.tile(glyph, size=36, icon_size=18, fill="$secondary",
                           color="$primary", radius=10),
                    k.label(title, FS_ITEM, 700),
                ], gap=12),
                k.para(desc, FS_BODY, 400, "$muted-foreground", lh=1.6),
            ], gap=10),
        ], gap=16, align="start", padding=[22, 0]))
    return page(2, "价值", [
        title_block(C.flat(C.VALUE_TITLE), C.flat(C.VALUE_LEAD)),
        k.col("价值清单", rows, justify="center", height="fill_container"),
    ], gap=12)


# ---------------------------------------------------------------- 03 流程
# The 16:9 cards break these into three short lines; a 4:3 grid cell is
# wider, so it gets its own two-line breaks (auto-wrapping left orphans).
FLOW_DESC = [
    "用一两句话说清主题、受众和风格，\n也可以直接从示例开始。",
    "AI 按你的描述排好页面结构、\n文字层级和整体配色。",
    "直接改文字、换图片、调颜色，\n或让 AI 按你的意见再改一轮。",
    "导出成图片，发布到内容平台，\n或发给团队一起看。",
]


def step_card(index, glyph, title, desc, ai):
    tag = (k.pill(C.TAG_AI, fill="$accent", color="$accent-foreground",
                  size=13, weight=700, pad=(4, 10), glyph="sparkles")
           if ai else
           k.pill(C.TAG_YOU, fill="$muted", color="$muted-foreground",
                  size=13, weight=600, pad=(4, 10), glyph="user"))
    head = k.row(f"{title} 眉行", [
        k.row(f"{title} 序号", [
            k.label(str(index), 17, 800, "$primary-foreground", family=NUM,
                    lh=1.0),
        ], width=34, height=34, justify="center", fill=solid("$primary"),
            cornerRadius=17),
        k.label(title, 22, 700),
        k.spacer(),
        tag,
    ], gap=12)
    stage = k.col(f"{title} 示意区", [P_STEP.step_visual(index)],
                  height="fill_container", justify="center",
                  fill=solid("$secondary"), padding=[10, 16],
                  cornerRadius=12)
    return k.card(f"步骤 {title}", [
        head,
        k.para(FLOW_DESC[index - 1], 17, 400, "$muted-foreground", lh=1.55),
        stage,
    ], gap=12, pad=(18, 18), radius=16, height="fill_container")


def flow():
    cards = [step_card(i, *s) for i, s in enumerate(C.FLOW, 1)]
    rows = [k.row(f"流程第 {r + 1} 行", cards[r * 2:r * 2 + 2], gap=16,
                  align="start", height="fill_container")
            for r in range(2)]
    return page(3, "流程", [
        title_block(C.FLOW_TITLE, C.FLOW_LEAD),
        k.col("流程宫格", rows, gap=16, height="fill_container"),
    ], gap=20)


# ---------------------------------------------------------------- 04 场景
def scene_card(kind, title, desc):
    stage = k.row(f"{title} 画幅区", [P_MINI.mini(kind)], width=210,
                  height="fill_container", justify="center", align="center",
                  fill=solid("$muted"), cornerRadius=12)
    words = k.col(f"{title} 文字", [
        k.label(title, FS_ITEM, 700),
        k.para(desc, 17, 400, "$muted-foreground", lh=1.6),
    ], gap=8, justify="center", height="fill_container")
    return k.card(f"场景 {title}", [
        k.row(f"{title} 内容", [stage, words], gap=18, align="start",
              height="fill_container"),
    ], pad=(12, 12), radius=16, height="fill_container")


def scenes():
    cards = [scene_card(*s) for s in C.SCENES]
    rows = [k.row(f"场景第 {r + 1} 行", cards[r * 2:r * 2 + 2], gap=16,
                  align="start", height="fill_container")
            for r in range(2)]
    return page(4, "场景", [
        title_block(C.SCENE_TITLE, C.SCENE_LEAD),
        k.col("场景宫格", rows, gap=16, height="fill_container"),
    ], gap=20)


# ---------------------------------------------------------------- 05 结束
def closing():
    steps = []
    for index, step in enumerate(C.CLOSE_STEPS, 1):
        steps.append(k.row(f"行动 {step}", [
            k.row(f"行动 {index} 序号", [
                k.label(str(index), 14, 800, "$accent-foreground",
                        family=NUM, lh=1.0),
            ], width=26, height=26, justify="center", fill=solid("$accent"),
                cornerRadius=13),
            k.label(step, 17, 600, "$primary-foreground"),
        ], gap=12))
    left = k.col("结束左栏", [
        k.label(C.CLOSE_KICKER, 16, 600, "$inverse-muted"),
        k.col("结束标题", [
            k.label(C.CLOSE_LINE_1, 50, 800, "$primary-foreground",
                    lh=1.18),
            highlight(C.CLOSE_LINE_2, 50),
        ], gap=10),
        k.para(C.CLOSE_LEAD, FS_BODY, 400, "$inverse-muted", lh=1.6),
        k.col("行动路径", steps, gap=12),
    ], gap=28, justify="center", height="fill_container")
    body = k.row("结束 · 正文", [left, P.start_card(width=620)], gap=36,
                 align="center")
    return board(k, ids, "05 结束页", body, foot(5, inverse=True),
                 fill="$primary")


def build():
    return place([cover(), value(), flow(), scenes(), closing()])


if __name__ == "__main__":
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                       "..", "crates", "op-editor-core", "assets",
                       "scene_templates", "openpencil-intro-deck-43.op")
    write_doc(os.path.normpath(out), C.VARS, build(),
              "OpenPencil 产品介绍 · 4:3", compact=True)

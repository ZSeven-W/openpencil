#!/usr/bin/env python3
"""openpencil-intro-deck.op — OpenPencil 产品介绍（1920×1080 · 16:9 · 5 页）

Studio 首页「演示文稿 · 16:9」的即时初稿。五页与示例一一对应：封面 → 产品
价值 → 创作流程 → 应用场景 → 结束页。文案、配色与负约束写在
`openpencil_intro_copy.py`，矢量示意部件在 `openpencil_intro_parts.py`
（4:3 版 `tpl_deck43_openpencil_intro.py` 与本文件同源）。

### 16:9 的构图

  - **封面 / 结束页**整页钴蓝，首尾呼应：标题第二行是一块荧光黄「划重点」，
    右侧各是一只产品窗口（封面：提示框 + 生成的三张卡；结束页：「开始设
    计」输入卡）。产品图用矢量示意而不是图库照片 —— 这份 deck 的主角是
    产品本身，一张泛泛的办公室照片什么也说明不了；矢量示意换主色时也跟着变。
  - **价值页**左标题右清单：宽画幅横向分栏，三条价值读成一列，比三张等高
    卡片少一大片空卡底。
  - **流程页**四张步骤卡横排、箭头相连；卡底一块「这一步长什么样」的示意
    区（提示框 / 三块画板 / 色板与字号 / 导出芯片），荧光黄只标 AI 参与的
    两步。
  - **场景页**四种成品各配一个色块缩略（轮播 / 海报 / 演示 / 长图），画幅
    比例本身就是信息。

字阶（px）：封面 128 · 页标题 72 · 引语 32 · 条目标题 36–40 · 正文 24–28
· 页脚 22。全部 flex 排布，文字只写宽不写高。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import openpencil_intro_copy as C
from appkit import NUM, Kit
from deckkit import H, W, place_boards
from oplib import Ids, frame, rect, solid, write_doc
from openpencil_intro_parts import Parts

ids = Ids()
k = Kit(ids)
P = Parts(k, ids, 1.0)
# Scene thumbnails sit in a ~370px-wide stage; at 1.0 they read as specks.
P_MINI = Parts(k, ids, 1.2)

PAD_X, PAD_TOP, PAD_BOTTOM = 120, 96, 64
FS_COVER, FS_TITLE, FS_LEAD = 128, 72, 32
FS_ITEM, FS_BODY, FS_META = 40, 28, 22


def footer(index, *, rule="$border", color="$muted-foreground"):
    line = rect(ids, "页脚线", width="fill_container", height=1,
                fill=solid(rule))
    row = k.row("页脚行", [
        k.label(C.LABEL, FS_META, 500, color),
        k.spacer(),
        k.label(f"{index:02d} / {C.TOTAL:02d}", FS_META, 600, color,
                family=NUM),
    ])
    return k.col("页脚", [line, row], gap=20)


def board(index, name, body, *, fill="$background", inverse=False):
    node = frame(ids, f"{index:02d} {name}", width=W, height=H,
                 layout="vertical", fill=solid(fill), clipContent=True,
                 padding=[PAD_TOP, PAD_X, PAD_BOTTOM, PAD_X], gap=40)
    body["height"] = "fill_container"
    foot = (footer(index, rule="$inverse-border", color="$inverse-muted")
            if inverse else footer(index))
    node["children"] = [body, foot]
    return node


def title_block(title, lead, *, width="fill_container"):
    return k.col("标题区", [
        rect(ids, "荧光标记", width=88, height=12, cornerRadius=6,
             fill=solid("$accent")),
        k.para(title, FS_TITLE, 700, lh=1.2, width=width, name="页标题"),
        k.para(lead, FS_LEAD, 400, "$muted-foreground", lh=1.6, width=width,
               name="引语"),
    ], gap=20, width=width)


def highlight(content, size, *, pad=(6, 24), radius=12, name="荧光色块"):
    """荧光黄色块里的一行深墨蓝大字 —— 整套 deck 的「划重点」。"""
    return k.row(name, [
        k.label(content, size, 800, "$accent-foreground", lh=1.18),
    ], width="fit_content", fill=solid("$accent"), padding=list(pad),
        cornerRadius=radius)


# ---------------------------------------------------------------- 01 封面
def cover():
    eyebrow = k.row("眉标", [
        k.tile("pen-tool", size=48, icon_size=24, fill="$accent",
               color="$accent-foreground", radius=12),
        k.label(C.COVER_EYEBROW, 26, 600, "$primary-foreground"),
    ], gap=16)
    heading = k.col("封面标题", [
        k.label(C.COVER_LINE_1, FS_COVER, 800, "$primary-foreground",
                lh=1.18),
        highlight(C.COVER_LINE_2, FS_COVER),
    ], gap=16)
    lead = k.para(C.COVER_LEAD, FS_LEAD, 400, "$inverse-muted", lh=1.6,
                  width=920, name="封面副标题")
    meta = k.row("封面信息", [
        k.col(f"信息 {cap}", [
            k.label(cap, FS_META, 500, "$inverse-muted"),
            k.label(val, 28, 600, "$primary-foreground"),
        ], gap=8, width="fit_content") for cap, val in C.COVER_META
    ], gap=64)
    left = k.col("封面左栏", [eyebrow, heading, lead, meta], gap=44,
                 justify="center", height="fill_container")
    body = k.row("封面 · 正文", [left, P.product_mock()], gap=64,
                 align="center")
    return board(1, "封面", body, fill="$primary", inverse=True)


# ---------------------------------------------------------------- 02 价值
def value():
    rows = []
    for index, (glyph, title, desc) in enumerate(C.VALUES, 1):
        if index > 1:
            rows.append(k.divider())
        rows.append(k.row(f"价值 {title}", [
            k.para(f"0{index}", 56, 800, "$primary", family=NUM, lh=1.1,
                   width=96, name=f"序号 0{index}"),
            k.col(f"{title} 文字", [
                k.row(f"{title} 标题行", [
                    k.tile(glyph, size=52, icon_size=26, fill="$secondary",
                           color="$primary", radius=14),
                    k.label(title, FS_ITEM, 700),
                ], gap=18),
                k.para(desc, FS_BODY, 400, "$muted-foreground", lh=1.6),
            ], gap=16),
        ], gap=32, align="start", padding=[40, 0]))
    left = title_block(C.VALUE_TITLE, C.VALUE_LEAD, width=560)
    left["justifyContent"] = "center"
    left["height"] = "fill_container"
    right = k.col("价值清单", rows, justify="center",
                  height="fill_container")
    body = k.row("价值 · 正文", [left, right], gap=120, align="center")
    return board(2, "价值", body)


# ---------------------------------------------------------------- 03 流程
def step_card(index, glyph, title, desc, ai):
    tag = (k.pill(C.TAG_AI, fill="$accent", color="$accent-foreground",
                  size=20, weight=700, pad=(6, 14), glyph="sparkles")
           if ai else
           k.pill(C.TAG_YOU, fill="$muted", color="$muted-foreground",
                  size=20, weight=600, pad=(6, 14), glyph="user"))
    stage = k.col(f"{title} 示意区", [
        P.gap_fill(f"{title} 示意上"),
        P.step_visual(index),
        P.gap_fill(f"{title} 示意下"),
        k.label(C.FLOW_EXAMPLES[index - 1], 22, 600,
                "$secondary-foreground"),
    ], gap=16, height="fill_container", fill=solid("$secondary"),
        padding=[20, 20], cornerRadius=16)
    return k.card(f"步骤 {title}", [
        k.row(f"{title} 眉行", [
            k.row(f"{title} 序号", [
                k.label(str(index), 28, 800, "$primary-foreground",
                        family=NUM, lh=1.0),
            ], width=56, height=56, justify="center",
                fill=solid("$primary"), cornerRadius=28),
            k.spacer(),
            tag,
        ]),
        k.row(f"{title} 标题行", [
            k.icon(glyph, 32, "$primary"),
            k.label(title, 34, 700),
        ], gap=14),
        k.para(desc, 24, 400, "$muted-foreground", lh=1.6),
        stage,
    ], gap=22, pad=(28, 28), radius=24, height="fill_container")


def flow():
    cells = []
    for index, (glyph, title, desc, ai) in enumerate(C.FLOW, 1):
        if index > 1:
            cells.append(k.col(f"箭头 {index}", [
                k.icon("arrow-right", 32, "$primary"),
            ], width="fit_content", height="fill_container",
                justify="center"))
        cells.append(step_card(index, glyph, title, desc, ai))
    steps = k.row("流程四步", cells, gap=14, align="start",
                  height="fill_container")
    return board(3, "流程", k.col("流程 · 正文", [
        title_block(C.FLOW_TITLE, C.FLOW_LEAD), steps,
    ], gap=48))


# ---------------------------------------------------------------- 04 场景
def scene_card(kind, title, desc):
    stage = k.row(f"{title} 画幅区", [P_MINI.mini(kind)], height="fill_container",
                  justify="center", align="center", fill=solid("$muted"),
                  cornerRadius=18)
    return k.card(f"场景 {title}", [
        stage,
        k.col(f"{title} 文字", [
            k.label(title, 36, 700),
            k.para(desc, 26, 400, "$muted-foreground", lh=1.6),
        ], gap=12, padding=[4, 8, 8, 8]),
    ], gap=24, pad=(20, 20), radius=24, height="fill_container")


def scenes():
    grid = k.row("场景四栏", [scene_card(*s) for s in C.SCENES], gap=28,
                 align="start", height="fill_container")
    return board(4, "场景", k.col("场景 · 正文", [
        title_block(C.SCENE_TITLE, C.SCENE_LEAD), grid,
    ], gap=48))


# ---------------------------------------------------------------- 05 结束
def closing():
    chips = []
    for index, step in enumerate(C.CLOSE_STEPS, 1):
        if index > 1:
            chips.append(k.icon("arrow-right", 26, "$inverse-muted"))
        chips.append(k.row(f"行动 {step}", [
            k.label(str(index), 22, 800, "$accent", family=NUM),
            k.label(step, 24, 600, "$primary-foreground"),
        ], gap=12, width="fit_content", fill=solid("$inverse-card"),
            padding=[14, 24], cornerRadius=999))
    left = k.col("结束左栏", [
        k.label(C.CLOSE_KICKER, 28, 600, "$inverse-muted"),
        k.col("结束标题", [
            k.label(C.CLOSE_LINE_1, 104, 800, "$primary-foreground",
                    lh=1.18),
            highlight(C.CLOSE_LINE_2, 104),
        ], gap=16),
        k.para(C.CLOSE_LEAD, FS_LEAD, 400, "$inverse-muted", lh=1.6,
               width=860),
        k.row("行动路径", chips, gap=16, width="fit_content"),
    ], gap=40, justify="center", height="fill_container")
    body = k.row("结束 · 正文", [left, P.start_card(width=620)], gap=64,
                 align="center")
    return board(5, "结束页", body, fill="$primary", inverse=True)


def build():
    return place_boards([cover(), value(), flow(), scenes(), closing()])


if __name__ == "__main__":
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                       "..", "crates", "op-editor-core", "assets",
                       "scene_templates", "openpencil-intro-deck.op")
    write_doc(os.path.normpath(out), C.VARS, build(),
              "OpenPencil 产品介绍 · 16:9", compact=True)

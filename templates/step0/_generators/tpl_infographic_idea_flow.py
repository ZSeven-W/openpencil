#!/usr/bin/env python3
"""idea-to-publish-flow.op — 从灵感到发布五步流程长图（1080×N 竖版）

Studio 首页「信息图 · 流程」的示例就是这一份：「做一张『从灵感到发布』的五步
流程长图：记录想法、整理结构、完成初稿、打磨视觉、检查发布。用编号和连线串起
来，每一步配一句简短说明。」空输入框点开始设计时它作为即时初稿载入，所以五步
的名字、顺序与示例逐字对应。

### 连线怎么画才不断

同档的 steps-flow-infographic 为了「改一句文案线就断」放弃了连线，只在卡片
之间放箭头。这张的示例明说要「编号和连线」，所以换一种不写死像素的做法：

    每一步是一行 row(alignItems: stretch)
      ├─ 轨道列（定宽 76）：序号圆 + 一根 height: fill_container 的竖线
      └─ 内容列：白卡 + 底部留白（下一步之前的间距）

行高由内容决定，轨道列被 stretch 拉到同一高度，竖线吃掉序号圆以下的全部
剩余高度 —— 正好接到下一行的序号圆顶上。改文案、加一行说明，线都跟着走。
最后一步没有竖线。

### 配色

深松绿页头 + 苔绿强调：和同档的柑橘橙（steps-flow）、青绿（data-report）
拉开色相。一个有彩色，其余全是中性明度序列。

硬契约同 tpl_infographic_data：内容距边缘 80、配色全走 color_vars、CJK 行
高 1.2/1.3/1.7、CJK 负字距不超过 -0.02em、根帧写 x/y、ROOT_H 量出来烤回。

### 负约束

  - 说明一律一行一句，不写 AI 套话，每一步写成能照着做的动作。
  - 不用 emoji、伪 3D、装饰插画；不画跨步骤的绝对定位线。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from oplib import (Ids, color_vars, frame, icon_font, rect, solid, text,
                   write_doc)

ids = Ids()

VARS = color_vars({
    "c-bg":          "#F6F8F3",
    "c-surface":     "#FFFFFF",
    "c-band":        "#15281E",
    "c-band-muted":  "#A7BDAF",
    "c-ink":         "#16231B",
    "c-muted":       "#5A6B60",
    "c-accent":      "#2F7D4F",
    "c-accent-deep": "#23613C",
    "c-accent-soft": "#DDEFE3",
    "c-border":      "#E1E8E1",
    # 连线：比描边深，读得出流向；比强调色浅，不和序号抢。
    "c-rail":        "#9CC3A8",
})

CJK = "Noto Sans SC"
NUM = "Inter"

W = 1080
EDGE = 80
LH_DISPLAY, LH_HEAD, LH_BODY = 1.2, 1.3, 1.7

BADGE = 76
RAIL_W = 6
STEP_SPACING = 36

# 量出来的根高（根设 fit_content 渲一次读 PNG 高度，再烤回来）。
ROOT_H = 2551

# (步骤名, 一句说明, 产出, 图标)
STEPS = [
    ("记录想法", "灵感来了先记下来，一句话就够，别急着判断好坏。",
     "想法清单", "lightbulb"),
    ("整理结构", "挑出最想说的那一条，排好开头、要点和结尾。",
     "三段提纲", "list-tree"),
    ("完成初稿", "照着提纲一口气写完，先求完整，再求好看。",
     "完整初稿", "pen-line"),
    ("打磨视觉", "选好版式和配色，重点加粗、配上图，一眼能看懂。",
     "成品图稿", "palette"),
    ("检查发布", "通读一遍查错字和数字，确认无误再点发布。",
     "发布上线", "send"),
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


def section_head(title, note):
    return col("区块头", [
        rect(ids, "强调短线", width=72, height=8, cornerRadius=999,
             fill=solid("$c-accent")),
        text(ids, "区块标题", title, 46, 700, "$c-ink", family=CJK,
             line_height=LH_HEAD),
        text(ids, "区块说明", note, 27, 400, "$c-muted", family=CJK,
             line_height=LH_BODY),
    ], gap=16)


# ------------------------------------------------------------------ 01 页头
def header():
    return band("01 页头", fill=solid("$c-band"), pad=[76, EDGE, 68, EDGE],
                gap=26, children=[
        chip("创作流程 · 五步走", bg="$c-accent", fg="$c-surface",
             icon="route"),
        text(ids, "主标题", "从灵感到发布，\n一步一步来", 76, 700,
             "$c-surface", family=CJK, line_height=LH_DISPLAY, spacing=-1.4),
        text(ids, "副标题", "每一步只做一件事。做完这一步，再进下一步。",
             28, 400, "$c-band-muted", family=CJK, line_height=LH_BODY),
        row("步骤速览", [
            label(f"速览 {index}", f"{index:02d} {name}", 24, 500,
                  "$c-band-muted")
            for index, (name, _, _, _) in enumerate(STEPS, 1)
        ], gap=28),
    ])


# ------------------------------------------------------------------ 02 流程
def badge(index):
    node = frame(ids, f"序号 {index}", width=BADGE, height=BADGE,
                 layout="horizontal", alignItems="center",
                 justifyContent="center", cornerRadius=BADGE // 2,
                 fill=solid("$c-accent"))
    node["children"] = [label("序号数字", f"{index:02d}", 30, 700,
                              "$c-surface", family=NUM, lh=1.0)]
    return node


def rail(index, last):
    """序号圆 + 往下接到下一步的竖线。竖线 fill_container，长度跟着行高走。"""
    kids = [badge(index)]
    if not last:
        kids.append(rect(ids, f"连线 {index}", width=RAIL_W,
                         height="fill_container", cornerRadius=RAIL_W // 2,
                         fill=solid("$c-rail")))
    # Bottom padding mirrors the gap under the badge, so the line stops as
    # far above the next badge as it starts below this one.
    return col(f"轨道 {index}", kids, gap=10, width=BADGE, align="center",
               height="fill_container", padding=[0, 0, 0 if last else 10, 0])


def step_card(name, desc, output, glyph):
    head = row("步骤头", [
        text(ids, "步骤名", name, 38, 700, "$c-ink", family=CJK,
             width="fill_container", line_height=LH_HEAD),
        frame(ids, "图标底", width=56, height=56, layout="horizontal",
              alignItems="center", justifyContent="center", cornerRadius=16,
              fill=solid("$c-accent-soft"),
              children=[icon_font(ids, "步骤图标", glyph, 28,
                                  "$c-accent-deep")]),
    ], gap=20)
    output_row = row("产出行", [
        label("产出名", "产出", 24, 500, "$c-muted"),
        chip(output, bg="$c-accent-soft", fg="$c-accent-deep", size=24),
    ], gap=14)
    card = col("步骤卡", [
        head,
        text(ids, "步骤说明", desc, 28, 400, "$c-muted", family=CJK,
             line_height=LH_BODY),
        output_row,
    ], gap=14, padding=[30, 32], cornerRadius=22)
    card["fill"] = solid("$c-surface")
    card["stroke"] = {"thickness": 2, "fill": solid("$c-border")}
    return card


def step_row(index, name, desc, output, glyph, last):
    body = col(f"步骤内容 {index}", [step_card(name, desc, output, glyph)],
               gap=0, padding=[0, 0, 0 if last else STEP_SPACING, 0])
    return row(f"步骤 {index}", [rail(index, last), body], gap=28,
               align="stretch")


def flow():
    rows = [step_row(index, *step, index == len(STEPS))
            for index, step in enumerate(STEPS, 1)]
    return band("02 流程", fill=[], pad=[64, EDGE, 72, EDGE], gap=36, children=[
        section_head("按顺序走完五步", "每一步配一句怎么做，卡片底部是这一步交出来的东西。"),
        col("步骤列表", rows, gap=0),
    ])


# ------------------------------------------------------------------ 03 收尾
def closing():
    return band("03 收尾", fill=solid("$c-band"), pad=[64, EDGE, 56, EDGE],
                gap=22, children=[
        row("收尾标题行", [
            icon_font(ids, "循环", "refresh-cw", 44, "$c-accent-soft"),
            text(ids, "收尾标题", "发完这一篇，再回到第一步", 46, 700,
                 "$c-surface", family=CJK, line_height=LH_HEAD),
        ], gap=18),
        text(ids, "收尾说明", "把发布后收到的反馈记进想法清单，下一篇就不用从零开始。",
             27, 400, "$c-band-muted", family=CJK, line_height=LH_BODY),
        row("署名行", [
            label("账号名", "@ 你的账号名", 26, 600, "$c-surface"),
            label("更新说明", "每周一张能照着做的图", 24, 400,
                  "$c-band-muted"),
        ], gap=16, padding=[18, 0, 0, 0]),
    ])


def build():
    page = frame(ids, "从灵感到发布流程长图", width=W, height=ROOT_H,
                 layout="vertical", gap=0, fill=solid("$c-bg"),
                 clipContent=True)
    page["children"] = [header(), flow(), closing()]
    page["x"], page["y"] = 0, 0
    return [page]


# 对比度（WCAG 相对亮度比，op-design-lint 门槛 2.0）：
#   c-surface     on c-band        15.9    c-band-muted on c-band        7.4
#   c-ink         on c-bg          15.4    c-muted      on c-surface     5.8
#   c-surface     on c-accent       5.2    c-accent-deep on c-accent-soft 5.9
#   c-rail        on c-bg           1.9 —— 连线是结构图形而非文字，靠序号圆
#   (5.2) 承担流向，线只负责把圆串起来。

if __name__ == "__main__":
    write_doc(sys.argv[1], VARS, build(), "从灵感到发布流程长图")

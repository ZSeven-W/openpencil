#!/usr/bin/env python3
"""research-findings-deck.op — 用户研究报告（1024×768 · 4:3 · 6 页，深色）

4:3 的第二套：研究汇报。评审会上投在会议室的老投影上、会后打印成讲义归
档。六页讲完一份研究：封面 → 研究方法 → 三个关键发现 → 困扰排行（横条
图）→ 一次买菜的情绪曲线 → 产品建议与下一步。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：夜里十点的厨房 —— 关了顶灯的深蓝、冰箱门缝里透出的一条薄
    荷色冷光、便签上一抹琥珀色的荧光笔。
  - **收敛**：深靛 #0F172A 底 + #1E293B 卡；主色薄荷 #5EEAD4 标「发现与
    数据」，琥珀 #FBBF24 只标「痛点」—— 两支颜色各管一类信息，不混用。
  - **论证**：研究报告的听众要在十几页里分清「我们看到了什么」和「用户
    在哪里难受」，颜色直接承担这个分类；其余一律中性灰阶。

### 负约束

  - 不引用任何真实品牌、平台、生鲜 App 的名称；数据为示意，页脚注明样本。
  - 横条宽度、情绪点高度都由数值换算（见 PAINS / JOURNEY）。
  - 不放用户照片占位；访谈引语用文字呈现。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from appkit import NUM, Kit
from deck43kit import (FS_BODY, FS_COVER, FS_ITEM, FS_LEAD, FS_META,
                       FS_TITLE, PAD_X, W, board, footer, place)
from oplib import Ids, color_vars, frame, rect, solid, stroke, write_doc

ids = Ids()
k = Kit(ids)

VARS = color_vars({
    "background":           "#0F172A",
    "foreground":           "#F1F5F9",
    "card":                 "#1E293B",
    "card-foreground":      "#F1F5F9",
    "primary":              "#5EEAD4",
    "primary-foreground":   "#042F2E",
    "secondary":            "#123B3F",
    "secondary-foreground": "#99F6E4",
    "muted":                "#26344D",
    "muted-foreground":     "#A5B1C4",
    "accent":               "#FBBF24",
    "accent-foreground":    "#1C1403",
    "accent-soft":          "#3A2F12",
    "border":               "#334155",
    "ring":                 "#5EEAD4",
})

LABEL = "双职工家庭买菜研究 · 2026 秋"
TOTAL = 6
INNER = W - PAD_X * 2    # 896


def title_block(title, lead=None):
    kids = [k.para(title, FS_TITLE, 700, lh=1.25, name="页标题")]
    if lead:
        kids.append(k.para(lead, FS_LEAD, 400, "$muted-foreground", lh=1.5,
                           name="引语"))
    return k.col("标题区", kids, gap=10)


def page(index, name, children, *, gap=32):
    body = k.col(f"{name} · 正文", children, gap=gap)
    return board(k, ids, f"{index:02d} {name}", body,
                 footer(k, ids, LABEL, index, TOTAL))


def filler(name="留白"):
    return frame(ids, name, width="fill_container", height="fill_container",
                 layout="none", fill=[])


# ---------------------------------------------------------------- 01 封面
def cover():
    eyebrow = k.pill("用户研究报告 · 2026 秋", fill="$secondary",
                     color="$secondary-foreground", size=15, pad=(6, 14),
                     glyph="search")
    heading = k.col("封面标题", [
        k.para("双职工家庭，\n为什么总在为买菜发愁", FS_COVER, 700, lh=1.18,
               name="封面主标题"),
        k.para("24 场家访加 1,260 份问卷，算清一周三餐背后的时间账。",
               FS_LEAD, 400, "$muted-foreground", lh=1.5, width=760,
               name="封面副标题"),
    ], gap=24)
    meta = k.row("封面信息", [
        meta_item("研究团队", "用户研究组"),
        meta_item("周期", "8 月 – 9 月"),
        meta_item("城市", "上海 · 成都 · 武汉"),
    ], gap=48)
    body = k.col("封面 · 正文", [eyebrow, heading, meta],
                 justify="space_between")
    return board(k, ids, "01 封面", body, footer(k, ids, LABEL, 1, TOTAL))


def meta_item(caption, value):
    return k.col(f"信息 {caption}", [
        k.label(caption, FS_META, 400, "$muted-foreground"),
        k.label(value, FS_BODY, 600),
    ], gap=6, width="fit_content")


# ---------------------------------------------------------------- 02 方法
METHODS = [
    ("24", "场", "深度家访", "每户 90 分钟，\n跟拍一次从买菜到开饭。"),
    ("1,260", "份", "有效问卷", "25–45 岁双职工，\n家里有 12 岁以下孩子。"),
    ("3", "座", "城市", "一线、新一线、二线\n各一座，控制通勤差异。"),
]


def methods():
    cards = []
    for number, unit, title, desc in METHODS:
        cards.append(k.card(f"方法 {title}", [
            k.row(f"{title} 数字", [
                k.label(number, 72, 700, "$primary", family=NUM, lh=1.05,
                        spacing=-1.5),
                k.label(unit, 22, 500, "$primary"),
            ], gap=6, width="fit_content", align="end"),
            filler(f"{title} 留白"),
            k.label(title, 26, 700),
            k.para(desc, FS_BODY, 400, "$muted-foreground", lh=1.6),
        ], gap=14, pad=(30, 26), radius=16, fill="$card", border="$border",
            height="fill_container"))
    quote = k.row("访谈引语", [
        rect(ids, "引语竖线", width=4, height=64, cornerRadius=2,
             fill=solid("$accent")),
        k.col("引语文字", [
            k.para("「周日列好了一周的菜单，到周三就全乱了。」", 24, 500,
                   lh=1.5, name="引语正文"),
            k.label("—— 受访者 07 · 成都 · 两个孩子的妈妈", 16, 400,
                    "$muted-foreground"),
        ], gap=8),
    ], gap=20, align="start")
    return page(2, "研究方法", [
        title_block("我们怎么做的这次研究", "定性看清行为，定量确认比例 —— 两条线的结论互相印证才写进报告。"),
        k.row("方法三栏", cards, gap=20, align="start",
              height="fill_container"),
        quote,
    ], gap=28)


# ---------------------------------------------------------------- 03 发现
FINDINGS = [
    ("01", "时间，而不是价格，才是第一约束",
     "被问到「最想改善什么」时，没人先提省钱。", "68%",
     "把「没时间」排在第一位"),
    ("02", "一周的计划，总在周三崩掉",
     "加班和孩子作业叠在周中，冰箱里的菜开始过期。", "周三",
     "一周里点外卖最多的一天"),
    ("03", "孩子的饭是不可妥协项",
     "大人可以凑合，孩子那份要新鲜、要单独做。", "3.4×",
     "单独给孩子备菜的频率"),
]


def findings():
    rows = []
    for index, (number, title, desc, figure, caption) in enumerate(FINDINGS):
        if index:
            rows.append(k.divider())
        rows.append(k.row(f"发现 {number}", [
            k.para(number, 24, 700, "$primary", family=NUM, width=48, lh=1.4,
                   name=f"发现序号 {number}"),
            k.col(f"发现 {number} 文字", [
                k.label(title, 26, 700),
                k.para(desc, FS_BODY, 400, "$muted-foreground", lh=1.5),
            ], gap=10),
            k.col(f"发现 {number} 数据", [
                k.para(figure, 56, 700, "$primary", family=NUM, width=220,
                       align="right", lh=1.1, name=f"数据 {number}"),
                k.para(caption, 15, 400, "$muted-foreground", width=220,
                       align="right", lh=1.4, name=f"数据说明 {number}"),
            ], gap=6, width=220),
        ], gap=20, padding=[34, 0], align="center"))
    return page(3, "关键发现", [
        title_block("三个关键发现", "每一条都同时出现在访谈与问卷里，才算数。"),
        k.col("发现列表", rows),
    ], gap=16)


# ---------------------------------------------------------------- 04 痛点
PAINS = [("菜不新鲜、没法挑", 0.62), ("送达时间说不准", 0.48),
         ("起送价逼着凑单", 0.37), ("想买的总缺货", 0.29),
         ("退换货太麻烦", 0.21)]
LABEL_W, VALUE_W = 200, 64
TRACK_W = INNER - LABEL_W - VALUE_W - 32


def pains():
    rows = []
    top = PAINS[0][1]
    for index, (name, share) in enumerate(PAINS):
        hot = index == 0
        fill_w = round(TRACK_W * share / top * 0.92)
        track = frame(ids, f"{name} 底轨", width=TRACK_W, height=36,
                      layout="horizontal", cornerRadius=6,
                      fill=solid("$muted"), clipContent=True)
        track["children"] = [rect(ids, f"{name} 横条", width=fill_w,
                                  height=36, cornerRadius=6,
                                  fill=solid("$accent" if hot
                                             else "$muted-foreground"))]
        rows.append(k.row(f"痛点 {name}", [
            k.para(name, 20, 600 if hot else 500, width=LABEL_W, lh=1.3,
                   name=f"痛点名 {name}"),
            track,
            k.para(f"{round(share * 100)}%", 24, 700,
                   "$accent" if hot else "$foreground", family=NUM,
                   width=VALUE_W, align="right", lh=1.3,
                   name=f"痛点值 {name}"),
        ], gap=16))
    note = k.row("图注", [
        k.icon("info", 16, "$muted-foreground"),
        k.label("多选题，n = 1,260；横条长度按占比换算，最长一条为 62%。", 14, 400,
                "$muted-foreground"),
    ], gap=8)
    callout = k.row("痛点结论", [
        k.icon("triangle-alert", 22, "$accent"),
        k.para("排第一的不是价格，而是「看不见、挑不了」—— 线上买菜最大的信任缺口在新鲜度。",
               FS_BODY, 500, "$accent", lh=1.5),
    ], gap=12, fill=solid("$accent-soft"), padding=[18, 22], cornerRadius=12,
        align="start")
    return page(4, "最困扰的环节", [
        title_block("最困扰的五个环节", "新鲜度与确定性两件事，占了前两名。"),
        k.col("痛点横条图", rows, gap=30),
        note,
        filler(),
        callout,
    ], gap=24)


# ---------------------------------------------------------------- 05 旅程
# 情绪分 1–5，点在 TRACK_H 高的轨道里按分值抬高。
JOURNEY = [
    ("计划", 3, "周日列菜单", "列得很满，\n但没考虑加班。"),
    ("下单", 2, "为起送价凑单", "为了免运费\n多买一堆零食。"),
    ("等待", 1, "送达时间不定", "开会时收到电话，\n菜在门口晒着。"),
    ("收货", 3, "缺货被替换", "替换品能用，\n但不是想要的。"),
    ("开饭", 5, "孩子吃光了", "这一刻觉得\n前面都值得。"),
]
TRACK_H = 230
DOT = 24


def journey():
    cols = []
    for stage, score, moment, pain in JOURNEY:
        low = score <= 2
        tone = "$accent" if low else "$primary"
        lift = round((score - 1) / 4 * (TRACK_H - DOT - 36))
        dot = {"type": "ellipse", "id": ids("e"), "name": f"{stage} 情绪点",
               "width": DOT, "height": DOT, "fill": solid(tone)}
        rail = k.col(f"{stage} 情绪轨", [
            k.label(moment, 16, 600, tone),
            dot,
        ], gap=10, align="center", justify="end", height=TRACK_H,
            padding=[0, 0, lift, 0])
        cols.append(k.col(f"阶段 {stage}", [
            rail,
            rect(ids, f"{stage} 轴段", width="fill_container", height=2,
                 fill=solid("$border")),
            k.label(stage, 26, 700),
            k.para(pain, 17, 400, "$muted-foreground", lh=1.5,
                   align="center"),
        ], gap=14, align="center"))
    legend = k.row("情绪图例", [
        k.row("图例 顺", [
            {"type": "ellipse", "id": ids("e"), "name": "图例点 顺",
             "width": 12, "height": 12, "fill": solid("$primary")},
            k.label("情绪较好", 14, 500, "$muted-foreground"),
        ], gap=8, width="fit_content"),
        k.row("图例 堵", [
            {"type": "ellipse", "id": ids("e"), "name": "图例点 堵",
             "width": 12, "height": 12, "fill": solid("$accent")},
            k.label("痛点集中", 14, 500, "$muted-foreground"),
        ], gap=8, width="fit_content"),
    ], gap=24)
    return page(5, "买菜旅程", [
        title_block("一次买菜的五个阶段", "情绪在「下单 → 等待」跌到谷底，开饭那一刻才回来。"),
        k.row("旅程五栏", cols, gap=12, align="start"),
        filler(),
        legend,
    ], gap=28)


# ---------------------------------------------------------------- 06 建议
ADVICE = [
    ("P0", "到货时间精确到 30 分钟", "解决「等待」谷底：按用户日程推荐送达窗口。",
     "预计等待焦虑 −40%"),
    ("P1", "周中补货一键重来", "周三自动提示「本周剩余菜单」，缺什么补什么。",
     "周中外卖替代 −15%"),
    ("P1", "儿童餐专区与新鲜度实拍", "孩子那份单独挑，收货前能看到实拍照片。",
     "新鲜度投诉 −30%"),
]


def advice():
    rows = []
    for level, title, desc, impact in ADVICE:
        urgent = level == "P0"
        rows.append(k.card(f"建议 {title}", [
            k.row(f"{title} 行", [
                k.pill(level, fill="$accent" if urgent else "$secondary",
                       color="$accent-foreground" if urgent
                       else "$secondary-foreground", size=14, weight=700,
                       pad=(4, 12)),
                k.col(f"{title} 文字", [
                    k.label(title, 24, 700),
                    k.para(desc, 17, 400, "$muted-foreground", lh=1.5),
                ], gap=8),
                k.para(impact, 17, 600, "$primary", width=180, align="right",
                       lh=1.4, name=f"预期 {title}"),
            ], gap=18),
        ], pad=(28, 26), radius=14, fill="$card", border="$border"))
    nxt = k.row("下一步", [
        k.icon("calendar-clock", 20, "$primary"),
        k.label("下一步：10 月对 P0 做两版原型，回到这 24 户家庭里测试。", 20, 600),
    ], gap=12)
    return page(6, "建议", [
        title_block("三条产品建议", "按「离谷底有多近」排优先级，而不是按实现成本。"),
        k.col("建议列表", rows, gap=18),
        filler(),
        nxt,
    ], gap=24)


def build():
    return place([cover(), methods(), findings(), pains(), journey(),
                  advice()])


if __name__ == "__main__":
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                       "..", "crates", "op-editor-core", "assets",
                       "scene_templates", "research-findings-deck.op")
    write_doc(os.path.normpath(out), VARS, build(), "用户研究报告 · 4:3",
              compact=True)

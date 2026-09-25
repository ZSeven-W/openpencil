#!/usr/bin/env python3
"""onboarding-training-deck.op — 新员工入职培训（1024×768 · 4:3 · 6 页）

4:3 是培训教室、会议室老投影和打印讲义的比例，入职培训是它最典型的用途：
一间会议室、一块旧幕布、一沓打印出来的讲义。六页走一节完整的课：封面 →
议程 → 做事原则 → 第一周安排 → 找谁帮忙 → 结尾清单。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：新人入职当天桌上那套东西 —— 米白的欢迎信纸、深墨蓝的工牌
    挂绳、一枚陶土色的公司徽章。
  - **收敛**：暖白纸 #FAF7F2 底 + 墨蓝 #1C2434 字；唯一有彩主色为陶土
    #C2410C，只标三件事：序号、时间轴上的「今天」、结尾页的勾。结尾页整页
    反成墨蓝底 —— 一节课的收束要在视觉上也「落地」。
  - **论证**：培训材料要打印，大面积彩色底在黑白打印里会变成一块灰。所
    以除结尾页外全部浅底，信息层级靠字重与留白，不靠色块。

### 负约束

  - 不放公司 logo、不写真实公司名与内部系统名；渠道写「内部通讯」。
  - 不用渐变、不用照片占位 —— 培训讲义以文字为主，空图位打印出来就是一个
    灰框。
  - 每页一个结论性标题；条目不超过 5 条，超过就拆页。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from appkit import NUM, Kit
from deck43kit import (FS_BODY, FS_COVER, FS_ITEM, FS_LEAD, FS_META,
                       FS_TITLE, board, footer, place)
from oplib import Ids, color_vars, frame, rect, solid, stroke, write_doc

ids = Ids()
k = Kit(ids)

VARS = color_vars({
    "background":           "#FAF7F2",
    "foreground":           "#1C2434",
    "card":                 "#FFFFFF",
    "card-foreground":      "#1C2434",
    "primary":              "#C2410C",
    "primary-foreground":   "#FFFFFF",
    "secondary":            "#FBE7DA",
    "secondary-foreground": "#9A3412",
    "muted":                "#F1ECE3",
    "muted-foreground":     "#645D53",
    "border":               "#E4DCCF",
    "ring":                 "#C2410C",
    "inverse":              "#1C2434",
    "inverse-foreground":   "#FAF7F2",
    "inverse-muted":        "#AEB4C0",
    "inverse-border":       "#39425A",
    "inverse-accent":       "#FB923C",
})

LABEL = "新人训练营 · 第 1 课"
TOTAL = 6


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


# ---------------------------------------------------------------- 01 封面
def cover():
    eyebrow = k.row("眉标", [
        k.tile("sparkles", size=32, icon_size=16, fill="$secondary",
               color="$primary", radius=8),
        k.label("新人训练营 · 第 1 课", 17, 600, "$primary"),
    ], gap=10)
    heading = k.col("封面标题", [
        rect(ids, "标题色条", width=64, height=6, cornerRadius=3,
             fill=solid("$primary")),
        k.para("欢迎加入，\n我们一起把事情做好", FS_COVER, 700, lh=1.18,
               name="封面主标题"),
        k.para("入职第一天的 45 分钟：认识公司、了解做事方式、排好你的第一周。",
               FS_LEAD, 400, "$muted-foreground", width=800, lh=1.5,
               name="封面副标题"),
    ], gap=24)
    meta = k.row("封面信息", [
        meta_item("主讲", "人力资源部 · 周岚"),
        meta_item("时长", "45 分钟 · 含问答"),
        meta_item("日期", "2026 年 9 月 28 日"),
    ], gap=48)
    body = k.col("封面 · 正文", [eyebrow, heading, meta], gap=0,
                 justify="space_between")
    return board(k, ids, "01 封面", body, footer(k, ids, LABEL, 1, TOTAL))


def meta_item(caption, value):
    return k.col(f"信息 {caption}", [
        k.label(caption, FS_META, 400, "$muted-foreground"),
        k.label(value, FS_BODY, 600),
    ], gap=6, width="fit_content")


# ---------------------------------------------------------------- 02 议程
AGENDA = [
    ("01", "公司是谁", "我们做什么、为谁做、接下来一年要去哪里。", "10 分钟"),
    ("02", "我们怎么做事", "三条做事原则，以及它们在日常工作里的样子。", "10 分钟"),
    ("03", "你的第一周", "每天一件主要的事，周五交一份三句话小结。", "15 分钟"),
    ("04", "找谁帮忙", "手续、设备、业务、报销，各有一个固定的人。", "10 分钟"),
]


def agenda():
    rows = []
    for index, (number, title, desc, minutes) in enumerate(AGENDA):
        if index:
            rows.append(k.divider())
        rows.append(k.row(f"议程 {title}", [
            k.para(number, 36, 700, "$primary", family=NUM, width=80,
                   lh=1.2, name=f"序号 {number}"),
            k.col(f"{title} 文字", [
                k.label(title, FS_ITEM, 600),
                k.para(desc, FS_BODY, 400, "$muted-foreground", lh=1.5),
            ], gap=6),
            k.pill(minutes, fill="$muted", color="$muted-foreground",
                   size=FS_META, pad=(5, 12)),
        ], gap=16, padding=[26, 0]))
    return page(2, "议程", [
        title_block("今天的四件事", "每一部分后面都留了提问时间，不用憋着。"),
        k.col("议程列表", rows),
    ], gap=20)


# ---------------------------------------------------------------- 03 原则
# 描述的断点写死在文案里：三栏卡的正文一行只排得下 12 个字，交给自动折行
# 会把「。」甩到下一行行首。
PRINCIPLES = [
    ("target", "结果先于过程",
     "先对齐要交付什么、\n给谁用、什么时候算完成，\n再决定怎么做。",
     "开工前写一句「完成的样子」", "先做再说，做完才发现不对"),
    ("message-square", "把话说在前面",
     "风险一出现就说，\n不等到截止前一天；\n坏消息越早越便宜。",
     "周会只讲卡点，不讲流水账", "截止前一天才说做不完"),
    ("scale", "对事不对人",
     "只讨论方案本身，\n不评判提方案的人；\n评审结论写进文档。",
     "评审结论 24 小时内同步", "在群里争论谁对谁错"),
]


def principles():
    cards = []
    for index, (glyph, title, desc, example, anti) in enumerate(PRINCIPLES,
                                                                 1):
        filler = frame(ids, f"{title} 留白", width="fill_container",
                       height="fill_container", layout="none", fill=[])
        cards.append(k.card(f"原则 {title}", [
            k.row(f"{title} 眉行", [
                k.label(f"0{index}", 44, 700, "$primary", family=NUM, lh=1.1),
                k.spacer(),
                k.tile(glyph, size=44, icon_size=22, fill="$secondary",
                       color="$primary", radius=12),
            ]),
            k.label(title, 26, 700),
            k.para(desc, FS_BODY, 400, "$muted-foreground", lh=1.6),
            filler,
            k.row(f"{title} 反例", [
                k.label("不", 15, 700, "$muted-foreground"),
                k.para(anti, 15, 400, "$muted-foreground", lh=1.45),
            ], gap=8, fill=solid("$muted"), padding=[10, 14],
                cornerRadius=10, align="start"),
            k.row(f"{title} 例子", [
                k.label("例", 15, 700, "$secondary-foreground"),
                k.para(example, 15, 500, "$secondary-foreground", lh=1.45),
            ], gap=8, fill=solid("$secondary"), padding=[10, 14],
                cornerRadius=10, align="start"),
        ], gap=16, pad=(28, 24), radius=16, height="fill_container"))
    grid = k.row("原则三栏", cards, gap=20, align="start",
                 height="fill_container")
    return page(3, "做事原则", [
        title_block("三条做事原则", "不是口号，是评审、周会和复盘里真正会用到的判断标准。"),
        grid,
    ])


# ---------------------------------------------------------------- 04 第一周
WEEK = [
    ("周一", "9/28", "认识团队", ["领设备、开账号", "和导师吃午饭"], True,
     "3F 前台"),
    ("周二", "9/29", "读懂业务", ["看三份核心文档", "旁听一次周会"], False,
     "团队文档库"),
    ("周三", "9/30", "第一次 1:1", ["和导师对齐目标", "定第一个小任务"], False,
     "导师工位"),
    ("周四", "10/1", "动手做", ["完成小任务", "提一个问题"], False, "项目群"),
    ("周五", "10/2", "交小结", ["写三句话小结", "约下周 1:1"], False,
     "周报系统"),
]


def week():
    cols = []
    for index, (day, date, title, todos, today, where) in enumerate(WEEK):
        dot = {"type": "ellipse", "id": ids("e"), "name": f"{day} 节点",
               "width": 16, "height": 16,
               "fill": solid("$primary" if today else "$background"),
               "stroke": stroke("$primary", 2)}
        # 末一天的连线画成底色：轴线在周五收住，而不是伸出画面。
        line = rect(ids, f"{day} 连线", width="fill_container", height=2,
                    fill=solid("$border" if index < len(WEEK) - 1
                               else "$background"))
        day_card = k.card(f"{day} 卡", [
            k.row(f"{day} 日期", [
                k.label(day, 17, 700, "$primary" if today else "$foreground"),
                k.label(date, FS_META, 500, "$muted-foreground", family=NUM),
            ], gap=8, align="end"),
            k.label(title, 22, 700),
            k.col(f"{day} 待办", [
                k.row(f"待办 {todo}", [
                    k.icon("check", 16, "$primary"),
                    k.para(todo, 16, 400, "$muted-foreground", lh=1.45),
                ], gap=6, align="start") for todo in todos
            ], gap=10),
            frame(ids, f"{day} 留白", width="fill_container",
                  height="fill_container", layout="none", fill=[]),
            k.row(f"{day} 地点", [
                k.icon("map-pin", 14, "$muted-foreground"),
                k.label(where, FS_META, 500, "$muted-foreground"),
            ], gap=6),
        ], gap=14, pad=(18, 14), radius=14, height="fill_container",
            fill="$card" if not today else "$card",
            border="$primary" if today else "$border")
        cols.append(k.col(f"第一周 {day}", [
            k.row(f"{day} 轴", [dot, line], gap=8),
            day_card,
        ], gap=16, height="fill_container"))
    timeline = k.row("时间轴", cols, gap=12, align="start",
                     height="fill_container")
    note = k.row("第一周提示", [
        k.icon("lightbulb", 20, "$secondary-foreground"),
        k.para("第一周不考核产出。真正要做到的只有一件事：知道每个问题该去问谁。",
               FS_BODY, 500, "$secondary-foreground", lh=1.5),
    ], gap=12, fill=solid("$secondary"), padding=[16, 20], cornerRadius=12,
        align="start")
    return page(4, "第一周", [
        title_block("你的第一周", "每天只安排一件主要的事，剩下的时间留给认识人和提问题。"),
        timeline, note,
    ], gap=28)


# ---------------------------------------------------------------- 05 找谁
CONTACTS = [
    ("file-text", "入职手续与假期", "合同、社保、考勤、请假流程",
     "年假从哪天开始算？", "周", "小周 · 人事伙伴"),
    ("laptop", "电脑与账号", "设备领取、权限开通、网络问题",
     "代码仓库的权限找谁开？", "服", "服务台 · 工作日 9–18 点"),
    ("compass", "业务与任务", "要做什么、先做什么、做到什么程度",
     "这个需求做到哪一步算完？", "林", "林澈 · 你的导师"),
    ("receipt", "报销与采购", "差旅、办公用品、发票要求",
     "打车票要不要写事由？", "财", "财务共享 · 每周二处理"),
]


def contact_card(glyph, topic, desc, question, initial, who):
    person = k.row(f"{topic} 联系人", [
        k.avatar(initial, size=32, fill="$secondary",
                 color="$secondary-foreground", font=13),
        k.label(who, 16, 600),
        k.spacer(),
        k.label("内部通讯", FS_META, 500, "$muted-foreground"),
    ], gap=10, padding=[14, 0, 0, 0])
    person["stroke"] = {"thickness": [1, 0, 0, 0], "fill": solid("$border")}
    return k.card(f"联系 {topic}", [
        k.row(f"{topic} 标题", [
            k.tile(glyph, size=44, icon_size=22, fill="$muted",
                   color="$foreground", radius=12),
            k.col(f"{topic} 文字", [
                k.label(topic, 22, 700),
                k.label(desc, 16, 400, "$muted-foreground"),
            ], gap=4),
        ], gap=14),
        k.row(f"{topic} 例问", [
            k.icon("message-circle", 16, "$secondary-foreground"),
            k.label(f"比如：{question}", 16, 500, "$secondary-foreground"),
        ], gap=8),
        frame(ids, f"{topic} 留白", width="fill_container",
              height="fill_container", layout="none", fill=[]),
        person,
    ], gap=16, pad=(22, 24), radius=14, height="fill_container")


def contacts():
    rows = [k.row(f"联系第 {i + 1} 行", [contact_card(*c) for c in pair],
                  gap=20, align="start", height="fill_container")
            for i, pair in enumerate([CONTACTS[:2], CONTACTS[2:]])]
    return page(5, "找谁帮忙", [
        title_block("遇到问题，找对的人", "四类问题各有一个固定的人，不用在群里广播。"),
        k.col("联系网格", rows, gap=20, height="fill_container"),
    ])


# ---------------------------------------------------------------- 06 结尾
TODOS = [
    ("今天下班前", "完成入职信息登记，确认工牌和门禁可用。"),
    ("本周三之前", "和导师约好第一次 1:1，带着三个问题去。"),
    ("本周五", "提交第一周小结，三句话就够：学到了什么、卡在哪、下周做什么。"),
]


def closing():
    items = []
    for when, what in TODOS:
        items.append(k.row(f"清单 {when}", [
            k.row(f"{when} 勾", [k.icon("check", 16, "$inverse")],
                  width=28, height=28, justify="center", cornerRadius=14,
                  fill=solid("$inverse-accent")),
            k.col(f"{when} 文字", [
                k.label(when, 16, 700, "$inverse-accent"),
                k.para(what, 20, 400, "$inverse-foreground", lh=1.5),
            ], gap=4),
        ], gap=16, align="start"))
    body = k.col("结尾 · 正文", [
        k.col("结尾标题", [
            k.para("有问题，随时问。", FS_COVER, 700, "$inverse-foreground",
                   lh=1.15, name="结尾主标题"),
            k.para("这份材料和课程录屏放在学习中心的「新人专区」，\n第一个月内随时回看。",
                   FS_LEAD, 400, "$inverse-muted", lh=1.5,
                   name="结尾副标题"),
        ], gap=18),
        k.col("本周清单", [
            k.label("本周只做这三件事", 16, 600, "$inverse-muted"),
        ] + items, gap=20),
    ], gap=64, justify="center")
    return board(k, ids, "06 结尾", body,
                 footer(k, ids, LABEL, 6, TOTAL, rule="$inverse-border",
                        color="$inverse-muted"),
                 fill="$inverse")


def build():
    return place([cover(), agenda(), principles(), week(), contacts(),
                  closing()])


if __name__ == "__main__":
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                       "..", "crates", "op-editor-core", "assets",
                       "scene_templates", "onboarding-training-deck.op")
    write_doc(os.path.normpath(out), VARS, build(), "新员工入职培训 · 4:3",
              compact=True)

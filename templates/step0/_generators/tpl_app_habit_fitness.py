#!/usr/bin/env python3
"""habit-fitness-app.op — 习惯与健身 App 三屏（375×812 × 3，深色）

App 界面场景的第二套：把「每天的小习惯」和「一节训练课」放进同一个 App。
三屏：今日（活动量 + 连续打卡 + 习惯清单）→ 课程详情（封面图位 + 动作
清单 + 开始键）→ 本周数据（每日时长柱图 + 睡眠心率 + 本周最佳）。
风格落在 `health-minimal-mobile-dark` 档：深炭底、活力绿主色、Inter 数字。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：清晨跑道 —— 天没亮透的炭灰、跑鞋上的一道荧绿、手表屏的冷光。
  - **收敛**：#111111 底 + #1A1A1A 卡 + #222222 抬升面；主色活力绿
    #22C55E（档案原值）只给「完成」这一个语义：进度条、打卡圆、主按钮。
    三支辅色（天蓝 / 琥珀 / 紫罗兰）只做图标块的类别区分。
  - **论证**：习惯 App 的正反馈就是「变绿」。所以绿色不拿去装饰标题或
    卡片边 —— 整屏只要出现绿色，就一定意味着「这件事做到了」。
    未完成用空心描边圆，不用红：打卡断一天就满屏标红，只会劝退。

### 负约束

  - 不放任何健身品牌、手表品牌的名称与图形。
  - 课程头图是「跟练视频」卡（时长 + 播放键），不放空图片槽 —— 空槽在
    深色界面里会载入成一块浅灰亮斑。
  - 不做发光、霓虹描边、渐变卡片 —— 档案要求的是克制的深色。
  - 柱高由真实分钟数换算（见 MINUTES）；未发生的周六周日画成 4px 的
    底线桩，而不是省略 —— 一周七天的节奏要看得出来。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from appkit import NUM, Kit
from oplib import Ids, color_vars, frame, rect, solid, stroke, write_doc

ids = Ids()
k = Kit(ids)

VARS = color_vars({
    "background":           "#111111",
    "foreground":           "#F5F5F5",
    "card":                 "#1A1A1A",
    "card-foreground":      "#F5F5F5",
    "secondary":            "#222222",
    "secondary-foreground": "#F5F5F5",
    "primary":              "#22C55E",
    "primary-foreground":   "#052E16",
    "muted":                "#262626",
    "muted-foreground":     "#A3A3A3",
    "accent":               "#15301F",
    "accent-foreground":    "#4ADE80",
    "border":               "#2A2A2A",
    "ring":                 "#22C55E",
    "chart-2":              "#38BDF8",
    "chart-2-soft":         "#10283A",
    "chart-3":              "#FBBF24",
    "chart-3-soft":         "#33280F",
    "chart-4":              "#A78BFA",
    "chart-4-soft":         "#261F3D",
    "chart-5":              "#F87171",
    "chart-5-soft":         "#3A1C1C",
})

PAD = 20
INNER = 375 - PAD * 2          # 335
CARD_INNER = INNER - 36        # 299（卡片左右各 18）

TABS = [("calendar-check", "今日", "/"), ("dumbbell", "训练", "/workout"),
        ("chart-column", "数据", "/stats"), ("user", "我的")]


def body(name, children, *, gap=16, pad_top=8):
    return k.col(name, children, gap=gap, height="fill_container",
                 padding=[pad_top, PAD, 16, PAD], clipContent=True)


def card(name, children, *, gap=14, pad=(18, 18), fill="$card"):
    return k.card(name, children, gap=gap, pad=pad, fill=fill,
                  border="$border", radius=20)


def section_head(title, action=None):
    kids = [k.label(title, 17, 700), k.spacer()]
    if action:
        kids.append(k.label(action, 13, 500, "$muted-foreground"))
    return k.row(f"区块标题 {title}", kids)


def check_circle(done, *, size=28, name="完成标记"):
    if not done:
        # 未完成是一个空心圆 —— 用 ellipse 叶子画，而不是空 frame：空的
        # 描边 frame 在几何质检里就是「没填内容的装饰壳」。
        return {"type": "ellipse", "id": ids("e"), "name": f"{name} · 未完成",
                "width": size, "height": size, "fill": [],
                "stroke": stroke("$muted-foreground", 2)}
    return k.row(name, [k.icon("check", round(size * 0.57),
                               "$primary-foreground")],
                 width=size, height=size, justify="center",
                 fill=solid("$primary"), cornerRadius=size / 2)


# ---------------------------------------------------------------- 01 今日
def mini_stat(glyph, tone, value, unit, caption):
    return k.col(f"小指标 {caption}", [
        k.tile(glyph, size=28, icon_size=15, fill=f"${tone}-soft",
               color=f"${tone}", radius=8),
        k.row(f"{caption} 数值", [
            k.label(value, 17, 700, family=NUM, lh=1.2),
            k.label(unit, 11, 400, "$muted-foreground"),
        ], gap=3, width="fit_content", align="end"),
        k.label(caption, 11, 400, "$muted-foreground"),
    ], gap=6)


def activity_card():
    figure = k.col("步数", [
        k.label("今日步数", 13, 500, "$muted-foreground"),
        k.row("步数行", [
            k.label("8,432", 34, 700, family=NUM, lh=1.1, spacing=-0.5),
            k.label("/ 10,000", 13, 400, "$muted-foreground", family=NUM),
        ], gap=8, width="fit_content", align="end"),
    ], gap=4, width="fit_content")
    head = k.row("活动卡眉行", [
        figure,
        k.spacer(),
        k.pill("已完成 84%", fill="$accent", color="$accent-foreground",
               size=11),
    ], align="end")
    stats = k.row("三项小指标", [
        mini_stat("flame", "chart-3", "426", "千卡", "消耗"),
        mini_stat("timer", "chart-2", "38", "分钟", "运动"),
        mini_stat("footprints", "chart-4", "5.6", "公里", "距离"),
    ], gap=12, align="start")
    for cell in stats["children"]:
        cell["width"] = "fill_container"
    return card("今日活动卡", [head,
                               k.bar(0.84, CARD_INNER, height=8,
                                     name="步数进度"),
                               stats], gap=14)


DAYS = [("一", "done", "21"), ("二", "done", "22"), ("三", "done", "23"),
        ("四", "done", "24"), ("五", "today", "25"), ("六", "todo", "26"),
        ("日", "todo", "27")]


def day_cell(day, state, date):
    if state == "done":
        dot = check_circle(True, size=34, name=f"周{day} 已打卡")
    else:
        dot = k.row(f"周{day} 圆", [
            k.label(date, 12, 600,
                    "$primary" if state == "today" else "$muted-foreground",
                    family=NUM, lh=1.2),
        ], width=34, height=34, justify="center", cornerRadius=17,
            fill=solid("$muted") if state == "todo" else None)
        if state == "today":
            dot["stroke"] = stroke("$primary", 2)
    return k.col(f"周{day}", [
        k.label(day, 12, 500, "$foreground" if state == "today"
                else "$muted-foreground"),
        dot,
    ], gap=8, align="center", width=36)


def streak_card():
    return card("连续打卡卡", [
        k.row("打卡卡标题", [
            k.label("连续打卡", 15, 600),
            k.spacer(),
            k.pill("12 天", glyph="flame", fill="$chart-3-soft",
                   color="$chart-3", size=12),
        ]),
        k.row("一周七天", [day_cell(*day) for day in DAYS],
              justify="space_between", align="start"),
    ], gap=16)


HABITS = [
    ("sunrise", "chart-3", "7:00 前起床", "已完成 · 6:42", True),
    ("brain", "chart-4", "冥想 10 分钟", "已完成 · 7:15", True),
    ("droplet", "chart-2", "喝够 8 杯水", "6 / 8 杯 · 还差 2 杯", False),
]


def habit_row(glyph, tone, title, sub, done):
    return k.row(f"习惯 {title}", [
        k.tile(glyph, size=40, icon_size=20, fill=f"${tone}-soft",
               color=f"${tone}", radius=12),
        k.col(f"{title} 文字", [
            k.label(title, 15, 500),
            k.label(sub, 12, 400,
                    "$accent-foreground" if done else "$muted-foreground"),
        ], gap=3),
        check_circle(done),
    ], gap=12, padding=[12, 0])


def today():
    head = k.row("问候行", [
        k.col("问候文字", [
            k.label("周五 · 9 月 25 日", 13, 400, "$muted-foreground"),
            k.label("今天也动一动", 24, 700, lh=1.25),
        ], gap=4),
        k.avatar("周", size=40, fill="$secondary",
                 color="$secondary-foreground"),
    ], gap=12)
    rows = []
    for index, habit in enumerate(HABITS):
        if index:
            rows.append(k.divider())
        rows.append(habit_row(*habit))
    habits = k.col("今日习惯", [
        section_head("今日习惯", "2 / 3 完成"),
        card("习惯清单卡", rows, gap=0, pad=(2, 16)),
    ], gap=12)
    content = body("今日内容", [head, activity_card(), streak_card(),
                                habits])
    return k.phone("01 今日", "/", content, tab=k.tab_bar(TABS, 0), index=0)


# ---------------------------------------------------------------- 02 课程
MOVES = [
    ("01", "猫牛式脊柱唤醒", "10 次 · 跟随呼吸", "0:45"),
    ("02", "站姿侧屈拉伸", "左右各 30 秒", "1:00"),
    ("03", "低弓步髋屈肌拉伸", "左右各 40 秒", "1:20"),
]


def move_row(number, name, sub, duration):
    return k.row(f"动作 {name}", [
        k.row(f"序号 {number}", [
            k.label(number, 13, 700, "$primary", family=NUM),
        ], width=36, height=36, justify="center", cornerRadius=10,
            fill=solid("$accent")),
        k.col(f"{name} 文字", [
            k.label(name, 15, 500),
            k.label(sub, 12, 400, "$muted-foreground"),
        ], gap=3),
        k.label(duration, 13, 500, "$muted-foreground", family=NUM),
    ], gap=12, padding=[10, 0])


def meta_cell(glyph, value, caption):
    cell = k.col(f"课程参数 {caption}", [
        k.icon(glyph, 18, "$primary"),
        k.label(value, 15, 600),
        k.label(caption, 11, 400, "$muted-foreground"),
    ], gap=4, align="center", fill=solid("$card"), padding=[12, 8],
        cornerRadius=14)
    cell["stroke"] = stroke("$border", 1)
    return cell


def workout():
    back = k.tile("chevron-left", size=36, icon_size=20, fill="$card",
                  radius=18, name="返回")
    back["events"] = {"onTap": [{"pop": None}]}
    save = k.tile("bookmark", size=36, icon_size=18, fill="$card", radius=18)
    bar = k.row("顶部栏", [
        back, k.row("顶部栏标题", [k.label("课程详情", 17, 600)],
                    justify="center"), save,
    ], height=52, gap=8)
    # 课程头图做成「跟练视频」卡而不是空图片槽：空 image 节点在载入时被
    # 统一刷成浅灰占位（op-pen-loader image_to_payload），压在深色界面里是一
    # 块刺眼的亮斑；这张卡本身就是完整的界面，不依赖配图。
    play = k.row("播放键", [k.icon("play", 26, "$primary-foreground")],
                 width=60, height=60, justify="center", cornerRadius=30,
                 fill=solid("$primary"), role="button")
    cover = k.row("跟练视频卡", [
        k.col("跟练视频文字", [
            k.pill("跟练视频", glyph="video", fill="$background",
                   color="$accent-foreground", size=11, pad=(3, 8)),
            k.label("15:00", 34, 700, family=NUM, lh=1.1, spacing=-0.5),
            k.label("12 个动作 · 跟着做就行", 12, 400, "$muted-foreground"),
        ], gap=8),
        play,
    ], gap=16, height=150, padding=[20, 22], fill=solid("$accent"),
        cornerRadius=20)
    intro = k.col("课程简介", [
        k.label("晨间唤醒 · 全身拉伸", 22, 700, lh=1.3),
        k.para("12 个动作从颈肩到脚踝，唤醒一夜僵硬的关节，适合起床后空腹练习。",
               13, 400, "$muted-foreground", lh=1.6),
    ], gap=6)
    metas = k.row("课程参数", [
        meta_cell("clock", "15 分钟", "时长"),
        meta_cell("gauge", "初级", "难度"),
        meta_cell("flame", "约 90 千卡", "消耗"),
    ], gap=10, align="start")
    for cell in metas["children"]:
        cell["width"] = "fill_container"
    rows = []
    for index, move in enumerate(MOVES):
        if index:
            rows.append(k.divider())
        rows.append(move_row(*move))
    moves = k.col("动作清单", [
        section_head("动作清单", "共 12 个"),
        k.col("动作列表", rows),
    ], gap=6)
    filler = frame(ids, "弹性留白", width="fill_container",
                   height="fill_container", layout="none", fill=[])
    start = k.row("开始训练按钮", [
        k.icon("play", 18, "$primary-foreground"),
        k.label("开始训练", 16, 600, "$primary-foreground"),
    ], gap=8, justify="center", height=52, fill=solid("$primary"),
        cornerRadius=16, role="button")
    content = k.col("课程内容", [bar, cover, intro, metas, moves, filler,
                                  start],
                    gap=14, height="fill_container", padding=[0, PAD, 8, PAD],
                    clipContent=True)
    return k.phone("02 课程详情", "/workout", content, index=1)


# ---------------------------------------------------------------- 03 数据
# 本周每日运动分钟数；今天是周五，周末尚未发生。
MINUTES = [35, 48, 20, 52, 38, 0, 0]
WEEK = ["一", "二", "三", "四", "五", "六", "日"]


def stat_tile(glyph, tone, caption, value, unit, note):
    tile = card(f"指标卡 {caption}", [
        k.row(f"{caption} 眉行", [
            k.tile(glyph, size=28, icon_size=15, fill=f"${tone}-soft",
                   color=f"${tone}", radius=8),
            k.label(caption, 12, 500, "$muted-foreground"),
        ], gap=8),
        k.row(f"{caption} 数值", [
            k.label(value, 22, 700, family=NUM, lh=1.2),
            k.label(unit, 12, 400, "$muted-foreground"),
        ], gap=4, width="fit_content", align="end"),
        k.label(note, 11, 400, "$muted-foreground"),
    ], gap=10, pad=(16, 16))
    tile["width"] = "fill_container"
    return tile


BESTS = [
    ("trophy", "chart-3", "最长一次跑步", "5.2 公里 · 周二"),
    ("footprints", "chart-2", "单日最多步数", "12,480 步 · 周四"),
    ("brain", "chart-4", "冥想累计", "70 分钟 · 7 次"),
]


def stats():
    head = k.row("数据标题行", [
        k.label("本周数据", 28, 700, lh=1.2),
        k.spacer(),
        k.segmented(["周", "月"], 0, width=96, on_fill="$secondary",
                    track="$card"),
    ])
    total = k.row("本周总时长", [
        k.label("3 小时 13 分", 26, 700, lh=1.2),
        k.pill("比上周 +38 分钟", glyph="trending-up", fill="$accent",
               color="$accent-foreground", size=11),
    ], gap=10)
    chart = card("每日时长图卡", [
        k.row("图卡标题", [
            k.label("每日运动时长", 15, 600),
            k.spacer(),
            k.label("单位：分钟", 11, 400, "$muted-foreground"),
        ]),
        total,
        k.chart_bars(MINUTES, max_h=100, bar_w=22, labels=WEEK, highlight=4,
                     value_labels=[str(m) if m else "–" for m in MINUTES],
                     off="$muted", name="本周每日时长柱图"),
    ], gap=14)
    pair = k.row("睡眠与心率", [
        stat_tile("moon", "chart-4", "平均睡眠", "7:18", "小时", "比上周多 22 分钟"),
        stat_tile("heart-pulse", "chart-5", "静息心率", "58", "次/分",
                  "处于良好区间"),
    ], gap=12, align="start")
    rows = []
    for index, (glyph, tone, title, value) in enumerate(BESTS):
        if index:
            rows.append(k.divider())
        rows.append(k.row(f"最佳 {title}", [
            k.tile(glyph, size=32, icon_size=16, fill=f"${tone}-soft",
                   color=f"${tone}", radius=10),
            k.label(title, 14, 500),
            k.spacer(),
            k.label(value, 13, 500, "$muted-foreground"),
        ], gap=12, padding=[10, 0]))
    best = k.col("本周最佳", [
        section_head("本周最佳"),
        card("本周最佳卡", rows, gap=0, pad=(2, 16)),
    ], gap=10)
    content = body("数据内容", [head, chart, pair, best])
    return k.phone("03 本周数据", "/stats", content, tab=k.tab_bar(TABS, 2),
                   index=2)


def build():
    return [today(), workout(), stats()]


if __name__ == "__main__":
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                       "..", "crates", "op-editor-core", "assets",
                       "scene_templates", "habit-fitness-app.op")
    write_doc(os.path.normpath(out), VARS, build(), "习惯与健身 App 三屏",
              compact=True)

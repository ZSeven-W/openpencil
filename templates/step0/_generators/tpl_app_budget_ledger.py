#!/usr/bin/env python3
"""budget-ledger-app.op — 记账 App 三屏（375×812 × 3）

App 界面场景的第一套：个人记账。三屏走一条完整的使用路径 —— 首页看结余、
统计页看钱花到了哪里、记一笔页把一笔支出录进去。风格落在
`finance-clean-mobile-light` 档：冷白底、信任青主色、数字优先。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：银行柜台的凭条与计算器 —— 冷白纸、墨色数字、一枚青色印章。
  - **收敛**：冷白 #F8FAFC 底 + 白卡 + 石板墨 #0F172A；唯一有彩主色为信任青
    #0D9488（档案原值）。分类色五支（琥珀 / 蓝 / 青 / 玫红 / 紫）只出现在
    账目图标块与分类条上，**不进任何文字底**。
  - **论证**：记账 App 的主角是数字，颜色只做两件事 —— 标出「主操作」
    （结余卡、记一笔键、完成键）与区分「钱去了哪一类」。收入用成功绿、支出
    用正文墨色而不是红：每天几十笔支出全标红，首页会读成一张警报单。

### 负约束

  - 不放任何银行 / 支付品牌的 logo 与名称；账户写「工资卡 · 尾号 6021」。
  - 不画假地图、不放装饰插画、不留无内容的色块。
  - 图表柱高由真实数值换算（见 SPEND），不是随手写的高度。
  - 头像用首字，不放人像照片。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from appkit import CJK, NUM, Kit
from oplib import Ids, color_vars, frame, rect, solid, stroke, write_doc

ids = Ids()
k = Kit(ids)

VARS = color_vars({
    "background":           "#F8FAFC",
    "foreground":           "#0F172A",
    "card":                 "#FFFFFF",
    "card-foreground":      "#0F172A",
    "primary":              "#0D9488",
    "primary-foreground":   "#FFFFFF",
    "primary-strong":       "#0F766E",
    "secondary":            "#CCFBF1",
    "secondary-foreground": "#115E59",
    "muted":                "#F1F5F9",
    "muted-foreground":     "#64748B",
    "accent":               "#CCFBF1",
    "accent-foreground":    "#115E59",
    "border":               "#E2E8F0",
    "ring":                 "#0D9488",
    "destructive":          "#DC2626",
    "--color-success":      "#047857",
    "--color-success-soft": "#D1FAE5",
    "chart-1":              "#B45309",
    "chart-1-soft":         "#FEF3C7",
    "chart-2":              "#1D4ED8",
    "chart-2-soft":         "#DBEAFE",
    "chart-3":              "#0F766E",
    "chart-3-soft":         "#CCFBF1",
    "chart-4":              "#BE185D",
    "chart-4-soft":         "#FCE7F3",
    "chart-5":              "#6D28D9",
    "chart-5-soft":         "#EDE9FE",
})

PAD = 20
INNER = 375 - PAD * 2          # 335
CARD_INNER = INNER - 32        # 303

TABS = [("house", "首页", "/"), ("chart-pie", "统计", "/stats"),
        ("wallet", "预算"), ("user", "我的")]


def fab():
    node = k.col("记一笔主键", [k.icon("plus", 26, "$primary-foreground")],
                 width=52, height=52, align="center", justify="center",
                 fill=solid("$primary"), cornerRadius=26)
    node["effects"] = [{"type": "shadow", "offsetX": 0, "offsetY": 6,
                        "blur": 14, "spread": 0,
                        "color": "rgba(13,148,136,0.28)"}]
    node["events"] = {"onTap": [{"push": "/add"}]}
    wrap = k.col("主键格", [node], width=64, align="center")
    return wrap


def body(name, children, *, gap=20, pad_top=8):
    node = k.col(name, children, gap=gap, height="fill_container",
                 padding=[pad_top, PAD, 16, PAD], clipContent=True)
    return node


def section_head(title, action=None):
    kids = [k.label(title, 17, 700), k.spacer()]
    if action:
        kids.append(k.label(action, 13, 500, "$primary"))
    return k.row(f"区块标题 {title}", kids)


# ---------------------------------------------------------------- 01 首页
def money_line(glyph, caption, amount):
    chip = k.tile(glyph, size=32, icon_size=16, fill="$primary-strong",
                  color="$primary-foreground", radius=10)
    return k.row(f"结余分项 {caption}", [
        chip,
        k.col(f"{caption} 文字", [
            k.label(caption, 12, 400, "$accent"),
            k.label(amount, 15, 600, "$primary-foreground", family=NUM),
        ], gap=2),
    ], gap=10)


def balance_card():
    figure = k.row("结余数字", [
        k.label("¥", 20, 600, "$primary-foreground", family=NUM),
        k.label("8,426.50", 36, 700, "$primary-foreground", family=NUM,
                lh=1.1, spacing=-0.5),
    ], gap=4, width="fit_content", align="end")
    head = k.row("结余卡眉行", [
        k.label("9 月结余", 13, 500, "$accent"),
        k.spacer(),
        k.pill("较 8 月 +¥1,208", fill="$primary-strong",
               color="$primary-foreground", size=11, glyph="trending-up"),
    ])
    split = k.row("收支两栏", [
        money_line("arrow-down-left", "收入", "¥15,200.00"),
        money_line("arrow-up-right", "支出", "¥6,773.50"),
    ], gap=12)
    split["children"][0]["width"] = "fill_container"
    split["children"][1]["width"] = "fill_container"
    card = k.col("结余卡", [head, figure, split], gap=14,
                 fill=solid("$primary"), padding=[20, 20], cornerRadius=20)
    return card


def budget_card():
    return k.card("预算卡", [
        k.row("预算卡标题", [
            k.label("9 月预算", 15, 600),
            k.spacer(),
            k.label("¥10,000", 13, 500, "$muted-foreground", family=NUM),
        ]),
        k.bar(0.68, CARD_INNER, height=8, name="预算进度"),
        k.row("预算卡脚注", [
            k.label("已用 68% · ¥6,773.50", 12, 400, "$muted-foreground"),
            k.spacer(),
            k.label("剩 6 天 · 日均可花 ¥537", 12, 600, "$primary"),
        ]),
    ], gap=12)


TRANSACTIONS = [
    ("utensils", "chart-1", "午餐 · 轻食沙拉", "餐饮 · 今天 12:30", "-¥38.00", False),
    ("car", "chart-2", "打车回家", "交通 · 昨天 22:14", "-¥26.50", False),
    ("briefcase", "chart-3", "9 月工资", "收入 · 9 月 20 日", "+¥15,200.00", True),
]


def transaction_row(glyph, tone, title, sub, amount, income):
    return k.row(f"账目 {title}", [
        k.tile(glyph, size=40, icon_size=20, fill=f"${tone}-soft",
               color=f"${tone}", radius=12),
        k.col(f"{title} 文字", [
            k.label(title, 15, 500),
            k.label(sub, 12, 400, "$muted-foreground"),
        ], gap=3),
        k.label(amount, 15, 600,
                "$--color-success" if income else "$foreground", family=NUM),
    ], gap=12, padding=[12, 0])


def home():
    greet = k.row("问候行", [
        k.col("问候文字", [
            k.label("早上好，", 13, 400, "$muted-foreground"),
            k.label("林夏", 22, 700, lh=1.25),
        ], gap=2),
        k.tile("bell", size=40, icon_size=20, fill="$card", radius=20),
        k.avatar("林", size=40, fill="$secondary",
                 color="$secondary-foreground"),
    ], gap=10)
    greet["children"][1]["stroke"] = stroke("$border", 1)
    rows = []
    for index, item in enumerate(TRANSACTIONS):
        if index:
            rows.append(k.divider())
        rows.append(transaction_row(*item))
    ledger = k.card("最近账单卡", rows, gap=0, pad=(4, 16))
    content = body("首页内容", [
        greet, balance_card(), budget_card(),
        k.col("最近账单", [section_head("最近账单", "查看全部"), ledger],
              gap=12),
    ])
    return k.phone("01 首页", "/", content,
                   tab=k.tab_bar(TABS, 0, fab=fab()), index=0)


# ---------------------------------------------------------------- 02 统计
# 近六个月支出（元）。柱高按数值换算，最高那根是 8 月。
SPEND = [5820, 7140, 6390, 8010, 7690, 6773.5]
SPEND_LABELS = ["4 月", "5 月", "6 月", "7 月", "8 月", "9 月"]
SPEND_TAGS = ["5.8k", "7.1k", "6.4k", "8.0k", "7.7k", "6.8k"]

CATEGORIES = [
    ("utensils", "chart-1", "餐饮", 0.32, "¥2,167"),
    ("house", "chart-3", "住房", 0.28, "¥1,897"),
    ("car", "chart-2", "交通", 0.14, "¥948"),
    ("shopping-bag", "chart-4", "购物", 0.12, "¥813"),
    ("ellipsis", "chart-5", "其他", 0.14, "¥948"),
]


def category_row(glyph, tone, name, share, amount):
    track = CARD_INNER - 32 - 12
    return k.row(f"分类 {name}", [
        k.tile(glyph, size=32, icon_size=16, fill=f"${tone}-soft",
               color=f"${tone}", radius=10),
        k.col(f"{name} 明细", [
            k.row(f"{name} 数字行", [
                k.label(name, 14, 500),
                k.spacer(),
                k.label(amount, 13, 600, family=NUM),
                k.label(f"{round(share * 100)}%", 13, 400,
                        "$muted-foreground", family=NUM),
            ], gap=8),
            k.bar(share / CATEGORIES[0][3], track, height=6,
                  fill=f"${tone}", name=f"{name} 占比条"),
        ], gap=8),
    ], gap=12)


def stats():
    head = k.row("统计标题行", [
        k.label("统计", 28, 700, lh=1.2),
        k.spacer(),
        k.segmented(["周", "月", "年"], 1, width=132),
    ])
    summary = k.col("本月支出", [
        k.label("9 月支出", 13, 400, "$muted-foreground"),
        k.row("本月支出数字行", [
            k.label("¥6,773.50", 32, 700, family=NUM, lh=1.15, spacing=-0.5),
            k.pill("较 8 月 -12%", fill="$--color-success-soft",
                   color="$--color-success", size=12, glyph="trending-down"),
        ], gap=10, align="center"),
    ], gap=4)
    chart = k.card("月度支出图卡", [
        k.row("图卡标题", [
            k.label("近 6 个月", 15, 600),
            k.spacer(),
            k.label("单位：元", 12, 400, "$muted-foreground"),
        ]),
        k.chart_bars(SPEND, max_h=96, bar_w=26, labels=SPEND_LABELS,
                     highlight=5, value_labels=SPEND_TAGS, off="$secondary",
                     name="近六月支出柱图"),
    ], gap=16)
    breakdown = k.card("分类占比卡", [
        k.row("分类卡标题", [
            k.label("分类占比", 15, 600),
            k.spacer(),
            k.label("共 86 笔", 12, 400, "$muted-foreground"),
        ]),
    ] + [category_row(*item) for item in CATEGORIES], gap=14)
    content = body("统计内容", [head, summary, chart, breakdown], gap=16)
    return k.phone("02 统计", "/stats", content,
                   tab=k.tab_bar(TABS, 1, fab=fab()), index=1)


# ---------------------------------------------------------------- 03 记一笔
PICKS = [("utensils", "餐饮"), ("car", "交通"), ("shopping-bag", "购物"),
         ("house", "住房"), ("gamepad-2", "娱乐"), ("pill", "医疗"),
         ("book-open", "学习"), ("ellipsis", "更多")]


def pick_cell(glyph, name, on):
    tile = k.tile(glyph, size=48, icon_size=22,
                  fill="$primary" if on else "$card",
                  color="$primary-foreground" if on else "$foreground",
                  radius=16)
    if not on:
        # 白块压在冷白底上只差 2% 明度，没有描边就只剩一个悬空的图标。
        tile["stroke"] = stroke("$border", 1)
    return k.col(f"分类格 {name}", [
        tile,
        k.label(name, 12, 600 if on else 400,
                "$foreground" if on else "$muted-foreground"),
    ], gap=6, align="center", width=PICK_W)


# 键盘内宽 343 − 两侧 12 = 319，四键三缝：(319 − 3 × 8) / 4 ≈ 73。
# 键写定宽而不是 fill_container：定宽格子由 space_between 分余量，四列永远
# 等宽，也不会被几何质检当成「四列文字表格」按 120 的可读地板去卡。
KEY_W = 73
PICK_W = 72


def key(label, *, primary=False, glyph=None):
    content = (k.icon(glyph, 22, "$foreground") if glyph else
               k.para(label, 15 if primary else 22,
                      600 if primary else 500,
                      "$primary-foreground" if primary else "$foreground",
                      family=CJK if primary else NUM, lh=1.2, width=KEY_W,
                      align="center", name=f"键面 {label}"))
    node = k.row(f"键 {label}", [content], justify="center", height=52,
                 width=KEY_W,
                 fill=solid("$primary" if primary else "$card"),
                 cornerRadius=12)
    if not primary:
        node["stroke"] = stroke("$border", 1)
    return node


def add_entry():
    close = k.tile("x", size=36, icon_size=18, fill="$muted", radius=18,
                   name="关闭")
    close["events"] = {"onTap": [{"pop": None}]}
    bar = k.row("顶部栏", [
        k.row("顶部栏左", [close], width=72),
        k.row("顶部栏中", [k.label("记一笔", 17, 600)], justify="center"),
        k.row("顶部栏右", [k.label("存为模板", 13, 500, "$primary")],
              width=72, justify="end"),
    ], height=52, gap=8)
    seg = k.row("收支切换行", [k.segmented(["支出", "收入", "转账"], 0,
                                         width=210)], justify="center")
    amount = k.col("金额", [
        k.label("支出金额", 13, 400, "$muted-foreground"),
        k.row("金额数字", [
            k.label("¥", 24, 600, family=NUM),
            k.label("128.00", 44, 700, family=NUM, lh=1.1, spacing=-1),
        ], gap=6, width="fit_content", align="end"),
    ], gap=6, align="center")
    grid = k.col("分类网格", [
        k.row("分类第一行", [pick_cell(g, n, i == 0)
                             for i, (g, n) in enumerate(PICKS[:4])],
              justify="space_between", align="start"),
        k.row("分类第二行", [pick_cell(g, n, False) for g, n in PICKS[4:]],
              justify="space_between", align="start"),
    ], gap=16, padding=[0, 8])
    chips = k.row("附加信息", [
        k.pill("今天", glyph="calendar", fill="$card",
               border="$border", pad=(6, 10)),
        k.pill("工资卡 · 6021", glyph="credit-card", fill="$card",
               border="$border", pad=(6, 10)),
        k.pill("和同事聚餐", glyph="pencil", fill="$card",
               border="$border", pad=(6, 10)),
    ], gap=8, justify="center")
    keys = [["1", "2", "3", None], ["4", "5", "6", "+"],
            ["7", "8", "9", "−"], [".", "0", "00", "完成"]]
    pad_rows = []
    for index, line in enumerate(keys):
        cells = []
        for label in line:
            if label is None:
                cells.append(key("退格", glyph="delete"))
            else:
                cells.append(key(label, primary=label == "完成"))
        pad_rows.append(k.row(f"键盘第 {index + 1} 行", cells,
                              justify="space_between"))
    keypad = k.col("数字键盘", pad_rows, gap=8, fill=solid("$muted"),
                   padding=[12, 12, 12, 12], cornerRadius=20)
    filler = frame(ids, "弹性留白", width="fill_container",
                   height="fill_container", layout="none", fill=[])
    content = k.col("记一笔内容", [bar, seg, amount, grid, chips, filler,
                                    keypad],
                    gap=18, height="fill_container",
                    padding=[0, 16, 8, 16], clipContent=True)
    return k.phone("03 记一笔", "/add", content, index=2)


def build():
    return [home(), stats(), add_entry()]


if __name__ == "__main__":
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                       "..", "crates", "op-editor-core", "assets",
                       "scene_templates", "budget-ledger-app.op")
    write_doc(os.path.normpath(out), VARS, build(), "记账 App 三屏",
              compact=True)

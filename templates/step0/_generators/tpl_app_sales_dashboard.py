#!/usr/bin/env python3
"""sales-dashboard-web.op — 桌面端经营看板（1440×900 单屏）

App 界面场景的桌面一套：电商 / SaaS 后台最常见的「概览」页。侧栏导航 +
顶栏（标题、搜索、时间范围、导出）+ 四张指标卡 + 收入趋势双序列柱图 +
渠道占比 + 最近订单表。风格落在 `saas-clean-light` 档：纯白卡面、浅灰底、
靛蓝主色、Inter 数字。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：办公桌上的一份打印周报 —— 白纸、灰色表格线、一支靛蓝笔圈出
    的重点。
  - **收敛**：#F9FAFB 底 + #FFFFFF 卡 + #E5E7EB 线；主色靛蓝 #4F46E5
    （档案原值）只标三样东西：当前导航项、本年序列、主按钮。去年序列用同
    色相的浅靛 #C7D2FE，两年对比读得出「同一件事的新旧」而不是两件事。
  - **论证**：看板一屏要放十几个数字，颜色越多越像仪表盘报警。所以状态
    徽标（完成 / 配送 / 待付 / 退款）用四支低饱和底 + 深色字，饱和色只留
    给主色一支。

### 负约束

  - 不放任何真实公司、平台、支付渠道的名称与 logo；渠道写「自营商城 /
    小程序 / 线下门店 / 分销伙伴」。
  - 不画假地图（「区域分布」这类图最容易画成一张没有数据的中国轮廓）。
  - 柱高、占比条宽度都由数据换算（见 REVENUE / CHANNELS）。
  - 头像一律首字。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from appkit import DESK_H, DESK_W, NUM, Kit
from oplib import Ids, color_vars, frame, rect, solid, stroke, write_doc

ids = Ids()
k = Kit(ids)

VARS = color_vars({
    "background":           "#F9FAFB",
    "foreground":           "#111827",
    "card":                 "#FFFFFF",
    "card-foreground":      "#111827",
    "primary":              "#4F46E5",
    "primary-foreground":   "#FFFFFF",
    "primary-soft":         "#C7D2FE",
    "secondary":            "#EEF2FF",
    "secondary-foreground": "#3730A3",
    "muted":                "#F3F4F6",
    "muted-foreground":     "#6B7280",
    "accent":               "#EEF2FF",
    "accent-foreground":    "#3730A3",
    "border":               "#E5E7EB",
    "input":                "#E5E7EB",
    "ring":                 "#4F46E5",
    "sidebar":              "#FFFFFF",
    "sidebar-foreground":   "#374151",
    "sidebar-primary":      "#4F46E5",
    "sidebar-accent":       "#EEF2FF",
    "sidebar-accent-foreground": "#3730A3",
    "sidebar-border":       "#E5E7EB",
    "--color-success":      "#047857",
    "--color-success-soft": "#D1FAE5",
    "--color-warning":      "#B45309",
    "--color-warning-soft": "#FEF3C7",
    "--color-info":         "#1D4ED8",
    "--color-info-soft":    "#DBEAFE",
    "chart-1":              "#4F46E5",
    "chart-2":              "#0EA5E9",
    "chart-3":              "#F59E0B",
    "chart-4":              "#10B981",
})

SIDEBAR_W = 248


def card(name, children, *, gap=16, pad=(20, 24), width="fill_container",
         **props):
    return k.card(name, children, gap=gap, pad=pad, radius=14,
                  border="$border", width=width, shadow=True, **props)


def button(label, glyph, *, primary=False, name=None):
    node = k.row(name or f"按钮 {label}", [
        k.icon(glyph, 16, "$primary-foreground" if primary else "$foreground"),
        k.label(label, 13, 600,
                "$primary-foreground" if primary else "$foreground"),
    ], gap=6, width="fit_content", height=40, padding=[0, 14],
        fill=solid("$primary" if primary else "$card"), cornerRadius=10,
        role="button")
    if not primary:
        node["stroke"] = stroke("$border", 1)
    return node


# ---------------------------------------------------------------- 侧栏
NAV_MAIN = [("layout-dashboard", "概览", True), ("shopping-cart", "订单", False),
            ("users", "客户", False), ("package", "商品", False),
            ("chart-column", "报表", False)]
NAV_SETTINGS = [("user-cog", "团队成员"), ("plug", "应用集成"),
                ("settings", "系统设置")]


def nav_item(glyph, label, active=False, badge=None):
    color = "$sidebar-accent-foreground" if active else "$sidebar-foreground"
    kids = [k.icon(glyph, 18, color),
            k.label(label, 14, 600 if active else 500, color)]
    if badge:
        kids += [k.spacer(), k.pill(badge, fill="$muted",
                                    color="$muted-foreground", size=11,
                                    pad=(2, 8))]
    return k.row(f"导航 {label}", kids, gap=12, height=40, padding=[0, 12],
                 fill=solid("$sidebar-accent") if active else None,
                 cornerRadius=10)


def sidebar():
    logo = k.row("品牌", [
        k.tile("layers", size=32, icon_size=18, fill="$primary",
               color="$primary-foreground", radius=9),
        k.label("云帆数据", 16, 700),
    ], gap=10, padding=[0, 8])
    group = lambda title: k.label(title, 11, 600, "$muted-foreground",  # noqa: E731
                                  name=f"分组 {title}")
    main = k.col("主导航", [group("工作台")] + [
        nav_item(g, t, a, "12" if t == "订单" else None)
        for g, t, a in NAV_MAIN
    ], gap=4)
    settings = k.col("设置导航", [group("管理")] + [
        nav_item(g, t) for g, t in NAV_SETTINGS
    ], gap=4)
    for grp in (main, settings):
        grp["children"][0] = k.row("分组标题行", [grp["children"][0]],
                                   padding=[0, 12, 6, 12])
    plan = k.card("套餐卡", [
        k.row("套餐标题", [
            k.icon("sparkles", 16, "$primary"),
            k.label("专业版试用", 13, 600),
        ], gap=6),
        k.para("还剩 12 天，升级后解锁多店铺汇总与自动周报。", 12, 400,
               "$muted-foreground", lh=1.5),
        k.bar(0.6, SIDEBAR_W - 32 - 32, height=6, track="$border",
              name="试用进度"),
        k.row("升级按钮", [k.label("升级套餐", 13, 600,
                                   "$primary-foreground")],
              justify="center", height=34, fill=solid("$primary"),
              cornerRadius=8, role="button"),
    ], gap=10, pad=(16, 16), fill="$muted", border=None, radius=12)
    user = k.row("当前用户", [
        k.avatar("陈", size=36, fill="$secondary",
                 color="$secondary-foreground", font=14),
        k.col("用户文字", [
            k.label("陈思远", 13, 600),
            k.label("运营负责人", 12, 400, "$muted-foreground"),
        ], gap=2),
        k.icon("chevrons-up-down", 16, "$muted-foreground"),
    ], gap=10, padding=[12, 8, 0, 8])
    user["stroke"] = {"thickness": [1, 0, 0, 0], "fill": solid("$border")}
    filler = frame(ids, "侧栏留白", width="fill_container",
                   height="fill_container", layout="none", fill=[])
    node = k.col("侧栏", [logo, main, settings, filler, plan, user], gap=24,
                 width=SIDEBAR_W, height="fill_container",
                 fill=solid("$sidebar"), padding=[24, 16, 20, 16],
                 role="sidebar")
    node["stroke"] = {"thickness": [0, 1, 0, 0],
                      "fill": solid("$sidebar-border")}
    return node


# ---------------------------------------------------------------- 顶栏
def header():
    search = k.row("搜索框", [
        k.icon("search", 16, "$muted-foreground"),
        k.label("搜索订单、客户或商品…", 13, 400, "$muted-foreground"),
        k.spacer(),
        k.pill("⌘ K", fill="$muted", color="$muted-foreground", size=11,
               pad=(2, 6)),
    ], gap=8, width=280, height=40, padding=[0, 12], fill=solid("$card"),
        cornerRadius=10)
    search["stroke"] = stroke("$input", 1)
    return k.row("顶栏", [
        k.col("页面标题", [
            k.label("经营概览", 24, 700, lh=1.25),
            k.label("9 月 1 日 – 9 月 25 日 · 对比上月同期", 13, 400,
                    "$muted-foreground"),
        ], gap=4),
        search,
        button("本月", "calendar"),
        button("导出报表", "download", primary=True),
    ], gap=12)


# ---------------------------------------------------------------- 指标卡
KPIS = [
    ("wallet", "总收入", "¥1,284,300", "+12.4%", True, "较上月 ¥1,142,600"),
    ("shopping-cart", "订单数", "8,642", "+6.1%", True, "日均 346 单"),
    ("user-plus", "新增客户", "1,238", "+18.2%", True, "复购率 34.5%"),
    ("rotate-ccw", "退款率", "1.8%", "−0.4pp", True, "低于行业均值 2.6%"),
]


def kpi(glyph, title, value, delta, good, note):
    tone = "--color-success" if good else "destructive"
    return card(f"指标 {title}", [
        k.row(f"{title} 眉行", [
            k.label(title, 13, 500, "$muted-foreground"),
            k.spacer(),
            k.tile(glyph, size=32, icon_size=16, fill="$accent",
                   color="$accent-foreground", radius=8),
        ]),
        k.label(value, 28, 700, family=NUM, lh=1.15, spacing=-0.5),
        k.row(f"{title} 变化", [
            k.pill(delta, fill=f"${tone}-soft", color=f"${tone}", size=11,
                   pad=(2, 8), glyph="trending-up" if delta[0] == "+"
                   else "trending-down"),
            k.label(note, 12, 400, "$muted-foreground"),
        ], gap=8),
    ], gap=10, pad=(16, 20))


# ---------------------------------------------------------------- 收入趋势
# 1–9 月收入（万元），本年 / 去年同月。柱高按 PLOT_MAX 换算。
REVENUE = [(82, 71), (91, 76), (88, 80), (97, 83), (104, 88), (112, 95),
           (108, 97), (121, 104), (128, 109)]
PLOT_H = 168
PLOT_MAX = 140
TICKS = ["140", "105", "70", "35", "0"]


def revenue_chart():
    # 柱对、基线、月份标签分三行排：柱对与标签都是定宽 GROUP_W 的格子，
    # 两行用同一个 space_between，标签就永远对准柱对的中心；基线夹在中
    # 间，所以柱子是从轴线上长出来的，而不是悬在标签上方。
    group_w = 32
    pairs, names = [], []
    for index, (this_year, last_year) in enumerate(REVENUE):
        pairs.append(k.row(f"{index + 1} 月柱对", [
            rect(ids, f"{index + 1} 月去年", width=14,
                 height=round(last_year / PLOT_MAX * PLOT_H), cornerRadius=4,
                 fill=solid("$primary-soft")),
            rect(ids, f"{index + 1} 月本年", width=14,
                 height=round(this_year / PLOT_MAX * PLOT_H), cornerRadius=4,
                 fill=solid("$primary")),
        ], gap=4, width=group_w, height=PLOT_H, align="end"))
        names.append(k.para(f"{index + 1} 月", 11, 400, "$muted-foreground",
                            width=group_w, align="center", lh=1.2,
                            name=f"月份 {index + 1}"))
    plot = k.row("柱区", pairs, align="end", justify="space_between",
                 padding=[0, 12])
    months = k.row("月份行", names, justify="space_between", padding=[0, 12])
    body = k.col("绘图区", [plot, k.divider(name="基线"), months], gap=0)
    months["padding"] = [8, 12, 0, 12]
    axis = k.col("纵轴", [
        k.label(tick, 11, 400, "$muted-foreground", family=NUM,
                name=f"刻度 {tick}") for tick in TICKS
    ], width=28, height=PLOT_H + 8, justify="space_between", align="end")
    legend = k.row("图例", [
        rect(ids, "图例 本年", width=10, height=10, cornerRadius=3,
             fill=solid("$primary")),
        k.label("2026 年", 12, 500, "$muted-foreground"),
        rect(ids, "图例 去年", width=10, height=10, cornerRadius=3,
             fill=solid("$primary-soft")),
        k.label("2025 年", 12, 500, "$muted-foreground"),
    ], gap=6, width="fit_content")
    return card("收入趋势卡", [
        k.row("收入趋势标题", [
            k.col("收入趋势文字", [
                k.label("收入趋势", 16, 600),
                k.label("单位：万元 · 1–9 月累计 ¥931 万，同比 +18.3%", 12,
                        400, "$muted-foreground"),
            ], gap=4),
            legend,
            k.segmented(["月", "季", "年"], 0, width=132),
        ], gap=20),
        k.row("图表", [axis, body], gap=12, align="start"),
    ], gap=20, height="fill_container")


# ---------------------------------------------------------------- 渠道占比
CHANNELS = [("chart-1", "自营商城", 0.42, "¥539,406"),
            ("chart-2", "小程序", 0.28, "¥359,604"),
            ("chart-3", "线下门店", 0.18, "¥231,174"),
            ("chart-4", "分销伙伴", 0.12, "¥154,116")]
CHANNEL_W = 380
STACK_W = CHANNEL_W - 48


def channel_card():
    segments = []
    used = 0
    for index, (tone, name, share, _) in enumerate(CHANNELS):
        width = (STACK_W - 3 * 3 - used if index == len(CHANNELS) - 1
                 else round((STACK_W - 9) * share))
        used += width
        segments.append(rect(ids, f"占比段 {name}", width=width, height=12,
                             cornerRadius=3, fill=solid(f"${tone}")))
    stacked = k.row("堆叠占比条", segments, gap=3, width=STACK_W)
    rows = []
    for tone, name, share, amount in CHANNELS:
        rows.append(k.row(f"渠道 {name}", [
            rect(ids, f"色点 {name}", width=10, height=10, cornerRadius=5,
                 fill=solid(f"${tone}")),
            # 名称 / 金额 / 占比三列写定宽：四行的数字右对齐成一条竖线。
            k.para(name, 14, 500, width=96, lh=1.3, name=f"渠道名 {name}"),
            k.spacer(),
            k.para(amount, 13, 600, family=NUM, width=90, align="right",
                   lh=1.3, name=f"渠道额 {name}"),
            k.para(f"{round(share * 100)}%", 13, 400, "$muted-foreground",
                   family=NUM, width=40, align="right", lh=1.3,
                   name=f"渠道比 {name}"),
        ], gap=10, height=32))
    return card("渠道占比卡", [
        k.row("渠道标题", [
            k.label("渠道占比", 16, 600),
            k.spacer(),
            k.label("按收入", 12, 400, "$muted-foreground"),
        ]),
        k.row("渠道总额", [
            k.label("¥1,284,300", 22, 700, family=NUM, lh=1.2),
            k.label("本月合计", 12, 400, "$muted-foreground"),
        ], gap=8, align="end"),
        stacked,
        k.col("渠道明细", rows, gap=4),
    ], gap=16, width=CHANNEL_W, height="fill_container")


# ---------------------------------------------------------------- 订单表
COLUMNS = [("订单号", 150), ("客户", 180), ("商品", None), ("金额", 120),
           ("状态", 120), ("下单时间", 150)]
ORDERS = [
    ("#20925-0418", "王宁", "无线降噪耳机 · 月光白", "¥1,299.00",
     ("已完成", "--color-success"), "今天 14:32"),
    ("#20925-0417", "刘可欣", "智能手表 S2 · 表带套装", "¥2,168.00",
     ("配送中", "--color-info"), "今天 13:05"),
    ("#20925-0415", "赵一鸣", "机械键盘 · 茶轴 87 键", "¥459.00",
     ("待付款", "--color-warning"), "今天 11:48"),
    ("#20924-0392", "孙悦", "便携咖啡机 · 旅行款", "¥699.00",
     ("已退款", "muted"), "昨天 21:16"),
]


def cell(content, width, *, name):
    node = k.row(name, [content], width=width or "fill_container")
    return node


def order_table():
    head = k.row("表头", [
        cell(k.label(title, 12, 600, "$muted-foreground"), width,
             name=f"表头 {title}")
        for title, width in COLUMNS
    ], height=40, gap=16, padding=[0, 24], fill=solid("$muted"))
    rows = [head]
    for number, customer, item, amount, (status, tone), when in ORDERS:
        soft = "$muted" if tone == "muted" else f"${tone}-soft"
        ink = "$muted-foreground" if tone == "muted" else f"${tone}"
        who = k.row(f"客户 {customer}", [
            k.avatar(customer[0], size=28, fill="$secondary",
                     color="$secondary-foreground", font=12),
            k.label(customer, 14, 500),
        ], gap=10, width="fit_content")
        rows.append(k.row(f"订单 {number}", [
            cell(k.label(number, 13, 500, family=NUM), 150, name="订单号格"),
            cell(who, 180, name="客户格"),
            cell(k.label(item, 14, 400), None, name="商品格"),
            cell(k.label(amount, 14, 600, family=NUM), 120, name="金额格"),
            cell(k.pill(status, fill=soft, color=ink, size=12, pad=(3, 10)),
                 120, name="状态格"),
            cell(k.label(when, 13, 400, "$muted-foreground"), 150,
                 name="时间格"),
        ], height=52, gap=16, padding=[0, 24]))
    table = k.col("订单表", [], gap=0)
    for index, row in enumerate(rows):
        if index > 1:
            table["children"].append(k.divider())
        table["children"].append(row)
    title = k.row("订单表标题", [
        k.label("最近订单", 16, 600),
        k.pill("今日 38 单", fill="$accent", color="$accent-foreground",
               size=11, pad=(2, 8)),
        k.spacer(),
        k.row("查看全部", [
            k.label("查看全部订单", 13, 600, "$primary"),
            k.icon("arrow-right", 14, "$primary"),
        ], gap=4, width="fit_content"),
    ], gap=10, padding=[18, 24])
    node = k.col("最近订单卡", [title, table], fill=solid("$card"),
                 cornerRadius=14, clipContent=True)
    node["stroke"] = stroke("$border", 1)
    node["effects"] = [{"type": "shadow", "offsetX": 0, "offsetY": 4,
                        "blur": 16, "spread": 0,
                        "color": "rgba(15,23,42,0.06)"}]
    return node


def build():
    kpis = k.row("指标卡行", [kpi(*item) for item in KPIS], gap=20,
                 align="start")
    middle = k.row("图表行", [revenue_chart(), channel_card()], gap=20,
                   height=300, align="start")
    main = k.col("主区", [header(), kpis, middle, order_table()], gap=16,
                 height="fill_container", padding=[28, 32, 28, 32],
                 clipContent=True)
    root = frame(ids, "经营概览", width=DESK_W, height=DESK_H,
                 layout="horizontal", fill=solid("$background"),
                 clipContent=True, x=0, y=0)
    root["screen"] = "/"
    root["children"] = [sidebar(), main]
    return [root]


if __name__ == "__main__":
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                       "..", "crates", "op-editor-core", "assets",
                       "scene_templates", "sales-dashboard-web.op")
    write_doc(os.path.normpath(out), VARS, build(), "经营概览看板",
              compact=True)

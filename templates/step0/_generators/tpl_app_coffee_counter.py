#!/usr/bin/env python3
"""coffee-counter-desktop.op — 咖啡门店订单台（1440×900 桌面单屏）

Studio 首页「应用界面 · 桌面」的示例就是这一份：「做一个咖啡门店桌面
App，包含订单列表、订单详情和营业概览。信息清楚，操作直接，蓝色强调待处理
订单。」所以一屏里三块一一对应：顶部营业概览（四个数）→ 左侧订单列表
（按状态分栏）→ 右侧订单详情（明细、备注、操作键）。

风格落在 `corporate-blue-light` 档：冷灰底、白卡、深石板字，主色深蓝
#1D4ED8 只标「待处理」—— 概览里的待处理数、列表里的待处理行、详情里的
「开始制作」。店员抬头一眼，蓝色的地方就是现在要做的事。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：吧台上方的出单屏 —— 灰白底、黑字小票、一块亮蓝的「新订单」
    提示条。
  - **收敛**：#F8FAFC 底 + #FFFFFF 卡 + #0F172A 字；左侧导航栏用深石板
    #0F172A 实底，与内容区拉开层级；制作中用琥珀、已完成用绿，都只做小徽
    标，不铺大色块。
  - **论证**：门店高峰时店员看屏幕的时间以秒计，颜色必须只回答一个问题：
    「哪一单在等我」。

### 负约束

  - 店名用首页示例标题里的虚构品牌「晨光咖啡」（英文示例叫 Daybreak，与
    网站示例同一个品牌），顶栏标题就是示例标题「晨光咖啡 · 门店工作台」。
  - 不出现任何真实咖啡品牌、收银系统、外卖平台的名称与 logo。
  - 顾客信息只到姓氏与手机尾号；金额、单号、时间均为示意数据。
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from appkit import DESK_H, DESK_W, NUM, Kit
from oplib import Ids, color_vars, frame, rect, solid, stroke, write_doc

ids = Ids()
k = Kit(ids)

VARS = color_vars({
    "background":           "#F8FAFC",
    "foreground":           "#0F172A",
    "card":                 "#FFFFFF",
    "card-foreground":      "#0F172A",
    "primary":              "#1D4ED8",
    "primary-foreground":   "#FFFFFF",
    "secondary":            "#EFF6FF",
    "secondary-foreground": "#1E40AF",
    "muted":                "#F1F5F9",
    "muted-foreground":     "#475569",
    "accent":               "#EFF6FF",
    "accent-foreground":    "#1E40AF",
    "border":               "#E2E8F0",
    "ring":                 "#1D4ED8",
    "sidebar":              "#0F172A",
    "sidebar-foreground":   "#94A3B8",
    "sidebar-primary":      "#FFFFFF",
    "sidebar-accent":       "#1E293B",
    "--color-success":      "#15803D",
    "--color-success-soft": "#DCFCE7",
    "--color-warning":      "#B45309",
    "--color-warning-soft": "#FEF3C7",
})

RAIL_W = 88
LIST_W = 540


def card(name, children, *, gap=16, pad=(20, 24), **props):
    return k.card(name, children, gap=gap, pad=pad, radius=14,
                  border="$border", shadow=True, **props)


def button(label, glyph=None, *, kind="outline"):
    colors = {"primary": ("$primary", "$primary-foreground"),
              "outline": ("$card", "$foreground"),
              "ghost": ("$muted", "$muted-foreground")}[kind]
    kids = ([k.icon(glyph, 16, colors[1])] if glyph else []) + [
        k.label(label, 14, 600, colors[1])]
    node = k.row(f"按钮 {label}", kids, gap=6, width="fit_content",
                 height=40, padding=[0, 16], fill=solid(colors[0]),
                 cornerRadius=10, role="button")
    if kind == "outline":
        node["stroke"] = stroke("$border", 1)
    return node


# ---------------------------------------------------------------- 导航栏
RAIL = [("receipt", "订单", True), ("coffee", "菜单", False),
        ("users", "会员", False), ("chart-column", "报表", False)]


def rail():
    items = []
    for glyph, label, on in RAIL:
        color = "$sidebar-primary" if on else "$sidebar-foreground"
        items.append(k.col(f"导航 {label}", [
            k.icon(glyph, 22, color),
            k.label(label, 11, 600 if on else 500, color),
        ], gap=6, align="center", width=64, padding=[10, 0],
            fill=solid("$sidebar-accent") if on else None, cornerRadius=12))
    logo = k.tile("coffee", size=40, icon_size=22, fill="$primary",
                  color="$primary-foreground", radius=12, name="门店标")
    settings = k.col("导航 设置", [
        k.icon("settings", 22, "$sidebar-foreground"),
        k.label("设置", 11, 500, "$sidebar-foreground"),
    ], gap=6, align="center", width=64, padding=[10, 0])
    filler = frame(ids, "导航留白", width="fill_container",
                   height="fill_container", layout="none", fill=[])
    return k.col("导航栏", [logo, k.col("导航项", items, gap=8,
                                        align="center"), filler, settings],
                 gap=28, width=RAIL_W, height="fill_container",
                 align="center", fill=solid("$sidebar"),
                 padding=[24, 12, 24, 12], role="sidebar")


# ---------------------------------------------------------------- 顶栏
def header():
    live = k.row("营业状态", [
        {"type": "ellipse", "id": ids("e"), "name": "营业中圆点",
         "width": 8, "height": 8, "fill": solid("$--color-success")},
        k.label("静安店 · 营业中 · 07:30 – 21:00", 13, 500, "$muted-foreground"),
        k.label("周五 9 月 25 日", 13, 400, "$muted-foreground"),
    ], gap=8, width="fit_content")
    search = k.row("搜索框", [
        k.icon("search", 16, "$muted-foreground"),
        k.label("搜索单号或手机尾号", 13, 400, "$muted-foreground"),
    ], gap=8, width=260, height=40, padding=[0, 12], fill=solid("$card"),
        cornerRadius=10, stroke=stroke("$border", 1))
    return k.row("顶栏", [
        k.col("门店标题", [k.label("晨光咖啡 · 门店工作台", 24, 700, lh=1.25), live],
              gap=6),
        search,
        button("打印日结", "printer"),
        button("新建订单", "plus", kind="primary"),
    ], gap=12)


# ---------------------------------------------------------------- 营业概览
def kpi(title, value, note, *, glyph, hot=False, note_tone=None):
    fg = "$primary-foreground" if hot else "$foreground"
    sub = "$secondary" if hot else "$muted-foreground"
    node = k.card(f"概览 {title}", [
        k.row(f"{title} 眉行", [
            k.label(title, 13, 500, sub),
            k.spacer(),
            k.icon(glyph, 18, sub),
        ]),
        k.label(value, 28, 700, fg, family=NUM, lh=1.15, spacing=-0.5),
        k.label(note, 12, 500, note_tone or sub),
    ], gap=8, pad=(16, 20), radius=14,
        fill="$primary" if hot else "$card",
        border=None if hot else "$border", shadow=not hot)
    return node


def overview():
    return k.row("营业概览", [
        kpi("今日营业额", "¥8,426", "较上周五 +9.2%", glyph="wallet",
            note_tone="$--color-success"),
        kpi("订单数", "214", "堂食 38 · 自取 131 · 外送 45",
            glyph="receipt"),
        kpi("待处理", "6 单", "最早一单已等 3 分钟", glyph="bell-ring",
            hot=True),
        kpi("平均出杯", "3:42", "高峰 8:30 – 9:30 · 4:58", glyph="timer"),
    ], gap=16, align="start")


# ---------------------------------------------------------------- 订单列表
ORDERS = [
    ("A128", "焦糖海盐拿铁 ×1 · 生椰拿铁 ×1", "自取 · 8:52", "待处理", True),
    ("A129", "冰美式 ×2 · 可颂 ×1", "自取 · 8:53", "待处理", False),
    ("B047", "桂花燕麦拿铁 ×3", "外送 · 8:53", "待处理", False),
    ("A127", "玫瑰荔枝冷萃 ×1", "自取 · 8:49", "制作中", False),
    ("C015", "青提气泡美式 ×2", "堂食 · 8:47", "制作中", False),
    ("A126", "生椰拿铁 ×1 · 贝果 ×1", "自取 · 8:41", "已完成", False),
]
STATUS = {"待处理": ("$secondary", "$secondary-foreground"),
          "制作中": ("$--color-warning-soft", "$--color-warning"),
          "已完成": ("$--color-success-soft", "$--color-success")}


def order_row(code, items, meta, status, selected):
    soft, ink = STATUS[status]
    bar = rect(ids, f"{code} 选中条", width=3, height=40, cornerRadius=1.5,
               fill=solid("$primary" if selected else "$card"))
    node = k.row(f"订单行 {code}", [
        bar,
        k.para(code, 16, 700, "$primary" if status == "待处理"
               else "$foreground", family=NUM, width=56, lh=1.3,
               name=f"单号 {code}"),
        k.col(f"{code} 文字", [
            k.label(items, 14, 500),
            k.label(meta, 12, 400, "$muted-foreground"),
        ], gap=4),
        k.pill(status, fill=soft, color=ink, size=12, pad=(3, 10)),
    ], gap=14, height=68, padding=[0, 20, 0, 0],
        fill=solid("$accent") if selected else None)
    if not selected:
        bar["fill"] = solid("$card")
    return node


def order_list():
    tabs = k.row("状态分栏", [
        tab_chip("待处理", "6", True),
        tab_chip("制作中", "4", False),
        tab_chip("已完成", "204", False),
    ], gap=6, padding=[0, 20])
    rows = []
    for index, order in enumerate(ORDERS):
        if index:
            rows.append(k.divider())
        rows.append(order_row(*order))
    node = k.col("订单列表", [
        k.row("列表标题", [k.label("订单列表", 16, 700), k.spacer(),
                           k.label("按下单时间", 12, 500,
                                   "$muted-foreground"),
                           k.icon("arrow-down-up", 14, "$muted-foreground")],
              gap=6, padding=[18, 20, 0, 20]),
        tabs,
        k.col("订单行", rows, clipContent=True, height="fill_container"),
    ], gap=14, width=LIST_W, height="fill_container", fill=solid("$card"),
        cornerRadius=14, clipContent=True)
    node["stroke"] = stroke("$border", 1)
    return node


def tab_chip(label, count, on):
    return k.row(f"分栏 {label}", [
        k.label(label, 13, 600 if on else 500,
                "$primary-foreground" if on else "$muted-foreground"),
        k.label(count, 12, 600, "$secondary" if on else "$muted-foreground",
                family=NUM),
    ], gap=6, width="fit_content", height=32, padding=[0, 14],
        fill=solid("$primary" if on else "$muted"), cornerRadius=16)


# ---------------------------------------------------------------- 订单详情
LINES = [
    ("焦糖海盐拿铁", "大杯 · 燕麦奶 · 少冰 · 加一份浓缩", "×1", "¥34"),
    ("生椰拿铁", "中杯 · 标准糖 · 热", "×1", "¥26"),
]


def detail():
    head = k.row("详情标题", [
        k.label("A128", 28, 700, family=NUM, lh=1.2),
        k.pill("待处理", fill="$secondary", color="$secondary-foreground",
               size=12, pad=(3, 10)),
        k.spacer(),
        k.label("8:52 下单 · 到店自取 · 已等 3 分钟", 13, 500,
                "$muted-foreground"),
    ], gap=12)
    guest = k.row("顾客", [
        k.avatar("林", size=40, fill="$secondary",
                 color="$secondary-foreground"),
        k.col("顾客文字", [
            k.label("林女士 · 尾号 6021", 15, 600),
            k.label("会员 · 本月第 12 单 · 常点燕麦奶", 12, 400,
                    "$muted-foreground"),
        ], gap=3),
        button("联系顾客", "phone"),
    ], gap=12, padding=[14, 16], fill=solid("$muted"), cornerRadius=12)
    lines = []
    for name, spec, qty, price in LINES:
        lines.append(k.row(f"明细 {name}", [
            k.tile("coffee", size=44, icon_size=20, fill="$secondary",
                   color="$primary", radius=10),
            k.col(f"{name} 文字", [
                k.label(name, 15, 600),
                k.label(spec, 13, 400, "$muted-foreground"),
            ], gap=4),
            k.label(qty, 14, 500, "$muted-foreground", family=NUM),
            k.para(price, 15, 600, family=NUM, width=64, align="right",
                   lh=1.3, name=f"价格 {name}"),
        ], gap=14))
    note = k.row("备注", [
        k.icon("message-square-text", 16, "$--color-warning"),
        k.label("备注：燕麦奶那杯请单独打包，谢谢！", 14, 500,
                "$--color-warning"),
    ], gap=8, padding=[12, 14], fill=solid("$--color-warning-soft"),
        cornerRadius=10)

    def money(caption, value, strong=False):
        return k.row(f"金额 {caption}", [
            k.label(caption, 15 if strong else 13, 600 if strong else 400,
                    "$foreground" if strong else "$muted-foreground"),
            k.spacer(),
            k.label(value, 22 if strong else 14, 700 if strong else 500,
                    family=NUM),
        ])

    filler = frame(ids, "详情留白", width="fill_container",
                   height="fill_container", layout="none", fill=[])
    actions = k.row("详情操作", [
        button("拒单", kind="ghost"),
        k.spacer(),
        button("打印小票", "printer"),
        button("开始制作", "play", kind="primary"),
    ], gap=12)
    return card("订单详情", [
        head, guest,
        k.col("饮品明细", [k.label("饮品明细 · 2 杯", 13, 600,
                                   "$muted-foreground")] + lines, gap=14),
        note,
        k.divider(),
        k.col("金额", [money("商品小计", "¥60"), money("会员 9 折", "-¥6"),
                       money("实付", "¥54", strong=True)], gap=10),
        filler,
        actions,
    ], gap=18, pad=(22, 24), height="fill_container")


def build():
    panes = k.row("主体双栏", [order_list(), detail()], gap=20,
                  height="fill_container", align="start")
    main = k.col("主区", [header(), overview(), panes], gap=20,
                 height="fill_container", padding=[24, 28, 24, 28],
                 clipContent=True)
    root = frame(ids, "咖啡门店订单台", width=DESK_W, height=DESK_H,
                 layout="horizontal", fill=solid("$background"),
                 clipContent=True, x=0, y=0)
    root["screen"] = "/"
    root["children"] = [rail(), main]
    return [root]


if __name__ == "__main__":
    out = os.path.join(HERE, "..", "..", "..", "crates", "op-editor-core",
                       "assets", "scene_templates",
                       "coffee-counter-desktop.op")
    write_doc(os.path.normpath(out), VARS, build(), "咖啡门店订单台",
              compact=True)

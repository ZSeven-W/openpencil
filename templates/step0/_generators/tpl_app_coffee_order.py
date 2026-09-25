#!/usr/bin/env python3
"""coffee-order-app.op — 咖啡点单 App 三屏（375×812 × 3）

Studio 首页「应用界面 · 手机」的示例就是这一份：「做一个咖啡点单 App，包含
首页、菜单和订单页。风格简洁温暖，突出咖啡图片和点单流程。」空输入框点开始
设计时，它作为即时初稿载入，模型再在这三屏上改内容。所以三屏的**结构**必须
与示例一一对应：首页（主推新品大图 + 分类 + 人气推荐）→ 菜单（左分类栏 +
右商品列表 + 购物车条）→ 订单（制作进度 + 取餐码 + 明细）。

风格落在 `warm-food-mobile-light` 档：暖象牙底、白卡、深可可字，主色取档案
的暖橙压深一档（#D9480F），白字压在上面才读得清。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：清晨咖啡馆的吧台 —— 象牙色的杯垫、深可可色的木台、焦糖在
    奶泡上拉出的一道橙褐。
  - **收敛**：#FFFCF6 底 + #FFFFFF 卡 + #21140F 字；主色焦橙 #D9480F 只给
    「加购、结算、当前步骤」三类动作；饮品类别用四支低饱和底色的图标块区分。
  - **论证**：点单流程里颜色就是引导 —— 用户一眼扫过去，橙色只出现在
    「下一步该点的地方」。

### 图片

  主推卡用仓库里已有的演示图（`op-editor-ui/assets/home_examples/
  coffee-demo.jpg`，为首页示例生成的概念图，非 OpenPencil 产物），内联成
  data URL；节点同时带 `imageSearchQuery` / `imagePrompt`，搜图 / 生图可
  以一键换掉它。商品缩略用带类别色的图标块，不堆四张同一张图。

### 负约束

  - 不出现任何真实咖啡品牌、门店、外卖平台的名称与 logo；门店写「静安店」。
  - 价格、取餐码、时间均为示意数据。
"""

import base64
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from appkit import NUM, Kit
from oplib import Ids, color_vars, frame, rect, solid, stroke, write_doc

ids = Ids()
k = Kit(ids)

REPO = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
PHOTO = os.path.join(REPO, "crates", "op-editor-ui", "assets",
                     "home_examples", "coffee-demo.jpg")

VARS = color_vars({
    "background":           "#FFFCF6",
    "foreground":           "#21140F",
    "card":                 "#FFFFFF",
    "card-foreground":      "#21140F",
    "primary":              "#D9480F",
    "primary-foreground":   "#FFFFFF",
    "secondary":            "#FFF0E3",
    "secondary-foreground": "#9A3412",
    "muted":                "#F6EEE4",
    "muted-foreground":     "#7A5B48",
    "border":               "#EFE4D8",
    "ring":                 "#D9480F",
    "--color-success":      "#15803D",
    "--color-success-soft": "#DCFCE7",
    "chart-1":              "#9A5B2E",
    "chart-1-soft":         "#F3E3D3",
    "chart-2":              "#B45309",
    "chart-2-soft":         "#FDEBC8",
    "chart-3":              "#3F6212",
    "chart-3-soft":         "#E6F0D2",
    "chart-4":              "#9D174D",
    "chart-4-soft":         "#FBE0EA",
})

PAD = 20
TABS = [("house", "首页", "/"), ("coffee", "菜单", "/menu"),
        ("receipt", "订单", "/order"), ("user", "我的")]


def body(name, children, *, gap=18, pad=(8, PAD, 16, PAD)):
    return k.col(name, children, gap=gap, height="fill_container",
                 padding=list(pad), clipContent=True)


def section_head(title, action=None):
    kids = [k.label(title, 17, 700), k.spacer()]
    if action:
        kids.append(k.label(action, 13, 500, "$muted-foreground"))
    return k.row(f"区块标题 {title}", kids)


def add_button(size=28):
    return k.row("加购键", [k.icon("plus", round(size * 0.57),
                                   "$primary-foreground")],
                 width=size, height=size, justify="center",
                 fill=solid("$primary"), cornerRadius=size / 2,
                 role="button")


def drink_tile(tone, glyph="coffee", size=56):
    return k.tile(glyph, size=size, icon_size=round(size * 0.46),
                  fill=f"${tone}-soft", color=f"${tone}", radius=16,
                  name=f"饮品图标 {tone}")


# ---------------------------------------------------------------- 01 首页
def hero():
    with open(PHOTO, "rb") as fh:
        url = "data:image/jpeg;base64," + base64.b64encode(fh.read()).decode()
    photo = {
        "type": "image", "id": ids("m"), "name": "主推饮品图",
        "src": url, "width": 132, "height": "fill_container",
        "cornerRadius": 16, "fill": solid("$muted"),
        "imageSearchQuery": "iced caramel latte glass",
        "imagePrompt": ("iced caramel latte in a tall glass with latte art, "
                        "warm morning sunlight, beige stone table, coffee "
                        "beans, soft shadows, photographic, no text"),
    }
    words = k.col("主推文字", [
        k.pill("秋季新品", fill="$secondary", color="$secondary-foreground",
               size=11, pad=(3, 8)),
        k.label("焦糖海盐拿铁", 20, 700, lh=1.25),
        k.para("焦糖的甜遇上一点海盐，\n冰饮热饮都好喝。", 13, 400,
               "$muted-foreground", lh=1.5),
        frame(ids, "主推留白", width="fill_container",
              height="fill_container", layout="none", fill=[]),
        k.row("主推价格行", [
            k.label("¥28", 20, 700, "$primary", family=NUM),
            k.label("¥32", 12, 400, "$muted-foreground", family=NUM),
            k.spacer(),
            k.row("立即点单", [k.label("立即点单", 13, 600,
                                        "$primary-foreground")],
                  width="fit_content", height=32, padding=[0, 14],
                  fill=solid("$primary"), cornerRadius=16, role="button"),
        ], gap=6),
    ], gap=8, height="fill_container")
    return k.row("主推卡", [photo, words], gap=14, height=196,
                 fill=solid("$card"), padding=[12, 14, 12, 12],
                 cornerRadius=20, align="start",
                 stroke=stroke("$border", 1))


CATEGORIES = ["人气", "拿铁", "美式", "特调", "茶饮"]
POPULAR = [("chart-1", "coffee", "生椰拿铁", "椰浆 · 冰", "¥26"),
           ("chart-3", "cup-soda", "青提气泡美式", "清爽 · 少糖", "¥24")]


def category_chips(active=0):
    chips = []
    for index, name in enumerate(CATEGORIES):
        on = index == active
        chips.append(k.pill(name, fill="$foreground" if on else "$card",
                            color="$background" if on else "$foreground",
                            size=13, weight=600 if on else 500,
                            pad=(7, 16), border=None if on else "$border"))
    return k.row("分类标签", chips, gap=8)


def popular_card(tone, glyph, name, desc, price):
    card = k.card(f"人气 {name}", [
        k.row(f"{name} 图位", [drink_tile(tone, glyph, 64)],
              justify="center", height=92, fill=solid(f"${tone}-soft"),
              cornerRadius=14),
        k.col(f"{name} 文字", [
            k.label(name, 15, 600),
            k.label(desc, 12, 400, "$muted-foreground"),
        ], gap=3),
        k.row(f"{name} 价格行", [
            k.label(price, 16, 700, family=NUM),
            k.spacer(),
            add_button(),
        ]),
    ], gap=10, pad=(10, 10), radius=18)
    card["width"] = "fill_container"
    return card


def home():
    head = k.row("问候行", [
        k.col("问候文字", [
            k.row("门店", [
                k.icon("map-pin", 14, "$primary"),
                k.label("静安店 · 到店自取", 13, 500, "$muted-foreground"),
                k.icon("chevron-down", 14, "$muted-foreground"),
            ], gap=4, width="fit_content"),
            k.label("早上好，来杯咖啡？", 22, 700, lh=1.3),
        ], gap=6),
        k.tile("bell", size=40, icon_size=20, fill="$card", radius=20),
    ], gap=12)
    head["children"][1]["stroke"] = stroke("$border", 1)
    search = k.row("搜索框", [
        k.icon("search", 16, "$muted-foreground"),
        k.label("搜索饮品、甜点", 14, 400, "$muted-foreground"),
    ], gap=8, height=44, padding=[0, 14], fill=solid("$card"),
        cornerRadius=22, stroke=stroke("$border", 1))
    popular = k.col("人气推荐", [
        section_head("人气推荐", "全部菜单"),
        k.row("人气两卡", [popular_card(*item) for item in POPULAR],
              gap=12, align="start"),
    ], gap=12)
    content = body("首页内容", [head, search, hero(), category_chips(),
                                popular])
    return k.phone("01 首页", "/", content, tab=k.tab_bar(TABS, 0), index=0)


# ---------------------------------------------------------------- 02 菜单
RAIL = ["人气", "拿铁", "美式", "特调", "茶饮", "轻食"]
MENU = [
    ("chart-1", "coffee", "焦糖海盐拿铁", "焦糖 · 海盐 · 可选燕麦奶", "¥28", "新品"),
    ("chart-1", "coffee", "生椰拿铁", "椰浆替代牛奶，口感更轻", "¥26", None),
    ("chart-2", "coffee", "桂花燕麦拿铁", "秋季限定 · 微甜", "¥27", "限定"),
    ("chart-4", "cup-soda", "玫瑰荔枝冷萃", "冷萃 12 小时 · 花果香", "¥29", None),
    ("chart-3", "cup-soda", "青提气泡美式", "气泡水 · 清爽少糖", "¥24", None),
]


def rail():
    items = []
    for index, name in enumerate(RAIL):
        on = index == 0
        mark = rect(ids, f"{name} 选中条", width=3, height=18, cornerRadius=1.5,
                    fill=solid("$primary" if on else "$muted"))
        item = k.row(f"分类 {name}", [
            mark,
            k.label(name, 14, 700 if on else 500,
                    "$foreground" if on else "$muted-foreground"),
        ], gap=12, height=48, fill=solid("$background") if on else None)
        if not on:
            item["children"][0]["fill"] = solid("$muted")
        items.append(item)
    return k.col("分类栏", items, width=84, height="fill_container",
                 fill=solid("$muted"), padding=[8, 0])


def menu_item(tone, glyph, name, desc, price, badge):
    words = [k.row(f"{name} 名称行", [k.label(name, 15, 600)] + (
        [k.pill(badge, fill="$secondary", color="$secondary-foreground",
                size=10, pad=(2, 6))] if badge else []), gap=6)]
    words.append(k.label(desc, 12, 400, "$muted-foreground"))
    words.append(k.row(f"{name} 价格行", [
        k.label(price, 16, 700, family=NUM),
        k.spacer(),
        add_button(26),
    ]))
    return k.row(f"商品 {name}", [
        drink_tile(tone, glyph, 64),
        k.col(f"{name} 文字", words, gap=6),
    ], gap=12, align="start", padding=[12, 0])


def cart_bar():
    return k.row("购物车条", [
        k.row("购物车图标", [k.icon("shopping-bag", 20, "$background")],
              width=40, height=40, justify="center", cornerRadius=20,
              fill=solid("$primary")),
        k.col("购物车文字", [
            k.label("已选 2 杯", 12, 400, "$muted"),
            k.label("¥54", 17, 700, "$background", family=NUM),
        ], gap=1),
        k.row("去结算", [k.label("去结算", 14, 600, "$primary-foreground")],
              width="fit_content", height=40, padding=[0, 20],
              fill=solid("$primary"), cornerRadius=20, role="button"),
    ], gap=12, height=60, padding=[0, 10, 0, 10], fill=solid("$foreground"),
        cornerRadius=30)


def menu():
    head = k.row("菜单顶部", [
        k.label("菜单", 24, 700, lh=1.25),
        k.spacer(),
        k.pill("静安店 · 自取", glyph="map-pin", fill="$card",
               border="$border", size=12, pad=(5, 10)),
    ], padding=[8, PAD, 8, PAD])
    rows = []
    for index, item in enumerate(MENU):
        if index:
            rows.append(k.divider())
        rows.append(menu_item(*item))
    listing = k.col("商品列表", [
        k.label("人气推荐", 13, 600, "$muted-foreground"),
        k.col("商品", rows),
    ], gap=4, height="fill_container", padding=[8, 16, 0, 16],
        clipContent=True)
    split = k.row("菜单主体", [rail(), listing], height="fill_container",
                  align="start")
    content = k.col("菜单内容", [head, split,
                                 k.row("购物车行", [cart_bar()],
                                       padding=[10, 16, 12, 16])],
                    height="fill_container", clipContent=True)
    return k.phone("02 菜单", "/menu", content, tab=k.tab_bar(TABS, 1),
                   index=1)


# ---------------------------------------------------------------- 03 订单
STEPS = [("已下单", "8:52", "done"), ("制作中", "8:54", "now"),
         ("待取餐", "约 9:02", "todo")]
LINES = [("chart-1", "焦糖海盐拿铁", "大杯 · 燕麦奶 · 少冰", "×1", "¥30"),
         ("chart-1", "生椰拿铁", "中杯 · 标准糖", "×1", "¥26")]


def step(name, time, state):
    on = state != "todo"
    dot = k.row(f"{name} 圆", [
        k.icon("check" if state == "done" else "coffee", 14,
               "$primary-foreground" if on else "$muted-foreground"),
    ], width=28, height=28, justify="center", cornerRadius=14,
        fill=solid("$primary" if on else "$muted"))
    return k.col(f"步骤 {name}", [
        dot,
        k.label(name, 13, 700 if state == "now" else 500,
                "$foreground" if on else "$muted-foreground"),
        k.label(time, 11, 400, "$muted-foreground", family=NUM),
    ], gap=6, align="center", width=72)


def order():
    head = k.row("订单顶部", [
        k.label("订单", 24, 700, lh=1.25),
        k.spacer(),
        k.label("历史订单", 13, 500, "$muted-foreground"),
    ])
    track = []
    for index, item in enumerate(STEPS):
        if index:
            # 连线包一层上内边距 13：线落在 28px 圆点的中线上，而不是
            # 贴着步骤列的顶边。
            track.append(k.col(f"进度线 {index}", [
                rect(ids, f"进度线段 {index}", width="fill_container",
                     height=2, fill=solid("$primary" if index == 1
                                          else "$border")),
            ], padding=[13, 0, 0, 0]))
        track.append(step(*item))
    progress = k.row("制作进度", track, align="start")
    status = k.card("取餐卡", [
        k.row("取餐卡眉行", [
            k.pill("制作中", glyph="flame", fill="$secondary",
                   color="$secondary-foreground", size=12),
            k.spacer(),
            k.label("静安店 · 到店自取", 12, 400, "$muted-foreground"),
        ]),
        k.col("取餐码", [
            k.label("取餐码", 13, 400, "$muted-foreground"),
            k.label("A 128", 44, 700, family=NUM, lh=1.1, spacing=1),
            k.label("预计 8 分钟后可取，做好会通知你", 13, 500, "$primary"),
        ], gap=4, align="center"),
        progress,
    ], gap=18, pad=(18, 18), radius=20)
    rows = []
    for tone, name, spec, qty, price in LINES:
        rows.append(k.row(f"明细 {name}", [
            drink_tile(tone, "coffee", 44),
            k.col(f"{name} 文字", [
                k.label(name, 14, 600),
                k.label(spec, 12, 400, "$muted-foreground"),
            ], gap=3),
            k.label(qty, 13, 400, "$muted-foreground", family=NUM),
            k.label(price, 14, 600, family=NUM),
        ], gap=12))

    def money(caption, value, *, strong=False, tone="$foreground"):
        return k.row(f"金额 {caption}", [
            k.label(caption, 14 if strong else 13, 600 if strong else 400,
                    "$foreground" if strong else "$muted-foreground"),
            k.spacer(),
            k.label(value, 18 if strong else 13, 700 if strong else 500,
                    tone, family=NUM),
        ])

    detail = k.card("订单明细卡", rows + [
        k.divider(),
        money("商品小计", "¥56"),
        money("新人券", "-¥6", tone="$--color-success"),
        money("实付", "¥50", strong=True),
    ], gap=12, pad=(16, 16), radius=20)
    actions = k.row("订单操作", [
        k.row("联系门店", [k.icon("phone", 16, "$foreground"),
                           k.label("联系门店", 14, 600)],
              gap=6, justify="center", height=44, cornerRadius=22,
              fill=solid("$card"), stroke=stroke("$border", 1)),
        k.row("再来一单", [k.label("再来一单", 14, 600,
                                   "$primary-foreground")],
              justify="center", height=44, cornerRadius=22,
              fill=solid("$primary"), role="button"),
    ], gap=12)
    content = body("订单内容", [head, status, detail, actions], gap=16)
    return k.phone("03 订单", "/order", content, tab=k.tab_bar(TABS, 2),
                   index=2)


def build():
    return [home(), menu(), order()]


if __name__ == "__main__":
    out = os.path.join(REPO, "crates", "op-editor-core", "assets",
                       "scene_templates", "coffee-order-app.op")
    write_doc(out, VARS, build(), "咖啡点单 App 三屏", compact=True)

#!/usr/bin/env python3
"""daybreak-coffee-site.op — Daybreak Coffee brand site (1200 wide, one long page)

This is the instant draft behind the Studio Home "网站设计" example: 「为
Daybreak 咖啡品牌设计一个官网。包含主视觉、招牌咖啡、品牌故事和门店信息，暖白与
森林绿，使用完整的纵向滚动页面。」 The empty-box Send loads it and a model
refines it in place, so the sections are exactly the four the brief names,
in reading order:

  nav → 主视觉 (hero) → 招牌咖啡 (four drink cards) → 品牌故事 (forest band)
  → 门店信息 (three store cards) → footer

### Palette (sample → converge → argue)

  - Sample: a café window at seven in the morning — limewashed walls, a
    potted fig by the door, the dark green enamel of the shop sign.
  - Converge: warm white #FAF6EE page, white cards, warm charcoal ink; one
    forest green #1F4A36 carries every action and the story band; a sage
    tint #E4ECE3 backs media slots and quiet chips. Caramel #9A5B2E is used
    only for small eyebrow labels, never for body text.
  - Argue: green is the brand's own sign colour, so it marks "the brand
    speaking" (CTA, story) while the menu and store info stay on warm white.

### Images

  The hero is the repo's demo photo (`op-editor-ui/assets/home_examples/
  coffee-demo.jpg`, generated concept material, not an OpenPencil result)
  inlined as a data URL. Every other picture is an empty `image` node with
  `imageSearchQuery` / `imagePrompt`, i.e. a real slot the image-fill path
  populates. Slots sit on a sage surface so the empty state reads as an
  intentional placeholder rather than a hole.

### Negative constraints

  - No real coffee brands, chains, delivery platforms or logos; the only
    brand is the fictional Daybreak Coffee. Street names are fictional.
  - Prices, years, counts and hours are illustrative.
"""

import base64
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from appkit import NUM, Kit
from oplib import Ids, color_vars, frame, rect, solid, stroke, text, write_doc

ids = Ids()
k = Kit(ids)

REPO = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
PHOTO = os.path.join(REPO, "crates", "op-editor-ui", "assets",
                     "home_examples", "coffee-demo.jpg")

SERIF = "Songti SC"
CJK = "Noto Sans SC"

VARS = color_vars({
    "background":           "#FAF6EE",
    "foreground":           "#1F2A23",
    "card":                 "#FFFFFF",
    "card-foreground":      "#1F2A23",
    "primary":              "#1F4A36",
    "primary-foreground":   "#FAF6EE",
    "secondary":            "#E4ECE3",
    "secondary-foreground": "#1F4A36",
    "muted":                "#F1EBDF",
    "muted-foreground":     "#66705F",
    "accent":               "#9A5B2E",
    "accent-foreground":    "#FFFFFF",
    "border":               "#E6DDCD",
    "ring":                 "#1F4A36",
    # Text and rules on the forest band / footer.
    "band-muted":           "#BFD0C3",
    "band-rule":            "#3A6450",
})

W = 1200
PAGE_H = 3161
PAD_X = 64


def page_section(name, children, *, gap=40, pad=(96, PAD_X, 96, PAD_X),
                 fill=None):
    return k.col(name, children, gap=gap, padding=list(pad),
                 fill=solid(fill) if fill else None)


def eyebrow(content, color="$accent"):
    return k.label(content, 13, 700, color, family=NUM, spacing=2,
                   name=f"眉题 {content}")


def heading(content, size=40, color="$foreground", name=None):
    return text(ids, name or f"标题 {content[:8]}", content, size, 700, color,
                family=SERIF, line_height=1.3, width="fill_container",
                growth="fixed-width")


def section_head(tag, title, sub, action=None):
    left = k.col(f"区块标题 {title}", [
        eyebrow(tag),
        heading(title),
        k.para(sub, 17, 400, "$muted-foreground", lh=1.7),
    ], gap=12, width=620)
    kids = [left, k.spacer()]
    if action:
        kids.append(k.row(f"区块链接 {action}", [
            k.label(action, 15, 600, "$primary"),
            k.icon("arrow-right", 18, "$primary"),
        ], gap=6, width="fit_content"))
    return k.row(f"区块头 {title}", kids, align="end")


def button(content, *, primary=True, glyph=None, height=52):
    color = "$primary-foreground" if primary else "$primary"
    kids = [k.label(content, 16, 600, color)]
    if glyph:
        kids.append(k.icon(glyph, 18, color))
    node = k.row(f"按钮 {content}", kids, gap=8, width="fit_content",
                 height=height, padding=[0, 28], cornerRadius=height / 2,
                 fill=solid("$primary") if primary else None,
                 justify="center", role="button")
    if not primary:
        node["stroke"] = stroke("$primary", 1.5)
    return node


def image_slot(name, width, height, query, prompt, radius=20):
    """An empty image slot the image-fill path can populate."""
    return {
        "type": "image", "id": ids("m"), "name": name, "src": "",
        "width": width, "height": height, "cornerRadius": radius,
        "imageSearchQuery": query, "imagePrompt": prompt,
    }


# ------------------------------------------------------------------- nav
def nav():
    mark = k.row("品牌标识", [
        k.tile("sunrise", size=40, icon_size=22, fill="$primary",
               color="$primary-foreground", radius=20),
        k.label("Daybreak Coffee", 21, 700, "$primary", family=NUM,
                spacing=-0.2),
    ], gap=12, width="fit_content")
    links = k.row("导航链接", [
        k.label(item, 15, 500, "$foreground")
        for item in ("招牌咖啡", "品牌故事", "门店信息", "咖啡豆订阅")
    ], gap=40, width="fit_content")
    return k.row("导航栏", [
        mark, k.spacer(), links,
        frame(ids, "导航间距", width=40, height=1, layout="none", fill=[]),
        button("找一家门店", glyph="map-pin", height=44),
    ], height=88, padding=[0, PAD_X], role="navigation")


# ------------------------------------------------------------------- hero
def hero():
    with open(PHOTO, "rb") as fh:
        url = "data:image/jpeg;base64," + base64.b64encode(fh.read()).decode()
    photo = {
        "type": "image", "id": ids("m"), "name": "主视觉 · 招牌拿铁",
        "src": url, "width": 452, "height": 560, "cornerRadius": 28,
        "imageSearchQuery": "iced latte glass morning sunlight cafe table",
        "imagePrompt": ("a tall glass of iced caramel latte with latte art on "
                        "a warm limestone café table, soft morning sunlight, "
                        "coffee beans, calm and natural, photographic, no "
                        "text"),
    }
    stats = k.row("主视觉数据", [
        stat("12", "年", "自家烘焙"),
        stat("6", "家", "城市门店"),
        stat("07:00", "", "每天开门"),
    ], gap=0, align="start")
    for cell in stats["children"]:
        cell["width"] = "fill_container"
    words = k.col("主视觉文字", [
        k.pill("每天清晨，现烘现磨", glyph="leaf", fill="$secondary",
               color="$secondary-foreground", size=14, pad=(7, 14)),
        heading("从一杯好咖啡，\n开始每一个清晨", 60, name="主标题"),
        k.para("Daybreak 是一家街角咖啡馆。我们每周烘一次豆子，\n"
               "只做自己每天早上也想喝的那几杯，\n"
               "让忙碌的一天，有一个安静的开头。",
               19, 400, "$muted-foreground", lh=1.75),
        k.row("主视觉按钮", [
            button("看看招牌咖啡", glyph="arrow-right"),
            button("我们的故事", primary=False),
        ], gap=16),
        k.divider(),
        stats,
    ], gap=28)
    return k.row("主视觉", [words, photo], gap=64,
                 padding=[40, PAD_X, 96, PAD_X], role="hero")


def stat(value, unit, label):
    number = k.row(f"数字 {label}", [
        k.label(value, 36, 700, "$primary", family=NUM, lh=1.1, spacing=-0.5),
    ], gap=4, width="fit_content", align="end")
    if unit:
        number["children"].append(k.label(unit, 16, 600, "$primary", lh=1.6))
    return k.col(f"数据 {label}", [
        number, k.label(label, 14, 400, "$muted-foreground"),
    ], gap=6)


# --------------------------------------------------------- signature menu
DRINKS = [
    ("晨光拿铁", "招牌", "双份浓缩配鲜奶，\n奶泡细密，带焦糖香。", "¥32",
     "latte art cup on wooden table",
     "a ceramic cup of hot latte with rosetta latte art on a light oak "
     "table, morning window light, minimal, photographic, no text"),
    ("森林冷萃", "限定", "低温慢萃 16 小时，\n入口清爽，回甘有可可。", "¥30",
     "cold brew coffee glass ice",
     "a clear glass of cold brew coffee with large ice cubes and a sprig "
     "of mint, green leafy background softly blurred, photographic, no text"),
    ("桂花燕麦拿铁", "季节", "燕麦奶打底，\n桂花糖浆只放一点点。", "¥34",
     "oat milk latte osmanthus",
     "an oat milk latte in a matte cup sprinkled with dried osmanthus "
     "flowers, warm autumn light, cozy café, photographic, no text"),
    ("今日手冲", "每周换豆", "每周换一支单一产区豆，\n店员会讲给你听。", "¥38",
     "pour over coffee kettle dripper",
     "pour-over coffee being brewed with a gooseneck kettle into a glass "
     "server, clean bar counter, soft daylight, photographic, no text"),
]


def drink_card(name, tag, desc, price, query, prompt):
    media = k.col(f"{name} 图位", [
        image_slot(f"{name} 饮品图", "fill_container", 200, query, prompt,
                   radius=16),
    ], fill=solid("$secondary"), cornerRadius=16)
    card = k.card(f"招牌 {name}", [
        media,
        k.col(f"{name} 文字", [
            k.row(f"{name} 标题行", [
                k.label(name, 20, 700, lh=1.3),
                k.spacer(),
                k.pill(tag, fill="$muted", color="$accent", size=12,
                       weight=600, pad=(4, 10)),
            ], gap=8),
            k.para(desc, 15, 400, "$muted-foreground", lh=1.65),
        ], gap=8),
        k.row(f"{name} 价格行", [
            k.label(price, 22, 700, "$primary", family=NUM),
            k.spacer(),
            k.row(f"{name} 规格", [
                k.label("热 / 冰", 13, 500, "$muted-foreground"),
            ], width="fit_content"),
        ]),
    ], gap=18, pad=(12, 12), radius=22, shadow=True)
    card["padding"] = [12, 12, 20, 12]
    for child in card["children"][1:]:
        child["padding"] = [0, 8]
    return card


def signature():
    cards = k.row("招牌咖啡卡片", [drink_card(*d) for d in DRINKS], gap=24,
                  align="start")
    for card in cards["children"]:
        card["width"] = "fill_container"
    return page_section("招牌咖啡", [
        section_head("SIGNATURE", "招牌咖啡",
                     "菜单不长，但每一杯都在吧台上被做过上百次。先从这四杯开始。",
                     "查看完整菜单"),
        cards,
    ], pad=(24, PAD_X, 112, PAD_X))


# ------------------------------------------------------------------ story
def story_fact(value, label):
    return k.col(f"故事数据 {label}", [
        k.label(value, 32, 700, "$primary-foreground", family=NUM, lh=1.1),
        k.label(label, 14, 400, "$band-muted"),
    ], gap=8)


def story():
    photo = k.col("品牌故事图位", [
        image_slot("品牌故事 · 门店与烘焙", "fill_container", "fill_container",
                   "small coffee shop roastery morning",
                   "the interior of a small neighborhood coffee roastery at "
                   "dawn, a barista weighing green beans beside a compact "
                   "drum roaster, plants, warm light, photographic, no text",
                   radius=24),
    ], width=460, height=520, fill=solid("$band-rule"),
        cornerRadius=24)
    facts = k.row("故事数据行", [
        story_fact("2014", "第一家店开业"),
        story_fact("4", "个合作庄园"),
        story_fact("32", "位咖啡师"),
    ], gap=0, align="start")
    for cell in facts["children"]:
        cell["width"] = "fill_container"
    rule = rect(ids, "故事分隔线", width="fill_container", height=1,
                fill=solid("$band-rule"))
    words = k.col("品牌故事文字", [
        eyebrow("OUR STORY", "$band-muted"),
        heading("一间街角小店，\n和一个早起的习惯", 44, "$primary-foreground",
                name="故事标题"),
        k.col("品牌故事正文", [
            k.para("2014 年，我们在梧桐路口开了第一家店，\n"
                   "只有六个座位。那时候的想法很简单：\n"
                   "让住在附近的人，出门前能喝到一杯认真做的咖啡。",
                   17, 400, "$band-muted", lh=1.8),
            k.para("十年过去，店多了几家，习惯没变：\n"
                   "生豆直接向庄园采购，每周一烘豆，七天内喝完；\n"
                   "杯子可以带回来，下一杯减三块。",
                   17, 400, "$band-muted", lh=1.8),
        ], gap=16),
        rule,
        facts,
    ], gap=24)
    band = k.row("品牌故事内容", [photo, words], gap=72, align="center")
    return page_section("品牌故事", [band], fill="$primary",
                        pad=(104, PAD_X, 104, PAD_X))


# ----------------------------------------------------------------- stores
STORES = [
    ("梧桐路店", "旗舰店", "梧桐路 128 号\n地铁 2 号线梧桐路站 B 口",
     "工作日 07:00 – 21:00\n周末 08:00 – 21:00", ["堂食", "烘焙工坊", "宠物友好"]),
    ("滨河路店", None, "滨河路 56 号\n河畔步道入口旁",
     "工作日 07:30 – 19:00\n周末 08:30 – 20:00", ["堂食", "外带", "户外座位"]),
    ("青石巷店", None, "青石巷 9 号\n老书局一楼",
     "工作日 08:00 – 18:00\n周末 09:00 – 18:00", ["外带", "咖啡豆零售", "自带杯优惠"]),
]


def info_line(glyph, content):
    return k.row(f"门店信息 {content[:6]}", [
        k.icon(glyph, 18, "$primary"),
        k.para(content, 15, 400, "$foreground", lh=1.6),
    ], gap=10, align="start")


def store_card(name, badge, address, hours, services):
    title = [k.label(f"Daybreak · {name}", 20, 700, lh=1.3), k.spacer()]
    if badge:
        title.append(k.pill(badge, fill="$primary",
                            color="$primary-foreground", size=12, weight=600,
                            pad=(4, 10)))
    card = k.card(f"门店 {name}", [
        k.row(f"{name} 标题行", title, gap=8),
        k.col(f"{name} 信息", [
            info_line("map-pin", address),
            info_line("clock", hours),
        ], gap=12),
        k.divider(),
        k.row(f"{name} 服务", [
            k.pill(item, fill="$secondary", color="$secondary-foreground",
                   size=13, pad=(5, 12)) for item in services
        ], gap=8),
    ], gap=20, pad=(28, 28), radius=22)
    card["width"] = "fill_container"
    return card


def stores():
    cards = k.row("门店卡片", [store_card(*s) for s in STORES], gap=24,
                  align="start")
    return page_section("门店信息", [
        section_head("VISIT US", "门店信息",
                     "三家店都在步行可达的街区里。周一上午是烘豆时间，"
                     "梧桐路店可以隔着玻璃看。", "在地图中查看"),
        cards,
    ], pad=(112, PAD_X, 112, PAD_X))


# ----------------------------------------------------------------- footer
def footer_col(title, items):
    return k.col(f"页脚栏 {title}", [
        k.label(title, 14, 700, "$primary-foreground"),
        *[k.label(item, 14, 400, "$band-muted") for item in items],
    ], gap=14, width=160)


def footer():
    brand = k.col("页脚品牌", [
        k.row("页脚标识", [
            k.tile("sunrise", size=36, icon_size=20, fill="$primary-foreground",
                   color="$primary", radius=18),
            k.label("Daybreak Coffee", 19, 700, "$primary-foreground",
                    family=NUM),
        ], gap=10, width="fit_content"),
        k.para("从一杯好咖啡，开始每一个清晨。", 15, 400, "$band-muted",
               lh=1.7),
    ], gap=16, width=360)
    top = k.row("页脚上部", [
        brand, k.spacer(),
        footer_col("逛逛", ["招牌咖啡", "咖啡豆订阅", "礼品卡"]),
        footer_col("了解", ["品牌故事", "门店信息", "加入我们"]),
        footer_col("联系", ["在线客服", "企业团购", "每天 09:00–18:00"]),
    ], align="start", gap=32)
    bottom = k.row("页脚下部", [
        k.label("© 2026 Daybreak Coffee", 13, 400, "$band-muted"),
        k.spacer(),
        k.label("隐私政策 · 使用条款", 13, 400, "$band-muted"),
    ])
    rule = rect(ids, "页脚分隔线", width="fill_container", height=1,
                fill=solid("$band-rule"))
    return k.col("页脚", [top, rule, bottom], gap=32,
                 padding=[64, PAD_X, 48, PAD_X], fill=solid("$foreground"),
                 role="footer")


def build():
    # Fixed to the measured content height (catalogue frame_height reads a
    # number); re-measure with a fit_content render if sections change.
    page = frame(ids, "Daybreak Coffee 品牌官网", width=W,
                 height=PAGE_H, layout="vertical",
                 fill=solid("$background"), x=0, y=0)
    page["children"] = [nav(), hero(), signature(), story(), stores(),
                        footer()]
    return [page]


if __name__ == "__main__":
    out = os.path.join(REPO, "crates", "op-editor-core", "assets",
                       "scene_templates", "daybreak-coffee-site.op")
    write_doc(out, VARS, build(), "Daybreak Coffee 品牌官网", compact=True)

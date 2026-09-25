#!/usr/bin/env python3
"""city-music-fest-poster.op — 城市音乐节活动海报一套（主海报 3:4 + 社交方图 1:1）

Studio 首页「活动海报」的示例就是这一份：「为「城市音乐节」做一套活动海报，
9 月 26 日，滨江公园。荧光绿与黑色，醒目的大字，包含主海报和社交媒体方图。」
卡片标题「音乐，让城市发光」，说明「2 种版式 · 一套活动视觉」，页面
「主海报 / 社交分享」。空输入框点开始设计时它作为即时初稿载入，所以两块板
的**内容**必须与示例一一对应：活动名、口号、日期、地点都在画面上，且是全
图最大的几样东西。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：夜里的江边草坪 —— 黑的是江面和夜空，荧光绿是舞台追光打在
    烟雾上的那一道。
  - **收敛**：近黑 #0B0C0A 底 + 米白 #F4F7EC 字 + 荧光绿 #C6FF1A 一支强调；
    灰绿 #9DA58F 只给次要说明。荧光绿上的字一律用底色近黑，不用白。
  - **论证**：活动海报在信息流里只有半秒，靠的是「一眼看见日期和地点」。
    荧光绿只落在三处：「城市」二字、日期、购票条 —— 看见绿色就看见了
    关键信息。

### 结构契约

  - 两块板都是 oplib.stack：正文层是 flex 纵列（space_between 分配余量），
    装饰层只放「声波涟漪」三道同心圆环，不承载任何文字。
  - 均衡器柱（声浪）是正文层里的一行定宽矩形，底对齐 —— 它是版式的一部
    分，参与排版，不写绝对坐标。
  - 所有文字 ≥32px（cardlib 契约）；会换行的正文 fixed-width，展示字
    auto 单行。
  - 购票二维码位是**矢量示意**（黑底方块 + qr-code 线性图标），不画一个
    可扫的假码。

### 负约束

  - 阵容全部为虚构乐队名，并在画面上注明「阵容为示意」；不出现任何真实
    艺人、票务平台、赞助品牌。
  - 主办写「城市音乐节组委会」。
  - 2026 年 9 月 26 日是周六，画面上的星期与之一致。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from cardlib import NUM, SANS, SQUARE, VERTICAL
from oplib import Ids, color_vars, frame, icon_font, rect, solid, stack, text, write_doc

ids = Ids()

VARS = color_vars({
    "c-night":     "#0B0C0A",
    "c-surface":   "#171915",
    "c-paper":     "#F4F7EC",
    "c-muted":     "#9DA58F",
    "c-neon":      "#C6FF1A",
    "c-neon-dim":  "#4E6612",
    "c-on-neon":   "#0B0C0A",
    "c-line":      "#2C3027",
})

TITLE_A = "城市"
TITLE_B = "音乐节"
TAGLINE = "音乐，让城市发光"
LINEUP = ["午夜渡轮", "潮汐信号", "玻璃海", "银杏回声", "北岸电台", "灯塔合唱团"]
ORGANISER = "城市音乐节组委会"

# 均衡器柱的相对高度（0–1）。手排而不是随机：左低右高再回落，读起来像一段
# 正在升起的声浪，最高的两根落在右侧三分之二处，把视线往日期那边带。
WAVE = [0.30, 0.52, 0.40, 0.68, 0.46, 0.84, 0.62, 1.00, 0.74, 0.90,
        0.56, 0.78, 0.44, 0.60, 0.34]


# ---------------------------------------------------------------- 骨架
def col(name, children, *, gap=0, width="fill_container", align="start",
        height="fit_content", **props):
    node = frame(ids, name, width=width, height=height, layout="vertical",
                 gap=gap, alignItems=align, fill=props.pop("fill", []),
                 **props)
    node["children"] = children
    return node


def row(name, children, *, gap=0, width="fill_container", align="center",
        height="fit_content", **props):
    node = frame(ids, name, width=width, height=height, layout="horizontal",
                 gap=gap, alignItems=align, fill=props.pop("fill", []),
                 **props)
    node["children"] = children
    return node


def spacer():
    return frame(ids, "弹性留白", width="fill_container", height=1,
                 layout="none", fill=[])


def label(name, content, size, weight, color, *, family=None, lh=1.2,
          spacing=0):
    """单行展示字 / 短标签：宽度跟内容走，永不折行。"""
    fam = family or (NUM if all(ch < "⺀" for ch in content) else SANS)
    return text(ids, name, content, size, weight, color, family=fam,
                line_height=lh, width="fit_content", growth="auto",
                spacing=spacing)


def para(name, content, size, weight, color, *, lh=1.5, spacing=0,
         width="fill_container"):
    """会换行的说明：宽度由容器给，高度由内容撑。"""
    return text(ids, name, content, size, weight, color, family=SANS,
                line_height=lh, width=width, growth="fixed-width",
                spacing=spacing)


# ---------------------------------------------------------------- 部件
def masthead():
    """页首：左英文活动名（荧光绿），右「第一届」。两端对齐。"""
    return row("页首", [
        label("英文活动名", "CITY MUSIC FESTIVAL", 32, 700, "$c-neon",
              spacing=4),
        spacer(),
        label("届次", "2026 · 第一届", 32, 500, "$c-muted"),
    ])


def equalizer(name, *, max_h, bar_w, gap, count=None, width="fit_content"):
    """声浪：一行底对齐的定宽柱。偶数根亮绿、奇数根暗绿，形成节奏。"""
    values = WAVE if count is None else WAVE[:count]
    bars = []
    for index, value in enumerate(values):
        bars.append(rect(ids, f"声浪柱 {index + 1}", width=bar_w,
                         height=max(bar_w, round(max_h * value)),
                         fill=solid("$c-neon" if index % 2 == 0
                                    else "$c-neon-dim")))
    node = row(name, bars, gap=gap, width=width, height=max_h, align="end")
    if width == "fill_container":
        # Full-bleed strip: spread the bars edge to edge of the text column.
        node["justifyContent"] = "space_between"
    return node


def qr_hint(size):
    """购票码位：近黑方块 + 线性二维码图标。只是「这里放码」的提示，不可扫。"""
    glyph = icon_font(ids, "二维码图标", "qr-code", round(size * 0.62),
                      "$c-neon")
    glyph["iconFontFamily"] = "lucide"
    box = row("购票码位", [glyph], width=size, height=size,
              fill=solid("$c-on-neon"), justifyContent="center")
    return box


def ticket_bar(*, qr=104):
    """购票条：整条荧光绿，码位 + 票价 + 主办。绿底上的字一律近黑。"""
    return row("购票条", [
        qr_hint(qr),
        col("票价", [
            label("票价", "早鸟票 ¥128", 44, 800, "$c-on-neon"),
            label("票价说明", "9 月 20 日前 · 扫码购票", 32, 500,
                  "$c-on-neon"),
        ], gap=6, width="fit_content"),
        spacer(),
        col("主办", [
            label("主办标签", "主办", 32, 500, "$c-on-neon"),
            label("主办单位", ORGANISER, 32, 700, "$c-on-neon"),
        ], gap=4, width="fit_content", align="end"),
    ], gap=28, fill=solid("$c-neon"), padding=[24, 32, 24, 24])


def ripples(cx, cy, radii):
    """装饰：三道同心声波圆环，荧光绿细描边、低不透明度，压在右上角出血。"""
    nodes = []
    for index, radius in enumerate(radii):
        ring = {
            "type": "ellipse", "id": ids("e"), "name": f"声波涟漪 {index + 1}",
            "x": cx - radius, "y": cy - radius,
            "width": radius * 2, "height": radius * 2,
            "fill": [],
            "stroke": {"thickness": 3, "fill": solid("$c-neon")},
            "opacity": [0.34, 0.22, 0.12][index],
        }
        nodes.append(ring)
    return nodes


# ---------------------------------------------------------------- 01 主海报
def poster():
    C = VERTICAL
    title = col("主标题", [
        label("主标题 · 城市", TITLE_A, 196, 900, "$c-neon", lh=1.0),
        label("主标题 · 音乐节", TITLE_B, 196, 900, "$c-paper", lh=1.0),
    ], gap=8, width="fit_content")
    hero = row("主视觉行", [
        title,
        spacer(),
        equalizer("声浪", max_h=260, bar_w=14, gap=12, count=9),
    ], align="end")
    tagline = row("口号行", [
        rect(ids, "口号引线", width=72, height=8, fill=solid("$c-neon")),
        label("口号", TAGLINE, 64, 700, "$c-paper", lh=1.25),
    ], gap=28)

    date = row("日期地点", [
        label("日期", "9.26", 200, 800, "$c-neon", lh=1.0, spacing=-8),
        col("时间地点", [
            label("星期时间", "周六 14:00 — 22:00", 40, 700, "$c-paper"),
            label("地点", "滨江公园", 64, 900, "$c-paper", lh=1.2),
            label("地点说明", "江畔大草坪 · 主舞台", 32, 500, "$c-muted"),
        ], gap=10, width="fit_content"),
    ], gap=40, align="center")

    lineup = col("阵容", [
        row("阵容标题行", [
            label("阵容标题", "演出阵容", 32, 700, "$c-neon"),
            spacer(),
            label("阵容说明", "阵容为示意", 32, 400, "$c-muted"),
        ]),
        rect(ids, "阵容分隔线", width="fill_container", height=2,
             fill=solid("$c-line")),
        para("阵容名单", "  /  ".join(LINEUP[:3]) + "\n"
             + "  /  ".join(LINEUP[3:]), 44, 700, "$c-paper", lh=1.45),
    ], gap=18)

    body = col("主海报 · 正文", [
        masthead(),
        col("标题组", [hero, tagline], gap=36),
        date,
        lineup,
        ticket_bar(),
    ], width="fill_container", height="fill_container",
        padding=C.padding, justifyContent="space_between")
    shell = stack(ids, "主海报", body, ripples(1000, 110, [230, 360, 500]),
                  width=C.width, height=C.height, fill=solid("$c-night"))
    shell["x"], shell["y"] = 0, 0
    return shell


# ---------------------------------------------------------------- 02 社交方图
def square():
    C = SQUARE
    title = row("主标题", [
        label("主标题 · 城市", TITLE_A, 168, 900, "$c-neon", lh=1.0),
        label("主标题 · 音乐节", TITLE_B, 168, 900, "$c-paper", lh=1.0),
    ], gap=0, width="fit_content", align="end")
    head = col("标题组", [
        title,
        label("口号", TAGLINE, 56, 700, "$c-paper", lh=1.25),
    ], gap=24)

    wave = equalizer("声浪", max_h=120, bar_w=16, gap=0,
                     width="fill_container")

    band = row("日期条", [
        label("日期", "9.26", 176, 800, "$c-on-neon", lh=1.0, spacing=-6),
        rect(ids, "日期竖线", width=4, height=148, fill=solid("$c-on-neon")),
        col("时间地点", [
            label("星期时间", "周六 14:00 开场", 40, 700, "$c-on-neon"),
            label("地点", "滨江公园", 64, 900, "$c-on-neon", lh=1.2),
        ], gap=8, width="fit_content"),
    ], gap=36, fill=solid("$c-neon"), padding=[28, 40, 28, 36])

    foot = row("页脚", [
        label("阵容", "午夜渡轮 / 潮汐信号 / 玻璃海 等", 32, 600,
              "$c-paper"),
        spacer(),
        label("主办", ORGANISER, 32, 500, "$c-muted"),
    ], gap=24)

    body = col("社交方图 · 正文", [
        masthead(),
        head,
        wave,
        band,
        foot,
    ], width="fill_container", height="fill_container",
        padding=C.padding, justifyContent="space_between")
    shell = stack(ids, "社交分享", body, ripples(1010, 90, [200, 310, 430]),
                  width=C.width, height=C.height, fill=solid("$c-night"))
    shell["x"], shell["y"] = VERTICAL.width + 120, 0
    return shell


# 对比度（WCAG 相对亮度比，op-design-lint 门槛 2.0；按 sRGB 公式算）：
#   c-neon    on c-night  16.0     c-paper   on c-night  18.3
#   c-muted   on c-night   7.6     c-on-neon on c-neon   16.0
# 承载文字的最低一对是 c-muted on c-night 的 7.6，只用在 32px 的说明行。
# c-neon-dim 只给均衡器暗柱（非文字图形）；c-line 是 2px 分隔线。

if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "..", "..", "..",
        "crates", "op-editor-core", "assets", "scene_templates",
        "city-music-fest-poster.op")
    write_doc(out, VARS, [poster(), square()], "城市音乐节 · 活动海报", compact=True)

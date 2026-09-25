#!/usr/bin/env python3
"""focus-mode-tutorial.op — 「三步开启专注模式」截图教程（1080×1440 × 3）

Studio 首页「截图教程」的示例就是这一份：「用三步讲清楚如何在 iPhone 上开启
专注模式：打开设置、选择专注模式、创建并设定。用蓝色标注重点，适合第一次使用
的人。」空输入框点开始设计时，它作为即时初稿载入，模型再在这三张上改内容。
所以帧数与示例的页码行一一对应：打开设置 / 选择模式 / 完成设置 —— 一步一张，
没有封面页，系列标题压在每张的顶栏里（第一张用实心蓝标签，后两张退成浅色）。

### 每张的骨架（自上而下全是 flex，不写坐标）

  顶栏（系列标签 + 页码）→ STEP 序号 → 一句动作标题 → 一到两行说明 →
  截图位（fill_container）→ 截图说明 → 蓝色提示条 → 页脚（下一步）。

### 截图位

截图位是一个 frame（合法的图片拖放目标）。首页预览里它不能是一块空灰框，
所以里面用矢量画了一屏**示意界面**：主屏图标格、设置列表、专注模式详情。
只用通用的 lucide 线性图标和圆角色块，不画任何厂商标志、不模仿系统专有图标。
要点的那一处用蓝色描边圈出来，旁边的标签写「轻点」—— 这就是示例要的「用
蓝色标注重点」。用户把自己的真截图拖进截图位即可替换这一屏示意。

### 步骤口径（与系统设置的真实路径一致）

  1. 主屏幕 → 轻点「设置」（找不到时在主屏幕中间向下轻扫搜索）
  2. 设置 → 专注模式（预设含勿扰模式、睡眠、工作等；iOS 15 起提供）
  3. 选一种模式或轻点右上角「+」新建 → 允许通知的人 / App → 添加时间表
     （设好后也能从右上角下拉打开控制中心一键开关）

### 负约束

  - 不出现任何厂商 logo 与专有图标；「iPhone」只作为示例原文里的设备名出现。
  - 任何文字不小于 32px（cardlib 字阶地板），示意界面里的字也一样。
  - 一张只标一处重点：标两处，第一次用的人就不知道先点哪个了。
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from cardlib import NUM, SANS, VERTICAL, step
from oplib import (Ids, color_vars, frame, icon_font, rect, solid, stroke,
                   text, write_doc)

ids = Ids()
REPO = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
OUT = os.path.join(REPO, "crates", "op-editor-core", "assets",
                   "scene_templates", "focus-mode-tutorial.op")

VARS = color_vars({
    "c-bg":          "#F4F7FC",
    "c-surface":     "#FFFFFF",
    "c-screen":      "#E7EDF6",
    "c-ink":         "#0F1B2D",
    "c-muted":       "#56657A",
    "c-accent":      "#1D5FE0",
    "c-accent-soft": "#DCE7FC",
    "c-on-accent":   "#FFFFFF",
    "c-border":      "#D6DFEC",
    # App tile colours in the mock screens: content, not brand expression.
    "c-app-red":     "#E5484D",
    "c-app-pink":    "#D6409F",
    "c-app-indigo":  "#5B5BD6",
    "c-app-violet":  "#8E4EC6",
    "c-app-orange":  "#F76B15",
    "c-app-green":   "#30A46C",
    "c-app-sky":     "#0B84E8",
    "c-app-teal":    "#12A594",
    "c-app-gray":    "#6F7B8B",
})

C = VERTICAL
TOTAL = 3
SERIES = "iPhone 专注模式 · 三步开启"


# ---------------------------------------------------------------- 骨架
def col(name, children, *, gap=0, width="fill_container",
        height="fit_content", align=None, justify=None, **props):
    node = frame(ids, name, width=width, height=height, layout="vertical",
                 gap=gap, fill=props.pop("fill", []), **props)
    if align:
        node["alignItems"] = align
    if justify:
        node["justifyContent"] = justify
    node["children"] = children
    return node


def row(name, children, *, gap=0, width="fill_container",
        height="fit_content", align="center", justify=None, **props):
    node = frame(ids, name, width=width, height=height, layout="horizontal",
                 gap=gap, alignItems=align, fill=props.pop("fill", []),
                 **props)
    if justify:
        node["justifyContent"] = justify
    node["children"] = children
    return node


def t(name, content, scale, weight, color, *, family=SANS,
      width="fill_container", growth="fixed-width", align=None):
    size, lh, spacing = step(scale)
    return text(ids, name, content, size, weight, color, family=family,
                line_height=lh, width=width, growth=growth, align=align,
                spacing=spacing)


def tag(name, content, scale, weight, color, *, family=None):
    if family is None:
        family = NUM if all(ch < "⺀" for ch in content) else SANS
    return t(name, content, scale, weight, color, family=family,
             width="fit_content", growth="auto")


def spacer(name="弹性留白"):
    return frame(ids, name, width="fill_container", height=1,
                 layout="none", fill=[])


def icon(glyph, size, color, name=None):
    node = icon_font(ids, name or f"图标 {glyph}", glyph, size, color)
    node["iconFontFamily"] = "lucide"
    return node


def pill(name, content, *, fill, color, pad=(8, 24)):
    return row(name, [tag(f"{name} · 字", content, "caption", 700, color)],
               width="fit_content", padding=list(pad), cornerRadius=999,
               fill=solid(fill))


def app_tile(glyph, fill, *, size=108, glyph_size=52, radius=26):
    return row(f"应用图标 {glyph}", [icon(glyph, glyph_size, "$c-on-accent")],
               width=size, height=size, justify="center",
               cornerRadius=radius, fill=solid(fill))


def ring(name, child, *, radius, pad=8):
    """蓝色描边圈：只圈要点的那一处。"""
    node = row(name, [child], width="fit_content", padding=[pad, pad],
               cornerRadius=radius, justify="center")
    node["stroke"] = stroke("$c-accent", 6)
    return node


# ---------------------------------------------------------------- 母版
def top_bar(index):
    first = index == 1
    series = pill("系列标签", SERIES,
                  fill="$c-accent" if first else "$c-accent-soft",
                  color="$c-on-accent" if first else "$c-accent")
    return row("顶栏", [
        series,
        spacer(),
        tag("页码", f"{index} / {TOTAL}", "caption", 700, "$c-muted"),
    ])


def footer(hint):
    return row("页脚", [
        tag("账号名", "@ 你的账号名", "caption", 600, "$c-muted"),
        spacer(),
        row("下一步", [
            tag("下一步文字", hint, "caption", 600, "$c-accent"),
            icon("arrow-right", 32, "$c-accent"),
        ], gap=8, width="fit_content"),
    ])


def tip_bar(content):
    return row("提示条", [
        icon("lightbulb", 36, "$c-accent"),
        t("提示文字", content, "caption", 500, "$c-ink"),
    ], gap=16, align="start", padding=[24, 28], cornerRadius=20,
        fill=solid("$c-accent-soft"))


def shot(name, screen):
    """截图位：可拖放替换的 frame，里面是一屏矢量示意界面。"""
    node = col(f"截图位 · {name}", [screen], height="fill_container",
               justify="center", align="center", padding=[36, 40],
               cornerRadius=32, fill=solid("$c-screen"))
    node["stroke"] = stroke("$c-border", 2)
    return node


def card(index, name, title, desc, screen, tip, hint):
    body = col("正文", [
        col("标题组", [
            tag("步骤序号", f"STEP {index}", "caption", 800, "$c-accent"),
            t("步骤标题", title, "title-1", 700, "$c-ink"),
            t("步骤说明", desc, "body", 400, "$c-muted"),
        ], gap=12),
        shot(name, screen),
        tag("截图说明", "示意界面 · 把你自己的截图拖进上面的框即可替换",
            "caption", 400, "$c-muted"),
        tip_bar(tip),
    ], gap=28, height="fill_container")
    node = frame(ids, f"{index:02d} {name}", width=C.width, height=C.height,
                 layout="vertical", gap=32, padding=C.padding,
                 fill=solid("$c-bg"), clipContent=True,
                 x=(index - 1) * (C.width + 120), y=0)
    node["children"] = [top_bar(index), body, footer(hint)]
    return node


# ---------------------------------------------------------------- 01
HOME_APPS = [
    [("camera", "相机", "$c-app-gray"), ("calendar", "日历", "$c-app-red"),
     ("clock", "时钟", "$c-app-orange"), ("image", "照片", "$c-app-pink")],
    [("map", "地图", "$c-app-green"), ("cloud-sun", "天气", "$c-app-sky"),
     ("notebook-pen", "备忘录", "$c-app-teal"),
     ("settings", "设置", "$c-app-gray")],
]


def home_screen():
    rows = []
    for apps in HOME_APPS:
        cells = []
        for glyph, label, fill in apps:
            hot = glyph == "settings"
            tile = app_tile(glyph, fill)
            if hot:
                cells.append(col(f"应用 {label}", [
                    ring("重点圈 · 设置", tile, radius=34),
                    pill("轻点标签", "轻点", fill="$c-accent",
                         color="$c-on-accent", pad=(4, 18)),
                ], gap=10, align="center"))
            else:
                cells.append(col(f"应用 {label}", [
                    row("图标位", [tile], width="fit_content",
                        padding=[8, 8]),
                    tag("应用名", label, "caption", 500, "$c-ink"),
                ], gap=10, align="center"))
        rows.append(row("图标行", cells, gap=8, align="start"))
    return col("示意 · 主屏幕", rows, gap=40, padding=[0, 12])


# ---------------------------------------------------------------- 02
SETTINGS_ROWS = [
    ("bell", "通知", "$c-app-red"),
    ("volume-2", "声音与触感", "$c-app-pink"),
    ("moon", "专注模式", "$c-app-indigo"),
    ("hourglass", "屏幕使用时间", "$c-app-violet"),
]


def list_row(glyph, label, fill, *, hot=False, value=None, trailing=None):
    kids = [app_tile(glyph, fill, size=56, glyph_size=30, radius=14),
            tag("行名", label, "body", 600 if hot else 500, "$c-ink"),
            spacer()]
    if value:
        kids.append(tag("行值", value, "caption", 400, "$c-muted"))
    if trailing is not None:
        kids.append(trailing)
    else:
        kids.append(icon("chevron-right", 32, "$c-muted"))
    node = row(f"设置行 {label}", kids, gap=20, height=96, padding=[0, 24],
               cornerRadius=20 if hot else 0,
               fill=solid("$c-accent-soft") if hot else None)
    if hot:
        node["stroke"] = stroke("$c-accent", 5)
    return node


def divider():
    return row("分隔行", [rect(ids, "分隔线", width="fill_container", height=2,
                                fill=solid("$c-border"))],
               padding=[0, 24, 0, 100])


def grouped(name, rows):
    kids = []
    for index, item in enumerate(rows):
        if index:
            kids.append(divider())
        kids.append(item)
    node = col(name, kids, fill=solid("$c-surface"), cornerRadius=24,
               padding=[8, 8])
    return node


def settings_screen():
    rows = []
    for glyph, label, fill in SETTINGS_ROWS:
        hot = label == "专注模式"
        rows.append(list_row(
            glyph, label, fill, hot=hot,
            trailing=pill("轻点标签", "轻点", fill="$c-accent",
                          color="$c-on-accent", pad=(4, 20)) if hot
            else None))
    return col("示意 · 设置", [
        tag("屏幕标题", "设置", "title-2", 700, "$c-ink"),
        grouped("设置分组", rows),
    ], gap=20)


# ---------------------------------------------------------------- 03
def focus_screen():
    header = row("模式标题", [
        app_tile("briefcase", "$c-app-sky", size=72, glyph_size=38,
                 radius=18),
        tag("模式名", "工作", "title-2", 700, "$c-ink"),
        spacer(),
        tag("模式状态", "未开启", "caption", 500, "$c-muted"),
    ], gap=20)
    rows = [
        list_row("users", "允许通知的人", "$c-app-green", value="2 人"),
        list_row("layout-grid", "允许通知的 App", "$c-app-orange",
                 value="3 个"),
        list_row("calendar-clock", "添加时间表", "$c-app-indigo", hot=True,
                 trailing=pill("轻点标签", "轻点", fill="$c-accent",
                               color="$c-on-accent", pad=(4, 20))),
    ]
    return col("示意 · 专注模式详情", [header, grouped("设定分组", rows)],
               gap=24)


def build():
    return [
        card(1, "打开设置", "打开「设置」",
             "在主屏幕找到灰色齿轮图标的「设置」，轻点打开。",
             home_screen(),
             "找不到时，在主屏幕中间向下轻扫，搜索「设置」。",
             "下一步：选择模式"),
        card(2, "选择模式", "选择「专注模式」",
             "在设置里往下找，轻点「专注模式」，\n能看到勿扰模式、睡眠、工作等预设。",
             settings_screen(),
             "没有这一项？先把系统更新到 iOS 15 或更高版本。",
             "下一步：完成设置"),
        card(3, "完成设置", "创建并设定",
             "选一种模式，或轻点右上角「+」新建；\n再设好允许通知的人和 App，添加时间表。",
             focus_screen(),
             "设好后从右上角下拉控制中心，也能一键开关。",
             "收藏这套，下次照着做"),
    ]


# 对比度（WCAG 相对亮度比）：
#   c-ink on c-bg 16.3 · c-muted on c-bg 5.6 · c-accent on c-bg 5.4
#   c-ink on c-screen 15.0 · c-muted on c-screen 5.1
#   c-ink on c-accent-soft 14.4 · c-accent on c-accent-soft 4.6
#   c-on-accent on c-accent 5.9 · c-ink on c-surface 17.6

if __name__ == "__main__":
    write_doc(sys.argv[1] if len(sys.argv) > 1 else OUT, VARS, build(),
              "专注模式截图教程 · 三步", compact=True)

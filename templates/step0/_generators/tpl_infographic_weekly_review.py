#!/usr/bin/env python3
"""weekly-review-infographic.op — 一周创作复盘数据长图（1080×N 竖版）

Studio 首页「信息图 · 数据」的示例就是这一份：「做一张『一周创作复盘』数据
长图。示例数据：发布 12 篇内容，累计阅读 2.4 万，新增关注 320。用清晰的数
字、对比条和三条结论呈现，标明这是示例数据。」空输入框点开始设计时它作为即时
初稿载入，所以版面必须与示例一一对应：

    深色页头（示例数据标签）→ 三个大数 → 本周 vs 上周对比条 → 每日阅读柱
    → 三条结论 → 示例数据声明

与同档的 data-report-infographic 共用阅读动线（深色页头 + 区块 + 深色页
脚），但色温与主图表都换掉：那张是冷调青绿 + 横向排行条，这张是靛蓝 + 成
对对比条 + 一周七根竖柱。

### 数据自洽（示例数据也要对得上）

  - 每日阅读七根柱加起来正好 24.0k = 示例里的「累计阅读 2.4 万」。
  - 大数卡上的变化量与对比条的上周值一一对应：12 vs 9（+3）、2.4 万 vs
    2.0 万（+20%）、320 vs 275（+45）。
  - 结论一引用的「两成」= 周三 4.8k / 24.0k。
  改任何一个数都要回头改另外两处 —— 没有任何一层会替你检查。

### 图表纪律

  - 一个强调色只给要说的那一条：对比条里只有「本周」上色，上周走中性轨
    道色；七根竖柱里只有周三上色。
  - 不画图例、坐标轴、网格线：数值直接标在条尾与柱顶。
  - 汉字 Noto Sans SC，数字 Inter，全图两个字族封顶。

硬契约同 tpl_infographic_data：内容距边缘 80、配色全走 color_vars、CJK 行
高 1.2/1.3/1.7、CJK 负字距不超过 -0.02em、根帧写 x/y、ROOT_H 量出来烤回。

### 负约束

  - 不出现任何真实平台、账号名与品牌；数据一律标「示例数据」。
  - 不用蓝紫渐变、霓虹线、复杂纹理；不用 emoji 与伪 3D 图标。
  - 不靠压字号硬塞：正文最小 24px。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from oplib import (Ids, color_vars, frame, icon_font, rect, solid, text,
                   write_doc)

ids = Ids()

VARS = color_vars({
    "c-bg":          "#F5F6FB",
    "c-surface":     "#FFFFFF",
    # 页头与页脚的深底：近黑的靛蓝，和强调色同一色相家族。
    "c-band":        "#161B3D",
    "c-band-muted":  "#A9B0D6",
    "c-ink":         "#171A33",
    "c-muted":       "#5B6184",
    "c-accent":      "#4356E0",
    # 小字压在浅靛底上时用的深一档。
    "c-accent-deep": "#2F3DB0",
    "c-accent-soft": "#E3E7FC",
    # 「上周」与非强调柱：中性轨道色，退成一个整体。
    "c-track":       "#E4E6F0",
    "c-track-2":     "#BCC1D8",
    "c-border":      "#E2E4EF",
})

CJK = "Noto Sans SC"
NUM = "Inter"

W = 1080
EDGE = 80
INNER = W - EDGE * 2

LH_DISPLAY, LH_HEAD, LH_BODY = 1.2, 1.3, 1.7

# 量出来的根高（根设 fit_content 渲一次读 PNG 高度，再烤回来）。
ROOT_H = 3539

KPIS = [
    ("12", "篇", "发布内容", "比上周 +3"),
    ("2.4", "万", "累计阅读", "比上周 +20%"),
    ("320", "", "新增关注", "比上周 +45"),
]

# (指标, 本周文案, 上周文案, 本周占轨道比例, 上周占轨道比例)
# 比例按两周中较大者 = 1.0 折算：12/12、9/12；2.4/2.4、2.0/2.4；320/320、275/320。
PAIRS = [
    ("发布内容", "12 篇", "9 篇", 1.00, 0.75),
    ("累计阅读", "2.4 万", "2.0 万", 1.00, 0.83),
    ("新增关注", "320", "275", 1.00, 0.86),
]

# 一周七天的阅读量（k）。合计 24.0k = 2.4 万。
DAYS = [("周一", 2.1), ("周二", 2.6), ("周三", 4.8), ("周四", 3.9),
        ("周五", 3.2), ("周六", 4.1), ("周日", 3.3)]
PEAK_DAY = 2
COLUMN_MAX_H = 300

TAKEAWAYS = [
    "周三那篇教程贡献了全周两成阅读，同类选题值得再写一篇。",
    "周末阅读不降反升，下周试着把发布挪到周六上午。",
    "多发了 3 篇，新增关注只比上周多 45，下周先提质量。",
]

# 成对对比条的三段固定宽度：标签 / 轨道 / 数值。
PAIR_LABEL_W = 60
PAIR_VALUE_W = 120
PAIR_GAP = 20
CARD_PAD = 36
PAIR_TRACK_W = INNER - CARD_PAD * 2 - PAIR_LABEL_W - PAIR_VALUE_W - PAIR_GAP * 2
PAIR_H = 30


def band(name, *, fill, pad, gap, children, align="start"):
    node = frame(ids, name, width="fill_container", height="fit_content",
                 layout="vertical", padding=pad, gap=gap, alignItems=align,
                 fill=fill)
    node["children"] = children
    return node


def col(name, children, *, gap=16, width="fill_container", align="start",
        height="fit_content", **props):
    node = frame(ids, name, width=width, height=height, layout="vertical",
                 gap=gap, alignItems=align, fill=[], **props)
    node["children"] = children
    return node


def row(name, children, *, gap=24, align="center", width="fill_container",
        **props):
    node = frame(ids, name, width=width, height="fit_content",
                 layout="horizontal", gap=gap, alignItems=align, fill=[],
                 **props)
    node["children"] = children
    return node


def chip(label, *, bg, fg, size=24, icon=None):
    node = frame(ids, "胶囊", width="fit_content", height="fit_content",
                 layout="horizontal", padding=[10, 22], gap=10,
                 cornerRadius=999, alignItems="center",
                 justifyContent="center", fill=solid(bg))
    node["children"] = []
    if icon:
        node["children"].append(icon_font(ids, "胶囊图标", icon, size, fg))
    node["children"].append(
        text(ids, "胶囊文字", label, size, 600, fg, family=CJK,
             width="fit_content", growth="auto", line_height=1.4))
    return node


def label(name, content, size, weight, color, *, family=CJK, lh=1.4):
    """单行短标签：宽度跟内容走，永不折行。"""
    return text(ids, name, content, size, weight, color, family=family,
                width="fit_content", growth="auto", line_height=lh)


def section_head(title, note):
    return col("区块头", [
        rect(ids, "强调短线", width=72, height=8, cornerRadius=999,
             fill=solid("$c-accent")),
        text(ids, "区块标题", title, 46, 700, "$c-ink", family=CJK,
             line_height=LH_HEAD),
        text(ids, "区块说明", note, 27, 400, "$c-muted", family=CJK,
             line_height=LH_BODY),
    ], gap=16)


def white_card(name, children, *, gap=22, pad=(CARD_PAD, CARD_PAD)):
    card = col(name, children, gap=gap, padding=list(pad), cornerRadius=24)
    card["fill"] = solid("$c-surface")
    card["stroke"] = {"thickness": 2, "fill": solid("$c-border")}
    return card


# ------------------------------------------------------------------ 01 页头
def header():
    return band("01 页头", fill=solid("$c-band"), pad=[76, EDGE, 68, EDGE],
                gap=26, children=[
        chip("示例数据 · 一周创作复盘", bg="$c-accent", fg="$c-surface",
             icon="info"),
        text(ids, "主标题", "一周创作，\n清晰复盘", 76, 700, "$c-surface",
             family=CJK, line_height=LH_DISPLAY, spacing=-1.4),
        text(ids, "副标题",
             "9.15 – 9.21 这一周：三个数字、两组对比、三条结论，\n"
             "看完就知道下周该做什么。",
             28, 400, "$c-band-muted", family=CJK, line_height=LH_BODY),
    ])


# ------------------------------------------------------------------ 02 大数
def kpi_card(value, unit, name, delta):
    parts = [text(ids, "数值", value, 80, 700, "$c-accent-deep", family=NUM,
                  width="fit_content", growth="auto", line_height=1.0,
                  spacing=-3)]
    if unit:
        parts.append(label("单位", unit, 30, 600, "$c-muted", lh=1.0))
    card = col("大数卡", [
        row("数值行", parts, gap=8, align="end", width="fit_content"),
        text(ids, "指标名", name, 28, 600, "$c-ink", family=CJK,
             line_height=1.4),
        chip(delta, bg="$c-accent-soft", fg="$c-accent-deep", size=24),
    ], gap=16, padding=[32, 26], cornerRadius=20)
    card["fill"] = solid("$c-surface")
    card["stroke"] = {"thickness": 2, "fill": solid("$c-border")}
    card["height"] = "fill_container"
    return card


def kpis():
    grid = row("大数网格", [kpi_card(*entry) for entry in KPIS], gap=20,
               align="stretch")
    return band("02 大数", fill=[], pad=[64, EDGE, 0, EDGE], gap=32, children=[
        section_head("先看三个数", "这一周最值得记住的，就是这三个。"),
        grid,
    ])


# ------------------------------------------------------------------ 03 对比
def pair_bar(which, value, ratio, *, this_week):
    track = frame(ids, f"轨道 · {which}", width=PAIR_TRACK_W, height=PAIR_H,
                  layout="horizontal", alignItems="center",
                  cornerRadius=PAIR_H // 2, clipContent=True,
                  fill=solid("$c-track"))
    track["children"] = [
        rect(ids, f"条 · {which}", width=max(round(PAIR_TRACK_W * ratio),
                                              PAIR_H),
             height=PAIR_H, cornerRadius=PAIR_H // 2,
             fill=solid("$c-accent" if this_week else "$c-track-2")),
    ]
    return row(f"对比条 · {which}", [
        # Fixed width, not fit_content: the table-overflow check reserves a
        # wide floor for every flex text column, and three of these rows in a
        # row read as a table.
        text(ids, "周别", which, 24, 600 if this_week else 400,
             "$c-ink" if this_week else "$c-muted", family=CJK,
             width=PAIR_LABEL_W, line_height=1.4),
        track,
        text(ids, "条数值", value, 28, 700,
             "$c-accent-deep" if this_week else "$c-muted", family=CJK,
             width=PAIR_VALUE_W, align="right", line_height=1.4),
    ], gap=PAIR_GAP)


def pair_block(name, this_value, last_value, this_ratio, last_ratio, last):
    kids = [
        text(ids, "指标", name, 30, 700, "$c-ink", family=CJK,
             line_height=1.4),
        col("两条", [
            pair_bar("本周", this_value, this_ratio, this_week=True),
            pair_bar("上周", last_value, last_ratio, this_week=False),
        ], gap=12),
    ]
    if not last:
        kids.append(rect(ids, "分割线", width="fill_container", height=2,
                         fill=solid("$c-border")))
    return col(f"对比组 {name}", kids, gap=18)


def comparison():
    blocks = [pair_block(*entry, index == len(PAIRS) - 1)
              for index, entry in enumerate(PAIRS)]
    legend = row("图例", [
        row("图例 本周", [
            rect(ids, "色点", width=18, height=18, cornerRadius=9,
                 fill=solid("$c-accent")),
            label("图例名", "本周", 24, 500, "$c-ink"),
        ], gap=10, width="fit_content"),
        row("图例 上周", [
            rect(ids, "色点", width=18, height=18, cornerRadius=9,
                 fill=solid("$c-track-2")),
            label("图例名", "上周", 24, 500, "$c-ink"),
        ], gap=10, width="fit_content"),
    ], gap=32)
    return band("03 对比", fill=[], pad=[72, EDGE, 0, EDGE], gap=32, children=[
        section_head("和上周比一比", "三项都在涨，但涨得不一样快。"),
        white_card("对比卡", [legend, *blocks], gap=26),
    ])


# ------------------------------------------------------------------ 04 每日
def day_column(day, value, hot):
    bar_h = max(8, round(value / max(v for _, v in DAYS) * COLUMN_MAX_H))
    stack = col(f"柱列 {day}", [
        label("柱值", f"{value:.1f}k", 24, 700 if hot else 500,
              "$c-accent-deep" if hot else "$c-muted", family=NUM),
        rect(ids, f"柱 {day}", width=56, height=bar_h, cornerRadius=12,
             fill=solid("$c-accent" if hot else "$c-track-2")),
    ], gap=10, align="center", height=COLUMN_MAX_H + 44)
    stack["justifyContent"] = "end"
    return col(f"柱组 {day}", [
        stack,
        label("日期", day, 24, 700 if hot else 400,
              "$c-ink" if hot else "$c-muted"),
    ], gap=14, align="center")


def daily():
    cols = [day_column(day, value, index == PEAK_DAY)
            for index, (day, value) in enumerate(DAYS)]
    chart = row("每日柱图", cols, gap=0, align="end",
                justifyContent="space_between")
    base = rect(ids, "基线", width="fill_container", height=2,
                fill=solid("$c-border"))
    total = row("合计行", [
        label("合计名", "七天合计", 26, 500, "$c-muted"),
        label("合计值", "24.0k ≈ 2.4 万", 28, 700, "$c-ink"),
    ], gap=16)
    return band("04 每日", fill=[], pad=[72, EDGE, 0, EDGE], gap=32, children=[
        section_head("阅读都花在了哪天", "每天的阅读量，单位千次。周三那根最高。"),
        white_card("柱图卡", [chart, base, total], gap=20,
                   pad=(40, CARD_PAD)),
    ])


# ------------------------------------------------------------------ 05 结论
def takeaways():
    items = []
    for index, line in enumerate(TAKEAWAYS, 1):
        badge = frame(ids, f"序号 {index}", width=48, height=48,
                      layout="horizontal", alignItems="center",
                      justifyContent="center", cornerRadius=24,
                      fill=solid("$c-accent-deep"))
        badge["children"] = [label("序号数字", str(index), 26, 700,
                                   "$c-surface", family=NUM, lh=1.0)]
        items.append(row("结论项", [
            badge,
            text(ids, "结论文字", line, 28, 500, "$c-ink", family=CJK,
                 line_height=LH_BODY),
        ], gap=20, align="start"))
    panel = col("结论面板", items, gap=24, padding=[40, 36], cornerRadius=24)
    panel["fill"] = solid("$c-accent-soft")
    return band("05 结论", fill=[], pad=[72, EDGE, 76, EDGE], gap=32, children=[
        section_head("三条结论", "每一条都能直接排进下周的计划。"),
        panel,
    ])


# ------------------------------------------------------------------ 06 页脚
def footer():
    return band("06 页脚", fill=solid("$c-band"), pad=[44, EDGE], gap=14,
                children=[
        row("声明行", [
            icon_font(ids, "声明图标", "info", 26, "$c-band-muted"),
            text(ids, "声明",
                 "示例数据：仅用于演示版式，换成你自己后台的数字即可。",
                 24, 400, "$c-band-muted", family=CJK, line_height=1.6),
        ], gap=12, align="center"),
        row("署名行", [
            label("账号名", "@ 你的账号名", 26, 600, "$c-surface"),
            label("更新说明", "每周一张复盘图", 24, 400, "$c-band-muted"),
        ], gap=16),
    ])


def build():
    page = frame(ids, "一周创作复盘长图", width=W, height=ROOT_H,
                 layout="vertical", gap=0, fill=solid("$c-bg"),
                 clipContent=True)
    page["children"] = [header(), kpis(), comparison(), daily(), takeaways(),
                        footer()]
    page["x"], page["y"] = 0, 0
    return [page]


# 对比度（WCAG 相对亮度比，op-design-lint 门槛 2.0）：
#   c-surface     on c-band        16.9    c-band-muted on c-band        7.6
#   c-ink         on c-bg          16.2    c-muted      on c-bg          5.8
#   c-accent-deep on c-surface      8.3    c-accent-deep on c-accent-soft 6.7
#   c-surface     on c-accent       5.6    c-ink        on c-accent-soft 14.0
# 强调色 #4356E0 只承载白字胶囊、色条与柱，不承载浅底小字。

if __name__ == "__main__":
    write_doc(sys.argv[1], VARS, build(), "一周创作复盘数据长图")

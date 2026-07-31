#!/usr/bin/env python3
"""slide-deck.op — 16:9 演示文稿模板（封面 + 目录 + 要点 + 数据 + 图表 + 结尾）

排版遵循 skills/domains/slides.md 的硬契约：
  - 每帧 1920×1080，内容距边缘 ≥100px
  - 正文 ≥24（取 28-32），标题 ≥40，关键数字 80-200
  - 行高：标题 1.1-1.2，正文放宽
  - 最多 2 个字体族，靠字重而非字号数量做层级
  - 2-3 个主色 + 中性色，强调色只用于强调
"""

import sys

from oplib import (Ids, color_vars, frame, rect, solid, stroke, text,
                   write_doc)

ids = Ids()

VARS = color_vars({
    "c-bg":          "#FFFFFF",
    "c-surface":     "#F4F6F9",
    "c-ink":         "#0B1220",
    "c-muted":       "#5A6B85",
    "c-accent":      "#2F5BEA",
    "c-accent-soft": "#E4EAFD",
    "c-border":      "#DCE2EC",
})

# 幻灯片尺寸与安全区。EDGE 同时是每帧的 padding —— 契约要求 ≥100。
W, H = 1920, 1080
EDGE = 120


def slide(name, *, fill="$c-bg", gap=48, justify="start"):
    """一帧幻灯片。固定 1920×1080，绝不 fit_content：投影比例是硬约束。"""
    node = frame(ids, name, width=W, height=H, layout="vertical",
                 padding=[EDGE, EDGE], gap=gap, justifyContent=justify,
                 fill=solid(fill), clipContent=True)
    node["children"] = []
    return node


def row(name, *, gap=40, align="start", width="fill_container",
        height="fit_content", fill=None, **props):
    # `fill=None` means a transparent structural wrapper; an explicit fill is
    # what makes a row a painted surface (card, panel).
    node = frame(ids, name, width=width, height=height, layout="horizontal",
                 gap=gap, alignItems=align, fill=fill or [], **props)
    node["children"] = []
    return node


def col(name, *, gap=16, width="fill_container", height="fit_content",
        fill=None, **props):
    node = frame(ids, name, width=width, height=height, layout="vertical",
                 gap=gap, fill=fill or [], **props)
    node["children"] = []
    return node


def eyebrow(label):
    """小标签。ALL CAPS 只在这种小标签上使用（契约允许的唯一例外）。"""
    node = frame(ids, "标签", width="fit_content", height="fit_content",
                 layout="horizontal", padding=[8, 18], cornerRadius=999,
                 fill=solid("$c-accent-soft"))
    node["children"] = [
        text(ids, "标签文字", label, 24, 600, "$c-accent",
             width="fit_content", spacing=1.2),
    ]
    return node


def accent_bar(width=120, height=8):
    return rect(ids, "强调条", width=width, height=height, cornerRadius=999,
                fill=solid("$c-accent"))


def cover():
    s = slide("01 封面", justify="center", gap=40)
    body = col("封面文案", gap=32)
    body["children"] = [
        eyebrow("2026 · 季度汇报"),
        text(ids, "主标题", "把复杂的事，讲成一句话", 104, 700, "$c-ink",
             line_height=1.15),
        text(ids, "副标题", "副标题写清楚这次要让听众记住的那个结论。", 34, 400,
             "$c-muted", line_height=1.4),
        accent_bar(),
    ]
    meta = row("落款", gap=32, align="center")
    meta["children"] = [
        text(ids, "演讲者", "演讲者姓名 · 团队", 28, 500, "$c-ink",
             width="fit_content"),
        text(ids, "日期", "2026.08", 28, 400, "$c-muted", width="fit_content"),
    ]
    s["children"] = [body, meta]
    return s


def agenda():
    entries = [
        ("01", "背景与问题", "我们面对的现状是什么"),
        ("02", "解决方案", "做了哪三件关键的事"),
        ("03", "结果与数据", "带来了多少可衡量的变化"),
        ("04", "下一步", "接下来三个月的计划"),
    ]
    s = slide("02 目录")
    s["children"].append(text(ids, "页标题", "目录", 64, 700, "$c-ink",
                              line_height=1.15))
    grid = row("目录网格", gap=32)
    for no, title, desc in entries:
        item = col("目录项", gap=14, padding=[36, 32], cornerRadius=20,
                   fill=solid("$c-surface"), height="fill_container")
        item["children"] = [
            text(ids, "序号", no, 40, 700, "$c-accent", width="fit_content",
                 line_height=1.1),
            text(ids, "条目标题", title, 34, 600, "$c-ink", line_height=1.25),
            text(ids, "条目说明", desc, 26, 400, "$c-muted", line_height=1.5),
        ]
        grid["children"].append(item)
    s["children"].append(grid)
    return s


def points():
    entries = [
        ("更快", "把等待时间从 8 秒压到 1.2 秒，用户不再中途离开。"),
        ("更稳", "关键路径的失败率下降到千分之一以内。"),
        ("更省", "同样的任务量，只用原来一半的算力预算。"),
    ]
    s = slide("03 要点")
    head = col("页头", gap=16)
    head["children"] = [
        text(ids, "页标题", "我们做了三件事", 64, 700, "$c-ink",
             line_height=1.15),
        text(ids, "页副标题", "每一件都能单独讲清楚，也都能被验证。", 30, 400,
             "$c-muted", line_height=1.4),
    ]
    grid = row("要点网格", gap=36)
    for index, (title, desc) in enumerate(entries, 1):
        item = col("要点", gap=20, padding=[40, 36], cornerRadius=20,
                   height="fill_container", fill=solid("$c-bg"),
                   stroke=stroke("$c-border", 2))
        item["children"] = [
            text(ids, "要点序号", f"0{index}", 32, 700, "$c-accent",
                 width="fit_content", line_height=1.1),
            text(ids, "要点标题", title, 40, 600, "$c-ink", line_height=1.2),
            text(ids, "要点说明", desc, 28, 400, "$c-muted", line_height=1.55),
        ]
        grid["children"].append(item)
    s["children"] = [head, grid]
    return s


def metrics():
    entries = [
        ("1.2s", "首屏加载", "较上季度 -85%"),
        ("99.9%", "关键路径成功率", "连续 90 天达标"),
        ("2.1x", "人均处理量", "同等人力下"),
    ]
    s = slide("04 数据")
    s["children"].append(text(ids, "页标题", "结果是可衡量的", 64, 700,
                              "$c-ink", line_height=1.15))
    grid = row("数据网格", gap=36, height="fill_container", align="stretch")
    for value, label, delta in entries:
        item = col("数据卡", gap=18, padding=[56, 44], cornerRadius=24,
                   height="fill_container", justifyContent="center",
                   fill=solid("$c-surface"))
        item["children"] = [
            # 关键数字：契约允许 80-200，这里取 140 让它成为唯一的视觉锚点。
            text(ids, "数值", value, 140, 700, "$c-accent", line_height=1.05),
            text(ids, "指标名", label, 32, 600, "$c-ink", line_height=1.25),
            text(ids, "变化", delta, 26, 400, "$c-muted", line_height=1.4),
        ]
        grid["children"].append(item)
    s["children"].append(grid)
    return s


def chart():
    s = slide("05 图表")
    head = col("页头", gap=16)
    head["children"] = [
        text(ids, "页标题", "趋势说明结论", 64, 700, "$c-ink",
             line_height=1.15),
        text(ids, "页副标题", "把图表要说明的那句话写在这里，别让听众自己找。",
             30, 400, "$c-muted", line_height=1.4),
    ]
    body = row("图表区", gap=48, height="fill_container", align="stretch")

    # 图表占位：一组等宽柱子。替换成真实数据时改高度即可，不必重建结构。
    plot = frame(ids, "图表占位", width="fill_container",
                 height="fill_container", layout="horizontal", gap=28,
                 alignItems="end", padding=[40, 40], cornerRadius=24,
                 fill=solid("$c-surface"))
    plot["children"] = [
        rect(ids, f"柱 {i + 1}", width="fill_container", height=height,
             cornerRadius=12,
             fill=solid("$c-accent" if i == 4 else "$c-accent-soft"))
        for i, height in enumerate([180, 250, 220, 330, 420])
    ]

    notes = col("图表说明", gap=24, width=520)
    for title, desc in [
        ("持续增长", "连续五个周期上升，没有出现回落。"),
        ("拐点在第四期", "投入的三项改动都在这一期生效。"),
        ("可以外推", "按当前斜率，下季度可达成目标线。"),
    ]:
        item = col("说明项", gap=8)
        item["children"] = [
            text(ids, "说明标题", title, 30, 600, "$c-ink", line_height=1.25),
            text(ids, "说明正文", desc, 26, 400, "$c-muted", line_height=1.55),
        ]
        notes["children"].append(item)

    body["children"] = [plot, notes]
    s["children"] = [head, body]
    return s


def closing():
    s = slide("06 结尾", fill="$c-ink", justify="center", gap=40)
    body = col("结尾文案", gap=32)
    body["children"] = [
        accent_bar(),
        text(ids, "结语", "谢谢，欢迎提问。", 96, 700, "#FFFFFF",
             line_height=1.15),
        text(ids, "联系方式", "name@example.com · 团队主页", 32, 400,
             "#9FB0CC", line_height=1.4),
    ]
    s["children"] = [body]
    return s


def build():
    return [cover(), agenda(), points(), metrics(), chart(), closing()]


if __name__ == "__main__":
    dst = sys.argv[1]
    write_doc(dst, VARS, build(), "演示文稿 · 16:9 模板")

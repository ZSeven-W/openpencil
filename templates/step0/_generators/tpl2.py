#!/usr/bin/env python3
"""knowledge-carousel.op — 小红书 3:4 知识轮播（封面 + 3 论点 + 总结）"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from oplib import Ids, frame, rect, path, text, solid, stroke, write_doc, color_vars

W, H, GAP = 1080, 1440, 120
PAD_X, PAD_Y = 72, 88

VARS = color_vars({
    "c-bg":          "#F4F5FA",
    "c-surface":     "#FFFFFF",
    "c-ink":         "#14183A",
    "c-muted":       "#63698C",
    "c-accent":      "#3B4CCA",
    "c-accent-soft": "#E5E8FB",
    "c-border":      "#DEE1F0",
})

ids = Ids()

POINTS = [
    ("先有结论，再有论据",
     "读者划到你这一页，只会给你一眼的时间。把结论放进标题，把论据留给正文，"
     "不要让人读完三行才知道你想说什么。你以为在铺垫，读者以为你没重点。",
     "标题写结论，正文写理由。"),
    ("一页只讲一个论点",
     "轮播的优势是节奏，不是容量。一页塞两个论点，读者一个都记不住；"
     "拆成两页，两个都记得住。多拆一页的成本几乎为零，真正贵的是读者的耐心。",
     "讲不完，就拆下一页。"),
    ("留白比字号更重要",
     "想让一句话被看见，办法不是把它放大，而是把它周围清空。"
     "四周留白足够时，30px 的字也会比挤在一堆里的 60px 更醒目。"
     "别用字号硬撑，先把周围清干净。",
     "想突出，先清空周围。"),
]


def page(name, children):
    node = frame(
        ids, name, width=W, height=H, layout="vertical",
        padding=[PAD_Y, PAD_X], gap=0,
        justifyContent="space_between", alignItems="start",
        fill=solid("$c-bg"), clipContent=True,
    )
    node["children"] = children
    return node


def block(name, children, gap=32, **extra):
    node = frame(ids, name, width="fill_container", height="fit_content",
                 layout="vertical", gap=gap, fill=[], alignItems="start")
    node["children"] = children
    node.update(extra)
    return node


def badge(label, *, fill_c="$c-accent-soft", text_c="$c-accent", size=26):
    node = frame(ids, f"徽章 · {label}", width="fit_content", height="fit_content",
                 layout="horizontal", padding=[12, 24], gap=0, cornerRadius=999,
                 alignItems="center", justifyContent="center", fill=solid(fill_c))
    node["children"] = [
        text(ids, "徽章文字", label, size, 600, text_c, width="fit_content",
             growth="auto", line_height=1.4)
    ]
    return node


def footer(page_no, total=5):
    node = frame(ids, "页脚", width="fill_container", height="fit_content",
                 layout="horizontal", justifyContent="space_between",
                 alignItems="center", gap=16, fill=[])
    node["children"] = [
        text(ids, "页脚品牌", "@ 你的账号名", 26, 500, "$c-muted",
             width="fit_content", growth="auto", line_height=1.4),
        text(ids, "页码", f"{page_no:02d} / {total:02d}", 26, 500, "$c-muted",
             width="fit_content", growth="auto", line_height=1.4),
    ]
    return node


def rule(width=120):
    return rect(ids, "强调短线", width=width, height=12, cornerRadius=6,
                fill=solid("$c-accent"))


def callout(quote):
    """轻装饰 · 金句卡：左侧 6px 品牌色边（PenStroke 单边 thickness）。"""
    node = frame(ids, "金句卡", width="fill_container", height="fit_content",
                 layout="vertical", padding=[32, 36], gap=0, cornerRadius=16,
                 alignItems="start", fill=solid("$c-surface"),
                 stroke={"thickness": {"left": 6}, "fill": solid("$c-accent")})
    node["children"] = [
        text(ids, "金句", quote, 34, 600, "$c-ink", line_height=1.5)
    ]
    return node


# ---------------------------------------------------------------- 01 封面
def toc_row(no, label):
    num = text(ids, "目录序号", f"{no:02d}", 30, 700, "$c-accent",
               width="fit_content", growth="auto", line_height=1.4,
               family="Inter")
    node = frame(ids, f"目录 {no}", width="fill_container", height="fit_content",
                 layout="horizontal", gap=24, alignItems="center", fill=[])
    node["children"] = [
        num,
        text(ids, "目录标题", label, 32, 500, "$c-ink", line_height=1.5),
    ]
    return node


def cover():
    head = block("封面头部", [badge("知识拆解")])

    hero = block("封面主标题区", [
        text(ids, "封面标题", "把一篇长文，\n拆成五张图", 88, 700, "$c-ink"),
        rule(132),
        text(ids, "封面副标题",
             "写得再好，没人读完也是白写。\n这套模板帮你把长文变成能划完的轮播。",
             32, 400, "$c-muted"),
    ], gap=36)

    toc = block("本期目录", [
        text(ids, "目录标签", "本期三个论点", 26, 600, "$c-muted",
             width="fit_content", growth="auto", line_height=1.4),
        toc_row(1, POINTS[0][0]),
        toc_row(2, POINTS[1][0]),
        toc_row(3, POINTS[2][0]),
    ], gap=22)

    return page("01 封面", [head, hero, toc, footer(1)])


# ------------------------------------------------------------ 02-04 论点页
def point_page(no):
    title, body, quote = POINTS[no - 1]
    main = block(f"0{no+1} 内容", [
        text(ids, "装饰序号", f"{no:02d}", 220, 700, "$c-accent-soft",
             width="fit_content", growth="auto", line_height=1.0,
             family="Inter"),
        rule(96),
        text(ids, "论点标题", title, 72, 700, "$c-ink"),
        text(ids, "论点阐述", body, 34, 400, "$c-muted", line_height=1.8),
    ], gap=28)
    return page(f"0{no+1} 论点 {no}", [main, callout(quote), footer(no + 1)])


# ---------------------------------------------------------------- 05 总结
def recap_card(no, label, sub):
    num = frame(ids, f"回顾序号 {no}", width=60, height=60, layout="horizontal",
                alignItems="center", justifyContent="center", cornerRadius=30,
                fill=solid("$c-accent-soft"))
    num["children"] = [
        text(ids, "序号", f"{no}", 30, 700, "$c-accent", width="fit_content",
             growth="auto", line_height=1.4, family="Inter")
    ]
    body = block("回顾文案", [
        text(ids, "回顾标题", label, 34, 600, "$c-ink", line_height=1.4),
        text(ids, "回顾说明", sub, 26, 400, "$c-muted", line_height=1.6),
    ], gap=8)
    node = frame(ids, f"回顾卡 {no}", width="fill_container", height="fit_content",
                 layout="horizontal", padding=[28, 32], gap=24,
                 alignItems="start", cornerRadius=16, fill=solid("$c-surface"))
    node["children"] = [num, body]
    return node


def summary_page():
    head = block("总结头部", [
        badge("总结"),
        text(ids, "总结标题", "三句话，\n记住这一篇。", 76, 700, "$c-ink"),
    ], gap=32)

    cards = block("回顾列表", [
        recap_card(1, "结论前置", "标题就是观点，别让人猜。"),
        recap_card(2, "一页一论点", "讲不完就拆页，别硬塞。"),
        recap_card(3, "先留白再放大", "清空周围，比放大字号有效。"),
    ], gap=20)

    cta_inner = block("关注卡内容", [
        text(ids, "关注标题", "觉得有用，就点个收藏", 40, 700, "#FFFFFF"),
        text(ids, "关注副文案", "关注我，每周拆一篇长文成图。", 28, 400,
             "#D6DBFA"),
    ], gap=16)
    cta = frame(ids, "关注引导卡", width="fill_container", height="fit_content",
                layout="vertical", padding=[44, 44], gap=0, cornerRadius=24,
                fill=solid("$c-accent"))
    cta["children"] = [cta_inner]

    return page("05 总结", [head, cards, cta, footer(5)])


def build():
    pages = [cover(), point_page(1), point_page(2), point_page(3),
             summary_page()]
    for i, p in enumerate(pages):
        p["x"] = i * (W + GAP)
        p["y"] = 0
    write_doc(sys.argv[1], VARS, pages, "知识轮播 · 小红书 3:4 模板")


if __name__ == "__main__":
    build()

#!/usr/bin/env python3
"""screenshot-tutorial.op — 小红书 3:4 截图教程卡（封面 + 3 步骤 + CTA）"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from oplib import (Ids, frame, rect, text, solid, stroke, write_doc,
                   color_vars, group, upload_disc, asset_fill,
                   PLACEHOLDER_DISC, PLACEHOLDER_ICON,
                   PLACEHOLDER_TITLE, PLACEHOLDER_SPEC)

W, H, GAP = 1080, 1440, 120
PAD_X, PAD_Y = 72, 88

VARS = color_vars({
    "c-bg":          "#FDF8F3",
    "c-surface":     "#FFFFFF",
    "c-ink":         "#1A1512",
    "c-muted":       "#7D6F64",
    "c-accent":      "#FF5A2C",
    "c-accent-soft": "#FFEDE5",
    "c-border":      "#EDE0D4",
})

ids = Ids()



def page(name, children, **extra):
    # gap=48 是硬下限：步骤页的占位框用 fill_container 吃掉全部余量，
    # 没有这个 gap，说明文案会紧贴页脚（taffy 会先扣 gap 再分配剩余空间）。
    node = frame(
        ids, name,
        width=W, height=H, layout="vertical",
        padding=[PAD_Y, PAD_X], gap=48,
        justifyContent="space_between", alignItems="start",
        fill=solid("$c-bg"), clipContent=True,
    )
    node["children"] = children
    node.update(extra)
    return node


def block(name, children, gap=32, **extra):
    """Transparent structural wrapper — no fill (design-principles.md)."""
    node = frame(ids, name, width="fill_container", height="fit_content",
                 layout="vertical", gap=gap, fill=[], alignItems="start")
    node["children"] = children
    node.update(extra)
    return node


def badge(label, *, fill_c, text_c, size=26):
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


# 三个步骤页的空态提示文案 —— bake_hints.py 用它烘焙 assets/*.png
HINTS = [
    ("拖入你的截图", "支持 PNG / JPG，建议宽度 ≥ 1080px"),
    ("拖入标注后的截图", "圈选/箭头建议用品牌色，粗细 4-6px"),
    ("拖入成品截图", "成品图建议留 5% 以上的四周留白"),
]
SLOT_W, SLOT_H = 936, 845          # 实测占位框布局尺寸，仅用于烘焙提示图
HINT_ASSET = "screenshot-tutorial-{i}.png"


def hint_children(hint, spec):
    """空态提示的矢量构造 —— 只被 bake_hints.py 用来烘焙成 PNG。

    模板本身不再挂这些节点：它们被烤进占位框的 fill[0]，这样拖图时
    set_node_fill_image_url 直接覆写 fill[0]（image_fill_upload.rs:88
    `fills[0] = body`），提示随之消失，用户零手工。
    """
    return [
        upload_disc(ids, "上传图标", 112, PLACEHOLDER_DISC, 52,
                    PLACEHOLDER_ICON),
        text(ids, "占位提示", hint, 32, 600, PLACEHOLDER_TITLE, align="center",
             line_height=1.4),
        text(ids, "占位规格", spec, 24, 400, PLACEHOLDER_SPEC, align="center"),
    ]


def shot_slot(idx):
    """截图占位框 —— 空态提示是 fill[0] 的内嵌 PNG，没有子节点。

    fill[1] 保留 $c-surface：提示图透明底，白卡仍由设计变量驱动；拖图后
    fill[0] 被换成用户截图（cover），fill[1] 依旧在底下兜白。
    """
    node = frame(ids, "截图占位框", width="fill_container",
                 height="fill_container", cornerRadius=28,
                 fill=[asset_fill(HINT_ASSET.format(i=idx), "fit"),
                       solid("$c-surface")[0]],
                 stroke=stroke("$c-border", 3), clipContent=True)
    return node


def deck_deco():
    """装饰用「一套图」示意 — 三张卡片叠放。

    遵守 deck 三规则：后层是纯装饰矩形（无任何文字/图标），偏移只做 14/28px
    的 peek（不重排），前层用不透明 surface 填充压住后层。
    layout="none" 下 children[0] 最靠前。
    """
    mini = frame(ids, "装饰卡 · 正面", x=0, y=0, width=360, height=240,
                 layout="vertical", padding=28, gap=16, cornerRadius=24,
                 alignItems="start", fill=solid("$c-surface"),
                 stroke=stroke("$c-border", 2))
    mini["children"] = [
        rect(ids, "示意 · 强调块", width=132, height=14, cornerRadius=7,
             fill=solid("$c-accent")),
        rect(ids, "示意 · 文本行 1", width="fill_container", height=12,
             cornerRadius=6, fill=solid("$c-border")),
        rect(ids, "示意 · 文本行 2", width=208, height=12, cornerRadius=6,
             fill=solid("$c-border")),
    ]
    deck = frame(ids, "封面装饰 · 一套图", width=388, height=268,
                 layout="none", fill=[])
    deck["children"] = [
        mini,
        rect(ids, "装饰卡 · 后层 1", x=14, y=14, width=360, height=240,
             cornerRadius=24, fill=solid("$c-accent-soft")),
        rect(ids, "装饰卡 · 后层 2", x=28, y=28, width=360, height=240,
             cornerRadius=24, fill=solid("$c-border")),
    ]
    return deck


# ---------------------------------------------------------------- 01 封面
def cover():
    head = block("封面头部", [
        badge("新手教程", fill_c="$c-accent", text_c="#FFFFFF"),
        deck_deco(),
    ], gap=64)

    hero = block("封面主标题区", [
        text(ids, "封面标题", "三步做出你的\n第一张教程图", 92, 700, "$c-ink"),
        rect(ids, "标题高亮条", width=132, height=14, cornerRadius=7,
             fill=solid("$c-accent")),
        text(ids, "封面副标题",
             "不用学软件，套模板换图换字，\n十分钟出一套能发的教程图。",
             34, 400, "$c-muted"),
    ], gap=36)

    return page("01 封面", [head, hero, footer(1)])


# ------------------------------------------------------------ 02-04 步骤页
def step(no, title, desc):
    head = frame(ids, "步骤头部", width="fill_container", height="fit_content",
                 layout="horizontal", gap=20, alignItems="center", fill=[])
    head["children"] = [
        badge(f"STEP {no}", fill_c="$c-accent-soft", text_c="$c-accent"),
    ]
    main = block(f"0{no+1} 内容", [
        head,
        text(ids, "步骤标题", title, 54, 700, "$c-ink"),
        shot_slot(no - 1),
        text(ids, "步骤说明", desc, 30, 400, "$c-muted"),
    ], gap=36, height="fill_container")
    return page(f"0{no+1} 步骤 {no}", [main, footer(no + 1)])


# ---------------------------------------------------------------- 05 CTA
def recap_row(no, label):
    dot = frame(ids, f"序号圆点 {no}", width=56, height=56, layout="horizontal",
                alignItems="center", justifyContent="center", cornerRadius=28,
                fill=solid("$c-accent-soft"))
    dot["children"] = [
        text(ids, "序号", str(no), 28, 700, "$c-accent", width="fit_content",
             growth="auto", line_height=1.4)
    ]
    row = frame(ids, f"回顾 {no}", width="fill_container", height="fit_content",
                layout="horizontal", gap=24, alignItems="center", fill=[])
    row["children"] = [dot, text(ids, "回顾文字", label, 32, 500, "$c-ink",
                                 line_height=1.5)]
    return row


def cta_page():
    head = block("总结头部", [
        badge("总结", fill_c="$c-accent", text_c="#FFFFFF"),
        text(ids, "总结标题", "就这三步，\n你也能做出来。", 76, 700, "$c-ink"),
    ], gap=32)

    recaps = block("回顾列表", [
        recap_row(1, "选一套模板，直接打开"),
        recap_row(2, "把截图拖进占位框"),
        recap_row(3, "改文案，导出发布"),
    ], gap=24)

    cta_inner = frame(ids, "关注卡内容", width="fill_container",
                      height="fit_content", layout="vertical", gap=16, fill=[],
                      alignItems="start")
    cta_inner["children"] = [
        text(ids, "关注标题", "关注我，持续更新模板", 40, 700, "#FFFFFF"),
        text(ids, "关注副文案", "评论区回复「模板」，拿走这套源文件。",
             28, 400, "#FFE7DE"),
    ]
    cta = frame(ids, "关注引导卡", width="fill_container", height="fit_content",
                layout="vertical", padding=[44, 44], gap=0, cornerRadius=28,
                fill=solid("$c-accent"))
    cta["children"] = [cta_inner]

    # 四个块直接交给 page 的 space_between 分配；再包一层 fit_content 的
    # body 会把内容全顶到上半页，底部留一大块空洞。
    return page("05 结尾 CTA", [head, recaps, cta, footer(5)])


def build():
    pages = [cover(), step(1, "截好你要讲的那一张图",
                           "截图只留关键区域，边缘留白裁掉。窗口截图记得关掉无关的标签页和通知，"
                           "画面越干净，读者越容易看懂你在讲哪一步。",),
             step(2, "标出最该被看见的地方",
                  "在截图上加一个圈或箭头，只标一处。标注超过两个，读者就不知道该看哪里了，"
                  "重点越少，记住的越多。",),
             step(3, "配一句人话说明",
                  "标题写「做什么」，正文写「怎么做」。一页只说一件事，说不完就拆成下一页，"
                  "不要把字塞满整张图。",),
             cta_page()]
    for i, p in enumerate(pages):
        p["x"] = i * (W + GAP)
        p["y"] = 0
    dst = sys.argv[1]
    write_doc(dst, VARS, pages, "截图教程卡 · 小红书 3:4 模板")


if __name__ == "__main__":
    build()

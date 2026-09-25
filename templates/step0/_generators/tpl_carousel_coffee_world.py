#!/usr/bin/env python3
"""coffee-world-carousel.op — 「咖啡的世界」知识卡片（1080×1440 × 5）

Studio 首页「图文卡片」的示例就是这一份：「做一套「咖啡的世界」知识卡片，
介绍阿拉比卡与罗布斯塔、产区、烘焙和风味。5 页轮播，暖杏色，杂志排版。」
空输入框点开始设计时，它作为即时初稿载入，模型再在这五页上改内容。所以
五页的结构与示例的页码行一一对应：封面 / 品种 / 产区 / 烘焙 / 风味。

### 杂志排版的三个信号

  - **刊头**：每页顶上一行「咖啡的世界 · COFFEE ISSUE」+ 期号，下压一道
    2px 墨线 —— 这一道线是整套的第一识别信号。
  - **衬线标题**：章节标题一律走 cardlib.SERIF（宋体），正文走黑体，
    数字走 Inter。三种字族各司其职，不混用。
  - **引语**：每个内页收尾一句加粗衬线引语，左边一道强调色竖线。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：杏子果肉的暖橙、烘焙豆由浅到深的三档棕、旧杂志纸的米杏。
  - **收敛**：#FBEEDF 纸底 + #FFF8F1 卡面 + #2B1A12 可可墨；强调色取熟杏
    压深一档（#B8531F），白底与纸底上都读得清。三档烘焙色只出现在第 4 页
    的色块里，它们是**内容**不是装饰，所以单独成变量。
  - **论证**：暖杏是「甜香」的颜色，与咖啡的焦糖、坚果风味同族；可可墨代
    替纯黑，整套不出现冷色。

### 图与插画

  封面主图用仓库里已有的演示图（`op-editor-ui/assets/home_examples/
  coffee-demo.jpg`，为首页示例生成的概念图），内联成 data URL；节点同时带
  `imageSearchQuery` / `imagePrompt`，搜图 / 生图可以一键换掉它。内页不
  堆同一张图：第 2 页是两颗矢量咖啡豆（椭圆豆形 + 中缝），第 3 页是回归线
  咖啡带示意条（不画假地图），第 4 页是三档烘焙色块和一条由浅到深的渐变
  轴，第 5 页是风味词标签。

### 事实口径（均为公开常识，可查证）

  - 阿拉比卡多种在海拔约 1000–2000 米，咖啡因约 1.2%–1.5%；罗布斯塔多在
    海拔 800 米以下，咖啡因约 2.2%–2.7%，大约是阿拉比卡的两倍。产量占比
    大致六四开。
  - 咖啡主要产区落在南北回归线之间（「咖啡带」）；巴西是全球最大的咖啡
    生产国，越南是最大的罗布斯塔产区，埃塞俄比亚被认为是阿拉比卡的原产地。
  - 浅烘在一爆前后出锅，深烘进入二爆；烘得越深，酸越低、苦与醇厚越高。

### 负约束

  - 不出现任何真实咖啡品牌、门店与 logo。
  - 不虚构数据：没有「某产区得分 92」这类编出来的数字。
  - 任何文字不小于 32px（cardlib 字阶地板）。
"""

import base64
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from cardlib import NUM, SANS, SERIF, VERTICAL, step
from oplib import (Ids, color_vars, frame, linear, path, rect, solid, stroke,
                   text, write_doc)

ids = Ids()
REPO = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
PHOTO = os.path.join(REPO, "crates", "op-editor-ui", "assets",
                     "home_examples", "coffee-demo.jpg")
OUT = os.path.join(REPO, "crates", "op-editor-core", "assets",
                   "scene_templates", "coffee-world-carousel.op")

VARS = color_vars({
    "c-bg":          "#FBEEDF",
    "c-surface":     "#FFF8F1",
    "c-ink":         "#2B1A12",
    "c-muted":       "#72523F",
    "c-accent":      "#B8531F",
    "c-accent-soft": "#F6D9BF",
    "c-border":      "#E8CFB7",
    "c-deep":        "#3A2418",
    "c-on-deep":     "#FBEEDF",
    "c-roast-light": "#C08A55",
    "c-roast-mid":   "#7B4A2A",
    "c-roast-dark":  "#3A2217",
    "c-bean-line":   "#FBEEDF",
})

C = VERTICAL
TOTAL = 5
SERIES = "咖啡的世界"


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


def tag(name, content, scale, weight, color, *, family=SANS):
    """单行短标签：宽度跟内容走，永不折行。"""
    return t(name, content, scale, weight, color, family=family,
             width="fit_content", growth="auto")


def rule(name="墨线", *, weight=2, color="$c-ink"):
    return rect(ids, name, width="fill_container", height=weight,
                fill=solid(color))


def spacer(name="弹性留白"):
    return frame(ids, name, width="fill_container", height=1,
                 layout="none", fill=[])


# ---------------------------------------------------------------- 母版
def masthead(page):
    return col("刊头", [
        row("刊头行", [
            tag("刊名", f"{SERIES} · COFFEE ISSUE", "caption", 700, "$c-ink",
                family=SERIF),
            spacer(),
            tag("期号", f"NO.{page:02d}", "caption", 600, "$c-accent",
                family=NUM),
        ]),
        rule("刊头墨线"),
    ], gap=16)


def footer(page):
    return row("页脚", [
        tag("账号名", "@ 你的账号名", "caption", 600, "$c-muted"),
        spacer(),
        tag("页码", f"{page:02d} / {TOTAL:02d}", "caption", 600, "$c-muted",
            family=NUM),
    ])


def chapter(page, kicker, title):
    """章节头：强调色小标 + 衬线大标题。"""
    return col("章节头", [
        tag("章节小标", f"{page:02d} · {kicker}", "caption", 700,
            "$c-accent"),
        t("章节标题", title, "display", 700, "$c-ink", family=SERIF),
    ], gap=16)


def pull_quote(content):
    """引语：左侧强调色竖线 + 加粗衬线。"""
    bar = rect(ids, "引语竖线", width=6, height="fill_container",
               fill=solid("$c-accent"))
    return row("引语", [
        bar,
        t("引语文字", content, "body-l", 700, "$c-ink", family=SERIF),
    ], gap=28, align="stretch")


def board(page, name, body):
    node = frame(ids, name, width=C.width, height=C.height,
                 layout="vertical", gap=32, padding=C.padding,
                 fill=solid("$c-bg"), clipContent=True,
                 x=(page - 1) * (C.width + 120), y=0)
    body["height"] = "fill_container"
    node["children"] = [masthead(page), body, footer(page)]
    return node


# ---------------------------------------------------------------- 01 封面
def cover_photo():
    with open(PHOTO, "rb") as fh:
        url = "data:image/jpeg;base64," + base64.b64encode(fh.read()).decode()
    return {
        "type": "image", "id": ids("m"), "name": "封面主图",
        "src": url, "width": 400, "height": "fill_container",
        "cornerRadius": 4, "fill": solid("$c-accent-soft"),
        "imageSearchQuery": "iced latte glass coffee beans warm light",
        "imagePrompt": ("an iced caramel latte in a tall glass beside a few "
                        "roasted coffee beans, warm apricot morning light, "
                        "editorial magazine photograph, no text"),
    }


TOC = [
    ("02", "品种", "阿拉比卡与罗布斯塔"),
    ("03", "产区", "都长在咖啡带上"),
    ("04", "烘焙", "浅、中、深三档"),
    ("05", "风味", "尝一口，说出来"),
]


def cover():
    words = col("封面文字", [
        tag("刊期", "第 01 期 · 咖啡专题", "caption", 700, "$c-accent"),
        t("封面标题", "咖啡的\n世界", "display-l", 700, "$c-ink",
          family=SERIF),
        rect(ids, "标题短线", width=96, height=8, fill=solid("$c-accent")),
        t("封面导语", "品种、产区、烘焙和风味，\n五页讲清一杯咖啡。", "body",
          400, "$c-muted"),
    ], gap=24, height="fill_container", justify="end")
    hero = row("封面主视觉", [words, cover_photo()], gap=40,
               height="fill_container", align="stretch")
    entries = []
    for number, name, desc in TOC:
        entries.append(col(f"目录项 {name}", [
            rule(f"目录线 {name}", weight=1, color="$c-border"),
            row(f"目录行 {name}", [
                tag("目录序号", number, "caption", 700, "$c-accent",
                    family=NUM),
                tag("目录名", name, "title-2", 700, "$c-ink", family=SERIF),
                spacer(),
                tag("目录说明", desc, "caption", 400, "$c-muted"),
            ], gap=28),
        ], gap=12))
    toc = col("本期目录", [
        tag("目录标题", "本期目录", "caption", 700, "$c-ink"),
        col("目录列表", entries, gap=12),
    ], gap=12)
    return board(1, "01 封面", col("封面内容", [hero, toc], gap=40))


# ---------------------------------------------------------------- 02 品种
def bean(name, width, height, crease, fill):
    """矢量咖啡豆：椭圆豆身 + 一道中缝，中缝用 flex 居中，不写坐标。"""
    body = {
        "type": "ellipse", "id": ids("e"), "name": f"{name} · 豆身",
        "width": width, "height": height, "fill": solid(fill),
    }
    seam = path(ids, f"{name} · 中缝", crease, width=24,
                height=round(height * 0.72), fill=[],
                stroke={"thickness": 6, "fill": solid("$c-bean-line")})
    holder = frame(ids, name, width=width, height=height, layout="none",
                   fill=[])
    seam["x"] = round((width - 24) / 2)
    seam["y"] = round(height * 0.14)
    body["x"], body["y"] = 0, 0
    # children[0] paints topmost: the seam sits over the bean body.
    holder["children"] = [seam, body]
    return holder


VARIETIES = [
    ("阿拉比卡", "Coffea arabica", 84, 116,
     "M12 0 C0 28 0 46 12 60 C24 74 24 92 12 120", "$c-roast-mid",
     [("种植海拔", "约 1000–2000 米"), ("咖啡因", "约 1.2%–1.5%"),
      ("风味", "酸度明亮，香气复杂")]),
    ("罗布斯塔", "Coffea canephora", 104, 116,
     "M12 0 C9 40 9 80 12 120", "$c-roast-dark",
     [("种植海拔", "多在 800 米以下"), ("咖啡因", "约 2.2%–2.7%"),
      ("风味", "苦感重，醇厚带坚果")]),
]


def variety_card(name, latin, bw, bh, crease, fill, facts):
    head = col(f"{name} · 头部", [
        row(f"{name} · 豆与名", [
            bean(f"{name} · 豆", bw, bh, crease, fill),
            tag("品种名", name, "title-2", 700, "$c-ink", family=SERIF),
        ], gap=24),
        tag("学名", latin, "caption", 400, "$c-muted", family=NUM),
    ], gap=12)
    rows = []
    for label, value in facts:
        rows.append(col(f"{name} · {label}", [
            tag("项目", label, "caption", 400, "$c-muted"),
            t("数值", value, "body", 600, "$c-ink"),
        ], gap=4))
    card = col(f"{name} 卡", [
        head, rule(f"{name} · 分隔", weight=1, color="$c-border"),
        col(f"{name} · 参数", rows, gap=20),
    ], gap=24, width="fill_container", height="fill_container",
        fill=solid("$c-surface"), padding=[32, 32], cornerRadius=24)
    card["stroke"] = stroke("$c-border", 2)
    return card


def varieties():
    cards = row("两品种对照", [variety_card(*v) for v in VARIETIES], gap=24,
                height="fill_container", align="stretch")
    body = col("品种内容", [
        chapter(2, "品种", "阿拉比卡与罗布斯塔"),
        t("品种导语", "全世界的咖啡约六成是阿拉比卡，约四成是罗布斯塔。",
          "body", 400, "$c-muted"),
        cards,
        pull_quote("罗布斯塔的咖啡因，大约是阿拉比卡的两倍。"),
    ], gap=28)
    return board(2, "02 品种", body)


# ---------------------------------------------------------------- 03 产区
def belt_line(name, label, *, dashed=False):
    line = rect(ids, f"{name} · 线", width="fill_container", height=3,
                fill=solid("$c-accent" if not dashed else "$c-muted"))
    return row(name, [
        tag(f"{name} · 名", label, "caption", 600,
            "$c-accent" if not dashed else "$c-muted"),
        line,
    ], gap=20)


def belt():
    """咖啡带示意：两条回归线夹着一条色带，赤道在正中 —— 示意，不是地图。"""
    band = row("咖啡带", [
        tag("咖啡带名", "咖啡带", "title-2", 700, "$c-ink", family=SERIF),
        spacer("咖啡带留白"),
        tag("咖啡带说明", "赤道 0° 两侧 · 温暖多雨", "caption", 600,
            "$c-muted"),
    ], gap=24, fill=solid("$c-accent-soft"), padding=[28, 36],
        cornerRadius=16)
    return col("咖啡带示意", [
        belt_line("北回归线", "北回归线 23.5°N"),
        band,
        belt_line("南回归线", "南回归线 23.5°S"),
    ], gap=18)


REGIONS = [
    ("拉丁美洲", "巴西 · 哥伦比亚 · 危地马拉",
     "全球最大的咖啡生产国，常见坚果与可可风味。"),
    ("非洲", "埃塞俄比亚 · 肯尼亚",
     "被认为是阿拉比卡的原产地，花香与柑橘明显。"),
    ("亚洲", "越南 · 印度尼西亚 · 中国云南",
     "越南是最大的罗布斯塔产区；云南以阿拉比卡为主。"),
]


def region_row(index, name, countries, note):
    return row(f"产区 {name}", [
        tag("产区序号", f"{index:02d}", "title-2", 700, "$c-accent",
            family=NUM),
        col(f"{name} · 文字", [
            row(f"{name} · 标题行", [
                tag("产区名", name, "body-l", 700, "$c-ink", family=SERIF),
                tag("国家", countries, "caption", 600, "$c-muted"),
            ], gap=20, align="end"),
            t("产区说明", note, "caption", 400, "$c-ink"),
        ], gap=6),
    ], gap=28, align="start")


def origins():
    rows = []
    for index, region in enumerate(REGIONS, 1):
        rows.append(rule(f"产区分隔 {index}", weight=1, color="$c-border"))
        rows.append(region_row(index, *region))
    body = col("产区内容", [
        chapter(3, "产区", "都长在咖啡带上"),
        t("产区导语", "咖啡树喜欢温暖多雨、不结霜的地方。", "body", 400,
          "$c-muted"),
        belt(),
        spacer("产区留白"),
        col("三大产区", rows, gap=20),
    ], gap=28)
    body["children"][3]["height"] = "fill_container"
    return board(3, "03 产区", body)


# ---------------------------------------------------------------- 04 烘焙
ROASTS = [
    ("浅烘", "Light", "$c-roast-light", "一爆前后出锅",
     "保留更多果酸与花香，豆色浅棕，适合手冲。"),
    ("中烘", "Medium", "$c-roast-mid", "一爆结束、二爆之前",
     "酸甜平衡，出现焦糖与坚果香，最百搭。"),
    ("深烘", "Dark", "$c-roast-dark", "进入二爆之后",
     "酸度低，苦味、烟熏与醇厚最突出。"),
]


def roast_row(name, latin, color, when, note):
    swatch = frame(ids, f"{name} · 色块", width=150, height="fill_container",
                   layout="vertical", justifyContent="end", padding=[0, 0, 18,
                                                                    0],
                   alignItems="center", cornerRadius=16, fill=solid(color))
    swatch["children"] = [
        tag("色块英文", latin, "caption", 700,
            "$c-ink" if latin == "Light" else "$c-on-deep", family=NUM),
    ]
    words = col(f"{name} · 文字", [
        row(f"{name} · 标题行", [
            tag("烘焙名", name, "title-2", 700, "$c-ink", family=SERIF),
            tag("出锅时机", when, "caption", 600, "$c-accent"),
        ], gap=20, align="end"),
        t("烘焙说明", note, "body", 400, "$c-ink"),
    ], gap=10)
    return row(f"烘焙 {name}", [swatch, words], gap=32, align="stretch")


def roast_axis():
    bar = rect(ids, "由浅到深", width="fill_container", height=20,
               cornerRadius=10,
               fill=linear(90, [(0, "#C08A55"), (0.5, "#7B4A2A"),
                                (1, "#3A2217")]))
    return col("风味走向", [
        bar,
        row("走向标注", [
            tag("左端", "酸度 · 花果香", "caption", 600, "$c-muted"),
            spacer(),
            tag("右端", "苦味 · 醇厚度", "caption", 600, "$c-muted"),
        ]),
    ], gap=14)


def roasting():
    rows = col("三档烘焙", [roast_row(*r) for r in ROASTS], gap=28,
               height="fill_container")
    for item in rows["children"]:
        item["height"] = "fill_container"
    body = col("烘焙内容", [
        chapter(4, "烘焙", "烘得越深，\n酸越少、苦越多"),
        rows,
        roast_axis(),
    ], gap=36)
    return board(4, "04 烘焙", body)


# ---------------------------------------------------------------- 05 风味
DIMENSIONS = [
    ("酸度", "像柑橘莓果般明亮的酸"),
    ("甜感", "焦糖、蜂蜜般的甜"),
    ("醇厚度", "入口的轻重与厚薄"),
    ("余韵", "咽下后留下的味道"),
]

NOTES = [("花果调", ["茉莉", "柑橘", "莓果"]),
         ("甜香调", ["焦糖", "蜂蜜", "红糖"]),
         ("烘焙调", ["坚果", "可可", "烟熏"])]


def chip(content, strong):
    node = row(f"风味词 {content}", [
        tag("风味词", content, "caption", 600,
            "$c-on-deep" if strong else "$c-ink"),
    ], width="fit_content", padding=[6, 24], cornerRadius=999,
        fill=solid("$c-accent" if strong else "$c-surface"))
    if not strong:
        node["stroke"] = stroke("$c-border", 2)
    return node


def flavour():
    dims = []
    for index, (name, note) in enumerate(DIMENSIONS, 1):
        number = row("序号位", [
            tag("维度序号", f"{index}", "title-2", 700, "$c-accent",
                family=NUM),
        ], width=40, justify="center")
        dims.append(row(f"维度 {name}", [
            number,
            col(f"{name} · 文字", [
                tag("维度名", name, "body-l", 700, "$c-ink", family=SERIF),
                t("维度说明", note, "caption", 400, "$c-muted"),
            ], gap=4),
        ], gap=24, align="start"))
    grid = col("四个维度", [
        row("维度第一行", dims[:2], gap=32, align="start"),
        row("维度第二行", dims[2:], gap=32, align="start"),
    ], gap=28)
    families = []
    for index, (family_name, words) in enumerate(NOTES):
        families.append(row(f"风味族 {family_name}", [
            tag("风味族名", family_name, "caption", 700, "$c-muted"),
            row(f"{family_name} · 词", [chip(w, index == 0 and i == 1)
                                        for i, w in enumerate(words)],
                gap=14, width="fit_content"),
        ], gap=24))
    notes = col("风味词表", [
        tag("风味词标题", "常见风味词", "caption", 700, "$c-ink"),
        *families,
    ], gap=16)
    summary = col("一句话总结", [
        tag("总结小标", "一句话记住", "caption", 700, "$c-accent-soft"),
        t("总结文字", "品种决定底子，产区给它性格，\n烘焙决定你喝到哪一面。",
          "body-l", 700, "$c-on-deep", family=SERIF),
    ], gap=14, fill=solid("$c-deep"), padding=[40, 44], cornerRadius=24)
    body = col("风味内容", [
        chapter(5, "风味", "尝一口，说出它的风味"),
        grid,
        notes,
        spacer("风味留白"),
        summary,
    ], gap=28)
    body["children"][3]["height"] = "fill_container"
    return board(5, "05 风味", body)


def build():
    return [cover(), varieties(), origins(), roasting(), flavour()]


# 对比度（WCAG 相对亮度比）：
#   c-ink on c-bg 14.6 · c-muted on c-bg 6.3 · c-accent on c-bg 4.3
#   c-ink on c-surface 16.0 · c-muted on c-surface 6.9
#   c-on-deep on c-deep 12.7 · c-accent-soft on c-deep 10.2
#   c-on-deep on c-accent 4.5 · c-on-deep on c-roast-light 2.3（仅 32px 粗体
#   英文标注）· c-on-deep on c-roast-mid 6.4 · on c-roast-dark 12.9

if __name__ == "__main__":
    write_doc(sys.argv[1] if len(sys.argv) > 1 else OUT, VARS, build(),
              "咖啡的世界 · 知识卡片五页", compact=True)

"""OpenPencil 产品介绍 deck 的矢量示意部件 —— 16:9 与 4:3 两版共用。

全部是色块与文字搭成的**产品示意**（提示框、生成卡、画幅缩略、导出芯片），
不是照片：这份 deck 讲的是产品本身，一张图库照片说明不了「说一句话就有初
稿」。颜色全走设计变量，换主色时示意图跟着变。

每个部件收一个缩放系数 `s`：16:9 版取 1.0，4:3 版按画板高度折算取 ~0.6；
同一个部件在两种画幅里比例一致，只是尺寸不同。
"""

from oplib import frame, rect, solid, stroke

import openpencil_intro_copy as C


class Parts:
    def __init__(self, k, ids, s=1.0):
        self.k, self.ids, self.s = k, ids, s

    def px(self, value):
        return max(1, round(value * self.s))

    def gap_fill(self, name):
        return frame(self.ids, name, width="fill_container",
                     height="fill_container", layout="none", fill=[])

    def bar(self, name, width, height, token):
        w = width if isinstance(width, str) else self.px(width)
        return rect(self.ids, name, width=w, height=self.px(height),
                    cornerRadius=self.px(height) / 2, fill=solid(token))

    # ------------------------------------------------------------ 产品窗口
    def thumb(self, index, caption, *, height=220):
        k, px = self.k, self.px
        tones = [("$primary", "$accent", "$inverse-muted"),
                 ("$secondary", "$primary", "$inverse-border"),
                 ("$muted", "$foreground", "$border")]
        body_fill, head, line = tones[index]
        art = k.col(f"缩略 {caption}", [
            rect(self.ids, "缩略标题块", width=px(96), height=px(18),
                 cornerRadius=px(4), fill=solid(head)),
            self.bar("缩略行 1", "fill_container", 10, line),
            self.bar("缩略行 2", 84, 10, line),
            self.gap_fill("缩略留白"),
            rect(self.ids, "缩略圆点", width=px(28), height=px(28),
                 cornerRadius=px(28) / 2, fill=solid(head)),
        ], gap=px(12), height=px(height), fill=solid(body_fill),
            padding=[px(20), px(18)], cornerRadius=px(14))
        return k.col(f"生成卡 {caption}", [
            art, k.label(caption, px(20), 600, "$muted-foreground"),
        ], gap=px(12), align="center")

    def window_bar(self):
        k, px = self.k, self.px
        dots = k.row("窗口圆点", [
            rect(self.ids, f"圆点 {i}", width=px(12), height=px(12),
                 cornerRadius=px(12) / 2, fill=solid("$border"))
            for i in range(3)
        ], gap=px(8), width="fit_content")
        return k.row("窗口栏", [
            dots, k.spacer(),
            k.label("OpenPencil", px(20), 600, "$muted-foreground"),
        ])

    def prompt_row(self, text, *, size=24):
        k, px = self.k, self.px
        return k.row("提示框", [
            k.tile("sparkles", size=px(44), icon_size=px(22), fill="$accent",
                   color="$accent-foreground", radius=px(12)),
            k.para(text, px(size), 500, lh=1.4),
            k.tile("arrow-up", size=px(44), icon_size=px(22),
                   fill="$primary", color="$primary-foreground",
                   radius=px(22), name="发送键"),
        ], gap=px(16), fill=solid("$muted"),
            padding=[px(14), px(14), px(14), px(18)], cornerRadius=px(18))

    def product_mock(self, *, width=660, thumb_h=220):
        """封面右侧的产品界面：提示框里的一句话 + 生成出来的三张卡。"""
        k, px = self.k, self.px
        thumbs = k.row("生成结果", [self.thumb(i, c, height=thumb_h)
                                   for i, c in enumerate(C.MOCK_CARDS)],
                       gap=px(16), align="start")
        status = k.row("生成状态", [
            k.icon("circle-check", px(22), "$primary"),
            k.label("初稿已生成 · 3 个画板，可以直接修改", px(20), 500,
                    "$muted-foreground"),
        ], gap=px(10))
        card = k.card("产品界面示意", [
            self.window_bar(), self.prompt_row(C.MOCK_PROMPT), thumbs, status,
        ], gap=px(24), pad=(px(24), px(28)), radius=px(28), border=None,
            width=px(width))
        card["effects"] = [{"type": "shadow", "offsetX": 0,
                            "offsetY": px(24), "blur": px(60), "spread": 0,
                            "color": "rgba(8,20,90,0.35)"}]
        return card

    def start_card(self, *, width=640):
        """结束页右侧的「开始设计」输入卡 —— 与封面的产品窗口首尾呼应。"""
        k, px = self.k, self.px
        field = k.col("输入区", [
            k.para(C.CLOSE_INPUT_HINT, px(24), 400, "$muted-foreground",
                   lh=1.6),
            self.gap_fill("输入留白"),
            k.row("输入工具行", [
                k.pill("演示文稿", fill="$card", color="$muted-foreground",
                       size=px(18), pad=(px(6), px(14)), glyph="presentation",
                       border="$border"),
                k.pill("海报", fill="$card", color="$muted-foreground",
                       size=px(18), pad=(px(6), px(14)), glyph="image",
                       border="$border"),
                k.spacer(),
            ], gap=px(10)),
        ], gap=px(16), height=px(230), fill=solid("$muted"),
            padding=[px(20), px(22)], cornerRadius=px(18))
        button = k.row("开始设计键", [
            k.label(C.CLOSE_BUTTON, px(24), 700, "$accent-foreground"),
            k.icon("arrow-right", px(24), "$accent-foreground"),
        ], gap=px(10), width="fit_content", fill=solid("$accent"),
            padding=[px(14), px(28)], cornerRadius=px(999), role="button")
        card = k.card("开始设计卡", [
            self.window_bar(),
            k.label(C.CLOSE_INPUT_LABEL, px(28), 700),
            field,
            k.row("按钮行", [k.spacer(), button]),
        ], gap=px(24), pad=(px(24), px(28)), radius=px(28), border=None,
            width=px(width))
        card["effects"] = [{"type": "shadow", "offsetX": 0,
                            "offsetY": px(24), "blur": px(60), "spread": 0,
                            "color": "rgba(8,20,90,0.35)"}]
        return card

    # ------------------------------------------------------------ 流程示意
    def step_visual(self, index):
        """流程卡中段「这一步长什么样」的小示意。"""
        k, px = self.k, self.px
        if index == 1:
            art = k.row("示意 提示", [
                k.icon("sparkles", px(22), "$primary"),
                k.label("秋季新品海报", px(22), 600),
                k.spacer(),
                rect(self.ids, "光标", width=px(3), height=px(26),
                     fill=solid("$primary")),
            ], gap=px(10), fill=solid("$card"), padding=[px(14), px(16)],
                cornerRadius=px(12))
            art["stroke"] = stroke("$border", 1)
        elif index == 2:
            tiles = []
            for i, (body, head) in enumerate([("$primary", "$accent"),
                                               ("$card", "$primary"),
                                               ("$card", "$foreground")]):
                tile = k.col(f"示意 画板 {i}", [
                    self.bar("画板 标题", 36, 8, head),
                    self.bar("画板 行", "fill_container", 5, "$border"),
                ], gap=px(8), height=px(96), fill=solid(body),
                    padding=[px(12), px(10)], cornerRadius=px(8))
                if body == "$card":
                    tile["stroke"] = stroke("$border", 1)
                tiles.append(tile)
            art = k.row("示意 画板", tiles, gap=px(10))
        elif index == 3:
            swatches = []
            for i, tok in enumerate(["$primary", "$accent", "$foreground",
                                     "$muted-foreground"]):
                # The selected swatch (index 0) wears a white ring.
                dot = {"type": "ellipse", "id": self.ids("e"),
                       "name": f"示意 色 {i}", "width": px(40),
                       "height": px(40), "fill": solid(tok)}
                if i == 0:
                    dot["stroke"] = stroke("$card", 4)
                swatches.append(dot)
            art = k.col("示意 调整", [
                k.row("示意 色板", swatches, gap=px(12)),
                k.row("示意 字号", [
                    k.label("Aa", px(26), 700, family="Inter"),
                    self.bar("字号 轨", "fill_container", 6, "$border"),
                    rect(self.ids, "字号 钮", width=px(22), height=px(22),
                         cornerRadius=px(11), fill=solid("$primary")),
                ], gap=px(12)),
            ], gap=px(16))
        else:
            art = k.row("示意 导出", [
                k.pill("PNG", fill="$card", color="$foreground",
                       size=px(20), weight=700, pad=(px(8), px(16)),
                       glyph="download", border="$border"),
                k.pill("分享", fill="$primary", color="$primary-foreground",
                       size=px(20), weight=700, pad=(px(8), px(16)),
                       glyph="send"),
            ], gap=px(12))
        return art

    # ------------------------------------------------------------ 成品画幅
    def mini(self, kind):
        """一种成品的缩略画幅：只用色块，画幅比例本身就是信息。"""
        k, px = self.k, self.px

        def sheet(name, w, h, head_tok, *, lines=2):
            kids = [self.bar(f"{name} 标题", round(w * 0.5), 10, head_tok)]
            kids += [self.bar(f"{name} 行 {i}", "fill_container", 6,
                              "$border") for i in range(lines)]
            node = k.col(name, kids, gap=px(8), width=px(w), height=px(h),
                         fill=solid("$card"), padding=[px(14), px(12)],
                         cornerRadius=px(8))
            node["stroke"] = stroke("$border", 1)
            return node

        if kind == "carousel":
            return k.row("缩略 轮播", [
                sheet("轮播 1", 76, 102, "$primary"),
                sheet("轮播 2", 92, 124, "$primary", lines=3),
                sheet("轮播 3", 76, 102, "$primary"),
            ], gap=px(12), width="fit_content")
        if kind == "poster":
            poster = k.col("海报", [
                rect(self.ids, "海报 色块", width="fill_container",
                     height=px(44), cornerRadius=px(4),
                     fill=solid("$accent")),
                self.bar("海报 行", 76, 8, "$inverse-muted"),
                self.bar("海报 行 2", 52, 8, "$inverse-muted"),
            ], gap=px(10), width=px(128), height=px(172),
                fill=solid("$primary"), padding=[px(16), px(14)],
                cornerRadius=px(8))
            square = k.col("方图", [
                rect(self.ids, "方图 圆", width=px(48), height=px(48),
                     cornerRadius=px(48) / 2, fill=solid("$primary")),
            ], width=px(104), height=px(104), fill=solid("$accent"),
                align="center", justify="center", cornerRadius=px(8))
            return k.row("缩略 海报", [poster, square], gap=px(14),
                         width="fit_content", align="end")
        if kind == "slides":
            chart = k.row("演示 柱", [
                rect(self.ids, f"柱 {i}", width=px(18), height=px(h),
                     cornerRadius=px(3),
                     fill=solid("$primary" if i == 3 else "$secondary"))
                for i, h in enumerate([30, 44, 38, 62])
            ], gap=px(8), width="fit_content", align="end")
            slide = k.col("演示", [
                self.bar("演示 标题", 110, 12, "$foreground"),
                k.row("演示 内容", [
                    k.col("演示 文字", [
                        self.bar(f"演示 行 {i}", "fill_container", 6,
                                 "$border") for i in range(3)
                    ], gap=px(8)),
                    chart,
                ], gap=px(16), align="end"),
            ], gap=px(18), width=px(256), height=px(144),
                fill=solid("$card"), padding=[px(16), px(16)],
                cornerRadius=px(8))
            slide["stroke"] = stroke("$border", 1)
            return slide
        blocks = [rect(self.ids, "长图 头", width="fill_container",
                       height=px(38), cornerRadius=px(4),
                       fill=solid("$primary"))]
        for i, w in enumerate(["fill_container", 60, "fill_container",
                               44]):
            blocks.append(self.bar(f"长图 段 {i}", w, 12,
                                   "$accent" if i == 1 else "$secondary"))
        long = k.col("长图", blocks, gap=px(12), width=px(104),
                     height=px(180), fill=solid("$card"),
                     padding=[px(10), px(10)], cornerRadius=px(8))
        long["stroke"] = stroke("$border", 1)
        return long

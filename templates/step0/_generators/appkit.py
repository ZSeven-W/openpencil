"""App 界面模板的共享度量层 —— 手机屏 375×812 / 桌面 1440×900。

oplib 是节点构造层；cardlib / deckkit 分别是卡片、deck 的度量层；这一层是
**App 界面体系专用**，只被 tpl_app_*.py 引用。

### 为什么 App 屏要单独一层

卡片与 deck 的字阶是按「1080 / 1920 宽的画板缩小到手机上看」推出来的，
App 屏是 1:1 的真实界面尺寸：375 宽的屏，正文 15、说明 12–13、标题 20–28，
这是 HIG / Material 3 的原生字阶（HIG Typography：Body 17pt、Footnote
13pt；M3 Body Medium 14sp）。拿卡片的 32px 地板套在 App 屏上，整屏只放得下
三行字。

### 结构契约（每一屏都成立）

  - 顶层屏 `layout: vertical`、定高 812、`clipContent`，自上而下三段：
    状态栏（44）→ 内容区（`fill_container`，自己做 flex）→ 底部标签栏。
    标签栏靠内容区的 fill 被推到底，**不写绝对坐标**。
  - 每屏写 `screen` 路由标签（"/"、"/stats" …），预览模式据此把几屏
    认成同一个 App 的页面。
  - 文字一律成对：短标签 `fit_content + auto`，会换行的正文
    `fill_container + fixed-width`。**绝不写 text 的 height。**
  - 颜色全走 shadcn 口径的设计变量（background / card / primary /
    muted-foreground / border …），换主色只改一处。
  - 图表是真数据画出来的柱与条：每根柱子的高度由数值换算，不画假地图、
    不留无内容的色块。
"""

from oplib import frame, icon_font, rect, solid, stroke, text

CJK = "Noto Sans SC"
NUM = "Inter"

PHONE_W, PHONE_H = 375, 812
STATUS_H = 44
TAB_H = 84          # 49 的标签行 + 34 的 Home 指示条安全区（HIG）
SCREEN_STRIDE = 455  # 屏与屏之间 80 的走道

DESK_W, DESK_H = 1440, 900


class Kit:
    """把 ids 绑进来，生成器里就不必每次传。"""

    def __init__(self, ids):
        self.ids = ids

    # ------------------------------------------------------------ 骨架
    def col(self, name, children, *, gap=0, width="fill_container",
            height="fit_content", fill=None, align=None, justify=None,
            **props):
        node = frame(self.ids, name, width=width, height=height,
                     layout="vertical", gap=gap, fill=fill or [], **props)
        if align:
            node["alignItems"] = align
        if justify:
            node["justifyContent"] = justify
        node["children"] = children
        return node

    def row(self, name, children, *, gap=0, width="fill_container",
            height="fit_content", fill=None, align="center", justify=None,
            **props):
        node = frame(self.ids, name, width=width, height=height,
                     layout="horizontal", gap=gap, alignItems=align,
                     fill=fill or [], **props)
        if justify:
            node["justifyContent"] = justify
        node["children"] = children
        return node

    def spacer(self, name="弹性留白"):
        """吃掉一行里剩下的宽度，把右侧元素推到行尾。"""
        return frame(self.ids, name, width="fill_container", height=1,
                     layout="none", fill=[])

    # ------------------------------------------------------------ 文字
    def label(self, content, size, weight=400, color="$foreground", *,
              family=None, lh=1.3, spacing=0, name=None, align=None):
        """单行短标签：宽度跟内容走，永不折行。"""
        fam = family or (NUM if all(ch < "⺀" for ch in str(content)) else CJK)
        return text(self.ids, name or f"标签 {content[:8]}", content, size,
                    weight, color, family=fam, line_height=lh,
                    width="fit_content", growth="auto", spacing=spacing,
                    align=align)

    def para(self, content, size, weight=400, color="$foreground", *,
             family=CJK, lh=1.5, width="fill_container", name=None,
             align=None, spacing=0):
        """会换行的正文：宽度由容器给，高度由内容撑。"""
        return text(self.ids, name or f"正文 {content[:8]}", content, size,
                    weight, color, family=family, line_height=lh,
                    width=width, growth="fixed-width", align=align,
                    spacing=spacing)

    def icon(self, glyph, size, color, name=None):
        node = icon_font(self.ids, name or f"图标 {glyph}", glyph, size, color)
        node["iconFontFamily"] = "lucide"
        return node

    # ------------------------------------------------------------ 部件
    def tile(self, glyph, *, size=40, icon_size=20, fill="$muted",
             color="$foreground", radius=12, name=None):
        """图标底块：定宽定高的圆角方块，图标居中。"""
        node = frame(self.ids, name or f"图标块 {glyph}", width=size,
                     height=size, layout="horizontal", alignItems="center",
                     justifyContent="center", cornerRadius=radius,
                     fill=solid(fill))
        node["children"] = [self.icon(glyph, icon_size, color)]
        return node

    def avatar(self, initial, *, size=40, fill="$secondary",
               color="$secondary-foreground", font=16, name="头像"):
        """首字头像：不放人像照片，名字的首字就是最稳的占位。"""
        node = frame(self.ids, name, width=size, height=size,
                     layout="horizontal", alignItems="center",
                     justifyContent="center", cornerRadius=size / 2,
                     fill=solid(fill))
        node["children"] = [self.label(initial, font, 600, color)]
        return node

    def pill(self, content, *, fill="$muted", color="$foreground", size=12,
             weight=500, pad=(4, 10), glyph=None, name=None, border=None):
        kids = []
        if glyph:
            kids.append(self.icon(glyph, size + 2, color))
        kids.append(self.label(content, size, weight, color, lh=1.2))
        node = self.row(name or f"标签胶囊 {content}", kids, gap=4,
                        width="fit_content", fill=solid(fill),
                        padding=list(pad), cornerRadius=999)
        if border:
            node["stroke"] = stroke(border, 1)
        return node

    def bar(self, value, total_width, *, height=8, fill="$primary",
            track="$muted", name="进度条"):
        """进度条：底轨定宽，实条宽度按数值换算（0–1）。"""
        filled = max(height, round(total_width * value))
        node = frame(self.ids, name, width=total_width, height=height,
                     layout="horizontal", cornerRadius=height / 2,
                     fill=solid(track), clipContent=True)
        node["children"] = [
            rect(self.ids, f"{name} · 已完成", width=filled, height=height,
                 cornerRadius=height / 2, fill=solid(fill)),
        ]
        return node

    def segmented(self, options, active, *, width, on_fill="$card",
                  track="$muted", name="分段控件"):
        """分段控件：底轨里等宽几段，选中段是一块抬起的卡。"""
        cells = []
        for index, option in enumerate(options):
            on = index == active
            cell = self.row(f"分段 {option}", [
                self.label(option, 13, 600 if on else 500,
                           "$foreground" if on else "$muted-foreground"),
            ], justify="center", height=30,
                fill=solid(on_fill) if on else None, cornerRadius=8)
            if on:
                cell["effects"] = [{"type": "shadow", "offsetX": 0,
                                    "offsetY": 1, "blur": 3, "spread": 0,
                                    "color": "rgba(15,23,42,0.10)"}]
            cells.append(cell)
        return self.row(name, cells, gap=2, width=width, fill=solid(track),
                        padding=[3, 3], cornerRadius=10)

    def card(self, name, children, *, gap=12, pad=(16, 16), fill="$card",
             radius=16, border="$border", width="fill_container",
             shadow=False, **props):
        node = self.col(name, children, gap=gap, width=width,
                        fill=solid(fill), padding=list(pad),
                        cornerRadius=radius, **props)
        if border:
            node["stroke"] = stroke(border, 1)
        if shadow:
            node["effects"] = [{
                "type": "shadow", "offsetX": 0, "offsetY": 4, "blur": 16,
                "spread": 0, "color": "rgba(15,23,42,0.06)",
            }]
        return node

    def divider(self, color="$border", name="分隔线"):
        return rect(self.ids, name, width="fill_container", height=1,
                    fill=solid(color))

    # ------------------------------------------------------------ 手机外壳
    def status_bar(self, color="$foreground"):
        """iOS 状态栏：时间 + 信号 / Wi-Fi / 电量，左右两端对齐。"""
        right = self.row("状态图标", [
            self.icon("signal", 16, color),
            self.icon("wifi", 16, color),
            self.icon("battery-full", 22, color),
        ], gap=6, width="fit_content")
        return self.row("状态栏", [
            self.label("9:41", 15, 600, color, family=NUM),
            self.spacer(),
            right,
        ], height=STATUS_H, padding=[0, 24, 0, 32], role="status-bar")

    def tab_bar(self, items, active, *, fill="$card", border="$border",
                on="$primary", off="$muted-foreground", fab=None):
        """底部标签栏。`items` 是 (glyph, 文字)；`fab` 给中间的主操作键。

        标签项等宽（每项 fill_container），中间的主操作键定宽 —— 五格宽度
        一致才不会出现「中间那个键把两侧挤歪」。
        """
        cells = []
        for index, item in enumerate(items):
            glyph, label = item[0], item[1]
            route = item[2] if len(item) > 2 else None
            if fab is not None and index == len(items) // 2:
                cells.append(fab)
            color = on if index == active else off
            cell = self.col(f"标签 {label}", [
                self.icon(glyph, 22, color),
                self.label(label, 10, 600 if index == active else 500, color,
                           lh=1.2),
            ], gap=4, align="center")
            if route and index != active:
                # 标签切换是平级替换，不压栈：来回点不会越点越深。
                cell["events"] = {"onTap": [{"replace": route}]}
            cells.append(cell)
        tabs = self.row("标签行", cells, align="center", height=49,
                        padding=[0, 8])
        indicator = self.row("Home 指示条行", [
            rect(self.ids, "Home 指示条", width=134, height=5,
                 cornerRadius=2.5, fill=solid("$foreground")),
        ], justify="center", height=35, align="center")
        node = self.col("底部标签栏", [tabs, indicator], fill=solid(fill),
                        padding=[4, 0, 0, 0], role="bottom-tab-bar")
        node["stroke"] = {"thickness": [1, 0, 0, 0], "fill": solid(border)}
        return node

    def phone(self, name, route, body, *, tab=None, fill="$background",
              status_color="$foreground", index=0):
        kids = [self.status_bar(status_color), body]
        if tab is not None:
            kids.append(tab)
        node = frame(self.ids, name, width=PHONE_W, height=PHONE_H,
                     layout="vertical", fill=solid(fill), clipContent=True,
                     x=index * SCREEN_STRIDE, y=0)
        if tab is None:
            # 没有标签栏的屏（弹层式录入页）把 Home 指示条的 34 安全区留在
            # 根上：内容区 fill 到安全区为止，最后一段不会贴着设备底边。
            node["padding"] = [0, 0, 34, 0]
        node["screen"] = route
        node["children"] = kids
        return node

    def chart_bars(self, values, *, max_h, bar_w, labels, highlight=None,
                   gap=None, on="$primary", off="$muted", label_color=None,
                   value_labels=None, name="柱状图"):
        """竖柱图：每根柱子的高度 = 数值 / 最大值 × max_h。

        柱与月份标签在同一列里上下排，柱列 `alignItems: end` 让柱子从底
        线长出来；一行里每列等宽（fill_container），所以标签永远对准柱心。
        """
        top = max(values)
        cols = []
        for index, (value, label) in enumerate(zip(values, labels)):
            hot = index == highlight
            h = max(4, round(value / top * max_h))
            bits = []
            if value_labels:
                bits.append(self.label(value_labels[index], 10,
                                       600 if hot else 500,
                                       "$foreground" if hot
                                       else "$muted-foreground", lh=1.2))
            bits.append(rect(self.ids, f"柱 {label}", width=bar_w, height=h,
                             cornerRadius=min(6, bar_w / 2),
                             fill=solid(on if hot else off)))
            stack = self.col(f"柱列 {label}", bits, gap=6, align="center",
                             justify="end", height=max_h + (20 if value_labels
                                                            else 0))
            cols.append(self.col(f"柱组 {label}", [
                stack,
                self.label(label, 11, 600 if hot else 400,
                           label_color or ("$foreground" if hot
                                           else "$muted-foreground"),
                           lh=1.2),
            ], gap=8, align="center"))
        return self.row(name, cols, gap=gap or 0, align="end")

"""Tiny helpers for authoring PenDocument (.op) JSON by hand.

Field names follow jian-ops-schema (camelCase, base+container flattened).
"""
import base64
import json
import os


class Ids:
    def __init__(self):
        self.n = 0

    def __call__(self, prefix="n"):
        self.n += 1
        return f"{prefix}{self.n}"


def solid(color):
    return [{"type": "solid", "color": color}]


def stroke(color, thickness=2):
    return {"thickness": thickness, "fill": solid(color)}


def linear(angle, stops):
    """线性渐变填充。`stops` 是 (offset, color) 序列。

    注意 op-design-lint 的对比度检测器只读 stops[0]，所以「文字压在渐变
    上」这件事必须自己按每个 stop 都量一遍（各 deck 生成器文件末尾的对比度
    表就是这么来的），别指望 lint 兜住。
    """
    return [{
        "type": "linear_gradient",
        "angle": angle,
        "stops": [{"offset": offset, "color": color} for offset, color in stops],
    }]


def radial(stops, *, cx=0.5, cy=0.5, radius=0.5):
    """径向渐变填充。cx/cy ∈ [0,1] 是节点框内的相对位置，radius 相对 max(w,h)。

    半径给到 0.5、最外一档 stop 取所在页的底色，光晕就正好在节点边界收干净，
    不会留下一圈可见的方边（TileMode::Clamp 会把边界外全刷成最后一个 stop）。
    """
    return [{
        "type": "radial_gradient",
        "cx": cx, "cy": cy, "radius": radius,
        "stops": [{"offset": offset, "color": color} for offset, color in stops],
    }]


def mix(a, b, t):
    """把 a 按 t 混向 b，返回不透明十六进制色。

    装饰层可以直接用节点 opacity，但**底色变量**不行：lint 会把半透明当成
    不透明来量对比度（`parse_hex_color` 丢 alpha），所以凡是文字要压上去的
    那一层，颜色一律在这里先算成实色再写进变量。
    """
    ca = [int(a.lstrip("#")[i:i + 2], 16) for i in (0, 2, 4)]
    cb = [int(b.lstrip("#")[i:i + 2], 16) for i in (0, 2, 4)]
    return "#" + "".join(f"{round(x + (y - x) * t):02X}" for x, y in zip(ca, cb))


def frame(ids, name, **props):
    node = {"type": "frame", "id": ids("f"), "name": name}
    node.update(props)
    return node


def rect(ids, name, **props):
    node = {"type": "rectangle", "id": ids("r"), "name": name}
    node.update(props)
    return node


def path(ids, name, d, **props):
    node = {"type": "path", "id": ids("p"), "name": name, "d": d}
    node.update(props)
    return node


def group(ids, name, **props):
    """A `group` lays out exactly like a frame but is NOT an image-drop target.

    `op-editor-core/src/image_drop.rs::node_accepts_image_drop` matches only
    Frame | Rectangle | Ellipse | Polygon | Path | Image — Group is excluded
    ("a structural wrapper with no painted body of its own"). Its `fill` is
    also never painted, so use it purely as a transparent wrapper.
    """
    node = {"type": "group", "id": ids("g"), "name": name}
    node.update(props)
    return node


def stack(ids, name, content, decor, *, width, height, fill, **props):
    """一帧的分层骨架：外壳 layout:none，内容层在上、装饰层在下。

    背景装饰（大圆弧、光晕、ghost 数字、网格暗纹）不能直接挂在正文那个
    flex 容器里 —— 它们会变成一个参与排版的兄弟，把内容挤走。所以外壳退成
    layout:none，只放两个 fill_container 的层：正文层照常做 flex，装饰层做
    绝对定位。

    **装饰层必须是最后一个孩子**：jian 的兄弟顺序是「index 0 最上」
    （`canvas_viewport_paint_mask.rs` 反着遍历 children），写反了大圆弧就会
    盖住正文。

    对比度上这层结构是中性的：lint 找底色只往上走祖先链，装饰兄弟不参与，
    正文的底仍然是外壳这一层的 `fill`。所以装饰必须自己克制到不影响可读性
    （本仓的做法：节点 opacity ≤0.5，且与底色同族）。
    """
    shell = frame(ids, name, width=width, height=height, layout="none",
                  fill=fill, clipContent=True, **props)
    if not decor:
        shell["children"] = [content]
        return shell
    ornament = frame(ids, f"{name} · 装饰", width="fill_container",
                     height="fill_container", layout="none", fill=[])
    ornament["children"] = decor
    shell["children"] = [content, ornament]
    return shell


def icon_font(ids, name, glyph, size, color, **props):
    """Lucide glyph as an `icon_font` node — non-fillable, so drops walk up."""
    node = {
        "type": "icon_font", "id": ids("i"), "name": name,
        "iconFontName": glyph, "width": size, "height": size,
        "fill": solid(color),
    }
    node.update(props)
    return node


# Calibration, all measured off real renders (see report):
#   "●" (U+25CF) at fontSize F paints a disc of 0.754*F px. Its line box is
#   F*lineHeight tall, so lineHeight 1.0 wraps a 113px disc in a 149px box and
#   injects ~17px of dead space below it. At lineHeight 0.78 the box is 1.0345*D
#   and the ink starts at y~0, which is why that value is pinned here.
#   Ink starts 0.0772*F in from the text node's left edge.
DOT_INK_RATIO = 0.754
DOT_LINE_HEIGHT = 0.78
DOT_INK_LEFT_RATIO = 0.0772
# `icon_font` scales the 24x24 lucide viewBox into the node box, so its ink is
# 0.844 of the declared size; a `path` node stretches the glyph to fill the box
# (ink 1.038 of size). This ratio keeps an icon_font swap ink-identical to the
# path it replaces.
ICONFONT_PER_PATH_PX = 1.038 / 0.844


def upload_disc(ids, name, diameter, disc_color, path_equiv_size, icon_color,
                glyph="upload"):
    """Tinted disc + upload glyph, built ONLY from non-fillable node kinds.

    A frame/ellipse disc or a `path` glyph is each a valid image-drop target
    (`image_drop.rs::node_accepts_image_drop` matches Frame|Rectangle|Ellipse|
    Polygon|Path|Image), so either would steal a drop aimed at the
    placeholder's centre. group + text + icon_font are all excluded, so a drop
    anywhere inside resolves outward to the placeholder box itself.

    `path_equiv_size` is the size the old `path` icon used; it is converted so
    the rendered glyph keeps the same ink footprint.
    """
    fs = round(diameter / DOT_INK_RATIO)
    disc = text(ids, f"{name} · 圆底", "●", fs, 400, disc_color,
                family="Inter", line_height=DOT_LINE_HEIGHT,
                width="fit_content", growth="auto")
    disc["x"] = -round(fs * DOT_INK_LEFT_RATIO)
    disc["y"] = 0
    gsize = round(path_equiv_size * ICONFONT_PER_PATH_PX)
    glyph_node = icon_font(ids, f"{name} · 图标", glyph, gsize, icon_color)
    glyph_node["x"] = round((diameter - gsize) / 2, 2)
    glyph_node["y"] = round((diameter - gsize) / 2, 2)
    # Box the group to the dot's LINE box so it can't grow past it.
    box = group(ids, name, width=diameter, height=round(fs * DOT_LINE_HEIGHT),
                layout="none", fill=[])
    # children[0] paints last (topmost): glyph over disc.
    box["children"] = [glyph_node, disc]
    return box


def text(ids, name, content, size, weight, color, *, family=None,
         line_height=None, width="fill_container", growth="fixed-width",
         align=None, spacing=0):
    """Text node. NEVER emits height — sizing is content-driven."""
    if family is None:
        family = "Noto Sans SC" if weight >= 600 or size >= 34 else "Inter"
    if line_height is None:
        # CJK ladder: display/headings tighter, body loose (cjk-typography.md)
        line_height = 1.25 if size >= 60 else 1.3 if size >= 34 else 1.6
    # CJK 负字距上限，与 cjk-typography.md / deckkit.py / qa/trackcheck.py 同源：
    # <48px 一律 0；>=48px 不得比 -0.02em 更负。**判定用比值，不先 round 上限**
    # ——round(76 × -0.02) = -2 会放行 -2，而 76px 的真实上限是 1.52。
    # 拉丁/数字节点豁免：页码、序号、Stat Value 这类 display 用 -0.03~-0.05em
    # 是正常排印，拿 CJK 的上限去卡它们是误伤。
    # 夹在这个总入口而不是各生成器里：同一个字距字面量常被多个字号复用
    # （yearreview 一个 -2 同时喂 64px 与 48px），逐处改必漏。
    if spacing < 0 and any("一" <= ch <= "鿿" for ch in str(content)):
        cap = 0 if size < 48 else round(size * 0.02, 2)
        if cap == int(cap):
            cap = int(cap)
        if abs(spacing) > cap:
            spacing = -cap if cap else 0
    node = {
        "type": "text", "id": ids("t"), "name": name,
        "content": content,
        "fontFamily": family,
        "fontSize": size,
        "fontWeight": weight,
        "fill": solid(color),
        "lineHeight": line_height,
        "letterSpacing": spacing,
        "textGrowth": growth,
    }
    if width is not None:
        node["width"] = width
    if align:
        node["textAlign"] = align
    return node


# 空态占位提示的中性灰阶 —— 故意写成字面值、不走设计变量。
# 上传占位属于「产品控件」而非品牌表达，绑主色会让「换主色只改一处」出现
# 例外：用户改了 $c-accent，其它元素全变、唯独占位圆还是旧色。中性灰对任何
# 主色都成立，所以这里烤死。
PLACEHOLDER_DISC = "#E5E7EB"
PLACEHOLDER_ICON = "#9CA3AF"
PLACEHOLDER_TITLE = "#4B5563"
PLACEHOLDER_SPEC = "#9CA3AF"

ASSET_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "assets")


def asset_fill(filename, mode="fit"):
    """Embed assets/<filename> as a data-URL image fill.

    Baked hints live in fill[0]. A dropped image overwrites exactly that slot
    (`image_fill_upload.rs`: `if fills.is_empty() { push } else { fills[0] = body }`),
    so the hint is replaced rather than stacked under the user's screenshot.
    """
    with open(os.path.join(ASSET_DIR, filename), "rb") as fh:
        url = "data:image/png;base64," + base64.b64encode(fh.read()).decode()
    return {"type": "image", "url": url, "mode": mode}


def write_doc(dst, variables, children, name, *, compact=False):
    doc = {
        "version": "1.0.0",
        "name": name,
        "variables": variables,
        "children": children,
    }
    with open(dst, "w", encoding="utf-8") as fh:
        if compact:
            json.dump(doc, fh, ensure_ascii=False, separators=(",", ":"))
        else:
            json.dump(doc, fh, ensure_ascii=False, indent=2)
        fh.write("\n")
    print(f"wrote {dst}")


def color_vars(mapping):
    return {k: {"type": "color", "value": v} for k, v in mapping.items()}

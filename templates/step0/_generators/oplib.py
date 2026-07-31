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


def write_doc(dst, variables, children, name):
    doc = {
        "version": "1.0.0",
        "name": name,
        "variables": variables,
        "children": children,
    }
    with open(dst, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, ensure_ascii=False, indent=2)
        fh.write("\n")
    print(f"wrote {dst}")


def color_vars(mapping):
    return {k: {"type": "color", "value": v} for k, v in mapping.items()}

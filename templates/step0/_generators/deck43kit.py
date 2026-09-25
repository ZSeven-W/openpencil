"""4:3 deck 的共享度量层 —— 1024×768 画板。

deckkit 是 1920×1080 的 16:9 deck 度量层；4:3 画板小了将近一半，字阶不能
直接缩放：投影到同一块幕布上，4:3 的一页在高度上与 16:9 一样满，所以字号
按**画板高度**折算（768 / 1080 ≈ 0.71），而不是按宽度（0.53）。按宽度折
算会把正文压到 15px，坐在后排就读不出来了。

  字阶（px / 行高）：封面 58 / 1.15 · 页标题 40 / 1.25 · 引语 22 / 1.5 ·
  条目标题 24 · 正文 19 / 1.6 · 注释与页脚 14 / 1.5
  （16:9 档的正文 28 / 1080 = 2.6% 画高，折到 768 是 20；页标题 56 → 40。）

板位：3 板一行，x 步长 1144（板间 120），y 步长 1008 —— 与 deckkit 同理，
第二行的帧名要留出屏幕空间，不能压到上一行的板上。
"""

from oplib import frame, rect, solid

W, H = 1024, 768
BOARDS_PER_ROW = 3
STRIDE_X, STRIDE_Y = 1144, 1008

PAD_X, PAD_TOP, PAD_BOTTOM = 64, 56, 40

FS_COVER, FS_TITLE, FS_LEAD = 58, 40, 22
FS_ITEM, FS_BODY, FS_META = 24, 19, 14


def place(boards):
    """每页写死 x/y：缺了坐标，六页会叠在原点成一页。"""
    for index, board in enumerate(boards):
        board["x"] = (index % BOARDS_PER_ROW) * STRIDE_X
        board["y"] = (index // BOARDS_PER_ROW) * STRIDE_Y
    return boards


def board(k, ids, name, body, footer, *, fill="$background"):
    """一页：定宽定高，内容区 fill 到页脚之上，页脚永远贴底。"""
    node = frame(ids, name, width=W, height=H, layout="vertical",
                 fill=solid(fill), clipContent=True,
                 padding=[PAD_TOP, PAD_X, PAD_BOTTOM, PAD_X], gap=24)
    body["height"] = "fill_container"
    node["children"] = [body, footer]
    return node


def footer(k, ids, label, index, total, *, rule="$border",
           color="$muted-foreground"):
    """页脚：细线 + 左侧 deck 名 + 右侧页码。"""
    line = rect(ids, "页脚线", width="fill_container", height=1,
                fill=solid(rule))
    row = k.row("页脚行", [
        k.label(label, FS_META, 400, color),
        k.spacer(),
        k.label(f"{index:02d} / {total:02d}", FS_META, 500, color),
    ])
    return k.col("页脚", [line, row], gap=12)

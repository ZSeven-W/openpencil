"""OpenPencil 产品介绍 deck 的共享文案与配色 —— 16:9 与 4:3 两版同源。

Studio 首页「演示文稿」的示例就是这一份：「为 OpenPencil 做一份 5 页产品介绍
PPT，面向内容创作者与小团队。包含封面、产品价值、创作流程、应用场景和结束页。
风格简洁，蓝色搭配荧光黄。」空输入框点开始设计时，按比例载入
`openpencil-intro-deck`（1920×1080）或 `openpencil-intro-deck-43`
（1024×768）作为即时初稿。两版讲的是同一件事，所以文案与配色只在这里写一次；
版式各自为自己的画幅重新构图（见两个 tpl 文件），不是等比缩放。

### 配色推导（采样 → 收敛 → 论证）

  - **采样**：一支蓝色签字笔写下的想法，旁边一道荧光笔划出的重点。
  - **收敛**：冷白 #F7F8FC 底 + 深墨蓝 #0B1233 字；主色钴蓝 #2447F5 承担
    封面、结束页整页底色与序号；荧光黄 #E4FF3A 只做「划重点」—— 封面标题
    的第二行色块、流程里「AI 协助」的标签、结束页的行动句。
  - **论证**：荧光黄在白底上对比度只有 1.1，永远不承载白底上的文字；它上面
    的字一律是深墨蓝（对比度 16.9）。钴蓝上的白字 6.4，浅蓝灰副文 4.7。

### 负约束

  - 不编造用户数、效率提升百分比或用户评价；只写产品确实能做的事。
  - 不出现第三方品牌名与 logo；示例题目用虚构的咖啡店。
  - 产品界面用矢量示意（提示框 + 生成的几张卡），不放一张泛泛的
    「团队在开会」图库照 —— 这份 deck 的主角是产品本身。
"""

from oplib import color_vars

VARS = color_vars({
    "background":           "#F7F8FC",
    "foreground":           "#0B1233",
    "card":                 "#FFFFFF",
    "card-foreground":      "#0B1233",
    "primary":              "#2447F5",
    "primary-foreground":   "#FFFFFF",
    "secondary":            "#E8EDFF",
    "secondary-foreground": "#1A33C7",
    "muted":                "#EEF1F8",
    "muted-foreground":     "#525B78",
    "accent":               "#E4FF3A",
    "accent-foreground":    "#0B1233",
    "border":               "#DDE3F0",
    "ring":                 "#2447F5",
    # 蓝底页（封面 / 结束页）上的文字与描边。
    "inverse-muted":        "#D6DEFF",
    "inverse-border":       "#4A68F7",
    "inverse-card":         "#3656F6",
})

LABEL = "OpenPencil · 产品介绍"
TOTAL = 5

COVER_EYEBROW = "OpenPencil · 产品介绍"
COVER_LINE_1 = "好想法，"
COVER_LINE_2 = "值得被看见"
# 文案里的 \n 是 16:9 版的断行（窄栏里手断，避免行首标点与孤字）；4:3 版
# 用 `flat()` 去掉它们，交给自己的栏宽重新折行。
COVER_LEAD = ("一个 AI 原生的开源设计工具：说出你的想法，\n"
              "得到一份可以直接修改的初稿，再在画布上改成你要的样子。")
COVER_META = [
    ("写给", "内容创作者与小团队"),
    ("形态", "开源 · 桌面端与网页端"),
]

# 画面里那只「产品界面」示意：提示框里的一句话 + 生成出来的三张卡。
MOCK_PROMPT = "为街角咖啡店做一套秋季新品海报"
MOCK_CARDS = ["主海报", "社交方图", "菜单卡"]

VALUE_TITLE = "把力气，\n留给想法本身"
VALUE_LEAD = "不必先学会一套专业软件，\n也不必从一张空白页开始。"
VALUES = [
    ("sparkles", "说一句话，就有初稿",
     "描述主题、对象和风格，AI 排好结构、文字和配色，\n画布上直接出现一份完整的初稿。"),
    ("mouse-pointer-2", "每一处都能改",
     "初稿是真正的设计文件，不是一张图：\n文字、颜色、图片和版式都能在画布上直接改。"),
    ("layers", "一块画布，多种成品",
     "图文卡片、演示文稿、信息图、活动海报和 App 界面，\n都在同一块画布上完成。"),
]

FLOW_TITLE = "四步，从一句话到可以发布"
FLOW_LEAD = "你决定方向和取舍，重复的排版工作交给 AI。"
FLOW = [
    ("message-square-text", "描述想法",
     "用一两句话说清主题、\n受众和风格，也可以\n直接从示例开始。", False),
    ("wand-sparkles", "生成初稿",
     "AI 按你的描述排好\n页面结构、文字层级\n和整体配色。", True),
    ("pencil", "在画布上调整",
     "直接改文字、换图片、\n调颜色，或让 AI 按\n你的意见再改一轮。", True),
    ("share", "导出分享",
     "导出成图片，发布到\n内容平台，或发给\n团队一起看。", False),
]
# 每一步卡底「这一步长什么样」的一行实例。
FLOW_EXAMPLES = ["「秋季新品海报」", "3 个画板一次出齐", "改标题 · 调主色",
                 "PNG · 发给团队"]
TAG_AI = "AI 协助"
TAG_YOU = "你来决定"

SCENE_TITLE = "一块画布，接住日常要做的图"
SCENE_LEAD = "个人创作者一个人就能做完一整套；小团队共用同一份设计文件。"
SCENES = [
    ("carousel", "图文卡片", "知识卡片、轮播图文，\n一次做完一整套。"),
    ("poster", "活动海报", "主海报和社交方图，\n风格保持一致。"),
    ("slides", "汇报演示", "产品介绍、周报复盘，\n结构清楚好讲。"),
    ("long", "信息长图", "数据、流程、对比，\n一张图说明白。"),
]

CLOSE_KICKER = "谢谢观看"
CLOSE_LINE_1 = "下一个好想法，"
CLOSE_LINE_2 = "从一句话开始"
CLOSE_LEAD = "打开 OpenPencil，选一个任务，写下你想做的东西。"
CLOSE_STEPS = ["打开 OpenPencil", "选择任务", "写下你的想法"]
CLOSE_INPUT_LABEL = "描述你想做的东西"
CLOSE_INPUT_HINT = "例如：为周六下午的读书会做一张活动海报，\n暖色调，写清时间和地点。"
CLOSE_BUTTON = "开始设计"


def flat(text):
    """去掉 16:9 版的手断行，交给当前栏宽重新折行。"""
    return text.replace("\n", "")

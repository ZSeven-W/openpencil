//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `zh_cn_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "搜索图片…",
        "imagePanel.searching" => "搜索中…",
        "imagePanel.noResults" => "未找到结果",
        "imagePanel.searchPrompt" => "搜索图片",
        "imagePanel.sourceNotice" => "图片来自 {{source}}。自由许可 — 使用前请核实许可协议。",
        "imagePanel.genNotConfigured" => "图片生成未配置",
        "imagePanel.openSettings" => "打开设置",
        "imagePanel.promptPlaceholder" => "描述要生成的图片…",
        "providerProbe.connectedViaCli" => "已通过 {{name}} CLI 连接",
        "providerProbe.cliExitedWithError" => "{{name}} CLI 退出并报错",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI 未输出版本信息",
        "providerProbe.modelQueryFailed" => "{{name}} 模型查询失败或超时",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} 模型查询失败。请先运行 {{command}} 完成认证。"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} 模型查询需要认证。请先运行 {{command}} 登录。"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} 返回了无法识别的模型列表",
        "providerProbe.connectedAs" => "已以 @{{login}}{{method}} 身份连接",
        "providerProbe.connectedViaGithub" => "已通过 GitHub 连接",
        "importProgress.figmaTitle" => "正在解析 Figma 文件…",
        "importProgress.htmlTitle" => "正在解析 HTML 和页面资源…",
        "importProgress.htmlSubtitle" => "正在读取样式和图片，请稍候",
        "importProgress.largeFileSubtitle" => "大型文件需要几秒钟，请稍候",
        "account.signedOutHint" => "登录后即可同步你的设置与偏好",
        "code.noUsableCode" => "AI 未返回可用代码。请重试，或切换 AI 模型后再试。",
        "code.previousResultKept" => "上次生成的代码仍已保留",
        "promptCenter.title" => "提示词中心",
        "promptCenter.searchPlaceholder" => "搜索提示词…",
        "promptCenter.category.all" => "全部",
        "promptCenter.category.starter" => "快速上手",
        "promptCenter.category.mobileApp" => "移动 App",
        "promptCenter.category.webPage" => "网页",
        "promptCenter.category.dashboard" => "仪表盘",
        "promptCenter.category.component" => "组件",
        "promptCenter.category.modify" => "改稿",
        "promptCenter.category.custom" => "我的",
        "promptCenter.empty" => "没有匹配的提示词",
        "promptCenter.saveCurrent" => "保存当前输入",
        "promptCenter.saveTitlePlaceholder" => "提示词标题",
        "promptCenter.save" => "保存",
        "promptCenter.cancel" => "取消",
        "promptCenter.delete" => "删除",
        "promptCenter.screens" => "{{count}} 屏",
        "promptCenter.freeform" => "自由发挥",
        "promptCenter.item.wander.title" => "Wander · 旅行行程规划",
        "promptCenter.item.forage.title" => "Forage · 时令菜谱",
        "promptCenter.item.still.title" => "Still · 冥想与睡前",
        "promptCenter.item.hearth.title" => "Hearth · 智能家居",
        "promptCenter.item.meteo.title" => "Meteo · 沉浸式天气",
        "promptCenter.item.marginalia.title" => "Marginalia · 阅读与批注",
        "promptCenter.item.lingua.title" => "Lingua · 语言学习",
        "promptCenter.item.daybreak.title" => "Daybreak · 咖啡预订",
        "promptCenter.item.verdant.title" => "Verdant · 植物养护",
        "promptCenter.item.companion.title" => "Companion · 宠物生活",
        "promptCenter.item.relic.title" => "Relic · 精品二手市集",
        "promptCenter.item.nocturne.title" => "Nocturne · 观星指南",
        "promptCenter.item.marquee.title" => "Marquee · 观影清单",
        "promptCenter.item.ritual.title" => "Ritual · 习惯养成",
        "promptCenter.item.ember.title" => "Ember · 心情日记",
        "promptCenter.item.volt.title" => "Volt · 电动车伴侣",
        "promptCenter.item.aloft.title" => "Aloft · 航班追踪",
        "promptCenter.item.gallery.title" => "Gallery · 展览与文化活动",
        "promptCenter.item.nightcap.title" => "Nightcap · 家庭调酒",
        "promptCenter.item.bloom.title" => "Bloom · 亲子成长记录",
        "promptCenter.item.extremeWeather.title" => "极限 · 天气 App",
        "promptCenter.item.extremeNowPlaying.title" => "极限 · 正在播放",
        "promptCenter.item.extremeDailyApp.title" => "极限 · 每日必开 App",
        "promptCenter.item.extremeCalendar.title" => "极限 · 日历",
        "promptCenter.item.extremeCalm.title" => "极限 · 宁静",
        "promptCenter.item.webOrbit.title" => "Orbit · AI 工作台官网",
        "promptCenter.item.webAtelier.title" => "Atelier · 家居品牌电商",
        "promptCenter.item.dashboardPulse.title" => "Pulse · 增长分析台",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · 物流运维中心",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · 企业数据表",
        "promptCenter.item.componentFormLab.title" => "Form Lab · 表单组件系统",
        "promptCenter.item.modifyPolishCurrent.title" => "精修当前界面",
        "promptCenter.item.modifyCompleteStates.title" => "补齐组件状态",
        "collab.ownerConfirm.title" => "确认你要加入谁的会话",
        "collab.ownerConfirm.hint" => "此会话的任何内容都尚未载入。",
        "collab.ownerConfirm.account" => "已验证账户",
        "collab.ownerConfirm.device" => "已验证设备",
        "collab.ownerConfirm.claimedName" => "该账户自选的名称（未经验证）",
        "collab.action.confirmOwner" => "加入此会话",
        "collab.action.rejectOwner" => "不加入",
        "collab.error.ownerNotConfirmed" => "你未确认主持人，因此未载入任何内容。",
        "sceneTemplate.title" => "场景模板",
        "sceneTemplate.searchPlaceholder" => "搜索场景或模板",
        "sceneTemplate.empty" => "没有匹配的模板",
        "sceneTemplate.frames" => "{{count}} 页",
        "sceneTemplate.filter.all" => "全部",
        "sceneTemplate.scene.tutorial" => "教程图",
        "sceneTemplate.scene.comparison" => "对比图",
        "sceneTemplate.scene.carousel" => "知识卡片",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => "三步截图教程卡",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "封面、三个操作步骤和结尾行动号召，替换截图与说明即可发布。"
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "知识观点轮播",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "封面、三个论点和总结页，适合把一个观点拆成可滑动的连续卡片。"
        }
        "sceneTemplate.item.beforeAfter.title" => "改版前后对比",
        "sceneTemplate.item.beforeAfter.summary" => {
            "左右并置的前后对比，配改动说明，适合复盘与作品展示。"
        }
        "sceneTemplate.item.slideDeck.title" => "演示文稿 · 六页",
        "sceneTemplate.item.slideDeck.summary" => {
            "封面、目录、要点、数据、图表和结尾，16:9 投影比例，替换文案即可上台。"
        }
        "fileMenu.newFromTemplate" => "从模板新建",
        _ => return super::zh_cn_collab::lookup(key),
    })
}

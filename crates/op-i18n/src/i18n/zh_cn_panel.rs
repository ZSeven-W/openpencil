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
        "promptCenter.item.webKilnform.title" => "Kilnform · 设计基建官网",
        "promptCenter.item.webReefwright.title" => "Reefwright · AI 客服知识官网",
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
        "sceneTemplate.generate.placeholder" => "描述主题，AI 直接生成整副演示文稿",
        "sceneTemplate.generate.button" => "生成",
        "sceneTemplate.generate.hint" => "新建一个文档，按主题直接生成整副演示文稿。",
        "sceneTemplate.generate.promptTemplate" => "为以下主题制作一份演示文稿（PPT）：{{topic}}",
        "sceneTemplate.card.addToCanvas" => "加入画布",
        "sceneTemplate.card.generateFrom" => "以此生成",
        "sceneTemplate.generate.basis" => "基于：",
        "sceneTemplate.filter.all" => "全部",
        "sceneTemplate.scene.tutorial" => "教程图",
        "sceneTemplate.scene.comparison" => "对比图",
        "sceneTemplate.scene.carousel" => "轮播",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.scene.card" => "卡片",
        "sceneTemplate.scene.web" => "网页",
        "sceneTemplate.generate.webPromptTemplate" => "为以下主题设计一个多区块的网页落地页：{{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "SaaS 落地页 · 橙色",
        "sceneTemplate.item.saasLandingOrange.summary" => "浅底黑卡配橙色主色的产品营销长页：导航、Hero 与产品截图、能力三卡、工作流演示、客户评价和订阅页脚，换掉文案就是一版官网。",
        "sceneTemplate.item.productLandingLight.title" => "产品落地页 · 浅色",
        "sceneTemplate.item.productLandingLight.summary" => "纸白报刊风的产品长页：Hero 交互演示卡、能力分栏、数据看板、新旧方案对比和三档定价，适合 SaaS 官网与产品发布。",
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
        "sceneTemplate.item.knowledgeCardVertical.title" => "知识卡片 · 竖版",
        "sceneTemplate.item.knowledgeCardVertical.summary" => {
            "3:4 单张图文卡，标题、四条要点和署名条，换掉文案就能发小红书。"
        }
        "sceneTemplate.item.knowledgeCardSquare.title" => "知识卡片 · 方版",
        "sceneTemplate.item.knowledgeCardSquare.summary" => {
            "1:1 方形卡，同一套版式的紧凑版，适合公众号头图与朋友圈。"
        }
        "sceneTemplate.item.pitchDeckDark.title" => "路演 deck · 深色",
        "sceneTemplate.item.pitchDeckDark.summary" => {
            "封面、问题、方案、数据、里程碑和联系页，深底大字，适合融资路演与产品发布。"
        }
        "sceneTemplate.item.lectureDeckLight.title" => "课件 deck · 浅色",
        "sceneTemplate.item.lectureDeckLight.summary" => {
            "课程封面、学习目标、概念讲解、例题、对比表和小结作业，纸白底耐看，适合上课投影。"
        }
        "sceneTemplate.item.minimalKeynote.title" => "极简 Keynote",
        "sceneTemplate.item.minimalKeynote.summary" => {
            "纯白留白、超大字号、一页一句话居中，九页里没有一张卡片，目录只有细线和数字，适合发布会与主题演讲。"
        }
        "sceneTemplate.item.gradientTech.title" => "渐变科技风",
        "sceneTemplate.item.gradientTech.summary" => {
            "深色渐变底加玻璃拟态卡，含架构、性能对比与客户墙，适合开发者产品发布。"
        }
        "sceneTemplate.scene.infographic" => "信息图",
        "sceneTemplate.item.punchQuoteCard.title" => "金句卡 · 大字报",
        "sceneTemplate.item.punchQuoteCard.summary" => {
            "3:4 墨底金句卡，两行巨标题加一条亮黄标语，只讲一句话，适合观点与语录。"
        }
        "sceneTemplate.item.journalChecklistCard.title" => "清单卡 · 知识库风",
        "sceneTemplate.item.journalChecklistCard.summary" => {
            "4:5 浅灰底上一张白色清单卡，五条可勾的待办、标签与引用块，适合周计划与打卡。"
        }
        "sceneTemplate.item.dataReportInfographic.title" => "数据结论长图",
        "sceneTemplate.item.dataReportInfographic.summary" => {
            "竖版信息长图：深色页头、三个大数、横向对比条、构成占比和三条结论，换掉数字就能发。"
        }
        "sceneTemplate.item.stepsFlowInfographic.title" => "流程步骤长图",
        "sceneTemplate.item.stepsFlowInfographic.summary" => {
            "竖版信息长图：五个带序号的步骤卡串成一条流程，配时长标签与两句提示，适合教程与攻略。"
        }
        "sceneTemplate.item.eventPosterDeck.title" => "活动策划 deck · 公告海报",
        "sceneTemplate.item.eventPosterDeck.summary" => "封面、亮点、日程、场地交通、票种和结尾，近白展墙底配红蓝色块，零圆角零渐变，适合市集、社团活动与开业招商。",
        "sceneTemplate.item.pitfallListInfographic.title" => "避坑清单长图",
        "sceneTemplate.item.pitfallListInfographic.summary" => "竖版信息长图：六条按频率排序的避坑项，每条给「错在哪」和「改成这样」，末尾附四行自检表，全篇无彩色。",
        "sceneTemplate.item.spineCultureCard.title" => "竖排书脊卡 · 鸣沙矿彩",
        "sceneTemplate.item.spineCultureCard.summary" => {
            "3:4 赭泥暗底上的竖排大标，配剥落壁面与矿彩颗粒，适合文化、长文与个人 IP 封面。"
        }
        "sceneTemplate.item.metricSingleCard.title" => "数据单值卡 · 网格汉字",
        "sceneTemplate.item.metricSingleCard.summary" => {
            "1:1 纯白底上一个巨大的数，瑞士国际主义的严格网格加一枚信号红方块，适合结论与成绩。"
        }
        "sceneTemplate.item.quoteFrameCard.title" => "引用书摘卡 · 绢本青绿",
        "sceneTemplate.item.quoteFrameCard.summary" => {
            "4:5 绢黄底上一句框起来的话，底部是石青石绿的双色山形，适合书摘、访谈与引用。"
        }
        "sceneTemplate.item.dailySignCard.title" => "日签卡 · 园林框景",
        "sceneTemplate.item.dailySignCard.summary" => {
            "3:4 粉墙底上一扇六角漏窗，窗内是日期与一句话，留白即装饰，适合日签与品牌短语。"
        }
        "sceneTemplate.item.priceTierCard.title" => "促销价格卡 · 霓虹骑楼",
        "sceneTemplate.item.priceTierCard.summary" => {
            "1:1 墨蓝夜色底上的三档价目表，配霓虹灯管描边与外散射，适合门店、活动与套餐报价。"
        }
        "sceneTemplate.item.noticeBoardCard.title" => "公告通知卡 · 铅字报刊",
        "sceneTemplate.item.noticeBoardCard.summary" => {
            "4:5 新闻纸底上的报头双线与编号条款，含套印错位与骑缝编号，适合通知、须知与规则说明。"
        }
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "时间线大事记长图",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "竖版信息长图：一条贯穿全图的时间轴，年份刻度配大事记卡，末尾收在下一步，适合复盘、品牌史与项目历程。",
        "sceneTemplate.item.conceptContrastInfographic.title" => "概念对比科普长图",
        "sceneTemplate.item.conceptContrastInfographic.summary" => "竖版信息长图：先给结论，再给两个概念各自的定义卡，然后逐维度拆成两栏表，最后给选择判据。",
        "sceneTemplate.item.rankingBoardInfographic.title" => "榜单 TOP N 长图",
        "sceneTemplate.item.rankingBoardInfographic.summary" => {
            "竖版信息长图：墨底黑金的推荐榜，前三名大徽章、四到八名小描边，每条给使用场景与频次。"
        }
        "sceneTemplate.item.faqThreadInfographic.title" => "问答 FAQ 长图",
        "sceneTemplate.item.faqThreadInfographic.summary" => {
            "竖版信息长图：六组一问一答，Q 实心 A 描边，不编号不排序，读者只读其中一条也成立。"
        }
        "sceneTemplate.item.dataStoryInfographic.title" => "数据故事长图",
        "sceneTemplate.item.dataStoryInfographic.summary" => "竖版信息长图：四个数字串成一条因果线，每段用十格方块阵表示比例，末尾收到一句能改做法的结论。",
        "sceneTemplate.item.challengeTrackerInfographic.title" => "30 天打卡挑战长图",
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "竖版信息长图：六列五行的三十格打卡阵，只在第 7、15、30 天给里程碑，存进相册每天划掉一格。",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "行业地图长图",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => {
            "竖版信息长图：二乘二的四区生态位阵列，每格挂三个位点并标出空位，石板灰底上浮白卡。"
        }
        "sceneTemplate.item.doDontComparison.title" => "好坏示范双栏",
        "sceneTemplate.item.doDontComparison.summary" => "3:4 单卡：同一件事的两种做法左右并排，不靠红绿而靠材质与图标区分对错，色觉障碍读者也读得出来。",
        "sceneTemplate.item.mythTruthComparison.title" => "误区与真相长图",
        "sceneTemplate.item.mythTruthComparison.summary" => "竖版长图：五组「大家都这么说 / 其实是这样」交错排开，误区偏窄浅底、真相偏宽深底，一次只处理一组。",
        "sceneTemplate.item.pricingTiersComparison.title" => "价格档位对比",
        "sceneTemplate.item.pricingTiersComparison.summary" => "3:4 单卡：免费 / Pro / 团队三档并排，价格当锚点往下读，右列包含左列，适合定价页与套餐说明。",
        "sceneTemplate.item.scenarioGuideComparison.title" => "场景选择指南长图",
        "sceneTemplate.item.scenarioGuideComparison.summary" => {
            "竖版长图：不摆参数，直接给七种处境，每种后面挂一个判定标签，读者只要找到自己那一行。"
        }
        "sceneTemplate.item.specTableComparison.title" => "参数表对比长图",
        "sceneTemplate.item.specTableComparison.summary" => "竖版长图：两个候选放进一张真表逐行比，赢的一格用深底反白顶起来，一眼扫下来就知道各自赢在哪。",
        "sceneTemplate.item.threeWayComparison.title" => "三方案横评长图",
        "sceneTemplate.item.threeWayComparison.summary" => "竖版长图：三个方案并排，中间一列是推荐项，每列第一行不是名字而是一句处境——读者在找哪一列是自己。",
        "sceneTemplate.item.timeShiftComparison.title" => "时间对比 · 一年前与现在",
        "sceneTemplate.item.timeShiftComparison.summary" => {
            "3:4 单卡：一条居中的标签脊柱，左边一年前、右边现在，同一项的两个取值落在同一行上。"
        }
        "sceneTemplate.item.tradeoffScaleComparison.title" => "优缺点天平",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "1:1 方卡：一根横梁两个托盘，左盘装值得、右盘装代价，每条前面留一个空方框——结论交给读者自己称。",
        "sceneTemplate.item.versionDiffComparison.title" => "新旧版本变化",
        "sceneTemplate.item.versionDiffComparison.summary" => {
            "1:1 方卡：不分左右两栏，每一行自己完成一次「旧 → 新」，顺着往下滑即可读完全部改动。"
        }
        "sceneTemplate.item.appOnboardingTriptych.title" => "App 新手引导三屏",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "3:4 单卡：三台并排的手机与空图位，把自己的三张引导图拖进去配上文案，一张就能拿去评审或发布。",
        "sceneTemplate.item.diyBlueprintGuide.title" => "手工 DIY 图解长图",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "竖版长图：材料规格表与步骤各占一半篇幅——手工翻车多在准备而不在手上，所以先把材料写清楚。",
        "sceneTemplate.item.photoCompositionTutorial.title" => "手机摄影构图教学",
        "sceneTemplate.item.photoCompositionTutorial.summary" => {
            "3:4 五帧：每帧一个深色取景框，荧光参考线压在图位之上——构图必须画在取景框上才说得清。"
        }
        "sceneTemplate.item.recipeFourStep.title" => "菜谱四步卡",
        "sceneTemplate.item.recipeFourStep.summary" => {
            "4:5 单卡 2×2 四宫格：四步全放在一张卡上，截图存相册就能照着做，站在灶台前不用翻页。"
        }
        "sceneTemplate.item.skincareRoutineCards.title" => "护肤步骤卡",
        "sceneTemplate.item.skincareRoutineCards.summary" => {
            "4:5 六帧：每步固定给用量、停留时长与早晚场次三个数——护肤翻车多在用量和间隔上。"
        }
        "sceneTemplate.item.softwareStepTutorial.title" => "软件操作步骤卡",
        "sceneTemplate.item.softwareStepTutorial.summary" => {
            "4:5 单卡：教程档唯一一张深色，界面截图位配编号操作说明，适合工具与软件的功能讲解。"
        }
        "sceneTemplate.item.storageMakeoverSteps.title" => "家居收纳改造步骤",
        "sceneTemplate.item.storageMakeoverSteps.summary" => {
            "3:4 六帧：每步除了动作与图位，固定给一条完成判定和一个耗时预算——做到那个状态才算做完。"
        }
        "sceneTemplate.item.weeklyReportLesson.title" => "职场周报小课长图",
        "sceneTemplate.item.weeklyReportLesson.summary" => {
            "竖版长图：讲完四段结构之后直接给一张带下划线空格的周报骨架，截图就能照着往里填。"
        }
        "sceneTemplate.item.workoutBreakdownGuide.title" => "健身动作分解长图",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "竖版长图：每个动作除图位与要点外，还有一条固定格式的组数 / 次数 / 休息参数条，存图照着数做。",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => "书影评拆解轮播",
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4 五板：钩子、带注解的原文、三个洞见、一句书摘、收束——把一部作品拆成能拿走的零件，不是复述剧情。",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "城市指南轮播",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => {
            "3:4 七板：照片与动线交替——地点板给做梦的读者，一日动线与吃住对照给做计划的读者。"
        }
        "sceneTemplate.item.datareportGridCarousel.title" => "数据报告轮播",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4 六板：数据页之间强制夹入非数据页，避免读者划到第三张图表就跳过，适合季报与行业观察。",
        "sceneTemplate.item.opinionLongformCarousel.title" => "观点长文轮播",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4 六板：一套严格的视觉母版贯穿全程，页码与标题永远在同一个位置——轮播划走就回不去，一致性是刚需。",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "问答体轮播",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => "3:4 六板：一问一板，每板左上角一个手写问号编号——问题本身就是往下划的理由，不需要留悬念。",
        "sceneTemplate.item.storyNightCarousel.title" => "故事叙事轮播",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4 七板：以时间为骨架的个人经历复盘，第五板那条时间轴是全套的承重墙，前四板都是它的一个刻度。",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => "干货合集轮播",
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4 六板：六个工具逐板展开，最后一板连页码一起列成目录——合集档的读者目的只有一个，收藏。",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "教程轮播",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => {
            "3:4 六板：一板一步，手指就是进度条，划一次等于做完一步，适合手作、软件与生活教程。"
        }
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "年度总结复盘轮播",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => {
            "3:4 八板：数字页冷、感受页热，两种温度交替推进，适合年终总结与个人年度复盘。"
        }
        "fileMenu.newFromTemplate" => "从模板新建",
        "fileMenu.exportSlideshowHtml" => "导出放映 HTML...",
        "fileMenu.exportPptx" => "导出 PowerPoint...",
        "dialog.slideshowHtmlTitle" => "导出放映",
        "dialog.slideshowHtmlSummary" => "已导出 {{count}} 张幻灯片到：",
        "dialog.slideshowHtmlEmpty" => "当前演示文稿没有可导出的幻灯片。",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "可导入的 HTML 内容不可用。",
        "htmlImport.warn.content.empty_body" => "HTML 正文中可导入的内容不可用。",
        "htmlImport.warn.content.dom_depth_truncated" => {
            "嵌套层级超过 {{max_depth}} 层的 HTML 已丢弃。"
        }
        "htmlImport.warn.content.node_limit_truncated" => "已达节点数上限，页面剩余内容已略去。",
        "htmlImport.warn.content.node_limit_mapping" => "已达节点数上限，部分 HTML 树已略去。",
        "htmlImport.warn.content.node_limit_inline_row" => "已达节点数上限，某个内联排版行已略去。",
        "htmlImport.warn.content.node_limit_pseudo" => "已达节点数上限，生成的伪元素已略去。",
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "嵌套超过 {{max_depth}} 层 @ 规则的 CSS 规则已忽略。"
        }
        "htmlImport.warn.css.unterminated_rule" => "未闭合的 CSS 规则已忽略。",
        "htmlImport.warn.css.marker_rules_unsupported" => "CSS ::marker 规则未导入。",
        "htmlImport.warn.css.nesting_unsupported" => "嵌套的 CSS 样式规则已忽略。",
        "htmlImport.warn.css.invalid_layer_name" => "无效的 @layer 名称 '{{name}}' 已忽略。",
        "htmlImport.warn.css.unsupported_statement" => "不支持的 @{{name}} 语句已忽略。",
        "htmlImport.warn.css.media_without_viewport" => "没有视口的 @media 规则已忽略。",
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "无效的 @layer 块名称 '{{name}}' 已忽略。"
        }
        "htmlImport.warn.css.unsupported_container_block" => "@container 块已忽略。",
        "htmlImport.warn.css.unsupported_block" => "不支持的 @{{name}} 块已忽略。",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "@font-face 网络字体 '{{family}}' 不可用。"
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "绝对定位元素的百分比偏移已近似处理。"
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "百分比的 position:relative 偏移已近似处理。"
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "没有确定轴向的 CSS aspect-ratio 已忽略。"
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "位于不确定包含块内的 CSS aspect-ratio 已忽略。"
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky 已忽略。",
        "htmlImport.warn.layout.grid_tracks_approximated" => "不支持的 CSS 网格轨道已近似处理。",
        "htmlImport.warn.layout.float_ignored" => "CSS float 已忽略。",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "节点级的 CSS mix-blend-mode 已近似处理。"
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll 已近似处理。"
        }
        "htmlImport.warn.layout.negative_margins_ignored" => "负的 CSS 外边距已忽略。",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => "视觉盒上的 CSS 外边距已忽略。",
        "htmlImport.warn.layout.inline_margin_wrapping_approximated" => "带 CSS 外边距的行内元素已装入独立盒，可能无法再跨行换行。",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box 的百分比尺寸已近似处理。"
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "显式起始线留下的空 CSS 网格单元已近似处理。"
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "跨度与起始线不匹配的 CSS 网格项已近似处理。"
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "已达节点数上限，CSS 网格行的包装容器已略去。"
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "使用 auto-fit / auto-fill 的 CSS 网格轨道宽度已近似处理。"
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas 定位未导入。"
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => "CSS grid-row 定位未导入。",
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` 已近似处理。"
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => "块轴方向的 CSS 自动外边距未导入。",
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "已达节点数上限，CSS 自动外边距对齐已略去。"
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "尺寸不确定的元素上的 CSS 流内偏移已丢弃。"
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "已达节点数上限，某个 CSS 流内偏移已略去。"
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "CSS 流内偏移（position:relative 内缩、transform 平移）已近似处理。"
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "某个盒无法承载偏移包装容器，其 CSS 流内偏移已丢弃。"
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "列向弹性容器上的 flex-wrap 未导入。"
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "flex-wrap:wrap-reverse 已近似处理。",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "宽度不确定的容器上的 flex-wrap 已忽略。"
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "换行弹性容器上的 CSS align-content 未导入。"
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "子项主轴尺寸不确定时的 flex-wrap 已忽略。"
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "已达节点数上限，flex-wrap 的换行行已略去。"
        }
        "htmlImport.warn.transform.unsupported_syntax" => "不支持的 CSS transform 语法已忽略。",
        "htmlImport.warn.transform.unsupported_function" => {
            "不支持的 CSS transform 函数（3D、matrix3d）已忽略。"
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "不确定轴向上的百分比 CSS transform 平移已丢弃。"
        }
        "htmlImport.warn.transform.non_finite_matrix" => "产生非有限矩阵的 CSS transform 已忽略。",
        "htmlImport.warn.transform.skew_dropped" => "CSS transform 斜切已丢弃。",
        "htmlImport.warn.transform.degenerate_scale" => {
            "缩放为零或非有限的 CSS transform 已近似处理。"
        }
        "htmlImport.warn.transform.mirroring_absolute" => "CSS transform 镜像已近似处理。",
        "htmlImport.warn.transform.origin_z_ignored" => "CSS transform-origin 的 Z 轴偏移已忽略。",
        "htmlImport.warn.transform.scale_not_baked" => {
            "无法烘焙进节点尺寸的 CSS transform 缩放已丢弃。"
        }
        "htmlImport.warn.transform.scale_baked" => {
            "烘焙进节点尺寸的 CSS transform 缩放已近似处理。"
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "自动尺寸元素上的 CSS transform 缩放已忽略。"
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "带方向或带间隔的 CSS background-repeat 已近似处理。"
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "显式指定的 CSS 背景平铺尺寸已忽略。"
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "自动尺寸元素上的 CSS background-size 已近似处理。"
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "需要图片固有尺寸的 CSS background-size 已近似处理。"
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "不支持的 CSS background-position 已忽略。"
        }
        "htmlImport.warn.visual.background_image_url_empty" => "空的 CSS 背景图片 URL 已忽略。",
        "htmlImport.warn.visual.conic_gradient_ignored" => "CSS 锥形渐变已忽略。",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "不支持的 CSS background-image 图层已忽略。"
        }
        "htmlImport.warn.visual.background_color_unresolved" => "无法解析的 CSS 背景色已忽略。",
        "htmlImport.warn.visual.background_position_dropped" => "CSS background-position 已忽略。",
        "htmlImport.warn.visual.border_colors_approximated" => {
            "分边设置的 CSS 边框颜色已近似处理。"
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "各边不一致的 CSS 边框样式已近似处理。"
        }
        "htmlImport.warn.visual.border_style_complex" => "复杂的 CSS 边框样式已近似处理。",
        "htmlImport.warn.visual.border_style_unsupported" => "不支持的 CSS 边框样式已近似处理。",
        "htmlImport.warn.visual.border_radius_elliptical" => "椭圆形的 CSS 边框圆角已近似处理。",
        "htmlImport.warn.visual.border_radius_unsupported" => "不支持的 CSS 边框圆角已忽略。",
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "不支持的 CSS box-shadow 图层已忽略。"
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => "CSS 渐变的颜色插值方式已忽略。",
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "不支持的 CSS linear-gradient 方向已忽略。"
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => "CSS 渐变的颜色提示点已忽略。",
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => "不支持的 CSS 渐变色标已忽略。",
        "htmlImport.warn.visual.gradient_too_few_stops" => "可用色标少于两个的 CSS 渐变已忽略。",
        "htmlImport.warn.visual.gradient_repeating_approximated" => "重复的 CSS 渐变已近似处理。",
        "htmlImport.warn.visual.gradient_stops_clamped" => "超出范围的 CSS 渐变色标已近似处理。",
        "htmlImport.warn.visual.blur_radius_unsupported" => "不支持的 CSS 模糊半径已忽略。",
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "不支持的 CSS filter drop-shadow() 已忽略。"
        }
        "htmlImport.warn.visual.filter_function_unsupported" => "不支持的 CSS filter 函数已忽略。",
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "不支持的 CSS backdrop-filter 函数已忽略。"
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "不支持的 CSS background-blend-mode 已忽略。"
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "单个填充上的 CSS mix-blend-mode 已近似处理。"
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "不支持的 CSS mix-blend-mode 已忽略。"
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} 已忽略。",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "渐变上的 CSS background-size 已忽略。"
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "不支持的 CSS radial-gradient 位置已忽略。"
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "椭圆形的 CSS radial-gradient 已近似处理。"
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "CSS radial-gradient 的范围关键字已近似处理。"
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "不支持的 CSS radial-gradient 尺寸已忽略。"
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => "不支持的 CSS text-shadow 图层已忽略。",
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "第一层之后的 CSS text-shadow 图层已忽略。"
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => "内联元素上的 CSS text-shadow 已忽略。",
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image 未导入。",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "`list-style-position: outside` 的悬挂标记已近似处理。"
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "不支持的 CSS list-style-type `{{value}}` 已近似处理。"
        }
        "htmlImport.warn.media.object_fit_scale_down" => "CSS object-fit:scale-down 已近似处理。",
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none 已忽略。",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position 已忽略。",
        "htmlImport.warn.media.image_intrinsic_axis_unresolved" => {
            "由于设定的尺寸为动态值或包含块尺寸不确定，无法使用图片的固有宽高比补全缺失的尺寸轴。"
        }
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "图片上不支持的 CSS mix-blend-mode 已忽略。"
        }
        "htmlImport.warn.media.inline_svg_placeholder" => "内联的 <svg> 元素已作为占位符导入。",
        "htmlImport.warn.media.input_type_fallback" => "不支持的 <input> 类型已近似处理。",
        "htmlImport.warn.media.element_placeholder" => "<{{tag}}> 元素已作为占位符导入。",
        "htmlImport.warn.media.picture_undecodable_types" => {
            "仅含无法解码源类型的 <picture> 已近似处理。"
        }
        "htmlImport.warn.table.rowspan_ignored" => "HTML rowspan 属性未导入。",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "行组未被 CSS 扁平化的表格，其列宽已近似处理。"
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "宽度不确定的 CSS 表格，其列宽已近似处理。"
        }
        "htmlImport.warn.resource.invalid_base_href" => "无效的 <base href> {{href}} 已忽略。",
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "项目源之外的 <base href> {{href}} 已忽略。"
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => "外部样式表 {{url}} 不可用。",
        "htmlImport.warn.resource.image_outside_origin" => {
            "项目源之外的图片 {{url}} 已作为占位符导入。"
        }
        "htmlImport.warn.resource.image_unavailable" => "不可用的图片 {{url}} 已作为占位符导入。",
        "htmlImport.warn.resource.css_import_invalid" => "无效的 CSS @import {{prelude}} 已忽略。",
        "htmlImport.warn.resource.css_import_unresolvable" => "CSS @import {{reference}} 不可用。",
        "htmlImport.warn.resource.css_import_cycle" => "循环引用的 CSS @import {{url}} 已忽略。",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "超出 {{max_depth}} 层深度的 CSS @import {{url}} 已忽略。"
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} 不可用。",
        "htmlImport.warn.project.multiple_html_entries" => {
            "发现 {{count}} 个 HTML 入口，已选用 {{entry}}，其余已近似处理。"
        }
        "htmlImport.warn.snapshot.truncated" => "部分浏览器快照已丢弃。",
        "htmlImport.warn.snapshot.node_limit" => "已达节点数上限，快照剩余内容已略去。",
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} 张受 CORS 污染的图片以远程 URL 保留，不可用。"
        }
        "htmlImport.warn.snapshot.invalid_rect" => "矩形缺失或无效的快照节点已丢弃。",
        "htmlImport.warn.snapshot.unknown_kind" => "类型未知的快照节点已丢弃。",
        "htmlImport.warn.snapshot.rejected" => "浏览器快照（{{reason}}）已丢弃。",
        "htmlImport.warn.snapshot.unsupported_transform" => "不支持的快照变换已忽略。",
        "htmlImport.warn.css.media_empty_query" => "空的 @media 查询已忽略。",
        "htmlImport.warn.css.media_unsupported_type" => "不支持的 @media 类型 '{{name}}' 已忽略。",
        "htmlImport.warn.css.media_unsupported_condition" => {
            "不支持的 @media 条件 '{{input}}' 已忽略。"
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "无效的 @media 方向 '{{value}}' 已忽略。"
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "不支持的 @media 特性 '{{name}}' 已忽略。"
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "不支持的 @media 范围 '({{input}})' 已忽略。"
        }
        "htmlImport.warn.css.media_invalid_range" => "无效的 @media 范围 '({{input}})' 已忽略。",
        "htmlImport.warn.css.media_invalid_length" => "无效的 @media 长度 '{{value}}' 已忽略。",
        "htmlImport.diagnostics.title" => "HTML 导入完成",
        "htmlImport.diagnostics.summary" => "降级项：{{count}}",
        "htmlImport.diagnostics.dismiss" => "关闭",
        "htmlImport.diagnostics.expand" => "显示详情",
        "htmlImport.diagnostics.collapse" => "隐藏详情",
        "htmlImport.diagnostics.more" => "+{{count}} 项",
        "dialog.pptxTitle" => "导出 PowerPoint",
        "dialog.pptxSummary" => "已导出 {{count}} 张幻灯片到：",
        "dialog.pptxEmpty" => "当前演示文稿没有可导出的幻灯片。",
        "settings.agents.acpQuickAdd" => "快速添加",
        "settings.agents.acpPresetAdd" => "添加",
        "settings.agents.acpNotInstalled" => "未安装",
        "assetCenter.title" => "资产中心",
        "assetCenter.tab.templates" => "模板",
        "assetCenter.tab.styles" => "风格",
        "assetCenter.style.empty" => "没有匹配的风格",
        "assetCenter.style.pinned" => "已钉住",
        "assetCenter.style.searchPlaceholder" => "搜索风格或标签",
        "assetCenter.style.generateHint" => "新建一个文档，按主题生成；已钉住的风格会被直接采用。",
        "ai.pinnedStyle" => "风格：{{name}}",
        "assetCenter.style.import" => "导入风格",
        "assetCenter.style.mine" => "我的风格",
        "assetCenter.style.builtIn" => "内置风格",
        "assetCenter.style.importTitle" => "导入 DESIGN.md",
        "assetCenter.style.importHint" => "粘贴 DESIGN.md 全文，然后确认导入。",
        "assetCenter.style.importSource" => "可以从 styles.refero.design 等 DESIGN.md 风格库复制内容。",
        "assetCenter.style.importConfirm" => "导入",
        "assetCenter.style.importCancel" => "取消",
        "assetCenter.style.importPickFile" => "选择文件…",
        "assetCenter.style.importHintFile" => "选择 DESIGN.md 文件，或在下方粘贴全文。",
        "assetCenter.style.importPlaceholder" => "在此粘贴 DESIGN.md",
        "assetCenter.style.importEmpty" => "这个文件是空的，或者内容太短，不像一份风格指南。",
        "assetCenter.style.importNotText" => "这个文件不是 Markdown 文本。",
        "assetCenter.style.importTooLarge" => "这个文件超过 512 KB。",
        "slidesPanel.tabSlides" => "幻灯片",
        "slidesPanel.tabCards" => "卡片",
        "slidesPanel.present" => "放映",
        "slidesPanel.exportPdf" => "导出 PDF",
        "slidesPanel.exportAllSlides" => "导出全部幻灯片",
        "slidesPanel.exportSelectedSlides" => "导出所选幻灯片（{{count}}）",
        "settings.tab.ai" => "AI",
        "settings.agents.heroTitle" => "连接你的 AI 服务商",
        "settings.agents.heroSubtitle" => {
            "OpenPencil 直接驱动本地 CLI Agent 与 API 服务商，连接任意一个即可开始生成设计。"
        }
        "settings.agents.statusConnected" => "已连接",
        "settings.agents.statusNotConnected" => "未连接",
        "settings.agents.statusChecking" => "正在检测…",
        "settings.mcp.heroTitle" => "从外部通过 MCP 连接 OpenPencil",
        "settings.mcp.heroSubtitle" => {
            "把任意支持 MCP 的 CLI 或编辑器指向这个工作区，即可用内置 Agent 同款工具驱动画布。"
        }
        "settings.mcp.terminalFootnote" => "* 启动时会自动为选中的 CLI 工具配置 MCP。",
        "settings.mcp.customConfigTitle" => "自定义 MCP 服务器配置",
        "settings.mcp.customConfigDesc" => "粘贴到任何读取标准 MCP server 配置块的客户端即可。",
        "settings.mcp.copyConfig" => "复制 MCP 配置",
        "settings.system.heroTitle" => "系统偏好",
        "settings.system.heroSubtitle" => "本机安装的外观、更新与画布行为。",
        "settings.system.appearance" => "外观",
        "settings.system.appearanceLight" => "浅色",
        "settings.system.appearanceDark" => "深色",
        "settings.system.pencilCursor" => "画笔光标",
        "settings.images.heroTitle" => "为设计配图",
        "settings.images.heroSubtitle" => "在 Openverse 搜索图片，或接入服务商按需生成。",
        "settings.fonts.heroTitle" => "本文档的字体",
        "settings.fonts.heroSubtitle" => "补齐文档需要但本机缺失的字体，并管理你导入的字体。",
        "settings.account.heroTitle" => "你的账户",
        "settings.account.heroSubtitle" => "登录后可在多设备间同步工作区与授权。",
        "tooltip.topbar.file" => "文件",
        "tooltip.topbar.import" => "导入",
        "tooltip.topbar.language" => "语言",
        "tooltip.topbar.collaboration" => "协作",
        "tooltip.topbar.preview" => "预览",
        "tooltip.topbar.exitPreview" => "退出预览",
        "tooltip.topbar.account" => "账户",
        "settings.agents.providerRollMore" => "等 {{count}} 家",
        "ai.thinking.adaptive" => "思考：自动",
        "ai.thinking.disabled" => "思考：关闭",
        "ai.thinking.enabled" => "思考：开启",
        "ai.designProgress.detail.repairsApplied" => "已应用 {{count}} 处自动修复",
        "ai.designProgress.detail.repairsMore" => "…还有 {{count}} 条(见日志)",
        "ai.styleCard.builtin" => "内置风格",
        "ai.styleCard.imported" => "导入的 DESIGN.md",
        "ai.styleCard.documentDesignMd" => "文档 design.md",
        _ => return super::zh_cn_collab::lookup(key),
    })
}

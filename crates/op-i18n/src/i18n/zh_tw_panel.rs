//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `zh_tw_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "搜尋圖片…",
        "imagePanel.searching" => "搜尋中…",
        "imagePanel.noResults" => "未找到結果",
        "imagePanel.searchPrompt" => "搜尋圖片",
        "imagePanel.sourceNotice" => "圖片來自 {{source}}。自由授權 — 使用前請確認授權條款。",
        "imagePanel.genNotConfigured" => "圖片生成尚未設定",
        "imagePanel.openSettings" => "開啟設定",
        "imagePanel.promptPlaceholder" => "描述要生成的圖片…",
        "providerProbe.connectedViaCli" => "已透過 {{name}} CLI 連線",
        "providerProbe.cliExitedWithError" => "{{name}} CLI 結束並回報錯誤",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI 未輸出版本資訊",
        "providerProbe.modelQueryFailed" => "{{name}} 模型查詢失敗或逾時",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} 模型查詢失敗。請先執行 {{command}} 完成驗證。"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} 模型查詢需要驗證。請先執行 {{command}} 登入。"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} 傳回無法辨識的模型清單",
        "promptCenter.title" => "提示詞中心",
        "promptCenter.searchPlaceholder" => "搜尋提示詞…",
        "promptCenter.category.all" => "全部",
        "promptCenter.category.starter" => "快速上手",
        "promptCenter.category.mobileApp" => "行動 App",
        "promptCenter.category.webPage" => "網頁",
        "promptCenter.category.dashboard" => "儀表板",
        "promptCenter.category.component" => "元件",
        "promptCenter.category.modify" => "改稿",
        "promptCenter.category.custom" => "我的",
        "promptCenter.empty" => "沒有符合的提示詞",
        "promptCenter.saveCurrent" => "儲存目前輸入",
        "promptCenter.saveTitlePlaceholder" => "提示詞標題",
        "promptCenter.save" => "儲存",
        "promptCenter.cancel" => "取消",
        "promptCenter.delete" => "刪除",
        "promptCenter.screens" => "{{count}} 個畫面",
        "promptCenter.freeform" => "自由發揮",
        "promptCenter.item.wander.title" => "Wander · 旅行行程規劃",
        "promptCenter.item.forage.title" => "Forage · 時令食譜",
        "promptCenter.item.still.title" => "Still · 冥想與睡前",
        "promptCenter.item.hearth.title" => "Hearth · 智慧家庭",
        "promptCenter.item.meteo.title" => "Meteo · 沉浸式天氣",
        "promptCenter.item.marginalia.title" => "Marginalia · 閱讀與註記",
        "promptCenter.item.lingua.title" => "Lingua · 語言學習",
        "promptCenter.item.daybreak.title" => "Daybreak · 咖啡預訂",
        "promptCenter.item.verdant.title" => "Verdant · 植物照護",
        "promptCenter.item.companion.title" => "Companion · 寵物生活",
        "promptCenter.item.relic.title" => "Relic · 精品二手市集",
        "promptCenter.item.nocturne.title" => "Nocturne · 觀星指南",
        "promptCenter.item.marquee.title" => "Marquee · 觀影清單",
        "promptCenter.item.ritual.title" => "Ritual · 習慣養成",
        "promptCenter.item.ember.title" => "Ember · 心情日記",
        "promptCenter.item.volt.title" => "Volt · 電動車夥伴",
        "promptCenter.item.aloft.title" => "Aloft · 航班追蹤",
        "promptCenter.item.gallery.title" => "Gallery · 展覽與文化活動",
        "promptCenter.item.nightcap.title" => "Nightcap · 家庭調酒",
        "promptCenter.item.bloom.title" => "Bloom · 親子成長記錄",
        "promptCenter.item.extremeWeather.title" => "極限 · 天氣 App",
        "promptCenter.item.extremeNowPlaying.title" => "極限 · 正在播放",
        "promptCenter.item.extremeDailyApp.title" => "極限 · 每日必開 App",
        "promptCenter.item.extremeCalendar.title" => "極限 · 行事曆",
        "promptCenter.item.extremeCalm.title" => "極限 · 寧靜",
        "promptCenter.item.webOrbit.title" => "Orbit · AI 工作台官網",
        "promptCenter.item.webAtelier.title" => "Atelier · 家居品牌電商",
        "promptCenter.item.webKilnform.title" => "Kilnform · 設計基建官網",
        "promptCenter.item.webReefwright.title" => "Reefwright · AI 客服知識官網",
        "promptCenter.item.dashboardPulse.title" => "Pulse · 成長分析台",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · 物流維運中心",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · 企業資料表",
        "promptCenter.item.componentFormLab.title" => "Form Lab · 表單元件系統",
        "promptCenter.item.modifyPolishCurrent.title" => "精修目前介面",
        "promptCenter.item.modifyCompleteStates.title" => "補齊元件狀態",
        "collab.ownerConfirm.title" => "確認你要加入誰的工作階段",
        "collab.ownerConfirm.hint" => "此工作階段的任何內容都尚未載入。",
        "collab.ownerConfirm.account" => "已驗證帳戶",
        "collab.ownerConfirm.device" => "已驗證裝置",
        "collab.ownerConfirm.claimedName" => "該帳戶自選的名稱（未經驗證）",
        "collab.action.confirmOwner" => "加入此工作階段",
        "collab.action.rejectOwner" => "不加入",
        "collab.error.ownerNotConfirmed" => "你未確認主持人，因此未載入任何內容。",
        "sceneTemplate.title" => "場景範本",
        "sceneTemplate.searchPlaceholder" => "搜尋場景或範本",
        "sceneTemplate.empty" => "沒有符合的範本",
        "sceneTemplate.frames" => "{{count}} 頁",
        "sceneTemplate.generate.placeholder" => "描述主題，AI 直接產生整份簡報",
        "sceneTemplate.generate.button" => "產生",
        "sceneTemplate.generate.hint" => "新增一個文件，依主題直接產生整份簡報。",
        "sceneTemplate.generate.promptTemplate" => "為以下主題製作一份簡報（PPT）：{{topic}}",
        "sceneTemplate.card.addToCanvas" => "加入畫布",
        "sceneTemplate.card.generateFrom" => "以此生成",
        "sceneTemplate.generate.basis" => "基於：",
        "sceneTemplate.filter.all" => "全部",
        "sceneTemplate.scene.tutorial" => "教學圖",
        "sceneTemplate.scene.comparison" => "對比圖",
        "sceneTemplate.scene.carousel" => "輪播",
        "sceneTemplate.scene.slides" => "簡報",
        "sceneTemplate.scene.card" => "卡片",
        "sceneTemplate.scene.web" => "網頁",
        "sceneTemplate.generate.webPromptTemplate" => "為以下主題設計一個多區塊的網頁著陸頁：{{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "SaaS 著陸頁 · 橘色",
        "sceneTemplate.item.saasLandingOrange.summary" => "淺底黑卡配橘色主色的產品行銷長頁：導覽列、Hero 與產品截圖、能力三卡、工作流程示範、客戶評價與訂閱頁尾，換掉文案就是一版官網。",
        "sceneTemplate.item.productLandingLight.title" => "產品著陸頁 · 淺色",
        "sceneTemplate.item.productLandingLight.summary" => "紙白報刊風的產品長頁：Hero 互動示範卡、能力分欄、資料看板、新舊方案對比與三檔定價，適合 SaaS 官網與產品發表。",
        "sceneTemplate.item.screenshotTutorial.title" => "三步截圖教學卡",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "封面、三個操作步驟和結尾行動呼籲，替換截圖與說明即可發布。"
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "知識觀點輪播",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "封面、三個論點和總結頁，適合將一個觀點拆成可滑動的連續卡片。"
        }
        "sceneTemplate.item.beforeAfter.title" => "改版前後對比",
        "sceneTemplate.item.beforeAfter.summary" => {
            "左右並置的前後對比，搭配改動說明，適合回顧與作品展示。"
        }
        "sceneTemplate.item.slideDeck.title" => "簡報 · 六頁",
        "sceneTemplate.item.slideDeck.summary" => {
            "封面、目錄、要點、資料、圖表和結尾，16:9 投影比例，替換文案即可上台。"
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "知識卡片 · 直式",
        "sceneTemplate.item.knowledgeCardVertical.summary" => {
            "3:4 單張圖文卡，標題、四條重點和署名列，換掉文案就能發布。"
        }
        "sceneTemplate.item.knowledgeCardSquare.title" => "知識卡片 · 方形",
        "sceneTemplate.item.knowledgeCardSquare.summary" => {
            "1:1 方形卡，同一套版式的精簡版，適合貼文封面與社群分享。"
        }
        "sceneTemplate.item.pitchDeckDark.title" => "路演 deck · 深色",
        "sceneTemplate.item.pitchDeckDark.summary" => {
            "封面、問題、方案、數據、里程碑與聯絡頁，深底大字，適合募資路演與產品發表。"
        }
        "sceneTemplate.item.lectureDeckLight.title" => "課件 deck · 淺色",
        "sceneTemplate.item.lectureDeckLight.summary" => {
            "課程封面、學習目標、概念講解、例題、對照表與總結作業，紙白底耐看，適合課堂投影。"
        }
        "sceneTemplate.item.minimalKeynote.title" => "極簡 Keynote",
        "sceneTemplate.item.minimalKeynote.summary" => {
            "純白留白、超大字級、一頁一句話置中，九頁裡沒有一張卡片，目錄只有細線和數字，適合發表會與主題演講。"
        }
        "sceneTemplate.item.gradientTech.title" => "漸層科技風",
        "sceneTemplate.item.gradientTech.summary" => {
            "深色漸層底加毛玻璃卡，含架構、效能對比與客戶牆，適合開發者產品發表。"
        }
        "sceneTemplate.scene.infographic" => "資訊圖",
        "sceneTemplate.item.punchQuoteCard.title" => "金句卡 · 大字報",
        "sceneTemplate.item.punchQuoteCard.summary" => {
            "3:4 墨底金句卡，兩行巨標題加一條亮黃標語，只講一句話，適合觀點與語錄。"
        }
        "sceneTemplate.item.journalChecklistCard.title" => "清單卡 · 知識庫風",
        "sceneTemplate.item.journalChecklistCard.summary" => {
            "4:5 淺灰底上一張白色清單卡，五條可勾的待辦、標籤與引用區塊，適合週計畫與打卡。"
        }
        "sceneTemplate.item.dataReportInfographic.title" => "數據結論長圖",
        "sceneTemplate.item.dataReportInfographic.summary" => {
            "直式資訊長圖：深色頁首、三個大數字、橫向比較條、組成占比和三條結論，換掉數字就能發。"
        }
        "sceneTemplate.item.stepsFlowInfographic.title" => "流程步驟長圖",
        "sceneTemplate.item.stepsFlowInfographic.summary" => {
            "直式資訊長圖：五張帶編號的步驟卡串成一條流程，附時長標籤與兩句提示，適合教學與攻略。"
        }
        "sceneTemplate.item.eventPosterDeck.title" => "活動企劃 deck · 公告海報",
        "sceneTemplate.item.eventPosterDeck.summary" => "封面、亮點、議程、場地交通、票種和結尾，近白展牆底配紅藍色塊，零圓角零漸層，適合市集、社團活動與開幕招商。",
        "sceneTemplate.item.pitfallListInfographic.title" => "避坑清單長圖",
        "sceneTemplate.item.pitfallListInfographic.summary" => "直式資訊長圖：六條按頻率排序的避坑項，每條給「錯在哪」和「改成這樣」，末尾附四行自檢表，全篇無彩色。",
        "sceneTemplate.item.spineCultureCard.title" => "直排書脊卡 · 鳴沙礦彩",
        "sceneTemplate.item.spineCultureCard.summary" => {
            "3:4 赭泥暗底上的直排大標，配剝落壁面與礦彩顆粒，適合文化、長文與個人 IP 封面。"
        }
        "sceneTemplate.item.metricSingleCard.title" => "數據單值卡 · 網格漢字",
        "sceneTemplate.item.metricSingleCard.summary" => {
            "1:1 純白底上一個巨大的數字，瑞士國際主義的嚴格網格加一枚信號紅方塊，適合結論與成績。"
        }
        "sceneTemplate.item.quoteFrameCard.title" => "引用書摘卡 · 絹本青綠",
        "sceneTemplate.item.quoteFrameCard.summary" => {
            "4:5 絹黃底上一句框起來的話，底部是石青石綠的雙色山形，適合書摘、訪談與引用。"
        }
        "sceneTemplate.item.dailySignCard.title" => "日籤卡 · 園林框景",
        "sceneTemplate.item.dailySignCard.summary" => {
            "3:4 粉牆底上一扇六角漏窗，窗內是日期與一句話，留白即裝飾，適合日籤與品牌短語。"
        }
        "sceneTemplate.item.priceTierCard.title" => "促銷價格卡 · 霓虹騎樓",
        "sceneTemplate.item.priceTierCard.summary" => {
            "1:1 墨藍夜色底上的三檔價目表，配霓虹燈管描邊與外散射，適合門市、活動與套餐報價。"
        }
        "sceneTemplate.item.noticeBoardCard.title" => "公告通知卡 · 鉛字報刊",
        "sceneTemplate.item.noticeBoardCard.summary" => {
            "4:5 新聞紙底上的報頭雙線與編號條款，含套印錯位與騎縫編號，適合通知、須知與規則說明。"
        }
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "時間線大事記長圖",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "直式資訊長圖：一條貫穿全圖的時間軸，年份刻度配大事記卡，末尾收在下一步，適合復盤、品牌史與專案歷程。",
        "sceneTemplate.item.conceptContrastInfographic.title" => "概念對比科普長圖",
        "sceneTemplate.item.conceptContrastInfographic.summary" => "直式資訊長圖：先給結論，再給兩個概念各自的定義卡，然後逐維度拆成兩欄表，最後給選擇判準。",
        "sceneTemplate.item.rankingBoardInfographic.title" => "榜單 TOP N 長圖",
        "sceneTemplate.item.rankingBoardInfographic.summary" => {
            "直式資訊長圖：墨底黑金的推薦榜，前三名大徽章、四到八名小描邊，每條給使用場景與頻次。"
        }
        "sceneTemplate.item.faqThreadInfographic.title" => "問答 FAQ 長圖",
        "sceneTemplate.item.faqThreadInfographic.summary" => {
            "直式資訊長圖：六組一問一答，Q 實心 A 描邊，不編號不排序，讀者只讀其中一條也成立。"
        }
        "sceneTemplate.item.dataStoryInfographic.title" => "數據故事長圖",
        "sceneTemplate.item.dataStoryInfographic.summary" => "直式資訊長圖：四個數字串成一條因果線，每段用十格方塊陣表示比例，末尾收到一句能改做法的結論。",
        "sceneTemplate.item.challengeTrackerInfographic.title" => "30 天打卡挑戰長圖",
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "直式資訊長圖：六欄五列的三十格打卡陣，只在第 7、15、30 天給里程碑，存進相簿每天劃掉一格。",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "產業地圖長圖",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => {
            "直式資訊長圖：二乘二的四區生態位陣列，每格掛三個位點並標出空位，石板灰底上浮白卡。"
        }
        "sceneTemplate.item.doDontComparison.title" => "好壞示範雙欄",
        "sceneTemplate.item.doDontComparison.summary" => "3:4 單卡：同一件事的兩種做法左右並排，不靠紅綠而靠材質與圖示區分對錯，色覺障礙讀者也讀得出來。",
        "sceneTemplate.item.mythTruthComparison.title" => "誤區與真相長圖",
        "sceneTemplate.item.mythTruthComparison.summary" => "直式長圖：五組「大家都這麼說 / 其實是這樣」交錯排開，誤區偏窄淺底、真相偏寬深底，一次只處理一組。",
        "sceneTemplate.item.pricingTiersComparison.title" => "價格級距對比",
        "sceneTemplate.item.pricingTiersComparison.summary" => "3:4 單卡：免費 / Pro / 團隊三檔並排，價格當錨點往下讀，右欄包含左欄，適合定價頁與方案說明。",
        "sceneTemplate.item.scenarioGuideComparison.title" => "情境選擇指南長圖",
        "sceneTemplate.item.scenarioGuideComparison.summary" => {
            "直式長圖：不擺參數，直接給七種處境，每種後面掛一個判定標籤，讀者只要找到自己那一行。"
        }
        "sceneTemplate.item.specTableComparison.title" => "規格表對比長圖",
        "sceneTemplate.item.specTableComparison.summary" => "直式長圖：兩個候選放進一張真表逐列比，贏的一格用深底反白頂起來，一眼掃下來就知道各自贏在哪。",
        "sceneTemplate.item.threeWayComparison.title" => "三方案橫評長圖",
        "sceneTemplate.item.threeWayComparison.summary" => "直式長圖：三個方案並排，中間一欄是推薦項，每欄第一列不是名字而是一句處境——讀者在找哪一欄是自己。",
        "sceneTemplate.item.timeShiftComparison.title" => "時間對比 · 一年前與現在",
        "sceneTemplate.item.timeShiftComparison.summary" => {
            "3:4 單卡：一條置中的標籤脊柱，左邊一年前、右邊現在，同一項的兩個取值落在同一列上。"
        }
        "sceneTemplate.item.tradeoffScaleComparison.title" => "優缺點天平",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "1:1 方卡：一根橫梁兩個托盤，左盤裝值得、右盤裝代價，每條前面留一個空方框——結論交給讀者自己秤。",
        "sceneTemplate.item.versionDiffComparison.title" => "新舊版本變化",
        "sceneTemplate.item.versionDiffComparison.summary" => {
            "1:1 方卡：不分左右兩欄，每一列自己完成一次「舊 → 新」，順著往下滑即可讀完全部變更。"
        }
        "sceneTemplate.item.appOnboardingTriptych.title" => "App 新手引導三屏",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "3:4 單卡：三台並排的手機與空圖位，把自己的三張引導圖拖進去配上文案，一張就能拿去評審或發布。",
        "sceneTemplate.item.diyBlueprintGuide.title" => "手作 DIY 圖解長圖",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "直式長圖：材料規格表與步驟各占一半篇幅——手作翻車多在準備而不在手上，所以先把材料寫清楚。",
        "sceneTemplate.item.photoCompositionTutorial.title" => "手機攝影構圖教學",
        "sceneTemplate.item.photoCompositionTutorial.summary" => {
            "3:4 五幀：每幀一個深色取景框，螢光參考線壓在圖位之上——構圖必須畫在取景框上才說得清。"
        }
        "sceneTemplate.item.recipeFourStep.title" => "食譜四步卡",
        "sceneTemplate.item.recipeFourStep.summary" => {
            "4:5 單卡 2×2 四宮格：四步全放在一張卡上，截圖存相簿就能照著做，站在爐台前不用翻頁。"
        }
        "sceneTemplate.item.skincareRoutineCards.title" => "保養步驟卡",
        "sceneTemplate.item.skincareRoutineCards.summary" => {
            "4:5 六幀：每步固定給用量、停留時長與早晚場次三個數——保養翻車多在用量和間隔上。"
        }
        "sceneTemplate.item.softwareStepTutorial.title" => "軟體操作步驟卡",
        "sceneTemplate.item.softwareStepTutorial.summary" => {
            "4:5 單卡：教學檔唯一一張深色，介面截圖位配編號操作說明，適合工具與軟體的功能講解。"
        }
        "sceneTemplate.item.storageMakeoverSteps.title" => "居家收納改造步驟",
        "sceneTemplate.item.storageMakeoverSteps.summary" => {
            "3:4 六幀：每步除了動作與圖位，固定給一條完成判定和一個耗時預算——做到那個狀態才算做完。"
        }
        "sceneTemplate.item.weeklyReportLesson.title" => "職場週報小課長圖",
        "sceneTemplate.item.weeklyReportLesson.summary" => {
            "直式長圖：講完四段結構之後直接給一張帶底線空格的週報骨架，截圖就能照著往裡填。"
        }
        "sceneTemplate.item.workoutBreakdownGuide.title" => "健身動作分解長圖",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "直式長圖：每個動作除圖位與要點外，還有一條固定格式的組數 / 次數 / 休息參數條，存圖照著數做。",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => "書影評拆解輪播",
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4 五板：鉤子、帶註解的原文、三個洞見、一句書摘、收束——把一部作品拆成能帶走的零件，不是複述劇情。",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "城市指南輪播",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => {
            "3:4 七板：照片與動線交替——地點板給做夢的讀者，一日動線與吃住對照給做計畫的讀者。"
        }
        "sceneTemplate.item.datareportGridCarousel.title" => "數據報告輪播",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4 六板：數據頁之間強制夾入非數據頁，避免讀者滑到第三張圖表就跳過，適合季報與產業觀察。",
        "sceneTemplate.item.opinionLongformCarousel.title" => "觀點長文輪播",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4 六板：一套嚴格的視覺母版貫穿全程，頁碼與標題永遠在同一個位置——輪播滑走就回不去，一致性是剛需。",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "問答體輪播",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => "3:4 六板：一問一板，每板左上角一個手寫問號編號——問題本身就是往下滑的理由，不需要留懸念。",
        "sceneTemplate.item.storyNightCarousel.title" => "故事敘事輪播",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4 七板：以時間為骨架的個人經歷復盤，第五板那條時間軸是全套的承重牆，前四板都是它的一個刻度。",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => "乾貨合集輪播",
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4 六板：六個工具逐板展開，最後一板連頁碼一起列成目錄——合集檔的讀者目的只有一個，收藏。",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "教學輪播",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => {
            "3:4 六板：一板一步，手指就是進度條，滑一次等於做完一步，適合手作、軟體與生活教學。"
        }
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "年度總結復盤輪播",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => {
            "3:4 八板：數字頁冷、感受頁熱，兩種溫度交替推進，適合年終總結與個人年度復盤。"
        }
        "fileMenu.newFromTemplate" => "從範本新增",
        "fileMenu.exportSlideshowHtml" => "匯出放映 HTML...",
        "fileMenu.exportPptx" => "匯出 PowerPoint...",
        "dialog.slideshowHtmlTitle" => "匯出放映",
        "dialog.slideshowHtmlSummary" => "已匯出 {{count}} 張投影片到：",
        "dialog.slideshowHtmlEmpty" => "目前簡報沒有可匯出的投影片。",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "可匯入的 HTML 內容無法使用。",
        "htmlImport.warn.content.empty_body" => "HTML 主體中可匯入的內容無法使用。",
        "htmlImport.warn.content.dom_depth_truncated" => {
            "巢狀層數超過 {{max_depth}} 層的 HTML 已捨棄。"
        }
        "htmlImport.warn.content.node_limit_truncated" => "已達節點上限，其餘頁面內容已略過。",
        "htmlImport.warn.content.node_limit_mapping" => "已達節點上限，部分 HTML 樹狀結構已略過。",
        "htmlImport.warn.content.node_limit_inline_row" => "已達節點上限，某個行內排版列已略過。",
        "htmlImport.warn.content.node_limit_pseudo" => "已達節點上限，產生的虛擬元素已略過。",
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "巢狀 at-rule 超過 {{max_depth}} 層的 CSS 規則已忽略。"
        }
        "htmlImport.warn.css.unterminated_rule" => "未結束的 CSS 規則已忽略。",
        "htmlImport.warn.css.marker_rules_unsupported" => "CSS ::marker 規則未匯入。",
        "htmlImport.warn.css.nesting_unsupported" => "巢狀的 CSS 樣式規則已忽略。",
        "htmlImport.warn.css.invalid_layer_name" => "無效的 @layer 名稱 '{{name}}' 已忽略。",
        "htmlImport.warn.css.unsupported_statement" => "不支援的 @{{name}} 陳述式已忽略。",
        "htmlImport.warn.css.media_without_viewport" => "未指定可視區域的 @media 規則已忽略。",
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "無效的 @layer 區塊名稱 '{{name}}' 已忽略。"
        }
        "htmlImport.warn.css.unsupported_container_block" => "@container 區塊已忽略。",
        "htmlImport.warn.css.unsupported_block" => "不支援的 @{{name}} 區塊已忽略。",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "@font-face 網頁字型 '{{family}}' 無法使用。"
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "絕對定位元素的百分比位移已近似處理。"
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "百分比的 position:relative 位移已近似處理。"
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "沒有確定軸向的 CSS aspect-ratio 已忽略。"
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "位於不確定包含區塊內的 CSS aspect-ratio 已忽略。"
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky 已忽略。",
        "htmlImport.warn.layout.grid_tracks_approximated" => "不支援的 CSS 格線軌道已近似處理。",
        "htmlImport.warn.layout.float_ignored" => "CSS float 已忽略。",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "節點層級的 CSS mix-blend-mode 已近似處理。"
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll 已近似處理。"
        }
        "htmlImport.warn.layout.negative_margins_ignored" => "負值的 CSS 邊界已忽略。",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => "視覺方塊上的 CSS 邊界已忽略。",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box 的百分比尺寸已近似處理。"
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "明確起始線所留下的空白 CSS 格線儲存格已近似處理。"
        }
        "htmlImport.warn.layout.grid_span_reflowed" => "跨距不符起始線的 CSS 格線項目已近似處理。",
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "已達節點上限，CSS 格線列的包裝元素已略過。"
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "使用 auto-fit / auto-fill 的 CSS 格線軌道寬度已近似處理。"
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas 的配置未匯入。"
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => "CSS grid-row 的配置未匯入。",
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` 已近似處理。"
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => "區塊軸向的 CSS 自動邊界未匯入。",
        "htmlImport.warn.layout.auto_margin_node_limit" => "已達節點上限，CSS 自動邊界對齊已略過。",
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "沒有確定尺寸之元素上的 CSS 流內位移已捨棄。"
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "已達節點上限，某個 CSS 流內位移已略過。"
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "CSS 流內位移（position:relative 內縮值、transform 平移）已近似處理。"
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "無法容納位移包裝元素之方塊上的 CSS 流內位移已捨棄。"
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "直向 flex 容器上的 flex-wrap 未匯入。"
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "flex-wrap:wrap-reverse 已近似處理。",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "沒有確定寬度之容器上的 flex-wrap 已忽略。"
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "換行 flex 容器上的 CSS align-content 未匯入。"
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "子項主軸尺寸不確定的 flex-wrap 已忽略。"
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => "已達節點上限，flex-wrap 的換行列已略過。",
        "htmlImport.warn.transform.unsupported_syntax" => "不支援的 CSS transform 語法已忽略。",
        "htmlImport.warn.transform.unsupported_function" => {
            "不支援的 CSS transform 函式（3D、matrix3d）已忽略。"
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "位於不確定軸向上的百分比 CSS transform 平移已捨棄。"
        }
        "htmlImport.warn.transform.non_finite_matrix" => "產生非有限矩陣的 CSS transform 已忽略。",
        "htmlImport.warn.transform.skew_dropped" => "CSS transform 的傾斜已捨棄。",
        "htmlImport.warn.transform.degenerate_scale" => {
            "縮放值為零或非有限值的 CSS transform 已近似處理。"
        }
        "htmlImport.warn.transform.mirroring_absolute" => "CSS transform 的鏡像已近似處理。",
        "htmlImport.warn.transform.origin_z_ignored" => "CSS transform-origin 的 Z 軸位移已忽略。",
        "htmlImport.warn.transform.scale_not_baked" => {
            "無法併入節點尺寸的 CSS transform 縮放已捨棄。"
        }
        "htmlImport.warn.transform.scale_baked" => {
            "已併入節點尺寸的 CSS transform 縮放已近似處理。"
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "自動尺寸元素上的 CSS transform 縮放已忽略。"
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "具方向性或帶間隔的 CSS background-repeat 已近似處理。"
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "明確指定的 CSS 背景拼貼尺寸已忽略。"
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "自動尺寸元素上的 CSS background-size 已近似處理。"
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "需要圖片內在尺寸的 CSS background-size 已近似處理。"
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "不支援的 CSS background-position 已忽略。"
        }
        "htmlImport.warn.visual.background_image_url_empty" => "空白的 CSS 背景圖片 URL 已忽略。",
        "htmlImport.warn.visual.conic_gradient_ignored" => "CSS 圓錐漸層已忽略。",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "不支援的 CSS background-image 圖層已忽略。"
        }
        "htmlImport.warn.visual.background_color_unresolved" => "無法解析的 CSS 背景色已忽略。",
        "htmlImport.warn.visual.background_position_dropped" => "CSS background-position 已忽略。",
        "htmlImport.warn.visual.border_colors_approximated" => {
            "各邊獨立的 CSS 框線顏色已近似處理。"
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "各邊混用的 CSS 框線樣式已近似處理。"
        }
        "htmlImport.warn.visual.border_style_complex" => "複雜的 CSS 框線樣式已近似處理。",
        "htmlImport.warn.visual.border_style_unsupported" => "不支援的 CSS 框線樣式已近似處理。",
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "橢圓形的 CSS 框線圓角半徑已近似處理。"
        }
        "htmlImport.warn.visual.border_radius_unsupported" => "不支援的 CSS 框線圓角半徑已忽略。",
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "不支援的 CSS box-shadow 圖層已忽略。"
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => "CSS 漸層的色彩內插方式已忽略。",
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "不支援的 CSS linear-gradient 方向已忽略。"
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => "CSS 漸層的色彩提示點已忽略。",
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "不支援的 CSS 漸層色彩停駐點已忽略。"
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => "可用停駐點少於兩個的 CSS 漸層已忽略。",
        "htmlImport.warn.visual.gradient_repeating_approximated" => "重複式的 CSS 漸層已近似處理。",
        "htmlImport.warn.visual.gradient_stops_clamped" => "超出範圍的 CSS 漸層停駐點已近似處理。",
        "htmlImport.warn.visual.blur_radius_unsupported" => "不支援的 CSS 模糊半徑已忽略。",
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "不支援的 CSS 濾鏡 drop-shadow() 已忽略。"
        }
        "htmlImport.warn.visual.filter_function_unsupported" => "不支援的 CSS 濾鏡函式已忽略。",
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "不支援的 CSS backdrop-filter 函式已忽略。"
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "不支援的 CSS background-blend-mode 已忽略。"
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "個別填色上的 CSS mix-blend-mode 已近似處理。"
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "不支援的 CSS mix-blend-mode 已忽略。"
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} 已忽略。",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "漸層上的 CSS background-size 已忽略。"
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "不支援的 CSS radial-gradient 位置已忽略。"
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "橢圓形的 CSS radial-gradient 已近似處理。"
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "CSS radial-gradient 的範圍關鍵字已近似處理。"
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "不支援的 CSS radial-gradient 尺寸已忽略。"
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => "不支援的 CSS text-shadow 圖層已忽略。",
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "第一層之後的 CSS text-shadow 圖層已忽略。"
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => "行內元素上的 CSS text-shadow 已忽略。",
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image 未匯入。",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "`list-style-position: outside` 的懸掛項目符號已近似處理。"
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "不支援的 CSS list-style-type `{{value}}` 已近似處理。"
        }
        "htmlImport.warn.media.object_fit_scale_down" => "CSS object-fit:scale-down 已近似處理。",
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none 已忽略。",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position 已忽略。",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "圖片上不支援的 CSS mix-blend-mode 已忽略。"
        }
        "htmlImport.warn.media.inline_svg_placeholder" => "行內的 <svg> 元素已改以預留位置匯入。",
        "htmlImport.warn.media.input_type_fallback" => "不支援的 <input> 類型已近似處理。",
        "htmlImport.warn.media.element_placeholder" => "<{{tag}}> 元素已改以預留位置匯入。",
        "htmlImport.warn.media.picture_undecodable_types" => {
            "來源類型皆無法解碼的 <picture> 已近似處理。"
        }
        "htmlImport.warn.table.rowspan_ignored" => "HTML 的 rowspan 屬性未匯入。",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "列群組遭 CSS 拆散之表格的欄寬已近似處理。"
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "沒有確定寬度之 CSS 表格的欄寬已近似處理。"
        }
        "htmlImport.warn.resource.invalid_base_href" => "無效的 <base href> {{href}} 已忽略。",
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "位於專案來源之外的 <base href> {{href}} 已忽略。"
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => "外部樣式表 {{url}} 無法使用。",
        "htmlImport.warn.resource.image_outside_origin" => {
            "位於專案來源之外的圖片 {{url}} 已改以預留位置匯入。"
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "無法使用的圖片 {{url}} 已改以預留位置匯入。"
        }
        "htmlImport.warn.resource.css_import_invalid" => "無效的 CSS @import {{prelude}} 已忽略。",
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "CSS @import {{reference}} 無法使用。"
        }
        "htmlImport.warn.resource.css_import_cycle" => "循環的 CSS @import {{url}} 已忽略。",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "超過深度 {{max_depth}} 的 CSS @import {{url}} 已忽略。"
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} 無法使用。",
        "htmlImport.warn.project.multiple_html_entries" => {
            "找到 {{count}} 個 HTML 進入點，已選用 {{entry}}，其餘已近似處理。"
        }
        "htmlImport.warn.snapshot.truncated" => "部分瀏覽器快照已捨棄。",
        "htmlImport.warn.snapshot.node_limit" => "已達節點上限，其餘快照內容已略過。",
        "htmlImport.warn.snapshot.tainted_images" => {
            "有 {{count}} 張受 CORS 汙染的圖片以遠端 URL 保留，無法使用。"
        }
        "htmlImport.warn.snapshot.invalid_rect" => "矩形範圍缺漏或無效的快照節點已捨棄。",
        "htmlImport.warn.snapshot.unknown_kind" => "類型不明的快照節點已捨棄。",
        "htmlImport.warn.snapshot.rejected" => "瀏覽器快照（{{reason}}）已捨棄。",
        "htmlImport.warn.snapshot.unsupported_transform" => "不支援的快照 transform 已忽略。",
        "htmlImport.warn.css.media_empty_query" => "空白的 @media 查詢條件已忽略。",
        "htmlImport.warn.css.media_unsupported_type" => "不支援的 @media 類型 '{{name}}' 已忽略。",
        "htmlImport.warn.css.media_unsupported_condition" => {
            "不支援的 @media 條件 '{{input}}' 已忽略。"
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "無效的 @media 方向 '{{value}}' 已忽略。"
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "不支援的 @media 特性 '{{name}}' 已忽略。"
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "不支援的 @media 範圍 '({{input}})' 已忽略。"
        }
        "htmlImport.warn.css.media_invalid_range" => "無效的 @media 範圍 '({{input}})' 已忽略。",
        "htmlImport.warn.css.media_invalid_length" => "無效的 @media 長度 '{{value}}' 已忽略。",
        "htmlImport.diagnostics.title" => "HTML 匯入完成",
        "htmlImport.diagnostics.summary" => "降級項目：{{count}}",
        "htmlImport.diagnostics.dismiss" => "關閉",
        "htmlImport.diagnostics.expand" => "顯示詳細資料",
        "htmlImport.diagnostics.collapse" => "隱藏詳細資料",
        "htmlImport.diagnostics.more" => "另有 {{count}} 項",
        "dialog.pptxTitle" => "匯出 PowerPoint",
        "dialog.pptxSummary" => "已匯出 {{count}} 張投影片到：",
        "dialog.pptxEmpty" => "目前簡報沒有可匯出的投影片。",
        "settings.agents.acpQuickAdd" => "快速新增",
        "settings.agents.acpPresetAdd" => "新增",
        "settings.agents.acpNotInstalled" => "未安裝",
        "assetCenter.title" => "資產中心",
        "assetCenter.tab.templates" => "範本",
        "assetCenter.tab.styles" => "風格",
        "assetCenter.style.empty" => "沒有符合的風格",
        "assetCenter.style.pinned" => "已釘選",
        "assetCenter.style.searchPlaceholder" => "搜尋風格或標籤",
        "assetCenter.style.generateHint" => "新建一個文件，依主題生成；已釘選的風格會直接採用。",
        "ai.pinnedStyle" => "風格：{{name}}",
        "assetCenter.style.import" => "匯入風格",
        "assetCenter.style.mine" => "我的風格",
        "assetCenter.style.builtIn" => "內建風格",
        "assetCenter.style.importTitle" => "匯入 DESIGN.md",
        "assetCenter.style.importHint" => "貼上 DESIGN.md 全文，然後確認匯入。",
        "assetCenter.style.importSource" => "可以從 styles.refero.design 等 DESIGN.md 風格庫複製內容。",
        "assetCenter.style.importConfirm" => "匯入",
        "assetCenter.style.importCancel" => "取消",
        "assetCenter.style.importPickFile" => "選擇檔案…",
        "assetCenter.style.importHintFile" => "選擇 DESIGN.md 檔案，或在下方貼上全文。",
        "assetCenter.style.importPlaceholder" => "在此貼上 DESIGN.md",
        "assetCenter.style.importEmpty" => "這個檔案是空的，或者內容太短，不像一份風格指南。",
        "assetCenter.style.importNotText" => "這個檔案不是 Markdown 文字。",
        "assetCenter.style.importTooLarge" => "這個檔案超過 512 KB。",
        "slidesPanel.tabSlides" => "投影片",
        "slidesPanel.tabCards" => "卡片",
        "slidesPanel.present" => "放映",
        "slidesPanel.exportPdf" => "匯出 PDF",
        "slidesPanel.exportAllSlides" => "匯出全部投影片",
        "slidesPanel.exportSelectedSlides" => "匯出所選投影片（{{count}}）",
        "settings.tab.ai" => "AI",
        "settings.agents.heroTitle" => "連接你的 AI 服務商",
        "settings.agents.heroSubtitle" => {
            "OpenPencil 直接驅動本機 CLI Agent 與 API 服務商，連接任一個即可開始產生設計。"
        }
        "settings.agents.statusConnected" => "已連接",
        "settings.agents.statusNotConnected" => "未連接",
        "settings.agents.statusChecking" => "正在檢測…",
        "settings.mcp.heroTitle" => "從外部透過 MCP 連接 OpenPencil",
        "settings.mcp.heroSubtitle" => {
            "把任何支援 MCP 的 CLI 或編輯器指向這個工作區，即可用內建 Agent 同款工具驅動畫布。"
        }
        "settings.mcp.terminalFootnote" => "* 啟動時會自動為選取的 CLI 工具設定 MCP。",
        "settings.mcp.customConfigTitle" => "自訂 MCP 伺服器設定",
        "settings.mcp.customConfigDesc" => "貼到任何讀取標準 MCP server 設定區塊的用戶端即可。",
        "settings.mcp.copyConfig" => "複製 MCP 設定",
        "settings.system.heroTitle" => "系統偏好",
        "settings.system.heroSubtitle" => "本機安裝的外觀、更新與畫布行為。",
        "settings.system.appearance" => "外觀",
        "settings.system.appearanceLight" => "淺色",
        "settings.system.appearanceDark" => "深色",
        "settings.system.pencilCursor" => "畫筆游標",
        "settings.images.heroTitle" => "為設計配圖",
        "settings.images.heroSubtitle" => "在 Openverse 搜尋圖片，或接上服務商依需求生成。",
        "settings.fonts.heroTitle" => "本文件的字型",
        "settings.fonts.heroSubtitle" => "補齊文件需要但本機缺少的字型，並管理你匯入的字型。",
        "settings.account.heroTitle" => "你的帳戶",
        "settings.account.heroSubtitle" => "登入後可在多裝置間同步工作區與授權。",
        "tooltip.topbar.file" => "檔案",
        "tooltip.topbar.import" => "匯入",
        "tooltip.topbar.language" => "語言",
        "tooltip.topbar.collaboration" => "協作",
        "tooltip.topbar.preview" => "預覽",
        "tooltip.topbar.exitPreview" => "結束預覽",
        "tooltip.topbar.account" => "帳戶",
        "settings.agents.providerRollMore" => "等 {{count}} 家",
        "ai.thinking.adaptive" => "思考：自動",
        "ai.thinking.disabled" => "思考：關閉",
        "ai.thinking.enabled" => "思考：開啟",
        "ai.designProgress.detail.repairsApplied" => "已套用 {{count}} 處自動修復",
        "ai.designProgress.detail.repairsMore" => "…還有 {{count}} 條(見記錄)",
        "ai.styleCard.builtin" => "內建風格",
        "ai.styleCard.imported" => "匯入的 DESIGN.md",
        "ai.styleCard.documentDesignMd" => "文件 design.md",
        _ => return super::zh_tw_collab::lookup(key),
    })
}

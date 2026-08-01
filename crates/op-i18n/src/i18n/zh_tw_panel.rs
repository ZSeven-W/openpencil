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
        "sceneTemplate.filter.all" => "全部",
        "sceneTemplate.scene.tutorial" => "教學圖",
        "sceneTemplate.scene.comparison" => "對比圖",
        "sceneTemplate.scene.carousel" => "知識卡片",
        "sceneTemplate.scene.slides" => "簡報",
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
        "fileMenu.newFromTemplate" => "從範本新增",
        _ => return super::zh_tw_collab::lookup(key),
    })
}

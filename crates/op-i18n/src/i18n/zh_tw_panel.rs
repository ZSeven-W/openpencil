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
        _ => return super::zh_tw_collab::lookup(key),
    })
}

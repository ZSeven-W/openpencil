//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `ja_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "画像を検索…",
        "imagePanel.searching" => "検索中…",
        "imagePanel.noResults" => "結果が見つかりません",
        "imagePanel.searchPrompt" => "画像を検索",
        "imagePanel.sourceNotice" => {
            "画像の提供元: {{source}}。自由ライセンス — 使用前にライセンスをご確認ください。"
        }
        "imagePanel.genNotConfigured" => "画像生成が未設定です",
        "imagePanel.openSettings" => "設定を開く",
        "imagePanel.promptPlaceholder" => "画像の内容を入力…",
        "providerProbe.connectedViaCli" => "{{name}} CLI 経由で接続しました",
        "providerProbe.cliExitedWithError" => "{{name}} CLI がエラーで終了しました",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI がバージョン情報を出力しませんでした",
        "providerProbe.modelQueryFailed" => "{{name}} のモデル取得に失敗またはタイムアウトしました",
        "providerProbe.modelQueryFailedRunLogin" => "{{name}} のモデル取得に失敗しました。{{command}} を一度実行して認証してください。",
        "providerProbe.modelQueryNeedsAuth" => "{{name}} のモデル取得には認証が必要です。{{command}} を一度実行してサインインしてください。",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} が認識できないモデル一覧を返しました",
        _ => return super::ja_collab::lookup(key),
    })
}

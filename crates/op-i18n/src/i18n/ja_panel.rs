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
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} のモデル取得に失敗しました。{{command}} を一度実行して認証してください。"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} のモデル取得には認証が必要です。{{command}} を一度実行してサインインしてください。"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} が認識できないモデル一覧を返しました",
        "promptCenter.title" => "プロンプトセンター",
        "promptCenter.searchPlaceholder" => "プロンプトを検索…",
        "promptCenter.category.all" => "すべて",
        "promptCenter.category.starter" => "はじめに",
        "promptCenter.category.mobileApp" => "モバイルアプリ",
        "promptCenter.category.webPage" => "Web ページ",
        "promptCenter.category.dashboard" => "ダッシュボード",
        "promptCenter.category.component" => "コンポーネント",
        "promptCenter.category.modify" => "修正",
        "promptCenter.category.custom" => "マイプロンプト",
        "promptCenter.empty" => "一致するプロンプトがありません",
        "promptCenter.saveCurrent" => "現在の入力を保存",
        "promptCenter.saveTitlePlaceholder" => "プロンプト名",
        "promptCenter.save" => "保存",
        "promptCenter.cancel" => "キャンセル",
        "promptCenter.delete" => "削除",
        "promptCenter.screens" => "{{count}}画面",
        "promptCenter.freeform" => "自由形式",
        "promptCenter.item.wander.title" => "Wander · 旅行プラン",
        "promptCenter.item.forage.title" => "Forage · 旬のレシピ",
        "promptCenter.item.still.title" => "Still · 瞑想と就寝",
        "promptCenter.item.hearth.title" => "Hearth · スマートホーム",
        "promptCenter.item.meteo.title" => "Meteo · 没入型天気",
        "promptCenter.item.marginalia.title" => "Marginalia · 読書と注釈",
        "promptCenter.item.lingua.title" => "Lingua · 言語学習",
        "promptCenter.item.daybreak.title" => "Daybreak · コーヒー注文",
        "promptCenter.item.verdant.title" => "Verdant · 植物ケア",
        "promptCenter.item.companion.title" => "Companion · ペットライフ",
        "promptCenter.item.relic.title" => "Relic · 厳選リユース市場",
        "promptCenter.item.nocturne.title" => "Nocturne · 星空観察ガイド",
        "promptCenter.item.marquee.title" => "Marquee · 映画ウォッチリスト",
        "promptCenter.item.ritual.title" => "Ritual · 習慣づくり",
        "promptCenter.item.ember.title" => "Ember · 気分日記",
        "promptCenter.item.volt.title" => "Volt · EV コンパニオン",
        "promptCenter.item.aloft.title" => "Aloft · フライト追跡",
        "promptCenter.item.gallery.title" => "Gallery · 展覧会と文化イベント",
        "promptCenter.item.nightcap.title" => "Nightcap · ホームカクテル",
        "promptCenter.item.bloom.title" => "Bloom · 子どもの成長記録",
        "promptCenter.item.extremeWeather.title" => "極限 · 天気アプリ",
        "promptCenter.item.extremeNowPlaying.title" => "極限 · 再生中",
        "promptCenter.item.extremeDailyApp.title" => "極限 · 毎日使いたいアプリ",
        "promptCenter.item.extremeCalendar.title" => "極限 · カレンダー",
        "promptCenter.item.extremeCalm.title" => "極限 · 静けさ",
        "promptCenter.item.webOrbit.title" => "Orbit · AI ワークベンチのランディングページ",
        "promptCenter.item.webAtelier.title" => "Atelier · 家具ブランドの EC サイト",
        "promptCenter.item.dashboardPulse.title" => "Pulse · グロース分析ダッシュボード",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · 物流オペレーション",
        "promptCenter.item.componentDataGrid.title" => {
            "Gridworks · エンタープライズデータテーブル"
        }
        "promptCenter.item.componentFormLab.title" => "Form Lab · フォームコンポーネントシステム",
        "promptCenter.item.modifyPolishCurrent.title" => "現在の画面を磨く",
        "promptCenter.item.modifyCompleteStates.title" => "コンポーネント状態を補完",
        "collab.ownerConfirm.title" => "参加先の相手を確認してください",
        "collab.ownerConfirm.hint" => "このセッションの内容はまだ何も読み込まれていません。",
        "collab.ownerConfirm.account" => "検証済みアカウント",
        "collab.ownerConfirm.device" => "検証済みデバイス",
        "collab.ownerConfirm.claimedName" => "このアカウントが設定した名前（未検証）",
        "collab.action.confirmOwner" => "このセッションに参加",
        "collab.action.rejectOwner" => "参加しない",
        "collab.error.ownerNotConfirmed" => "ホストを確認しなかったため、何も読み込まれませんでした。",
        "sceneTemplate.title" => "シーンテンプレート",
        "sceneTemplate.searchPlaceholder" => "シーンやテンプレートを検索…",
        "sceneTemplate.empty" => "一致するテンプレートがありません",
        "sceneTemplate.frames" => "{{count}}ページ",
        "sceneTemplate.filter.all" => "すべて",
        "sceneTemplate.scene.tutorial" => "チュートリアル画像",
        "sceneTemplate.scene.comparison" => "比較画像",
        "sceneTemplate.scene.carousel" => "ナレッジカード",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => {
            "3ステップのスクリーンショットチュートリアルカード"
        }
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "表紙、3つの操作ステップ、最後のCTAで構成。スクリーンショットと説明を差し替えるだけで公開できます。"
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "ナレッジ・インサイトカルーセル",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "表紙、3つの論点、まとめページで構成。1つの主張をスワイプできる連続カードに展開するのに最適です。"
        }
        "sceneTemplate.item.beforeAfter.title" => "リニューアル前後の比較",
        "sceneTemplate.item.beforeAfter.summary" => {
            "左右に並べたビフォー・アフターに変更内容を添え、振り返りや作品紹介に最適です。"
        }
        "sceneTemplate.item.slideDeck.title" => "プレゼンテーション · 6ページ",
        "sceneTemplate.item.slideDeck.summary" => {
            "表紙、目次、要点、データ、グラフ、締めの6ページ構成。16:9の投影比率で、テキストを差し替えるだけで発表できます。"
        }
        "fileMenu.newFromTemplate" => "テンプレートから新規作成",
        _ => return super::ja_collab::lookup(key),
    })
}

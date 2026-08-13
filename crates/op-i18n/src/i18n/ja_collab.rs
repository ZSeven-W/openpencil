//! コラボレーション UI の文言。

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "collab.topbar.collaborate" => "共同編集",
        "collab.topbar.starting" => "共同編集を開始しています…",
        "collab.topbar.joining" => "参加しています…",
        "collab.topbar.authenticating" => "認証しています…",
        "collab.topbar.connected" => "接続済み",
        "collab.topbar.reconnecting" => "再接続しています…",
        "collab.topbar.readOnly" => "読み取り専用",
        "collab.topbar.ended" => "セッション終了",
        "collab.topbar.participants" => "参加者 {{count}} 人",
        "collab.topbar.unavailable" => "このビルドでは共同編集を利用できません",
        "collab.action.start" => "セッションを作成",
        "collab.action.join" => "セッションに参加",
        "collab.home.hint" => "新しいセッションを作成するか、招待で参加します。",
        "collab.create.choose" => "他のユーザーの接続方法を選択します。",
        "collab.action.leave" => "セッションから退出",
        "collab.action.retry" => "再試行",
        "collab.action.cancel" => "キャンセル",
        "collab.action.connect" => "接続",
        "collab.action.copyInviteCode" => "招待コードをコピー",
        "collab.action.copyInviteLink" => "招待リンクをコピー",
        "collab.action.findNearby" => "近くを検索",
        "collab.join.code" => "招待コードまたは IP アドレス",
        "collab.join.codePlaceholder" => "A2C4E6G8J0 または 192.168.1.8:43120",
        "collab.join.publicHint" => "招待コードでインターネット経由の安全な接続ができます。",
        "collab.join.nearby" => "近くのセッション",
        "collab.session.invite" => "公開招待コード",
        "collab.session.preparingInvite" => "安全な招待を準備中…",
        "collab.session.publicReady" => "公開リレーの準備ができました",
        "collab.session.region" => "ホームリレー地域",
        "collab.connection.lan" => "ローカルネットワーク",
        "collab.connection.relay" => "公開リレー",
        "collab.region.china" => "中国",
        "collab.region.global" => "グローバル",
        "collab.error.inviteUnavailable" => "この招待は利用できないか、有効期限が切れています。",
        "collab.error.relayUnavailable" => "公開リレーは一時的に利用できません。",
        "collab.error.inviteInvalid" => "招待コードが無効です。確認してもう一度お試しください。",
        "collab.error.inviteExpired" => {
            "招待コードの有効期限が切れています。オーナーに新しいコードを依頼してください。"
        }
        "collab.error.relayNotConfigured" => "このデバイスには公開リレーが設定されていません。",
        "collab.error.regionUnavailable" => "招待のホームリレー地域は利用できません。",
        "collab.error.secureKeyUnavailable" => "デバイスのセキュアキーにアクセスできません。キーチェーンのアクセス権を確認して再試行してください。",
        "collab.error.rateLimited" => "接続試行が多すぎます。しばらくしてから再試行してください。",
        "collab.action.discardPending" => "保留中の編集を破棄",
        "collab.action.saveAsFork" => "分岐として保存",
        "collab.action.approveEditor" => "編集者として承認",
        "collab.action.approveViewer" => "閲覧者として承認",
        "collab.action.rejectAdmission" => "拒否",
        "collab.admission.request" => "認証済みの参加者がアクセスをリクエストしています。",
        "collab.join.title" => "共同編集セッションに参加",
        "collab.join.discovering" => "ローカルネットワークのセッションを検索しています…",
        "collab.join.noSessions" => "ローカルセッションが見つかりません",
        "collab.join.address" => "IP アドレスとポート",
        "collab.join.addressPlaceholder" => "192.168.1.8:43120",
        "collab.join.authenticating" => "安全なセッションを確認しています…",
        "collab.join.incompatible" => "このセッションは互換性のないバージョンです",
        "collab.join.signInRequired" => "セッションを開始または参加するにはログインしてください",
        "collab.session.title" => "共同編集",
        "collab.session.name" => "セッション：{{name}}",
        "collab.session.shareAddress" => "共有アドレス",
        "collab.session.role.owner" => "オーナー",
        "collab.session.role.editor" => "編集者",
        "collab.session.role.viewer" => "閲覧者",
        "collab.session.pending" => "オーナーによる編集の確認を待っています…",
        "collab.status.disconnectedReadOnly" => "接続が切れました。再接続中は編集できません。",
        "collab.status.ticketExpired" => {
            "共同編集のログイン期限が切れました。再度ログインしてください。"
        }
        "collab.status.ownerLeft" => {
            "オーナーが退出したためセッションは終了しました。別のコピーを保存できます。"
        }
        "collab.status.epochChanged" => {
            "オーナーが新しいセッションを開始しました。保留中の編集は送信されていません。"
        }
        "collab.status.undoConflict" => {
            "同じ項目が後から編集されたため、この変更は元に戻せません。"
        }
        "collab.status.unsupportedEdit" => "この編集は共同編集で未対応のため適用されませんでした。",
        "collab.status.profileUnavailable" => {
            "プロフィール画像を読み込めないため、イニシャルを表示します。"
        }
        "collab.reject.staleBase" => "文書が先に変更されました。同期してから再試行します。",
        "collab.reject.readOnly" => "このセッションでは閲覧のみ可能です。",
        "collab.reject.unsupported" => "オーナーはこの編集に対応していません。",
        "collab.reject.conflict" => "この編集は新しい変更と競合しています。",
        "collab.reject.conflictDetail" => "破棄: {{node}} の {{fields}}。",
        "collab.action.reapply" => "自分の編集を再適用",
        "collab.reject.resourceLimit" => "この編集はセッションの上限を超えています。",
        "collab.reject.authentication" => "共同編集の認証が無効になりました。",
        "collab.reject.unknown" => "オーナーがこの編集を拒否しました。",
        "collab.gate.pages" => "ページの変更は共同編集でまだ利用できません。",
        "collab.gate.pageBackground" => "ページ背景の変更は共同編集でまだ利用できません。",
        "collab.gate.variablesThemes" => "変数とテーマは共同編集でまだ利用できません。",
        "collab.gate.components" => "コンポーネント登録の変更は共同編集でまだ利用できません。",
        "collab.gate.uikit" => "UIKit の変更は共同編集でまだ利用できません。",
        "collab.gate.externalAssets" => "画像、SVG、HTML、その他の外部素材はまだ取り込めません。",
        "collab.gate.clipboardPaste" => "文書内容の貼り付けは共同編集でまだ利用できません。",
        "collab.gate.duplicate" => "ノードの複製は共同編集でまだ利用できません。",
        "collab.gate.bulkWrite" => "共同編集中は文書の一括変更が無効です。",
        "collab.gate.replaceDocument" => "共同編集中は文書全体を置き換えられません。",
        "collab.gate.rootMetadata" => "文書メタデータの変更は共同編集でまだ利用できません。",
        "collab.gate.typography" => "文字組みの変更は共同編集でまだ利用できません。",
        "collab.gate.effects" => "エフェクトは共同編集でまだ利用できません。",
        "collab.gate.visibilityLocking" => "表示とロックの変更は共同編集でまだ利用できません。",
        "collab.gate.nodeReplacement" => "ノードの置き換えは共同編集でまだ利用できません。",
        "collab.gate.nodeProperty" => "このノード属性は共同編集でまだ利用できません。",
        "collab.gate.nodeKind" => "このノード種類は共同編集でまだ利用できません。",
        "collab.gate.sessionTransition" => "共同編集セッションの準備中は編集できません。",
        "collab.gate.readOnly" => "この共同編集セッションは読み取り専用です。",
        "collab.gate.pendingEdit" => "保留中の編集が確認されるまで次の変更を待ってください。",
        "collab.gate.aiMcp" => "共同編集中は AI と MCP による文書書き込みが無効です。",
        "collab.gate.undoUnavailable" => {
            "共同編集中は全体の取り消しが無効です。確認済みの自分の変更のみ取り消せます。"
        }
        "collab.gate.redoUnavailable" => "共同編集ではやり直しをまだ利用できません。",
        "collab.gate.ownerOnlySave" => "共有元ファイルを保存できるのはオーナーだけです。",
        "collab.gate.leaveSessionFirst" => {
            "別の文書を開くか置き換える前に共同編集を退出してください。"
        }
        "collab.status.localEditPreserved" => {
            "リモート版を適用しました。「元に戻す」でローカルの変更を復元できます。"
        }
        "collab.a11y.participant" => "{{name}}、{{role}}",
        "collab.a11y.remoteCursor" => "{{name}} のカーソル",
        "icon.catalogLoading" => "アイコンカタログを読み込んでいます…",
        "sceneTemplate.documentUnavailable" => "このテンプレートのドキュメントを読み込めませんでした。もう一度お試しください。",
        "sceneTemplate.deleteFailed" => "このテンプレートを削除できませんでした。もう一度お試しください。",
        "sceneTemplate.item.soundingNavyDeck.title" => "測深海図 · 戦略提案デッキ",
        "sceneTemplate.item.soundingNavyDeck.summary" => "海図の温白にスチールブルーと墨藍。各ページは結論から始め、出典付きの数値ひとつで裏づける。水深プロファイルの棒と航跡目次が七ページを貫く、コンサル提出物や戦略報告向け。",
        "sceneTemplate.item.tidemarkSlateDeck.title" => "潮痕スレート · データレビュー デッキ",
        "sceneTemplate.item.tidemarkSlateDeck.summary" => "スレートグレーの地に全体タイル、トレンド線、明細表、リスク一覧、スイムレーンのロードマップ。七ページでデータレビューを一巡する、四半期の振り返りや指標定例向け。",
        "sceneTemplate.item.banxinRuleDeck.title" => "版心と界線 · 中国語組版デッキ",
        "sceneTemplate.item.banxinRuleDeck.summary" => "版心、界線、天の注記、魚尾のノンブルで組み立てた中国語本文のための組版。本文は一行三十字以内に収め、二段の対照や引用ページが読みやすい、講義や研究会向け。",
        "sceneTemplate.item.gridpaperGraphiteDeck.title" => "方眼グラファイト · 学術発表デッキ",
        "sceneTemplate.item.gridpaperGraphiteDeck.summary" => "方眼紙の地にグラファイトの文字。課題と空白、手法、図と表の結果二ページ、限界、結論、参考文献まで八ページを審査の順に配置、着手報告や学位審査向け。",
        "sceneTemplate.item.dossierLinenDeck.title" => "リネン調ドシエ · 文書メモデッキ",
        "sceneTemplate.item.dossierLinenDeck.summary" => "リネン紙の書類束の質感で、表紙、背景、現状データ、分析、案の比較から決議まで。単体で読み切れるメモとして仕上げた八ページ、意思決定レビュー向け。",
        "sceneTemplate.item.ledgerTickDeck.title" => "台帳の目盛 · 競合マトリクス デッキ",
        "sceneTemplate.item.ledgerTickDeck.summary" => "台帳の罫線の上に評価基準、主マトリクス、分位の目盛、差と強みの対照。七ページで競合比較を照合可能な帳簿として語る、製品選定や市場分析向け。",
        "sceneTemplate.item.brandConceptSheet.title" => "ブランドコンセプトシート",
        "sceneTemplate.item.brandConceptSheet.summary" => "主案のロックアップ、構成ロジック、白黒反転、最小サイズでの成立を一枚で確認する横長ボード。初回コンセプトの受け渡し向け。",
        "sceneTemplate.item.logoQaBoard.title" => "ロゴ融合 QA ボード",
        "sceneTemplate.item.logoQaBoard.summary" => "意味の共有、構造上の依存、シルエットの統一、そして副次的な読みが貼り足しではなく発見に見えるかを四つの枠で確認します。",
        "account.mcpToken" => "MCP トークン",
        "menu.saveAsTemplate" => "テンプレートとして保存…",
        "menu.saveAsTemplate.saved" => "テンプレートとして保存しました",
        "menu.saveAsTemplate.failed" => "テンプレートを保存できませんでした",
        "assetCenter.template.mine" => "マイテンプレート",
        "assetCenter.template.builtIn" => "内蔵テンプレート",
        "ai.designProgress.detail.failureReason" => "失敗理由：{{reason}}",
        "ai.designProgress.detail.noDiagnostic" => "エージェントがエラーの説明を返さずに失敗しました。",
        "ai.designProgress.detail.noResult" => "このセクションの結果を返す前にエージェントが停止しました。",
        "ai.designProgress.detail.connectionClosed" => "このセクションの結果が返る前にエージェント接続が切れました。",
        "ai.designProgress.detail.stoppedByUser" => "ユーザーが停止したため、このセクションは完了していません。",
        _ => return None,
    })
}

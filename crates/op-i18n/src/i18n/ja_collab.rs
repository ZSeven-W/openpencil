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
        "collab.action.start" => "セッションを開始",
        "collab.action.join" => "セッションに参加",
        "collab.action.leave" => "セッションから退出",
        "collab.action.retry" => "再試行",
        "collab.action.cancel" => "キャンセル",
        "collab.action.connect" => "接続",
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
        "collab.a11y.participant" => "{{name}}、{{role}}",
        "collab.a11y.remoteCursor" => "{{name}} のカーソル",
        _ => return None,
    })
}

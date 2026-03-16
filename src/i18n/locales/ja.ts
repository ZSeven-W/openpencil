import type { TranslationKeys } from './en'

const ja: TranslationKeys = {
  // ── Common ──
  'common.rename': '名前を変更',
  'common.duplicate': '複製',
  'common.delete': '削除',
  'common.cancel': 'キャンセル',
  'common.save': '保存',
  'common.close': '閉じる',
  'common.connect': '接続',
  'common.disconnect': '切断',
  'common.import': 'インポート',
  'common.export': 'エクスポート',
  'common.name': '名前',
  'common.untitled': '無題',
  'common.best': '最適',
  'common.selected': '{{count}} 件選択中',

  // ── Toolbar ──
  'toolbar.select': '選択',
  'toolbar.text': 'テキスト',
  'toolbar.frame': 'フレーム',
  'toolbar.hand': 'ハンド',
  'toolbar.undo': '元に戻す',
  'toolbar.redo': 'やり直す',
  'toolbar.variables': '変数',
  'toolbar.uikitBrowser': 'UIKit ブラウザ',

  // ── Shapes ──
  'shapes.rectangle': '長方形',
  'shapes.ellipse': '楕円',
  'shapes.line': '線',
  'shapes.icon': 'アイコン',
  'shapes.importImageSvg': '画像または SVG をインポート\u2026',
  'shapes.pen': 'ペン',
  'shapes.shapeTools': 'シェイプツール',
  'shapes.moreShapeTools': 'その他のシェイプツール',

  // ── Top Bar ──
  'topbar.hideLayers': 'レイヤーを非表示',
  'topbar.showLayers': 'レイヤーを表示',
  'topbar.new': '新規',
  'topbar.open': '開く',
  'topbar.save': '保存',
  'topbar.importFigma': 'Figma をインポート',
  'topbar.codePanel': 'コード',
  'topbar.lightMode': 'ライトモード',
  'topbar.darkMode': 'ダークモード',
  'topbar.fullscreen': 'フルスクリーン',
  'topbar.exitFullscreen': 'フルスクリーンを終了',
  'topbar.edited': '— 編集済み',
  'topbar.agentsAndMcp': 'Agents & MCP',
  'topbar.setupAgentsMcp': 'Agents & MCP を設定',
  'topbar.connected': '接続済み',
  'topbar.agentStatus': '{{agents}} Agent{{agentSuffix}} · {{mcp}} MCP',

  // ── Right Panel ──
  'rightPanel.design': 'デザイン',
  'rightPanel.code': 'コード',
  'rightPanel.noSelection': '要素を選択してください',

  // ── Pages ──
  'pages.title': 'ページ',
  'pages.addPage': 'ページを追加',
  'pages.moveUp': '上に移動',
  'pages.moveDown': '下に移動',

  // ── Status Bar ──
  'statusbar.zoomOut': '縮小',
  'statusbar.zoomIn': '拡大',
  'statusbar.resetZoom': 'ズームをリセット',

  // ── Updater ──
  'updater.softwareUpdate': 'ソフトウェアアップデート',
  'updater.dismiss': '閉じる',
  'updater.current': '現在のバージョン',
  'updater.latest': '最新バージョン',
  'updater.unknown': '不明',
  'updater.checking': '確認中...',
  'updater.downloadProgress': 'ダウンロード進捗',
  'updater.checkAgain': '再確認',
  'updater.restartInstall': '再起動してインストール',
  'updater.installing': 'インストール中...',
  'updater.releaseDate': 'リリース日：{{date}}',
  'updater.restartHint':
    '再起動してアップデートを適用します。再起動には通常 10〜15 秒かかります。',
  'updater.unknownError': '不明なアップデートエラーです。',
  'updater.title.checking': 'アップデートを確認中',
  'updater.title.available': 'アップデートが見つかりました',
  'updater.title.downloading': 'アップデートをダウンロード中',
  'updater.title.downloaded': 'インストール準備完了',
  'updater.title.error': 'アップデートに失敗しました',
  'updater.subtitle.checking': '最新リリースを確認中...',
  'updater.subtitle.available': 'バージョン {{version}} が利用可能です。',
  'updater.subtitle.availableGeneric': '新しいバージョンが利用可能です。',
  'updater.subtitle.downloading':
    'バージョン {{version}} をバックグラウンドでダウンロード中。',
  'updater.subtitle.downloadingGeneric':
    'アップデートパッケージをバックグラウンドでダウンロード中。',
  'updater.subtitle.downloaded':
    'バージョン {{version}} のダウンロードが完了しました。',
  'updater.subtitle.downloadedGeneric':
    'アップデートのダウンロードが完了しました。',
  'updater.subtitle.error':
    'アップデートの確認またはダウンロードができませんでした。',

  // ── Layers ──
  'layers.title': 'レイヤー',
  'layers.empty':
    'レイヤーがありません。ツールバーからシェイプを描画してください。',

  // ── Layer Context Menu ──
  'layerMenu.groupSelection': '選択をグループ化',
  'layerMenu.createComponent': 'コンポーネントを作成',
  'layerMenu.detachComponent': 'コンポーネントを解除',
  'layerMenu.detachInstance': 'インスタンスを解除',
  'layerMenu.booleanUnion': '合体',
  'layerMenu.booleanSubtract': '前面で型抜き',
  'layerMenu.booleanIntersect': '交差',
  'layerMenu.toggleLock': 'ロックの切り替え',
  'layerMenu.toggleVisibility': '表示の切り替え',

  // ── Property Panel ──
  'property.createComponent': 'コンポーネントを作成',
  'property.detachComponent': 'コンポーネントを解除',
  'property.goToComponent': 'コンポーネントに移動',
  'property.detachInstance': 'インスタンスを解除',

  // ── Fill ──
  'fill.title': '塗り',
  'fill.solid': '単色',
  'fill.linear': '線形グラデーション',
  'fill.radial': '放射グラデーション',
  'fill.image': '画像',
  'fill.stops': 'カラーストップ',
  'fill.angle': '角度',

  // ── Image ──
  'image.title': '画像',
  'image.fit': 'フィットモード',
  'image.fill': '塗りつぶし',
  'image.fitMode': 'フィット',
  'image.crop': 'クロップ',
  'image.tile': 'タイル',
  'image.clickToUpload': 'クリックしてアップロード',
  'image.changeImage': '画像を変更',
  'image.adjustments': '調整',
  'image.exposure': '露出',
  'image.contrast': 'コントラスト',
  'image.saturation': '彩度',
  'image.temperature': '色温度',
  'image.tint': '色合い',
  'image.highlights': 'ハイライト',
  'image.shadows': 'シャドウ',
  'image.reset': 'リセット',

  // ── Stroke ──
  'stroke.title': '線',

  // ── Appearance ──
  'appearance.layer': 'レイヤー',
  'appearance.opacity': '不透明度',

  // ── Layout ──
  'layout.flexLayout': 'フレックスレイアウト',
  'layout.freedom': 'フリー（レイアウトなし）',
  'layout.vertical': '垂直レイアウト',
  'layout.horizontal': '水平レイアウト',
  'layout.alignment': '配置',
  'layout.gap': '間隔',
  'layout.spaceBetween': '均等配置（両端）',
  'layout.spaceAround': '均等配置（周囲）',
  'layout.dimensions': 'サイズ',
  'layout.fillWidth': '幅を埋める',
  'layout.fillHeight': '高さを埋める',
  'layout.hugWidth': '幅に合わせる',
  'layout.hugHeight': '高さに合わせる',
  'layout.clipContent': 'コンテンツをクリップ',

  // ── Padding ──
  'padding.title': 'パディング',
  'padding.paddingMode': 'パディングモード',
  'padding.paddingValues': 'パディング値',
  'padding.oneValue': '全辺統一値',
  'padding.horizontalVertical': '水平/垂直',
  'padding.topRightBottomLeft': '上/右/下/左',

  // ── Typography ──
  'text.typography': 'タイポグラフィ',
  'text.lineHeight': '行の高さ',
  'text.letterSpacing': '文字間隔',
  'text.horizontal': '水平',
  'text.vertical': '垂直',
  'text.alignLeft': '左揃え',
  'text.alignCenter': '中央揃え',
  'text.alignRight': '右揃え',
  'text.justify': '均等割り付け',
  'text.top': '上',
  'text.middle': '中央',
  'text.bottom': '下',
  'text.weight.thin': 'Thin',
  'text.weight.light': 'Light',
  'text.weight.regular': 'Regular',
  'text.weight.medium': 'Medium',
  'text.weight.semibold': 'Semibold',
  'text.weight.bold': 'Bold',
  'text.weight.black': 'Black',
  'text.font.search': 'フォントを検索\u2026',
  'text.font.bundled': 'バンドル',
  'text.font.system': 'システム',
  'text.font.loading': 'フォントを読み込み中\u2026',
  'text.font.noResults': 'フォントが見つかりません',

  // ── Text Layout ──
  'textLayout.title': 'レイアウト',
  'textLayout.dimensions': 'サイズ',
  'textLayout.resizing': 'リサイズ',
  'textLayout.autoWidth': '自動 W',
  'textLayout.autoWidthDesc': '自動幅 \u2014 テキストが水平に拡張',
  'textLayout.autoHeight': '自動 H',
  'textLayout.autoHeightDesc':
    '自動高さ \u2014 幅固定、高さが自動調整',
  'textLayout.fixed': '固定',
  'textLayout.fixedDesc':
    '固定サイズ \u2014 幅と高さの両方が固定',
  'textLayout.fillWidth': '幅を埋める',
  'textLayout.fillHeight': '高さを埋める',

  // ── Effects ──
  'effects.title': 'エフェクト',
  'effects.dropShadow': 'ドロップシャドウ',
  'effects.blur': 'ぼかし',
  'effects.spread': '広がり',
  'effects.color': '色',

  // ── Export ──
  'export.title': 'エクスポート',
  'export.format': '形式',
  'export.scale': '倍率',
  'export.selectedOnly': '選択項目のみエクスポート',
  'export.exportFormat': '{{format}} をエクスポート',
  'export.exportLayer': 'レイヤーをエクスポート',

  // ── Corner Radius ──
  'cornerRadius.title': '角丸',

  // ── Size / Position ──
  'size.position': '位置',

  // ── Icon ──
  'icon.title': 'アイコン',
  'icon.searchIcons': 'アイコンを検索...',
  'icon.noIconsFound': 'アイコンが見つかりません',
  'icon.typeToSearch': '入力して Iconify アイコンを検索',
  'icon.iconsCount': '{{count}} 個のアイコン',

  // ── Variables Panel ──
  'variables.addTheme': 'テーマを追加',
  'variables.addVariant': 'バリアントを追加',
  'variables.addVariable': '変数を追加',
  'variables.searchVariables': '変数を検索...',
  'variables.noMatch': '一致する変数がありません',
  'variables.noDefined': '変数が定義されていません',
  'variables.closeShortcut': '閉じる (\u2318\u21e7V)',
  'variables.presets': 'プリセット',
  'variables.savePreset': '現在の設定をプリセットとして保存…',
  'variables.loadPreset': 'プリセットを読み込み',
  'variables.importPreset': 'ファイルからインポート…',
  'variables.exportPreset': 'ファイルにエクスポート…',
  'variables.presetName': 'プリセット名',
  'variables.noPresets': '保存されたプリセットはありません',

  // ── AI Chat ──
  'ai.newChat': '新しいチャット',
  'ai.collapse': '折りたたむ',
  'ai.tryExample': 'サンプルを試してデザイン...',
  'ai.tipSelectElements':
    'ヒント：チャットの前にキャンバス上の要素を選択するとコンテキストが提供されます。',
  'ai.generating': '生成中...',
  'ai.designWithAgent': 'Agent でデザイン...',
  'ai.attachImage': '画像を添付',
  'ai.stopGenerating': '生成を停止',
  'ai.sendMessage': 'メッセージを送信',
  'ai.loadingModels': 'モデルを読み込み中...',
  'ai.noModelsConnected': 'モデルが接続されていません',
  'ai.quickAction.loginScreen': 'モバイルログイン画面をデザイン',
  'ai.quickAction.loginScreenPrompt':
    'メール入力、パスワード入力、ログインボタン、ソーシャルログインオプションを含む、モダンなモバイルログイン画面をデザインしてください',
  'ai.quickAction.foodApp': 'フードアプリのホームページ',
  'ai.quickAction.foodAppPrompt':
    'Generate a well-designed food mobile app homepage',
  'ai.quickAction.bottomNav': 'ボトムナビゲーションバーをデザイン',
  'ai.quickAction.bottomNavPrompt':
    'ホーム、検索、追加、メッセージ、プロフィールの 5 つのタブを含むモバイルアプリのボトムナビゲーションバーをデザインしてください',
  'ai.quickAction.colorPalette': 'アプリのカラーパレットを提案',
  'ai.quickAction.colorPalettePrompt':
    'ペットケアアプリ向けのモダンなカラーパレットを提案してください',

  // ── Code Panel ──
  'code.reactTailwind': 'React + Tailwind',
  'code.htmlCss': 'HTML + CSS',
  'code.cssVariables': 'CSS Variables',
  'code.copyClipboard': 'クリップボードにコピー',
  'code.copied': 'コピーしました！',
  'code.download': 'コードファイルをダウンロード',
  'code.closeCodePanel': 'コードパネルを閉じる',
  'code.genCssVars': 'ドキュメント全体の CSS 変数を生成中',
  'code.genSelected': '{{count}} 個の選択要素のコードを生成中',
  'code.genDocument': 'ドキュメント全体のコードを生成中',
  'code.aiEnhance': 'AI で改善',
  'code.cancelEnhance': '改善をキャンセル',
  'code.resetEnhance': '元に戻す',
  'code.enhancing': 'AI がコードを改善中...',
  'code.enhanced': 'AI により改善済み',

  // ── Save Dialog ──
  'save.saveAs': '名前を付けて保存',
  'save.fileName': 'ファイル名',

  // ── Agent Settings ──
  'agents.title': 'Agents & MCP を設定',
  'agents.agentsOnCanvas': 'キャンバス上の Agents',
  'agents.mcpIntegrations': 'ターミナルでの MCP 連携',
  'agents.transport': 'トランスポート',
  'agents.port': 'ポート',
  'agents.mcpRestart':
    'MCP 連携はターミナルの再起動後に有効になります。',
  'agents.modelCount': '{{count}} 個のモデル',
  'agents.connectionFailed': '接続に失敗しました',
  'agents.serverError': 'サーバーエラー {{status}}',
  'agents.failedTo': '{{action}}に失敗しました',
  'agents.failedToMcp': 'MCP サーバーの{{action}}に失敗しました',
  'agents.failedTransport': 'トランスポートの更新に失敗しました',
  'agents.failedMcpTransport': 'MCP トランスポートの更新に失敗しました',
  'agents.claudeCode': 'Claude Code',
  'agents.claudeModels': 'Claude モデル',
  'agents.codexCli': 'Codex CLI',
  'agents.openaiModels': 'OpenAI モデル',
  'agents.opencode': 'OpenCode',
  'agents.opencodeDesc': '75 以上の LLM プロバイダー',
  'agents.copilot': 'GitHub Copilot',
  'agents.copilotDesc': 'GitHub Copilot モデル',
  'agents.mcpServer': 'MCP サーバー',
  'agents.mcpServerStart': '開始',
  'agents.mcpServerStop': '停止',
  'agents.mcpServerRunning': '実行中',
  'agents.mcpServerStopped': '停止中',
  'agents.mcpLanAccess': 'LAN アクセス',
  'agents.mcpClientConfig': 'クライアント設定',
  'agents.stdio': 'stdio',
  'agents.http': 'http',
  'agents.stdioHttp': 'stdio + http',
  'agents.autoUpdate': '自動アップデート確認',
  'agents.notInstalled': '未インストール',
  'agents.install': 'インストール',
  'agents.installing': 'インストール中...',
  'agents.installFailed': 'インストール失敗',
  'agents.viewDocs': 'ドキュメント',

  // ── Figma Import ──
  'figma.title': 'Figma からインポート',
  'figma.dropFile': '.fig ファイルをここにドロップ',
  'figma.orBrowse': 'またはクリックして参照',
  'figma.exportTip':
    'Figma からエクスポート：ファイル \u2192 ローカルコピーを保存 (.fig)',
  'figma.selectFigFile': '.fig ファイルを選択してください',
  'figma.noPages': '.fig ファイルにページが見つかりません',
  'figma.parseFailed': '.fig ファイルの解析に失敗しました',
  'figma.convertFailed': 'Figma ファイルの変換に失敗しました',
  'figma.parsing': '.fig ファイルを解析中...',
  'figma.converting': 'ノードを変換中...',
  'figma.selectPage':
    'このファイルには {{count}} ページあります。インポートするページを選択してください：',
  'figma.layers': '{{count}} レイヤー',
  'figma.importAll': 'すべてのページをインポート',
  'figma.importComplete': 'インポート完了！',
  'figma.moreWarnings': '...他 {{count}} 件の警告',
  'figma.tryAgain': '再試行',
  'figma.layoutMode': 'レイアウトモード：',
  'figma.preserveLayout': 'Figma のレイアウトを維持',
  'figma.autoLayout': 'OpenPencil 自動レイアウト',
  'figma.comingSoon': '近日公開',

  // ── Landing Page ──
  'landing.open': 'Open',
  'landing.pencil': 'Pencil',
  'landing.tagline':
    'オープンソースのベクターデザインツール。Design as Code。',
  'landing.newDesign': '新規デザイン',
  'landing.shortcutHint':
    '{{key1}} + {{key2}} を押して新規デザインを作成',

  // ── 404 ──
  'notFound.message': 'ページが見つかりません',

  // ── Component Browser ──
  'componentBrowser.title': 'UIKit ブラウザ',
  'componentBrowser.exportKit': 'キットをエクスポート',
  'componentBrowser.importKit': 'キットをインポート',
  'componentBrowser.kit': 'キット：',
  'componentBrowser.all': 'すべて',
  'componentBrowser.imported': '（インポート済み）',
  'componentBrowser.components': 'コンポーネント',
  'componentBrowser.searchComponents': 'コンポーネントを検索...',
  'componentBrowser.deleteKit': '{{name}} を削除',
  'componentBrowser.category.all': 'すべて',
  'componentBrowser.category.buttons': 'ボタン',
  'componentBrowser.category.inputs': '入力',
  'componentBrowser.category.cards': 'カード',
  'componentBrowser.category.nav': 'ナビゲーション',
  'componentBrowser.category.layout': 'レイアウト',
  'componentBrowser.category.feedback': 'フィードバック',
  'componentBrowser.category.data': 'データ',
  'componentBrowser.category.other': 'その他',

  // ── Variable Picker ──
  'variablePicker.boundTo': '--{{name}} にバインド済み',
  'variablePicker.bindToVariable': '変数にバインド',
  'variablePicker.unbind': 'バインドを解除',
  'variablePicker.noVariables': '{{type}} 型の変数が定義されていません',
} as const

export default ja

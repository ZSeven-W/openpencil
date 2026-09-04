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
        "promptCenter.item.webKilnform.title" => "Kilnform · デザイン基盤サイト",
        "promptCenter.item.webReefwright.title" => "Reefwright · AI サポートナレッジサイト",
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
        "sceneTemplate.generate.placeholder" => "テーマを入力すると、AI がスライド一式を生成します",
        "sceneTemplate.generate.button" => "生成",
        "sceneTemplate.generate.hint" => "新しいドキュメントを作成し、テーマからスライド一式を生成します。",
        "sceneTemplate.generate.promptTemplate" => "次のテーマでプレゼンテーション（PPT）を作成してください：{{topic}}",
        "sceneTemplate.card.addToCanvas" => "キャンバスに追加",
        "sceneTemplate.card.generateFrom" => "これをもとに生成",
        "sceneTemplate.generate.basis" => "ベース：",
        "sceneTemplate.filter.all" => "すべて",
        "sceneTemplate.scene.tutorial" => "チュートリアル",
        "sceneTemplate.scene.comparison" => "比較",
        "sceneTemplate.scene.carousel" => "カルーセル",
        "sceneTemplate.scene.slides" => "スライド",
        "sceneTemplate.scene.card" => "カード",
        "sceneTemplate.scene.web" => "Web ページ",
        "sceneTemplate.generate.webPromptTemplate" => "次のテーマで、複数セクションからなる Web ランディングページを設計してください：{{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "SaaS ランディングページ · オレンジ",
        "sceneTemplate.item.saasLandingOrange.summary" => "明るい地に黒いパネルを敷き、オレンジを主役にしたマーケティングページ。ナビ、製品スクリーンショット付きヒーロー、機能カード 3 枚、ワークフロー紹介、お客様の声、購読フッターまで。文言を差し替えればそのままサイトになります。",
        "sceneTemplate.item.productLandingLight.title" => "プロダクトページ · ライト",
        "sceneTemplate.item.productLandingLight.summary" => "紙のように白い新聞風のプロダクトページ。操作できるヒーローデモ、機能の段組み、分析ボード、旧方式との比較、3 段階の料金表。SaaS サイトや製品発表に。",
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
        "sceneTemplate.item.knowledgeCardVertical.title" => "ナレッジカード · 縦型",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "見出し・4つの要点・署名欄をまとめた3:4の1枚カード。文言を差し替えるだけで投稿できます。",
        "sceneTemplate.item.knowledgeCardSquare.title" => "ナレッジカード · 正方形",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "同じレイアウトの1:1カード。記事のヘッダー画像やSNS投稿に収まる密度です。",
        "sceneTemplate.item.pitchDeckDark.title" => "ピッチデッキ · ダーク",
        "sceneTemplate.item.pitchDeckDark.summary" => "表紙、課題、ソリューション、数字、ロードマップ、連絡先の6枚。暗い地に大きな文字で、資金調達や発表会向けです。",
        "sceneTemplate.item.lectureDeckLight.title" => "授業スライド · ライト",
        "sceneTemplate.item.lectureDeckLight.summary" => "講義表紙、学習目標、概念解説、例題、比較表、まとめと課題。紙のような白地で、90分見続けても疲れません。",
        "sceneTemplate.item.minimalKeynote.title" => "ミニマル Keynote",
        "sceneTemplate.item.minimalKeynote.summary" => "余白と特大の文字で、1 枚に 1 文を中央揃え。9 枚を通してカードは一つも使わず、目次は罫線と数字だけ。発表会や基調講演向け。",
        "sceneTemplate.item.gradientTech.title" => "グラデーション テック",
        "sceneTemplate.item.gradientTech.summary" => "ダークなグラデーション地にすりガラスのカード。構成図・性能比較・導入企業の枠まで入った開発者向け発表テンプレート。",
        "sceneTemplate.scene.infographic" => "インフォグラフィック",
        "sceneTemplate.item.punchQuoteCard.title" => "パンチライン カード",
        "sceneTemplate.item.punchQuoteCard.summary" => "3:4 の黒地カード。特大の2行と黄色いハイライト帯だけで、伝えたい一言を届ける。意見や名言の投稿向け。",
        "sceneTemplate.item.journalChecklistCard.title" => {
            "チェックリスト カード · ナレッジベース風"
        }
        "sceneTemplate.item.journalChecklistCard.summary" => "淡いグレー地に白いリストカードを一枚。チェックできる5項目、タグ、引用ブロック付き。週の予定や記録に。",
        "sceneTemplate.item.dataReportInfographic.title" => "データ報告インフォグラフィック",
        "sceneTemplate.item.dataReportInfographic.summary" => "縦長のスクロール画像。濃色の見出し帯、3つの大きな数字、横棒の比較、構成比、3行の結論まで。数字を差し替えるだけ。",
        "sceneTemplate.item.stepsFlowInfographic.title" => "手順フロー インフォグラフィック",
        "sceneTemplate.item.stepsFlowInfographic.summary" => "縦長のスクロール画像。番号付きの5ステップを1本の流れにまとめ、所要時間ラベルと2つのヒントを添えた。チュートリアル向け。",
        "sceneTemplate.item.eventPosterDeck.title" => "イベント企画 deck · 告知ポスター",
        "sceneTemplate.item.eventPosterDeck.summary" => "表紙・見どころ・スケジュール・アクセス・チケット・締めの6枚。白い展示壁のような地に赤と青の色面、角丸なしグラデーションなし。マルシェやサークル企画、開店案内に。",
        "sceneTemplate.item.pitfallListInfographic.title" => {
            "落とし穴チェックリスト インフォグラフィック"
        }
        "sceneTemplate.item.pitfallListInfographic.summary" => "縦長のスクロール画像。やりがちな失敗を頻度順に6つ、それぞれ「何がまずいか」と「こう直す」を添え、最後に4行の公開前チェック。色は白黒グレーのみ。",
        "sceneTemplate.item.spineCultureCard.title" => "縦組み背表紙カード · 鉱物顔料",
        "sceneTemplate.item.spineCultureCard.summary" => "赭土の暗い地に縦組みの大見出し、剥落した壁面と顔料の粒。3:4。文化・長文・個人ブランドの表紙に。",
        "sceneTemplate.item.metricSingleCard.title" => "単一指標カード · グリッド漢字",
        "sceneTemplate.item.metricSingleCard.summary" => "純白に大きな数字ひとつ。厳格なスイス・グリッドと赤い信号の四角がひとつだけ。1:1。結論と実績に。",
        "sceneTemplate.item.quoteFrameCard.title" => "引用カード · 絹本青緑",
        "sceneTemplate.item.quoteFrameCard.summary" => "古びた絹の黄色地に、枠で囲んだ一文。裾には石青と石緑の山。4:5。書き抜き・インタビュー・引用に。",
        "sceneTemplate.item.dailySignCard.title" => "日めくりカード · 庭園の窓",
        "sceneTemplate.item.dailySignCard.summary" => "白漆喰の壁に六角の窓をひとつ、その中に日付と一行。余白が装飾。3:4。日めくりとブランドの一言に。",
        "sceneTemplate.item.priceTierCard.title" => "料金カード · アーケードのネオン",
        "sceneTemplate.item.priceTierCard.summary" => "藍色の夜を地に三段の料金表。ネオン管の輪郭と滲む光。1:1。店舗・イベント・セット料金に。",
        "sceneTemplate.item.noticeBoardCard.title" => "お知らせカード · 活版印刷",
        "sceneTemplate.item.noticeBoardCard.summary" => "新聞紙の地に見出し罫の二本線と番号付きの条項。刷りズレと通し番号つき。4:5。告知・注意書き・規則に。",
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "年表インフォグラフィック",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "縦長のスクロール画像。全体を貫く一本の軸に年号の目盛りと出来事カードを並べ、最後は次の一手で締める。振り返り・沿革・プロジェクト史に。",
        "sceneTemplate.item.conceptContrastInfographic.title" => "概念比較インフォグラフィック",
        "sceneTemplate.item.conceptContrastInfographic.summary" => "縦長のスクロール画像。まず結論、次に二つの概念それぞれの定義カード、続いて観点ごとの二段組、最後に選び方。",
        "sceneTemplate.item.rankingBoardInfographic.title" => {
            "TOP N ランキング インフォグラフィック"
        }
        "sceneTemplate.item.rankingBoardInfographic.summary" => "縦長のスクロール画像。墨地に金のおすすめ表。上位三つは大きなバッジ、四位以下は線のバッジ。使いどころと頻度つき。",
        "sceneTemplate.item.faqThreadInfographic.title" => "FAQ インフォグラフィック",
        "sceneTemplate.item.faqThreadInfographic.summary" => "縦長のスクロール画像。六組の一問一答、Q は塗り A は線。番号も順序もなく、どれか一組だけ読んでも成立する。",
        "sceneTemplate.item.dataStoryInfographic.title" => "データストーリー インフォグラフィック",
        "sceneTemplate.item.dataStoryInfographic.summary" => "縦長のスクロール画像。四つの数字を因果の一本線につなぎ、各段は十マスの升目で割合を示し、最後は動かせる結論で締める。",
        "sceneTemplate.item.challengeTrackerInfographic.title" => {
            "30日チャレンジ インフォグラフィック"
        }
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "縦長のスクロール画像。六列五行の三十マス。節目は 7・15・30 日目だけ。保存して一日一マス消していく。",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "業界マップ インフォグラフィック",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => "縦長のスクロール画像。一本の連なりを二×二の四区画に開き、各区画に三つの担い手と空きを記す。スレート地に白カード。",
        "sceneTemplate.item.doDontComparison.title" => "良い例・悪い例の二段組",
        "sceneTemplate.item.doDontComparison.summary" => "3:4 カード。同じことの二つのやり方を左右に並べ、赤と緑ではなく質感とアイコンで見分ける。色覚に配慮した対比。",
        "sceneTemplate.item.mythTruthComparison.title" => "思い込みと実際",
        "sceneTemplate.item.mythTruthComparison.summary" => "縦長画像。「よく言われること／実はこう」を五組、思い込みは細く淡く左に、実際は広く濃く右に。一度に一組ずつ読む。",
        "sceneTemplate.item.pricingTiersComparison.title" => "料金プラン比較",
        "sceneTemplate.item.pricingTiersComparison.summary" => "3:4 カード。無料・Pro・チームの三段を横並びに。価格を起点に読み進め、右の段は左の段を含む。料金ページ向け。",
        "sceneTemplate.item.scenarioGuideComparison.title" => "シーン別の選び方",
        "sceneTemplate.item.scenarioGuideComparison.summary" => "縦長画像。スペックは並べず、七つの状況にそれぞれ判定タグを付ける。読者は自分の行を見つけるだけでいい。",
        "sceneTemplate.item.specTableComparison.title" => "スペック比較表",
        "sceneTemplate.item.specTableComparison.summary" => "縦長画像。二つの候補を一つの表で一行ずつ比較し、勝った側のセルを濃い地で持ち上げる。どこで勝つか一目で分かる。",
        "sceneTemplate.item.threeWayComparison.title" => "三案の横並び比較",
        "sceneTemplate.item.threeWayComparison.summary" => "縦長画像。三つの案を横に並べ、中央がおすすめ。各列の一行目は名前ではなく状況——読者は自分の列を探している。",
        "sceneTemplate.item.timeShiftComparison.title" => "一年前と今",
        "sceneTemplate.item.timeShiftComparison.summary" => "3:4 カード。中央にラベルの背骨を通し、左が一年前、右が今。同じ項目の二つの値が同じ行に並ぶ。",
        "sceneTemplate.item.tradeoffScaleComparison.title" => "メリットとデメリットの天秤",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "1:1 カード。一本の梁に二つの皿、左に「価値」右に「代償」、各行の頭に空のチェックボックス。判断は読者に委ねる。",
        "sceneTemplate.item.versionDiffComparison.title" => "バージョン差分",
        "sceneTemplate.item.versionDiffComparison.summary" => "1:1 カード。左右に分けず、各行がそれ自体で「旧 → 新」を完結させる。上から下へ読むだけでいい。",
        "sceneTemplate.item.appOnboardingTriptych.title" => "アプリ オンボーディング三面",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "3:4 カード。並んだ三台のスマホに空の画像枠。自分の三枚を入れて文言を添えれば、そのままレビューにも投稿にも使える。",
        "sceneTemplate.item.diyBlueprintGuide.title" => "DIY 図解ガイド",
        "sceneTemplate.item.diyBlueprintGuide.summary" => {
            "縦長画像。材料と寸法の表が手順と同じだけの紙面を取る。DIY は手ではなく準備で失敗する。"
        }
        "sceneTemplate.item.photoCompositionTutorial.title" => "スマホ写真の構図レッスン",
        "sceneTemplate.item.photoCompositionTutorial.summary" => "3:4 五枚。各ページは暗いファインダー枠に蛍光のガイド線。構図はフレームの上に描かないと説明できない。",
        "sceneTemplate.item.recipeFourStep.title" => "四手順レシピカード",
        "sceneTemplate.item.recipeFourStep.summary" => "4:5 カード、2×2。四手順を一枚に収める。スクショして見ながら作れる——コンロの前でページはめくれない。",
        "sceneTemplate.item.skincareRoutineCards.title" => "スキンケア手順カード",
        "sceneTemplate.item.skincareRoutineCards.summary" => "4:5 六枚。各手順に必ず三つの数字：量・待ち時間・朝夜。失敗は順番ではなく量と間隔で起きる。",
        "sceneTemplate.item.softwareStepTutorial.title" => "ソフト操作手順カード",
        "sceneTemplate.item.softwareStepTutorial.summary" => "4:5 カード。教程枠で唯一のダーク。画面キャプチャ枠と番号付きの操作説明。ツールや機能紹介に。",
        "sceneTemplate.item.storageMakeoverSteps.title" => "収納リフォームの手順",
        "sceneTemplate.item.storageMakeoverSteps.summary" => "3:4 六枚。動作と画像枠に加え、各手順に完了条件と所要時間を必ず添える。その状態になったら次へ進める。",
        "sceneTemplate.item.weeklyReportLesson.title" => "週報の書き方レッスン",
        "sceneTemplate.item.weeklyReportLesson.summary" => "縦長画像。四段構成を説明したあと、下線の空欄が入った週報の骨組みを渡す。スクショして埋めるだけ。",
        "sceneTemplate.item.workoutBreakdownGuide.title" => "トレーニング動作分解",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "縦長画像。各種目に画像枠とポイントのほか、セット数・回数・休憩の固定フォーマット帯を付ける。",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => "書評・映画評の分解カルーセル",
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4 五枚。フック、注釈付きの引用、三つの洞察、心に残る一文、締め。あらすじの再話ではなく、持ち帰れる部品に分解する。",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "シティガイド カルーセル",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => "3:4 七枚。場所と動線を交互に——場所の枚は夢を見る読者へ、一日の動線と食と宿の対照は計画する読者へ。",
        "sceneTemplate.item.datareportGridCarousel.title" => "データレポート カルーセル",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4 六枚。データ面の前後に必ず非データ面を挟み、三枚目のグラフで離脱させない。四半期報告や業界観察に。",
        "sceneTemplate.item.opinionLongformCarousel.title" => "論考カルーセル",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4 六枚。厳密な視覚マスターを全編に通し、ページ番号と見出しは常に同じ位置。前の板は戻れないから一貫性は必須。",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "Q&A カルーセル",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => {
            "3:4 六枚。一問一枚、各面の隅に手書きの疑問符番号。問いそのものが次へ進む理由になる。"
        }
        "sceneTemplate.item.storyNightCarousel.title" => "ストーリー カルーセル",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4 七枚。時間を骨格にした個人の振り返り。五枚目の年表が全体の耐力壁で、前の四枚はその目盛りの拡大。",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => "ツール集カルーセル",
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4 六枚。六つのツールを一枚ずつ、最後の一枚にページ番号付きの目次。合集の読者の目的は保存だけ。",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "チュートリアル カルーセル",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => {
            "3:4 六枚。一枚一手順、指が進捗バー。手作り・ソフト・暮らしの手順に。"
        }
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "年間振り返り カルーセル",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => {
            "3:4 八枚。数字の面は冷たく、思いの面は温かく、交互に進む。年末のまとめや個人の総括に。"
        }
        "fileMenu.newFromTemplate" => "テンプレートから新規作成",
        "fileMenu.exportSlideshowHtml" => "スライドショー HTML を書き出し...",
        "fileMenu.exportPptx" => "PowerPoint を書き出し...",
        "dialog.slideshowHtmlTitle" => "スライドショーを書き出し",
        "dialog.slideshowHtmlSummary" => "{{count}} 枚のスライドを次の場所に書き出しました:",
        "dialog.slideshowHtmlEmpty" => "このプレゼンテーションには書き出せるスライドがありません。",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "インポート可能な HTML コンテンツは利用できません。",
        "htmlImport.warn.content.empty_body" => "HTML 本文内にインポート可能なコンテンツは利用できません。",
        "htmlImport.warn.content.dom_depth_truncated" => "{{max_depth}} 階層より深くネストした HTML を破棄しました。",
        "htmlImport.warn.content.node_limit_truncated" => "ノード上限に達したため、残りのページ内容を省略しました。",
        "htmlImport.warn.content.node_limit_mapping" => "ノード上限に達したため、HTML ツリーの一部を省略しました。",
        "htmlImport.warn.content.node_limit_inline_row" => "ノード上限に達したため、インライン整形行を省略しました。",
        "htmlImport.warn.content.node_limit_pseudo" => "ノード上限に達したため、生成された擬似要素を省略しました。",
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "{{max_depth}} 層より深いアット規則にネストした CSS 規則を無視しました。"
        }
        "htmlImport.warn.css.unterminated_rule" => "終端のない CSS 規則を無視しました。",
        "htmlImport.warn.css.marker_rules_unsupported" => "CSS ::marker 規則をインポートしませんでした。",
        "htmlImport.warn.css.nesting_unsupported" => "ネストした CSS スタイル規則を無視しました。",
        "htmlImport.warn.css.invalid_layer_name" => "無効な @layer 名 '{{name}}' を無視しました。",
        "htmlImport.warn.css.unsupported_statement" => "未対応の @{{name}} 文を無視しました。",
        "htmlImport.warn.css.media_without_viewport" => "ビューポートのない @media 規則を無視しました。",
        "htmlImport.warn.css.invalid_layer_block_name" => "無効な @layer ブロック名 '{{name}}' を無視しました。",
        "htmlImport.warn.css.unsupported_container_block" => "@container ブロックを無視しました。",
        "htmlImport.warn.css.unsupported_block" => "未対応の @{{name}} ブロックを無視しました。",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "@font-face のウェブフォント '{{family}}' は利用できません。"
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "絶対配置された要素のパーセント指定オフセットを近似しました。"
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "パーセント指定の position:relative オフセットを近似しました。"
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "確定した軸のない CSS aspect-ratio を無視しました。"
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "サイズ不確定の包含ブロック内の CSS aspect-ratio を無視しました。"
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky を無視しました。",
        "htmlImport.warn.layout.grid_tracks_approximated" => "未対応の CSS グリッドトラックを近似しました。",
        "htmlImport.warn.layout.float_ignored" => "CSS float を無視しました。",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "ノード単位の CSS mix-blend-mode を近似しました。"
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => "CSS overflow: auto / scroll を近似しました。",
        "htmlImport.warn.layout.negative_margins_ignored" => "負の CSS マージンを無視しました。",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => "視覚ボックス上の CSS マージンを無視しました。",
        "htmlImport.warn.layout.inline_margin_wrapping_approximated" => "CSS マージン付きのインライン要素をボックス化したため、行をまたいで折り返せない場合があります。",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box のパーセント指定サイズを近似しました。"
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => "明示的な開始ラインで生じた空の CSS グリッドセルを近似しました。",
        "htmlImport.warn.layout.grid_span_reflowed" => "開始ラインに収まらないスパンを持つ CSS グリッド項目を近似しました。",
        "htmlImport.warn.layout.grid_rows_node_limit" => "ノード上限に達したため、CSS グリッドの行ラッパーを省略しました。",
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "auto-fit / auto-fill を使う CSS グリッドのトラック幅を近似しました。"
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas による配置をインポートしませんでした。"
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => "CSS grid-row による配置をインポートしませんでした。",
        "htmlImport.warn.layout.grid_column_unsupported" => "CSS grid-column `{{value}}` を近似しました。",
        "htmlImport.warn.layout.block_auto_margins_ignored" => "ブロック軸方向の CSS 自動マージンをインポートしませんでした。",
        "htmlImport.warn.layout.auto_margin_node_limit" => "ノード上限に達したため、CSS 自動マージンによる配置を省略しました。",
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "サイズが確定しない要素のフロー内 CSS オフセットを破棄しました。"
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => "ノード上限に達したため、フロー内 CSS オフセットを省略しました。",
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "フロー内 CSS オフセット（position:relative のインセット、transform の平行移動）を近似しました。"
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "オフセット用ラッパーを持てないボックスのフロー内 CSS オフセットを破棄しました。"
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "列方向の flex コンテナーの flex-wrap をインポートしませんでした。"
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "flex-wrap:wrap-reverse を近似しました。",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => "幅が確定しないコンテナーの flex-wrap を無視しました。",
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "折り返す flex コンテナーの CSS align-content をインポートしませんでした。"
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "子要素の主軸サイズが不確定な flex-wrap を無視しました。"
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => "ノード上限に達したため、flex-wrap の行を省略しました。",
        "htmlImport.warn.transform.unsupported_syntax" => "未対応の CSS transform 構文を無視しました。",
        "htmlImport.warn.transform.unsupported_function" => {
            "未対応の CSS transform 関数（3D、matrix3d）を無視しました。"
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "不確定な軸に対するパーセント指定の CSS transform 平行移動を破棄しました。"
        }
        "htmlImport.warn.transform.non_finite_matrix" => "非有限の行列を生じる CSS transform を無視しました。",
        "htmlImport.warn.transform.skew_dropped" => "CSS transform の skew を破棄しました。",
        "htmlImport.warn.transform.degenerate_scale" => "拡大率がゼロまたは非有限の CSS transform を近似しました。",
        "htmlImport.warn.transform.mirroring_absolute" => "CSS transform の鏡像反転を近似しました。",
        "htmlImport.warn.transform.origin_z_ignored" => "CSS transform-origin の Z オフセットを無視しました。",
        "htmlImport.warn.transform.scale_not_baked" => {
            "ノードサイズに焼き込めなかった CSS transform の拡大縮小を破棄しました。"
        }
        "htmlImport.warn.transform.scale_baked" => "ノードサイズに焼き込んだ CSS transform の拡大縮小を近似しました。",
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "自動サイズの要素に対する CSS transform の拡大縮小を無視しました。"
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "方向指定または間隔付きの CSS background-repeat を近似しました。"
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => "明示的な CSS 背景タイルサイズを無視しました。",
        "htmlImport.warn.visual.background_size_auto_box" => {
            "自動サイズの要素に対する CSS background-size を近似しました。"
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "画像の固有サイズを必要とする CSS background-size を近似しました。"
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "未対応の CSS background-position を無視しました。"
        }
        "htmlImport.warn.visual.background_image_url_empty" => "空の CSS 背景画像 URL を無視しました。",
        "htmlImport.warn.visual.conic_gradient_ignored" => "CSS の円錐グラデーションを無視しました。",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "未対応の CSS background-image レイヤーを無視しました。"
        }
        "htmlImport.warn.visual.background_color_unresolved" => "解決できない CSS 背景色を無視しました。",
        "htmlImport.warn.visual.background_position_dropped" => "CSS background-position を無視しました。",
        "htmlImport.warn.visual.border_colors_approximated" => "辺ごとの CSS ボーダー色を近似しました。",
        "htmlImport.warn.visual.border_styles_approximated" => "辺ごとに異なる CSS ボーダースタイルを近似しました。",
        "htmlImport.warn.visual.border_style_complex" => "複雑な CSS ボーダースタイルを近似しました。",
        "htmlImport.warn.visual.border_style_unsupported" => "未対応の CSS ボーダースタイルを近似しました。",
        "htmlImport.warn.visual.border_radius_elliptical" => "楕円形の CSS 角丸半径を近似しました。",
        "htmlImport.warn.visual.border_radius_unsupported" => "未対応の CSS 角丸半径を無視しました。",
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => "未対応の CSS box-shadow レイヤーを無視しました。",
        "htmlImport.warn.visual.gradient_interpolation_ignored" => "CSS グラデーションの色補間方式を無視しました。",
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "未対応の CSS linear-gradient の方向を無視しました。"
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => "CSS グラデーションの色ヒントを無視しました。",
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => "未対応の CSS グラデーションの色停止点を無視しました。",
        "htmlImport.warn.visual.gradient_too_few_stops" => "使用可能な停止点が 2 つ未満の CSS グラデーションを無視しました。",
        "htmlImport.warn.visual.gradient_repeating_approximated" => "繰り返しの CSS グラデーションを近似しました。",
        "htmlImport.warn.visual.gradient_stops_clamped" => "範囲外の CSS グラデーション停止点を近似しました。",
        "htmlImport.warn.visual.blur_radius_unsupported" => "未対応の CSS ぼかし半径を無視しました。",
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "未対応の CSS filter の drop-shadow() を無視しました。"
        }
        "htmlImport.warn.visual.filter_function_unsupported" => "未対応の CSS filter 関数を無視しました。",
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "未対応の CSS backdrop-filter 関数を無視しました。"
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "未対応の CSS background-blend-mode を無視しました。"
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => "個々の塗りに対する CSS mix-blend-mode を近似しました。",
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => "未対応の CSS mix-blend-mode を無視しました。",
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} を無視しました。",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "グラデーションに対する CSS background-size を無視しました。"
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "未対応の CSS radial-gradient の位置を無視しました。"
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => "楕円形の CSS radial-gradient を近似しました。",
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "CSS radial-gradient の範囲キーワードを近似しました。"
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "未対応の CSS radial-gradient のサイズを無視しました。"
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => "未対応の CSS text-shadow レイヤーを無視しました。",
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "2 つ目以降の CSS text-shadow レイヤーを無視しました。"
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => "インライン要素の CSS text-shadow を無視しました。",
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image をインポートしませんでした。",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "`list-style-position: outside` のぶら下げマーカーを近似しました。"
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "未対応の CSS list-style-type `{{value}}` を近似しました。"
        }
        "htmlImport.warn.media.object_fit_scale_down" => "CSS object-fit:scale-down を近似しました。",
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none を無視しました。",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position を無視しました。",
        "htmlImport.warn.media.image_intrinsic_axis_unresolved" => {
            "指定サイズが動的であるか、包含ブロックのサイズが未確定なため、画像の固有アスペクト比から不足している軸を決定できませんでした。"
        }
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "画像に対する未対応の CSS mix-blend-mode を無視しました。"
        }
        "htmlImport.warn.media.inline_svg_placeholder" => "インラインの <svg> 要素をプレースホルダーとしてインポートしました。",
        "htmlImport.warn.media.input_type_fallback" => "未対応の <input> の種類を近似しました。",
        "htmlImport.warn.media.element_placeholder" => "<{{tag}}> 要素をプレースホルダーとしてインポートしました。",
        "htmlImport.warn.media.picture_undecodable_types" => {
            "デコードできない種類のソースのみを持つ <picture> を近似しました。"
        }
        "htmlImport.warn.table.rowspan_ignored" => "HTML の rowspan 属性をインポートしませんでした。",
        "htmlImport.warn.table.row_groups_unflattened" => "CSS が行グループの平坦化を解除した表の列幅を近似しました。",
        "htmlImport.warn.table.indefinite_width_approximated" => "幅が確定しない CSS 表の列幅を近似しました。",
        "htmlImport.warn.resource.invalid_base_href" => "無効な <base href> {{href}} を無視しました。",
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "プロジェクトのオリジン外の <base href> {{href}} を無視しました。"
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => "外部スタイルシート {{url}} は利用できません。",
        "htmlImport.warn.resource.image_outside_origin" => {
            "プロジェクトのオリジン外の画像 {{url}} をプレースホルダーとしてインポートしました。"
        }
        "htmlImport.warn.resource.image_unavailable" => "利用できない画像 {{url}} をプレースホルダーとしてインポートしました。",
        "htmlImport.warn.resource.css_import_invalid" => "無効な CSS @import {{prelude}} を無視しました。",
        "htmlImport.warn.resource.css_import_unresolvable" => "CSS @import {{reference}} は利用できません。",
        "htmlImport.warn.resource.css_import_cycle" => "循環する CSS @import {{url}} を無視しました。",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "深さ {{max_depth}} を超える CSS @import {{url}} を無視しました。"
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} は利用できません。",
        "htmlImport.warn.project.multiple_html_entries" => {
            "HTML エントリーが {{count}} 件見つかりました。{{entry}} を選択し、残りを近似しました。"
        }
        "htmlImport.warn.snapshot.truncated" => "ブラウザースナップショットの一部を破棄しました。",
        "htmlImport.warn.snapshot.node_limit" => "ノード上限に達したため、残りのスナップショット内容を省略しました。",
        "htmlImport.warn.snapshot.tainted_images" => {
            "CORS で汚染された画像 {{count}} 件はリモート URL のまま保持され、利用できません。"
        }
        "htmlImport.warn.snapshot.invalid_rect" => "矩形が欠落または無効なスナップショットノードを破棄しました。",
        "htmlImport.warn.snapshot.unknown_kind" => "種類が不明なスナップショットノードを破棄しました。",
        "htmlImport.warn.snapshot.rejected" => "ブラウザースナップショット（{{reason}}）を破棄しました。",
        "htmlImport.warn.snapshot.unsupported_transform" => "未対応のスナップショット変換を無視しました。",
        "htmlImport.warn.css.media_empty_query" => "空の @media クエリを無視しました。",
        "htmlImport.warn.css.media_unsupported_type" => "未対応の @media タイプ '{{name}}' を無視しました。",
        "htmlImport.warn.css.media_unsupported_condition" => "未対応の @media 条件 '{{input}}' を無視しました。",
        "htmlImport.warn.css.media_invalid_orientation" => "無効な @media の向き '{{value}}' を無視しました。",
        "htmlImport.warn.css.media_unsupported_feature" => "未対応の @media 特性 '{{name}}' を無視しました。",
        "htmlImport.warn.css.media_unsupported_range" => "未対応の @media 範囲 '({{input}})' を無視しました。",
        "htmlImport.warn.css.media_invalid_range" => "無効な @media 範囲 '({{input}})' を無視しました。",
        "htmlImport.warn.css.media_invalid_length" => "無効な @media の長さ '{{value}}' を無視しました。",
        "htmlImport.diagnostics.title" => "HTML インポート完了",
        "htmlImport.diagnostics.summary" => "劣化した項目：{{count}}",
        "htmlImport.diagnostics.dismiss" => "閉じる",
        "htmlImport.diagnostics.expand" => "詳細を表示",
        "htmlImport.diagnostics.collapse" => "詳細を隠す",
        "htmlImport.diagnostics.more" => "他 {{count}} 件",
        "dialog.pptxTitle" => "PowerPoint を書き出し",
        "dialog.pptxSummary" => "{{count}} 枚のスライドを次の場所に書き出しました:",
        "dialog.pptxEmpty" => "このプレゼンテーションには書き出せるスライドがありません。",
        "settings.agents.acpQuickAdd" => "クイック追加",
        "settings.agents.acpPresetAdd" => "追加",
        "settings.agents.acpNotInstalled" => "未インストール",
        "assetCenter.title" => "アセットセンター",
        "assetCenter.tab.templates" => "テンプレート",
        "assetCenter.tab.styles" => "スタイル",
        "assetCenter.style.empty" => "一致するスタイルがありません",
        "assetCenter.style.pinned" => "ピン留め中",
        "assetCenter.style.searchPlaceholder" => "スタイルやタグを検索",
        "assetCenter.style.generateHint" => "新しいドキュメントをトピックから生成します。ピン留めしたスタイルが使われます。",
        "ai.pinnedStyle" => "スタイル：{{name}}",
        "assetCenter.style.import" => "スタイルを読み込む",
        "assetCenter.style.mine" => "マイスタイル",
        "assetCenter.style.builtIn" => "組み込みスタイル",
        "assetCenter.style.importTitle" => "DESIGN.md を読み込む",
        "assetCenter.style.importHint" => "DESIGN.md の全文を貼り付けてから、読み込みを確定してください。",
        "assetCenter.style.importSource" => "styles.refero.design などの DESIGN.md ライブラリから内容をコピーできます。",
        "assetCenter.style.importConfirm" => "読み込む",
        "assetCenter.style.importCancel" => "キャンセル",
        "assetCenter.style.importPickFile" => "ファイルを選択…",
        "assetCenter.style.importHintFile" => "DESIGN.md ファイルを選ぶか、下に全文を貼り付けてください。",
        "assetCenter.style.importPlaceholder" => "ここに DESIGN.md を貼り付け",
        "assetCenter.style.importEmpty" => "このファイルは空か、スタイルガイドとしては短すぎます。",
        "assetCenter.style.importNotText" => "このファイルは Markdown テキストとして読めません。",
        "assetCenter.style.importTooLarge" => "このファイルは 512 KB を超えています。",
        "slidesPanel.tabSlides" => "スライド",
        "slidesPanel.tabCards" => "カード",
        "slidesPanel.present" => "再生",
        "slidesPanel.exportPdf" => "PDF を書き出し",
        "slidesPanel.exportAllSlides" => "すべてのスライドを書き出し",
        "slidesPanel.exportSelectedSlides" => "選択したスライドを書き出し（{{count}}）",
        "settings.tab.ai" => "AI",
        "settings.agents.heroTitle" => "AI プロバイダーを接続",
        "settings.agents.heroSubtitle" => "OpenPencil はローカルの CLI エージェントと API プロバイダーを直接動かします。いずれかを接続するとデザイン生成を開始できます。",
        "settings.agents.statusConnected" => "接続済み",
        "settings.agents.statusNotConnected" => "未接続",
        "settings.agents.statusChecking" => "状態を確認中…",
        "settings.mcp.heroTitle" => "外部から MCP で OpenPencil に接続",
        "settings.mcp.heroSubtitle" => "MCP に対応した CLI やエディターをこのワークスペースに向けるだけで、内蔵エージェントと同じツールでキャンバスを操作できます。",
        "settings.mcp.terminalFootnote" => "* 起動時に、選択した CLI ツールへ MCP が自動設定されます。",
        "settings.mcp.customConfigTitle" => "カスタム MCP サーバー設定",
        "settings.mcp.customConfigDesc" => "標準の MCP server ブロックを読むクライアントにそのまま貼り付けてください。",
        "settings.mcp.copyConfig" => "MCP 設定をコピー",
        "settings.system.heroTitle" => "システム設定",
        "settings.system.heroSubtitle" => "このインストールの外観・更新・キャンバス動作。",
        "settings.system.appearance" => "外観",
        "settings.system.appearanceLight" => "ライト",
        "settings.system.appearanceDark" => "ダーク",
        "settings.system.pencilCursor" => "ペンカーソル",
        "settings.images.heroTitle" => "デザインに使う画像",
        "settings.images.heroSubtitle" => "Openverse で写真を検索するか、プロバイダーを接続して必要に応じて生成します。",
        "settings.fonts.heroTitle" => "このドキュメントのフォント",
        "settings.fonts.heroSubtitle" => "ドキュメントが要求していてこの端末にないフォントを解決し、読み込んだフォントを管理します。",
        "settings.account.heroTitle" => "アカウント",
        "settings.account.heroSubtitle" => "サインインすると、ワークスペースとライセンスを複数の端末で同期できます。",
        "tooltip.topbar.file" => "ファイル",
        "tooltip.topbar.import" => "インポート",
        "tooltip.topbar.language" => "言語",
        "tooltip.topbar.collaboration" => "コラボレーション",
        "tooltip.topbar.preview" => "プレビュー",
        "tooltip.topbar.exitPreview" => "プレビューを終了",
        "tooltip.topbar.account" => "アカウント",
        "settings.agents.providerRollMore" => "他 {{count}} 社",
        "ai.thinking.adaptive" => "思考: 自動",
        "ai.thinking.disabled" => "思考: オフ",
        "ai.thinking.enabled" => "思考: オン",
        "ai.designProgress.detail.repairsApplied" => "{{count}} 件の自動修正を適用",
        "ai.designProgress.detail.repairsMore" => "…他 {{count}} 件(ログ参照)",
        "ai.styleCard.builtin" => "組み込みスタイル",
        "ai.styleCard.imported" => "インポートした DESIGN.md",
        "ai.styleCard.documentDesignMd" => "ドキュメントの design.md",
        _ => return super::ja_collab::lookup(key),
    })
}

<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>OpenPencil 用の DeepSeek Harness プラグイン — 会話の中で実際の <code>.op</code> ドキュメントをプレビュー、検査、編集できます。</strong><br />
  <sub>正確なマルチフレームプレビュー &bull; インタラクティブキャンバス &bull; マネージドエディター &bull; エージェントネイティブなデザインツール</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · 現在のプラグインリリース: <code>0.1.0-rc.7</code> · DSH <code>0.1.1-rc.2</code> までテスト済み</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md"><b>日本語</b></a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><img src="https://img.shields.io/npm/v/%40zseven-w%2Fdsh-openpencil?style=flat&color=cfb537" alt="npm" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/dsh-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/dsh-check.yml?label=CI" alt="CI" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil — マルチフレームプレビューとサイドバーエディター" width="100%" />
</p>
<p align="center"><sub>インタラクティブキャンバスとマネージドエディターワークベンチによる正確なマルチフレーム <code>.op</code> プレビュー</sub></p>

## DSH OpenPencil の特長

DSH OpenPencil は [DeepSeek Harness](https://github.com/deepseek-ai/DSH) と [OpenPencil](https://github.com/ZSeven-W/openpencil) を連携させ、生成された画像を返すのではなく、エージェントが実際に編集可能でインタラクティブなデザインキャンバスを操作できるようにします。

<table>
<tr>
<td width="50%">

### 🖼️ 正確なマルチフレームプレビュー

インストール済みの OpenPencil ヘッドレスエクスポーターがデザインを忠実に再現したプレビューをレンダリングします。最初のトップレベルフレームは大きなリプレイ安全な PNG として、さらに水平スクロール可能なサムネイルレール、クリックでの選択、マルチフレームドキュメント用の前後ナビゲーションを備えています。

</td>
<td width="50%">

### 🗺️ インタラクティブキャンバス

「インタラクティブキャンバスを開く」は、パン・ズーム・フィット機能を持つ読み取り専用の OpenPencil Web SDK を遅延マウントします。会話から離れることなく、任意のページ、ネストされたノード、非アクティブなページを検査できます。

</td>
</tr>
<tr>
<td width="50%">

### ✏️ マネージドエディター

`editable: true` を指定すると、編集アクションがマネージド OpenPencil エディター（選択、レイヤー、プロパティ、描画ツール、元に戻す/やり直し、明示的な保存セマンティクス）を開きます。リサイズ可能な右側のワークベンチで表示され、全画面オプションも利用できます。

</td>
<td width="50%">

### 🤖 エージェントネイティブなデザインツール

5つの直接操作ツールと6つの `openpencil_pipeline_*` ツールにより、エージェントはマネージド OpenPencil ランタイム上で実際のキャンバスを作成、検査、洗練、公開、変更、読み取りできます。

</td>
</tr>
<tr>
<td width="50%">

### 🔐 ケーパビリティで制御された認可

画像とドキュメントの認可は、署名付きでハッシュにバインドされたケーパビリティです。ブラウザのメタデータが任意のホストパスを公開することは決してなく、署名付きのプレビュー/エディターのケーパビリティが正規のツール結果やモデルコンテキストに入ることもありません。

</td>
<td width="50%">

### ⚡ トランザクション安全性

フルパイプラインのドキュメントは、すべてのネイティブ品質ゲートと DSH 品質ゲートを通過するまで、非公開のプライベートドラフトに保持されます。公開時に既存パスを上書きせず、中止や失敗したバッチが空のターゲットを残すこともありません。

</td>
</tr>
<tr>
<td width="50%">

### 🌍 DSH の外観と操作感に準拠

ツールカードとマネージドエディターは、編集セッションをリロードすることなく、DSH の中国語/英語ロケールとライト/ダークテーマに追従します。

</td>
<td width="50%">

### 🎯 ひとつの完全なワークフロー

「要件 → サイドバーの private live canvas → 正確な PNG をユーザーへ示す2つの意味単位バッチ → ネイティブ／DSH の決定論的品質ゲート → アトミック公開」— DSH 内で完結するひとつのループです。

</td>
</tr>
</table>

## DSH へのインストール

DSH は別パッケージです。未導入なら一度インストールします:

```sh
npm install -g @deepseek-ai/dsh@latest
```

次にプラグインをプロファイルへ追加し、Web アプリを起動します:

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@next
dsh web
```

ローカル開発では、このチェックアウトをビルドし、その絶対パスを Web プロファイルへリンクしてから DSH を完全に再起動します:

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

`link:` 依存関係により以後の再ビルドがこのチェックアウトから反映されます。ただし、同梱の Web プロファイルはホストバンドルを既定でホットリロードしないため、プロファイル依存関係を置き換えた後は DSH の完全な再起動が必要です。

DSH をグローバルに入れたくない場合は、同じ 2 ステップを `pnpm dlx` で実行します:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@next
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> OpenPencil プラグインは公開されており、npm トークンは不要です。DSH プレリリース自体にレジストリ認証が必要な場合は、その認証情報をチェックアウト外のユーザーレベルまたは一時的な npm 設定に保持してください。このリポジトリには意図的にレジストリの認証情報が含まれていません。

## デザインツール

任意のスタイル選択は `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin` の順です。元の brief と `style_selection` を渡します。カタログと指定ガイドはモデル推薦なしで利用でき、推薦プロバイダーは既定で無効です。この引き渡しは script のみ対応し、app-agent とコマースは非対応です。通常の app-agent 生成は利用できます。

| ツール | 機能 |
| --- | --- |
| `openpencil_new` | 単純な作業向けの互換高速パスです。1つのトランザクション型 QuickJS `batch_design` スクリプトを実行し、存在しない場合にのみ公開して編集可能なプレゼンテーションを返します。本番品質のデザインには以下のフルパイプラインを優先してください。 |
| `openpencil_pipeline_begin` | 所有セッション専用のプライベートドラフトと唯一の root を開始し、同じ live canvas をサイドバーですぐに開きます。ターゲット `.op` は未公開のままです。 |
| `openpencil_pipeline_context` | begin contract に本当に不足している guideline、style、theme、UI kit の詳細を1件だけ限定的に読み込みます。起動時の refresh loop ではありません。 |
| `openpencil_pipeline_batch` | 直接 QuickJS の生成スクリプトを最大2本実行します。成功した各トランザクションはユーザー向けの正確な PNG 表示だけを試み、ツールが返ったら直ちに `next` に従います。 |
| `openpencil_pipeline_inspect` | ユーザーが明示的に要求した場合にのみ手動診断を提供します。通常の生成ではプレビューやモデルの画像確認ステップとして使用しません。 |
| `openpencil_pipeline_finish` | ネイティブと DSH の決定論的ゲートを実行し、最終化後の正確な PNG をレンダリングして `createIfAbsent` でアトミックに公開します。`needs_correction`、`canContinue: true`、完全で空でない `repairTargets`、`omitted: 0` がすべて揃った場合に限り、U-only 修復1回と最後の finish 1回を許可します。それ以外の未公開結果は terminal です。 |
| `openpencil_pipeline_abort` | ターゲットファイルを作成せず、未公開ドラフトを破棄します。 |
| `openpencil_create` | 既存のライブキャンバス上でノードを生成・再構築するために、トランザクション型 `batch_design` プログラムを適用します。 |
| `openpencil_edit` | 明示的なノード、またはユーザーが選択した単一のノードを変更します。 |
| `openpencil_render` | 不変でコンテンツアドレス型の `.op` スナップショットを作成し、アクティブページ上のすべてのトップレベルフレームをレンダリングします — オプションの `scale` と `editable` 付き。 |
| `openpencil_selection` | ライブエディターのキャンバスで選択されている正確なノードを読み取ります。 |

## エージェントのデザインワークフロー

通常の生成では固定の短い経路を使います。`openpencil_pipeline_begin` → 直接 QuickJS の `openpencil_pipeline_batch` を2回 → `openpencil_pipeline_finish` を1回です。Begin は唯一の root を作成し、プライベートな live canvas をサイドバーですぐに開きます。公開が成功するまで要求したワークスペースパスは存在しません。begin または batch が成功するたびに、説明、計画、比較、検査、無関係なツール呼び出しを挟まず、次の必須呼び出しを直ちに行います。既定ではデザイン全体で画像は正確に1枚とし、各 Hero/Product/Art/Media frame の主ビジュアルも1つだけにして、画像とプレースホルダーアイコンを併置しません。

成功した2つのバッチはいずれも、ユーザー向けに正確な PNG プレビューを表示しようとします。ツールが返ったら直ちに `next` に従ってください。`next` が `previewUnavailable` を返しても、スクリプトはすでに live canvas にコミット済みです。バッチを再実行せず、`openpencil_pipeline_inspect` や `read_image` も呼び出さないでください。`openpencil_pipeline_inspect` はユーザーが明示的に要求した診断専用です。

Finish は OpenPencil ネイティブの最終化、lint、コントラスト、レイアウト検査と DSH の決定論的品質ゲートを実行し、正常な同じ呼び出しで正確な最終 PNG をレンダリングしてターゲットをアトミックに公開します。修復を許可するのは、結果に `stage: "needs_correction"`、`canContinue: true`、完全で空でない `repairTargets` と `omitted: 0` が同時にあり、各 target に `operation: "U"`、正確で空でない `nodeId`、空でない `patch` がある場合だけです。全 target を1つの U-only batch でまとめて適用し、説明を挟まず finish を最後に1回だけ呼びます。それ以外の未公開結果、error、`canContinue: false` は terminal です。1回だけ報告し、retry、inspect、image/context の読み取り、abort、代替 draft の開始は行いません。ゲート失敗または `openpencil_pipeline_abort` ではターゲットは作成されません。published result の正確な最終 PNG と live editor はすでに authoritative なので、それを返して直ちに終了します。サイドバーは idle のときだけ自動で開き、明示的な切り替え用に **キャンバスを編集** が常に残ります。

同じ実行中の DSH サービス内では、ブラウザーの切り替えや再読み込み後も、厳密に解析された `openpencil_new` または `openpencil_pipeline_finish` の永続 publication を、正確な PNG と明示的な **キャンバスを編集** 操作として復元できます。履歴カードがサイドバーを自動で開くことはなく、ユーザーがその操作をクリックする必要があります。通常の履歴 `openpencil_render` は読み取り専用のままで、非 loopback 接続にはエディター権限を発行しません。

同梱の `openpencil-design` skill は引き続きスクリプトと品質のガイドを提供し、マネージドランタイムはデスクトップバイナリに依存しません。`openpencil_new` は互換性のある単一バッチ高速パスとして残りますが、本番品質の生成にはフルパイプラインを優先してください。

`openpencil_create` と `openpencil_edit` は、既存のライブキャンバスに対してのみ使用します。これらの編集は、エディターの保存アクションが実行されるまで未保存のままです。

## Web ビューアーアセット

DSH はクライアントプラグインに `client.js` のみを配信するため、OpenPencil ESM SDK、その WASM、CanvasKit は明示的な同一オリジンのアセットとしてステージングされます:

```sh
pnpm run sync:viewer-assets
```

同期コマンドは OpenPencil モノレポのルート（`../..`）を優先使用し、隣接する `../openpencil` チェックアウトにフォールバックします。`OPENPENCIL_ROOT` または `--openpencil-root` で上書きできます。ビルド済みの完全なアセットディレクトリは `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE` で選択できます。ランタイムの参照先は `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR` で上書きできます。

ビューアーアセットは、ユーザーがキャンバスを開いた後にのみ遅延ロードされます。それらが存在しないか無効な場合、PNG プレビューは引き続き利用可能で、キャンバスボタンは表示されません。

## マネージドエディター

編集可能なセッションは OpenPencil のマネージド Web ホストを使用します — `op-vscode` と同じアーキテクチャです。プラグインは、認可されたユーザー操作の後にのみホストを起動し、デーモントークンをメモリ内に保持し、iframe のソースとオリジンを検証し、エディターセッションが終了するとプロセスを閉じます。エディターのサーフェスは段階的に選択されます。ホストがそのシームを宣言する場合はネイティブの Tool 詳細、それ以外の場合はリサイズと全画面コントロールを備えたプラグインの右側ワークベンチです。

起動時は低速マウントにも安全な listening handshake を使い、同梱ホストがバインド済みアドレスを通知してから readiness probe を開始します。デスクトップ版 OpenPencil のインストールは不要です。

公開パッケージをインストールすると、`darwin-arm64`、`darwin-x64`、`linux-arm64`、`linux-x64`、`win32-arm64`、`win32-x64` の6つのネイティブプラットフォームパッケージから、現在の OS/CPU に対応するものが選択されます。Linux の2パッケージは glibc 向けです。ルートパッケージはこれらを厳密なバージョンの `optionalDependencies` として宣言し、パッケージマネージャーが適切なバリアント（例: `@zseven-w/dsh-openpencil-darwin-arm64`）を選択できるようにします。このパッケージには、互いに対応する `op-host-web-server`、エディターの Web バンドル、CanvasKit が1つのランタイムとして同梱されています。そのため、マネージドエディターは `/Applications/OpenPencil.app`、`PATH` 上の `openpencil-desktop`、OpenPencil のソースチェックアウトに依存しません。

キャンバスが未保存の状態で DSH がプラグインをリロードまたはアンロードした場合、ホストは不透明なローカルリカバリードラフトを最大7日間保持します。同じソースを再度開くと、ライブキャンバスへの復元前に確認を求められます。リカバリーはユーザーが明示的に保存するまで `.op` ファイルを上書きしません。

公式の6プラットフォームパッケージでは、保護された release ビルド中に中国向け／グローバル向けのコラボレーション bootstrap エンドポイントが注入および検証され、検証に成功した場合のみ公開されます。この注入を行わないローカルのセルフビルドでは、DSH を起動する前に `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap` で bootstrap を上書きできます。値は `https` を使用し、パスは厳密に `/api/v1/collaboration/bootstrap` でなければなりません。

デバイス間のキャンバス同期には、PC/DSH ネイティブランタイムとモバイルアプリの両方を、現在のコラボレーションキュー修正を含む同じ OpenPencil リリース系列へ更新する必要があります。古いモバイルアプリと新しい PC ランタイムを組み合わせると、リモートカーソルは表示されてもキャンバスのコミットを受信できない場合があります。

このリポジトリで開発する場合は、DSH を起動する前にエディターの Web bundle、ネイティブホストの順でビルドし、対応するランタイムをステージングします。

`pnpm run build:editor-web` は、OpenPencil が正式にサポートする WASM bundle gate を実行します。Bash、`wasm32-unknown-unknown` target を含む Cargo/Rust、`wasm-bindgen` CLI、Binaryen の `wasm-opt`、Node.js、`gzip` が必要です。CanvasKit に EMSDK は不要です。Web ビルドはコラボレーション bootstrap のビルド変数を使用しません。`pnpm run build:editor-runtime` の前に `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN` と `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL` の両方を設定してください。これらはネイティブ Cargo ビルドだけで使用され、どちらかが未設定なら fail closed で失敗します。両方のビルドが成功した後、最後のコマンドでランタイムをステージングします。

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

ランタイムを明示的に上書きする場合は、対応する次の3項目を完全な1セットとして指定する必要があります:

- `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY` — `op-host-web-server` 用;
- `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR` — ビルド済みのエディター Web バンドル用;
- `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR` — CanvasKit アセット用。

一部だけを指定した構成は無効です。プラグインがカスタムパスと同梱ランタイムのアセットを組み合わせることはありません。

保存は楽観的ソースハッシュ、アトミックな置き換え、後継ケーパビリティを使用します。エディターの外部でソースが変更された場合、プラグインは上書きせずに競合を報告します。

## 結果メタデータ

モデルに表示される結果はプレーンな JSON のままです。ブラウザ専用の `presentationMeta.$dshOpenPencil` は次の追加の認可を保持します:

- `image`: PNG パス、プレビュー/ダウンロード URL、実際の幅/高さ;
- `frames`: アクティブページの順序での正確にレンダリングされたすべてのトップレベルフレーム。ノードの id/名前/インデックスと署名付き PNG URL を含みます;
- `document`: ソースアクションパスに加え、不変のスナップショット URL、バイト数、SHA-256;
- `viewer`: アセットルートが接続されている場合の、リビジョン付き SDK/WASM/CanvasKit URL;
- `editor`: `editable: true` が認可された場合の、スコープ付きの起動/更新ケーパビリティ。

結果には `renderer`、`rendererBinary`、`fidelity`、および任意の警告も記録されます。既存の PNG のみの schema-v1 メッセージは引き続きレンダリング可能です。

DSH `0.1.1-rc.2` は、PTC/Code Mode 配下にネストされたツールのブラウザプレゼンテーションメタデータを永続化しません。プラグインは、同一オリジンでセッションにバインドされたエンドポイントを通じて、その UI-only の投影を復元します。ブラウザは session id、call id、不変のドキュメント SHA-256 のみを送信し、ホストは永続的な DSH セッションログから正式な結果を解決し、短命のインプロセスマーカーを直近のライブ編集の認可にのみ使用します。署名付きのプレビュー/エディターのケーパビリティが正規のツール結果やモデルコンテキストに入ることはありません。通常の `openpencil_render` の永続履歴は読み取り専用のままです。厳密に解析された `openpencil_new` または `openpencil_pipeline_finish` の永続 publication は、loopback 接続でユーザーが明示的にクリックした場合にのみエディター権限を取得できます。サイドバーの自動オープンは、直近の信頼できるライブ結果だけに限定されます。

制限付きリプレイのため、ネストされたメタデータのリカバリーは最大128個のトップレベルフレームを受け入れます。より大きな Code Mode の結果は、正規の JSON フォールバックを通じて引き続き利用可能です。

## 現在の制限

- 既存キャンバスへの追跡編集には、すでに開いているマネージドエディターが必要です。変更は、ユーザーがその保存アクションを実行するまで未保存のままです。
- 軽量な Web SDK キャンバスは読み取り専用です。本格的な編集には別のマネージドエディターのサーフェスを使用します。DSH `0.1.1-rc.2` では、プラグインは全画面オプション付きのリサイズ可能な右ワークベンチを使用します。
- 正確なギャラリーはアクティブページ上のトップレベルフレームを対象とします。非アクティブなページやネストされたノードの検査には、引き続きインタラクティブキャンバスを使用します。
- レンダリングとスナップショットのキャッシュには、製品レベルの保持ポリシーがまだ必要です。

## プロジェクト構造

```text
packages/op-dsh/
├── src/                       Plugin sources (TypeScript)
│   ├── index.ts               Host plugin entry — Cordis service, tools, assets
│   ├── tool.ts / design-tools.ts / new-tool.ts   Host-side design tools
│   ├── renderer.ts            Exact OpenPencil renderer + Jian fallback
│   ├── editor-host.ts / editor-recovery.ts       Managed editor lifecycle + drafts
│   ├── viewer-assets.ts       Web SDK / WASM / CanvasKit asset staging
│   ├── mcp-client.ts          OpenPencil MCP connection
│   └── client/                Browser client — React workbench, gallery, selection dock
├── lib/                       Compiled output (published to npm)
├── scripts/                   Build helpers — viewer asset sync, client build, host tests
├── tests/                     Node test suites (client, host API, MCP, viewer assets)
├── docs/images/               Documentation screenshots
├── cordis.patch.yml           DSH bundle patch that mounts the plugin
├── tsconfig.json              Host / Node TypeScript config
└── tsconfig.client.json       Browser client TypeScript config
```

## ビルドと検証

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

ビルドには Node 24.11 以降と pnpm が必要です。DSH のホスト/クライアントパッケージは、対象の DSH プロファイルが提供するピア依存関係です。ビルドツールは、ローカルの開発依存関係、アクティブなリンク済み DSH チェックアウト、またはインストール済みの DSH ソースバンドルから解決されます。`DSH_SOURCE_ROOT` でソースチェックアウトを明示的に選択できます。ロックファイルは、その環境が別途プロビジョニングされる場合に、スタンドアロンの公開ビルドツールを固定します。

プライベートな DSH プレリリースでは、発行された npm 認証情報をこのリポジトリの外（たとえばユーザーレベルまたは一時的な `.npmrc`）に保持し、要求されたバージョンを直接実行してください:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

`.npmrc`、`NPM_TOKEN`、またはコピーしたレジストリ認証情報をコミットしないでください。このリポジトリはデフォルトでローカルの npm 設定を無視します。

`test:host` は実際の正確なレンダリングを実行し、PNG の IHDR ジオメトリと SHA-256 を検証し、HTTP 経由で不変の画像/ドキュメントケーパビリティを実行し、ビューアーアセットが付与可能かどうかを確認します。期待される寸法はフィクスチャ固有です。

## エコシステム

DSH OpenPencil は **[OpenPencil](https://github.com/ZSeven-W/openpencil)** — 世界初のオープンソースの AI ネイティブベクターデザインツール — 用の DeepSeek Harness プラグインであり、純 Rust の AI ネイティブツール群 **[ZSeven-W](https://github.com/ZSeven-W)** ファミリーの一員です。

| プロジェクト | 概要 |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | このプラグインが操作するデザインツール — プロンプトからキャンバスへの生成、並行エージェントチーム、デザイン・アズ・コードの `.op` ファイル、組み込みの MCP サーバー。ここでの正確なプレビュー、インタラクティブキャンバス、マネージドエディターは、すべて OpenPencil 自身によって動作しています。 |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | LLM エージェントを出荷するための純 Rust の非同期ランタイム — マルチプロバイダー、エンドツーエンドのツール対応、構造化された権限、本物の MCP、`unsafe` ゼロ。OpenPencil の組み込みエージェントランタイムを支えています。 |
| **[jian](https://github.com/ZSeven-W/jian)** | 純 Rust の GPU-Skia UI フレームワーク — ウィジェット、レイアウト、イベント、ホットリロードをひとつのスタックに統合。OpenPencil の UI フレームワークであり、このプラグインのフォールバックレンダラーの源泉です。 |
| **[Zode](https://github.com/ZSeven-W/zode)** | ターミナル向けのオープンソースの AI ネイティブコーディングアシスタント — コードを読み、コマンドを実行し、MCP 経由で OpenPencil を操作します。 |
| **[noema](https://github.com/ZSeven-W/noema)** | コーディングエージェント向けのローカルファーストで非ベクターのメモリシステム — 検査可能なファイルとしての永続的なメモリで、ランタイムをまたいで動作します。 |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | AI エージェントに `op` でのデザイン方法を教える LLM スキルプラグイン — この DSH プラグインのコンパニオンです。 |

同じ DSH プラグインファミリー：

- [DSH Android](https://github.com/ZSeven-W/dsh-android) — 会話の中で動く Android エミュレータや USB 接続の実機を、すべて adb 経由で操作
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) — Claude Code / Codex から DSH エージェントに作業を委譲
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) — 会話の中で動く iOS シミュレータと USB 接続の実機
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) — DSH の長期記憶

## コントリビューション

コントリビューションを歓迎します！フォークしてクローンし、ブランチを作成し、`pnpm run build` とテストスイートを実行し、[Conventional Commits](https://www.conventionalcommits.org/) に従ってコミットし、`main` に対して PR を開いてください。

## コミュニティ

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong>Discord に参加</strong>
</a>
— 質問をしたり、デザインを共有したり、機能を提案したりできます。

**認定コミュニティ: [LINUX DO](https://linux.do/)**

## ライセンス

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

サードパーティコンポーネントは [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md) に記載されています。

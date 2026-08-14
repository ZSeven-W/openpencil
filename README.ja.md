<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>世界初のオープンソース AI ネイティブベクターデザインツール。</strong><br />
  <sub>並行エージェントチーム &bull; Design-as-Code &bull; 内蔵 MCP サーバー &bull; マルチモデルインテリジェンス</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md"><b>日本語</b></a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/rust-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/rust-check.yml?label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>画像をクリックしてデモ動画を視聴</sub></p>

<br />

<p align="center">
  <a href="https://github.com/InnerAura/hyperweave" title="Star history rendered with HyperWeave">
    <img src="./screenshot/openpencil-star-history.svg" alt="OpenPencil star history rendered by HyperWeave" width="100%" />
  </a>
</p>
<p align="center">
  <sub>The trend above is rendered with <a href="https://github.com/InnerAura/hyperweave"><b>HyperWeave</b></a> — an experimental SVG artifact layer for agents. &nbsp;<a href="https://github.com/InnerAura/hyperweave"><b>Explore&nbsp;→</b></a></sub>
</p>

## Why OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 プロンプト → キャンバス

自然言語で任意の UI を記述。ストリーミングアニメーションでリアルタイムに無限キャンバス上に表示。要素を選択してチャットすることで既存のデザインを修正。

</td>
<td width="50%">

### 🤖 並行エージェントチーム

オーケストレーターが複雑なページを空間的なサブタスクに分解。複数の AI エージェントがヒーロー、機能紹介、フッターなど異なるセクションを同時に処理し、すべてが並列でストリーミング。

</td>
</tr>
<tr>
<td width="50%">

### 🧠 マルチモデルインテリジェンス

各モデルの能力に自動適応。Claude にはシンキング付きフルプロンプト、GPT-4o/Gemini ではシンキングを無効化、小規模モデル（MiniMax、Qwen、Llama）には信頼性の高い出力のために簡略化プロンプトを使用。

</td>
<td width="50%">

### 🔌 MCP サーバー

Claude Code、Codex、OpenCode、Kiro、Copilot CLI にワンクリックでインストール。ターミナルからデザイン — MCP 対応エージェントを通じて `.op` ファイルの読み取り、作成、編集が可能。

</td>
</tr>
<tr>
<td width="50%">

### 📦 Design-as-Code

`.op` ファイルは JSON — 人間が読みやすく、Git フレンドリーで差分比較可能。デザイン変数は CSS カスタムプロパティを生成。React + Tailwind または HTML + CSS へのコードエクスポート。

</td>
<td width="50%">

### 🖥️ どこでも動作

Web アプリ + macOS・Windows・Linux ネイティブデスクトップ — 単一の Rust コア、単一の自己完結型バイナリ、ブラウザエンジン不要。`.op` ファイル関連付け — ダブルクリックで開く。

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

ターミナルからデザインツールを操作。`op design`、`op insert` — バッチデザインDSL、ノード操作。ファイルやstdinからのパイプ入力に対応。デスクトップアプリまたはWebサーバーと連携。

</td>
<td width="50%">

### 🎯 マルチプラットフォームコードエクスポート

1つの`.op`ファイルからReact + Tailwind、HTML + CSS、Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Nativeへエクスポート。デザイン変数はCSSカスタムプロパティに変換。

</td>
</tr>
</table>

## インストール

**macOS（Homebrew）：**

```bash
brew tap zseven-w/openpencil
brew install --cask openpencil
```

**Windows（Scoop）：**

```powershell
scoop bucket add openpencil https://github.com/zseven-w/scoop-openpencil
scoop install openpencil
```

**Linux / Windows 直接ダウンロード：** [GitHub Releases](https://github.com/ZSeven-W/openpencil/releases) — `.exe`（Windows）、`.AppImage` / `.deb`（Linux）

**Nix（Linux x86_64）：**

```bash
nix develop
nix run .                         # デスクトップアプリを起動
nix build .#openpencil            # ネイティブ Web ホスト + CanvasKit Web バンドル
nix build .#op-cli                # `op` CLI
nix build .#prebuilt              # 対応する upstream デスクトップアーカイブを使用
nix build .#prebuilt-cli          # 対応する upstream CLI アーカイブを使用
nix build .#web-server            # GL 不要のネイティブ Web サーバー + Web バンドル
nix build .#runtime-prebuilt      # ビルド済みデスクトップ + `op` CLI ランタイム
nix build .#web-sdk-packages      # Web SDK の npm tarball
nix build .#appimage              # ポータブルなデスクトップ AppImage
```

この flake は `rust-toolchain.toml` で固定された Rust toolchain を使用し、
現在は `x86_64-linux` 向けに公開されています。flake はまだ Debian
パッケージを生成しないため、`.deb` が必要な場合は upstream release
artifact を使用してください。`prebuilt` output は workspace のソース
バージョンとは独立して、`nix/release-manifest.json` に固定された release
バージョンと hash を使用します。release の公開後、release workflow が
この manifest を更新する PR を作成します。merge されるまでは、ビルド済み
output は以前に公開された release を引き続き使用し、ソースビルド output
は常に checkout 中のソースを使用します。

**CLI（`op`）：**

```bash
brew install zseven-w/openpencil/op
```

またはインストールスクリプトを使用します（macOS / Linux）：

```bash
curl -fsSL https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.sh | bash
```

最新のプレリリースを許可する場合：

```bash
curl -fsSL https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.sh | OP_PRERELEASE=1 bash
```

Windows PowerShell：

```powershell
irm https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.ps1 | iex
```

最新のプレリリースを許可する場合：

```powershell
$env:OP_PRERELEASE = "1"; irm https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.ps1 | iex
```

## クローン（サブモジュールを含む）

```bash
git clone --recurse-submodules https://github.com/ZSeven-W/openpencil.git
# クローン済みの場合、古いサブモジュール URL に .gitmodules の変更を反映するため、先に同期します：
git submodule sync --recursive && git submodule update --init --recursive
```

`vendor/` 配下には 3 つのサブモジュールがあります。すべて公開され、HTTPS で取得されるため SSH キーは不要です：`jian`（GPU-Skia UI フレームワーク — widget/render/event）、`casement`（winit fork）、`agent`（`agent-rs` — OP と Zode で共有する製品横断 Rust agent runtime）。`vendor/anthropic-agent-sdk` はリポジトリ内で直接管理されており、サブモジュールではありません。

## クイックスタート

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

またはデスクトップアプリとして実行：

```bash
cargo run -p op-host-desktop
```

> **前提条件：** 製品のビルドには [Rust](https://www.rust-lang.org/)（stable）が必要です。[Bun](https://bun.sh/) >= 1.0 および [Node.js](https://nodejs.org/) >= 18 は `packages/` 配下の web SDK にのみ必要です。

### Docker

タグ付きの Rust リリースでは、単一の web-host イメージを公開します。AI CLI を同梱していた引退済みの TypeScript イメージはもう公開されません。

| イメージ | 含まれるもの |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:vX.Y.Z` | Rust web host、wasm bundle、CanvasKit assets |

Web UI は built-in agent profiles のみを公開します。Claude/Codex/OpenCode/Copilot CLI ツールは Docker イメージに同梱されません。

**実行：**

```bash
VERSION="$(scripts/workspace-version.sh)"
docker run -d -p 3100:3100 "ghcr.io/zseven-w/openpencil-web:v${VERSION}"
```

その後 `http://localhost:3100/` を開いてください。

**ローカルビルド：**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## AI ネイティブデザイン

**プロンプトから UI へ**

- **テキストからデザインへ** — ページを説明すると、ストリーミングアニメーションでリアルタイムにキャンバス上に生成
- **オーケストレーター** — 複雑なページを空間サブタスクに分解し、並列生成をサポート
- **デザイン修正** — 要素を選択し、自然言語で変更内容を記述
- **ビジョン入力** — スクリーンショットやモックアップを参照として添付してデザイン

**マルチエージェントサポート**

| エージェント                      | 設定方法                                                                                          |
| --------------------------------- | ------------------------------------------------------------------------------------------------- |
| **ビルトイン（9+ プロバイダー）** | プロバイダープリセットから選択し、リージョンを切り替え — Anthropic、OpenAI、Google、DeepSeek など |
| **Claude Code**                   | 設定不要 — ローカル OAuth で Claude Agent SDK を使用                                              |
| **Codex CLI**                     | エージェント設定で接続（`Cmd+,`）                                                                 |
| **OpenCode**                      | エージェント設定で接続（`Cmd+,`）                                                                 |
| **GitHub Copilot**                | `copilot login` 後、エージェント設定で接続（`Cmd+,`）                                             |

**モデル能力プロファイル** — モデルの階層に応じてプロンプト、シンキングモード、タイムアウトを自動適応。フル階層モデル（Claude）には完全なプロンプト、標準階層（GPT-4o、Gemini、DeepSeek）ではシンキングを無効化、ベーシック階層（MiniMax、Qwen、Llama、Mistral）には最大限の信頼性のために簡略化されたネスト JSON プロンプトを使用。

**i18n** — 15言語での完全なインターフェースローカライゼーション：English、简体中文、繁體中文、日本語、한국어、Français、Español、Deutsch、Português、Русский、हिन्दी、Türkçe、ไทย、Tiếng Việt、Bahasa Indonesia。

**MCP サーバー**

- 内蔵 MCP サーバー（`op-mcp` crate） — Claude Code / Codex / OpenCode / Kiro / Copilot CLI にワンクリックでインストール
- Node.js は不要 — デスクトップバイナリ経由の stdio トランスポート（`--mcp <path>`）に加え、実行中のアプリからライブ HTTP エンドポイント（`127.0.0.1:<port>/mcp`）を提供
- ターミナルからのデザイン自動化：MCP 対応エージェントを通じて `.op` ファイルの読み取り、作成、編集が可能
- **レイヤードデザインワークフロー** — `design_skeleton` → `design_content` → `design_refine` による高忠実度マルチセクションデザイン
- **セグメント化プロンプト取得** — 必要なデザイン知識のみをロード（schema、layout、roles、icons、planning など）
- マルチページサポート — MCP ツールを通じてページの作成、名前変更、並べ替え、複製が可能

**コード生成**

- React + Tailwind CSS、HTML + CSS、CSS Variables
- Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Native

## CLI — `op`

グローバルインストールしてターミナルからデザインツールを操作：

```bash
brew install zseven-w/openpencil/op
```

```bash
op start                     # デスクトップアプリを起動
op start --headless --file design.op # ヘッドレスサーバーを起動
op design @landing.txt       # ファイルからバッチデザイン
op design @ui.js             # ループ対応のサンドボックス JavaScript
op insert '{"type":"rectangle"}' # ノードを挿入
op import:figma design.fig   # Figma ファイルをインポート
cat design.dsl | op design - # stdin からパイプ入力
```

インライン文字列、`@filepath`、stdin（`-`）に対応し、デスクトップアプリ、Web サーバー、ファイルベースのヘッドレスサーバーと連携します。全コマンドは [CLI コマンドリファレンス](./crates/op-cli/src/usage.txt) を参照してください。

**LLM スキル** — [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) をインストールして、AIエージェントに `op` を使ったデザインを教えます。検出済みのエージェントには `op install`、対象を指定する場合は `op install --target codex` を実行します。

## 機能

**キャンバスと描画**

- パン、ズーム、スマートアライメントガイド、スナッピング対応の無限キャンバス
- 矩形、楕円、直線、多角形、ペン（ベジェ）、Frame、テキスト
- ブーリアン演算 — 合体、型抜き、交差（コンテキストツールバー付き）
- アイコンピッカー（Iconify）と画像インポート（PNG/JPEG/SVG/WebP/GIF）
- オートレイアウト — 垂直/水平方向、ギャップ・パディング・justify・align 対応
- タブナビゲーション付きマルチページドキュメント

**デザインシステム**

- デザイン変数 — カラー・数値・文字列トークン、`$variable` 参照付き
- マルチテーマサポート — 複数のテーマ軸、各軸に複数バリアント（Light/Dark、Compact/Comfortable）
- コンポーネントシステム — インスタンスとオーバーライドを持つ再利用可能なコンポーネント
- CSS 同期 — カスタムプロパティの自動生成、コード出力に `var(--name)` を使用
- 再利用可能な UIKit — `.pen` ファイルからコンポーネントキットをインポート/エクスポート

**AI とエージェント**

- ストリーミング生成とオーケストレーター駆動の空間分解によるプロンプトからキャンバスへの変換
- 並行エージェントチーム — 複数のデザイナーが異なるセクションを並列で処理し、各メンバーにキャンバスインジケーター
- レイヤードワークフロー — `design_skeleton` → `design_content` → `design_refine`、各フェーズごとに焦点を絞ったプロンプト
- スタイルガイド — 50+ のビルトインスタイル（glassmorphism、brutalist、retro など）、タグベースのファジーマッチング対応、プランニングと生成に統合
- マルチモデル能力プロファイル — モデル階層に応じてシンキングモード、エフォート、プロンプト形状を自動適応
- ビルトインエージェントランタイム（Rust）+ Anthropic、Claude Agent SDK、OpenCode、Codex、Copilot、Google Gemini API プロバイダー
- 中国系 LLM プロバイダー向け Anthropic フォーマットパススルー — Kimi、Zhipu、GLM、DouBao、Ark、Bailian/DashScope、ModelScope、Coding Plans

**コラボレーション**

- 認証済みのピアツーピアセッション — パブリックリレーのフォールバックと内蔵のリージョン別コラボレーションハブ
- リージョンタグ付きの 10 文字ペアリングコードで参加 — LAN セッションはアカウント不要
- ライブリモートカーソル、アカウント間コラボレーション、編集単位の詳細と破棄された編集のリプレイを備えたコンフリクトパネル
- `--serve-web` デーモン向けのオンラインマルチテナントモード — op-hub に対して認証し、アカウント間でテナントを共有
- デバイスログイン — serve-web デーモンを通じてブラウザからサインイン、エディター内にプロフィールアバターとユーザー名を表示

**プレゼンテーションデッキ**

- テンプレートピッカー付きの 6 種類の 16:9 デッキテンプレート、加えてプロジェクターサイズでの AI デッキプランニング — 1 画面につき 1 スライド
- プレゼンターコントロールでデッキをスライドショーとして発表
- デッキを PDF（スライドごとに 1 ページ）、自己完結型スライドショー HTML ファイル、編集可能な PowerPoint（`.pptx`）、または hyperframes ビデオコンポジションとしてエクスポート
- スライドレールナビゲーター；エージェントはプロンプトに加えて、デッキボードのジオメトリ（アスペクト、オーバーフロー、センタリング）も検証

**テンプレート & Web キャプチャ**

- シーンテンプレートセンター — 6 シーンにわたる 58 テンプレートを閲覧できるカタログ、ファイル ▸ テンプレートから新規作成 で開く
- Web、ダッシュボード、コンポーネント、修正のエントリとビジュアルプロンプトプレビューを備えたプロンプトセンター
- アセットセンター — デュアルアクションテンプレートと DESIGN.md スタイルインポートを備えた全画面レスポンシブギャラリー
- [Chrome Web キャプチャ拡張機能](https://chromewebstore.google.com/detail/openpencil/ahfofgelkdaobcmglejmjbpnajhpgcij) — 任意の Web ページを忠実な HTML / レイアウトキャプチャで OpenPencil にスナップショット；スナップショットはハブ上のアカウントの受信トレイに配信されるか、オフラインではすぐに開ける `.op` ファイルとしてダウンロード

**Git 統合**

- SSH / HTTPS 認証と SSH キー管理を備えたクローンウィザード
- ブランチピッカー — 作成、切り替え、削除、マージをすべて Git パネルから
- プル / プッシュカスケード — 認証リトライとノンファストフォワード処理対応
- フォルダーモード三方向マージ — ディスク上の `MERGE_HEAD` 状態追跡付き
- コンフリクトパネル — ノード/フィールド単位の三方向カード、インライン JSON エディター、一括操作、インライン diff ブロック
- リモート設定と SSH キー UI、Git サーフェス全体にわたる 15 ロケール i18n

**エクスポート**

- キャンバスエクスポート — PNG、JPEG、WEBP、PDF（`Cmd+Shift+P`）
- コードエクスポート — React + Tailwind、HTML + CSS、Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Native
- インクリメンタル MCP コード生成パイプライン — `codegen_plan`、`codegen_submit_chunk`、`codegen_assemble`、`codegen_clean`

**Figma インポート**

- レイアウト、フィル、ストローク、エフェクト、テキスト、画像、ベクターを保持して `.fig` ファイルをインポート

**デスクトップアプリ**

- macOS・Windows・Linux ネイティブ対応 — 単一の自己完結型バイナリ（winit + GPU Skia、Electron 不要）
- `.op` ファイル関連付け — ダブルクリックで開く、シングルインスタンスロック
- GitHub Releases に対するバックグラウンド更新チェック
- 名前を付けて保存、最近使った項目を開く、および終了時の未保存変更ダイアログを備えたネイティブアプリケーションメニュー
- 最近使ったファイルの永続化

## 技術スタック

|                    |                                                                                  |
| ------------------ | -------------------------------------------------------------------------------- |
| **コア**           | Rust ワークスペース（`crates/`） — エディター状態、ウィジェット、ホスト、MCP、AI、codegen |
| **レンダリング**   | あらゆる場所で GPU Skia — ネイティブは `skia-safe`（GL）、ブラウザは CanvasKit（WASM/WebGL2） |
| **UI フレームワーク** | jian — ベンダー化された純 Rust の GPU-Skia UI フレームワーク：ウィジェット、レイアウト、イベント、ホットリロード（`vendor/jian`） |
| **ウィンドウイング** | winit（ベンダー化された `casement` フォーク）                                   |
| **デスクトップ**   | ネイティブバイナリ `openpencil-desktop` — ブラウザエンジン不要                   |
| **Web SDK**        | `op-web-sdk` + React 19 / Vue 3 アダプター — 読み取り専用 `.op` ビューアー（TypeScript） |
| **CLI**            | `op` — ターミナル制御、バッチデザインDSL                                         |
| **AI**             | ビルトイン Rust エージェントランタイム · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **リント**         | clippy · rustfmt（Rust） · oxlint · oxfmt（Web SDK）                             |
| **ファイル形式**   | `.op` — JSON ベース、人間が読みやすく、Git フレンドリー                          |

## エコシステム

OpenPencil は **[ZSeven-W](https://github.com/ZSeven-W)** による純 Rust・AI ネイティブなツール群の一員です。これらは互いに組み合わさります：`jian` が OpenPencil をレンダリングし、`agent-rs` がそのエージェントを実行し、`noema` が記憶し、`zode` がターミナルからデザインします。

| プロジェクト | 内容 |
| ------- | ---------- |
| **[DSH OpenPencil](https://github.com/ZSeven-W/dsh-openpencil)** | OpenPencil 向けの DeepSeek Harness プラグイン — 会話の中で、正確なマルチフレーム `.op` プレビュー、インタラクティブなキャンバス、エージェントネイティブな設計ツールを備えたマネージドエディタを提供します。 |
| **[Zode](https://github.com/ZSeven-W/zode)** | ターミナル向けのオープンソース AI ネイティブコーディングアシスタント — コードを読み、コマンドを実行し、ファイルを検索し、git を管理する高速な Rust TUI（`ratatui`）。MCP 経由で OpenPencil を駆動します。 |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | LLM エージェントを届けるための純 Rust 非同期ランタイム — マルチプロバイダー、エンドツーエンドでツール対応、構造化された権限、本物の MCP、`unsafe` ゼロ。OpenPencil のビルトインエージェントランタイム（`vendor/agent`）と Zode を支えます。 |
| **[jian](https://github.com/ZSeven-W/jian)** | 純 Rust の GPU-Skia UI フレームワーク — ウィジェット、レイアウト、イベント、ホットリロードを一つのスタックに。宣言的な `.op` ドキュメントを、JS ランタイムも DOM も Electron もない、ネイティブで AI 制御可能なアプリへと変えます。OpenPencil の UI フレームワーク（`vendor/jian`）。 |
| **[noema](https://github.com/ZSeven-W/noema)** | コーディングエージェント向けのローカルファースト・非ベクター記憶システム。検査可能なファイルとしての永続的な記憶、新規エントリのレビューキュー、そして語彙的（埋め込み不要）な想起 — Zode、Codex、Claude Code、MCP ランタイム間で機能します。 |

## なぜ Rust か

OpenPencil は **Rust** で一から書き直されました ([#129](https://github.com/ZSeven-W/openpencil/issues/129))。書き直しは完了しています — TypeScript + Electron エディターは `v0.7.5` で廃止され、このリポジトリの Rust ワークスペースが製品そのものです：ひとつのネイティブコアで劇的に軽量かつ高速になり、単一のコードベースからより多くのプラットフォームで動作します。

|                       | TypeScript + Electron（廃止済み、`v0.7.5`）    | Rust（現在）                                                        |
| --------------------- | ---------------------------------------------- | ------------------------------------------------------------------- |
| **デスクトップランタイム** | Electron — Chromium + Node.js をバンドル       | ネイティブウィンドウ（`winit` + GPU Skia）、ブラウザエンジン不要     |
| **デスクトップフットプリント** | インストールごとに Chromium ランタイム全体     | 単一の自己完結型バイナリ — **55.5 MB**                              |
| **Web ペイロード**    | JS + WASM バンドル                             | **8.2 MB** wasm / **2.18 MB** gzip（転送時）                        |
| **レンダリング**      | Web 上の CanvasKit/Skia                        | **すべての**ターゲットで単一の GPU アクセラレーション Skia バックエンド |
| **メモリ**            | JavaScript GC によるポーズ                     | GC なし — Rust 所有権モデル、予測可能なレイテンシ                   |
| **コードベース**      | Web スタック + Electron | 単一 Rust ワークスペース：エディター・CLI・MCP・AI・codegen・Figma・Git |
| **ターゲット**        | Web + デスクトップ、別々のスタック             | デスクトップ（macOS/Win/Linux）・モバイル（iOS/Android）・ブラウザ — 単一コア |

**実測改善値**

- **極小フットプリント** — デスクトップアプリ全体が、ブラウザエンジンと Node ランタイムのバンドルではなく、**55.5 MB** のネイティブバイナリ 1 つ。Web ビルドはアイコンカタログを分割した後、**8.2 MB** 生 / **2.18 MB** gzip（転送量 −48%）。
- **大規模ドキュメントへのスケール** — **10,000-node** のライブキャンバス（ネスト 4 段のオートレイアウト）で、書き込み・読み込み・レイアウトスナップショットを**パニックなし、アイドル CPU ~0%** で実行。全 10k ノードのレイアウトスナップショットは **~0.68 s** で返却。
- **高速インタラクション** — パン/ズームがフレームごとにドキュメントを再シリアライズしなくなった（ホットパスの修正 1 点でホイールズーム CPU を **~69% から ~0%** に削減）。ドラッグはシーンをインクリメンタルにパッチし、テキスト計測はキャッシュされ、再描画は 1 フレームあたり 1 回にまとめられる。
- **単一コア、あらゆる画面** — 同じエディターステートと同じレンダリングバックエンドが、WASM 経由でネイティブデスクトップ、モバイル、ブラウザにコンパイル — 同期を保つための並列再実装は不要。
- **GPU Skia everywhere** — ネイティブは GL コンテキスト上の `skia-safe` でレンダリング。ブラウザは WebGL2 上の CanvasKit でレンダリング — 同じ描画コード、同じ出力。
- **ネイティブアクセシビリティ** — macOS・Windows・Linux では AccessKit を使用し、Web ではブラウザの a11y ツリーに依存する代わりに DOM ミラーを提供。
- **型チェック済みの単一ワークスペース** — MCP ホスト、CLI、AI プロバイダー、コード生成、Figma インポート、Git 統合がすべて単一の Rust ワークスペースに収まり、CI では `cargo-deny` によるサプライチェーンのゲーティングを実施。

> **ステータス：** TypeScript エディターは `v0.7.5` で廃止され、現在は Git 履歴にのみ残っています。このリポジトリは Rust ワークスペースです。Rust 製品は現在活発に開発中です（下記のロードマップを参照）。

## プロジェクト構成

```text
openpencil/
├── crates/                   Rust ワークスペース — 製品本体
│   ├── op-editor-core/       正規の `.op`（PenDocument）エディター状態 + EditorCommand + デザイン変数
│   ├── op-editor-ui/         プラットフォーム非依存ウィジェット + RenderBackend ファサード（wasm32 対応）
│   ├── op-editor-host-core/  全ホスト共有のトランスポート非依存ホストステートマシン
│   ├── op-host-native/       ネイティブホストライブラリ — winit + skia-safe GL（デスクトップ + モバイル）
│   ├── op-host-web/          ブラウザバンドル — wasm32 cdylib、CanvasKit レンダラー
│   ├── op-host-desktop/      デスクトップバイナリ `openpencil-desktop`；`--serve-web` デーモンも兼ねる
│   ├── op-host-services/     ヘッドレス serve-web / MCP デーモンライブラリ
│   ├── op-host-web-server/   軽量な GL 非依存 Web サーバーバイナリ
│   ├── op-cli/               CLI ツール — `op` コマンド
│   ├── op-mcp/               MCP サーバー — ツール、バッチデザイン、レイヤードワークフロー
│   ├── op-ai/                AI プロバイダー、チャットランタイム、ストリーミング
│   ├── op-ai-skills/         AI プロンプトスキルエンジン（フェーズ駆動プロンプト読込）
│   ├── op-orchestrator/      並行エージェントチームのオーケストレーション
│   ├── op-codegen/           コードジェネレーター（React、HTML、Vue、Flutter、...）
│   ├── op-figma/             Figma .fig ファイルパーサーとコンバーター
│   ├── op-git/               Git 統合 — クローン、ブランチ、プッシュ/プル、マージ
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Web SDK ワークスペース（Bun）
│   ├── op-web-sdk/           読み取り専用 `.op` Web ビューアー SDK（wasm バンドルをラップ）
│   ├── op-web-sdk-react/     React 19 アダプター
│   └── op-web-sdk-vue/       Vue 3 アダプター
├── vendor/                   ベンダー化されたサブシステム（git サブモジュール）
│   ├── jian/                 GPU-Skia UI フレームワーク — ウィジェット/レンダー/イベント
│   ├── casement/             winit フォーク
│   └── agent/                製品横断 Rust エージェントランタイム（agent-rs）
└── .githooks/                プレコミットのバージョンドリフト検査
```

## キーボードショートカット

| キー        | 操作           |     | キー          | 操作                            |
| ----------- | -------------- | --- | ------------- | ------------------------------- |
| `V`         | 選択           |     | `Cmd+S`       | 保存                            |
| `R`         | 矩形           |     | `Cmd+Z`       | 元に戻す                        |
| `O`         | 楕円           |     | `Cmd+Shift+Z` | やり直す                        |
| `L`         | 直線           |     | `Cmd+C/X/V/D` | コピー/カット/ペースト/複製     |
| `T`         | テキスト       |     | `Cmd+G`       | グループ化                      |
| `F`         | Frame          |     | `Cmd+Shift+G` | グループ解除                    |
| `P`         | ペンツール     |     | `Cmd+Shift+P` | エクスポート (PNG/JPG/WEBP/PDF) |
| `H`         | ハンド（パン） |     | `Cmd+Shift+C` | コードパネル                    |
| `Del`       | 削除           |     | `Cmd+Shift+V` | 変数パネル                      |
| `[ / ]`     | 重ね順の変更   |     | `Cmd+J`       | AI チャット                     |
| 矢印キー    | 1px 微調整     |     | `Cmd+,`       | エージェント設定                |
| `Cmd+Alt+U` | ブーリアン合体 |     | `Cmd+Alt+S`   | ブーリアン型抜き                |
| `Cmd+Alt+I` | ブーリアン交差 |     | `Cmd+Shift+S` | 名前を付けて保存                |

## スクリプト

```bash
# Product (Rust — run from the repo root)
cargo build --workspace              # Build all crates (add --release for prod)
cargo test --workspace               # Run all tests
cargo check --workspace              # Type check
cargo clippy --workspace --all-targets -- -D warnings   # Lint
cargo fmt --all                      # Format
bash scripts/start-web-rust.sh       # Web dev server (wasm bundle + headless host)
cargo run -p op-host-desktop         # Desktop app (binary: openpencil-desktop)
cargo run -p op-cli -- <args>        # CLI (binary: op)

# Web SDK / JS tooling (run from packages/)
cd packages && bun run lint          # Lint the web SDK (oxlint); also: bun run format
cd packages && bun run generate-iconify-catalog   # Regenerate the Rust icon catalog assets

# バージョン同期（リポジトリルートから実行）
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

## コントリビュート

コントリビューションを歓迎します！アーキテクチャの詳細とコードスタイルについては [CLAUDE.md](./CLAUDE.md) をご覧ください。

1. フォークしてクローン
2. バージョンドリフト検査を有効化：`git config core.hooksPath .githooks`
3. ブランチを作成：`git checkout -b feat/my-feature`
4. チェックを実行：`cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
5. [Conventional Commits](https://www.conventionalcommits.org/) 形式でコミット：`feat(canvas): add rotation snapping`
6. `main` ブランチに PR を作成

## ロードマップ

- [x] CSS 同期付きデザイン変数とトークン
- [x] コンポーネントシステム（インスタンスとオーバーライド）
- [x] オーケストレーター付き AI デザイン生成
- [x] レイヤードデザインワークフロー付き MCP サーバー統合
- [x] マルチページサポート
- [x] Figma `.fig` インポート
- [x] ブーリアン演算（合体、型抜き、交差）
- [x] マルチモデル能力プロファイル
- [x] 再利用可能な Rust crate と Web SDK パッケージによる Cargo workspace
- [x] デスクトップおよび Web 向け Rust エディター
- [x] CLIツール（`op`）ターミナル制御
- [x] ビルトイン Rust Agent Runtime（マルチプロバイダー対応）
- [x] i18n — 15言語対応
- [x] JavaScript、React、Vue 向けの wasm ベース Viewer SDK
- [x] タグベースのマッチングと MCP ツールを備えた Style Guides
- [x] 委任とキャンバスインジケーターを備えた Concurrent Agent Teams
- [x] Git 統合（クローン、ブランチ、プッシュ/プル、フォルダーモード三方向マージ）
- [x] キャンバスエクスポート（SVG / PNG / JPEG / WEBP / PDF）
- [x] 共同編集 — 認証済み P2P、パブリックリレー、リージョン別ハブ
- [x] プレゼンテーションデッキ — テンプレート、スライドショープレゼンター、PDF/HTML/PPTX/ビデオエクスポート
- [x] デバイスログインとオンラインマルチテナント Web ホスティング
- [x] HTML / ブラウザスナップショットインポートを備えた Chrome Web キャプチャ拡張機能
- [ ] プラグインシステム

## コントリビューター

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## スポンサー

OpenPencil は無料でオープンソースです。開発は、これを便利だと感じてくださる皆さんの支援で成り立っています — キャンバスを開いたままにしてくれてありがとう。

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

**[MrQyun](https://github.com/mrqyun)** さん、ありがとうございます — あなたの名前もここに並べませんか？**[スポンサーになる →](https://github.com/sponsors/ZSeven-W)**

## コミュニティ

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Discord に参加する</strong>
</a>
— 質問、デザインの共有、機能のリクエストはこちら。

**認定コミュニティ：[LINUX DO](https://linux.do/)**

## フォークしているサードパーティライブラリ

OpenPencil の基盤となる成果を提供してくださったアップストリームのメンテナーに感謝します。以下のコピーは、OpenPencil 固有の統合要件に対応するためだけに保守しています。

- **[casement](https://github.com/ZSeven-W/casement)** — **[winit](https://github.com/rust-windowing/winit)** からのフォーク。
- **[anthropic-agent-sdk](./vendor/anthropic-agent-sdk)** — **[bartolli/anthropic-agent-sdk](https://github.com/bartolli/anthropic-agent-sdk)** をベンダリングし、ローカルフォークとして保守。

各プロジェクトには、それぞれのアップストリームライセンスが引き続き適用されます。

## セキュリティ評価

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## ライセンス

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

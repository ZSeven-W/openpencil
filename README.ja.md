<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/KwXp6BJD"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>画像をクリックしてデモ動画を視聴</sub></p>

<br />

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

Claude Code、Codex、Gemini、OpenCode、Kiro、Copilot CLI にワンクリックでインストール。ターミナルからデザイン — MCP 対応エージェントを通じて `.op` ファイルの読み取り、作成、編集が可能。

</td>
</tr>
<tr>
<td width="50%">

### 📦 Design-as-Code

`.op` ファイルは JSON — 人間が読みやすく、Git フレンドリーで差分比較可能。デザイン変数は CSS カスタムプロパティを生成。React + Tailwind または HTML + CSS へのコードエクスポート。

</td>
<td width="50%">

### 🖥️ どこでも動作

Web アプリ + Electron による macOS・Windows・Linux ネイティブデスクトップ。GitHub Releases からの自動アップデート。`.op` ファイル関連付け — ダブルクリックで開く。

</td>
</tr>
</table>

## クイックスタート

```bash
# 依存関係をインストール
bun install

# http://localhost:3000 で開発サーバーを起動
bun --bun run dev
```

またはデスクトップアプリとして実行：

```bash
bun run electron:dev
```

> **前提条件：** [Bun](https://bun.sh/) >= 1.0 および [Node.js](https://nodejs.org/) >= 18

## AI ネイティブデザイン

**プロンプトから UI へ**
- **テキストからデザインへ** — ページを説明すると、ストリーミングアニメーションでリアルタイムにキャンバス上に生成
- **オーケストレーター** — 複雑なページを空間サブタスクに分解し、並列生成をサポート
- **デザイン修正** — 要素を選択し、自然言語で変更内容を記述
- **ビジョン入力** — スクリーンショットやモックアップを参照として添付してデザイン

**マルチエージェントサポート**

| エージェント | 設定方法 |
| --- | --- |
| **Claude Code** | 設定不要 — ローカル OAuth で Claude Agent SDK を使用 |
| **Codex CLI** | エージェント設定で接続（`Cmd+,`） |
| **OpenCode** | エージェント設定で接続（`Cmd+,`） |
| **GitHub Copilot** | `copilot login` 後、エージェント設定で接続（`Cmd+,`） |

**モデル能力プロファイル** — モデルの階層に応じてプロンプト、シンキングモード、タイムアウトを自動適応。フル階層モデル（Claude）には完全なプロンプト、標準階層（GPT-4o、Gemini、DeepSeek）ではシンキングを無効化、ベーシック階層（MiniMax、Qwen、Llama、Mistral）には最大限の信頼性のために簡略化されたネスト JSON プロンプトを使用。

**MCP サーバー**
- 内蔵 MCP サーバー — Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI にワンクリックでインストール
- Node.js を自動検出 — 未インストールの場合は HTTP トランスポートに自動フォールバックし、MCP HTTP サーバーを自動起動
- ターミナルからのデザイン自動化：MCP 対応エージェントを通じて `.op` ファイルの読み取り、作成、編集が可能
- **レイヤードデザインワークフロー** — `design_skeleton` → `design_content` → `design_refine` による高忠実度マルチセクションデザイン
- **セグメント化プロンプト取得** — 必要なデザイン知識のみをロード（schema、layout、roles、icons、planning など）
- マルチページサポート — MCP ツールを通じてページの作成、名前変更、並べ替え、複製が可能

**コード生成**
- React + Tailwind CSS、HTML + CSS、CSS Variables
- Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Native

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
- マルチテーマサポート — 複数のテーマ軸、各軸に複数バリアント（ライト/ダーク、コンパクト/コンフォータブル）
- コンポーネントシステム — インスタンスとオーバーライドを持つ再利用可能なコンポーネント
- CSS 同期 — カスタムプロパティの自動生成、コード出力に `var(--name)` を使用

**Figma インポート**
- レイアウト、フィル、ストローク、エフェクト、テキスト、画像、ベクターを保持して `.fig` ファイルをインポート

**デスクトップアプリ**
- Electron によるネイティブ macOS・Windows・Linux 対応
- `.op` ファイル関連付け — ダブルクリックで開く、シングルインスタンスロック
- GitHub Releases からの自動アップデート
- ネイティブアプリケーションメニューとファイルダイアログ

## 技術スタック

| | |
| --- | --- |
| **フロントエンド** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **キャンバス** | CanvasKit/Skia（WASM、GPU アクセラレーション） |
| **状態管理** | Zustand v5 |
| **サーバー** | Nitro |
| **デスクトップ** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **ランタイム** | Bun · Vite 7 |
| **ファイル形式** | `.op` — JSON ベース、人間が読みやすく、Git フレンドリー |

## プロジェクト構成

```text
src/
  canvas/          CanvasKit/Skia エンジン — 描画、同期、レイアウト、ガイド、ペンツール
  components/      React UI — エディター、パネル、共有ダイアログ、アイコン
  services/ai/     AI チャット、オーケストレーター、デザイン生成、ストリーミング
  services/figma/  Figma .fig バイナリインポートパイプライン
  services/codegen React+Tailwind および HTML+CSS コードジェネレーター
  stores/          Zustand — キャンバス、ドキュメント、ページ、履歴、AI、設定
  variables/       デザイントークンの解決とリファレンス管理
  mcp/             外部 CLI 統合用 MCP サーバーツール
  uikit/           再利用可能なコンポーネントキットシステム
server/
  api/ai/          Nitro API — ストリーミングチャット、生成、バリデーション
  utils/           Claude CLI、OpenCode、Codex、Copilot クライアントラッパー
electron/
  main.ts          ウィンドウ、Nitro フォーク、ネイティブメニュー、自動アップデーター
  preload.ts       IPC ブリッジ
```

## キーボードショートカット

| キー | 操作 | | キー | 操作 |
| --- | --- | --- | --- | --- |
| `V` | 選択 | | `Cmd+S` | 保存 |
| `R` | 矩形 | | `Cmd+Z` | 元に戻す |
| `O` | 楕円 | | `Cmd+Shift+Z` | やり直す |
| `L` | 直線 | | `Cmd+C/X/V/D` | コピー/カット/ペースト/複製 |
| `T` | テキスト | | `Cmd+G` | グループ化 |
| `F` | Frame | | `Cmd+Shift+G` | グループ解除 |
| `P` | ペンツール | | `Cmd+Shift+E` | エクスポート |
| `H` | ハンド（パン） | | `Cmd+Shift+C` | コードパネル |
| `Del` | 削除 | | `Cmd+Shift+V` | 変数パネル |
| `[ / ]` | 重ね順の変更 | | `Cmd+J` | AI チャット |
| 矢印キー | 1px 微調整 | | `Cmd+,` | エージェント設定 |
| `Cmd+Alt+U` | ブーリアン合体 | | `Cmd+Alt+S` | ブーリアン型抜き |
| `Cmd+Alt+I` | ブーリアン交差 | | | |

## スクリプト

```bash
bun --bun run dev          # 開発サーバー（ポート 3000）
bun --bun run build        # 本番ビルド
bun --bun run test         # テストの実行（Vitest）
npx tsc --noEmit           # 型チェック
bun run electron:dev       # Electron 開発モード
bun run electron:build     # Electron パッケージング
```

## コントリビュート

コントリビューションを歓迎します！アーキテクチャの詳細とコードスタイルについては [CLAUDE.md](./CLAUDE.md) をご覧ください。

1. フォークしてクローン
2. ブランチを作成：`git checkout -b feat/my-feature`
3. チェックを実行：`npx tsc --noEmit && bun --bun run test`
4. [Conventional Commits](https://www.conventionalcommits.org/) 形式でコミット：`feat(canvas): add rotation snapping`
5. `main` ブランチに PR を作成

## ロードマップ

- [x] CSS 同期付きデザイン変数とトークン
- [x] コンポーネントシステム（インスタンスとオーバーライド）
- [x] オーケストレーター付き AI デザイン生成
- [x] レイヤードデザインワークフロー付き MCP サーバー統合
- [x] マルチページサポート
- [x] Figma `.fig` インポート
- [x] ブーリアン演算（合体、型抜き、交差）
- [x] マルチモデル能力プロファイル
- [ ] 共同編集
- [ ] プラグインシステム

## コントリビューター

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## コミュニティ

<a href="https://discord.gg/KwXp6BJD">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Discord に参加する</strong>
</a>
— 質問、デザインの共有、機能のリクエストはこちら。


## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## ライセンス

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

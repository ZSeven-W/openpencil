# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [**日本語**](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

[OpenPencil](https://github.com/ZSeven-W/openpencil) 用 CLI — ターミナルからデザインツールを操作できます。

## インストール

```bash
npm install -g @zseven-w/openpencil
```

## プラットフォーム対応

CLI はすべてのプラットフォームで OpenPencil デスクトップアプリを自動検出して起動します：

| プラットフォーム | 検出されるインストールパス                                                                              |
| ---------------- | ------------------------------------------------------------------------------------------------------- |
| **macOS**        | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                         |
| **Windows**      | NSIS ユーザー単位 (`%LOCALAPPDATA%`)、マシン単位 (`%PROGRAMFILES%`)、ポータブル                          |
| **Linux**        | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak  |

## 使い方

```bash
op <command> [options]
```

### 入力方法

JSON または DSL を受け付ける引数は、3 つの方法で渡すことができます：

```bash
op design '...'              # インライン文字列（小さなペイロード向け）
op design @design.txt        # ファイルから読み込み（大きなデザインに推奨）
cat design.txt | op design - # 標準入力から読み込み（パイプ）
```

### アプリ制御

```bash
op start [--desktop|--web]   # OpenPencil を起動（デフォルトはデスクトップ）
op stop                      # 実行中のインスタンスを停止
op status                    # 実行中かどうかを確認
```

### デザイン（バッチ DSL）

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### ドキュメント操作

```bash
op open [file.op]            # ファイルを開く、またはライブキャンバスに接続
op save <file.op>            # 現在のドキュメントを保存
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # 現在のキャンバスの選択を取得
```

### ノード操作

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### コードエクスポート

```bash
op export <format> [--out file]
# フォーマット: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### 変数とテーマ

```bash
op vars                      # 変数を取得
op vars:set <json>           # 変数を設定
op themes                    # テーマを取得
op themes:set <json>         # テーマを設定
op theme:save <file.optheme> # テーマプリセットを保存
op theme:load <file.optheme> # テーマプリセットを読み込み
op theme:list [dir]          # テーマプリセットを一覧表示
```

### ページ

```bash
op page list                 # ページを一覧表示
op page add [--name N]       # ページを追加
op page remove <id>          # ページを削除
op page rename <id> <name>   # ページの名前を変更
op page reorder <id> <index> # ページを並べ替え
op page duplicate <id>       # ページを複製
```

### インポート

```bash
op import:svg <file.svg>     # SVG ファイルをインポート
op import:figma <file.fig>   # Figma .fig ファイルをインポート
```

### レイアウト

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### グローバルフラグ

```text
--file <path>     対象の .op ファイル（デフォルト: ライブキャンバス）
--page <id>       対象のページ ID
--pretty          人間が読みやすい JSON 出力
--help            ヘルプを表示
--version         バージョンを表示
```

## ライセンス

MIT

# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [**繁體中文**](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

[OpenPencil](https://github.com/ZSeven-W/openpencil) 的命令列工具 — 從終端機控制設計工具。

## 安裝

```bash
npm install -g @zseven-w/openpencil
```

## 平台支援

CLI 會自動偵測並啟動所有平台上的 OpenPencil 桌面應用程式：

| 平台        | 偵測的安裝路徑                                                                                      |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`、`~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS 使用者安裝（`%LOCALAPPDATA%`）、全域安裝（`%PROGRAMFILES%`）、可攜版                            |
| **Linux**   | `/usr/bin`、`/usr/local/bin`、`~/.local/bin`、AppImage（`~/Applications`、`~/Downloads`）、Snap、Flatpak |

## 使用方式

```bash
op <command> [options]
```

### 輸入方式

接受 JSON 或 DSL 的參數可透過三種方式傳入：

```bash
op design '...'              # 內嵌字串（適合小型內容）
op design @design.txt        # 從檔案讀取（建議用於大型設計）
cat design.txt | op design - # 從標準輸入讀取（管線傳輸）
```

### 應用程式控制

```bash
op start [--desktop|--web]   # 啟動 OpenPencil（預設為桌面版）
op stop                      # 停止執行中的實例
op status                    # 檢查是否正在執行
```

### 設計（批次 DSL）

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### 文件操作

```bash
op open [file.op]            # 開啟檔案或連線至即時畫布
op save <file.op>            # 儲存目前的文件
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # 取得目前畫布的選取項目
```

### 節點操作

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### 程式碼匯出

```bash
op export <format> [--out file]
# 格式：react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### 變數與主題

```bash
op vars                      # 取得變數
op vars:set <json>           # 設定變數
op themes                    # 取得主題
op themes:set <json>         # 設定主題
op theme:save <file.optheme> # 儲存主題預設
op theme:load <file.optheme> # 載入主題預設
op theme:list [dir]          # 列出主題預設
```

### 頁面

```bash
op page list                 # 列出頁面
op page add [--name N]       # 新增頁面
op page remove <id>          # 移除頁面
op page rename <id> <name>   # 重新命名頁面
op page reorder <id> <index> # 重新排序頁面
op page duplicate <id>       # 複製頁面
```

### 匯入

```bash
op import:svg <file.svg>     # 匯入 SVG 檔案
op import:figma <file.fig>   # 匯入 Figma .fig 檔案
```

### 版面配置

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### 全域旗標

```text
--file <path>     目標 .op 檔案（預設：即時畫布）
--page <id>       目標頁面 ID
--pretty          人類可讀的 JSON 輸出
--help            顯示說明
--version         顯示版本
```

## 授權條款

MIT

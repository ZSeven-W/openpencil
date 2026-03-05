<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>AI 原生開源設計工具。設計即程式碼。</strong><br />
  從提示詞到畫布 UI。多智能體編排。內建 MCP 伺服器。程式碼生成。
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <b>繁體中文</b> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#quick-start">快速開始</a> ·
  <a href="#ai-native-design">AI</a> ·
  <a href="#features">功能特色</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#contributing">參與貢獻</a>
</p>

## 快速開始

```bash
# 安裝相依套件
bun install

# 在 http://localhost:3000 啟動開發伺服器
bun --bun run dev
```

或以桌面應用程式形式執行：

```bash
bun run electron:dev
```

> **前置條件：** [Bun](https://bun.sh/) >= 1.0 以及 [Node.js](https://nodejs.org/) >= 18

## AI 原生設計

OpenPencil 從底層就圍繞 AI 構建——不是作為外掛程式，而是作為核心工作流程。

**提示詞生成 UI**
- **文字轉設計** — 描述一個頁面，即時以串流動畫在畫布上生成
- **編排器** — 將複雜頁面分解為空間子任務，支援並行生成
- **設計修改** — 選取元素後，以自然語言描述變更
- **視覺輸入** — 附加截圖或線框圖作為參考進行設計

**多智能體支援**

| 智能體 | 設定方式 |
| --- | --- |
| **Claude Code** | 無需設定 — 使用 Claude Agent SDK 本地 OAuth |
| **Codex CLI** | 在 Agent 設定中連接（`Cmd+,`） |
| **OpenCode** | 在 Agent 設定中連接（`Cmd+,`） |

**MCP 伺服器**
- 內建 MCP 伺服器 — 一鍵安裝至 Claude Code / Codex / Gemini / OpenCode / Kiro CLI
- 從終端機進行設計自動化：透過任意 MCP 相容的智能體讀取、建立和修改 `.op` 檔案

**程式碼生成**
- React + Tailwind CSS
- HTML + CSS
- 從設計令牌生成 CSS Variables

## 功能特色

**畫布與繪圖**
- 無限畫布，支援平移、縮放、智慧對齊參考線和吸附
- 矩形、橢圓、直線、多邊形、鋼筆（貝茲曲線）、Frame、文字
- 圖示選擇器（Iconify）和圖片匯入（PNG/JPEG/SVG/WebP/GIF）
- 自動版面配置 — 垂直/水平方向，支援間距、內邊距、主軸對齊、交叉軸對齊
- 多頁面文件，支援分頁導覽

**設計系統**
- 設計變數 — 顏色、數字、字串令牌，支援 `$variable` 參照
- 多主題支援 — 多個主題軸，每個軸有多個變體（亮色/暗色、緊湊/舒適）
- 元件系統 — 可重複使用元件，支援實體和覆寫
- CSS 同步 — 自動生成自訂屬性，程式碼輸出中使用 `var(--name)`

**Figma 匯入**
- 匯入 `.fig` 檔案，保留版面配置、填色、筆觸、效果、文字、圖片和向量圖形

**桌面應用程式**
- 透過 Electron 支援原生 macOS、Windows 和 Linux
- 從 GitHub Releases 自動更新
- 原生應用程式選單和檔案對話框

## 技術堆疊

| | |
| --- | --- |
| **前端** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **畫布** | Fabric.js v7 |
| **狀態管理** | Zustand v5 |
| **伺服器** | Nitro |
| **桌面端** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **執行環境** | Bun · Vite 7 |
| **檔案格式** | `.op` — 基於 JSON，人類可讀，對 Git 友好 |

## 專案結構

```text
src/
  canvas/          Fabric.js 引擎 — 繪圖、同步、版面配置、參考線、鋼筆工具
  components/      React UI — 編輯器、面板、共用對話框、圖示
  services/ai/     AI 聊天、編排器、設計生成、串流處理
  services/figma/  Figma .fig 二進位檔案匯入管線
  services/codegen React+Tailwind 和 HTML+CSS 程式碼生成器
  stores/          Zustand — 畫布、文件、頁面、歷程、AI、設定
  variables/       設計令牌解析與參照管理
  mcp/             供外部 CLI 整合使用的 MCP 伺服器工具
  uikit/           可重複使用元件套件系統
server/
  api/ai/          Nitro API — 串流聊天、生成、驗證
  utils/           Claude CLI、OpenCode、Codex 客戶端封裝
electron/
  main.ts          視窗、Nitro 子處理序、原生選單、自動更新
  preload.ts       IPC 橋接
```

## 鍵盤快捷鍵

| 按鍵 | 操作 | | 按鍵 | 操作 |
| --- | --- | --- | --- | --- |
| `V` | 選取 | | `Cmd+S` | 儲存 |
| `R` | 矩形 | | `Cmd+Z` | 復原 |
| `O` | 橢圓 | | `Cmd+Shift+Z` | 重做 |
| `L` | 直線 | | `Cmd+C/X/V/D` | 複製/剪下/貼上/重複 |
| `T` | 文字 | | `Cmd+G` | 群組 |
| `F` | Frame | | `Cmd+Shift+G` | 解散群組 |
| `P` | 鋼筆工具 | | `Cmd+Shift+E` | 匯出 |
| `H` | 手形（平移） | | `Cmd+Shift+C` | 程式碼面板 |
| `Del` | 刪除 | | `Cmd+Shift+V` | 變數面板 |
| `[ / ]` | 調整圖層順序 | | `Cmd+J` | AI 聊天 |
| 方向鍵 | 微移 1px | | `Cmd+,` | 智能體設定 |

## 指令碼命令

```bash
bun --bun run dev          # 開發伺服器（連接埠 3000）
bun --bun run build        # 正式版建置
bun --bun run test         # 執行測試（Vitest）
npx tsc --noEmit           # 型別檢查
bun run electron:dev       # Electron 開發模式
bun run electron:build     # Electron 封裝
```

## 參與貢獻

歡迎貢獻！請查閱 [CLAUDE.md](./CLAUDE.md) 了解架構細節和程式碼風格。

1. Fork 並複製存放庫
2. 建立分支：`git checkout -b feat/my-feature`
3. 執行檢查：`npx tsc --noEmit && bun --bun run test`
4. 使用 [Conventional Commits](https://www.conventionalcommits.org/) 提交：`feat(canvas): add rotation snapping`
5. 向 `main` 分支發起 PR

## 路線圖

- [x] 設計變數與令牌，支援 CSS 同步
- [x] 元件系統（實體與覆寫）
- [x] 帶編排器的 AI 設計生成
- [x] MCP 伺服器整合
- [x] 多頁面支援
- [x] Figma `.fig` 匯入
- [ ] 布林運算（聯集、減去、交集）
- [ ] 協同編輯
- [ ] 外掛程式系統

## 貢獻者

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## 社群

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> 加入我們的 Discord</strong>
</a>
— 提問、分享設計、提出功能建議。

## 授權條款

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

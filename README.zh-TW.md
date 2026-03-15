<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>全球首個開源 AI 原生向量設計工具。</strong><br />
  <sub>並行智能體團隊 &bull; 設計即程式碼 &bull; 內建 MCP 伺服器 &bull; 多模型智慧</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <b>繁體中文</b> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — 點擊觀看示範影片" width="100%" />
  </a>
</p>
<p align="center"><sub>點擊圖片觀看示範影片</sub></p>

<br />

## 為什麼選擇 OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 提示詞 → 畫布

用自然語言描述任何 UI。即時以串流動畫在無限畫布上生成。選取元素並透過對話修改現有設計。

</td>
<td width="50%">

### 🤖 並行智能體團隊

編排器將複雜頁面分解為空間子任務。多個 AI 智能體同時處理不同區塊 — 主視覺、功能區塊、頁尾 — 全部並行串流生成。

</td>
</tr>
<tr>
<td width="50%">

### 🧠 多模型智慧

自動適配每個模型的能力。Claude 獲得完整提示詞與思考模式；GPT-4o/Gemini 停用思考模式；較小模型（MiniMax、Qwen、Llama）獲得精簡提示詞，確保輸出可靠。

</td>
<td width="50%">

### 🔌 MCP 伺服器

一鍵安裝至 Claude Code、Codex、Gemini、OpenCode、Kiro 或 Copilot CLI。從終端機進行設計 — 透過任意 MCP 相容的智能體讀取、建立和修改 `.op` 檔案。

</td>
</tr>
<tr>
<td width="50%">

### 📦 設計即程式碼

`.op` 檔案是 JSON — 人類可讀、對 Git 友好、可差異比較。設計變數生成 CSS 自訂屬性。程式碼匯出為 React + Tailwind 或 HTML + CSS。

</td>
<td width="50%">

### 🖥️ 隨處執行

Web 應用程式 + 透過 Electron 在 macOS、Windows 和 Linux 上原生執行。從 GitHub Releases 自動更新。`.op` 檔案關聯 — 雙擊即可開啟。

</td>
</tr>
</table>

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
| **GitHub Copilot** | 執行 `copilot login` 後在 Agent 設定中連接（`Cmd+,`） |

**模型能力設定檔** — 自動依據模型層級調整提示詞、思考模式和逾時設定。完整層級模型（Claude）獲得完整提示詞；標準層級（GPT-4o、Gemini、DeepSeek）停用思考模式；基礎層級（MiniMax、Qwen、Llama、Mistral）獲得精簡巢狀 JSON 提示詞，確保最大可靠性。

**MCP 伺服器**
- 內建 MCP 伺服器 — 一鍵安裝至 Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI
- 自動偵測 Node.js — 若未安裝則自動回退到 HTTP 傳輸模式並啟動 MCP HTTP 伺服器
- 從終端機進行設計自動化：透過任意 MCP 相容的智能體讀取、建立和修改 `.op` 檔案
- **分層設計工作流** — `design_skeleton` → `design_content` → `design_refine`，適用於高保真多區塊設計
- **分段提示詞擷取** — 僅載入所需的設計知識（schema、layout、roles、icons、planning 等）
- 多頁面支援 — 透過 MCP 工具建立、重新命名、重新排序和複製頁面

**程式碼生成**
- React + Tailwind CSS、HTML + CSS、CSS Variables
- Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Native

## 功能特色

**畫布與繪圖**
- 無限畫布，支援平移、縮放、智慧對齊參考線和吸附
- 矩形、橢圓、直線、多邊形、鋼筆（貝茲曲線）、Frame、文字
- 布林運算 — 聯合、減去、交集，搭配上下文工具列
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
- `.op` 檔案關聯 — 雙擊即可開啟，支援單一實體鎖定
- 從 GitHub Releases 自動更新
- 原生應用程式選單和檔案對話框

## 技術堆疊

| | |
| --- | --- |
| **前端** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **畫布** | CanvasKit/Skia（WASM、GPU 加速） |
| **狀態管理** | Zustand v5 |
| **伺服器** | Nitro |
| **桌面端** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **執行環境** | Bun · Vite 7 |
| **檔案格式** | `.op` — 基於 JSON，人類可讀，對 Git 友好 |

## 專案結構

```text
src/
  canvas/          CanvasKit/Skia 引擎 — 繪圖、同步、版面配置、參考線、鋼筆工具
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
  utils/           Claude CLI、OpenCode、Codex、Copilot 客戶端封裝
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
| `Cmd+Alt+U` | 布林聯合 | | `Cmd+Alt+S` | 布林減去 |
| `Cmd+Alt+I` | 布林交集 | | | |

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
- [x] MCP 伺服器整合，支援分層設計工作流
- [x] 多頁面支援
- [x] Figma `.fig` 匯入
- [x] 布林運算（聯集、減去、交集）
- [x] 多模型能力設定檔
- [ ] 協同編輯
- [ ] 外掛程式系統

## 貢獻者

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## 社群

<a href="https://discord.gg/KwXp6BJD">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> 加入我們的 Discord</strong>
</a>
— 提問、分享設計、提出功能建議。


## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## 授權條款

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

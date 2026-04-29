<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>全球首個開源 AI 原生向量設計工具。</strong><br />
  <sub>並行智能體團隊 &bull; 設計即程式碼 &bull; 內建 MCP 伺服器 &bull; 多模型智慧</sub>
</p>

<p align="center">
  <a href="./README.md"><b>English</b></a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — 點擊觀看示範影片" width="100%" />
  </a>
</p>
<p align="center"><sub>點擊圖片觀看示範影片</sub></p>

<br />

> **注意：** 另有一個同名的開源專案 — [OpenPencil](https://github.com/open-pencil/open-pencil)，專注於相容 Figma 的視覺設計與即時協作。本專案專注於 AI 原生的設計轉程式碼工作流。

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
<tr>
<td width="50%">

### ⌨️ CLI — `op`

從終端機控制設計工具。`op design`、`op insert` — 批次設計 DSL、節點操作。支援從檔案或 stdin 管道輸入。可搭配桌面應用程式或 Web 伺服器使用。

</td>
<td width="50%">

### 🎯 多平台程式碼匯出

從單個 `.op` 檔案匯出至 React + Tailwind、HTML + CSS、Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Native。設計變數自動轉換為 CSS 自訂屬性。

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

### Docker

提供多種映像檔變體 — 選擇適合您需求的版本：

| 映像檔                       | 大小    | 包含                 |
| ---------------------------- | ------- | -------------------- |
| `openpencil:latest`          | ~226 MB | 僅 Web 應用程式      |
| `openpencil-claude:latest`   | —       | + Claude Code CLI    |
| `openpencil-codex:latest`    | —       | + Codex CLI          |
| `openpencil-opencode:latest` | —       | + OpenCode CLI       |
| `openpencil-copilot:latest`  | —       | + GitHub Copilot CLI |
| `openpencil-gemini:latest`   | —       | + Gemini CLI         |
| `openpencil-full:latest`     | ~1 GB   | 所有 CLI 工具        |

**執行（僅 Web）：**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**搭配 AI CLI 執行（例如 Claude Code）：**

AI 聊天功能依賴 Claude CLI OAuth 登入。使用 Docker volume 來保留登入狀態：

```bash
# 步驟 1 — 登入（僅需一次）
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# 步驟 2 — 啟動
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**本地建置：**

```bash
# 基礎（僅 Web）
docker build --target base -t openpencil .

# 搭配特定 CLI
docker build --target with-claude -t openpencil-claude .

# 完整版（所有 CLI）
docker build --target full -t openpencil-full .
```

## AI 原生設計

**提示詞生成 UI**

- **文字轉設計** — 描述一個頁面，即時以串流動畫在畫布上生成
- **編排器** — 將複雜頁面分解為空間子任務，支援並行生成
- **設計修改** — 選取元素後，以自然語言描述變更
- **視覺輸入** — 附加截圖或線框圖作為參考進行設計

**多智能體支援**

| 智能體                | 設定方式                                                              |
| --------------------- | --------------------------------------------------------------------- |
| **內建（9+ 提供商）** | 從提供商預設中選擇並切換區域 — Anthropic、OpenAI、Google、DeepSeek 等 |
| **Claude Code**       | 無需設定 — 使用 Claude Agent SDK 本地 OAuth                           |
| **Codex CLI**         | 在 Agent 設定中連接（`Cmd+,`）                                        |
| **OpenCode**          | 在 Agent 設定中連接（`Cmd+,`）                                        |
| **GitHub Copilot**    | 執行 `copilot login` 後在 Agent 設定中連接（`Cmd+,`）                 |
| **Gemini CLI**        | 在 Agent 設定中連接（`Cmd+,`）                                        |

**模型能力設定檔** — 自動依據模型層級調整提示詞、思考模式和逾時設定。完整層級模型（Claude）獲得完整提示詞；標準層級（GPT-4o、Gemini、DeepSeek）停用思考模式；基礎層級（MiniMax、Qwen、Llama、Mistral）獲得精簡巢狀 JSON 提示詞，確保最大可靠性。

**國際化** — 完整介面本地化，支援 15 種語言：English、简体中文、繁體中文、日本語、한국어、Français、Español、Deutsch、Português、Русский、हिन्दी、Türkçe、ไทย、Tiếng Việt、Bahasa Indonesia。

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

## CLI — `op`

全域安裝後即可從終端機控制設計工具：

```bash
npm install -g @zseven-w/openpencil
```

```bash
op start                     # 啟動桌面應用程式
op design @landing.txt       # 從檔案批次設計
op insert '{"type":"RECT"}'  # 插入節點
op import:figma design.fig   # 匯入 Figma 檔案
cat design.dsl | op design - # 從 stdin 管道輸入
```

支援三種輸入方式：內嵌字串、`@filepath`（從檔案讀取）、`-`（從 stdin 讀取）。可搭配桌面應用程式或 Web 開發伺服器使用。完整命令參考請查閱 [CLI README](./apps/cli/README.md)。

**LLM 技能** — 安裝 [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) 外掛，教 AI 智慧體（Claude Code、Cursor、Codex、Gemini CLI 等）使用 `op` 進行設計。

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
- 可重複使用 UIKit — 從 `.pen` 檔案匯入/匯出元件套件

**AI 與智能體**

- 提示詞轉畫布，支援串流生成與編排器驅動的空間分解
- 並發 Agent 團隊 — 多位設計師並行處理不同區塊，每位成員帶畫布指示器
- 分層工作流 — `design_skeleton` → `design_content` → `design_refine`，每個階段使用聚焦的提示詞
- 風格指南 — 50+ 內建風格（glassmorphism、brutalist、retro 等），支援基於標籤的模糊匹配，並接入規劃與生成流程
- 多模型能力設定檔 — 依模型層級自動適配思考模式、推理強度與提示詞形態
- 內建智能體執行環境（`agent-native`，Zig NAPI）+ Anthropic、Claude Agent SDK、OpenCode、Codex、Copilot、Gemini 提供商
- 中國大型語言模型 Anthropic 格式透傳 — Kimi、Zhipu、GLM、DouBao、Ark、Bailian/DashScope、ModelScope、Coding Plans

**Git 整合**

- 複製精靈，支援 SSH / HTTPS 認證與 SSH 金鑰管理
- 分支選擇器 — 建立、切換、刪除、合併，全部在 Git 面板中完成
- 拉取 / 推送級聯，支援認證重試與非快轉推送處理
- 資料夾模式三路合併，在磁碟上追蹤 `MERGE_HEAD` 狀態
- 衝突面板 — 提供逐節點 / 逐欄位三路卡片、內嵌 JSON 編輯器、批次操作與內嵌 diff 區塊
- 遠端設定與 SSH 金鑰介面；整個 Git 功能涵蓋 15 種語言的 i18n

**匯出**

- 畫布匯出 — PNG、JPEG、WEBP、PDF（`Cmd+Shift+P`）
- 程式碼匯出 — React + Tailwind、HTML + CSS、Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Native
- 增量 MCP 程式碼生成流水線 — `codegen_plan`、`codegen_submit_chunk`、`codegen_assemble`、`codegen_clean`

**Figma 匯入**

- 匯入 `.fig` 檔案，保留版面配置、填色、筆觸、效果、文字、圖片和向量圖形

**桌面應用程式**

- 透過 Electron 支援原生 macOS、Windows 和 Linux
- `.op` 檔案關聯 — 雙擊即可開啟，支援單一實體鎖定
- 從 GitHub Releases 自動更新
- 原生應用程式選單，支援另存新檔、開啟最近使用，以及關閉時的未儲存變更對話框
- 最近使用檔案持久化

## 技術堆疊

|              |                                                                                  |
| ------------ | -------------------------------------------------------------------------------- |
| **前端**     | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui · i18next                |
| **畫布**     | CanvasKit/Skia（WASM、GPU 加速）                                                 |
| **狀態管理** | Zustand v5                                                                       |
| **伺服器**   | Nitro                                                                            |
| **桌面端**   | Electron 35                                                                      |
| **CLI**      | `op` — 終端機控制、批次設計 DSL                                                  |
| **AI**       | Vercel AI SDK v6 · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **執行環境** | Bun · Vite 7                                                                     |
| **檔案格式** | `.op` — 基於 JSON，人類可讀，對 Git 友好                                         |

## 專案結構

```text
openpencil/
├── apps/
│   ├── web/                 TanStack Start Web 應用程式
│   │   ├── src/
│   │   │   ├── canvas/      CanvasKit/Skia 引擎 — 繪圖、同步、版面配置
│   │   │   ├── components/  React UI — 編輯器、面板、共用對話框、圖示
│   │   │   ├── services/ai/ AI 聊天、編排器、設計生成、串流處理
│   │   │   ├── stores/      Zustand — 畫布、文件、頁面、歷程、AI
│   │   │   ├── mcp/         供外部 CLI 整合使用的 MCP 伺服器工具
│   │   │   ├── hooks/       鍵盤快捷鍵、檔案拖放、Figma 貼上
│   │   │   └── uikit/       可重複使用元件套件系統
│   │   └── server/
│   │       ├── api/ai/      Nitro API — 串流聊天、生成、驗證
│   │       └── utils/       Claude CLI、OpenCode、Codex、Copilot 客戶端封裝
│   ├── desktop/             Electron 桌面應用程式
│   │   ├── main.ts          視窗、Nitro 子處理序、原生選單、自動更新
│   │   ├── ipc-handlers.ts  原生檔案對話框、主題同步、偏好設定 IPC
│   │   └── preload.ts       IPC 橋接
│   └── cli/                 CLI 工具 — `op` 命令
│       ├── src/commands/    設計、文件、匯出、匯入、節點、頁面、變數命令
│       ├── connection.ts    與執行中應用程式的 WebSocket 連線
│       └── launcher.ts      自動偵測並啟動桌面應用程式或 Web 伺服器
├── packages/
│   ├── pen-types/           PenDocument 模型型別定義
│   ├── pen-core/            文件樹操作、版面配置引擎、變數
│   ├── pen-codegen/         程式碼生成器（React、HTML、Vue、Flutter...）
│   ├── pen-figma/           Figma .fig 檔案解析器與轉換器
│   ├── pen-renderer/        獨立 CanvasKit/Skia 渲染器
│   ├── pen-sdk/             整合 SDK（重新匯出所有套件）
│   ├── pen-ai-skills/       AI 提示詞技能引擎（分階段 prompt 載入）
│   └── agent/               AI Agent SDK（Vercel AI SDK、多提供商、Agent 團隊）
└── .githooks/               Pre-commit 版本號同步（從分支名稱）
```

## 鍵盤快捷鍵

| 按鍵        | 操作         |     | 按鍵          | 操作                    |
| ----------- | ------------ | --- | ------------- | ----------------------- |
| `V`         | 選取         |     | `Cmd+S`       | 儲存                    |
| `R`         | 矩形         |     | `Cmd+Z`       | 復原                    |
| `O`         | 橢圓         |     | `Cmd+Shift+Z` | 重做                    |
| `L`         | 直線         |     | `Cmd+C/X/V/D` | 複製/剪下/貼上/重複     |
| `T`         | 文字         |     | `Cmd+G`       | 群組                    |
| `F`         | Frame        |     | `Cmd+Shift+G` | 解散群組                |
| `P`         | 鋼筆工具     |     | `Cmd+Shift+P` | 匯出 (PNG/JPG/WEBP/PDF) |
| `H`         | 手形（平移） |     | `Cmd+Shift+C` | 程式碼面板              |
| `Del`       | 刪除         |     | `Cmd+Shift+V` | 變數面板                |
| `[ / ]`     | 調整圖層順序 |     | `Cmd+J`       | AI 聊天                 |
| 方向鍵      | 微移 1px     |     | `Cmd+,`       | 智能體設定              |
| `Cmd+Alt+U` | 布林聯合     |     | `Cmd+Alt+S`   | 布林減去                |
| `Cmd+Alt+I` | 布林交集     |     | `Cmd+Shift+S` | 另存新檔                |

## 指令碼命令

```bash
bun --bun run dev          # 開發伺服器（連接埠 3000）
bun --bun run build        # 正式版建置
bun --bun run test         # 執行測試（Vitest）
npx tsc --noEmit           # 型別檢查
bun run bump <version>     # 在所有 package.json 間同步版本號
bun run electron:dev       # Electron 開發模式
bun run electron:build     # Electron 封裝
bun run cli:dev            # 從原始碼執行 CLI
bun run cli:compile        # 編譯 CLI 到 dist
```

## 參與貢獻

歡迎貢獻！請查閱 [CLAUDE.md](./CLAUDE.md) 了解架構細節和程式碼風格。

1. Fork 並複製存放庫
2. 設定版本同步：`git config core.hooksPath .githooks`
3. 建立分支：`git checkout -b feat/my-feature`
4. 執行檢查：`npx tsc --noEmit && bun --bun run test`
5. 使用 [Conventional Commits](https://www.conventionalcommits.org/) 提交：`feat(canvas): add rotation snapping`
6. 向 `main` 分支發起 PR

## 路線圖

- [x] 設計變數與令牌，支援 CSS 同步
- [x] 元件系統（實體與覆寫）
- [x] 帶編排器的 AI 設計生成
- [x] MCP 伺服器整合，支援分層設計工作流
- [x] 多頁面支援
- [x] Figma `.fig` 匯入
- [x] 布林運算（聯集、減去、交集）
- [x] 多模型能力設定檔
- [x] Monorepo 重構，支援可重複使用套件
- [x] CLI 工具（`op`）終端控制
- [x] 內建 AI Agent SDK，支援多提供商
- [x] 國際化 — 15 種語言
- [x] Git 整合（複製、分支、推送/拉取、資料夾模式三路合併）
- [x] 畫布點陣圖匯出（PNG / JPEG / WEBP / PDF）
- [ ] 協同編輯
- [ ] 外掛程式系統

## 貢獻者

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## 贊助者

OpenPencil 免費且開源,開發完全由覺得它好用的人們贊助 —— 感謝你讓這塊畫布一直保持開放。

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

感謝 **[MrQyun](https://github.com/mrqyun)** —— 想把自己的名字也放在這裡?**[成為贊助者 →](https://github.com/sponsors/ZSeven-W)**

## 社群

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
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

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

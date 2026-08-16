<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/rust-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/rust-check.yml?label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — 點擊觀看示範影片" width="100%" />
  </a>
</p>
<p align="center"><sub>點擊圖片觀看示範影片</sub></p>

<br />

<p align="center">
  <a href="https://github.com/InnerAura/hyperweave" title="Star history rendered with HyperWeave">
    <img src="./screenshot/openpencil-star-history.svg" alt="OpenPencil star history rendered by HyperWeave" width="100%" />
  </a>
</p>
<p align="center">
  <sub>The trend above is rendered with <a href="https://github.com/InnerAura/hyperweave"><b>HyperWeave</b></a> — an experimental SVG artifact layer for agents. &nbsp;<a href="https://github.com/InnerAura/hyperweave"><b>Explore&nbsp;→</b></a></sub>
</p>

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

一鍵安裝至 Claude Code、Codex、OpenCode、Kiro 或 Copilot CLI。從終端機進行設計 — 透過任意 MCP 相容的智能體讀取、建立和修改 `.op` 檔案。

</td>
</tr>
<tr>
<td width="50%">

### 📦 設計即程式碼

`.op` 檔案是 JSON — 人類可讀、對 Git 友好、可差異比較。設計變數生成 CSS 自訂屬性。程式碼匯出為 React + Tailwind 或 HTML + CSS。

</td>
<td width="50%">

### 🖥️ 隨處執行

Web 應用程式 + macOS、Windows 和 Linux 上的原生桌面應用程式 — 單一 Rust 核心、單一自包含二進位檔，無需瀏覽器引擎。`.op` 檔案關聯 — 雙擊即可開啟。

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

## 安裝

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

**Linux / Windows 直接下載：** [GitHub Releases](https://github.com/ZSeven-W/openpencil/releases) — `.exe`（Windows）、`.AppImage` / `.deb`（Linux）

**Nix（Linux x86_64）：**

```bash
nix develop
nix run .                         # 啟動桌面應用程式
nix build .#openpencil            # 原生 Web host + CanvasKit Web bundle
nix build .#op-cli                # `op` CLI
nix build .#prebuilt              # 使用對應的 upstream 桌面封存檔
nix build .#prebuilt-cli          # 使用對應的 upstream CLI 封存檔
nix build .#web-server            # 不依賴 GL 的原生 Web server + Web bundle
nix build .#runtime-prebuilt      # 預先建置的桌面端 + `op` CLI runtime
nix build .#web-sdk-packages      # Web SDK 的 npm tarball
nix build .#appimage              # 可攜式桌面 AppImage
```

flake 使用 `rust-toolchain.toml` 中固定的 Rust toolchain，目前發布於
`x86_64-linux`。flake 尚未產生 Debian 套件；需要 `.deb` 時請使用
upstream release artifact。`prebuilt` output 使用
`nix/release-manifest.json` 中固定的 release 版本與 hash，不受 workspace
原始碼版本影響。release 發布後，release workflow 會建立 PR 來更新此
manifest。在 PR 合併前，預先建置的 output 仍使用上一個已發布的 release；
從原始碼建置的 output 則始終使用目前 checkout 的原始碼。

**CLI（`op`）：**

```bash
brew install zseven-w/openpencil/op
```

或使用安裝指令碼（macOS / Linux）：

```bash
curl -fsSL https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.sh | bash
```

若要允許最新的預發布版本：

```bash
curl -fsSL https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.sh | OP_PRERELEASE=1 bash
```

Windows PowerShell：

```powershell
irm https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.ps1 | iex
```

若要允許最新的預發布版本：

```powershell
$env:OP_PRERELEASE = "1"; irm https://raw.githubusercontent.com/ZSeven-W/openpencil/main/scripts/install-op.ps1 | iex
```

## 複製（含子模組）

```bash
git clone --recurse-submodules https://github.com/ZSeven-W/openpencil.git
# 若已複製，請先同步，讓舊的子模組 URL 套用 .gitmodules 的變更：
git submodule sync --recursive && git submodule update --init --recursive
```

`vendor/` 下有三個子模組，全部公開並透過 HTTPS 取得（不需要 SSH 金鑰）：`jian`（GPU-Skia UI framework — widget/render/event）、`casement`（winit fork）以及 `agent`（`agent-rs` — OP 與 Zode 共用的跨產品 Rust agent runtime）。`vendor/anthropic-agent-sdk` 直接納入版本控制，並非子模組。

## 快速開始

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

或以桌面應用程式形式執行：

```bash
cargo run -p op-host-desktop
```

> **前置條件：** 建置產品需要 [Rust](https://www.rust-lang.org/)（stable）。[Bun](https://bun.sh/) >= 1.0 和 [Node.js](https://nodejs.org/) >= 18 僅用於 `packages/` 下的 web SDK。

### Docker

帶標籤的 Rust release 會發布單一 web-host 映像檔。內建 AI CLI 的舊 TypeScript 映像檔已不再發布。

| 映像檔 | 包含 |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:vX.Y.Z` | Rust web host、wasm bundle 和 CanvasKit 資源 |

Web UI 僅公開內建 agent profiles；Docker 映像檔不再內建 Claude/Codex/OpenCode/Copilot CLI 工具。

**執行：**

```bash
VERSION="$(scripts/workspace-version.sh)"
docker run -d -p 3100:3100 "ghcr.io/zseven-w/openpencil-web:v${VERSION}"
```

然後開啟 `http://localhost:3100/`。

**本地建置：**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
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

**模型能力設定檔** — 自動依據模型層級調整提示詞、思考模式和逾時設定。完整層級模型（Claude）獲得完整提示詞；標準層級（GPT-4o、Gemini、DeepSeek）停用思考模式；基礎層級（MiniMax、Qwen、Llama、Mistral）獲得精簡巢狀 JSON 提示詞，確保最大可靠性。

**國際化** — 完整介面本地化，支援 15 種語言：English、简体中文、繁體中文、日本語、한국어、Français、Español、Deutsch、Português、Русский、हिन्दी、Türkçe、ไทย、Tiếng Việt、Bahasa Indonesia。

**MCP 伺服器**

- 內建 MCP 伺服器（`op-mcp` crate）— 一鍵安裝至 Claude Code / Codex / OpenCode / Kiro / Copilot CLI
- 無需 Node.js — 透過桌面二進位檔（`--mcp <path>`）提供 stdio 傳輸，並由執行中的應用程式提供即時 HTTP 端點（`127.0.0.1:<port>/mcp`）
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
brew install zseven-w/openpencil/op
```

```bash
op start                     # 啟動桌面應用程式
op start --headless --file design.op # 啟動無頭伺服器
op design @landing.txt       # 從檔案批次設計
op design @ui.js             # 支援迴圈的沙箱 JavaScript
op insert '{"type":"rectangle"}' # 插入節點
op import:figma design.fig   # 匯入 Figma 檔案
cat design.dsl | op design - # 從 stdin 管道輸入
```

支援內嵌字串、`@filepath` 與 stdin（`-`），可搭配桌面應用程式、Web 伺服器或檔案型無頭伺服器使用。所有命令請查閱 [CLI 命令參考](./crates/op-cli/src/usage.txt)。

**LLM 技能** — 安裝 [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) 外掛，教 AI 智慧體使用 `op` 進行設計。可執行 `op install` 為偵測到的 Agent 安裝，或使用 `op install --target codex` 指定目標。

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
- 內建智能體執行環境（Rust）+ Anthropic、Claude Agent SDK、OpenCode、Codex、Copilot、Google Gemini API 提供商
- 中國大型語言模型 Anthropic 格式透傳 — Kimi、Zhipu、GLM、DouBao、Ark、Bailian/DashScope、ModelScope、Coding Plans

**協作**

- 經過身分驗證的點對點工作階段，支援公共中繼回退與內建的區域協作 Hub
- 使用 10 位帶區域標籤的配對碼加入 — 區域網路工作階段無需帳號
- 即時遠端游標、跨帳號協作，以及帶逐條編輯詳情和可重播已捨棄編輯的衝突面板
- `--serve-web` daemon 的線上多租戶模式，向 op-hub 進行身分驗證並支援跨帳號租戶共享
- 裝置登入 — 透過 serve-web daemon 從瀏覽器登入，編輯器中顯示頭像與使用者名稱

**簡報**

- 六個 16:9 簡報範本，配備範本選擇器，另有投影機尺寸的 AI 簡報規劃 — 一畫面一張投影片
- 以投影片放映方式進行簡報，配有簡報者控制
- 將簡報匯出為 PDF（每張投影片一頁）、自包含的投影片放映 HTML 檔案、可編輯的 PowerPoint（`.pptx`），或 hyperframes 影片合成
- 投影片軌道導覽器；Agent 會驗證簡報畫板的幾何（長寬比、溢位、置中）以及提示詞

**範本與網頁擷取**

- 場景範本中心 — 一個可瀏覽的目錄，涵蓋六大場景下的 58 個範本，從「檔案 ▸ 從範本新增」開啟
- 提示詞中心，包含 web、儀表板、元件與修改項目及視覺化提示詞預覽
- 素材中心 — 全視窗響應式圖庫，配有雙動作範本與 DESIGN.md 風格匯入
- [Chrome 網頁擷取擴充功能](https://chromewebstore.google.com/detail/openpencil/ahfofgelkdaobcmglejmjbpnajhpgcij) — 將任意網頁快照到 OpenPencil，支援高保真的 HTML / 版面擷取；快照會送達你帳號在 hub 上的收件匣，或在離線時下載為一個可直接開啟的 `.op` 檔案

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

- 原生 macOS、Windows 和 Linux — 單一自包含二進位檔（winit + GPU Skia，無 Electron）
- `.op` 檔案關聯 — 雙擊即可開啟，支援單一實體鎖定
- 背景檢查 GitHub Releases 更新
- 原生應用程式選單，支援另存新檔、開啟最近使用，以及關閉時的未儲存變更對話框
- 最近使用檔案持久化

## 技術堆疊

|                  |                                                                                    |
| ---------------- | ---------------------------------------------------------------------------------- |
| **核心**         | Rust workspace（`crates/`）— 編輯器狀態、widgets、hosts、MCP、AI、程式碼生成      |
| **渲染**         | 全平台 GPU Skia — 原生端使用 `skia-safe`（GL），瀏覽器端使用 CanvasKit（WASM/WebGL2）|
| **UI 框架**      | jian — 內建的純 Rust GPU-Skia UI 框架：widgets、版面配置、事件、熱重載（`vendor/jian`） |
| **視窗系統**     | winit（vendored `casement` fork）                                                 |
| **桌面端**       | 原生二進位檔 `openpencil-desktop` — 無需瀏覽器引擎                                |
| **Web SDK**      | `op-web-sdk` + React 19 / Vue 3 轉接器 — 唯讀 `.op` 檢視器（TypeScript）          |
| **CLI**          | `op` — 終端機控制、批次設計 DSL                                                   |
| **AI**           | 內建 Rust Agent 執行環境 · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**         | clippy · rustfmt（Rust）· oxlint · oxfmt（Web SDK）                              |
| **檔案格式**     | `.op` — 基於 JSON，人類可讀，對 Git 友好                                          |

## 生態系

OpenPencil 是 **[ZSeven-W](https://github.com/ZSeven-W)** 出品的一系列純 Rust、AI 原生工具家族中的一員。它們彼此協作：`jian` 渲染 OpenPencil，`agent-rs` 執行它的智能體，`noema` 負責記憶，`zode` 則從終端機進行設計。

| 專案 | 簡介 |
| ---- | ---- |
| **[DSH OpenPencil](https://github.com/ZSeven-W/dsh-openpencil)** | OpenPencil 的 DeepSeek Harness 外掛 — 在對話中提供精確的多畫板 `.op` 預覽、可互動畫布，以及具備代理原生設計工具的託管編輯器。 |
| **[Zode](https://github.com/ZSeven-W/zode)** | 面向終端機的開源、AI 原生程式設計助手 — 一個快速的 Rust TUI（`ratatui`），可讀取你的程式碼、執行命令、搜尋檔案並管理 git。透過 MCP 驅動 OpenPencil。 |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | 用於交付 LLM 智能體的純 Rust 非同步執行環境 — 多提供商、端到端工具能力、結構化權限、真正的 MCP、零 `unsafe`。為 OpenPencil 內建的智能體執行環境（`vendor/agent`）和 Zode 提供動力。 |
| **[jian](https://github.com/ZSeven-W/jian)** | 純 Rust、GPU-Skia UI 框架 — widgets、版面配置、事件和熱重載集於一棧。將宣告式的 `.op` 文件變為原生、AI 可控的應用程式，無需 JS 執行環境、無 DOM、無 Electron。OpenPencil 的 UI 框架（`vendor/jian`）。 |
| **[noema](https://github.com/ZSeven-W/noema)** | 面向程式設計智能體的本地優先、非向量記憶系統。以可檢視的檔案形式提供持久記憶、為新項目提供審閱佇列，以及詞彙式（無需嵌入）召回 — 適用於 Zode、Codex、Claude Code 和 MCP 執行環境。 |

## 為何選擇 Rust

OpenPencil 已以 **Rust** 從頭重寫完成（[#129](https://github.com/ZSeven-W/openpencil/issues/129)）。重寫工作已經完成——TypeScript + Electron 版本的編輯器已在 `v0.7.5` 退役，本存放庫中的 Rust workspace 就是產品本身：一個原生核心，體積大幅縮小、速度更快，並能從單一程式碼庫支援更多平台。

|                       | TypeScript + Electron（已退役，`v0.7.5`）        | Rust（現行版本）                                                      |
| --------------------- | ------------------------------------------------ | -------------------------------------------------------------------- |
| **桌面端執行環境**    | Electron — 內建 Chromium + Node.js               | 原生視窗（`winit` + GPU Skia），無需瀏覽器引擎                        |
| **桌面端安裝體積**    | 每次安裝包含完整 Chromium 執行環境               | 單一自包含二進位檔 — **55.5 MB**                                      |
| **Web 傳輸大小**      | JS + WASM 套件                                   | **8.2 MB** wasm / **2.18 MB** gzip 傳輸                              |
| **渲染**              | Web 上的 CanvasKit/Skia                          | 在**所有**目標平台共用同一套 GPU 加速 Skia 後端                       |
| **記憶體**            | JavaScript GC 暫停                               | 無 GC — Rust 所有權機制，延遲可預測                                   |
| **程式碼庫**          | Web 技術棧 + Electron          | 單一 Rust workspace：編輯器 · CLI · MCP · AI · 程式碼生成 · Figma · Git |
| **支援平台**          | Web + 桌面端，兩套獨立技術棧                     | 桌面端（macOS/Win/Linux）· 行動端（iOS/Android）· 瀏覽器 — 同一核心  |

**實測改善成果**

- **極小體積** — 整個桌面應用程式僅為一個 **55.5 MB** 原生二進位檔，而非打包瀏覽器引擎加上 Node 執行環境。Web 版本在拆分圖示目錄後為 **8.2 MB** 原始大小 / **2.18 MB** gzip（傳輸減少 −48%）。
- **可擴展至大型文件** — **10,000 個節點**的即時畫布（巢狀自動版面配置，四層深）可在**無 panic 且閒置 CPU 約 ~0%** 的情況下完成讀寫與版面快照；全部 10k 節點的完整版面快照約在 **~0.68 s** 內返回。
- **快速互動** — 縮放平移不再在每個影格重新序列化文件（單一熱路徑修正將滾輪縮放 CPU 佔用從 **~69% 降至 ~0%**）；拖曳以增量方式更新場景、文字測量結果已快取，且每影格重繪次數合並為一次。
- **一個核心，全平台** — 相同的編輯器狀態與渲染後端，透過 WASM 編譯至原生桌面端、行動端和瀏覽器——無需維護多套平行實作。
- **GPU Skia 全平台統一** — 原生端透過 `skia-safe` 在 GL 上下文渲染；瀏覽器端透過 CanvasKit 在 WebGL2 上渲染——相同的繪圖程式碼，相同的輸出結果。
- **原生無障礙支援** — 在 macOS、Windows 和 Linux 使用 AccessKit，在 Web 上使用 DOM 鏡像，而非依賴瀏覽器的 a11y 樹狀結構。
- **單一型別檢查 workspace** — MCP 宿主、CLI、AI 供應商、程式碼生成、Figma 匯入和 Git 整合全部位於單一 Rust workspace，並由 CI 中的 `cargo-deny` 進行供應鏈管控。

> **狀態：** TypeScript 版編輯器已在 `v0.7.5` 退役，僅存於 Git 歷史紀錄中；本存放庫即為 Rust workspace。Rust 產品正在積極開發中（請見下方路線圖）。

## 專案結構

```text
openpencil/
├── crates/                   Rust workspace — 產品本體
│   ├── op-editor-core/       正典 `.op`（PenDocument）編輯器狀態 + EditorCommand + 設計變數
│   ├── op-editor-ui/         平台無關的 widgets + RenderBackend facade（wasm32-clean）
│   ├── op-editor-host-core/  所有 host 共用、與傳輸方式無關的 host 狀態機
│   ├── op-host-native/       原生 host 函式庫 — winit + skia-safe GL（桌面端 + 行動端）
│   ├── op-host-web/          瀏覽器套件 — wasm32 cdylib，CanvasKit 渲染器
│   ├── op-host-desktop/      桌面二進位檔 `openpencil-desktop`；同時也是 `--serve-web` daemon
│   ├── op-host-services/     Headless serve-web / MCP daemon 函式庫
│   ├── op-host-web-server/   輕量、無 GL 依賴的 web-server 二進位檔
│   ├── op-cli/               CLI 工具 — `op` 命令
│   ├── op-mcp/               MCP 伺服器 — 工具、批次設計、分層工作流
│   ├── op-ai/                AI 提供商、聊天執行環境、串流處理
│   ├── op-ai-skills/         AI 提示詞技能引擎（分階段 prompt 載入）
│   ├── op-orchestrator/      並行 Agent 團隊編排
│   ├── op-codegen/           程式碼生成器（React、HTML、Vue、Flutter...）
│   ├── op-figma/             Figma .fig 檔案解析器與轉換器
│   ├── op-git/               Git 整合 — 複製、分支、推送/拉取、合併
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Web SDK workspace（Bun）
│   ├── op-web-sdk/           唯讀 `.op` Web 檢視器 SDK（包裝 wasm 套件）
│   ├── op-web-sdk-react/     React 19 轉接器
│   └── op-web-sdk-vue/       Vue 3 轉接器
├── vendor/                   內建子系統（Git submodules）
│   ├── jian/                 GPU-Skia UI 框架 — 元件/渲染/事件
│   ├── casement/             winit fork
│   └── agent/                跨產品 Rust Agent 執行環境（agent-rs）
└── .githooks/                Pre-commit 版本漂移檢查
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

# 版本同步（從存放庫根目錄執行）
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

## 參與貢獻

歡迎貢獻！請查閱 [CLAUDE.md](./CLAUDE.md) 了解架構細節和程式碼風格。

1. Fork 並複製存放庫
2. 啟用版本漂移檢查：`git config core.hooksPath .githooks`
3. 建立分支：`git checkout -b feat/my-feature`
4. 執行檢查：`cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
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
- [x] Cargo workspace，包含可重複使用的 Rust crate 與 Web SDK 套件
- [x] Rust 桌面端與 Web 端編輯器
- [x] CLI 工具（`op`）終端控制
- [x] 內建 Rust Agent Runtime，支援多提供商
- [x] 國際化 — 15 種語言
- [x] 基於 wasm 的 JavaScript、React 與 Vue Viewer SDK
- [x] Style Guides，支援以標籤配對和 MCP 工具
- [x] Concurrent Agent Teams，支援任務委派和畫布狀態指示
- [x] Git 整合（複製、分支、推送/拉取、資料夾模式三路合併）
- [x] 畫布匯出（SVG / PNG / JPEG / WEBP / PDF）
- [x] 協同編輯 — 經過身分驗證的 P2P、公共中繼與區域 Hub
- [x] 簡報 — 範本、投影片放映簡報者，以及 PDF/HTML/PPTX/影片匯出
- [x] 裝置登入與線上多租戶 Web 託管
- [x] Chrome 網頁擷取擴充功能，支援 HTML / 瀏覽器快照匯入
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
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> 加入我們的 Discord</strong>
</a>
— 提問、分享設計、提出功能建議。

**認可社群：[LINUX DO](https://linux.do/)**

## 第三方 Fork 與致謝

感謝上游維護者的工作，OpenPencil 得以在此基礎上建構。以下副本僅為滿足 OpenPencil 特定的整合需求而維護：

- **[casement](https://github.com/ZSeven-W/casement)** — fork 自 **[winit](https://github.com/rust-windowing/winit)**。
- **[anthropic-agent-sdk](./vendor/anthropic-agent-sdk)** — 自 **[bartolli/anthropic-agent-sdk](https://github.com/bartolli/anthropic-agent-sdk)** 引入存放庫，並作為本地 fork 維護。

各專案仍適用其各自的上游授權條款。

## 安全評估

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## 授權條款

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

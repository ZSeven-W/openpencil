<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>DeepSeek Harness 的 OpenPencil 外掛程式——在對話中預覽、檢視並編輯真實的 <code>.op</code> 文件。</strong><br />
  <sub>精確多影格預覽 &bull; 互動式畫布 &bull; 受管編輯器 &bull; 代理原生設計工具</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · 目前外掛程式發行版：<code>0.1.0-rc.7</code> · 已通過 DSH <code>0.1.1-rc.2</code> 測試</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md"><b>繁體中文</b></a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
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
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil — 多影格預覽與側邊欄編輯器" width="100%" />
</p>
<p align="center"><sub>結合互動式畫布與受管編輯器工作台的精確多影格 <code>.op</code> 預覽</sub></p>

## 為什麼選擇 DSH OpenPencil

DSH OpenPencil 將 [DeepSeek Harness](https://github.com/deepseek-ai/DSH) 與 [OpenPencil](https://github.com/ZSeven-W/openpencil) 串接起來，讓代理（Agent）直接驅動一個真實、可編輯、可互動的設計畫布，而不是回傳一張生成的圖片。

<table>
<tr>
<td width="50%">

### 🖼️ 精確多影格預覽

所安裝的 OpenPencil 無頭匯出器會產生忠於設計的預覽：第一個頂層影格以大型、可安全重播的 PNG 呈現，另附水平捲動的縮圖列，可點選選取，並為多影格文件提供上一張／下一張導覽。

</td>
<td width="50%">

### 🗺️ 互動式畫布

「開啟互動式畫布」會以惰性載入方式掛載唯讀的 OpenPencil Web SDK，支援平移、縮放與適配——不必離開對話，即可檢視任何頁面、巢狀節點或未啟用的頁面。

</td>
</tr>
<tr>
<td width="50%">

### ✏️ 受管編輯器

當 `editable: true` 時，編輯動作會在可調整大小的右側工作台開啟受管的 OpenPencil 編輯器，並提供全螢幕選項——包含選取、圖層、屬性、繪圖工具、復原／重做，以及明確的儲存語意。

</td>
<td width="50%">

### 🤖 代理原生設計工具

五個直接操作畫布的工具，加上六個 `openpencil_pipeline_*` 工具，讓代理能透過受管的 OpenPencil 執行期建立、檢查、細化、發布、修改並讀取真實畫布。

</td>
</tr>
<tr>
<td width="50%">

### 🔐 能力門控授權

圖片與文件授權是經過簽署、綁定雜湊的能力。瀏覽器後設資料絕不會暴露任意的宿主路徑，而簽署過的預覽／編輯器能力也絕不會進入標準工具結果或模型上下文。

</td>
<td width="50%">

### ⚡ 交易性安全

完整管線中的文件會留在私有、未發布的草稿內，直到通過全部原生與 DSH 品質門檻。發布絕不會覆寫既有路徑，中止或批次失敗也不會留下空的目標檔案。

</td>
</tr>
<tr>
<td width="50%">

### 🌍 延續 DSH 的外觀與操作感受

工具卡片與受管編輯器會沿用 DSH 的中／英文語系與淺色／深色主題，無須重新載入編輯階段。

</td>
<td width="50%">

### 🎯 一套完整的工作流程

「需求 → 側邊欄中的 private live canvas → 兩次向使用者展示精確 PNG 的語意化批次 → 原生／DSH 確定性品質門檻 → 原子發布」——全程在 DSH 內完成閉環。

</td>
</tr>
</table>

## 安裝到 DSH

DSH 是獨立的套件。若尚未安裝，先裝一次：

```sh
npm install -g @deepseek-ai/dsh@latest
```

接著把外掛裝進某個 profile 並啟動 Web 應用：

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@next
dsh web
```

本機開發時，先建置目前的 checkout，再將其絕對路徑連結至 Web profile，然後完整重新啟動 DSH：

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

`link:` 相依項目會讓後續重新建置的產物直接從此 checkout 生效；但替換 profile 相依項目後必須完整重新啟動 DSH，因為隨附的 Web profile 預設不會熱重載宿主套件。

不想全域安裝 DSH？用 `pnpm dlx` 執行同樣的兩步：

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@next
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> OpenPencil 外掛程式是公開的，不需要 npm token。如果 DSH 預發行版本身需要 registry 驗證，請將該憑證存放在 checkout 目錄以外的使用者層級或暫時性 npm 設定中。本儲存庫刻意不包含任何 registry 憑證。

## 設計工具

可選風格流程為 `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin`，begin 保留原始 brief 並傳入 `style_selection`。目錄瀏覽和精確選擇無需模型推薦；推薦提供者預設關閉。此交接支援 script，app-agent 與電商風格交接暫不支援；一般 app-agent 生成仍可用。

| 工具 | 功能說明 |
| --- | --- |
| `openpencil_new` | 適合簡單工作的相容快速路徑：執行一份交易性 QuickJS `batch_design` 指令碼，以「僅在不存在時建立」語意發布並回傳可編輯呈現。正式設計應優先使用下方完整管線。 |
| `openpencil_pipeline_begin` | 啟動僅屬於目前工作階段的私有草稿與唯一 root，立即在側邊欄開啟同一個 live canvas；目標 `.op` 仍維持未發布。 |
| `openpencil_pipeline_context` | 只有 begin contract 確實缺少某項 guideline、style、theme 或 UI kit 細節時，才定向載入一次；不是啟動時的 refresh loop。 |
| `openpencil_pipeline_batch` | 最多執行兩段直接 QuickJS generation script。每次成功交易只會嘗試向使用者展示精確 PNG；工具回傳後立即遵循 `next`。 |
| `openpencil_pipeline_inspect` | 僅在使用者明確要求時提供手動診斷；普通生成不會把它當作預覽或模型看圖步驟。 |
| `openpencil_pipeline_finish` | 執行原生與 DSH 的確定性門檻，渲染最終化後的精確 PNG，再透過 `createIfAbsent` 原子發布。只有結果同時為 `needs_correction`、`canContinue: true`、含完整非空 `repairTargets` 且 `omitted: 0` 時，才允許一次純 `U(...)` 修復與最後一次 finish；其他未發布結果一律終止。 |
| `openpencil_pipeline_abort` | 捨棄未發布草稿，不建立目標檔案。 |
| `openpencil_create` | 套用交易性的 `batch_design` 程式，在既有的即時畫布上產生或重構節點。 |
| `openpencil_edit` | 修改明確指定的節點，或使用者選取的單一節點。 |
| `openpencil_render` | 建立不可變、以內容定址的 `.op` 快照，並渲染作用中頁面上的所有頂層影格——可選的 `scale` 與 `editable`。 |
| `openpencil_selection` | 讀取即時編輯器畫布中實際選取的節點。 |

## 代理設計工作流程

普通生成採用固定短路徑：`openpencil_pipeline_begin` → 兩次直接 QuickJS `openpencil_pipeline_batch` → 一次 `openpencil_pipeline_finish`。Begin 會建立唯一 root，並立即在側邊欄開啟私有 live canvas；發布成功前，要求的工作區路徑並不存在。每次 begin 或 batch 成功後，都必須不加敘述、規劃、比較、檢查或無關工具呼叫地立即執行下一步。除非使用者明確要求多圖，整份設計預設只用一張 image；每個 Hero/Product/Art/Media frame 只能有一個主視覺，禁止 image 與佔位 icon 並存。

兩次成功 batch 都會嘗試向使用者展示精確 PNG 預覽。工具回傳後立即遵循 `next`。若 `next` 回報 `previewUnavailable`，指令碼已提交到 live canvas：禁止重跑 batch，也不要呼叫 `openpencil_pipeline_inspect` 或 `read_image`。`openpencil_pipeline_inspect` 只用於使用者明確要求的診斷。

Finish 會執行 OpenPencil 原生最終化、lint、對比與版面檢查，以及 DSH 確定性品質門檻，並在同一次健康呼叫中渲染精確最終 PNG、原子發布目標。只有結果同時包含 `stage: "needs_correction"`、`canContinue: true`、完整非空 `repairTargets`、`omitted: 0`，且每個 target 都有 `operation: "U"`、精確非空 `nodeId` 與非空 `patch` 時，才允許修復：一次把所有 target 套用到唯一一段純 U-only batch，然後不加敘述地只再呼叫一次 finish。其他未發布結果、error 或 `canContinue: false` 一律終止：只回報一次，禁止 retry、inspect、讀取 image/context、abort 或另起 draft。門檻失敗或呼叫 `openpencil_pipeline_abort` 時不會建立目標。published result 中的精確最終 PNG 與 live editor 已是權威結果，應直接回傳並結束。側邊欄只在 idle 時自動開啟，並保留 **編輯畫布** 供明確切換。

在同一個持續執行的 DSH 服務內，切換瀏覽器或重新載入後，經過嚴格解析的 `openpencil_new` 或 `openpencil_pipeline_finish` 持久化 publication 可以還原為精確 PNG 與明確的 **編輯畫布** 動作。歷史卡片絕不會自動開啟側邊欄，必須由使用者點擊該動作。一般歷史 `openpencil_render` 一律維持唯讀，非 loopback 連線也絕不會取得編輯器授權。

隨套件提供的 `openpencil-design` skill 仍負責指令碼與品質指引，受管執行期也不依賴桌面版二進位檔。`openpencil_new` 繼續作為相容的單批次快速路徑，但正式設計生成應優先使用完整管線。

只有在處理既有的即時畫布時，才使用 `openpencil_create` 與 `openpencil_edit`。它們的編輯內容在執行編輯器的儲存動作之前都會維持未儲存狀態。

## Web 檢視器資源

DSH 僅為用戶端外掛程式提供 `client.js`，因此 OpenPencil ESM SDK、其 WASM 與 CanvasKit 會以明確的同源資源方式進行暫存：

```sh
pnpm run sync:viewer-assets
```

同步指令預設使用上層 OpenPencil monorepo 根目錄（`../..`），並可退回同層的 `../openpencil` checkout。可用 `OPENPENCIL_ROOT` 或 `--openpencil-root` 覆寫。完整的預先建置資源目錄可透過 `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE` 選取。執行期的查詢路徑則可用 `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR` 覆寫。

檢視器資源會在使用者開啟畫布之後才進行惰性載入。如果資源缺失或無效，PNG 預覽仍然可用，且不會宣傳畫布按鈕。

## 受管編輯器

可編輯的階段使用 OpenPencil 的受管 Web 宿主——與 `op-vscode` 所使用的架構相同。外掛程式只在經過授權的使用者動作之後才會啟動宿主，將守護程式的 token 保存在記憶體中，驗證 iframe 的來源與 origin，並在編輯階段結束時關閉程序。編輯器介面會逐步選定：當宿主宣告該接縫時使用原生的 Tool 詳細資訊，否則使用外掛程式具備調整大小與全螢幕控制項的右側工作台。

啟動流程採用可安全因應慢速掛載的 listening handshake：只有在隨套件宿主回報已綁定位址後才開始就緒探測。無須安裝桌面版 OpenPencil。

發行版提供六個原生平台套件目標：`darwin-arm64`、`darwin-x64`、`linux-arm64`、`linux-x64`、`win32-arm64` 與 `win32-x64`；兩個 Linux 套件皆以 glibc 為目標。根套件將所有平台套件宣告為精確版本的 `optionalDependencies`，由 npm 依 OS 與 CPU 選用相符的套件。每個平台套件都會將 `op-host-web-server`、編輯器 Web 套件與 CanvasKit 作為一組彼此相符的原子執行期一併提供。因此，受管編輯器預設不依賴 `/Applications/OpenPencil.app`、`PATH` 中的 `openpencil-desktop`，也不依賴 OpenPencil 原始碼 checkout。

如果在畫布仍有未儲存變更時，DSH 重新載入或卸載外掛程式，宿主會保留一份不透明的本機復原草稿，最長七天。重新開啟相同來源時，會先詢問再將草稿還原到即時畫布；在使用者明確儲存之前，復原絕不會覆寫 `.op` 檔案。

正式六平台套件會在受保護的 release 建置中注入並驗證中國區／全球區協作 bootstrap 端點，通過驗證後才發佈。未注入該設定的本機自建版本，可在啟動 DSH 前以 `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap` 覆寫；該值必須使用 `https`，且路徑必須嚴格為 `/api/v1/collaboration/bootstrap`。

跨裝置畫布同步要求 PC/DSH 原生執行期與行動端 App 都更新至包含目前協作佇列修正的同一 OpenPencil 發行線。舊版行動端與新版 PC 執行期混用時，仍可能只看得到遠端游標，卻收不到畫布提交。

在此儲存庫中開發時，啟動 DSH 前必須依序建置編輯器 Web bundle、建置原生宿主，再暫存這套彼此相符的執行期。

`pnpm run build:editor-web` 會執行 OpenPencil 正式支援的 WASM bundle gate。它需要 Bash、具備 `wasm32-unknown-unknown` target 的 Cargo/Rust、`wasm-bindgen` CLI、Binaryen 的 `wasm-opt`、Node.js 與 `gzip`；CanvasKit 不需要 EMSDK。Web 建置不使用協作 bootstrap 建置變數。執行 `pnpm run build:editor-runtime` 前必須同時設定 `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN` 與 `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL`；它們只供原生 Cargo 建置使用，缺少任一變數都會 fail closed。兩項建置都成功後，再執行最後一個命令暫存執行期。

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

若要明確覆寫執行期，下列三項必須作為完整且彼此相符的一組同時提供：

- `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY`：用於 `op-host-web-server`；
- `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR`：用於已建置的編輯器 Web 套件；
- `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR`：用於 CanvasKit 資源。

只提供其中一部分屬於無效設定；外掛程式不會將自訂路徑與隨套件提供的執行期資源混合使用。

儲存採用樂觀的來源雜湊、原子替換與後繼（successor）能力。如果來源在編輯器之外發生變更，外掛程式會回報衝突，而不是覆寫它。

## 結果後設資料

模型可見的結果維持為純 JSON。僅供瀏覽器使用的 `presentationMeta.$dshOpenPencil` 承載下列附加授權：

- `image`：PNG 路徑、預覽／下載 URL，以及真實的寬／高；
- `frames`：依作用中頁面順序排列、每個精確渲染的頂層影格，包含其節點 id／名稱／索引與簽署的 PNG URL；
- `document`：來源動作路徑，加上不可變的快照 URL、位元組數與 SHA-256；
- `viewer`：在資源路由已掛接時提供帶版本修訂的 SDK／WASM／CanvasKit URL；
- `editor`：在 `editable: true` 獲得授權時提供具範疇的啟動／重新整理能力。

結果也會記錄 `renderer`、`rendererBinary`、`fidelity` 與任何警告。既有、僅含 PNG 的 schema-v1 訊息仍可正常渲染。

DSH `0.1.1-rc.2` 不會為巢狀於 PTC/Code Mode 之下的工具持久化瀏覽器呈現後設資料。外掛程式會透過同源、綁定 session 的端點復原該 UI-only 投影：瀏覽器只會傳送 session id、call id 與不可變的文件 SHA-256，而宿主則從持久的 DSH session 記錄中解析權威結果，並僅使用短暫的處理程序內標記來授權近期的即時編輯。簽署過的預覽／編輯器能力絕不會進入標準工具結果或模型上下文。一般 `openpencil_render` 的持久歷史一律唯讀。經過嚴格解析的 `openpencil_new` 或 `openpencil_pipeline_finish` 持久化 publication，只有在 loopback 連線且使用者明確點擊後才可能取得編輯器授權；自動開啟側邊欄僅保留給近期、可信的即時結果。

為控制重播範圍，巢狀後設資料的復原最多接受 128 個頂層影格；更大的 Code Mode 結果仍可透過其標準 JSON 備援取得。

## 目前的限制

- 對既有畫布進行後續編輯，需要一個已開啟的受管編輯器。在使用者執行其儲存動作之前，變更都會維持未儲存狀態。
- 輕量的 Web SDK 畫布為唯讀；完整的編輯功能使用獨立的受管編輯器介面。在 DSH `0.1.1-rc.2` 上，外掛程式使用具全螢幕選項的可調整大小右側工作台。
- 精確畫廊涵蓋作用中頁面上的頂層影格；互動式畫布仍是檢視未啟用頁面與巢狀節點的方式。
- 渲染與快照快取仍需要產品層級的保留策略。

## 專案結構

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

## 建置與驗證

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

建置需要 Node 24.11 或更新版本，以及 pnpm。DSH 的 host／client 套件是由目標 DSH profile 提供的對等相依（peer dependency）。建置工具會從本機開發相依、目前連結的 DSH checkout 或已安裝的 DSH 來源套件中解析；`DSH_SOURCE_ROOT` 可明確指定來源 checkout。當該環境為另行佈建時，lockfile 會固定獨立的公開建置工具。

若為私有的 DSH 預發行版，請將核發的 npm 憑證存放在本儲存庫之外（例如使用者層級或暫時性的 `.npmrc`），並直接執行要求的版本：

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

絕對不要提交 `.npmrc`、`NPM_TOKEN` 或複製的 registry 憑證。本儲存庫預設會忽略本機的 npm 設定。

`test:host` 會執行一次真實的精確渲染，驗證 PNG IHDR 幾何與 SHA-256，透過 HTTP 測試不可變的圖片／文件能力，並檢查檢視器資源是否可被授權。預期的尺寸會依測試樣本（fixture）而異。

## 生態系

DSH OpenPencil 是 **[OpenPencil](https://github.com/ZSeven-W/openpencil)** 的 DeepSeek Harness 外掛程式——OpenPencil 是全球第一款開源的 AI 原生向量設計工具——同時也是 **[ZSeven-W](https://github.com/ZSeven-W)** 這個純 Rust、AI 原生工具家族的一份子。

| 專案 | 說明 |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | 本外掛程式所驅動的設計工具——提示詞到畫布的生成、並行代理團隊、以設計即程式碼（design-as-code）呈現的 `.op` 檔案，以及內建的 MCP 伺服器。這裡的精確預覽、互動式畫布與受管編輯器皆由 OpenPencil 本身驅動。 |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | 純 Rust 的非同步執行期，用於交付 LLM 代理——多供應商、端到端具備工具能力、結構化權限、真正的 MCP、零 `unsafe`。為 OpenPencil 內建的代理執行期提供動力。 |
| **[jian](https://github.com/ZSeven-W/jian)** | 純 Rust、GPU-Skia 的 UI 框架——將 widgets、版面配置、事件與熱重載整合在單一技術棧中。是 OpenPencil 的 UI 框架，也是此外掛程式備援渲染器的來源。 |
| **[Zode](https://github.com/ZSeven-W/zode)** | 開源、AI 原生的終端機程式設計助手——讀取你的程式碼、執行指令，並透過 MCP 驅動 OpenPencil。 |
| **[noema](https://github.com/ZSeven-W/noema)** | 為程式設計代理打造、以本機為優先的非向量記憶系統——以可檢視的檔案形式提供持久記憶，可跨執行期運作。 |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | 教導 AI 代理如何使用 `op` 進行設計的 LLM skill 外掛程式——是此 DSH 外掛程式的夥伴專案。 |

同系列的 DSH 外掛：

- [DSH Android](https://github.com/ZSeven-W/dsh-android) — 在對話中執行 Android 模擬器或 USB 實機，全部由 adb 驅動
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) — 從 Claude Code / Codex 把任務派給 DSH agent
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) — 在對話中執行 iOS 模擬器與 USB 連接的實機
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) — DSH 的長期記憶

## 貢獻

歡迎貢獻！Fork 並 clone、建立分支、執行 `pnpm run build` 與測試套件、使用 [Conventional Commits](https://www.conventionalcommits.org/) 提交，並針對 `main` 開啟 PR。

## 社群

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> 加入我們的 Discord</strong>
</a>
— 提問、分享設計、建議功能。

**認可社群：[LINUX DO](https://linux.do/)**

## 授權

[MIT](./LICENSE) — 版權所有（c）2026 ZSeven-W

第三方元件列於 [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)。

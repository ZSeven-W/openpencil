<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Công cụ thiết kế vector mã nguồn mở thuần AI đầu tiên trên thế giới.</strong><br />
  <sub>Đội Tác nhân Đồng thời &bull; Design-as-Code &bull; Máy chủ MCP Tích hợp &bull; Trí tuệ Đa mô hình</sub>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — nhấp để xem demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Nhấp vào hình ảnh để xem video demo</sub></p>

<br />

> **Lưu ý:** Có một dự án mã nguồn mở khác cùng tên — [OpenPencil](https://github.com/open-pencil/open-pencil), tập trung vào thiết kế trực quan tương thích Figma với cộng tác thời gian thực. Dự án này tập trung vào quy trình AI-native từ thiết kế sang mã.

## Tại sao chọn OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Canvas

Mô tả bất kỳ giao diện nào bằng ngôn ngữ tự nhiên. Xem nó xuất hiện trên canvas vô hạn theo thời gian thực với hiệu ứng streaming. Chỉnh sửa thiết kế hiện có bằng cách chọn các phần tử và trò chuyện.

</td>
<td width="50%">

### 🤖 Đội Tác nhân Đồng thời

Bộ điều phối phân rã các trang phức tạp thành các tác vụ con theo không gian. Nhiều tác nhân AI làm việc trên các phần khác nhau đồng thời — hero, features, footer — tất cả streaming song song.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Trí tuệ Đa mô hình

Tự động thích ứng với khả năng của từng mô hình. Claude nhận prompt đầy đủ với thinking; GPT-4o/Gemini tắt thinking; các mô hình nhỏ hơn (MiniMax, Qwen, Llama) nhận prompt đơn giản hóa cho đầu ra đáng tin cậy.

</td>
<td width="50%">

### 🔌 Máy chủ MCP

Cài đặt một cú nhấp vào Claude Code, Codex, Gemini, OpenCode, Kiro hoặc Copilot CLI. Thiết kế từ terminal — đọc, tạo và chỉnh sửa tệp `.op` thông qua bất kỳ tác nhân tương thích MCP nào.

</td>
</tr>
<tr>
<td width="50%">

### 📦 Design-as-Code

Tệp `.op` là JSON — dễ đọc, thân thiện Git, dễ so sánh khác biệt. Biến thiết kế tạo ra thuộc tính tùy chỉnh CSS. Xuất mã sang React + Tailwind hoặc HTML + CSS.

</td>
<td width="50%">

### 🖥️ Chạy Mọi nơi

Ứng dụng web + desktop gốc trên macOS, Windows và Linux — một nhân Rust duy nhất, một tệp nhị phân độc lập duy nhất, không có browser engine. Liên kết tệp `.op` — nhấp đúp để mở.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Điều khiển công cụ thiết kế từ terminal của bạn. `op design`, `op insert` — batch design DSL, thao tác node. Pipe từ tệp hoặc stdin. Hoạt động với ứng dụng desktop hoặc web server.

</td>
<td width="50%">

### 🎯 Xuất mã Đa nền tảng

Xuất từ một tệp `.op` duy nhất sang React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Biến thiết kế trở thành thuộc tính tùy chỉnh CSS.

</td>
</tr>
</table>

## Bắt đầu nhanh

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

Hoặc chạy dưới dạng ứng dụng desktop:

```bash
cargo run -p op-host-desktop
```

> **Yêu cầu:** [Rust](https://www.rust-lang.org/) (stable) để build sản phẩm. [Bun](https://bun.sh/) >= 1.0 và [Node.js](https://nodejs.org/) >= 18 chỉ cần cho web SDK trong `packages/`.

### Docker

Các bản phát hành Rust có tag chỉ phát hành một image web host. Các image TypeScript cũ có kèm AI CLI không còn được phát hành.

| Image | Bao gồm |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:vX.Y.Z` | Rust web host, wasm bundle và tài nguyên CanvasKit |

Web UI chỉ hiển thị các built-in agent profiles; công cụ Claude/Codex/OpenCode/Copilot/Gemini CLI không được đóng gói trong Docker images.

**Chạy:**

```bash
VERSION="$(scripts/workspace-version.sh)"
docker run -d -p 3100:3100 "ghcr.io/zseven-w/openpencil-web:v${VERSION}"
```

Sau đó mở `http://localhost:3100/`.

**Build cục bộ:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## Thiết kế thuần AI

**Từ Prompt đến Giao diện**

- **Văn bản thành thiết kế** — mô tả một trang, nhận kết quả được tạo ra trên canvas theo thời gian thực với hiệu ứng streaming
- **Orchestrator** — phân rã các trang phức tạp thành các tác vụ con không gian để tạo song song
- **Chỉnh sửa thiết kế** — chọn các phần tử, sau đó mô tả thay đổi bằng ngôn ngữ tự nhiên
- **Đầu vào hình ảnh** — đính kèm ảnh chụp màn hình hoặc bản phác thảo để thiết kế dựa trên tham chiếu

**Hỗ trợ Đa tác nhân**

| Tác nhân                           | Cài đặt                                                                                                      |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| **Tích hợp sẵn (9+ nhà cung cấp)** | Chọn từ các preset nhà cung cấp với bộ chuyển đổi khu vực — Anthropic, OpenAI, Google, DeepSeek và nhiều hơn |
| **Claude Code**                    | Không cần cấu hình — sử dụng Claude Agent SDK với OAuth cục bộ                                               |
| **Codex CLI**                      | Kết nối trong Cài đặt tác nhân (`Cmd+,`)                                                                     |
| **OpenCode**                       | Kết nối trong Cài đặt tác nhân (`Cmd+,`)                                                                     |
| **GitHub Copilot**                 | `copilot login` rồi kết nối trong Cài đặt tác nhân (`Cmd+,`)                                                 |
| **Gemini CLI**                     | Kết nối trong Cài đặt tác nhân (`Cmd+,`)                                                                     |

**Hồ sơ Năng lực Mô hình** — tự động thích ứng prompt, chế độ thinking và thời gian chờ theo từng cấp mô hình. Mô hình cấp đầy đủ (Claude) nhận prompt hoàn chỉnh; cấp tiêu chuẩn (GPT-4o, Gemini, DeepSeek) tắt thinking; cấp cơ bản (MiniMax, Qwen, Llama, Mistral) nhận prompt JSON lồng nhau đơn giản hóa để đảm bảo độ tin cậy tối đa.

**i18n** — Bản địa hóa giao diện đầy đủ bằng 15 ngôn ngữ: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia.

**Máy chủ MCP**

- Máy chủ MCP tích hợp sẵn (crate `op-mcp`) — cài đặt một cú nhấp vào Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI
- Không cần Node.js — stdio transport qua tệp nhị phân desktop (`--mcp <path>`), cộng với một HTTP endpoint trực tiếp (`127.0.0.1:<port>/mcp`) từ ứng dụng đang chạy
- Tự động hóa thiết kế từ terminal: đọc, tạo và chỉnh sửa các tệp `.op` qua bất kỳ tác nhân tương thích MCP nào
- **Quy trình thiết kế phân lớp** — `design_skeleton` → `design_content` → `design_refine` cho thiết kế đa phần có độ trung thực cao hơn
- **Truy xuất prompt phân đoạn** — chỉ tải kiến thức thiết kế cần thiết (schema, layout, roles, icons, planning, v.v.)
- Hỗ trợ nhiều trang — tạo, đổi tên, sắp xếp lại và nhân bản trang qua các công cụ MCP

**Tạo mã nguồn**

- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

Cài đặt toàn cục và điều khiển công cụ thiết kế từ terminal của bạn:

```bash
brew install zseven-w/openpencil/op
```

```bash
op start                     # Khởi chạy ứng dụng desktop
op start --headless --file design.op # Khởi chạy máy chủ headless
op design @landing.txt       # Thiết kế hàng loạt từ tệp
op design @ui.js             # JavaScript sandbox với vòng lặp
op insert '{"type":"rectangle"}' # Chèn một node
op import:figma design.fig   # Nhập tệp Figma
cat design.dsl | op design - # Pipe từ stdin
```

Hỗ trợ chuỗi inline, `@filepath` và stdin (`-`). Hoạt động với ứng dụng desktop, web server hoặc server headless dựa trên tệp. Xem [tham chiếu lệnh CLI](./crates/op-cli/src/usage.txt) để biết đầy đủ các lệnh.

**LLM Skill** — cài đặt plugin [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) để dạy AI agent thiết kế bằng `op`. Chạy `op install` cho các agent được phát hiện hoặc `op install --target codex` cho một đích cụ thể.

## Tính năng

**Canvas và Vẽ**

- Canvas vô hạn với pan, zoom, hướng dẫn căn chỉnh thông minh và snapping
- Hình chữ nhật, Hình ellipse, Đường thẳng, Đa giác, Bút (Bezier), Frame, Văn bản
- Phép toán Boolean — hợp nhất, trừ, giao nhau với thanh công cụ ngữ cảnh
- Trình chọn icon (Iconify) và nhập hình ảnh (PNG/JPEG/SVG/WebP/GIF)
- Auto-layout — dọc/ngang với gap, padding, justify, align
- Tài liệu nhiều trang với điều hướng bằng tab

**Hệ thống Thiết kế**

- Biến thiết kế — token màu sắc, số, chuỗi với tham chiếu `$variable`
- Hỗ trợ đa chủ đề — nhiều trục, mỗi trục có các biến thể (Sáng/Tối, Thu gọn/Thoải mái)
- Hệ thống component — các component có thể tái sử dụng với instances và overrides
- Đồng bộ CSS — thuộc tính tùy chỉnh tự động tạo, `var(--name)` trong đầu ra mã
- UIKits có thể tái sử dụng — nhập/xuất các kit component từ tệp `.pen`

**AI và Tác nhân**

- Prompt-to-canvas với tạo streaming và phân rã không gian theo orchestrator
- Đội tác nhân song song — nhiều nhà thiết kế làm việc trên các phần khác nhau song song, với chỉ báo canvas theo từng thành viên
- Quy trình phân lớp — `design_skeleton` → `design_content` → `design_refine` với prompt tập trung cho từng giai đoạn
- Style Guides — hơn 50 style tích hợp (glassmorphism, brutalist, retro, v.v.) với khớp mờ dựa trên tag, tích hợp vào lập kế hoạch và tạo
- Hồ sơ năng lực đa mô hình — tự động điều chỉnh chế độ tư duy, nỗ lực và hình thức prompt theo cấp mô hình
- Runtime tác nhân tích hợp (Rust) + nhà cung cấp Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini
- Chuyển tiếp định dạng Anthropic cho các nhà cung cấp LLM Trung Quốc — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Tích hợp Git**

- Wizard clone với xác thực SSH / HTTPS và quản lý khóa SSH
- Bộ chọn branch — tạo, chuyển, xóa, merge, tất cả từ bảng Git
- Pull / push theo tầng với thử lại xác thực và xử lý non-fast-forward
- Merge ba chiều chế độ thư mục với theo dõi trạng thái `MERGE_HEAD` trên đĩa
- Bảng xung đột với thẻ ba chiều theo từng node / field, trình chỉnh sửa JSON nội tuyến, hành động hàng loạt và khối diff nội tuyến
- Giao diện cài đặt remote và khóa SSH; i18n 15 ngôn ngữ trên toàn bộ bề mặt Git

**Xuất**

- Xuất canvas — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- Xuất mã — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- Pipeline codegen MCP tăng dần — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Nhập từ Figma**

- Nhập tệp `.fig` với layout, fills, strokes, effects, văn bản, hình ảnh và vector được bảo toàn

**Ứng dụng Desktop**

- macOS, Windows và Linux gốc — một tệp nhị phân độc lập duy nhất (winit + GPU Skia, không có Electron)
- Liên kết tệp `.op` — nhấp đúp để mở, khóa phiên bản đơn
- Kiểm tra cập nhật nền so với GitHub Releases
- Menu ứng dụng gốc với Lưu thành, Mở gần đây và hộp thoại thay đổi chưa lưu khi đóng
- Lưu danh sách tệp gần đây

## Công nghệ

|                    |                                                                                             |
| ------------------ | ------------------------------------------------------------------------------------------- |
| **Lõi**            | Rust workspace (`crates/`) — trạng thái editor, widget, host, MCP, AI, codegen              |
| **Dựng hình**      | GPU Skia ở khắp nơi — `skia-safe` (GL) trên nền tảng gốc, CanvasKit (WASM/WebGL2) trên trình duyệt |
| **Framework UI**   | jian — framework UI GPU-Skia thuần Rust được vendor hóa: widget, layout, sự kiện, hot reload (`vendor/jian`) |
| **Cửa sổ**         | winit (bản fork `casement` được vendor hóa)                                                 |
| **Desktop**        | Tệp nhị phân gốc `openpencil-desktop` — không có browser engine                             |
| **Web SDK**        | `op-web-sdk` + các adapter React 19 / Vue 3 — trình xem `.op` chỉ đọc (TypeScript)           |
| **CLI**            | `op` — điều khiển từ terminal, batch design DSL                                             |
| **AI**             | Runtime tác nhân Rust tích hợp sẵn · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**           | clippy · rustfmt (Rust) · oxlint · oxfmt (web SDK)                                           |
| **Định dạng tệp**  | `.op` — dựa trên JSON, dễ đọc, thân thiện với Git                                            |

## Hệ sinh thái

OpenPencil là một phần của bộ công cụ thuần Rust, thuần AI đến từ **[ZSeven-W](https://github.com/ZSeven-W)**. Chúng phối hợp với nhau: `jian` dựng hình OpenPencil, `agent-rs` chạy các tác nhân của nó, `noema` ghi nhớ, và `zode` thiết kế từ terminal.

| Dự án | Là gì |
| ----- | ----- |
| **[Zode](https://github.com/ZSeven-W/zode)** | Trợ lý lập trình mã nguồn mở, thuần AI cho terminal của bạn — một Rust TUI nhanh (`ratatui`) đọc mã của bạn, chạy lệnh, tìm kiếm tệp và quản lý git. Điều khiển OpenPencil qua MCP. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | Một runtime bất đồng bộ thuần Rust để phát hành các tác nhân LLM — đa nhà cung cấp, hỗ trợ công cụ đầu-cuối, quyền có cấu trúc, MCP thực thụ, không có `unsafe`. Cung cấp năng lượng cho runtime tác nhân tích hợp sẵn của OpenPencil (`vendor/agent`) và Zode. |
| **[jian](https://github.com/ZSeven-W/jian)** | Framework UI GPU-Skia thuần Rust — widget, layout, sự kiện và hot reload trong một stack duy nhất. Biến một tài liệu `.op` khai báo thành một ứng dụng gốc, có thể điều khiển bằng AI mà không cần JS runtime, không DOM, không Electron. Framework UI của OpenPencil (`vendor/jian`). |
| **[noema](https://github.com/ZSeven-W/noema)** | Hệ thống bộ nhớ ưu tiên cục bộ, phi vector cho các tác nhân lập trình. Bộ nhớ bền vững dưới dạng tệp có thể kiểm tra, hàng đợi đánh giá cho các mục mới, và truy xuất từ vựng (không cần embedding) — hoạt động trên Zode, Codex, Claude Code và các runtime MCP. |

## Tại sao chọn Rust

OpenPencil đã được viết lại từ đầu bằng **Rust** ([#129](https://github.com/ZSeven-W/openpencil/issues/129)). Việc viết lại đã hoàn tất — editor TypeScript + Electron đã bị khai tử tại `v0.7.5`, và Rust workspace trong repo này chính là sản phẩm: một nhân gốc duy nhất nhỏ hơn và nhanh hơn đáng kể, chạy trên nhiều nền tảng hơn từ một codebase duy nhất.

|                              | TypeScript + Electron (đã khai tử, `v0.7.5`)   | Rust (hiện tại)                                                      |
| ---------------------------- | ---------------------------------------------- | -------------------------------------------------------------------- |
| **Runtime desktop**          | Electron — đi kèm Chromium + Node.js           | Cửa sổ gốc (`winit` + GPU Skia), không có browser engine             |
| **Dung lượng desktop**       | Toàn bộ runtime Chromium mỗi lần cài đặt       | Tệp nhị phân độc lập duy nhất — **55.5 MB**                          |
| **Tải trọng web**            | Bundle JS + WASM                               | **8.2 MB** wasm / **2.18 MB** gzip qua mạng                         |
| **Dựng hình**                | CanvasKit/Skia trên web                        | Một backend Skia tăng tốc GPU trên **mọi** nền tảng                  |
| **Bộ nhớ**                   | Dừng do JavaScript GC                         | Không có GC — sở hữu Rust, độ trễ có thể dự đoán                    |
| **Codebase**                 | Web stack + Electron          | Một Rust workspace: editor · CLI · MCP · AI · codegen · Figma · Git  |
| **Nền tảng**                 | Web + desktop, hai stack riêng biệt            | Desktop (macOS/Win/Linux) · mobile (iOS/Android) · browser — một nhân |

**Cải thiện đã đo lường**

- **Dung lượng nhỏ** — toàn bộ ứng dụng desktop là một tệp nhị phân gốc **55.5 MB** thay vì một browser engine đóng gói cùng với Node runtime. Bản dựng web là **8.2 MB** thô / **2.18 MB** gzip sau khi tách catalog icon (−48% qua mạng).
- **Mở rộng tốt với tài liệu lớn** — canvas trực tiếp với **10,000 node** (auto-layout lồng nhau, bốn cấp độ) ghi, đọc và snapshot layout **không có panic và ~0% CPU khi rỗi**; snapshot layout đầy đủ của toàn bộ 10k node trả về trong **~0.68 s**.
- **Tương tác nhanh** — pan/zoom không còn tuần tự hóa lại tài liệu mỗi frame (một bản sửa hot-path duy nhất giảm CPU zoom bằng bánh xe từ **~69% xuống ~0%**); kéo cập nhật cảnh gia tăng, đo lường văn bản được cache, và các lần vẽ lại gộp thành một lần mỗi frame.
- **Một nhân, mọi màn hình** — cùng trạng thái editor và cùng render backend biên dịch sang desktop gốc, mobile và browser qua WASM — không có các cài đặt lại song song cần đồng bộ.
- **GPU Skia ở khắp nơi** — gốc dựng hình qua `skia-safe` trên GL context; browser dựng hình qua CanvasKit trên WebGL2 — cùng mã vẽ, cùng đầu ra.
- **Trợ năng gốc** — AccessKit trên macOS, Windows và Linux, cộng với DOM mirror trên web, thay vì dựa vào cây a11y của browser.
- **Một workspace có kiểm tra kiểu** — MCP host, CLI, nhà cung cấp AI, tạo mã, nhập Figma và tích hợp Git đều nằm trong một Rust workspace duy nhất, với `cargo-deny` kiểm soát chuỗi cung ứng trong CI.

> **Trạng thái:** editor TypeScript đã bị khai tử tại `v0.7.5` và chỉ còn tồn tại trong lịch sử git; repo này chính là Rust workspace. Sản phẩm Rust đang được phát triển tích cực (xem Lộ trình bên dưới).

## Cấu trúc dự án

```text
openpencil/
├── crates/                   Rust workspace — sản phẩm chính
│   ├── op-editor-core/       Trạng thái editor `.op` (PenDocument) chuẩn + EditorCommand + biến thiết kế
│   ├── op-editor-ui/         Widget không phụ thuộc nền tảng + RenderBackend facade (wasm32-clean)
│   ├── op-editor-host-core/  Máy trạng thái host không phụ thuộc transport, dùng chung cho mọi host
│   ├── op-host-native/       Thư viện host gốc — winit + skia-safe GL (desktop + mobile)
│   ├── op-host-web/          Bundle trình duyệt — wasm32 cdylib, renderer CanvasKit
│   ├── op-host-desktop/      Binary desktop `openpencil-desktop`; đồng thời là daemon `--serve-web`
│   ├── op-host-services/     Thư viện daemon serve-web / MCP headless
│   ├── op-host-web-server/   Binary web-server không cần GL
│   ├── op-cli/               Công cụ CLI — lệnh `op`
│   ├── op-mcp/               Máy chủ MCP — công cụ, batch design, quy trình phân lớp
│   ├── op-ai/                Nhà cung cấp AI, runtime chat, streaming
│   ├── op-ai-skills/         Engine kỹ năng AI prompt (tải prompt theo giai đoạn)
│   ├── op-orchestrator/      Điều phối đội tác nhân đồng thời
│   ├── op-codegen/           Bộ tạo mã (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Trình phân tích và chuyển đổi tệp Figma .fig
│   ├── op-git/               Tích hợp Git — clone, branch, push/pull, merge
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Web SDK workspace (Bun)
│   ├── op-web-sdk/           SDK trình xem web `.op` chỉ đọc (bọc bundle wasm)
│   ├── op-web-sdk-react/     Adapter React 19
│   └── op-web-sdk-vue/       Adapter Vue 3
├── vendor/                   Các subsystem được vendor hóa (git submodules)
│   ├── jian/                 Framework UI GPU-Skia — widget/render/event
│   ├── casement/             Bản fork của winit
│   └── agent/                Runtime tác nhân Rust đa sản phẩm (agent-rs)
└── .githooks/                Kiểm tra độ lệch phiên bản trước commit
```

## Phím tắt

| Phím        | Hành động         |     | Phím          | Hành động                 |
| ----------- | ----------------- | --- | ------------- | ------------------------- |
| `V`         | Chọn              |     | `Cmd+S`       | Lưu                       |
| `R`         | Hình chữ nhật     |     | `Cmd+Z`       | Hoàn tác                  |
| `O`         | Hình ellipse      |     | `Cmd+Shift+Z` | Làm lại                   |
| `L`         | Đường thẳng       |     | `Cmd+C/X/V/D` | Sao chép/Cắt/Dán/Nhân bản |
| `T`         | Văn bản           |     | `Cmd+G`       | Nhóm                      |
| `F`         | Frame             |     | `Cmd+Shift+G` | Bỏ nhóm                   |
| `P`         | Công cụ bút       |     | `Cmd+Shift+P` | Xuất (PNG/JPG/WEBP/PDF)   |
| `H`         | Tay (pan)         |     | `Cmd+Shift+C` | Bảng mã                   |
| `Del`       | Xóa               |     | `Cmd+Shift+V` | Bảng biến                 |
| `[ / ]`     | Sắp xếp lại       |     | `Cmd+J`       | AI chat                   |
| Mũi tên     | Dịch chuyển 1px   |     | `Cmd+,`       | Cài đặt tác nhân          |
| `Cmd+Alt+U` | Hợp nhất Boolean  |     | `Cmd+Alt+S`   | Trừ Boolean               |
| `Cmd+Alt+I` | Giao nhau Boolean |     | `Cmd+Shift+S` | Lưu thành                 |

## Scripts

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

# Đồng bộ phiên bản (chạy từ thư mục gốc của kho mã)
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

## Đóng góp

Chào mừng đóng góp! Xem [CLAUDE.md](./CLAUDE.md) để biết chi tiết về kiến trúc và phong cách mã.

1. Fork và clone
2. Bật kiểm tra độ lệch phiên bản: `git config core.hooksPath .githooks`
3. Tạo branch: `git checkout -b feat/my-feature`
4. Chạy kiểm tra: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
5. Commit theo [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
6. Mở PR vào nhánh `main`

## Lộ trình

- [x] Biến thiết kế & token với đồng bộ CSS
- [x] Hệ thống component (instances & overrides)
- [x] Tạo thiết kế AI với orchestrator
- [x] Tích hợp máy chủ MCP với quy trình thiết kế phân lớp
- [x] Hỗ trợ nhiều trang
- [x] Nhập Figma `.fig`
- [x] Phép toán Boolean (hợp nhất, trừ, giao)
- [x] Hồ sơ năng lực đa mô hình
- [x] Workspace Cargo với các crate Rust và gói Web SDK có thể tái sử dụng
- [x] Trình chỉnh sửa Rust cho desktop và Web
- [x] Công cụ CLI (`op`) điều khiển từ terminal
- [x] Rust Agent Runtime tích hợp sẵn với hỗ trợ đa nhà cung cấp
- [x] i18n — 15 ngôn ngữ
- [x] Viewer SDK dựa trên wasm cho JavaScript, React và Vue
- [x] Style Guides với đối sánh theo tag và công cụ MCP
- [x] Concurrent Agent Teams với khả năng ủy quyền và chỉ báo trên canvas
- [x] Tích hợp Git (clone, branch, push/pull, merge ba chiều chế độ thư mục)
- [x] Xuất canvas (SVG / PNG / JPEG / WEBP / PDF)
- [ ] Chỉnh sửa cộng tác
- [ ] Hệ thống plugin

## Người đóng góp

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Nhà tài trợ

OpenPencil miễn phí và mã nguồn mở. Việc phát triển được tài trợ bởi những người thấy nó hữu ích — cảm ơn bạn đã giữ cho canvas luôn mở.

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

Cảm ơn **[MrQyun](https://github.com/mrqyun)** — muốn tên mình xuất hiện ở đây? **[Trở thành nhà tài trợ →](https://github.com/sponsors/ZSeven-W)**

## Cộng đồng

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Tham gia Discord của chúng tôi</strong>
</a>
— Đặt câu hỏi, chia sẻ thiết kế, đề xuất tính năng.

**Cộng đồng được công nhận: [LINUX DO](https://linux.do/)**

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## Đánh giá

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## Giấy phép

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

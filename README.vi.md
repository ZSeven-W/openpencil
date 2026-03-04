<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Công cụ thiết kế mã nguồn mở thuần AI. Design-as-Code.</strong><br />
  Từ prompt đến giao diện trên canvas. Điều phối đa tác nhân. Máy chủ MCP tích hợp sẵn. Tạo mã nguồn.
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <b>Tiếng Việt</b> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#quick-start">Bắt đầu nhanh</a> ·
  <a href="#ai-native-design">AI</a> ·
  <a href="#features">Tính năng</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#contributing">Đóng góp</a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Nhấp vào hình ảnh để xem video demo</sub></p>

<br />

## Bắt đầu nhanh

```bash
# Cài đặt các phụ thuộc
bun install

# Khởi động máy chủ phát triển tại http://localhost:3000
bun --bun run dev
```

Hoặc chạy dưới dạng ứng dụng desktop:

```bash
bun run electron:dev
```

> **Yêu cầu:** [Bun](https://bun.sh/) >= 1.0 và [Node.js](https://nodejs.org/) >= 18

## Thiết kế thuần AI

OpenPencil được xây dựng xung quanh AI từ nền tảng — không phải như một plugin mà là một quy trình làm việc cốt lõi.

**Từ Prompt đến Giao diện**
- **Văn bản thành thiết kế** — mô tả một trang, nhận kết quả được tạo ra trên canvas theo thời gian thực với hiệu ứng streaming
- **Orchestrator** — phân rã các trang phức tạp thành các tác vụ con không gian để tạo song song
- **Chỉnh sửa thiết kế** — chọn các phần tử, sau đó mô tả thay đổi bằng ngôn ngữ tự nhiên
- **Đầu vào hình ảnh** — đính kèm ảnh chụp màn hình hoặc bản phác thảo để thiết kế dựa trên tham chiếu

**Hỗ trợ Đa tác nhân**

| Tác nhân | Cài đặt |
| --- | --- |
| **Claude Code** | Không cần cấu hình — sử dụng Claude Agent SDK với OAuth cục bộ |
| **Codex CLI** | Kết nối trong Cài đặt tác nhân (`Cmd+,`) |
| **OpenCode** | Kết nối trong Cài đặt tác nhân (`Cmd+,`) |

**Máy chủ MCP**
- Máy chủ MCP tích hợp sẵn — cài đặt một cú nhấp vào Claude Code / Codex / Gemini / OpenCode / Kiro CLI
- Tự động hóa thiết kế từ terminal: đọc, tạo và chỉnh sửa các tệp `.op` qua bất kỳ tác nhân tương thích MCP nào

**Tạo mã nguồn**
- React + Tailwind CSS
- HTML + CSS
- CSS Variables từ các token thiết kế

## Tính năng

**Canvas và Vẽ**
- Canvas vô hạn với pan, zoom, hướng dẫn căn chỉnh thông minh và snapping
- Hình chữ nhật, Hình ellipse, Đường thẳng, Đa giác, Bút (Bezier), Frame, Văn bản
- Trình chọn icon (Iconify) và nhập hình ảnh (PNG/JPEG/SVG/WebP/GIF)
- Auto-layout — dọc/ngang với gap, padding, justify, align
- Tài liệu nhiều trang với điều hướng bằng tab

**Hệ thống Thiết kế**
- Biến thiết kế — token màu sắc, số, chuỗi với tham chiếu `$variable`
- Hỗ trợ đa chủ đề — nhiều trục, mỗi trục có các biến thể (Sáng/Tối, Thu gọn/Thoải mái)
- Hệ thống component — các component có thể tái sử dụng với instances và overrides
- Đồng bộ CSS — thuộc tính tùy chỉnh tự động tạo, `var(--name)` trong đầu ra mã

**Nhập từ Figma**
- Nhập tệp `.fig` với layout, fills, strokes, effects, văn bản, hình ảnh và vector được bảo toàn

**Ứng dụng Desktop**
- macOS, Windows và Linux gốc qua Electron
- Tự động cập nhật từ GitHub Releases
- Menu ứng dụng gốc và hộp thoại tệp

## Công nghệ

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Canvas** | Fabric.js v7 |
| **Trạng thái** | Zustand v5 |
| **Máy chủ** | Nitro |
| **Desktop** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **Runtime** | Bun · Vite 7 |
| **Định dạng tệp** | `.op` — dựa trên JSON, dễ đọc, thân thiện với Git |

## Cấu trúc dự án

```text
src/
  canvas/          Fabric.js engine — vẽ, đồng bộ, layout, hướng dẫn, công cụ bút
  components/      React UI — editor, panels, hộp thoại dùng chung, icons
  services/ai/     AI chat, orchestrator, tạo thiết kế, streaming
  services/figma/  Pipeline nhập binary Figma .fig
  services/codegen Bộ tạo mã React+Tailwind và HTML+CSS
  stores/          Zustand — canvas, document, pages, history, AI, settings
  variables/       Giải quyết token thiết kế và quản lý tham chiếu
  mcp/             Công cụ máy chủ MCP để tích hợp CLI bên ngoài
  uikit/           Hệ thống kit component có thể tái sử dụng
server/
  api/ai/          Nitro API — streaming chat, generation, validation
  utils/           Claude CLI, OpenCode, Codex client wrappers
electron/
  main.ts          Cửa sổ, Nitro fork, menu gốc, auto-updater
  preload.ts       IPC bridge
```

## Phím tắt

| Phím | Hành động | | Phím | Hành động |
| --- | --- | --- | --- | --- |
| `V` | Chọn | | `Cmd+S` | Lưu |
| `R` | Hình chữ nhật | | `Cmd+Z` | Hoàn tác |
| `O` | Hình ellipse | | `Cmd+Shift+Z` | Làm lại |
| `L` | Đường thẳng | | `Cmd+C/X/V/D` | Sao chép/Cắt/Dán/Nhân bản |
| `T` | Văn bản | | `Cmd+G` | Nhóm |
| `F` | Frame | | `Cmd+Shift+G` | Bỏ nhóm |
| `P` | Công cụ bút | | `Cmd+Shift+E` | Xuất |
| `H` | Tay (pan) | | `Cmd+Shift+C` | Bảng mã |
| `Del` | Xóa | | `Cmd+Shift+V` | Bảng biến |
| `[ / ]` | Sắp xếp lại | | `Cmd+J` | AI chat |
| Mũi tên | Dịch chuyển 1px | | `Cmd+,` | Cài đặt tác nhân |

## Scripts

```bash
bun --bun run dev          # Máy chủ phát triển (cổng 3000)
bun --bun run build        # Build production
bun --bun run test         # Chạy kiểm thử (Vitest)
npx tsc --noEmit           # Kiểm tra kiểu
bun run electron:dev       # Electron dev
bun run electron:build     # Đóng gói Electron
```

## Đóng góp

Chào mừng đóng góp! Xem [CLAUDE.md](./CLAUDE.md) để biết chi tiết về kiến trúc và phong cách mã.

1. Fork và clone
2. Tạo branch: `git checkout -b feat/my-feature`
3. Chạy kiểm tra: `npx tsc --noEmit && bun --bun run test`
4. Commit theo [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
5. Mở PR vào nhánh `main`

## Lộ trình

- [x] Biến thiết kế & token với đồng bộ CSS
- [x] Hệ thống component (instances & overrides)
- [x] Tạo thiết kế AI với orchestrator
- [x] Tích hợp máy chủ MCP
- [x] Hỗ trợ nhiều trang
- [x] Nhập Figma `.fig`
- [ ] Phép toán Boolean (hợp nhất, trừ, giao)
- [ ] Chỉnh sửa cộng tác
- [ ] Hệ thống plugin

## Người đóng góp

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Cộng đồng

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Tham gia Discord của chúng tôi</strong>
</a>
— Đặt câu hỏi, chia sẻ thiết kế, đề xuất tính năng.

## Giấy phép

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

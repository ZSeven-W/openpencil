<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Alat desain vektor open-source berbasis AI pertama di dunia.</strong><br />
  <sub>Tim Agen Konkuren &bull; Design-as-Code &bull; Server MCP Bawaan &bull; Kecerdasan Multi-model</sub>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — klik untuk menonton demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Klik gambar untuk menonton video demo</sub></p>

<br />

## Mengapa OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Kanvas

Deskripsikan UI apa pun dalam bahasa alami. Saksikan hasilnya muncul di kanvas tak terbatas secara real-time dengan animasi streaming. Modifikasi desain yang ada dengan memilih elemen dan berdialog.

</td>
<td width="50%">

### 🤖 Tim Agen Konkuren

Orkestrator menguraikan halaman kompleks menjadi sub-tugas spasial. Beberapa agen AI bekerja pada bagian yang berbeda secara bersamaan — hero, fitur, footer — semuanya streaming secara paralel.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Kecerdasan Multi-Model

Secara otomatis menyesuaikan dengan kemampuan setiap model. Claude mendapat prompt lengkap dengan thinking; GPT-4o/Gemini menonaktifkan thinking; model yang lebih kecil (MiniMax, Qwen, Llama) mendapat prompt yang disederhanakan untuk keluaran yang andal.

</td>
<td width="50%">

### 🔌 Server MCP

Instal satu klik ke Claude Code, Codex, Gemini, OpenCode, Kiro, atau Copilot CLI. Desain dari terminal Anda — baca, buat, dan modifikasi file `.op` melalui agen yang kompatibel dengan MCP.

</td>
</tr>
<tr>
<td width="50%">

### 📦 Design-as-Code

File `.op` adalah JSON — mudah dibaca manusia, ramah Git, mudah dibandingkan. Variabel desain menghasilkan CSS custom properties. Ekspor kode ke React + Tailwind atau HTML + CSS.

</td>
<td width="50%">

### 🖥️ Berjalan di Mana Saja

Aplikasi web + desktop native di macOS, Windows, dan Linux melalui Electron. Pembaruan otomatis dari GitHub Releases. Asosiasi file `.op` — klik dua kali untuk membuka.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Kontrol alat desain dari terminal Anda. `op design`, `op insert`, `op export` — batch design DSL, manipulasi node, ekspor kode. Pipe dari file atau stdin. Bekerja dengan aplikasi desktop atau web server.

</td>
<td width="50%">

### 🎯 Ekspor Kode Multi-Platform

Ekspor dari satu file `.op` ke React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Variabel desain menjadi CSS custom properties.

</td>
</tr>
</table>

## Mulai Cepat

```bash
# Instal dependensi
bun install

# Jalankan server pengembangan di http://localhost:3000
bun --bun run dev
```

Atau jalankan sebagai aplikasi desktop:

```bash
bun run electron:dev
```

> **Prasyarat:** [Bun](https://bun.sh/) >= 1.0 dan [Node.js](https://nodejs.org/) >= 18

### Docker

Tersedia beberapa varian image — pilih yang sesuai kebutuhan Anda:

| Image | Ukuran | Termasuk |
| --- | --- | --- |
| `openpencil:latest` | ~226 MB | Hanya aplikasi web |
| `openpencil-claude:latest` | — | + Claude Code CLI |
| `openpencil-codex:latest` | — | + Codex CLI |
| `openpencil-opencode:latest` | — | + OpenCode CLI |
| `openpencil-copilot:latest` | — | + GitHub Copilot CLI |
| `openpencil-gemini:latest` | — | + Gemini CLI |
| `openpencil-full:latest` | ~1 GB | Semua alat CLI |

**Jalankan (hanya web):**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**Jalankan dengan AI CLI (misal Claude Code):**

Chat AI bergantung pada login OAuth Claude CLI. Gunakan volume Docker untuk menyimpan sesi login:

```bash
# Langkah 1 — Login (satu kali)
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# Langkah 2 — Mulai
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**Build secara lokal:**

```bash
# Dasar (hanya web)
docker build --target base -t openpencil .

# Dengan CLI tertentu
docker build --target with-claude -t openpencil-claude .

# Lengkap (semua CLI)
docker build --target full -t openpencil-full .
```

## Desain Berbasis AI

**Dari Prompt ke UI**
- **Teks ke desain** — deskripsikan halaman, dan hasilkan di kanvas secara real-time dengan animasi streaming
- **Orkestrator** — menguraikan halaman kompleks menjadi sub-tugas spasial untuk pembuatan secara paralel
- **Modifikasi desain** — pilih elemen, lalu deskripsikan perubahan dalam bahasa alami
- **Masukan visual** — lampirkan tangkapan layar atau mockup sebagai referensi desain

**Dukungan Multi-Agen**

| Agen | Pengaturan |
| --- | --- |
| **Claude Code** | Tanpa konfigurasi — menggunakan Claude Agent SDK dengan OAuth lokal |
| **Codex CLI** | Hubungkan di Pengaturan Agen (`Cmd+,`) |
| **OpenCode** | Hubungkan di Pengaturan Agen (`Cmd+,`) |
| **GitHub Copilot** | `copilot login` lalu hubungkan di Pengaturan Agen (`Cmd+,`) |
| **Gemini CLI** | Hubungkan di Pengaturan Agen (`Cmd+,`) |

**Profil Kemampuan Model** — secara otomatis menyesuaikan prompt, mode thinking, dan timeout per tingkatan model. Model tingkat penuh (Claude) mendapat prompt lengkap; tingkat standar (GPT-4o, Gemini, DeepSeek) menonaktifkan thinking; tingkat dasar (MiniMax, Qwen, Llama, Mistral) mendapat prompt JSON bertingkat yang disederhanakan untuk keandalan maksimum.

**Server MCP**
- Server MCP bawaan — instal satu klik ke Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI
- Deteksi otomatis Node.js — jika tidak terinstal, otomatis beralih ke transport HTTP dan memulai server MCP HTTP
- Otomasi desain dari terminal: baca, buat, dan modifikasi file `.op` melalui agen yang kompatibel dengan MCP
- **Alur kerja desain berlapis** — `design_skeleton` → `design_content` → `design_refine` untuk desain multi-bagian dengan fidelitas lebih tinggi
- **Pengambilan prompt tersegmentasi** — muat hanya pengetahuan desain yang Anda butuhkan (schema, layout, roles, icons, planning, dll.)
- Dukungan multi-halaman — buat, ganti nama, urutkan ulang, dan duplikasi halaman melalui alat MCP

**Pembuatan Kode**
- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

Instal secara global dan kontrol alat desain dari terminal Anda:

```bash
npm install -g @zseven-w/openpencil
```

```bash
op start                     # Jalankan aplikasi desktop
op design @landing.txt       # Desain batch dari file
op insert '{"type":"RECT"}'  # Sisipkan sebuah node
op export react --out .      # Ekspor ke React + Tailwind
op import:figma design.fig   # Impor file Figma
cat design.dsl | op design - # Pipe dari stdin
```

Mendukung tiga metode input: string inline, `@filepath` (baca dari file), atau `-` (baca dari stdin). Bekerja dengan aplikasi desktop atau web dev server. Lihat [CLI README](./apps/cli/README.md) untuk referensi perintah lengkap.

## Fitur

**Kanvas & Menggambar**
- Kanvas tak terbatas dengan pan, zoom, panduan perataan cerdas, dan snapping
- Persegi panjang, Elips, Garis, Poligon, Pen (Bezier), Frame, Teks
- Operasi Boolean — gabungan, kurangi, irisan dengan toolbar kontekstual
- Pemilih ikon (Iconify) dan impor gambar (PNG/JPEG/SVG/WebP/GIF)
- Auto-layout — vertikal/horizontal dengan gap, padding, justify, align
- Dokumen multi-halaman dengan navigasi tab

**Sistem Desain**
- Variabel desain — token warna, angka, string dengan referensi `$variable`
- Dukungan multi-tema — beberapa sumbu, masing-masing dengan varian (Terang/Gelap, Ringkas/Nyaman)
- Sistem komponen — komponen yang dapat digunakan ulang dengan instans dan penggantian
- Sinkronisasi CSS — properti kustom yang dibuat otomatis, `var(--name)` dalam keluaran kode

**Impor Figma**
- Impor file `.fig` dengan tata letak, fill, stroke, efek, teks, gambar, dan vektor tetap terjaga

**Aplikasi Desktop**
- macOS, Windows, dan Linux native melalui Electron
- Asosiasi file `.op` — klik dua kali untuk membuka, kunci instans tunggal
- Pembaruan otomatis dari GitHub Releases
- Menu aplikasi native dan dialog file

## Tumpukan Teknologi

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Kanvas** | CanvasKit/Skia (WASM, akselerasi GPU) |
| **State** | Zustand v5 |
| **Server** | Nitro |
| **Desktop** | Electron 35 |
| **CLI** | `op` — kontrol terminal, batch design DSL, ekspor kode |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Runtime** | Bun · Vite 7 |
| **Format file** | `.op` — berbasis JSON, mudah dibaca manusia, ramah Git |

## Struktur Proyek

```text
openpencil/
├── apps/
│   ├── web/                 Aplikasi web TanStack Start
│   │   ├── src/
│   │   │   ├── canvas/      Mesin CanvasKit/Skia — menggambar, sinkronisasi, tata letak
│   │   │   ├── components/  UI React — editor, panel, dialog bersama, ikon
│   │   │   ├── services/ai/ Chat AI, orkestrator, pembuatan desain, streaming
│   │   │   ├── stores/      Zustand — kanvas, dokumen, halaman, riwayat, AI
│   │   │   ├── mcp/         Alat server MCP untuk integrasi CLI eksternal
│   │   │   ├── hooks/       Pintasan keyboard, seret file, tempel Figma
│   │   │   └── uikit/       Sistem kit komponen yang dapat digunakan ulang
│   │   └── server/
│   │       ├── api/ai/      Nitro API — chat streaming, pembuatan, validasi
│   │       └── utils/       Pembungkus Claude CLI, OpenCode, Codex, Copilot
│   ├── desktop/             Aplikasi desktop Electron
│   │   ├── main.ts          Jendela, fork Nitro, menu native, pembaruan otomatis
│   │   ├── ipc-handlers.ts  Dialog file native, sinkronisasi tema, preferensi IPC
│   │   └── preload.ts       Jembatan IPC
│   └── cli/                 Alat CLI — perintah `op`
│       ├── src/commands/    Perintah design, document, export, import, node, page, variable
│       ├── connection.ts    Koneksi WebSocket ke aplikasi yang berjalan
│       └── launcher.ts      Deteksi otomatis dan jalankan aplikasi desktop atau web server
├── packages/
│   ├── pen-types/           Definisi tipe untuk model PenDocument
│   ├── pen-core/            Operasi pohon dokumen, mesin tata letak, variabel
│   ├── pen-codegen/         Generator kode (React, HTML, Vue, Flutter, ...)
│   ├── pen-figma/           Parser dan konverter file Figma .fig
│   ├── pen-renderer/        Renderer CanvasKit/Skia mandiri
│   └── pen-sdk/             SDK payung (re-ekspor semua paket)
└── .githooks/               Pre-commit sinkronisasi versi dari nama branch
```

## Pintasan Keyboard

| Tombol | Aksi | | Tombol | Aksi |
| --- | --- | --- | --- | --- |
| `V` | Pilih | | `Cmd+S` | Simpan |
| `R` | Persegi panjang | | `Cmd+Z` | Batalkan |
| `O` | Elips | | `Cmd+Shift+Z` | Ulangi |
| `L` | Garis | | `Cmd+C/X/V/D` | Salin/Potong/Tempel/Duplikat |
| `T` | Teks | | `Cmd+G` | Grup |
| `F` | Frame | | `Cmd+Shift+G` | Pisahkan grup |
| `P` | Alat pen | | `Cmd+Shift+E` | Ekspor |
| `H` | Hand (pan) | | `Cmd+Shift+C` | Panel kode |
| `Del` | Hapus | | `Cmd+Shift+V` | Panel variabel |
| `[ / ]` | Ubah urutan | | `Cmd+J` | Chat AI |
| Panah | Geser 1px | | `Cmd+,` | Pengaturan agen |
| `Cmd+Alt+U` | Union Boolean | | `Cmd+Alt+S` | Subtract Boolean |
| `Cmd+Alt+I` | Intersect Boolean | | | |

## Skrip

```bash
bun --bun run dev          # Server pengembangan (port 3000)
bun --bun run build        # Build produksi
bun --bun run test         # Jalankan pengujian (Vitest)
npx tsc --noEmit           # Pemeriksaan tipe
bun run bump <version>     # Sinkronisasi versi di semua package.json
bun run electron:dev       # Pengembangan Electron
bun run electron:build     # Paket Electron
bun run cli:dev            # Jalankan CLI dari sumber
bun run cli:compile        # Kompilasi CLI ke dist
```

## Berkontribusi

Kontribusi sangat disambut! Lihat [CLAUDE.md](./CLAUDE.md) untuk detail arsitektur dan gaya kode.

1. Fork dan clone
2. Atur sinkronisasi versi: `git config core.hooksPath .githooks`
3. Buat cabang: `git checkout -b feat/my-feature`
4. Jalankan pemeriksaan: `npx tsc --noEmit && bun --bun run test`
5. Commit dengan [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
6. Buka PR ke `main`

## Peta Jalan

- [x] Variabel & token desain dengan sinkronisasi CSS
- [x] Sistem komponen (instans & penggantian)
- [x] Pembuatan desain AI dengan orkestrator
- [x] Integrasi server MCP dengan alur kerja desain berlapis
- [x] Dukungan multi-halaman
- [x] Impor Figma `.fig`
- [x] Operasi boolean (gabung, kurangi, potong)
- [x] Profil kemampuan multi-model
- [x] Restrukturisasi monorepo dengan paket yang dapat digunakan ulang
- [x] Alat CLI (`op`) kontrol terminal
- [ ] Pengeditan kolaboratif
- [ ] Sistem plugin

## Kontributor

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Komunitas

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Bergabung dengan Discord kami</strong>
</a>
— Ajukan pertanyaan, bagikan desain, sarankan fitur.

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## Lisensi

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

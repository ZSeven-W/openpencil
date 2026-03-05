<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Alat desain open-source berbasis AI. Design-as-Code.</strong><br />
  Dari prompt ke UI di kanvas. Orkestrasi multi-agen. Server MCP bawaan. Pembuatan kode.
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md"><b>Bahasa Indonesia</b></a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#mulai-cepat">Mulai Cepat</a> ·
  <a href="#desain-berbasis-ai">AI</a> ·
  <a href="#fitur">Fitur</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#berkontribusi">Berkontribusi</a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Klik gambar untuk menonton video demo</sub></p>

<br />

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

## Desain Berbasis AI

OpenPencil dibangun dengan AI sebagai inti — bukan sebagai plugin, melainkan sebagai alur kerja utama.

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

**Server MCP**
- Server MCP bawaan — instal satu klik ke Claude Code / Codex / Gemini / OpenCode / Kiro CLI
- Otomasi desain dari terminal: baca, buat, dan modifikasi file `.op` melalui agen yang kompatibel dengan MCP

**Pembuatan Kode**
- React + Tailwind CSS
- HTML + CSS
- CSS Variables dari token desain

## Fitur

**Kanvas & Menggambar**
- Kanvas tak terbatas dengan pan, zoom, panduan perataan cerdas, dan snapping
- Persegi panjang, Elips, Garis, Poligon, Pen (Bezier), Frame, Teks
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
- Pembaruan otomatis dari GitHub Releases
- Menu aplikasi native dan dialog file

## Tumpukan Teknologi

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Kanvas** | Fabric.js v7 |
| **State** | Zustand v5 |
| **Server** | Nitro |
| **Desktop** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **Runtime** | Bun · Vite 7 |
| **Format file** | `.op` — berbasis JSON, mudah dibaca manusia, ramah Git |

## Struktur Proyek

```text
src/
  canvas/          Mesin Fabric.js — menggambar, sinkronisasi, tata letak, panduan, alat pen
  components/      UI React — editor, panel, dialog bersama, ikon
  services/ai/     Chat AI, orkestrator, pembuatan desain, streaming
  services/figma/  Pipeline impor biner Figma .fig
  services/codegen Generator kode React+Tailwind dan HTML+CSS
  stores/          Zustand — kanvas, dokumen, halaman, riwayat, AI, pengaturan
  variables/       Resolusi token desain dan manajemen referensi
  mcp/             Alat server MCP untuk integrasi CLI eksternal
  uikit/           Sistem kit komponen yang dapat digunakan ulang
server/
  api/ai/          Nitro API — chat streaming, pembuatan, validasi
  utils/           Pembungkus klien Claude CLI, OpenCode, Codex
electron/
  main.ts          Jendela, fork Nitro, menu native, pembaruan otomatis
  preload.ts       Jembatan IPC
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

## Skrip

```bash
bun --bun run dev          # Server pengembangan (port 3000)
bun --bun run build        # Build produksi
bun --bun run test         # Jalankan pengujian (Vitest)
npx tsc --noEmit           # Pemeriksaan tipe
bun run electron:dev       # Pengembangan Electron
bun run electron:build     # Paket Electron
```

## Berkontribusi

Kontribusi sangat disambut! Lihat [CLAUDE.md](./CLAUDE.md) untuk detail arsitektur dan gaya kode.

1. Fork dan clone
2. Buat cabang: `git checkout -b feat/my-feature`
3. Jalankan pemeriksaan: `npx tsc --noEmit && bun --bun run test`
4. Commit dengan [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
5. Buka PR ke `main`

## Peta Jalan

- [x] Variabel & token desain dengan sinkronisasi CSS
- [x] Sistem komponen (instans & penggantian)
- [x] Pembuatan desain AI dengan orkestrator
- [x] Integrasi server MCP
- [x] Dukungan multi-halaman
- [x] Impor Figma `.fig`
- [ ] Operasi boolean (gabung, kurangi, potong)
- [ ] Pengeditan kolaboratif
- [ ] Sistem plugin

## Kontributor

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Komunitas

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Bergabung dengan Discord kami</strong>
</a>
— Ajukan pertanyaan, bagikan desain, sarankan fitur.

## Lisensi

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

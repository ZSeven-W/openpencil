<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/rust-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/rust-check.yml?label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — klik untuk menonton demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Klik gambar untuk menonton video demo</sub></p>

<br />

> **Catatan:** Ada proyek open-source lain dengan nama yang sama — [OpenPencil](https://github.com/open-pencil/open-pencil), yang berfokus pada desain visual kompatibel Figma dengan kolaborasi real-time. Proyek ini berfokus pada alur kerja AI-native dari desain ke kode.

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

Aplikasi web + desktop native di macOS, Windows, dan Linux — satu inti Rust, satu binary mandiri, tanpa engine browser. Asosiasi file `.op` — klik dua kali untuk membuka.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Kontrol alat desain dari terminal Anda. `op design`, `op insert` — batch design DSL, manipulasi node. Pipe dari file atau stdin. Bekerja dengan aplikasi desktop atau web server.

</td>
<td width="50%">

### 🎯 Ekspor Kode Multi-Platform

Ekspor dari satu file `.op` ke React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Variabel desain menjadi CSS custom properties.

</td>
</tr>
</table>

## Mulai Cepat

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

Atau jalankan sebagai aplikasi desktop:

```bash
cargo run -p op-host-desktop
```

> **Prasyarat:** [Rust](https://www.rust-lang.org/) (stable) untuk membangun produk. [Bun](https://bun.sh/) >= 1.0 dan [Node.js](https://nodejs.org/) >= 18 hanya diperlukan untuk web SDK di `packages/`.

### Docker

Rilis Rust bertag menerbitkan satu image web host. Image TypeScript lama yang menyertakan AI CLI tidak lagi diterbitkan.

| Image | Termasuk |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:v0.8.0` | Rust web host, wasm bundle, dan aset CanvasKit |

Web UI hanya menampilkan built-in agent profiles; alat Claude/Codex/OpenCode/Copilot/Gemini CLI tidak dibundel ke dalam image Docker.

**Jalankan:**

```bash
docker run -d -p 3100:3100 ghcr.io/zseven-w/openpencil-web:v0.8.0
```

Lalu buka `http://localhost:3100/`.

**Build secara lokal:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## Desain Berbasis AI

**Dari Prompt ke UI**

- **Teks ke desain** — deskripsikan halaman, dan hasilkan di kanvas secara real-time dengan animasi streaming
- **Orkestrator** — menguraikan halaman kompleks menjadi sub-tugas spasial untuk pembuatan secara paralel
- **Modifikasi desain** — pilih elemen, lalu deskripsikan perubahan dalam bahasa alami
- **Masukan visual** — lampirkan tangkapan layar atau mockup sebagai referensi desain

**Dukungan Multi-Agen**

| Agen                     | Pengaturan                                                                                          |
| ------------------------ | --------------------------------------------------------------------------------------------------- |
| **Bawaan (9+ penyedia)** | Pilih dari preset penyedia dengan pemilih wilayah — Anthropic, OpenAI, Google, DeepSeek dan lainnya |
| **Claude Code**          | Tanpa konfigurasi — menggunakan Claude Agent SDK dengan OAuth lokal                                 |
| **Codex CLI**            | Hubungkan di Pengaturan Agen (`Cmd+,`)                                                              |
| **OpenCode**             | Hubungkan di Pengaturan Agen (`Cmd+,`)                                                              |
| **GitHub Copilot**       | `copilot login` lalu hubungkan di Pengaturan Agen (`Cmd+,`)                                         |
| **Gemini CLI**           | Hubungkan di Pengaturan Agen (`Cmd+,`)                                                              |

**Profil Kemampuan Model** — secara otomatis menyesuaikan prompt, mode thinking, dan timeout per tingkatan model. Model tingkat penuh (Claude) mendapat prompt lengkap; tingkat standar (GPT-4o, Gemini, DeepSeek) menonaktifkan thinking; tingkat dasar (MiniMax, Qwen, Llama, Mistral) mendapat prompt JSON bertingkat yang disederhanakan untuk keandalan maksimum.

**i18n** — Lokalisasi antarmuka lengkap dalam 15 bahasa: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia.

**Server MCP**

- Server MCP bawaan (crate `op-mcp`) — instal satu klik ke Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI
- Tidak memerlukan Node.js — transport stdio melalui binary desktop (`--mcp <path>`), ditambah endpoint HTTP langsung (`127.0.0.1:<port>/mcp`) dari aplikasi yang berjalan
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
brew install zseven-w/openpencil/op
```

```bash
op start                     # Jalankan aplikasi desktop
op start --headless --file design.op # Jalankan server headless
op design @landing.txt       # Desain batch dari file
op design @ui.js             # JavaScript sandbox dengan loop
op insert '{"type":"rectangle"}' # Sisipkan sebuah node
op import:figma design.fig   # Impor file Figma
cat design.dsl | op design - # Pipe dari stdin
```

Mendukung string inline, `@filepath`, dan stdin (`-`). Bekerja dengan aplikasi desktop, server web, atau server headless berbasis file. Lihat [referensi perintah CLI](./crates/op-cli/src/usage.txt).

**LLM Skill** — instal plugin [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) untuk mengajarkan agen AI mendesain dengan `op`. Jalankan `op install` untuk agen yang terdeteksi atau `op install --target codex` untuk target tertentu.

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
- UIKit yang dapat digunakan ulang — impor/ekspor kit komponen dari file `.pen`

**AI & Agen**

- Prompt-ke-kanvas dengan pembuatan streaming dan dekomposisi spasial yang digerakkan orkestrator
- Tim Agen Bersamaan — beberapa desainer mengerjakan bagian yang berbeda secara paralel dengan indikator kanvas per anggota
- Alur kerja berlapis — `design_skeleton` → `design_content` → `design_refine` dengan prompt yang terfokus per fase
- Panduan Gaya — 50+ gaya bawaan (glassmorphism, brutalist, retro, dll.) dengan pencocokan fuzzy berbasis tag, terintegrasi ke perencanaan dan pembuatan
- Profil kemampuan multi-model — secara otomatis menyesuaikan mode berpikir, upaya, dan bentuk prompt berdasarkan tingkat model
- Runtime agen bawaan (Rust) + penyedia Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini
- Passthrough format Anthropic untuk penyedia LLM Tiongkok — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Integrasi Git**

- Wizard clone dengan autentikasi SSH / HTTPS dan manajemen kunci SSH
- Pemilih branch — buat, beralih, hapus, merge, semua dari panel Git
- Kaskade pull / push dengan percobaan ulang autentikasi dan penanganan non-fast-forward
- Merge tiga arah mode folder dengan pelacakan status `MERGE_HEAD` di disk
- Panel konflik dengan kartu tiga arah per node / field, editor JSON inline, aksi massal, dan blok diff inline
- UI pengaturan remote dan kunci SSH; i18n 15 bahasa di seluruh permukaan Git

**Ekspor**

- Ekspor kanvas — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- Ekspor kode — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- Pipeline codegen MCP inkremental — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Impor Figma**

- Impor file `.fig` dengan tata letak, fill, stroke, efek, teks, gambar, dan vektor tetap terjaga

**Aplikasi Desktop**

- macOS, Windows, dan Linux native — satu binary mandiri (winit + GPU Skia, tanpa Electron)
- Asosiasi file `.op` — klik dua kali untuk membuka, kunci instans tunggal
- Pemeriksaan pembaruan latar belakang terhadap GitHub Releases
- Menu aplikasi native dengan Simpan sebagai, Buka Terbaru, dan dialog perubahan belum tersimpan saat ditutup
- Persistensi file terbaru

## Tumpukan Teknologi

|                 |                                                                                  |
| --------------- | -------------------------------------------------------------------------------- |
| **Inti**        | Workspace Rust (`crates/`) — state editor, widget, host, MCP, AI, codegen        |
| **Rendering**   | GPU Skia di mana saja — `skia-safe` (GL) pada native, CanvasKit (WASM/WebGL2) di browser |
| **Framework UI** | jian — framework UI GPU-Skia murni-Rust yang di-vendor: widget, layout, event, hot reload (`vendor/jian`) |
| **Windowing**   | winit (fork `casement` yang di-vendor)                                          |
| **Desktop**     | Binary native `openpencil-desktop` — tanpa engine browser                        |
| **Web SDK**     | `op-web-sdk` + adapter React 19 / Vue 3 — viewer `.op` baca-saja (TypeScript)     |
| **CLI**         | `op` — kontrol terminal, batch design DSL                                        |
| **AI**          | Runtime agen Rust bawaan · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**        | clippy · rustfmt (Rust) · oxlint · oxfmt (web SDK)                               |
| **Format file** | `.op` — berbasis JSON, mudah dibaca manusia, ramah Git                           |

## Ekosistem

OpenPencil adalah bagian dari keluarga alat murni-Rust, berbasis AI dari **[ZSeven-W](https://github.com/ZSeven-W)**. Mereka saling melengkapi: `jian` merender OpenPencil, `agent-rs` menjalankan agennya, `noema` mengingat, dan `zode` mendesain dari terminal.

| Proyek | Apa itu |
| ------ | ------- |
| **[Zode](https://github.com/ZSeven-W/zode)** | Asisten koding open-source berbasis AI untuk terminal Anda — TUI Rust yang cepat (`ratatui`) yang membaca kode Anda, menjalankan perintah, mencari file, dan mengelola git. Menggerakkan OpenPencil melalui MCP. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | Runtime async murni-Rust untuk merilis agen LLM — multi-penyedia, mampu memakai alat secara ujung-ke-ujung, izin terstruktur, MCP asli, tanpa `unsafe`. Menggerakkan runtime agen bawaan OpenPencil (`vendor/agent`) dan Zode. |
| **[jian](https://github.com/ZSeven-W/jian)** | Framework UI GPU-Skia murni-Rust — widget, layout, event, dan hot reload dalam satu stack. Mengubah dokumen `.op` deklaratif menjadi aplikasi native yang dapat dikendalikan AI tanpa runtime JS, tanpa DOM, tanpa Electron. Framework UI OpenPencil (`vendor/jian`). |
| **[noema](https://github.com/ZSeven-W/noema)** | Sistem memori local-first, non-vektor untuk agen koding. Memori tahan lama sebagai file yang dapat diperiksa, antrean tinjauan untuk entri baru, dan pengambilan leksikal (tanpa embedding) — bekerja di seluruh Zode, Codex, Claude Code, dan runtime MCP. |

## Mengapa Rust

OpenPencil telah ditulis ulang dari awal dalam **Rust** ([#129](https://github.com/ZSeven-W/openpencil/issues/129)). Penulisan ulang ini telah selesai — editor TypeScript + Electron dipensiunkan pada `v0.7.5`, dan workspace Rust di repo ini adalah produknya: satu inti native yang jauh lebih kecil dan lebih cepat, serta berjalan di lebih banyak platform dari satu basis kode.

|                           | TypeScript + Electron (dipensiunkan, `v0.7.5`)     | Rust (saat ini)                                                                |
| ------------------------- | -------------------------------------------------- | ----------------------------------------------------------------------------- |
| **Runtime desktop**       | Electron — menyertakan Chromium + Node.js          | Jendela native (`winit` + GPU Skia), tanpa engine browser                     |
| **Jejak desktop**         | Runtime Chromium penuh per instalasi               | Satu binary mandiri — **55.5 MB**                                             |
| **Payload web**           | Bundle JS + WASM                                   | **8.2 MB** wasm / **2.18 MB** gzip melalui jaringan                          |
| **Rendering**             | CanvasKit/Skia di web                              | Satu backend Skia berakselerasi GPU di **setiap** target                      |
| **Memori**                | Jeda JavaScript GC                                 | Tanpa GC — kepemilikan Rust, latensi yang dapat diprediksi                   |
| **Basis kode**            | Web stack + Electron              | Satu workspace Rust: editor · CLI · MCP · AI · codegen · Figma · Git          |
| **Target**                | Web + desktop, dua stack terpisah                  | Desktop (macOS/Win/Linux) · mobile (iOS/Android) · browser — satu inti        |

**Peningkatan terukur**

- **Jejak kecil** — seluruh aplikasi desktop adalah satu binary native **55.5 MB** alih-alih engine browser terbundel ditambah runtime Node. Build web berukuran **8.2 MB** mentah / **2.18 MB** gzip setelah pemisahan katalog ikon (−48% melalui jaringan).
- **Skalabel untuk dokumen besar** — kanvas live dengan **10,000 node** (auto-layout bertingkat, empat level dalam) menulis, membaca, dan mengambil snapshot layout **tanpa panik dan ~0% CPU idle**; snapshot layout penuh dari semua 10k node dikembalikan dalam **~0.68 dtk**.
- **Interaksi cepat** — pan/zoom tidak lagi menyerial ulang dokumen setiap frame (satu perbaikan hot-path memotong CPU wheel-zoom dari **~69% menjadi ~0%**); seret menambal scene secara inkremental, pengukuran teks di-cache, dan repaint digabungkan menjadi satu per frame.
- **Satu inti, setiap layar** — state editor yang sama dan backend render yang sama dikompilasi ke desktop native, mobile, dan browser melalui WASM — tanpa reimplementasi paralel yang perlu disinkronkan.
- **GPU Skia di mana saja** — native merender melalui `skia-safe` pada konteks GL; browser merender melalui CanvasKit pada WebGL2 — kode gambar yang sama, keluaran yang sama.
- **Aksesibilitas native** — AccessKit di macOS, Windows, dan Linux, ditambah mirror DOM di web, alih-alih mengandalkan pohon a11y browser.
- **Satu workspace bertipe** — host MCP, CLI, penyedia AI, pembuatan kode, impor Figma, dan integrasi Git semuanya berada dalam satu workspace Rust, dengan penjagaan rantai pasokan `cargo-deny` di CI.

> **Status:** editor TypeScript dipensiunkan pada `v0.7.5` dan kini hanya ada dalam riwayat git; repositori ini adalah workspace Rust. Rilis Rust `v0.8.0` sedang dalam pengembangan aktif (lihat Peta Jalan di bawah).

## Struktur Proyek

```text
openpencil/
├── crates/                   Workspace Rust — produknya
│   ├── op-editor-core/       State editor `.op` (PenDocument) kanonis + EditorCommand + variabel desain
│   ├── op-editor-ui/         Widget platform-independen + fasad RenderBackend (wasm32-clean)
│   ├── op-editor-host-core/  State machine host tanpa transport, digunakan bersama semua host
│   ├── op-host-native/       Lib host native — winit + skia-safe GL (desktop + mobile)
│   ├── op-host-web/          Bundle browser — wasm32 cdylib, renderer CanvasKit
│   ├── op-host-desktop/      Binary desktop `openpencil-desktop`; juga daemon `--serve-web`
│   ├── op-host-services/     Lib daemon serve-web / MCP headless
│   ├── op-host-web-server/   Binary web-server ringan tanpa GL
│   ├── op-cli/               Alat CLI — perintah `op`
│   ├── op-mcp/               Server MCP — alat, batch design, alur kerja berlapis
│   ├── op-ai/                Penyedia AI, runtime chat, streaming
│   ├── op-ai-skills/         Engine skill prompt AI (pemuatan prompt bertahap)
│   ├── op-orchestrator/      Orkestrasi tim agen konkuren
│   ├── op-codegen/           Generator kode (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Parser dan konverter file Figma .fig
│   ├── op-git/               Integrasi Git — clone, branch, push/pull, merge
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Workspace Web SDK (Bun)
│   ├── op-web-sdk/           SDK viewer web `.op` baca-saja (membungkus bundle wasm)
│   ├── op-web-sdk-react/     Adapter React 19
│   └── op-web-sdk-vue/       Adapter Vue 3
├── vendor/                   Subsistem yang di-vendor (git submodules)
│   ├── jian/                 Framework UI GPU-Skia — widget/render/event
│   ├── casement/             Fork winit
│   └── agent/                Runtime agen Rust lintas produk (agent-rs)
└── .githooks/                Pre-commit sinkronisasi versi dari nama branch
```

## Pintasan Keyboard

| Tombol      | Aksi              |     | Tombol        | Aksi                         |
| ----------- | ----------------- | --- | ------------- | ---------------------------- |
| `V`         | Pilih             |     | `Cmd+S`       | Simpan                       |
| `R`         | Persegi panjang   |     | `Cmd+Z`       | Batalkan                     |
| `O`         | Elips             |     | `Cmd+Shift+Z` | Ulangi                       |
| `L`         | Garis             |     | `Cmd+C/X/V/D` | Salin/Potong/Tempel/Duplikat |
| `T`         | Teks              |     | `Cmd+G`       | Grup                         |
| `F`         | Frame             |     | `Cmd+Shift+G` | Pisahkan grup                |
| `P`         | Alat pen          |     | `Cmd+Shift+P` | Ekspor (PNG/JPG/WEBP/PDF)    |
| `H`         | Hand (pan)        |     | `Cmd+Shift+C` | Panel kode                   |
| `Del`       | Hapus             |     | `Cmd+Shift+V` | Panel variabel               |
| `[ / ]`     | Ubah urutan       |     | `Cmd+J`       | Chat AI                      |
| Panah       | Geser 1px         |     | `Cmd+,`       | Pengaturan agen              |
| `Cmd+Alt+U` | Union Boolean     |     | `Cmd+Alt+S`   | Subtract Boolean             |
| `Cmd+Alt+I` | Intersect Boolean |     | `Cmd+Shift+S` | Simpan sebagai               |

## Skrip

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
cd packages && bun run bump <version>             # Sync SDK package.json versions
```

## Berkontribusi

Kontribusi sangat disambut! Lihat [CLAUDE.md](./CLAUDE.md) untuk detail arsitektur dan gaya kode.

1. Fork dan clone
2. Atur sinkronisasi versi: `git config core.hooksPath .githooks`
3. Buat cabang: `git checkout -b feat/my-feature`
4. Jalankan pemeriksaan: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
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
- [x] Workspace Cargo dengan crate Rust dan paket SDK web yang dapat digunakan ulang
- [x] Editor Rust untuk desktop dan web
- [x] Alat CLI (`op`) kontrol terminal
- [x] Runtime agen Rust bawaan dengan dukungan multi-penyedia
- [x] i18n — 15 bahasa
- [x] SDK viewer berbasis wasm untuk JavaScript, React, dan Vue
- [x] Style Guides dengan pencocokan berbasis tag dan alat MCP
- [x] Agent Teams serentak dengan delegasi dan indikator kanvas
- [x] Integrasi Git (clone, branch, push/pull, merge tiga arah mode folder)
- [x] Ekspor kanvas (SVG / PNG / JPEG / WEBP / PDF)
- [ ] Pengeditan kolaboratif
- [ ] Sistem plugin

## Kontributor

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Sponsor

OpenPencil gratis dan open source. Pengembangan didanai oleh orang-orang yang merasa terbantu — terima kasih telah menjaga kanvas tetap terbuka.

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

Terima kasih kepada **[MrQyun](https://github.com/mrqyun)** — ingin nama Anda muncul di sini? **[Jadi sponsor →](https://github.com/sponsors/ZSeven-W)**

## Komunitas

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
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

## Penilaian

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## Lisensi

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

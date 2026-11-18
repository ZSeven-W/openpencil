<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>Plugin DeepSeek Harness untuk OpenPencil — pratinjau, periksa, dan edit dokumen <code>.op</code> asli di dalam percakapan.</strong><br />
  <sub>Pratinjau Multi-Frame Akurat &bull; Kanvas Interaktif &bull; Editor Terkelola &bull; Alat Desain Asli-Agen</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · Rilis plugin saat ini: <code>0.1.0-rc.7</code> · Diuji hingga DSH <code>0.1.1-rc.2</code></sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md"><b>Bahasa Indonesia</b></a>
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
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil — pratinjau multi-frame dan editor bilah samping" width="100%" />
</p>
<p align="center"><sub>Pratinjau <code>.op</code> multi-frame akurat dengan kanvas interaktif dan workbench editor terkelola</sub></p>

## Mengapa DSH OpenPencil

DSH OpenPencil menghubungkan [DeepSeek Harness](https://github.com/deepseek-ai/DSH) dengan [OpenPencil](https://github.com/ZSeven-W/openpencil) sehingga sebuah Agen menggerakkan kanvas desain yang nyata, dapat diedit, dan interaktif — alih-alih mengembalikan gambar yang dihasilkan.

<table>
<tr>
<td width="50%">

### 🖼️ Pratinjau Multi-Frame Akurat

Eksportir OpenPencil headless yang terpasang merender pratinjau yang setia pada desain: frame tingkat-teratas pertama sebagai PNG besar yang aman untuk diputar ulang, plus deretan thumbnail yang dapat digulir secara horizontal, klik-untuk-memilih, serta navigasi sebelumnya/berikutnya untuk dokumen multi-frame.

</td>
<td width="50%">

### 🗺️ Kanvas Interaktif

"Buka kanvas interaktif" memasang OpenPencil Web SDK bersifat baca-saja secara lazy dengan pan, zoom, dan fit — periksa halaman mana pun, simpul bersarang, atau halaman nonaktif tanpa meninggalkan percakapan.

</td>
</tr>
<tr>
<td width="50%">

### ✏️ Editor Terkelola

Dengan `editable: true`, aksi edit membuka editor OpenPencil terkelola — seleksi, lapisan, properti, alat gambar, undo/redo, dan semantik simpan eksplisit — di workbench sisi kanan yang dapat diubah ukurannya dengan opsi layar penuh.

</td>
<td width="50%">

### 🤖 Alat Desain Asli-Agen

Lima alat kanvas langsung dan enam alat `openpencil_pipeline_*` memungkinkan Agen membuat, memeriksa, menyempurnakan, menerbitkan, mengubah, dan membaca kanvas nyata melalui runtime OpenPencil terkelola.

</td>
</tr>
<tr>
<td width="50%">

### 🔐 Izin Berbasis Kapabilitas

Izin gambar dan dokumen adalah kapabilitas bertanda tangan yang terikat hash. Metadata peramban tidak pernah mengekspos jalur host sembarangan, dan kapabilitas pratinjau/editor bertanda tangan tidak pernah masuk ke hasil alat kanonis atau konteks model.

</td>
<td width="50%">

### ⚡ Keamanan Transaksional

Dokumen pipeline penuh tetap menjadi draf privat yang belum diterbitkan sampai semua gerbang kualitas native dan DSH lolos. Penerbitan tidak menimpa jalur yang sudah ada, dan pembatalan atau batch gagal tidak meninggalkan target kosong.

</td>
</tr>
<tr>
<td width="50%">

### 🌍 Mengikuti Tampilan & Nuansa DSH

Kartu alat dan editor terkelola mengikuti lokale Tionghoa/Inggris serta tema terang/gelap DSH tanpa memuat ulang sesi pengeditan.

</td>
<td width="50%">

### 🎯 Satu Alur Kerja Lengkap

"Kebutuhan → private live canvas di bilah samping → dua batch semantik dengan PNG presisi untuk pengguna → gerbang kualitas deterministik native/DSH → penerbitan atomik" — satu alur lengkap di dalam DSH.

</td>
</tr>
</table>

## Instalasi ke DSH

DSH adalah paket terpisah. Pasang sekali jika belum ada:

```sh
npm install -g @deepseek-ai/dsh@latest
```

Lalu tambahkan plugin ke sebuah profil dan jalankan aplikasi web:

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@next
dsh web
```

Untuk pengembangan lokal, bangun checkout ini, tautkan path absolutnya ke profil Web, lalu mulai ulang DSH sepenuhnya:

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

Dependensi `link:` membuat hasil build ulang berikutnya langsung terlihat dari checkout ini. Namun, DSH harus dimulai ulang sepenuhnya setelah dependensi profil diganti karena profil Web bawaan tidak melakukan hot reload bundle host secara default.

Tidak ingin memasang DSH secara global? Jalankan dua langkah yang sama lewat `pnpm dlx`:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@next
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> Plugin OpenPencil bersifat publik dan tidak memerlukan token npm. Jika DSH prerelease itu sendiri memerlukan autentikasi registry, simpan kredensial tersebut di konfigurasi npm tingkat-pengguna atau sementara di luar checkout. Repositori ini sengaja tidak memuat kredensial registry apa pun.

## Alat Desain

Pilihan opsional mengikuti `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin` dengan brief asli dan `style_selection`. Katalog dan pilihan tepat tidak membutuhkan rekomendasi; penyedia bawaan dimatikan. Handoff mendukung script, bukan app-agent atau commerce. Pembuatan app-agent biasa tetap tersedia.

| Alat | Fungsinya |
| --- | --- |
| `openpencil_new` | Jalur cepat kompatibel untuk pekerjaan sederhana: menjalankan satu skrip QuickJS `batch_design` transaksional, menerbitkan hanya jika target belum ada, dan mengembalikan presentasi yang dapat diedit. Utamakan pipeline penuh untuk desain produksi. |
| `openpencil_pipeline_begin` | Memulai draf privat milik sesi beserta satu-satunya root, langsung membuka live canvas yang sama di bilah samping, dan menjaga target `.op` tetap belum diterbitkan. |
| `openpencil_pipeline_context` | Memuat satu detail guideline, style, theme, atau UI kit yang terbatas hanya bila detail tersebut benar-benar tidak ada dalam kontrak begin; ini bukan loop penyegaran saat startup. |
| `openpencil_pipeline_batch` | Menjalankan paling banyak dua skrip generasi QuickJS langsung. Setiap transaksi yang berhasil hanya mencoba menampilkan PNG presisi kepada pengguna; setelah alat kembali, segera ikuti `next`. |
| `openpencil_pipeline_inspect` | Menyediakan diagnosis manual hanya bila pengguna memintanya secara eksplisit; generasi biasa tidak menggunakannya sebagai langkah pratinjau atau pemeriksaan gambar oleh model. |
| `openpencil_pipeline_finish` | Menjalankan gerbang deterministik native dan DSH, merender PNG presisi pasca-final, lalu menerbitkan secara atomik dengan `createIfAbsent`. Hanya hasil `needs_correction`, `canContinue: true`, `repairTargets` lengkap dan tidak kosong, serta `omitted: 0` yang mengizinkan satu perbaikan U-only dan satu finish terakhir; hasil lain yang belum terbit bersifat terminal. |
| `openpencil_pipeline_abort` | Membuang draf yang belum diterbitkan tanpa membuat file target. |
| `openpencil_create` | Menerapkan program `batch_design` transaksional untuk menghasilkan atau menyusun ulang simpul pada kanvas langsung yang sudah ada. |
| `openpencil_edit` | Mengubah simpul eksplisit atau simpul tunggal yang dipilih pengguna. |
| `openpencil_render` | Membuat snapshot `.op` yang tidak dapat diubah dan diarahkan-oleh-konten, lalu merender setiap frame tingkat-teratas pada halaman aktif — `scale` dan `editable` opsional. |
| `openpencil_selection` | Membaca simpul persis yang dipilih pada kanvas editor langsung. |

## Alur Kerja Desain Agen

Untuk generasi biasa, gunakan jalur singkat tetap: `openpencil_pipeline_begin` → dua `openpencil_pipeline_batch` QuickJS langsung → satu `openpencil_pipeline_finish`. Begin membuat satu-satunya root dan langsung membuka live canvas privat di bilah samping; jalur workspace yang diminta belum ada sampai penerbitan berhasil. Setelah setiap begin atau batch berhasil, segera lakukan panggilan berikutnya tanpa narasi, perencanaan, perbandingan, inspeksi, atau alat yang tidak terkait. Secara default seluruh desain memakai tepat satu gambar; setiap frame Hero/Product/Art/Media memiliki tepat satu visual utama, jangan pernah memasangkan gambar dengan ikon placeholder.

Kedua batch yang berhasil mencoba menampilkan pratinjau PNG presisi kepada pengguna. Setelah alat kembali, segera ikuti `next`. Jika `next` melaporkan `previewUnavailable`, skrip sudah di-commit ke live canvas: jangan jalankan ulang batch dan jangan panggil `openpencil_pipeline_inspect` atau `read_image`. `openpencil_pipeline_inspect` hanya untuk diagnosis yang diminta pengguna secara eksplisit.

Finish menjalankan finalisasi, lint, pemeriksaan kontras dan layout native OpenPencil serta gerbang kualitas deterministik DSH, lalu dalam satu panggilan sehat merender PNG final presisi dan menerbitkan target secara atomik. Perbaikan hanya diizinkan bila hasil sekaligus memuat `stage: "needs_correction"`, `canContinue: true`, `repairTargets` lengkap dan tidak kosong dengan `omitted: 0`, serta setiap target memiliki `operation: "U"`, `nodeId` persis yang tidak kosong, dan `patch` tidak kosong. Terapkan semua target bersama dalam tepat satu batch U-only, lalu tanpa narasi panggil finish tepat satu kali lagi. Hasil lain yang belum terbit, error, atau `canContinue: false` bersifat terminal: laporkan sekali dan jangan retry, inspect, membaca image/context, abort, atau membuat draf pengganti. Jika gerbang gagal atau `openpencil_pipeline_abort` dipanggil, target tetap tidak ada. PNG final presisi dan live editor dalam hasil published sudah authoritative; kembalikan hasil itu dan segera akhiri. Bilah samping hanya auto-open saat idle dan **Edit canvas** tetap tersedia untuk peralihan eksplisit.

Dalam layanan DSH yang sama dan masih berjalan, berganti browser atau memuat ulang dapat memulihkan publication tahan lama yang diparse secara ketat dari `openpencil_new` atau `openpencil_pipeline_finish` sebagai PNG presisi beserta aksi **Edit canvas** yang eksplisit. Kartu historis tidak pernah membuka bilah samping secara otomatis; pengguna harus mengeklik aksi tersebut. `openpencil_render` historis biasa tetap hanya-baca, dan koneksi non-loopback tidak pernah menerima izin editor.

Skill `openpencil-design` bawaan tetap menjadi panduan scripting dan kualitas, sementara runtime terkelola tidak bergantung pada biner desktop. `openpencil_new` tetap menjadi jalur cepat satu batch yang kompatibel, tetapi generasi berkualitas produksi sebaiknya mengutamakan pipeline penuh.

Gunakan `openpencil_create` dan `openpencil_edit` hanya untuk kanvas langsung yang sudah ada. Pengeditannya tetap belum disimpan hingga aksi Simpan pada editor.

## Aset Penampil Web

DSH hanya menyajikan `client.js` untuk plugin klien, sehingga OpenPencil ESM SDK, WASM-nya, dan CanvasKit disiapkan sebagai aset same-origin yang eksplisit:

```sh
pnpm run sync:viewer-assets
```

Perintah sinkronisasi menggunakan root monorepo OpenPencil (`../..`), dengan fallback ke checkout `../openpencil` yang bersebelahan. Ganti dengan `OPENPENCIL_ROOT` atau `--openpencil-root`. Direktori aset pra-build yang lengkap dapat dipilih dengan `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE`. Pencarian saat runtime dapat diganti dengan `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR`.

Aset penampil dimuat secara lazy hanya setelah pengguna membuka kanvas. Jika tidak ada atau tidak valid, pratinjau PNG tetap tersedia dan tidak ada tombol kanvas yang ditampilkan.

## Editor Terkelola

Sesi yang dapat diedit menggunakan host web terkelola OpenPencil — arsitektur yang sama dengan `op-vscode`. Plugin memulai host hanya setelah aksi pengguna yang berwenang, menyimpan token daemon di memori, memvalidasi sumber dan origin iframe, serta menutup proses saat sesi editor berakhir. Permukaan editor dipilih secara progresif: detail Tool native saat host mendeklarasikan sambungan tersebut, jika tidak maka workbench sisi kanan plugin dengan kontrol ubah-ukuran dan layar penuh.

Startup menggunakan listening handshake yang aman untuk mount lambat: probe kesiapan baru dimulai setelah host bawaan mengumumkan alamat yang telah di-bind. Instalasi OpenPencil desktop tidak diperlukan.

Instalasi yang dipublikasikan memilih paket yang sesuai dengan OS/CPU saat ini dari enam paket platform native: `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`, `win32-arm64`, dan `win32-x64`; kedua paket Linux menargetkan glibc. Paket root mendeklarasikan semuanya sebagai `optionalDependencies` dengan versi persis agar pengelola paket memilih varian yang benar (misalnya `@zseven-w/dsh-openpencil-darwin-arm64`). Paket tersebut menyertakan `op-host-web-server`, bundel web editor, dan CanvasKit sebagai satu runtime yang saling cocok. Karena itu, editor terkelola tidak bergantung pada `/Applications/OpenPencil.app`, `openpencil-desktop` di `PATH`, ataupun checkout sumber OpenPencil.

Jika DSH memuat ulang atau melepas plugin saat kanvas masih kotor (belum disimpan), host menyimpan draf pemulihan lokal yang tidak transparan hingga tujuh hari. Membuka ulang sumber yang sama akan menanyakan sebelum memulihkannya ke kanvas langsung; pemulihan tidak pernah menimpa file `.op` hingga pengguna menyimpan secara eksplisit.

Paket resmi untuk keenam platform menerima endpoint bootstrap kolaborasi China dan Global selama build release yang dilindungi; nilai yang disuntikkan divalidasi sebelum dipublikasikan. Build mandiri lokal tanpa penyuntikan tersebut dapat menimpa bootstrap sebelum memulai DSH dengan `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap`; nilainya harus menggunakan `https` dan path harus tepat `/api/v1/collaboration/bootstrap`.

Sinkronisasi kanvas lintas perangkat mengharuskan runtime native PC/DSH dan aplikasi seluler sama-sama diperbarui ke lini rilis OpenPencil yang sama dan memuat perbaikan antrean kolaborasi saat ini. Menggabungkan aplikasi seluler lama dengan runtime PC yang lebih baru masih dapat menampilkan kursor jarak jauh tanpa menerima commit kanvas.

Saat mengembangkan dari repositori ini, sebelum menjalankan DSH build terlebih dahulu Web bundle editor, lalu host native, kemudian stage runtime yang saling cocok tersebut.

`pnpm run build:editor-web` menjalankan WASM bundle gate yang didukung resmi oleh OpenPencil. Langkah ini memerlukan Bash, Cargo/Rust dengan target `wasm32-unknown-unknown`, CLI `wasm-bindgen`, `wasm-opt` dari Binaryen, Node.js, dan `gzip`; CanvasKit tidak memerlukan EMSDK. Web build tidak menggunakan variabel build bootstrap kolaborasi. Sebelum `pnpm run build:editor-runtime`, tetapkan `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN` dan `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL`. Keduanya hanya digunakan oleh native Cargo build, yang akan fail closed jika salah satunya tidak ada. Setelah kedua build berhasil, stage runtime dengan perintah terakhir.

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

Override runtime eksplisit hanya diterima sebagai satu set lengkap yang saling cocok:

- `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY` untuk `op-host-web-server`;
- `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR` untuk bundel web editor yang telah dibangun;
- `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR` untuk aset CanvasKit.

Memberikan hanya sebagian dari set tersebut merupakan konfigurasi yang tidak valid; plugin tidak menggabungkan path khusus dengan aset runtime yang disertakan dalam paket.

Penyimpanan menggunakan hash sumber optimistis, penggantian atomik, dan kapabilitas penerus. Jika sumber berubah di luar editor, plugin melaporkan konflik alih-alih menimpanya.

## Metadata Hasil

Hasil yang terlihat model tetap berupa JSON polos. `presentationMeta.$dshOpenPencil` khusus peramban membawa izin tambahan untuk:

- `image`: jalur PNG, URL pratinjau/unduh, serta lebar/tinggi asli;
- `frames`: setiap frame tingkat-teratas yang dirender akurat sesuai urutan halaman aktif, termasuk id/nama/indeks simpul dan URL PNG bertanda tangan;
- `document`: jalur aksi sumber plus URL snapshot yang tidak dapat diubah, byte, dan SHA-256;
- `viewer`: URL SDK/WASM/CanvasKit berrevisi saat rute aset terpasang;
- `editor`: kapabilitas peluncuran/penyegaran terbatas saat `editable: true` diberi otorisasi.

Hasil juga mencatat `renderer`, `rendererBinary`, `fidelity`, dan peringatan apa pun. Pesan schema-v1 yang hanya berisi PNG yang sudah ada tetap dapat dirender.

DSH `0.1.1-rc.2` tidak menyimpan metadata presentasi peramban secara persisten untuk alat yang bersarang di bawah PTC/Code Mode. Plugin memulihkan proyeksi UI-only tersebut melalui endpoint same-origin yang terikat sesi: peramban hanya mengirim session id, call id, dan SHA-256 dokumen yang tidak dapat diubah, sementara host menyelesaikan hasil otoritatif dari log sesi DSH yang tahan lama dan menggunakan penanda dalam-proses berumur pendek hanya untuk mengotorisasi pengeditan langsung terbaru. Kapabilitas pratinjau/editor bertanda tangan tidak pernah masuk ke hasil alat kanonis atau konteks model. Riwayat tahan lama dari `openpencil_render` biasa tetap hanya-baca. Publication tahan lama yang diparse secara ketat dari `openpencil_new` atau `openpencil_pipeline_finish` hanya dapat menerima izin editor melalui loopback dan setelah klik eksplisit pengguna; pembukaan otomatis bilah samping hanya untuk hasil langsung terbaru yang tepercaya.

Untuk pemutaran ulang yang terbatas, pemulihan metadata bersarang menerima hingga 128 frame tingkat-teratas; hasil Code Mode yang lebih besar tetap tersedia melalui fallback JSON kanonisnya.

## Batasan Saat Ini

- Pengeditan lanjutan pada kanvas yang sudah ada memerlukan editor terkelola yang sudah terbuka. Perubahan tetap belum disimpan hingga pengguna memanggil aksi Simpan.
- Kanvas Web SDK yang ringan bersifat baca-saja; pengeditan penuh menggunakan permukaan editor terkelola terpisah. Pada DSH `0.1.1-rc.2`, plugin menggunakan workbench kanan yang dapat diubah ukurannya dengan opsi layar penuh.
- Galeri akurat mencakup frame tingkat-teratas pada halaman aktif; kanvas interaktif tetap menjadi cara untuk memeriksa halaman nonaktif dan simpul bersarang.
- Cache render dan snapshot masih memerlukan kebijakan retensi tingkat produk.

## Struktur Proyek

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

## Build dan Verifikasi

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

Build memerlukan Node 24.11 atau lebih baru dan pnpm. Paket host/klien DSH adalah dependensi peer yang disediakan oleh profil DSH target. Alat build diselesaikan dari dependensi dev lokal, checkout DSH tertaut yang aktif, atau bundel sumber DSH yang terpasang; `DSH_SOURCE_ROOT` dapat memilih checkout sumber secara eksplisit. Lockfile mengunci alat build publik yang berdiri sendiri saat lingkungan tersebut disediakan secara terpisah.

Untuk DSH prerelease privat, simpan kredensial npm yang diterbitkan di luar repositori ini (misalnya di `.npmrc` tingkat-pengguna atau sementara) dan jalankan versi yang diminta secara langsung:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

Jangan pernah melakukan commit pada `.npmrc`, `NPM_TOKEN`, atau kredensial registry yang disalin. Repositori ini mengabaikan konfigurasi npm lokal secara default.

`test:host` melakukan render akurat sungguhan, memvalidasi geometri PNG IHDR dan SHA-256, menguji kapabilitas gambar/dokumen yang tidak dapat diubah melalui HTTP, dan memeriksa bahwa aset penampil dapat diberikan izin. Dimensi yang diharapkan bersifat spesifik terhadap fixture.

## Ekosistem

DSH OpenPencil adalah plugin DeepSeek Harness untuk **[OpenPencil](https://github.com/ZSeven-W/openpencil)** — alat desain vektor AI-native open-source pertama di dunia — dan bagian dari keluarga **[ZSeven-W](https://github.com/ZSeven-W)** yang berisi alat AI-native murni-Rust.

| Proyek | Penjelasan |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | Alat desain yang digerakkan plugin ini — generasi prompt-ke-kanvas, tim agen konkuren, file `.op` desain-sebagai-kode, dan server MCP bawaan. Pratinjau akurat, kanvas interaktif, dan editor terkelola di sini ditenagai oleh OpenPencil itu sendiri. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | Runtime async murni-Rust untuk mengirimkan agen LLM — multi-penyedia, mampu menggunakan alat end-to-end, izin terstruktur, MCP nyata, tanpa `unsafe` sama sekali. Menopang runtime agen bawaan OpenPencil. |
| **[jian](https://github.com/ZSeven-W/jian)** | Framework UI GPU-Skia murni-Rust — widget, tata letak, event, dan hot reload dalam satu stack. Framework UI OpenPencil, dan sumber renderer fallback plugin ini. |
| **[Zode](https://github.com/ZSeven-W/zode)** | Asisten pengodean AI-native open-source untuk terminal Anda — membaca kode Anda, menjalankan perintah, dan menggerakkan OpenPencil melalui MCP. |
| **[noema](https://github.com/ZSeven-W/noema)** | Sistem memori non-vektor, local-first untuk agen pengodean — memori tahan lama sebagai file yang dapat diperiksa, bekerja di berbagai runtime. |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | Plugin skill LLM yang mengajarkan agen AI cara mendesain dengan `op` — pendamping plugin DSH ini. |

Plugin DSH lainnya:

- [DSH Android](https://github.com/ZSeven-W/dsh-android) — emulator Android atau perangkat USB langsung di dalam percakapan, digerakkan sepenuhnya melalui adb
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) — delegasikan pekerjaan ke agen DSH dari Claude Code / Codex
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) — iOS Simulator langsung — dan iPhone via USB — di dalam percakapan
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) — memori jangka panjang untuk DSH

## Kontribusi

Kontribusi sangat diterima! Fork dan clone, buat branch, jalankan `pnpm run build` dan rangkaian tes, commit dengan [Conventional Commits](https://www.conventionalcommits.org/), lalu buka PR ke `main`.

## Komunitas

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Gabung Discord kami</strong>
</a>
— Ajukan pertanyaan, bagikan desain, sarankan fitur.

**Komunitas yang diakui: [LINUX DO](https://linux.do/)**

## Lisensi

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

Komponen pihak ketiga tercantum di [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md).

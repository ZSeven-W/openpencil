# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [**Bahasa Indonesia**](./README.id.md)

CLI untuk [OpenPencil](https://github.com/ZSeven-W/openpencil) — kendalikan alat desain dari terminal Anda.

## Instalasi

```bash
npm install -g @zseven-w/openpencil
```

## Dukungan Platform

CLI secara otomatis mendeteksi dan meluncurkan aplikasi desktop OpenPencil di semua platform:

| Platform    | Jalur instalasi yang terdeteksi                                                                     |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS per-pengguna (`%LOCALAPPDATA%`), per-mesin (`%PROGRAMFILES%`), portabel                        |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## Penggunaan

```bash
op <perintah> [opsi]
```

### Metode Input

Argumen yang menerima JSON atau DSL dapat diberikan dengan tiga cara:

```bash
op design '...'              # String inline (data kecil)
op design @design.txt        # Baca dari file (disarankan untuk desain besar)
cat design.txt | op design - # Baca dari stdin (piping)
```

### Kontrol Aplikasi

```bash
op start [--desktop|--web]   # Jalankan OpenPencil (desktop secara default)
op stop                      # Hentikan instance yang berjalan
op status                    # Periksa apakah sedang berjalan
```

### Desain (Batch DSL)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### Operasi Dokumen

```bash
op open [file.op]            # Buka file atau hubungkan ke kanvas langsung
op save <file.op>            # Simpan dokumen saat ini
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # Dapatkan seleksi kanvas saat ini
```

### Manipulasi Node

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### Ekspor Kode

```bash
op export <format> [--out file]
# Format: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### Variabel & Tema

```bash
op vars                      # Dapatkan variabel
op vars:set <json>           # Atur variabel
op themes                    # Dapatkan tema
op themes:set <json>         # Atur tema
op theme:save <file.optheme> # Simpan preset tema
op theme:load <file.optheme> # Muat preset tema
op theme:list [dir]          # Daftar preset tema
```

### Halaman

```bash
op page list                 # Daftar halaman
op page add [--name N]       # Tambah halaman
op page remove <id>          # Hapus halaman
op page rename <id> <name>   # Ganti nama halaman
op page reorder <id> <index> # Urutkan ulang halaman
op page duplicate <id>       # Duplikasi halaman
```

### Impor

```bash
op import:svg <file.svg>     # Impor file SVG
op import:figma <file.fig>   # Impor file Figma .fig
```

### Tata Letak

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### Flag Global

```text
--file <path>     File .op target (default: kanvas langsung)
--page <id>       ID halaman target
--pretty          Output JSON yang mudah dibaca
--help            Tampilkan bantuan
--version         Tampilkan versi
```

## Lisensi

MIT

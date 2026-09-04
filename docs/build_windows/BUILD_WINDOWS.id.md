
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · **Bahasa Indonesia**

# Panduan Build OpenPencil untuk Windows

## 1. Mengunduh Sumber Kode

1. Unduh sumber kode dari repositori openpencil

2. openpencil memiliki 3 submodul dependensi di direktori `vendor` (jian, casement, agent) yang perlu diunduh secara terpisah

    - 2.1 jian (framework UI): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → ekstrak ke `vendor\jian`

    - 2.2 casement (pembungkus window): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → ekstrak ke `vendor\casement`

    - 2.3 agent (runtime Agent): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → ekstrak ke `vendor\agent`

3. Setelah ekstraksi, pastikan path berikut ada:

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. Persiapan Lingkungan — *Windows 10 x64 (toolchain MSVC)*

1. Instal Visual Studio dengan memilih **"Development with C++ for Desktop"**

2. Instal toolchain Rust `stable-x86_64-pc-windows-msvc`

    - 2.1 Konfigurasi mirror rustup (untuk mempercepat unduhan)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Konfigurasi mirror dependensi Cargo. Buka direktori pengguna (`C:\Users\<username>\`), temukan atau buat folder `.cargo`, lalu buat file `config.toml` di dalamnya.

        Tempelkan konten berikut ke `C:\Users\<username>\.cargo\config.toml`:

        ```toml
        [source.crates-io]
        replace-with = 'rsproxy-sparse'

        [source.rsproxy]
        registry = "https://rsproxy.cn/crates.io-index"

        [source.rsproxy-sparse]
        registry = "sparse+https://rsproxy.cn/index/"

        [registries.rsproxy]
        index = "https://rsproxy.cn/crates.io-index"

        [net]
        git-fetch-with-cli = true
        ```

    - 2.3 Unduh `rustup-init.exe` dari rustup.rs dan jalankan. Eksekusi perintah berikut di PowerShell. **Sesuai dengan `openpencil 0.8.4`**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 Instal wasm-bindgen-cli untuk pembuatan binding WASM-JS. Eksekusi perintah berikut di PowerShell. **Sesuai dengan `openpencil 0.8.4`**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. Build

1. Di root repositori, buka PowerShell dan eksekusi:

    ```powershell
    cargo build --workspace --release
    ```

    > Anda mungkin menemui error `error: failed to run custom build command for skia-bindings v0.97.2`
    
    - 1.1 Unduh manual paket dengan backend GL: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 Atur environment variable (ganti dengan path Anda yang sebenarnya):

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 Jalankan lagi langkah 3.1 untuk kompilasi ulang

2. Build Web Bundle

    - 2.1 Kompilasi library WASM. Di root repositori, buka PowerShell dan eksekusi:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 Generate file binding JS

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 Salin aset rendering CanvasKit dari `crates\op-host-web\assets\canvaskit` ke `web-bundle`:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 Struktur direktori akhir

        ```plaintext
        target\release\
        ├── openpencil-desktop.exe
        ├── op-host-web-server.exe
        ├── op.exe
        └── web-bundle\
            ├── canvaskit\
            │   ├── canvaskit.js
            │   └── canvaskit.wasm
            ├── snippets\
            ├── op_host-web.d.ts
            ├── op_host_web.js
            ├── op_host_web_bg.wasm
            └── op_host_web_bg.wasm.d.ts
        ```

## 4. Konfigurasi Environment Variable untuk Plugin dsh-openpencil

1. Jalankan PowerShell sebagai administrator dan eksekusi perintah berikut (ganti path dengan root repositori yang sebenarnya):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. Verifikasi

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # Jendela muncul sebentar lalu menutup — layanan berjalan di latar belakang dan mendengarkan port 3100
    ```

2. op-host-web-server.exe  <i>` (tidak diperlukan untuk plugin dsh) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # Anda akan melihat dua baris "openpencil-desktop --serve-web:" dan port 3100 di PowerShell
    ```

---

# Pemecahan Masalah

1. Perhatikan parameter `--features` saat kompilasi WASM

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > Gunakan `--features "web canvaskit"` bukan `--features "web"`.
    > 
    > `--features "web"` bisa kompilasi berhasil, tetapi file JS yang dihasilkan tidak mengekspor fungsi `mount_ck`.
    > 
    >

2. Server memetakan seluruh direktori `web-bundle` ke rute HTTP `/pkg/`

    > Path disk `web-bundle/op_host_web.js` → URL browser `/pkg/op_host_web.js`
    > 
    > Jika halaman melaporkan `/pkg/op_host_web.js` tidak ditemukan, file tersebut tidak ada di `web-bundle/`.

3. Dokumen ini ditulis untuk versi `openpencil 0.8.4`. Versi toolchain rustup `1.94`、wasm-bindgen-cli `0.2.117` dan skia-bindings `0.97.2` selaras dengan versi ini — versi lain mungkin berbeda.

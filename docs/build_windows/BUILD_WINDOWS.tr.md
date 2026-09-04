
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · **Türkçe** · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# OpenPencil Windows Derleme Kılavuzu

## 1. Kaynak kodunu indirme

1. openpencil deposunun kaynak kodunu indirin

2. openpencil, `vendor` dizininde 3 bağımlılık alt modülü (jian, casement, agent) içerir ve bunları ayrı ayrı indirmeniz gerekir

    - 2.1 jian (UI çerçevesi): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → `vendor\jian` dizinine çıkarın

    - 2.2 casement (pencere sarmalayıcı): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → `vendor\casement` dizinine çıkarın

    - 2.3 agent (Agent çalışma zamanı): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → `vendor\agent` dizinine çıkarın

3. Çıkardıktan sonra aşağıdaki yolların mevcut olduğundan emin olun:

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. Ortam hazırlığı — *Windows 10 x64 (MSVC araç zinciri)*

1. Visual Studio'yu **"C++ ile Masaüstü Geliştirme"** iş yükünü seçerek yükleyin

2. Rust araç zincirini `stable-x86_64-pc-windows-msvc` olarak yükleyin

    - 2.1 Rustup aynasını yapılandırın (indirmeleri hızlandırmak için)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Cargo bağımlılık aynasını yapılandırın. Kullanıcı dizininizi (`C:\Users\<username>\`) açın, `.cargo` klasörünü bulun veya oluşturun ve içine `config.toml` dosyası oluşturun.

        `C:\Users\<username>\.cargo\config.toml` dosyasına aşağıdaki içeriği yapıştırın:

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

    - 2.3 rustup.rs adresinden `rustup-init.exe` dosyasını indirin ve çalıştırın. PowerShell'de aşağıdaki komutları çalıştırın. **`openpencil 0.8.4` sürümüne karşılık gelir**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 WASM-JS bağımlılık oluşturma için wasm-bindgen-cli'yi yükleyin. PowerShell'de aşağıdaki komutu çalıştırın. **`openpencil 0.8.4` sürümüne karşılık gelir**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. Derleme

1. Depo kök dizininde PowerShell'i açın ve çalıştırın:

    ```powershell
    cargo build --workspace --release
    ```

    > `error: failed to run custom build command for skia-bindings v0.97.2` hatası alabilirsiniz
    
    - 1.1 GL arka uçlu paketi manuel olarak indirin: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 Ortam değişkenini ayarlayın (gerçek yolunuzla değiştirin):

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 Yeniden derlemek için 3.1 adımını tekrar çalıştırın

2. Web Bundle derlemesi

    - 2.1 Kütüphaneyi derleyin. Depo kök dizininde PowerShell'i açın ve çalıştırın:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 JS bağımlılık dosyalarını oluşturun

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 CanvasKit işleme varlıklarını `crates\op-host-web\assets\canvaskit` dizininden `web-bundle` dizinine kopyalayın:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 Son dizin yapısı

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

## 4. dsh-openpencil eklentisi için ortam değişkenlerini yapılandırma

1. Yönetici olarak PowerShell'i çalıştırın ve aşağıdaki komutları çalıştırın (yolu gerçek depo kök dizinyle değiştirin):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. Doğrulama

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # Pencere kısa süre görünür ve kapanır — arka planda 3100 portunu dinleyen servis çalışır
    ```

2. op-host-web-server.exe  <i>` (dsh eklentisi için gerekli değildir) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # PowerShell'de iki "openpencil-desktop --serve-web:" satırı ve 3100 portunu göreceksiniz
    ```

---

# Sorun giderme

1. WASM derlerken `--features` parametresine dikkat edin

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > `--features "web"` yerine `--features "web canvaskit"` kullanın.
    > 
    > `--features "web"` ile derleme başarılı olur, ancak oluşturulan JS dosyası `mount_ck` fonksiyonunu dışa aktarmaz.
    > 
    >

2. Sunucu, tüm `web-bundle` dizinini HTTP `/pkg/` rotasına eşler

    > Disk yolu `web-bundle/op_host_web.js` → Tarayıcı URL'si `/pkg/op_host_web.js`
    > 
    > Sayfa `/pkg/op_host_web.js` bulunamadığını belirtiyorsa, `web-bundle/` dizininde dosya eksiktir.

3. Bu belge `openpencil 0.8.4` sürümü için yazılmıştır. Rustup araç zinciri sürümü `1.94`、wasm-bindgen-cli sürümü `0.2.117` ve skia-bindings sürümü `0.97.2` bu sürüme görelidir — diğer sürümler farklı olabilir.

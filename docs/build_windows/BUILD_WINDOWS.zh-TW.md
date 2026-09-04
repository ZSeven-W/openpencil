
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · **繁體中文** · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# OpenPencil Windows 編譯建置文件

## 一、下載原始碼

1. 下載 openpencil 儲存庫原始碼

2. openpencil 在 `vendor` 目錄下有 3 個依賴子模組（jian、casement、agent），需要個別下載

    - 2.1 jian（UI 框架）：[https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → 解壓到 `vendor\jian`

    - 2.2 casement（視窗封裝）：[https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → 解壓到 `vendor\casement`

    - 2.3 agent（Agent 執行時）：[https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → 解壓到 `vendor\agent`

3. 解壓後確認路徑存在：

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 二、環境準備 — *Windows 10 x64（MSVC 工具鏈）*

1. 安裝 Visual Studio，安裝時勾選**「使用 C++ 的桌面開發」**

2. 安裝 Rust 工具鏈 `stable-x86_64-pc-windows-msvc`

    - 2.1 設定 rustup 工具鏈映像（解決下載速度慢的問題）

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 設定 Cargo 依賴映像。打開使用者目錄（`C:\Users\<username>\`），找到 `.cargo` 資料夾，沒有就新建一個，在裡面新建 `config.toml` 檔案

        在 `C:\Users\<username>\.cargo\config.toml` 貼上以下內容：

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

    - 2.3 從 rustup.rs 下載 `rustup-init.exe`，雙擊安裝。PowerShell 執行以下命令，**對應 `openpencil 0.8.4` 版本**：

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 安裝 wasm-bindgen-cli，用於 WASM 與 JS 的綁定生成。PowerShell 執行以下命令，**對應 `openpencil 0.8.4` 版本**：

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 三、編譯

1. 在儲存庫根目錄打開 PowerShell，執行：

    ```powershell
    cargo build --workspace --release
    ```

    > 可能會遇到 `error: failed to run custom build command for skia-bindings v0.97.2` 錯誤
    
    - 1.1 手動下載帶 GL 後端的套件：[skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 設定環境變數（替換為實際路徑）：

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 重新執行步驟 3.1 進行編譯

2. 建置 Web Bundle

    - 2.1 編譯 WASM 函式庫。在儲存庫根目錄打開 PowerShell，執行：

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 生成 JS 綁定檔案

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 複製 CanvasKit 渲染資源。從 `crates\op-host-web\assets\canvaskit` 複製到 `web-bundle`：

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 最終目錄結構

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

## 四、設定 dsh-openpencil 外掛所需的環境變數

1. 以系統管理員身分執行 PowerShell，執行以下命令（將路徑替換為實際的儲存庫根目錄）：

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 五、驗證

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # 視窗會一閃而過，背景執行服務並監聽 3100 埠
    ```

2. op-host-web-server.exe  <i>`（dsh 外掛可以不用這個檔案）`</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # 會看到兩段「openpencil-desktop --serve-web:」，在 PowerShell 中監聽 3100 埠
    ```

---

# 疑難排解

1. WASM 編譯時注意 `--features` 參數

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > 請使用 `--features "web canvaskit"` 而非 `--features "web"`。
    > 
    > `--features "web"` 雖然能編譯成功，但生成的 JS 檔案不會匯出 `mount_ck` 函數。
    > 
    >

2. 伺服器已將整個 `web-bundle` 目錄對應到 HTTP 的 `/pkg/` 路由

    > 磁碟路徑 `web-bundle/op_host_web.js` → 瀏覽器 URL `/pkg/op_host_web.js`
    > 
    > 頁面顯示找不到 `/pkg/op_host_web.js` 時，表示 `web-bundle/` 中缺少該檔案。

3. 本文檔以 `openpencil 0.8.4` 版本編寫，rustup 工具鏈版本 `1.94`、wasm-bindgen-cli 版本 `0.2.117`、skia-bindings 版本 `0.97.2` 均與該版本對齊，其他版本可能存在差異

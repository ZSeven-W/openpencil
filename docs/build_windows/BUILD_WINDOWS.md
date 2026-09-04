
> [**English**](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# OpenPencil Windows Build Guide

## 1. Download Source Code

1. **Option A (recommended):** Clone with submodules automatically:

    ```bash
    git clone --recurse-submodules https://github.com/ZSeven-W/openpencil.git
    ```

    > If already cloned without submodules, run from the repository root:
    >
    > ```bash
    > git submodule update --init --recursive
    > ```

    **Or Option B:** Manually download and extract the following 3 repositories:

    - jian (UI framework): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → extract to `vendor\jian`

    - casement (window wrapper): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → extract to `vendor\casement`

    - agent (Agent runtime): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → extract to `vendor\agent`

2. Confirm the 3 submodules under `vendor` are ready:

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. Environment Setup — *Windows 10 x64 (MSVC Toolchain)*

1. Install Visual Studio with the **"Desktop development with C++"** workload checked

2. Install the Rust toolchain `stable-x86_64-pc-windows-msvc`:

    - 2.1 Download `rustup-init.exe` from rustup.rs and run it. Execute the following in PowerShell. **Corresponds to `openpencil 0.8.4`**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.2 Install wasm-bindgen-cli for WASM-to-JS binding generation. Execute the following in PowerShell. **Corresponds to `openpencil 0.8.4`**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. Build

1. In the repository root, open PowerShell and run:

    ```powershell
    cargo build --workspace --release
    ```

    > You may encounter `error: failed to run custom build command for skia-bindings v0.97.2`
    
    - 1.1 Manually download the GL-backend package: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 Set the environment variable (replace with your actual path):
    
         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 Re-run step 3.1 to rebuild

2. Build the Web Bundle

    - 2.1 Compile the WASM library. In the repository root, open PowerShell and run:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 Generate JS binding files

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 Copy CanvasKit rendering assets from `crates\op-host-web\assets\canvaskit` to `web-bundle`:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 Final directory structure (`target\release\`)

        ```plaintext
        release\
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

## 4. Configure Environment Variables for the dsh-openpencil Plugin

1. Run PowerShell as Administrator and execute the following (replace the path with your actual repository root):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. Verification

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # The window flashes and closes — the service runs in the background on port 3100
    ```

2. op-host-web-server.exe  <i>` (not required for the dsh plugin) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # You will see two "openpencil-desktop --serve-web:" lines and port 3100 in PowerShell
    ```

---

# Notes

1. `--features` parameter

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > Use `--features "web canvaskit"`, not `--features "web"`.
    > 
    > `--features "web"` compiles successfully, but the generated JS file will not export the `mount_ck` function.

2. The server maps the entire `web-bundle` directory to the HTTP `/pkg/` route

    > Disk path `web-bundle/op_host_web.js` → Browser URL `/pkg/op_host_web.js`
    > 
    > If the page reports `/pkg/op_host_web.js` not found, the file is missing from `web-bundle/`.

3. This document is written for `openpencil 0.8.4`. The rustup toolchain version `1.94`, wasm-bindgen-cli version `0.2.117`, and skia-bindings version `0.97.2` are aligned with this version — other versions may differ.

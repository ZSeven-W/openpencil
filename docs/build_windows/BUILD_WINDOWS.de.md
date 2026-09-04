
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · **Deutsch** · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# OpenPencil Windows Build-Anleitung

## 1. Quellcode herunterladen

1. Laden Sie den Quellcode des openpencil-Repositories herunter

2. openpencil enthält 3 Abhängigkeits-Submodules im `vendor`-Verzeichnis (jian, casement, agent), die separat heruntergeladen werden müssen

    - 2.1 jian (UI-Framework): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → in `vendor\jian` entpacken

    - 2.2 casement (Fenster-Wrapper): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → in `vendor\casement` entpacken

    - 2.3 agent (Agent-Laufzeitumgebung): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → in `vendor\agent` entpacken

3. Überprüfen Sie nach dem Entpacken, dass die folgenden Pfade vorhanden sind:

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. Umgebung vorbereiten — *Windows 10 x64 (MSVC-Toolchain)*

1. Installieren Sie Visual Studio mit der Workload **„Desktopentwicklung mit C++"**

2. Installieren Sie die Rust-Toolchain `stable-x86_64-pc-windows-msvc`

    - 2.1 Konfigurieren Sie den rustup-Spiegel (zur Beschleunigung der Downloads)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Konfigurieren Sie den Cargo-Abhängigkeitsspiegel. Öffnen Sie Ihr Benutzerverzeichnis (`C:\Users\<username>\`), finden oder erstellen Sie den Ordner `.cargo` und erstellen Sie darin eine Datei `config.toml`.

        Fügen Sie den folgenden Inhalt in `C:\Users\<username>\.cargo\config.toml` ein:

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

    - 2.3 Laden Sie `rustup-init.exe` von rustup.rs herunter und führen Sie es aus. Führen Sie die folgenden Befehle in PowerShell aus. **Entspricht `openpencil 0.8.4`**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 Installieren Sie wasm-bindgen-cli für die WASM-JS-Bindungserstellung. Führen Sie den folgenden Befehl in PowerShell aus. **Entspricht `openpencil 0.8.4`**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. Kompilierung

1. Öffnen Sie PowerShell im Repository-Root und führen Sie aus:

    ```powershell
    cargo build --workspace --release
    ```

    > Sie können den Fehler `error: failed to run custom build command for skia-bindings v0.97.2` erhalten
    
    - 1.1 Laden Sie manuell das Paket mit GL-Backend herunter: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 Setzen Sie die Umgebungsvariable (ersetzen Sie durch Ihren tatsächlichen Pfad):

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 Führen Sie Schritt 3.1 erneut aus, um neu zu kompilieren

2. Web-Bundle erstellen

    - 2.1 WASM-Bibliothek kompilieren. Öffnen Sie PowerShell im Repository-Root und führen Sie aus:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 JS-Bindungsdateien generieren

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 Kopieren Sie die CanvasKit-Rendering-Ressourcen von `crates\op-host-web\assets\canvaskit` nach `web-bundle`:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 Endliche Verzeichnisstruktur

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

## 4. Umgebungsvariablen für das dsh-openpencil-Plugin konfigurieren

1. Führen Sie PowerShell als Administrator aus und führen Sie die folgenden Befehle aus (ersetzen Sie den Pfad durch den tatsächlichen Repository-Root):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. Überprüfung

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # Das Fenster blinkt kurz auf und schließt sich — der Dienst läuft im Hintergrund auf Port 3100
    ```

2. op-host-web-server.exe  <i>` (nicht erforderlich für das dsh-Plugin) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # Sie sehen zwei Zeilen „openpencil-desktop --serve-web:" und Port 3100 in PowerShell
    ```

---

# Fehlerbehebung

1. Achten Sie auf den `--features`-Parameter beim Kompilieren von WASM

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > Verwenden Sie `--features "web canvaskit"` und nicht `--features "web"`.
    > 
    > `--features "web"` kompiliert erfolgreich, aber die generierte JS-Datei exportiert die Funktion `mount_ck` nicht.
    > 
    >

2. Der Server bildet das gesamte `web-bundle`-Verzeichnis auf die HTTP-Route `/pkg/` ab

    > Festplattenpfad `web-bundle/op_host_web.js` → Browser-URL `/pkg/op_host_web.js`
    > 
    > Wenn die Seite meldet, dass `/pkg/op_host_web.js` nicht gefunden wurde, fehlt die Datei in `web-bundle/`.

3. Dieses Dokument wurde für `openpencil 0.8.4` verfasst. Die Rustup-Toolchain-Version `1.94`, wasm-bindgen-cli-Version `0.2.117` und skia-bindings-Version `0.97.2` sind auf diese Version abgestimmt — andere Versionen können abweichen.

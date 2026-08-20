
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · **Español** · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# Guía de compilación de OpenPencil para Windows

## 1. Descargar el código fuente

1. Descargue el código fuente del repositorio openpencil

2. openpencil tiene 3 submódulos de dependencias en el directorio `vendor` (jian, casement, agent) que deben descargarse por separado

    - 2.1 jian (framework UI): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → extraer en `vendor\jian`

    - 2.2 casement (wrapper de ventana): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → extraer en `vendor\casement`

    - 2.3 agent (runtime de Agent): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → extraer en `vendor\agent`

3. Después de extraer, verifique que existan las siguientes rutas:

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. Preparación del entorno — *Windows 10 x64 (cadena de herramientas MSVC)*

1. Instale Visual Studio seleccionando **"Desarrollo de escritorio con C++"**

2. Instale la cadena de herramientas Rust `stable-x86_64-pc-windows-msvc`

    - 2.1 Configure el espejo de rustup (para acelerar las descargas)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Configure el espejo de dependencias de Cargo. Abra su directorio de usuario (`C:\Users\<username>\`), encuentre o cree la carpeta `.cargo` y cree un archivo `config.toml` dentro.

        Pegue el siguiente contenido en `C:\Users\<username>\.cargo\config.toml`:

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

    - 2.3 Descargue `rustup-init.exe` desde rustup.rs y ejecútelo. En PowerShell ejecute los siguientes comandos. **Corresponde a `openpencil 0.8.4`**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 Instale wasm-bindgen-cli para la generación de enlaces WASM-JS. En PowerShell ejecute el siguiente comando. **Corresponde a `openpencil 0.8.4`**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. Compilación

1. En la raíz del repositorio, abra PowerShell y ejecute:

    ```powershell
    cargo build --workspace --release
    ```

    > Puede encontrar el error `error: failed to run custom build command for skia-bindings v0.97.2`
    
    - 1.1 Descargue manualmente el paquete con backend GL: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 Establezca la variable de entorno (reemplace con su ruta real):

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 Vuelva a ejecutar el paso 3.1 para recompilar

2. Construcción del bundle Web

    - 2.1 Compilación de la biblioteca WASM. En la raíz del repositorio, abra PowerShell y ejecute:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 Generación de archivos de enlaces JS

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 Copie los recursos de renderizado CanvasKit desde `crates\op-host-web\assets\canvaskit` a `web-bundle`:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 Estructura de directorios final

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

## 4. Configurar variables de entorno para el plugin dsh-openpencil

1. Ejecute PowerShell como administrador y ejecute los siguientes comandos (reemplace la ruta con la raíz real del repositorio):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. Verificación

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # La ventana aparece brevemente y se cierra — el servicio escucha en el puerto 3100 en segundo plano
    ```

2. op-host-web-server.exe  <i>` (no requerido para el plugin dsh) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # Verá dos líneas de «openpencil-desktop --serve-web:» y el puerto 3100 en PowerShell
    ```

---

# Solución de problemas

1. Preste atención al parámetro `--features` al compilar WASM

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > Use `--features "web canvaskit"` en lugar de `--features "web"`.
    > 
    > `--features "web"` compila correctamente, pero el archivo JS generado no exporta la función `mount_ck`.
    > 
    >

2. El servidor mapea todo el directorio `web-bundle` a la ruta HTTP `/pkg/`

    > Ruta en disco `web-bundle/op_host_web.js` → URL del navegador `/pkg/op_host_web.js`
    > 
    > Si la página indica que no se encuentra `/pkg/op_host_web.js`, falta el archivo en `web-bundle/`.

3. Este documento está escrito para la versión `openpencil 0.8.4`. La versión de la cadena de herramientas rustup `1.94`, wasm-bindgen-cli `0.2.117` y skia-bindings `0.97.2` están alineadas con esta versión — otras versiones pueden diferir.


> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · **Русский** · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# Руководство по сборке OpenPencil для Windows

## 1. Скачивание исходного кода

1. Скачайте исходный код репозитория openpencil

2. В каталоге `vendor` openpencil находятся 3 подмодуля зависимостей (jian, casement, agent), которые необходимо скачать отдельно

    - 2.1 jian (UI-фреймворк): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → распаковать в `vendor\jian`

    - 2.2 casement (обёртка окон): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → распаковать в `vendor\casement`

    - 2.3 agent (среда выполнения Agent): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → распаковать в `vendor\agent`

3. После распаковки убедитесь, что следующие пути существуют:

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. Подготовка окружения — *Windows 10 x64 (цепочка инструментов MSVC)*

1. Установите Visual Studio, выбрав рабочую нагрузку **«Разработка для классических приложений на C++»**

2. Установите цепочку инструментов Rust `stable-x86_64-pc-windows-msvc`

    - 2.1 Настройте зеркало rustup (для ускорения загрузки)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Настройте зеркало зависимостей Cargo. Откройте каталог пользователя (`C:\Users\<username>\`), найдите или создайте папку `.cargo` и создайте файл `config.toml` внутри.

        Вставьте следующее содержимое в `C:\Users\<username>\.cargo\config.toml`:

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

    - 2.3 Скачайте `rustup-init.exe` с rustup.rs и запустите его. Выполните следующие команды в PowerShell. **Соответствует `openpencil 0.8.4`**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 Установите wasm-bindgen-cli для генерации привязок WASM-JS. Выполните следующую команду в PowerShell. **Соответствует `openpencil 0.8.4`**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. Сборка

1. В корне репозитория откройте PowerShell и выполните:

    ```powershell
    cargo build --workspace --release
    ```

    > Может возникнуть ошибка `error: failed to run custom build command for skia-bindings v0.97.2`
    
    - 1.1 Скачайте вручную пакет с GL-бэкендом: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 Установите переменную окружения (замените на реальный путь):

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 Повторите шаг 3.1 для повторной сборки

2. Сборка веб-бандла

    - 2.1 Компиляция WASM-библиотеки. В корне репозитория откройте PowerShell и выполните:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 Генерация файлов привязок JS

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 Скопируйте ресурсы рендеринга CanvasKit из `crates\op-host-web\assets\canvaskit` в `web-bundle`:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 Итоговая структура каталогов

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

## 4. Настройка переменных окружения для плагина dsh-openpencil

1. Запустите PowerShell от имени администратора и выполните следующие команды (замените путь на реальный корень репозитория):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. Проверка

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # Окно мелькнет и закроется — служба работает в фоновом режиме и слушает порт 3100
    ```

2. op-host-web-server.exe  <i>` (не требуется для плагина dsh) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # Вы увидите две строки «openpencil-desktop --serve-web:» и порт 3100 в PowerShell
    ```

---

# Устранение неполадок

1. Обратите внимание на параметр `--features` при компиляции WASM

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > Используйте `--features "web canvaskit"`, а не `--features "web"`.
    > 
    > `--features "web"` компилируется успешно, но сгенерированный JS-файл не экспортирует функцию `mount_ck`.
    > 
    >

2. Сервер отображает весь каталог `web-bundle` на маршрут HTTP `/pkg/`

    > Путь на диске `web-bundle/op_host_web.js` → URL в браузере `/pkg/op_host_web.js`
    > 
    > Если страница сообщает, что `/pkg/op_host_web.js` не найден, файл отсутствует в `web-bundle/`.

3. Этот документ написан для версии `openpencil 0.8.4`. Версия цепочки инструментов rustup `1.94`, wasm-bindgen-cli `0.2.117` и skia-bindings `0.97.2` соответствуют этой версии — другие версии могут отличаться.

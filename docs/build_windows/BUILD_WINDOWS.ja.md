
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · **日本語** · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# OpenPencil Windows ビルドガイド

## 1. ソースコードのダウンロード

1. openpencil リポジトリのソースコードをダウンロードします

2. openpencil には `vendor` ディレクトリに 3 つの依存サブモジュール（jian、casement、agent）があり、それぞれ個別にダウンロードする必要があります

    - 2.1 jian（UI フレームワーク）：[https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → `vendor\jian` に展開

    - 2.2 casement（ウィンドウラッパー）：[https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → `vendor\casement` に展開

    - 2.3 agent（Agent ランタイム）：[https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → `vendor\agent` に展開

3. 展開後、以下のパスが存在することを確認：

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. 環境準備 — *Windows 10 x64（MSVC ツールチェーン）*

1. Visual Studio をインストールし、**「C++によるデスクトップ開発」** ワークロードを選択

2. Rust ツールチェーン `stable-x86_64-pc-windows-msvc` をインストール

    - 2.1 rustup ミラーの設定（ダウンロード速度の向上）

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Cargo 依存関係ミラーの設定。ユーザーディレクトリ（`C:\Users\<username>\`）を開き、`.cargo` フォルダがなければ作成し、その中に `config.toml` ファイルを作成

        `C:\Users\<username>\.cargo\config.toml` に以下の内容を貼り付け：

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

    - 2.3 rustup.rs から `rustup-init.exe` をダウンロードし、ダブルクリックしてインストール。PowerShell で以下を実行。**`openpencil 0.8.4` に対応**：

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 wasm-bindgen-cli をインストール（WASM と JS のバインド生成用）。PowerShell で以下を実行。**`openpencil 0.8.4` に対応**：

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. ビルド

1. リポジトリルートで PowerShell を開き、以下を実行：

    ```powershell
    cargo build --workspace --release
    ```

    > `error: failed to run custom build command for skia-bindings v0.97.2` エラーが発生する場合があります
    
    - 1.1 GL バックエンド付きパッケージを手動ダウンロード：[skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 環境変数を設定（実際のパスに置き換え）：

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 ステップ 3.1 を再実行してリビルド

2. Web バンドルのビルド

    - 2.1 WASM ライブラリのコンパイル。リポジトリルートで PowerShell を開き、以下を実行：

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 JS バインドファイルの生成

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 CanvasKit レンダリングアセットを `crates\op-host-web\assets\canvaskit` から `web-bundle` にコピー：

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 最終ディレクトリ構造

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

## 4. dsh-openpencil プラグインの環境変数設定

1. 管理者として PowerShell を実行し、以下を実行（パスを実際のリポジトリルートに置き換え）：

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. 検証

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # ウィンドウが一瞬表示されて閉じます — バックグラウンドで3100ポートをリッスンします
    ```

2. op-host-web-server.exe  <i>`（dsh プラグインでは不要）`</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # PowerShell で「openpencil-desktop --serve-web:」が2行表示され、3100ポートをリッスンします
    ```

---

# トラブルシューティング

1. WASM ビルド時の `--features` パラメータに注意

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > `--features "web"` ではなく `--features "web canvaskit"` を使用してください。
    > 
    > `--features "web"` でもコンパイルは成功しますが、生成された JS ファイルに `mount_ck` 関数がエクスポートされません。
    > 
    >

2. サーバーは `web-bundle` ディレクトリ全体を HTTP の `/pkg/` ルートにマッピングします

    > ディスクパス `web-bundle/op_host_web.js` → ブラウザ URL `/pkg/op_host_web.js`
    > 
    > ページに `/pkg/op_host_web.js` が見つからないと表示される場合は、`web-bundle/` にファイルがありません。

3. 本ドキュメントは `openpencil 0.8.4` バージョンで作成されています。rustup ツールチェーンバージョン `1.94`、wasm-bindgen-cli バージョン `0.2.117`、skia-bindings バージョン `0.97.2` はこのバージョンに合わせたものです。他のバージョンでは異なる場合があります。

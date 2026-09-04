
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · **Português** · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# Guia de compilação do OpenPencil para Windows

## 1. Baixar o código-fonte

1. Baixe o código-fonte do repositório openpencil

2. openpencil possui 3 submódulos de dependências no diretório `vendor` (jian, casement, agent) que precisam ser baixados separadamente

    - 2.1 jian (framework UI): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → extrair para `vendor\jian`

    - 2.2 casement (wrapper de janela): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → extrair para `vendor\casement`

    - 2.3 agent (runtime do Agent): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → extrair para `vendor\agent`

3. Após a extração, verifique se os seguintes caminhos existem:

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. Preparação do ambiente — *Windows 10 x64 (toolchain MSVC)*

1. Instale o Visual Studio marcando **"Desenvolvimento para Desktop com C++"**

2. Instale a toolchain Rust `stable-x86_64-pc-windows-msvc`

    - 2.1 Configure o espelho do rustup (para acelerar downloads)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Configure o espelho de dependências do Cargo. Abra seu diretório de usuário (`C:\Users\<username>\`), encontre ou crie a pasta `.cargo` e crie um arquivo `config.toml` dentro dela.

        Cole o seguinte conteúdo em `C:\Users\<username>\.cargo\config.toml`:

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

    - 2.3 Baixe `rustup-init.exe` no rustup.rs e execute-o. Execute os seguintes comandos no PowerShell. **Corresponde ao `openpencil 0.8.4`**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 Instale o wasm-bindgen-cli para geração de bindings WASM-JS. Execute o seguinte comando no PowerShell. **Corresponde ao `openpencil 0.8.4`**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. Compilação

1. Na raiz do repositório, abra o PowerShell e execute:

    ```powershell
    cargo build --workspace --release
    ```

    > Você pode encontrar o erro `error: failed to run custom build command for skia-bindings v0.97.2`
    
    - 1.1 Baixe manualmente o pacote com backend GL: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 Defina a variável de ambiente (substitua pelo seu caminho real):

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 Execute novamente o passo 3.1 para recompilar

2. Construção do bundle Web

    - 2.1 Compilar a biblioteca WASM. Na raiz do repositório, abra o PowerShell e execute:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 Gerar arquivos de bindings JS

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 Copie os recursos de renderização CanvasKit de `crates\op-host-web\assets\canvaskit` para `web-bundle`:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 Estrutura final de diretórios

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

## 4. Configurar variáveis de ambiente para o plugin dsh-openpencil

1. Execute o PowerShell como administrador e execute os seguintes comandos (substitua o caminho pela raiz real do repositório):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. Verificação

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # A janela aparece brevemente e fecha — o serviço escuta na porta 3100 em segundo plano
    ```

2. op-host-web-server.exe  <i>` (não necessário para o plugin dsh) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # Você verá duas linhas de "openpencil-desktop --serve-web:" e a porta 3100 no PowerShell
    ```

---

# Solução de problemas

1. Preste atenção ao parâmetro `--features` ao compilar WASM

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > Use `--features "web canvaskit"` em vez de `--features "web"`.
    > 
    > `--features "web"` compila com sucesso, mas o arquivo JS gerado não exporta a função `mount_ck`.
    > 
    >

2. O servidor mapeia todo o diretório `web-bundle` para a rota HTTP `/pkg/`

    > Caminho no disco `web-bundle/op_host_web.js` → URL do navegador `/pkg/op_host_web.js`
    > 
    > Se a página informar que `/pkg/op_host_web.js` não foi encontrado, o arquivo está faltando em `web-bundle/`.

3. Este documento foi escrito para a versão `openpencil 0.8.4`. A versão da toolchain rustup `1.94`, wasm-bindgen-cli `0.2.117` e skia-bindings `0.97.2` estão alinhadas com esta versão — outras versões podem diferir.

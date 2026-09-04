
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · **Français** · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# Guide de compilation OpenPencil pour Windows

## 1. Téléchargement du code source

1. Téléchargez le code source du dépôt openpencil

2. openpencil possède 3 sous-modules de dépendances dans le répertoire `vendor` (jian, casement, agent) qui doivent être téléchargés séparément

    - 2.1 jian (framework UI) : [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → extraire dans `vendor\jian`

    - 2.2 casement (wrapper de fenêtre) : [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → extraire dans `vendor\casement`

    - 2.3 agent (runtime Agent) : [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → extraire dans `vendor\agent`

3. Après extraction, vérifiez que les chemins suivants existent :

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. Préparation de l'environnement — *Windows 10 x64 (chaîne d'outils MSVC)*

1. Installez Visual Studio en cochant **« Développement de bureau avec C++ »**

2. Installez la chaîne d'outils Rust `stable-x86_64-pc-windows-msvc`

    - 2.1 Configurez le miroir rustup (pour accélérer les téléchargements)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Configurez le miroir des dépendances Cargo. Ouvrez votre répertoire utilisateur (`C:\Users\<username>\`), trouvez ou créez le dossier `.cargo`, puis créez un fichier `config.toml` à l'intérieur.

        Collez le contenu suivant dans `C:\Users\<username>\.cargo\config.toml` :

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

    - 2.3 Téléchargez `rustup-init.exe` depuis rustup.rs et exécutez-le. Dans PowerShell, lancez les commandes suivantes. **Correspond à `openpencil 0.8.4`** :

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 Installez wasm-bindgen-cli pour la génération des liaisons WASM-JS. Dans PowerShell, lancez la commande suivante. **Correspond à `openpencil 0.8.4`** :

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. Compilation

1. À la racine du dépôt, ouvrez PowerShell et exécutez :

    ```powershell
    cargo build --workspace --release
    ```

    > Vous pouvez rencontrer l'erreur `error: failed to run custom build command for skia-bindings v0.97.2`
    
    - 1.1 Téléchargez manuellement le paquet avec backend GL : [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 Définissez la variable d'environnement (remplacez par votre chemin réel) :

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 Ré-exécutez l'étape 3.1 pour recompiler

2. Construction du bundle Web

    - 2.1 Compilation de la bibliothèque WASM. À la racine du dépôt, ouvrez PowerShell et exécutez :

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 Génération des fichiers de liaisons JS

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 Copiez les ressources de rendu CanvasKit depuis `crates\op-host-web\assets\canvaskit` vers `web-bundle` :

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 Structure de répertoire finale

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

## 4. Configuration des variables d'environnement pour le plugin dsh-openpencil

1. Exécutez PowerShell en tant qu'administrateur et lancez les commandes suivantes (remplacez le chemin par la racine réelle du dépôt) :

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. Vérification

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # La fenêtre apparaît brièvement puis se ferme — le service écoute sur le port 3100 en arrière-plan
    ```

2. op-host-web-server.exe  <i>` (non requis pour le plugin dsh) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # Vous verrez deux lignes « openpencil-desktop --serve-web: » et le port 3100 dans PowerShell
    ```

---

# Dépannage

1. Attention au paramètre `--features` lors de la compilation WASM

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > Utilisez `--features "web canvaskit"` et non `--features "web"`.
    > 
    > `--features "web"` compile avec succès, mais le fichier JS généré n'exporte pas la fonction `mount_ck`.
    > 
    >

2. Le serveur mappe l'intégralité du répertoire `web-bundle` sur la route HTTP `/pkg/`

    > Chemin disque `web-bundle/op_host_web.js` → URL navigateur `/pkg/op_host_web.js`
    > 
    > Si la page indique que `/pkg/op_host_web.js` est introuvable, le fichier est manquant dans `web-bundle/`.

3. Ce document est rédigé pour la version `openpencil 0.8.4`. La version de la chaîne d'outils rustup `1.94`, de wasm-bindgen-cli `0.2.117` et de skia-bindings `0.97.2` sont alignées sur cette version — d'autres versions peuvent différer.

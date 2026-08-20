
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · **한국어** · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# OpenPencil Windows 빌드 가이드

## 1. 소스 코드 다운로드

1. openpencil 저장소 소스 코드를 다운로드합니다

2. openpencil에는 `vendor` 디렉토리에 3개의 의존성 서브모듈(jian, casement, agent)이 있어 각각 별도로 다운로드해야 합니다

    - 2.1 jian (UI 프레임워크): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → `vendor\jian`에 압축 해제

    - 2.2 casement (윈도우 래퍼): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → `vendor\casement`에 압축 해제

    - 2.3 agent (Agent 런타임): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → `vendor\agent`에 압축 해제

3. 압축 해제 후 다음 경로가 존재하는지 확인：

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. 환경 준비 — *Windows 10 x64 (MSVC 툴체인)*

1. Visual Studio를 설치할 때 **"C++를 사용한 데스크톱 개발"** 워크로드를 선택

2. Rust 툴체인 `stable-x86_64-pc-windows-msvc` 설치

    - 2.1 rustup 미러 설정 (다운로드 속도 향상)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Cargo 의존성 미러 설정. 사용자 디렉토리(`C:\Users\<username>\`)를 열고 `.cargo` 폴더가 없으면 만들고 그 안에 `config.toml` 파일 생성

        `C:\Users\<username>\.cargo\config.toml`에 다음 내용을 붙여넣기:

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

    - 2.3 rustup.rs에서 `rustup-init.exe`를 다운로드하고 더블클릭하여 설치. PowerShell에서 다음을 실행. **`openpencil 0.8.4` 버전 대응**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 wasm-bindgen-cli 설치 (WASM와 JS 바인딩 생성용). PowerShell에서 다음을 실행. **`openpencil 0.8.4` 버전 대응**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. 빌드

1. 저장소 루트에서 PowerShell을 열고 실행:

    ```powershell
    cargo build --workspace --release
    ```

    > `error: failed to run custom build command for skia-bindings v0.97.2` 오류가 발생할 수 있습니다
    
    - 1.1 GL 백엔드 패키지를 수동 다운로드: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 환경 변수 설정 (실제 경로로 대체):

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 단계 3.1을 다시 실행하여 재빌드

2. Web 번들 빌드

    - 2.1 WASM 라이브러리 컴파일. 저장소 루트에서 PowerShell을 열고 실행:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 JS 바인딩 파일 생성

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 CanvasKit 렌더링 에셋을 `crates\op-host-web\assets\canvaskit`에서 `web-bundle`로 복사:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 최종 디렉토리 구조

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

## 4. dsh-openpencil 플러그인 환경 변수 설정

1. 관리자 권한으로 PowerShell을 실행하고 다음을 실행 (경로를 실제 저장소 루트로 대체):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. 검증

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # 창이 순간 나타났다 사라집니다 — 백그라운드에서 3100 포트를 수신합니다
    ```

2. op-host-web-server.exe  <i>`(dsh 플러그인에서는 필요하지 않음)`</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # PowerShell에서 "openpencil-desktop --serve-web:"가 두 번 표시되고 3100 포트를 수신합니다
    ```

---

# 문제 해결

1. WASM 빌드 시 `--features` 매개변수에 주의

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > `--features "web"`이 아닌 `--features "web canvaskit"`을 사용하세요.
    > 
    > `--features "web"`으로도 컴파일이 성공하지만, 생성된 JS 파일에 `mount_ck` 함수가 내보내지지 않습니다.
    > 
    >

2. 서버는 `web-bundle` 디렉토리 전체를 HTTP의 `/pkg/` 라우트에 매핑합니다

    > 디스크 경로 `web-bundle/op_host_web.js` → 브라우저 URL `/pkg/op_host_web.js`
    > 
    > 페이지에서 `/pkg/op_host_web.js`를 찾을 수 없다고 표시되면 `web-bundle/`에 파일이 없는 것입니다.

3. 이 문서는 `openpencil 0.8.4` 버전을 기준으로 작성되었습니다. rustup 툴체인 버전 `1.94`、wasm-bindgen-cli 버전 `0.2.117`、skia-bindings 버전 `0.97.2`는 이 버전에 맞춰진 것이며, 다른 버전은 다를 수 있습니다.

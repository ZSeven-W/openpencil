
> [English](./BUILD_WINDOWS.md) · **简体中文** · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# OpenPencil 编译构建部署文档

## 一、下载源码

1. **方式一（推荐）：** 使用 git 克隆并自动拉取子模块：

    ```bash
    git clone --recurse-submodules https://github.com/ZSeven-W/openpencil.git
    ```

    > 如果已经克隆过但未拉取子模块，可以在仓库根目录执行：
    >
    > ```bash
    > git submodule update --init --recursive
    > ```

    **或 方式二：** 手动下载并解压以下 3 个仓库到对应目录：

    - jian（UI 框架）：[https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → 解压到 `vendor\jian`

    - casement（窗口封装）：[https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → 解压到 `vendor\casement`

    - agent（Agent 运行时）：[https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → 解压到 `vendor\agent`

2. 确认 vendor 目录下的 3 个子模块已就绪：

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 二、环境准备，*Windows 10 x64（MSVC 工具链）*
1. 安装 Visual Studio，安装时勾选 **「使用 C\+\+ 的桌面开发」**

2. 安装 rust 工具链 `stable-x86_64-pc-windows-msvc` 

    - 2.1 配置 rustup 工具链镜像（解决安装下载慢）

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 配置 Cargo 依赖镜像，打开用户目录（`C:\Users\你的用户名\`），找到 `.cargo` 文件夹，没有就新建一个，在里面新建文件 `config.toml`
    
        `C:\Users\你的用户名\.cargo\config.toml`，粘贴以下配置并保存

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

    - 2.3 从 rustup\.rs 下载 `rustup-init.exe`，双击`rustup-init.exe`安装配置，PowerShell执行以下命令, **对应 `openpencil 0.8.4` 版本**

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 安装 wasm‑bindgen‑cli，用于处理 WASM 与 JS 的绑定生成，PowerShell 执行以下命令, **对应 `openpencil 0.8.4` 版本**

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 三、编译

1. 在仓库根目录打开 PowerShell，执行：

    ```powershell
    cargo build --workspace --release
    ```

    > 可能会遇到 `error: failed to run custom build command for skia-bindings v0.97.2` 错误
    
    -  1\.1 手动下载带 GL 后端的包：[https://github\.com/rust\-skia/skia\-binaries/releases/download/0\.97\.2/skia\-binaries\-da8fc6731fc439bc3b6a\-x86\_64\-pc\-windows\-msvc\-gl\-jpegd\-jpege\-pdf\-textlayout\.tar\.gz](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    -  1\.2 设置环境变量，把 **<i><u>刚下载的 skia-binaries 的目录</u></i>** 替换为实际目录

         ```powershell
        $env:SKIA_BINARIES_URL="file://刚下载的skia-binaries的目录\skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    -  1\.3 执行步骤 `三.1` 重新编译

2. 构建 Web Bundle

    - 2\.1 编译 WASM 库，在仓库根目录打开 PowerShell，执行：

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2\.2 生成 JS 绑定文件

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2\.3 补全 CanvasKit 渲染资源，从 `crates\op-host-web\assets\` 目录把 `canvaskit` 复制到 `web-bundle` 中

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2\.4 最终目录结构（`target\release\` 下）

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

## 四、配置 dsh‑openpencil 插件需要的环境变量

1. 以管理员身份运行 PowerShell，执行以下命令，**把 <i><u>openpencil‑main 根目录</u></i> 替换为实际仓库目录**：

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "openpencil-main根目录\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "openpencil-main根目录\target\release\openpencil-desktop.exe", "User")
    ```

## 五、验证

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    #屏幕会一闪而过，后台运行openpencil-desktop服务监听3100端口
    ```

2. op-host-web-server.exe  <i>` (dsh插件可以不用这个文件) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    #会看到两段“openpencil-desktop --serve-web: ”，在PowerShell中监听3100端口
    ```

---

# 注意

1.  --feature 参数

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > 是`--features "web canvaskit"` 而不是`--features "web"`，
    > 
    >`--features "web"`能编译成功，但生成的js文件没有导出 `mount_ck` 函数
    > 
    >

2. web-bundle 目录映射到 HTTP 的 /pkg/ 路由

    > 磁盘路径 `web-bundle/op_host_web.js` → 浏览器对应 `/pkg/op_host_web.js`
    > 
    > 页面报找不到 `/pkg/op_host_web.js` 时，是文件夹没有 `web-bundle/op_host_web.js` 文件

3. 本文档以 `openpencil 0.8.4` 版本编写，rustup 工具链版本 `1.94`、wasm-bindgen-cli 版本 `0.2.117`、skia-bindings 版本 `0.97.2` 均与该版本对齐，其他版本可能存在差异


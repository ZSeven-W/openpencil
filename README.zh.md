<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>全球首个 AI 原生开源矢量设计工具。</strong><br />
  <sub>并发 Agent 团队 &bull; 设计即代码 &bull; 内置 MCP 服务器 &bull; 多模型智能</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md"><b>简体中文</b></a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/rust-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/rust-check.yml?label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — 点击观看演示视频" width="100%" />
  </a>
</p>
<p align="center"><sub>点击图片观看演示视频</sub></p>

<br />

> **注意：** 另有一个同名的开源项目 — [OpenPencil](https://github.com/open-pencil/open-pencil)，专注于兼容 Figma 的可视化设计与实时协作。本项目专注于 AI 原生的设计转代码工作流。

## 为什么选择 OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 提示词 → 画布

用自然语言描述任意 UI，实时以流式动画在无限画布上生成。选中已有元素，通过对话即可修改设计。

</td>
<td width="50%">

### 🤖 并发 Agent 团队

编排器将复杂页面分解为空间子任务。多个 AI 智能体同时处理不同区块 — Hero、功能区、页脚 — 全部并行流式生成。

</td>
</tr>
<tr>
<td width="50%">

### 🧠 多模型智能

自动适配每个模型的能力。Claude 获得完整提示词和思考模式；GPT-4o/Gemini 关闭思考模式；小模型（MiniMax、Qwen、Llama）使用简化提示词以确保输出可靠性。

</td>
<td width="50%">

### 🔌 MCP 服务器

一键安装到 Claude Code、Codex、Gemini、OpenCode、Kiro 或 Copilot CLI。从终端进行设计 — 通过任意 MCP 兼容的智能体读取、创建和修改 `.op` 文件。

</td>
</tr>
<tr>
<td width="50%">

### 📦 设计即代码

`.op` 文件是 JSON — 人类可读、对 Git 友好、可进行 diff 对比。设计变量生成 CSS 自定义属性。代码导出为 React + Tailwind 或 HTML + CSS。

</td>
<td width="50%">

### 🖥️ 全平台运行

Web 应用 + macOS、Windows 和 Linux 原生桌面端 — 单一 Rust 核心，单个自包含二进制文件，无需浏览器引擎。`.op` 文件关联 — 双击即可打开。

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

从终端控制设计工具。`op design`、`op insert` — 批量设计 DSL、节点操作。支持从文件或 stdin 管道输入。可搭配桌面应用或 Web 服务器使用。

</td>
<td width="50%">

### 🎯 多平台代码导出

从单个 `.op` 文件导出到 React + Tailwind、HTML + CSS、Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Native。设计变量自动转换为 CSS 自定义属性。

</td>
</tr>
</table>

## 快速开始

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

或以桌面应用形式运行：

```bash
cargo run -p op-host-desktop
```

> **前置条件：** 构建产品需要 [Rust](https://www.rust-lang.org/)（stable）。[Bun](https://bun.sh/) >= 1.0 和 [Node.js](https://nodejs.org/) >= 18 仅用于 `packages/` 下的 web SDK。

### Docker

带标签的 Rust release 会发布单一 web-host 镜像。带内置 AI CLI 的旧 TypeScript 镜像不再发布。

| 镜像 | 包含 |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:v0.8.1` | Rust web host、wasm bundle 和 CanvasKit 资源 |

Web UI 只暴露内置 agent profiles；Docker 镜像不再内置 Claude/Codex/OpenCode/Copilot/Gemini CLI 工具。

**运行：**

```bash
docker run -d -p 3100:3100 ghcr.io/zseven-w/openpencil-web:v0.8.1
```

然后打开 `http://localhost:3100/`。

**本地构建：**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## AI 原生设计

**提示词生成 UI**

- **文字转设计** — 描述一个页面，实时以流式动画在画布上生成
- **编排器** — 将复杂页面分解为空间子任务，支持并行生成
- **设计修改** — 选中元素后，用自然语言描述更改
- **视觉输入** — 附加截图或线框图作为参考进行设计

**多智能体支持**

| 智能体                | 配置方式                                                              |
| --------------------- | --------------------------------------------------------------------- |
| **内置（9+ 提供商）** | 从提供商预设中选择并切换区域 — Anthropic、OpenAI、Google、DeepSeek 等 |
| **Claude Code**       | 无需配置 — 使用 Claude Agent SDK 本地 OAuth                           |
| **Codex CLI**         | 在 Agent 设置中连接（`Cmd+,`）                                        |
| **OpenCode**          | 在 Agent 设置中连接（`Cmd+,`）                                        |
| **GitHub Copilot**    | 运行 `copilot login` 后在 Agent 设置中连接（`Cmd+,`）                 |
| **Gemini CLI**        | 在 Agent 设置中连接（`Cmd+,`）                                        |

**模型能力配置** — 自动根据模型层级适配提示词、思考模式和超时时间。完整层级模型（Claude）获得完整提示词；标准层级模型（GPT-4o、Gemini、DeepSeek）关闭思考模式；基础层级模型（MiniMax、Qwen、Llama、Mistral）使用简化的嵌套 JSON 提示词以确保最大可靠性。

**国际化** — 完整界面本地化，支持 15 种语言：English、简体中文、繁體中文、日本語、한국어、Français、Español、Deutsch、Português、Русский、हिन्दी、Türkçe、ไทย、Tiếng Việt、Bahasa Indonesia。

**MCP 服务器**

- 内置 MCP 服务器（`op-mcp` crate）— 一键安装到 Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI
- 无需 Node.js — 通过桌面应用二进制文件（`--mcp <path>`）使用 stdio 传输，运行中的应用还提供实时 HTTP 端点（`127.0.0.1:<port>/mcp`）
- 从终端进行设计自动化：通过任意 MCP 兼容的智能体读取、创建和修改 `.op` 文件
- **分层设计工作流** — `design_skeleton` → `design_content` → `design_refine`，实现更高保真度的多区块设计
- **分段提示词检索** — 按需加载所需的设计知识（schema、layout、roles、icons、planning 等）
- 多页面支持 — 通过 MCP 工具创建、重命名、重新排序和复制页面

**代码生成**

- React + Tailwind CSS、HTML + CSS、CSS Variables
- Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Native

## CLI — `op`

全局安装后即可从终端控制设计工具：

```bash
brew install zseven-w/openpencil/op
```

```bash
op start                     # 启动桌面应用
op start --headless --file design.op # 启动无头服务器
op design @landing.txt       # 从文件批量设计
op design @ui.js             # 支持循环的沙箱 JavaScript
op insert '{"type":"rectangle"}' # 插入节点
op import:figma design.fig   # 导入 Figma 文件
cat design.dsl | op design - # 从 stdin 管道输入
```

支持内联字符串、`@filepath` 与 stdin（`-`），可搭配桌面应用、Web 服务器或文件型无头服务器使用。所有命令请查阅 [CLI 命令参考](./crates/op-cli/src/usage.txt)。

**LLM 技能** — 安装 [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) 插件，教 AI 智能体使用 `op` 进行设计。可运行 `op install` 为检测到的 Agent 安装，或使用 `op install --target codex` 指定目标。

## 功能特性

**画布与绘图**

- 无限画布，支持平移、缩放、智能对齐参考线和吸附
- 矩形、椭圆、直线、多边形、钢笔（贝塞尔）、Frame、文本
- 布尔运算 — 联合、减去、交集，配合上下文工具栏
- 图标选择器（Iconify）和图片导入（PNG/JPEG/SVG/WebP/GIF）
- 自动布局 — 垂直/水平方向，支持间距、内边距、主轴对齐、交叉轴对齐
- 多页面文档，支持标签页导航

**设计系统**

- 设计变量 — 颜色、数字、字符串令牌，支持 `$variable` 引用
- 多主题支持 — 多个主题轴，每个轴有多个变体（浅色/深色、紧凑/舒适）
- 组件系统 — 可复用组件，支持实例和覆盖
- CSS 同步 — 自动生成自定义属性，代码输出中使用 `var(--name)`
- 可复用 UIKit — 从 `.pen` 文件导入/导出组件套件

**AI 与智能体**

- 提示词转画布，支持流式生成与编排器驱动的空间分解
- 并发 Agent 团队 — 多个设计师并行处理不同区块，每位成员带画布指示器
- 分层工作流 — `design_skeleton` → `design_content` → `design_refine`，每个阶段使用聚焦的提示词
- 风格指南 — 50+ 内置风格（glassmorphism、brutalist、retro 等），支持基于标签的模糊匹配，并接入规划与生成流程
- 多模型能力配置 — 按模型层级自动适配思考模式、推理强度与提示词形态
- 内置智能体运行时（Rust）+ Anthropic、Claude Agent SDK、OpenCode、Codex、Copilot、Gemini 提供商
- 国产大模型 Anthropic 格式透传 — Kimi、Zhipu、GLM、DouBao、Ark、Bailian/DashScope、ModelScope、Coding Plans

**Git 集成**

- 克隆向导，支持 SSH / HTTPS 认证与 SSH 密钥管理
- 分支选择器 — 创建、切换、删除、合并，全部在 Git 面板中完成
- 拉取 / 推送级联，支持认证重试与非快进推送处理
- 文件夹模式三路合并，在磁盘上跟踪 `MERGE_HEAD` 状态
- 冲突面板 — 提供逐节点 / 逐字段三路卡片、内联 JSON 编辑器、批量操作与内联 diff 块
- 远程设置与 SSH 密钥界面；整个 Git 功能覆盖 15 种语言的 i18n

**导出**

- 画布导出 — PNG、JPEG、WEBP、PDF（`Cmd+Shift+P`）
- 代码导出 — React + Tailwind、HTML + CSS、Vue、Svelte、Flutter、SwiftUI、Jetpack Compose、React Native
- 增量 MCP 代码生成流水线 — `codegen_plan`、`codegen_submit_chunk`、`codegen_assemble`、`codegen_clean`

**Figma 导入**

- 导入 `.fig` 文件，保留布局、填充、描边、效果、文本、图片和矢量图形

**桌面应用**

- 原生支持 macOS、Windows 和 Linux — 单一自包含二进制文件（winit + GPU Skia，无需 Electron）
- `.op` 文件关联 — 双击即可打开，单实例锁定
- 后台检查 GitHub Releases 更新
- 原生应用菜单，支持另存为、打开最近使用，以及关闭时的未保存更改对话框
- 最近使用文件持久化

## 技术栈

|              |                                                                                  |
| ------------ | -------------------------------------------------------------------------------- |
| **核心**     | Rust workspace（`crates/`）— 编辑器状态、组件、宿主、MCP、AI、代码生成           |
| **渲染**     | 全平台统一使用 GPU Skia — 原生端 `skia-safe`（GL），浏览器端 CanvasKit（WASM/WebGL2） |
| **UI 框架** | jian — 内置的纯 Rust GPU-Skia UI 框架：组件、布局、事件、热重载（`vendor/jian`） |
| **窗口管理** | winit（内置 `casement` fork）                                                    |
| **桌面端**   | 原生二进制文件 `openpencil-desktop` — 无浏览器引擎                              |
| **Web SDK**  | `op-web-sdk` + React 19 / Vue 3 适配器 — 只读 `.op` 查看器（TypeScript）         |
| **CLI**      | `op` — 终端控制、批量设计 DSL                                                    |
| **AI**       | 内置 Rust Agent 运行时 · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **代码检查** | clippy · rustfmt（Rust）· oxlint · oxfmt（web SDK）                              |
| **文件格式** | `.op` — 基于 JSON，人类可读，对 Git 友好                                         |

## 生态

OpenPencil 是 **[ZSeven-W](https://github.com/ZSeven-W)** 出品的一系列纯 Rust、AI 原生工具家族中的一员。它们彼此协作：`jian` 渲染 OpenPencil，`agent-rs` 运行它的智能体，`noema` 负责记忆，`zode` 则从终端进行设计。

| 项目 | 简介 |
| ---- | ---- |
| **[Zode](https://github.com/ZSeven-W/zode)** | 面向终端的开源、AI 原生编程助手 — 一个快速的 Rust TUI（`ratatui`），可读取你的代码、运行命令、搜索文件并管理 git。通过 MCP 驱动 OpenPencil。 |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | 用于交付 LLM 智能体的纯 Rust 异步运行时 — 多提供商、端到端工具能力、结构化权限、真正的 MCP、零 `unsafe`。为 OpenPencil 内置的智能体运行时（`vendor/agent`）和 Zode 提供动力。 |
| **[jian](https://github.com/ZSeven-W/jian)** | 纯 Rust、GPU-Skia UI 框架 — 组件、布局、事件和热重载集于一栈。将声明式的 `.op` 文档变为原生、AI 可控的应用，无需 JS 运行时、无 DOM、无 Electron。OpenPencil 的 UI 框架（`vendor/jian`）。 |
| **[noema](https://github.com/ZSeven-W/noema)** | 面向编程智能体的本地优先、非向量记忆系统。以可检视的文件形式提供持久记忆、为新条目提供审阅队列，以及词法式（无需嵌入）召回 — 适用于 Zode、Codex、Claude Code 和 MCP 运行时。 |

## 为什么选择 Rust

OpenPencil 已从头用 **Rust** 完成重写（[#129](https://github.com/ZSeven-W/openpencil/issues/129)）。重写已经完成 — TypeScript + Electron 版编辑器已在 `v0.7.5` 退役，本仓库中的 Rust workspace 就是当前产品本身：一个体积更小、速度更快的原生核心，从单一代码库支持更多平台。

|                   | TypeScript + Electron（已退役，`v0.7.5`）  | Rust（当前）                                                             |
| ----------------- | ------------------------------------------ | ----------------------------------------------------------------------- |
| **桌面运行时**    | Electron — 内置 Chromium + Node.js         | 原生窗口（`winit` + GPU Skia），无浏览器引擎                             |
| **桌面体积**      | 每次安装均含完整 Chromium 运行时           | 单一自包含二进制文件 — **55.5 MB**                                       |
| **Web 包体积**    | JS + WASM 包                               | **8.2 MB** wasm / 传输 **2.18 MB** gzip                                 |
| **渲染**          | Web 端 CanvasKit/Skia                      | **所有**目标平台统一使用 GPU 加速 Skia 后端                              |
| **内存**          | JavaScript GC 暂停                         | 无 GC — Rust 所有权，延迟可预期                                          |
| **代码库**        | Web 技术栈 + Electron     | 单一 Rust workspace：editor · CLI · MCP · AI · codegen · Figma · Git    |
| **目标平台**      | Web + 桌面，两套独立技术栈                 | 桌面（macOS/Win/Linux）· 移动端（iOS/Android）· 浏览器 — 共用同一核心   |

**可量化的性能提升**

- **极小体积** — 整个桌面应用是一个 **55.5 MB** 的原生二进制文件，而非捆绑浏览器引擎和 Node.js 运行时。拆分图标目录后，Web 构建产物为 **8.2 MB** 原始 / **2.18 MB** gzip（传输体积 −48%）。
- **大文档扩展性** — **10,000-node** 实时画布（嵌套四层自动布局）在写入、读取和快照布局时**不崩溃、空闲 CPU 约为 ~0%**；对全部 10k 节点执行完整布局快照仅需 **~0.68 s**。
- **流畅交互** — 平移/缩放不再在每一帧都重新序列化文档（单个热路径修复将滚轮缩放 CPU 占用从 **~69% 降至 ~0%**）；拖拽增量更新场景图，文本测量结果已缓存，重绘合并为每帧一次。
- **一套核心，全端覆盖** — 相同的编辑器状态和渲染后端，可编译为原生桌面、移动端，以及通过 WASM 运行的浏览器端 — 无需维护多套并行实现。
- **GPU Skia 全覆盖** — 原生端通过 `skia-safe` 在 GL 上下文上渲染；浏览器端通过 CanvasKit 在 WebGL2 上渲染 — 同一套绘图代码，同一套输出结果。
- **原生无障碍** — macOS、Windows 和 Linux 上通过 AccessKit 实现，Web 端通过 DOM 镜像实现，而非依赖浏览器自身的无障碍树。
- **单一类型检查工作区** — MCP 宿主、CLI、AI 提供商、代码生成、Figma 导入和 Git 集成全部位于同一个 Rust workspace，并在 CI 中通过 `cargo-deny` 进行供应链管控。

> **状态：** TypeScript 版编辑器已在 `v0.7.5` 退役，仅存在于 Git 历史中；本仓库即为 Rust workspace。`v0.8.1` 版本的 Rust 发布正在积极开发中（参见下方路线图）。

## 项目结构

```text
openpencil/
├── crates/                   Rust workspace — 产品本体
│   ├── op-editor-core/       规范的 `.op`（PenDocument）编辑器状态 + EditorCommand + 设计变量
│   ├── op-editor-ui/         平台无关组件 + RenderBackend 门面（wasm32 兼容）
│   ├── op-editor-host-core/  所有宿主共用、与传输层无关的宿主状态机
│   ├── op-host-native/       原生宿主库 — winit + skia-safe GL（桌面端 + 移动端）
│   ├── op-host-web/          浏览器构建产物 — wasm32 cdylib，CanvasKit 渲染器
│   ├── op-host-desktop/      桌面应用二进制 `openpencil-desktop`；同时也是 `--serve-web` 守护进程
│   ├── op-host-services/     无头 serve-web / MCP 守护进程库
│   ├── op-host-web-server/   无 GL 依赖的轻量级 Web 服务器二进制
│   ├── op-cli/               CLI 工具 — `op` 命令
│   ├── op-mcp/               MCP 服务器 — 工具、批量设计、分层工作流
│   ├── op-ai/                AI 提供商、聊天运行时、流式处理
│   ├── op-ai-skills/         AI 提示词技能引擎（分阶段 prompt 加载）
│   ├── op-orchestrator/      并发 Agent 团队编排
│   ├── op-codegen/           代码生成器（React、HTML、Vue、Flutter 等）
│   ├── op-figma/             Figma .fig 文件解析器与转换器
│   ├── op-git/               Git 集成 — 克隆、分支、推送/拉取、合并
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Web SDK 工作区（Bun）
│   ├── op-web-sdk/           只读 `.op` Web 查看器 SDK（封装 wasm 构建产物）
│   ├── op-web-sdk-react/     React 19 适配器
│   └── op-web-sdk-vue/       Vue 3 适配器
├── vendor/                   内置子系统（git 子模块）
│   ├── jian/                 GPU-Skia UI 框架 — 组件/渲染/事件
│   ├── casement/             winit fork
│   └── agent/                跨产品 Rust Agent 运行时（agent-rs）
└── .githooks/                预提交版本漂移检查
```

## 键盘快捷键

| 按键        | 操作         |     | 按键          | 操作                    |
| ----------- | ------------ | --- | ------------- | ----------------------- |
| `V`         | 选择         |     | `Cmd+S`       | 保存                    |
| `R`         | 矩形         |     | `Cmd+Z`       | 撤销                    |
| `O`         | 椭圆         |     | `Cmd+Shift+Z` | 重做                    |
| `L`         | 直线         |     | `Cmd+C/X/V/D` | 复制/剪切/粘贴/重复     |
| `T`         | 文本         |     | `Cmd+G`       | 编组                    |
| `F`         | Frame        |     | `Cmd+Shift+G` | 取消编组                |
| `P`         | 钢笔工具     |     | `Cmd+Shift+P` | 导出 (PNG/JPG/WEBP/PDF) |
| `H`         | 手形（平移） |     | `Cmd+Shift+C` | 代码面板                |
| `Del`       | 删除         |     | `Cmd+Shift+V` | 变量面板                |
| `[ / ]`     | 调整层级顺序 |     | `Cmd+J`       | AI 聊天                 |
| 方向键      | 微移 1px     |     | `Cmd+,`       | 智能体设置              |
| `Cmd+Alt+U` | 布尔联合     |     | `Cmd+Alt+S`   | 布尔减去                |
| `Cmd+Alt+I` | 布尔交集     |     | `Cmd+Shift+S` | 另存为                  |

## 脚本命令

```bash
# Product (Rust — run from the repo root)
cargo build --workspace              # Build all crates (add --release for prod)
cargo test --workspace               # Run all tests
cargo check --workspace              # Type check
cargo clippy --workspace --all-targets -- -D warnings   # Lint
cargo fmt --all                      # Format
bash scripts/start-web-rust.sh       # Web dev server (wasm bundle + headless host)
cargo run -p op-host-desktop         # Desktop app (binary: openpencil-desktop)
cargo run -p op-cli -- <args>        # CLI (binary: op)

# Web SDK / JS tooling (run from packages/)
cd packages && bun run lint          # Lint the web SDK (oxlint); also: bun run format
cd packages && bun run generate-iconify-catalog   # Regenerate the Rust icon catalog assets

# 版本同步（从仓库根目录运行）
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

## 参与贡献

欢迎贡献！请查阅 [CLAUDE.md](./CLAUDE.md) 了解架构细节和代码风格。

1. Fork 并克隆仓库
2. 启用版本漂移检查：`git config core.hooksPath .githooks`
3. 创建分支：`git checkout -b feat/my-feature`
4. 运行检查：`cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
5. 使用 [Conventional Commits](https://www.conventionalcommits.org/) 提交：`feat(canvas): add rotation snapping`
6. 向 `main` 分支发起 PR

## 路线图

- [x] 设计变量与令牌，支持 CSS 同步
- [x] 组件系统（实例与覆盖）
- [x] 带编排器的 AI 设计生成
- [x] MCP 服务器集成与分层设计工作流
- [x] 多页面支持
- [x] Figma `.fig` 导入
- [x] 布尔运算（合并、减去、相交）
- [x] 多模型能力配置
- [x] Cargo workspace，包含可复用的 Rust crate 和 Web SDK 包
- [x] Rust 桌面端与 Web 端编辑器
- [x] CLI 工具（`op`）终端控制
- [x] 内置 Rust Agent Runtime，支持多提供商
- [x] 国际化 — 15 种语言
- [x] 基于 wasm 的 JavaScript、React 与 Vue Viewer SDK
- [x] Style Guides，支持基于标签的匹配和 MCP 工具
- [x] Concurrent Agent Teams，支持任务委派和画布状态指示
- [x] Git 集成（克隆、分支、推送/拉取、文件夹模式三路合并）
- [x] 画布导出（SVG / PNG / JPEG / WEBP / PDF）
- [ ] 协同编辑
- [ ] 插件系统

## 贡献者

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## 赞助者

OpenPencil 免费开源,开发完全由觉得它好用的人们资助 —— 感谢你让这块画布一直保持开放。

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

感谢 **[MrQyun](https://github.com/mrqyun)** —— 想把自己的名字也放在这里?**[成为赞助者 →](https://github.com/sponsors/ZSeven-W)**

## 社区

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> 加入我们的 Discord</strong>
</a>
— 提问、分享设计、提出功能建议。

**飞书交流群**

<img src="./screenshot/557517811-62010928-d91a-4223-bc10-9ee7a4fbf043.jpg" alt="飞书交流群" width="240" />

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## 安全评估

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## 许可证

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

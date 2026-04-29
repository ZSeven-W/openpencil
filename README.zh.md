<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
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

Web 应用 + 通过 Electron 支持 macOS、Windows 和 Linux 原生桌面端。从 GitHub Releases 自动更新。`.op` 文件关联 — 双击即可打开。

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
# 安装依赖
bun install

# 在 http://localhost:3000 启动开发服务器
bun --bun run dev
```

或以桌面应用形式运行：

```bash
bun run electron:dev
```

> **前置条件：** [Bun](https://bun.sh/) >= 1.0 以及 [Node.js](https://nodejs.org/) >= 18

### Docker

提供多个镜像变体 — 按需选择：

| 镜像                         | 大小    | 包含                 |
| ---------------------------- | ------- | -------------------- |
| `openpencil:latest`          | ~226 MB | 仅 Web 应用          |
| `openpencil-claude:latest`   | —       | + Claude Code CLI    |
| `openpencil-codex:latest`    | —       | + Codex CLI          |
| `openpencil-opencode:latest` | —       | + OpenCode CLI       |
| `openpencil-copilot:latest`  | —       | + GitHub Copilot CLI |
| `openpencil-gemini:latest`   | —       | + Gemini CLI         |
| `openpencil-full:latest`     | ~1 GB   | 全部 CLI 工具        |

**运行（仅 Web）：**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**运行 AI CLI（以 Claude Code 为例）：**

AI 聊天依赖 Claude CLI 的 OAuth 登录。使用 Docker volume 持久化登录会话：

```bash
# 第一步 — 登录（仅需一次）
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# 第二步 — 启动
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**本地构建：**

```bash
# 基础版（仅 Web）
docker build --target base -t openpencil .

# 指定 CLI
docker build --target with-claude -t openpencil-claude .

# 完整版（全部 CLI）
docker build --target full -t openpencil-full .
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

- 内置 MCP 服务器 — 一键安装到 Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI
- 自动检测 Node.js — 若未安装则自动回退到 HTTP 传输模式并启动 MCP HTTP 服务器
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
npm install -g @zseven-w/openpencil
```

```bash
op start                     # 启动桌面应用
op design @landing.txt       # 从文件批量设计
op insert '{"type":"RECT"}'  # 插入节点
op import:figma design.fig   # 导入 Figma 文件
cat design.dsl | op design - # 从 stdin 管道输入
```

支持三种输入方式：内联字符串、`@filepath`（从文件读取）、`-`（从 stdin 读取）。可搭配桌面应用或 Web 开发服务器使用。完整命令参考请查阅 [CLI README](./apps/cli/README.md)。

**LLM 技能** — 安装 [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) 插件，教 AI 智能体（Claude Code、Cursor、Codex、Gemini CLI 等）使用 `op` 进行设计。

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
- 内置智能体运行时（`agent-native`，Zig NAPI）+ Anthropic、Claude Agent SDK、OpenCode、Codex、Copilot、Gemini 提供商
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

- 通过 Electron 支持原生 macOS、Windows 和 Linux
- `.op` 文件关联 — 双击即可打开，单实例锁定
- 从 GitHub Releases 自动更新
- 原生应用菜单，支持另存为、打开最近使用，以及关闭时的未保存更改对话框
- 最近使用文件持久化

## 技术栈

|              |                                                                                  |
| ------------ | -------------------------------------------------------------------------------- |
| **前端**     | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui · i18next                |
| **画布**     | CanvasKit/Skia（WASM, GPU 加速）                                                 |
| **状态管理** | Zustand v5                                                                       |
| **服务器**   | Nitro                                                                            |
| **桌面端**   | Electron 35                                                                      |
| **CLI**      | `op` — 终端控制、批量设计 DSL                                                    |
| **AI**       | Vercel AI SDK v6 · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **运行时**   | Bun · Vite 7                                                                     |
| **文件格式** | `.op` — 基于 JSON，人类可读，对 Git 友好                                         |

## 项目结构

```text
openpencil/
├── apps/
│   ├── web/                 TanStack Start Web 应用
│   │   ├── src/
│   │   │   ├── canvas/      CanvasKit/Skia 引擎 — 绘图、同步、布局
│   │   │   ├── components/  React UI — 编辑器、面板、共享对话框、图标
│   │   │   ├── services/ai/ AI 聊天、编排器、设计生成、流式处理
│   │   │   ├── stores/      Zustand — 画布、文档、页面、历史、AI
│   │   │   ├── mcp/         供外部 CLI 集成使用的 MCP 服务器工具
│   │   │   ├── hooks/       键盘快捷键、文件拖放、Figma 粘贴
│   │   │   └── uikit/       可复用组件套件系统
│   │   └── server/
│   │       ├── api/ai/      Nitro API — 流式聊天、生成、验证
│   │       └── utils/       Claude CLI、OpenCode、Codex、Copilot 客户端封装
│   ├── desktop/             Electron 桌面应用
│   │   ├── main.ts          窗口、Nitro 子进程、原生菜单、自动更新
│   │   ├── ipc-handlers.ts  原生文件对话框、主题同步、偏好设置 IPC
│   │   └── preload.ts       IPC 桥接
│   └── cli/                 CLI 工具 — `op` 命令
│       ├── src/commands/    设计、文档、导出、导入、节点、页面、变量命令
│       ├── connection.ts    与运行中应用的 WebSocket 连接
│       └── launcher.ts      自动检测并启动桌面应用或 Web 服务器
├── packages/
│   ├── pen-types/           PenDocument 模型类型定义
│   ├── pen-core/            文档树操作、布局引擎、变量
│   ├── pen-codegen/         代码生成器（React、HTML、Vue、Flutter 等）
│   ├── pen-figma/           Figma .fig 文件解析与转换
│   ├── pen-renderer/        独立 CanvasKit/Skia 渲染器
│   ├── pen-sdk/             聚合 SDK（重新导出所有包）
│   ├── pen-ai-skills/       AI 提示词技能引擎（分阶段 prompt 加载）
│   └── agent/               AI Agent SDK（Vercel AI SDK、多提供商、Agent 团队）
└── .githooks/               预提交钩子：从分支名同步版本号
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
bun --bun run dev          # 开发服务器（端口 3000）
bun --bun run build        # 生产构建
bun --bun run test         # 运行测试（Vitest）
npx tsc --noEmit           # 类型检查
bun run bump <version>     # 同步所有 package.json 的版本号
bun run electron:dev       # Electron 开发模式
bun run electron:build     # Electron 打包
bun run cli:dev            # 从源码运行 CLI
bun run cli:compile        # 编译 CLI 到 dist
```

## 参与贡献

欢迎贡献！请查阅 [CLAUDE.md](./CLAUDE.md) 了解架构细节和代码风格。

1. Fork 并克隆仓库
2. 设置版本同步：`git config core.hooksPath .githooks`
3. 创建分支：`git checkout -b feat/my-feature`
4. 运行检查：`npx tsc --noEmit && bun --bun run test`
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
- [x] Monorepo 重构与可复用包
- [x] CLI 工具（`op`）终端控制
- [x] 内置 AI Agent SDK，支持多提供商
- [x] 国际化 — 15 种语言
- [x] Git 集成（克隆、分支、推送/拉取、文件夹模式三路合并）
- [x] 画布栅格导出（PNG / JPEG / WEBP / PDF）
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
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
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

## 许可证

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

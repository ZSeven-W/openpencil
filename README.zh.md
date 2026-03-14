<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://discord.gg/KwXp6BJD"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — 点击观看演示视频" width="100%" />
  </a>
</p>
<p align="center"><sub>点击图片观看演示视频</sub></p>

<br />

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

自动适配每个模型的能力。Claude 获得完整提示词和思考模式；GPT-4o/Gemini 关闭思考模式；小模型（MiniMax、通义千问、Llama）使用简化提示词以确保输出可靠性。

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

## AI 原生设计

**提示词生成 UI**
- **文字转设计** — 描述一个页面，实时以流式动画在画布上生成
- **编排器** — 将复杂页面分解为空间子任务，支持并行生成
- **设计修改** — 选中元素后，用自然语言描述更改
- **视觉输入** — 附加截图或线框图作为参考进行设计

**多智能体支持**

| 智能体 | 配置方式 |
| --- | --- |
| **Claude Code** | 无需配置 — 使用 Claude Agent SDK 本地 OAuth |
| **Codex CLI** | 在 Agent 设置中连接（`Cmd+,`） |
| **OpenCode** | 在 Agent 设置中连接（`Cmd+,`） |
| **GitHub Copilot** | 运行 `copilot login` 后在 Agent 设置中连接（`Cmd+,`） |

**模型能力配置** — 自动根据模型层级适配提示词、思考模式和超时时间。完整层级模型（Claude）获得完整提示词；标准层级模型（GPT-4o、Gemini、DeepSeek）关闭思考模式；基础层级模型（MiniMax、通义千问、Llama、Mistral）使用简化的嵌套 JSON 提示词以确保最大可靠性。

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
- 从设计令牌生成 CSS Variables

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

**Figma 导入**
- 导入 `.fig` 文件，保留布局、填充、描边、效果、文本、图片和矢量图形

**桌面应用**
- 通过 Electron 支持原生 macOS、Windows 和 Linux
- `.op` 文件关联 — 双击即可打开，单实例锁定
- 从 GitHub Releases 自动更新
- 原生应用菜单和文件对话框

## 技术栈

| | |
| --- | --- |
| **前端** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **画布** | CanvasKit/Skia（WASM, GPU 加速） |
| **状态管理** | Zustand v5 |
| **服务器** | Nitro |
| **桌面端** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **运行时** | Bun · Vite 7 |
| **文件格式** | `.op` — 基于 JSON，人类可读，对 Git 友好 |

## 项目结构

```text
src/
  canvas/          CanvasKit/Skia 引擎 — 绘图、同步、布局、参考线、钢笔工具
  components/      React UI — 编辑器、面板、共享对话框、图标
  services/ai/     AI 聊天、编排器、设计生成、流式处理
  services/figma/  Figma .fig 二进制文件导入流水线
  services/codegen 多平台代码生成器（React、HTML、Vue、Svelte、Flutter、SwiftUI、Compose、React Native）
  stores/          Zustand — 画布、文档、页面、历史、AI、设置
  variables/       设计令牌解析与引用管理
  mcp/             供外部 CLI 集成使用的 MCP 服务器工具
  uikit/           可复用组件套件系统
server/
  api/ai/          Nitro API — 流式聊天、生成、验证
  utils/           Claude CLI、OpenCode、Codex、Copilot 客户端封装
electron/
  main.ts          窗口、Nitro 子进程、原生菜单、自动更新
  preload.ts       IPC 桥接
```

## 键盘快捷键

| 按键 | 操作 | | 按键 | 操作 |
| --- | --- | --- | --- | --- |
| `V` | 选择 | | `Cmd+S` | 保存 |
| `R` | 矩形 | | `Cmd+Z` | 撤销 |
| `O` | 椭圆 | | `Cmd+Shift+Z` | 重做 |
| `L` | 直线 | | `Cmd+C/X/V/D` | 复制/剪切/粘贴/重复 |
| `T` | 文本 | | `Cmd+G` | 编组 |
| `F` | Frame | | `Cmd+Shift+G` | 取消编组 |
| `P` | 钢笔工具 | | `Cmd+Shift+E` | 导出 |
| `H` | 手形（平移） | | `Cmd+Shift+C` | 代码面板 |
| `Del` | 删除 | | `Cmd+Shift+V` | 变量面板 |
| `[ / ]` | 调整层级顺序 | | `Cmd+J` | AI 聊天 |
| 方向键 | 微移 1px | | `Cmd+,` | 智能体设置 |
| `Cmd+Alt+U` | 布尔联合 | | `Cmd+Alt+S` | 布尔减去 |
| `Cmd+Alt+I` | 布尔交集 | | | |

## 脚本命令

```bash
bun --bun run dev          # 开发服务器（端口 3000）
bun --bun run build        # 生产构建
bun --bun run test         # 运行测试（Vitest）
npx tsc --noEmit           # 类型检查
bun run electron:dev       # Electron 开发模式
bun run electron:build     # Electron 打包
```

## 参与贡献

欢迎贡献！请查阅 [CLAUDE.md](./CLAUDE.md) 了解架构细节和代码风格。

1. Fork 并克隆仓库
2. 创建分支：`git checkout -b feat/my-feature`
3. 运行检查：`npx tsc --noEmit && bun --bun run test`
4. 使用 [Conventional Commits](https://www.conventionalcommits.org/) 提交：`feat(canvas): add rotation snapping`
5. 向 `main` 分支发起 PR

## 路线图

- [x] 设计变量与令牌，支持 CSS 同步
- [x] 组件系统（实例与覆盖）
- [x] 带编排器的 AI 设计生成
- [x] MCP 服务器集成与分层设计工作流
- [x] 多页面支持
- [x] Figma `.fig` 导入
- [x] 布尔运算（合并、减去、相交）
- [x] 多模型能力配置
- [ ] 协同编辑
- [ ] 插件系统

## 贡献者

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## 社区

<a href="https://discord.gg/KwXp6BJD">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
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

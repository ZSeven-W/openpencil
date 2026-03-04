<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>AI 原生开源设计工具。设计即代码。</strong><br />
  从提示词到画布 UI。多智能体编排。内置 MCP 服务器。代码生成。
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md"><b>简体中文</b></a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#quick-start">快速开始</a> ·
  <a href="#ai-native-design">AI</a> ·
  <a href="#features">功能特性</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#contributing">参与贡献</a>
</p>

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

OpenPencil 从底层就围绕 AI 构建——不是作为插件，而是作为核心工作流。

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

**MCP 服务器**
- 内置 MCP 服务器 — 一键安装到 Claude Code / Codex / Gemini / OpenCode / Kiro CLI
- 从终端进行设计自动化：通过任意 MCP 兼容的智能体读取、创建和修改 `.op` 文件

**代码生成**
- React + Tailwind CSS
- HTML + CSS
- 从设计令牌生成 CSS Variables

## 功能特性

**画布与绘图**
- 无限画布，支持平移、缩放、智能对齐参考线和吸附
- 矩形、椭圆、直线、多边形、钢笔（贝塞尔）、Frame、文本
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
- 从 GitHub Releases 自动更新
- 原生应用菜单和文件对话框

## 技术栈

| | |
| --- | --- |
| **前端** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **画布** | Fabric.js v7 |
| **状态管理** | Zustand v5 |
| **服务器** | Nitro |
| **桌面端** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **运行时** | Bun · Vite 7 |
| **文件格式** | `.op` — 基于 JSON，人类可读，对 Git 友好 |

## 项目结构

```text
src/
  canvas/          Fabric.js 引擎 — 绘图、同步、布局、参考线、钢笔工具
  components/      React UI — 编辑器、面板、共享对话框、图标
  services/ai/     AI 聊天、编排器、设计生成、流式处理
  services/figma/  Figma .fig 二进制文件导入流水线
  services/codegen React+Tailwind 和 HTML+CSS 代码生成器
  stores/          Zustand — 画布、文档、页面、历史、AI、设置
  variables/       设计令牌解析与引用管理
  mcp/             供外部 CLI 集成使用的 MCP 服务器工具
  uikit/           可复用组件套件系统
server/
  api/ai/          Nitro API — 流式聊天、生成、验证
  utils/           Claude CLI、OpenCode、Codex 客户端封装
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
- [x] MCP 服务器集成
- [x] 多页面支持
- [x] Figma `.fig` 导入
- [ ] 布尔运算（合并、减去、相交）
- [ ] 协同编辑
- [ ] 插件系统

## 贡献者

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## 社区

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> 加入我们的 Discord</strong>
</a>
— 提问、分享设计、提出功能建议。

## 许可证

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

# @zseven-w/openpencil

[English](./README.md) · [**简体中文**](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

[OpenPencil](https://github.com/ZSeven-W/openpencil) 的命令行工具 — 从终端控制设计工具。

## 安装

```bash
npm install -g @zseven-w/openpencil
```

## 平台支持

CLI 会自动检测并启动各平台上的 OpenPencil 桌面应用：

| 平台        | 检测的安装路径                                                                                      |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS 用户级 (`%LOCALAPPDATA%`)、系统级 (`%PROGRAMFILES%`)、便携版                                    |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## 用法

```bash
op <command> [options]
```

### 输入方式

接受 JSON 或 DSL 的参数支持三种传入方式：

```bash
op design '...'              # 内联字符串（适合小型内容）
op design @design.txt        # 从文件读取（推荐用于大型设计）
cat design.txt | op design - # 从标准输入读取（管道传入）
```

### 应用控制

```bash
op start [--desktop|--web]   # 启动 OpenPencil（默认桌面版）
op stop                      # 停止运行中的实例
op status                    # 检查运行状态
```

### 设计（批量 DSL）

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### 文档操作

```bash
op open [file.op]            # 打开文件或连接到实时画布
op save <file.op>            # 保存当前文档
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # 获取当前画布选中项
```

### 节点操作

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### 代码导出

```bash
op export <format> [--out file]
# 格式：react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### 变量与主题

```bash
op vars                      # 获取变量
op vars:set <json>           # 设置变量
op themes                    # 获取主题
op themes:set <json>         # 设置主题
op theme:save <file.optheme> # 保存主题预设
op theme:load <file.optheme> # 加载主题预设
op theme:list [dir]          # 列出主题预设
```

### 页面

```bash
op page list                 # 列出页面
op page add [--name N]       # 添加页面
op page remove <id>          # 删除页面
op page rename <id> <name>   # 重命名页面
op page reorder <id> <index> # 调整页面顺序
op page duplicate <id>       # 复制页面
```

### 导入

```bash
op import:svg <file.svg>     # 导入 SVG 文件
op import:figma <file.fig>   # 导入 Figma .fig 文件
```

### 布局

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### 全局选项

```text
--file <path>     目标 .op 文件（默认：实时画布）
--page <id>       目标页面 ID
--pretty          人类可读的 JSON 输出
--help            显示帮助
--version         显示版本
```

## 许可证

MIT

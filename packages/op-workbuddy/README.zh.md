# WorkBuddy OpenPencil 连接器

**面向腾讯 WorkBuddy 的 OpenPencil 连接器与内嵌交互式 Rust 画布（`@zseven-w/workbuddy-openpencil`）。**

[English](./README.md) · [**简体中文**](./README.zh.md) · [安装指南](./INSTALL.md) · [架构说明](./docs/architecture.md)

## 核心特性

- **原生内嵌交互画布（MCP Apps）**：通过 `open_canvas` 暴露 `text/html;profile=mcp-app` 资源并在 `_meta.ui.csp.frameDomains` 中声明本地 loopback panel 源，适配 WorkBuddy 的 MCP Apps 宿主，在对话面板内直接运行可编辑 Rust WASM 画布。
- **完整原生设计工具**：通过单一串行控制器代理所有 OpenPencil Rust MCP 工具（`get_editor_state`、`batch_design`、`snapshot_layout`、`lint_document`、`get_screenshot`、`codegen_export` 等），对话工具与画布操作共享同一份文档。
- **选区联动与代码生成闭环**：通过 `ui/update-model-context` 同步选区，提供 `canvas_selection`、`canvas_import_image`（本地图片直嵌）与代码面板 `ui/message` 交接。
- **受保护保存与恢复草稿**：每个会话在 `~/.workbuddy-openpencil/sessions/` 维护独立恢复副本；保存时校验基线 SHA-256 并拒绝覆盖外部改动。
- **一键配置与校验**：自动写入 `~/.workbuddy/mcp.json`（保留其他连接器）、安装受管 Skill 到 `~/.workbuddy/skills/`、私密记录 Node 22+ 路径并通过 `host-check` 在精简 GUI `PATH` 下真实验证启动链路。

## 快速开始

```sh
npm install -g @zseven-w/workbuddy-openpencil
workbuddy-openpencil setup
```

随后打开 WorkBuddy → **连接器**，找到 **openpencil** 并点击 **信任 / 启用**，新建对话即可使用。

如需手动在 WorkBuddy 界面粘贴配置，可运行：

```sh
workbuddy-openpencil print-config
```

## 常用命令

| 命令 | 说明 |
| --- | --- |
| `workbuddy-openpencil setup` | 校验运行包、注册 `~/.workbuddy/mcp.json`、安装 Skill 并执行 `host-check` |
| `workbuddy-openpencil print-config` | 输出可直接粘贴到「自定义连接器」的 `mcpServers` JSON |
| `workbuddy-openpencil host-check` | 模拟 GUI 精简 `PATH` 验证连接器启动、工具与 MCP App 资源 |
| `workbuddy-openpencil canvas [--file PATH]` | 终端持久化画布模式 |
| `workbuddy-openpencil doctor [--port PORT]` | 诊断已运行的 OpenPencil 桌面端 MCP 服务 |
| `workbuddy-openpencil uninstall` | 从 `~/.workbuddy/mcp.json` 与 `~/.workbuddy/skills/` 安全卸载 |

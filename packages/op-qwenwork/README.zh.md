# 千问办公（QwenWork）OpenPencil 连接器

**面向千问办公（QwenWork / `QwenWorkCN.app`）的 OpenPencil 连接器、本地交互式 Rust 画布与套件包（`@zseven-w/qwenwork-openpencil`）。**

[English](./README.md) · [**简体中文**](./README.zh.md) · [安装指南](./INSTALL.md) · [架构说明](./docs/architecture.md)

## 核心特性

- **托管交互式 Rust 画布**：每个对话独占一个受控 `op-host-web-server`（Rust WASM + CanvasKit），同时暴露标准 MCP Apps（`open_canvas`、`text/html;profile=mcp-app`）与浏览器/预览打开入口（`canvas_open_browser`）。
- **全面覆盖千问办公三种接入方式**：
  1. `qwenwork-openpencil setup` — 自动识别 `~/.qwenworkcn` / `~/.qwenwork` / `~/.qoderworkcn` / `~/.qoderwork`，原子合并 `mcp.json`、安装受管 Skill 并完成 `host-check`。
  2. `qwenwork-openpencil print-config` — 输出可直接粘贴到 **扩展 → 连接器 → 添加 → 粘贴 JSON 配置** 的 `mcpServers` 配置（兼容账号隔离模式）。
  3. `qwenwork-openpencil pack-plugin` — 零依赖一键打包符合千问办公校验规范的本地套件 `.zip`（含 `.qwenwork-plugin/plugin.json`、`.mcp.json`、`CONNECTORS.md` 与 `skills/`），可在 **扩展 → 套件 → 导入本地套件** 直接导入。
- **完整原生设计工具**：通过单一串行控制器代理所有 OpenPencil Rust MCP 工具（`get_editor_state`、`batch_design`、`snapshot_layout`、`lint_document`、`get_screenshot`、`codegen_export` 等）。
- **选区联动与代码生成闭环**：支持 `canvas_selection`、`canvas_import_image`（本地图片直嵌）、受保护 `canvas_save` 与代码面板交接（`canvas_codegen_*`）。
- **受保护保存与恢复草稿**：每个会话在 `~/.qwenwork-openpencil/sessions/` 维护独立恢复副本；保存时校验基线 SHA-256 并拒绝覆盖外部改动。

## 快速开始

```sh
npm install -g @zseven-w/qwenwork-openpencil
qwenwork-openpencil setup
```

若千问办公处于账号隔离模式（界面中尚未出现 `openpencil` 连接器），可直接任选其一：
- **粘贴连接器 JSON**：运行 `qwenwork-openpencil print-config`，在千问办公 **扩展 → 连接器 → 添加 → 粘贴 JSON 配置** 粘贴保存。
- **导入本地套件 ZIP**：运行 `qwenwork-openpencil pack-plugin`，在千问办公 **扩展 → 套件 → 导入本地套件** 选择生成的 `openpencil-qwenwork-plugin.zip`。

## 常用命令

| 命令 | 说明 |
| --- | --- |
| `qwenwork-openpencil setup` | 校验运行包、注册 `mcp.json`、安装 Skill 并执行 `host-check` |
| `qwenwork-openpencil print-config` | 输出可直接粘贴到「扩展 → 连接器」的 `mcpServers` JSON |
| `qwenwork-openpencil pack-plugin [--output PATH]` | 打包可直接导入「扩展 → 套件」的本地套件 `.zip` |
| `qwenwork-openpencil host-check` | 模拟 GUI 精简 `PATH` 验证连接器启动、工具与 MCP App 资源 |
| `qwenwork-openpencil canvas [--file PATH]` | 终端持久化画布模式 |
| `qwenwork-openpencil doctor [--port PORT]` | 诊断已运行的 OpenPencil 桌面端 MCP 服务 |
| `qwenwork-openpencil uninstall` | 从 `mcp.json` 与 `skills/` 安全卸载 |

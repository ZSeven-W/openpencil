# 豆包办公（豆包电脑版）OpenPencil 连接器

**面向豆包电脑版「工作任务 / 技能·连接器·伙伴」的 OpenPencil 本地 `stdio` MCP 连接器、交互式 Rust 画布与技能打包工具（`@zseven-w/doubao-openpencil`）。**

[English](./README.md) · [**简体中文**](./README.zh.md) · [安装指南](./INSTALL.md) · [架构说明](./docs/architecture.md)

## 核心特性

- **托管交互式 Rust 画布**：每个工作任务独占一个受控 `op-host-web-server`（Rust WASM + CanvasKit），提供 `canvas_open_browser` 与 `open_canvas` 返回受令牌保护的本地 `panelUrl`（同时暴露标准 MCP Apps 资源，兼容未来内嵌预览）。
- **完整适配豆包电脑版「技能·连接器·伙伴」**：
  - `doubao-openpencil print-config`：输出标准 `mcpServers` JSON 配置，可直接粘贴导入。
  - `doubao-openpencil print-form`：输出「手动添加自定义连接器（本地命令 / stdio）」逐项表单内容，无需手工拼命令。
  - `doubao-openpencil pack-skill`：零依赖一键打包可上传的豆包技能 `.zip`（含 `SKILL.md` 及 `references/`，支持 `--flat` 平铺模式），可在 **我的技能 → 新建 → 上传技能** 直接拖入安装。
- **GUI 启动零环境问题**：私密记录 Node 22+ 路径到 `~/.doubao-openpencil/node-path`，避免豆包桌面端因 GUI `PATH` 缺失 nvm/Homebrew 而启动失败，并通过 `host-check` 实测验证。
- **完整原生设计工具与受保护保存**：单一串行控制器代理所有 OpenPencil Rust MCP 工具（`get_editor_state`、`batch_design`、`snapshot_layout`、`lint_document`、`get_screenshot`、`codegen_export` 等），恢复草稿存于 `~/.doubao-openpencil/sessions/`，保存前校验 SHA-256 基线。

## 快速开始

```sh
npm install -g @zseven-w/doubao-openpencil
doubao-openpencil setup
doubao-openpencil print-form
```

随后在豆包电脑版中完成两步：
1. **添加连接器**：打开 **技能·连接器·伙伴 → 连接器 → 手动添加 / 自定义连接器**，按 `print-form` 逐项填写（或粘贴 `print-config` JSON），保存并启用 **openpencil**。
2. **上传技能包**：打开 **技能·连接器·伙伴 → 我的技能 → 新建 → 上传技能**，拖入 `~/.doubao-openpencil/exports/openpencil-design-doubao-skill.zip`（或运行 `doubao-openpencil pack-skill` 现打）。

## 常用命令

| 命令 | 说明 |
| --- | --- |
| `doubao-openpencil setup` | 校验运行包、记录 Node 22+、执行 `host-check` 并导出连接器 JSON 与技能 ZIP |
| `doubao-openpencil print-config` | 输出标准 `mcpServers` JSON 配置 |
| `doubao-openpencil print-form` | 输出「手动添加本地 stdio 连接器」表单字段 |
| `doubao-openpencil pack-skill [--skill NAME] [--flat]` | 打包可直接上传至「我的技能」的 `.zip` 技能包 |
| `doubao-openpencil host-check` | 模拟 GUI 精简 `PATH` 校验启动器、工具与 UI 资源 |
| `doubao-openpencil canvas [--file PATH]` | 终端持久化画布模式 |
| `doubao-openpencil doctor [--port PORT]` | 诊断已运行的 OpenPencil 桌面端 MCP 服务 |
| `doubao-openpencil uninstall` | 从可选 `--mcp-config` / `--skills-dir` 安全卸载受管条目 |

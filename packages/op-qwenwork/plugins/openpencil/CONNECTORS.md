# 连接器说明

本套件通过 `.mcp.json` 与 `qwenwork-openpencil` CLI 提供 OpenPencil 本地 MCP 连接器。

## 连接器：openpencil

- **协议**：MCP (Stdio JSON-RPC 2.0)
- **启动器**：`scripts/launch-mcp`（Windows 为 `scripts/launch-mcp.cmd`）
- **运行环境**：Node.js 22+ 与对应的 OpenPencil Rust 编辑运行包（`op-host-web-server` + WASM + CanvasKit）
- **核心能力**：
  - `open_canvas` / `canvas_open_browser`：为当前会话拉起隔离的托管 Rust 编辑器与本地可交互画布 `panelUrl`
  - `canvas_status` / `canvas_selection`：读取文档版本、恢复草稿路径与当前选区节点
  - `canvas_import_image`：将本地 PNG/JPEG/WebP 资产直接嵌入图片节点，不在对话中传输 base64
  - `canvas_save` / `canvas_checkpoint`：受保护的原子保存与恢复草稿
  - `canvas_codegen_*`：代码面板多框架转代码闭环
  - 以及全部原生 OpenPencil 设计工具（`get_editor_state`、`batch_design`、`batch_get`、`snapshot_layout`、`lint_document`、`get_screenshot`、`codegen_export` 等）

## 账号隔离模式导入方式

若千问办公客户端处于账号隔离模式（如 macOS 千问办公客户端），可运行：

```sh
qwenwork-openpencil print-config
```

将输出的 JSON 粘贴到千问办公的 **扩展 → 连接器 → 添加 → 粘贴 JSON 配置**，并启用该连接器；同时可将 `qwenwork-openpencil pack-plugin` 生成的 `.zip` 套件导入 **扩展 → 套件 → 导入本地套件** 以加载设计 Skill。

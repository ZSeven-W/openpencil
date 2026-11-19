# Qoder OpenPencil 插件与连接器

**面向 Qoder（Qoder Desktop / Qoder CN / Qoder IDE & CLI）的 OpenPencil 原生 MCP Apps 插件与交互式 Rust 画布（`@zseven-w/qoder-openpencil`）。**

[English](./README.md) · [**简体中文**](./README.zh.md) · [安装指南](./INSTALL.md) · [架构说明](./docs/architecture.md)

## 核心特性

- **原生适配 Qoder MCP Apps 2026-01-26（`qoder/ui`）**：
  - 在 `open_canvas` 上声明 `global` 与 `thread` 入口，安装后可直接从左侧 **更多 → OpenPencil Canvas**（全屏独立工作区）或会话右侧栏 **+ → Apps → OpenPencil Canvas**（与对话并排）一键打开。
  - 在 `_meta.ui.csp.frameDomains` 中声明本地 `127.0.0.1` 画布源（适配 Qoder Desktop 的本地回环白名单），并声明 `availableDisplayModes: ["inline", "fullscreen"]` 与 `preferredDisplayMode: "fullscreen"`。
  - 支持明暗主题同步、通过 `ui/update-model-context`（带 `qoder/title` 与 `structuredContent`）同步选区上下文，以及代码面板 `ui/message` 交接。
- **完整覆盖 Qoder 插件注册与导入规范**：
  - `qoder-openpencil setup`：自动识别 `~/.qoder` / `~/.qoder-cn`（兼容 `QODER_CONFIG_DIR` / `QODERCN_CONFIG_DIR`），写入版本化本地缓存 `plugins/cache/local/openpencil/<version>/`，原子合并 `plugins/installed_plugins_v2.json` 并在 `settings.json` 启用 `enabledPlugins["openpencil@local"] = true`。
  - `qoder-openpencil pack-plugin`：零依赖生成符合 Qoder 规范的本地插件 `.zip`（`.qoder-plugin/plugin.json` 与 `${QODER_PLUGIN_ROOT}` 版 `.mcp.json` 直接位于 ZIP 根目录），支持界面 **扩展 → 导入本地插件** 或 `qoder-settings.install_extension`。
  - `qoder-openpencil setup --mcp-settings` / `print-config`：同时兼容 Qoder IDE / CLI 的全局 `settings.json` `mcpServers` + `skills/` 模式。
- **完整原生设计工具与受保护保存**：单一串行控制器代理所有 OpenPencil Rust MCP 工具，会话恢复草稿保存在 `~/.qoder-openpencil/sessions/`，保存前校验 SHA-256 基线。

## 快速开始

```sh
npm install -g @zseven-w/qoder-openpencil
qoder-openpencil setup
```

重启 Qoder（或在 **扩展 → 已安装** 刷新）即可在左侧 **更多** 或会话右侧栏 **+ → Apps** 中打开 **OpenPencil Canvas**。

## 常用命令

| 命令 | 说明 |
| --- | --- |
| `qoder-openpencil setup` | 校验运行包、注册并启用 `openpencil@local` 插件并执行 `host-check` |
| `qoder-openpencil pack-plugin [--output PATH]` | 打包符合 Qoder 根目录规范的本地插件 `.zip` |
| `qoder-openpencil print-config` | 输出适用于 `settings.json` 的 `mcpServers` JSON |
| `qoder-openpencil host-check` | 模拟 GUI 精简 `PATH` 校验启动器、`qoder/ui` 入口与 MCP App 资源 |
| `qoder-openpencil canvas [--file PATH]` | 终端持久化画布模式 |
| `qoder-openpencil doctor [--port PORT]` | 诊断已运行的 OpenPencil 桌面端 MCP 服务 |
| `qoder-openpencil uninstall` | 安全卸载 `openpencil@local`、`mcpServers.openpencil` 及受管 Skill |

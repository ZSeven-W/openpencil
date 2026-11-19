# OpenPencil Connector for Doubao Desktop (豆包办公 / 豆包电脑版)

This connector provides a local `stdio` MCP server (`openpencil`) that runs a managed Rust OpenPencil canvas per conversation, paired with an uploadable `SKILL.md` bundle for Doubao Desktop's **技能·连接器·伙伴** workspace.

## Prerequisites

- **Node.js 22+** (run `doubao-openpencil setup` once so `~/.doubao-openpencil/node-path` is recorded for GUI launches)
- **OpenPencil Rust editor runtime** (`@zseven-w/doubao-openpencil-<platform>`, staged development runtime, or `OPENPENCIL_EDITOR_RUNTIME_DIR`)
- No API keys or external cloud tokens are required; all canvas traffic stays on `127.0.0.1` with per-session capability tokens.

## Add to Doubao Desktop

1. **Connector (连接器)**: Open **技能·连接器·伙伴 → 连接器 → 手动添加 / 自定义连接器** (`本地命令 / stdio`), and paste either the JSON from `doubao-openpencil print-config` or the form values from `doubao-openpencil print-form`.
2. **Skill (技能)**: Run `doubao-openpencil pack-skill` (or use the ZIP exported by `doubao-openpencil setup`), then upload it under **技能·连接器·伙伴 → 我的技能 → 新建 → 上传技能**.

# Install OpenPencil Connector for Doubao Desktop (豆包办公 / 豆包电脑版)

## Prerequisites

- **Node.js 22+**
- **Doubao Desktop (豆包电脑版)** with Work Mode (**工作任务 / 技能·连接器·伙伴**)
- No Rust toolchain or standalone OpenPencil desktop app is required when installing `@zseven-w/doubao-openpencil` from npm (its matching platform optional package carries `op-host-web-server`, WASM, and CanvasKit).

## Install from npm

```sh
npm install -g @zseven-w/doubao-openpencil
doubao-openpencil setup
```

## Install from a monorepo checkout

```sh
git clone https://github.com/ZSeven-W/openpencil.git
cd openpencil/packages/op-doubao
npm run build:editor-runtime && npm run stage:editor-runtime
# Or point at an existing verified runtime root:
# npm run configure-runtime -- /absolute/path/to/runtime
npm run setup
```

## What `setup` does

1. Resolves and SHA-256 verifies the atomic Rust OpenPencil editor runtime (`op-host-web-server` + `web-bundle` + `canvaskit`).
2. Runs an isolated live verification: loads the local panel, performs a native `update_node` edit, saves to `.op`, and reopens the file.
3. Privately records your Node 22+ executable path in `~/.doubao-openpencil/node-path` so Doubao's GUI process works even when its `PATH` omits nvm/fnm/volta/Homebrew.
4. Runs `host-check` under a stripped GUI `PATH` to verify MCP `initialize`, `tools/list`, and `resources/list`.
5. Exports both ready-to-import files under `~/.doubao-openpencil/exports/`:
   - `doubao-connector.json` (ready-to-paste `mcpServers` JSON)
   - `openpencil-design-doubao-skill.zip` (ready-to-upload Skill ZIP)

## Connect inside Doubao Desktop (豆包电脑版)

### Step 1: Add the local `stdio` Connector (连接器)

Open Doubao Desktop → **技能·连接器·伙伴 → 连接器 → 手动添加 / 自定义连接器**:
- **If the UI accepts JSON**: paste the output of `doubao-openpencil print-config`.
- **If the UI uses form fields (`本地命令 / stdio`)**: run `doubao-openpencil print-form` and fill in `名称`, `传输方式`, `命令`, and `工作目录` (`参数` and `环境变量` stay empty).

Save, enable **openpencil**, and reload/restart Doubao Desktop if prompted.

### Step 2: Upload the OpenPencil Design Skill (上传技能)

1. Open Doubao Desktop → **技能·连接器·伙伴 → 我的技能 → 新建 → 上传技能**.
2. Drag in `~/.doubao-openpencil/exports/openpencil-design-doubao-skill.zip` (or build a custom copy via `doubao-openpencil pack-skill [--flat] --output ./openpencil-design.zip`).
3. In any Doubao Work Task chat, type `/openpencil-design` or ask Doubao to design with OpenPencil (`用op设计...`).

## Optional direct file merge

If you manage a local MCP JSON file or skills folder directly:

```sh
doubao-openpencil setup --mcp-config /path/to/mcp.json --skills-dir /path/to/skills
```

## Diagnostics & Uninstall

```sh
doubao-openpencil host-check
doubao-openpencil doctor
doubao-openpencil canvas --file /absolute/path/design.op
doubao-openpencil uninstall --mcp-config /path/to/mcp.json --skills-dir /path/to/skills
```

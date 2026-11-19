# Install and start OpenPencil with Codex

Give Codex this prompt (replace the example with your own design request):

> Install and set up `@zseven-w/codex-openpencil` following https://github.com/ZSeven-W/openpencil/blob/main/packages/op-codex/INSTALL.md, open an editable canvas, and create a single-page mobile task app.

中文：

> 按照 https://github.com/ZSeven-W/openpencil/blob/main/packages/op-codex/INSTALL.md 帮我安装并配置 `@zseven-w/codex-openpencil`，打开可编辑画布，做一个单页手机任务管理 APP。

## Instructions for the installing agent

The user wants a working canvas and a design, so continue through verification
and opening the canvas. This is a local install from npm; no GitHub account,
`git clone`, API key, DSH, Rust compiler, or separate OpenPencil desktop app is needed.

1. Check for Node.js 22+ (with npm) and a recent Codex CLI with `plugin marketplace`
   and `plugin add` commands. If a prerequisite is missing, install it from its
   official distribution using the user's normal package manager when authorized
   by their install request. Use the official [Node.js](https://nodejs.org/en/download)
   and [Codex CLI](https://developers.openai.com/codex/cli) distributions.
   Preserve existing Codex settings, models and unrelated MCP servers.
   Setup records the current Node executable privately so the desktop host can
   launch it even when its PATH differs from the terminal. Linux also needs fontconfig
   and FreeType runtime libraries (`libfontconfig1` and `libfreetype6` on Ubuntu).
   Windows needs the matching [Microsoft Visual C++ v14 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist).
   The native package imports `MSVCP140.dll` and `VCRUNTIME140.dll`; check that the
   matching runtime is installed and install Microsoft's signed package if absent.
   These are system prerequisites the installing agent should resolve before
   proceeding, rather than asking the user to find a Rust binary manually.
2. Install the published npm package globally (without `--omit=optional`, so npm
   selectively installs the matching `@zseven-w/codex-openpencil-<os>-<cpu>` native
   runtime package) and run setup:

   ```sh
   npm install -g @zseven-w/codex-openpencil@latest
   codex-openpencil setup
   ```

   Do not use `--omit=optional` or `--no-optional`: npm uses `optionalDependencies`
   with `os`, `cpu`, and Linux `libc` selectors so only the single runtime matching
   your OS and CPU is downloaded.

   `codex-openpencil setup` verifies every payload file SHA-256 against the
   package's `openpencil-runtime.json`, proves panel startup, a native edit, save,
   and reopen using an isolated test document, records the verified runtime root
   in `~/.codex-openpencil/runtime.json`, registers the installed npm package as
   the local `zseven-openpencil` marketplace, and installs `openpencil@zseven-openpencil`.
   It then starts Codex's actual plugin loader and verifies the canvas controls.
   Success must contain `"ok": true`, `"pluginInstalled": true`, and
   `"pluginHost": { "ok": true, ... }`; do not report success after a failed
   prerequisite, install, smoke test, plugin command or host check. This check
   opens no chat and does not edit a design. `OPENPENCIL_CODEX_BIN` can identify
   a non-PATH Codex executable.

   If the user previously registered `zseven-openpencil` from a local git clone,
   remove the old marketplace source first (`codex plugin marketplace remove zseven-openpencil`)
   and rerun `codex-openpencil setup`.
3. Prefer `canvas_open_browser` in Codex, then show its `panelUrl` with
   `open_in_codex` (browser target) or `@Browser`. Use an external browser only
   when the user explicitly requests one. Codex currently
   filters HTTP frame origins from its MCP Apps sandbox, including loopback;
   do not embed this URL or ask users to weaken their security settings.
   The **＋ → OpenPencil Canvas** launcher asks the current Codex chat to open
   the same canvas in its built-in browser when host text messaging is available.
   It waits for a real editor-ready receipt. Hosts without messaging show a
   copyable opening instruction to send in the current chat. It does not launch
   the system browser.
   If only `open_canvas` is available in the current chat, call it and show its
   returned `panelUrl` in the browser. Verify
   `canvas_status` plus `get_editor_state` before designing. Read the installed
   `openpencil-design` skill and its node schema before editing. If this chat has
   not loaded the new plugin yet, **open a canvas now** with the persistent
   terminal entry point:

   ```sh
   codex-openpencil canvas
   # Existing document (copied into a recovery session):
   codex-openpencil canvas --file /absolute/path/design.op
   ```

   Run this in a persistent terminal/PTY and keep stdin open. Wait for its
   `ready: true` JSON line. Show `panelUrl` in the user's normal Codex browser
   panel or authenticated browser. Its random loopback URL is temporary and local.
   The process accepts one JSON request per line through stdin, so the agent can
   create the requested design **in this chat** before a plugin reconnect:

   ```json
   {"method":"tools/list"}
   {"name":"get_editor_state","arguments":{}}
   {"name":"canvas_selection","arguments":{}}
   ```

   Inspect the returned native tool schemas before using `batch_design`,
   `update_node`, layout inspection or rendering tools. They use the same Rust
   document as the visible canvas. Read the result of every write; never replay
   a failed or timed-out mutation automatically. Do not start a second canvas
   for the same design while this process is active. Native screenshot results
   are written to private image files and returned as `imagePath`; open those
   files with the agent's image viewer for visual QA.
4. Complete the user's requested design, inspect layout and preview it, save to
   an absolute `.op` path with `canvas_save`, and read the saved result back.
   A fresh demonstration should be a single page unless the user requested
   multiple pages; clarify page count for a substantive brief as described by
   the design skill. Leave the useful canvas process running and report its
   panel URL and saved file. `{"quit":true}` closes only that owned session;
   recovery drafts survive process shutdown.
5. State separately whether the current chat uses the installed MCP plugin or
   the terminal entry point. Codex may require reconnecting the plugin or a new
   chat to load newly installed tools. Do not claim those tools appeared in the
   current chat without verifying them. After reconnect, use `canvas_open_browser` with
   the saved `.op` file to resume the design.

## Supported runtime builds

Each release publishes `@zseven-w/codex-openpencil` plus six atomic native
platform packages (`@zseven-w/codex-openpencil-<os>-<cpu>`):

| System | CPUs | Platform package | Compatibility floor |
| --- | --- | --- | --- |
| macOS | Apple Silicon | `@zseven-w/codex-openpencil-darwin-arm64` | Built on macOS 15 native runner |
| macOS | Intel | `@zseven-w/codex-openpencil-darwin-x64` | Built on macOS 15 Intel native runner |
| Windows | ARM64 | `@zseven-w/codex-openpencil-win32-arm64` | MSVC build; Windows 11 ARM runner |
| Windows | x64 | `@zseven-w/codex-openpencil-win32-x64` | MSVC build; Windows Server 2025 x64 runner |
| Linux | x64 | `@zseven-w/codex-openpencil-linux-x64` | glibc (`>= 2.35`), Ubuntu 22.04 or newer |
| Linux | ARM64 | `@zseven-w/codex-openpencil-linux-arm64` | glibc (`>= 2.39`), Ubuntu 24.04 or newer |

Alpine/musl and other architectures skip the optional glibc package automatically
and need a custom complete runtime. Unsupported platforms fail explicitly; the
resolver never substitutes a different binary.

## Repair, update, and manual runtime

- To update to the latest release:
  ```sh
  npm install -g @zseven-w/codex-openpencil@latest
  codex-openpencil setup
  ```
  Reconnect the installed plugin afterward (toggle it off/on in plugin management,
  or restart Codex). Closing only the canvas tab can retain the previous running
  bridge and UI resource. Valid custom runtimes remain selected.
- If the desktop reports `No such file or directory` while the terminal works,
  rerun `codex-openpencil setup` to refresh the private Node registration, then run
  `codex-openpencil host-check`.
  The launcher reads `~/.codex-openpencil/node-path`, independently of a custom
  runtime state directory. It never changes your system PATH or another Node install.
  A passing host check proves a fresh Codex connection can load the tools; verify
  tool discovery separately in the current chat after reconnecting.
- For an invalid configuration, inspect it first. `codex-openpencil setup --repair-runtime`
  explicitly re-selects the installed npm platform runtime package.
- `codex-openpencil setup --runtime-only` verifies and activates the runtime without
  changing Codex plugin registration.
- `codex-openpencil configure-runtime /absolute/complete/runtime` selects a custom
  runtime. An existing valid choice is preserved by normal setup and first use.
- Developing from a monorepo checkout: stage the local runtime into
  `packages/op-codex/npm/<platform>` with `npm run stage:editor-runtime` (or set
  `OPENPENCIL_EDITOR_RUNTIME_DIR`), then run `npm run setup` / `npm run canvas`
  inside `packages/op-codex/`.

Private configuration lives under `~/.codex-openpencil/`; override its location with
`OPENPENCIL_CODEX_STATE_DIR`. Setup never reads or edits a user's live OpenPencil
document during its smoke test.

# Install and start OpenPencil with Codex

Give Codex this prompt (replace the example with your own design request):

> Install and set up https://github.com/ZSeven-W/openpencil following packages/op-codex/INSTALL.md, open an editable canvas, and create a single-page mobile task app.

中文：

> 按照 https://github.com/ZSeven-W/openpencil 的 packages/op-codex/INSTALL.md 帮我安装并配置 OpenPencil，打开可编辑画布，做一个单页手机任务管理 APP。

## Instructions for the installing agent

The user wants a working canvas and a design, so continue through verification
and opening the canvas. This is a local install; no GitHub account, API key, DSH,
Rust compiler, npm dependency install, or separate OpenPencil desktop app is needed.

1. Check for Node.js 22+, Git, and a recent Codex CLI with `plugin marketplace`
   and `plugin add` commands. If a prerequisite is missing, install it from its
   official distribution using the user's normal package manager when authorized
   by their install request. Use the official [Node.js](https://nodejs.org/en/download),
   [Git](https://git-scm.com/downloads), and [Codex CLI](https://developers.openai.com/codex/cli)
   distributions. Preserve existing Codex settings, models and unrelated MCP servers.
   Setup records the current Node executable privately so the desktop host can
   launch it even when its PATH differs from the terminal. Linux also needs fontconfig
   and FreeType runtime libraries (`libfontconfig1` and `libfreetype6` on Ubuntu).
   Windows needs the matching [Microsoft Visual C++ v14 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist).
   The native package imports `MSVCP140.dll` and `VCRUNTIME140.dll`; check that the
   matching runtime is installed and install Microsoft's signed package if absent.
   These are system prerequisites the installing agent should resolve before
   proceeding, rather than asking the user to find a Rust binary manually.
2. Clone the public repository over HTTPS into a persistent, user-owned location.
   Do not overwrite another checkout or choose a temporary installation directory.
   For an existing checkout, inspect local changes before updating it.

   ```sh
   git clone https://github.com/ZSeven-W/openpencil.git
   cd openpencil/packages/op-codex
   npm run setup
   ```

   If Git cannot reach GitHub through the user's existing network, inspect the
   normal proxy settings and pass that HTTP proxy to this process with
   `HTTPS_PROXY`. Do not replace the user's global Git or system proxy settings.

   There are no npm runtime dependencies. `npm run setup` acquires the pinned
   complete runtime for the local OS/CPU, verifies the archive SHA-256 and every
   payload file, then proves panel startup, a native edit, save, and reopen using
   an isolated test document. It finally registers this persistent checkout as the local marketplace and
   installs `openpencil@zseven-openpencil`. It then starts Codex's actual plugin
   loader and verifies the canvas controls. Success must contain `"ok": true`,
   `"pluginInstalled": true`, and `"pluginHost": { "ok": true, ... }`; do not report
   success after a failed prerequisite, download, smoke test, plugin command or
   host check. This check opens no chat and does not edit a design. `OPENPENCIL_CODEX_BIN` can identify a
   non-PATH Codex executable. No runtime is downloaded during tool discovery.
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
   not loaded the new plugin yet, **open a canvas now** with the plugin package's
   persistent terminal entry point (from `packages/op-codex`):

   ```sh
   npm run canvas
   # Existing document (copied into a recovery session):
   npm run canvas -- --file /absolute/path/design.op
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

| System | CPUs | Compatibility floor |
| --- | --- | --- |
| macOS | Apple Silicon, Intel | Built on macOS 15 native runners |
| Windows | ARM64, x64 | MSVC builds; Windows 11 ARM / Windows Server 2025 x64 runners |
| Linux | x64 | glibc, Ubuntu 22.04 or newer |
| Linux | ARM64 | glibc, Ubuntu 24.04 or newer |

Alpine/musl and other architectures need a custom complete runtime. Unsupported
platforms fail explicitly; the installer never substitutes a different binary.

## Repair, update, and manual runtime

- Rerun `npm --prefix packages/op-codex run setup` (or `npm run setup` inside `packages/op-codex`) after a network interruption. It reuses a verified cache.
- If the desktop reports `No such file or directory` while the terminal works,
  rerun setup to refresh the private Node registration, then run `npm run host-check` from `packages/op-codex`.
  The launcher reads `~/.codex-openpencil/node-path`, independently of a custom
  runtime state directory. It never changes your system PATH or another Node install.
  A passing host check proves a fresh Codex connection can load the tools; verify
  tool discovery separately in the current chat after reconnecting.
- For an invalid configuration, inspect it first. `npm run setup -- --repair-runtime` (from `packages/op-codex`)
  explicitly selects the published runtime. It preserves the previous runtime
  directory; a corrupted immutable cache must be moved aside before a clean retry.
- `npm run setup -- --runtime-only` installs/verifies the runtime without changing
  Codex plugin registration.
- `npm run configure-runtime -- /absolute/complete/runtime` selects a custom
  runtime. An existing valid choice is preserved by normal setup and first use.
- To update, inspect the checkout, pull the published changes and rerun setup.
  Managed downloads follow the new pinned release; valid custom runtimes remain
  selected. An interrupted upgrade retains the previous runtime and configuration.
  Reconnect the installed plugin afterward (toggle it off/on in plugin management,
  or restart Codex). Closing only the canvas tab can retain the previous running
  bridge and UI resource. Do not delete unrelated settings.

Runtime downloads use this repository's pinned GitHub Release assets. Private
configuration lives under `~/.codex-openpencil/`; override its location with
`OPENPENCIL_CODEX_STATE_DIR`. The installer never reads or edits a user's live
OpenPencil document during its smoke test.

Downloads honor `HTTPS_PROXY` / `https_proxy`, `NO_PROXY`, and an enabled macOS
HTTPS system proxy without changing system settings. SOCKS-only environments
should expose their HTTP proxy endpoint via `HTTPS_PROXY`.

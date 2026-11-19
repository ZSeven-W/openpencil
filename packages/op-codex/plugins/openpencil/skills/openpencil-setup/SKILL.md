---
name: openpencil-setup
description: Connect and diagnose the local Rust OpenPencil editor for the Codex OpenPencil plugin. Use for initial setup, missing tools, or a disconnected local canvas.
---

# Connect OpenPencil

The plugin opens a managed interactive Rust canvas or connects to an existing
desktop editor. The matching complete native editor, editable WASM, and CanvasKit
runtime is distributed via the platform-selective `@zseven-w/codex-openpencil-<os>-<cpu>`
npm optional dependency and verified (SHA-256 of every payload file, manifest,
version, revision and platform) before launch.

## Interactive canvas

1. In Codex, call `canvas_open_browser`, optionally with the user's existing
   absolute `.op` path, then open its `panelUrl` in the Codex browser panel
   (`open_in_codex`, browser target) or `@Browser`. Use an external browser only
   when the user explicitly requests one. The source
   is copied into a recovery draft. If only `open_canvas` is available, its
   returned `panelUrl` works the same way. Codex currently filters local HTTP
   iframe origins, so its MCP Apps entrypoint shows a browser launcher. Do not
   request weaker browser security or claim an embedded canvas was verified.
   Its button prepares the same canvas URL and, where host text messaging is
   supported, sends one opening request to the current chat through `ui/message`.
   Open the supplied exact URL with `open_in_codex` (browser target) or `@Browser`;
   preserve its theme/launch fragment so the panel can confirm editor readiness.
   Do not create a second canvas, launch an OS browser or use a deep-link workaround.
   Without host messaging, the user copies an opening instruction into the current
   chat. A message acknowledgment is not a connected canvas. `ui/open-link`
   acknowledgments also do not prove navigation: the current bridge drops HTTP URLs.
2. Runtime configuration is recorded privately in `~/.codex-openpencil/runtime.json`
   by `codex-openpencil setup` (pointing to the installed npm platform package, or
   a user-configured custom runtime). If the runtime is missing or drifted after
   upgrading `@zseven-w/codex-openpencil`, run:
   `npm install -g @zseven-w/codex-openpencil@latest && codex-openpencil setup`,
   then reconnect the plugin and call `canvas_open_browser`.
   Existing invalid configurations are reported without switching runtimes; use
   `codex-openpencil setup --repair-runtime` to explicitly re-select the installed
   npm platform package. A complete custom package can be selected with
   `codex-openpencil configure-runtime /absolute/runtime/root` or
   `OPENPENCIL_EDITOR_RUNTIME_DIR`; it needs a matching native server, editable
   WASM, CanvasKit and integrity manifest. A desktop binary alone is insufficient.
3. Use the UI resource/thread entrypoint, or show the returned local `panelUrl`
   in hosts without MCP Apps. Verify `canvas_status` and `get_editor_state`:
   they must describe the same managed document before editing.
4. Each conversation keeps one canvas. Use another chat for a different document.
   Recovery drafts survive in `~/.codex-openpencil/sessions/`. A stopped session's
   URL is temporary; reopen its recovery file in a new session.
5. Reconnect or open a new chat after upgrading the plugin. Resource loading and
   visibly displayed embedded host UI are separate verification steps.

For a new install, follow
[INSTALL.md](https://github.com/ZSeven-W/openpencil/blob/main/packages/op-codex/INSTALL.md).
Installing `@zseven-w/codex-openpencil` globally via npm (without `--omit=optional`)
and running `codex-openpencil setup` verifies an isolated native edit/save/reopen
before installing the plugin, registers the current Node executable privately, and
verifies canvas tools through Codex's actual plugin loader. For a desktop startup
error such as `No such file or directory` despite a working terminal, rerun
`codex-openpencil setup` and `codex-openpencil host-check`. Do not rewrite
unrelated MCP settings.
A successful fresh host check does not prove this existing chat has reloaded.
If this chat has not loaded newly installed MCP tools yet,
`codex-openpencil canvas` provides the same editable canvas and accepts
native tool requests through a persistent terminal's stdin. Use that entry point
to complete the design now, then explain that reconnect loads the installed tools.
Never mistake a successful install for a verified live canvas.

## Existing desktop editor

1. Try `get_editor_state` through this plugin's OpenPencil tools. A response proves
   the actual connection; an installed package alone does not.
2. If unavailable, run `codex-openpencil doctor` (or `node scripts/mcp-bridge.mjs --doctor`
   from the installed plugin directory, two levels above this file). The doctor
   reads the editor's `~/.openpencil/.op-mcp-port` and verifies the instance identity.
   The plugin's launcher uses the Node.js 22+ executable registered by setup, then
   known local Node locations if needed; it never starts a managed runtime to solve
   discovery.
3. For a missing editor, use the official [OpenPencil installation instructions](https://github.com/ZSeven-W/openpencil).
   When launch is within the user's request, run `op start`, then enable MCP
   in the editor's Agent Settings. An old `op` binary may not discover the current
   Rust editor; trust the bridge doctor and editor state over that binary's status.
4. For an intentional headless/web endpoint without a live discovery record,
   set `OPENPENCIL_MCP_PORT` in the MCP process environment or append
   `--port PORT` to the bundled server args. Only local loopback ports are accepted.
   Do not guess a port or rewrite unrelated Codex server configuration.
5. If MCP discovery failed at session startup, reconnect/reload the plugin or
   start a new chat after the editor is reachable. Verify `get_editor_state`
   again and report the active document/page before editing.

A stale identity error requires reopening the intended editor or fixing its MCP
setting. The bridge refuses to silently fall back to a different canvas. Keep
discovery tokens out of chat, logs, committed files, and reports.

After either canvas opener, all native tools target that managed session instead of the
discovered desktop editor. Report the target before edits; never silently take
over a different document.

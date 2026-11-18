---
name: openpencil-setup
description: Connect and diagnose the local Rust OpenPencil editor for the Codex OpenPencil plugin. Use for initial setup, missing tools, or a disconnected local canvas.
---

# Connect OpenPencil

The plugin opens a managed interactive Rust canvas or connects to an existing
desktop editor. First canvas use automatically downloads the matching complete
runtime from pinned public GitHub Release assets and validates its SHA-256,
payload manifest, version and platform before configuring or starting it.

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
2. Missing runtime configuration triggers installation automatically. Allow the
   initial download to finish; do not repeatedly call either opener after a host
   timeout. For preflight or a slow connection, run
   `node scripts/runtime-install.mjs` from the installed plugin directory, then
   call `canvas_open_browser`. Failed downloads do not configure partial runtimes.
   Existing invalid configurations are reported without switching runtimes.
   Configuration stays private in `~/.codex-openpencil/runtime.json`.
   A complete custom package can be selected with
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

For a new install, follow the monorepo's
[INSTALL.md](https://github.com/ZSeven-W/openpencil/blob/main/packages/op-codex/INSTALL.md).
Its `npm run setup` (run from `packages/op-codex`) verifies an isolated native edit/save/reopen before installing
the plugin, registers the current Node executable privately, and verifies canvas
tools through Codex's actual plugin loader. For a desktop startup error such as
`No such file or directory` despite a working terminal, rerun setup and
`npm run host-check` from that repository. Do not rewrite unrelated MCP settings.
A successful fresh host check does not prove this existing chat has reloaded.
If this chat has not loaded newly installed MCP tools yet,
`npm run canvas` in the repository provides the same editable canvas and accepts
native tool requests through a persistent terminal's stdin. Use that entry point
to complete the design now, then explain that reconnect loads the installed tools.
Never mistake a successful install for a verified live canvas.

## Existing desktop editor

1. Try `get_editor_state` through this plugin's OpenPencil tools. A response proves
   the actual connection; an installed package alone does not.
2. If unavailable, run `node scripts/mcp-bridge.mjs --doctor` from the installed
   plugin directory (two levels above this file). The doctor reads the editor's
   `~/.openpencil/.op-mcp-port` and verifies the instance identity. The plugin's
   launcher uses the Node.js 22+ executable registered by setup, then known local
   Node locations if needed; it never downloads a runtime to solve discovery.
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

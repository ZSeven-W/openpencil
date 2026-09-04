# OpenPencil for VS Code

Design-as-code inside your editor — open, view, and edit OpenPencil `.op` files, wire your AI tools to the OpenPencil MCP server, and generate framework code from a design.

## Features

- **Custom `.op` editor** — `.op` design files open in a live OpenPencil canvas hosted in a VS Code webview, backed by a local OpenPencil daemon. Edits are tracked as document changes, so save, save-as, revert, and hot-exit backup all work like a native editor.
- **Live preview** — use the Play control in the embedded OpenPencil top bar to enter the same interactive Preview mode as the desktop/web editor; the canvas stays in the VS Code tab instead of opening a separate renderer.
- **OpenPencil account** — the account control uses the managed daemon's device-login flow. VS Code opens the verification page in the system browser, while credentials remain in OpenPencil's native auth store rather than extension settings or webview storage.
- **Collaboration** — after signing in, the embedded collaboration control starts or joins the same LAN/public-relay sessions as the native editor. Presence, document operations, reconnect state, and read-only fallback are owned by the shared Rust collaboration runtime.
- **`.fig` import** — opening a `.fig` file converts it into design content on the fly; saves land on the sibling `.op` file (for example `design.fig` → `design.op`) rather than writing back to the `.fig`.
- **MCP configuration** — the extension runs a local MCP proxy and can write the connection into your IDE's MCP config, so an AI assistant can drive OpenPencil through the Model Context Protocol. Commands: *Configure MCP* and *Remove MCP*.
- **AI skill install** — install (or remove) the bundled OpenPencil design skill into your IDE's rules directory (Cursor / Trae / Windsurf). Commands: *Install Skill* and *Remove Skill*. The built-in VS Code chat participant is currently a read-only design advisor; it does not yet run the complete create/inspect/finalize pipeline.
- **Code generation** — generate React or Vue code from the active design. When VS Code's language-model API is available it runs directly; otherwise it hands off a pre-filled prompt to your IDE's chat. Command: *Generate Code*. The target framework is configurable (`react` or `vue`).
- **`@openpencil` chat participant** — an OpenPencil design assistant registered in VS Code chat.
- **Workspace-trust aware** — in an untrusted workspace `.op` files show a read-only placeholder and no local daemon or MCP config is started; granting trust assembles the full stack and reopens the files.

## Requirements

- **VS Code** `^1.90.0` or newer.
- The published platform VSIX already includes the matching native
  **`op-host-web-server` runtime and Web bundle**. Source checkouts may instead
  use `<workspace>/target/debug/op-host-web-server`, or set
  `openpencil.dev.daemonPath` to another development build. The extension does
  not require the OpenPencil desktop application. A trusted workspace is
  required to start the daemon and write MCP config.

## Getting Started

1. Trust the workspace when prompted (required for the live editor and MCP).
2. Open any `.op` file — it opens in the OpenPencil canvas editor. Opening a `.fig` file imports it and edits are saved to the sibling `.op`.
3. Use the account avatar in the canvas top bar to sign in. The approval page opens in your system browser and the editor updates when the device flow completes.
4. Use **Play** for an interactive preview, or **Collaborate** to start/join a LAN or public-relay session.
5. Run **OpenPencil: Configure MCP** from the Command Palette to register the local MCP proxy with your AI tooling.
6. Run **OpenPencil: Generate Code** to produce React or Vue code from the current design, or chat with **@openpencil** in the VS Code chat view.

## Settings

- `openpencil.dev.daemonPath` — optional path to a development `op-host-web-server`; an empty value prefers the VSIX-bundled runtime, then probes the workspace `target/debug` build.
- `openpencil.proxy.port` — port for the local MCP proxy (`0` auto-selects).
- `openpencil.codegen.framework` — target framework for generated code (`react` or `vue`).

## Host boundaries

The extension does not implement a second account or collaboration stack. It
supervises the bundled native daemon and embeds the shared Rust web editor. The
webview relay accepts only typed control messages; external navigation is
limited to HTTPS (plus loopback HTTP for a development SSO), and collaboration
traffic remains token-authenticated between the webview and its per-document
managed daemon.

The stable MCP proxy currently targets an already-open design. Creating a new
`.op` from an empty chat, publishing staged screenshots/final PNG, and opening
the resulting editor exactly once still need a host-neutral pipeline core plus
a VS Code adapter; they should not be implemented by copying DSH-specific
session or presentation code into this extension.

## Learn More

Part of the [OpenPencil](https://github.com/ZSeven-W/openpencil) project — an open-source, AI-native, design-as-code vector tool.

## License

MIT

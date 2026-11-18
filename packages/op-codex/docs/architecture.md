# Architecture and scope

```text
Codex plugin host -> Node.js stdio bridge -> one serialized controller
  before either canvas opener -> instance-verified existing Rust MCP editor
  after either canvas opener  -> owned managed Rust daemon -> shared document

Codex browser panel -> capability-protected loopback panel -> Rust WASM editor
                         Save/Save As/checkpoint               live sync
                                |                               |
                                +------ managed daemon ---------+

MCP Apps resource -> browser launcher in Codex
                 -> embedded loopback panel when host frame policy permits
```

The Rust engine owns native design tools, layouts, rendering, undo and live sync.
The plugin adds `open_canvas`, `canvas_open_browser`, `canvas_status`, `canvas_selection`, `canvas_save`,
model-only `canvas_import_image` and app-only `canvas_checkpoint`. It reuses the complete editable engine.
Native images and JSON-RPC errors pass through.

## Process and document ownership

Reading the UI resource starts only a lightweight panel server; it never switches
the active document. Opening a canvas verifies every runtime manifest SHA-256
entry, platform and release. If configuration is absent, the pinned platform
archive downloads on demand through the existing HTTPS proxy settings. The
installer bounds download and extraction sizes, rejects links/traversal, checks
archive and file hashes, and activates a complete runtime atomically. Existing
valid custom runtimes remain selected; invalid configurations fail explicitly.
The daemon launches with `--serve-web --managed
--port 0` on a recovery copy, its own supervisor lease and settings root. The
bounded handshake must match the bundle version. Discovery does not download or
start the Rust editor.

While managed, all native tools route to the owned daemon; browser edits and
Codex therefore share a document. `save_document` uses the guarded save. One
conversation cannot silently replace its canvas with another file. On EOF or
shutdown, the bridge checkpoints and stops only its own child. Recovery drafts
persist; reopen them in a new chat after a stopped session.

Before opening a canvas, desktop discovery verifies the `openpencil-mcp` marker
and live instance token for each request. Explicit loopback ports verify the
marker. Stale records, redirects, legacy REST and unrelated listeners are rejected.
No failed mutation is replayed automatically.

## UI and persistence contract

`ui://openpencil/canvas/v0.3.13.html` is `text/html;profile=mcp-app`. `open_canvas`
advertises `_meta.ui.resourceUri` and the `openai/ui` thread entrypoint. The wrapper
negotiates `ui/initialize`, handles tool results and sends selection context when
supported. Its CSP permits only this panel's exact origin. The panel checks an
opaque 256-bit URL capability, Host and Origin, and accepts only the declared panel controls.
Codex currently removes HTTP origins from the sandbox's frame allowlist. In its
`codex-sandbox:` surface, the wrapper therefore renders a browser launcher rather
than navigating a blocked iframe. `canvas_open_browser` is a separate model-facing
opener without UI-entrypoint metadata; it returns the same managed session and
URL even when Codex hides `open_canvas` from model tool discovery. The installing
agent opens that URL in the existing Codex browser panel. Other hosts can embed
only when their effective frame policy permits the local origin.
The launcher calls app-only `canvas_prepare_browser`, which returns only the
controller-owned loopback URL and a two-minute ready receipt. It accepts a theme,
never a caller-provided URL or command, and never launches a browser. When the
host advertises text messaging, one standard `ui/message` asks the current Codex
chat to open that exact URL with its built-in browser interface. Without that
capability, the user copies an opening instruction into the current chat.
`canvas_launch_status` stays pending until the native editor is ready and its
local panel submits the matching receipt. A message acknowledgment alone is not
success. An uncertain handoff is never automatically resent, and no OS opener,
deep link, sandbox-policy change or external-browser fallback is used.
The iframe lifecycle token stays in private UI state, outside model-facing text.
Editor messages require the exact frame source and origin. No global MCP config
or anonymous browser is created.

Plugin chrome mirrors `Theme::light()` and `Theme::dark()` in the pinned Rust
runtime's `crates/op-editor-ui/src/theme.rs`: neutral surfaces and borders, blue
interactive accents, and the editor's success/destructive colors. Review these
values in `ui/panel.css` and the loading wrapper whenever the runtime catalog
changes. The host theme selects the palette; design document colors are separate.

The current Codex chat owns instructions, edit progress and results. The panel
shows only selected count/names in its existing footer, with no extra selection
row, prompt form or edit-request lifecycle. The wrapper deduplicates selection
context updates to capable hosts and clears stale targets without sending a user
message. Codex verifies the current selection through `canvas_selection`, reads
those nodes and edits their subtrees, then inspects the preview and saved design.

## Image acquisition

The design skill defaults new raster assets to the host's built-in `image_gen`.
Before generation, Codex sends a brief chat progress notice explaining the intended
assets and possible quota use, then proceeds without a separate per-call permission
question. Explicit no-generation/stock-only requests take precedence. Unavailable
or failed generation uses scoped native stock-search fallback; Generate-only slots
must first be converted to Search for that route. Uncertain generation calls are
not replayed, and unsuccessful stock search does not loop back to generation.
The bridge never invokes a generation provider, configures an API key or tracks
credits. Host tools retain their own availability, approvals and accounting.

`canvas_import_image` is model-only and excluded from the panel RPC allowlist. It
reads a regular local raster file up to 8 MiB, checks the expected document version,
active page and existing image target, then performs one native node patch. Image
bytes are embedded directly without exposing base64 to the conversation. The
readback must match; unknown writes are never retried. Geometry and unrelated nodes
are preserved, acquisition hints are cleared, and imports do not save the source
file. The skill persists generated originals in the project, then inspects the
native preview, saves the `.op` and verifies portable image data on reopen.
Version/page checks precede the write; they are not an atomic compare-and-swap
against simultaneous interactive edits.

## Save and recovery

The panel flushes the editor snapshot gate before Save. The controller compares
that disk snapshot to the daemon's document, resolving only schema-owned image
references and checking page/geometry metadata. Text, variables and free-form
state are never normalized away. Stale snapshots stop the save. Saved UI snapshots
retain compact images and editor metadata; native saves and recovery checkpoints
preserve daemon page/geometry metadata.

Existing sources are SHA-256 checked before staging and again before atomic
replacement. Save As creates exclusively and refuses an existing destination.
A later edit keeps the UI dirty when an earlier save completes. Source conflicts
preserve external content and recovery drafts. These checks are not a distributed
lock against an external write between the final check and rename. Model-triggered
Save uses the daemon snapshot; use the UI Save button to explicitly flush pending
interactive edits first.

Stdio and panel controls share an ordered queue. Reads/requests are bounded at
32 MiB. Identity probes have a 3-second deadline, normal MCP requests 120 seconds,
startup 20 seconds and UI saves 15 seconds. A timeout never retries an edit.

## Ownership and validation boundaries

The portable MCP entry uses an executable launcher (with a Windows `.cmd`
counterpart). Setup atomically records `process.execPath` in the private
`~/.codex-openpencil/node-path` file. The launcher validates Node 22+ before
starting the bridge, retaining literal paths with spaces and shell characters.
It does not rely on desktop login-shell PATH import or change global settings.
After registration, setup starts `codex app-server` and checks
`mcpServerStatus/list` for the plugin's canvas tools. This uses the actual plugin
manifest/cache/launcher path, opens no chat and performs no design mutation.
It remains distinct from the loaded tools and visible UI in an existing chat.

- `ZSeven-W/openpencil`: Rust engine, schemas, renderer and persistence.
- `ZSeven-W/codex-openpencil`: plugin, session adapter, UI and workflows.
- `ZSeven-W/dsh-openpencil`: DSH integration and compatible runtime packaging.

Native tool/resource loading, browser interaction and Codex desktop MCP Apps
presentation are separate checks. Consult [validation](validation.md). A browser
panel alone does not prove embedded presentation. Automatic runtime acquisition and
fresh application-state onboarding are covered by the published six-platform
checks; system prerequisites are documented in [INSTALL.md](../INSTALL.md).
Embedded host presentation and public directory submission remain separate work;
there are no install hooks.

The UI follows [MCP Apps](https://modelcontextprotocol.io/extensions/apps/overview)
and [OpenAI UI extensions](https://developers.openai.com/plugins/build/chatgpt-ui).
Repo marketplace distribution does not imply public directory approval.

## Code panel handoff

In `chat=host` mode, Generate captures the framework and active page/selected
nodes, flushes pending edits and checks that the daemon document matches. The
app-only `canvas_codegen_prepare` exports deterministic baseline source without
calling an AI provider. The panel can preview, copy and download it immediately.
The Rust bridge binds every response to its request, document lifetime/revision
and framework; cancellation invalidates the pending request.

The MCP Apps wrapper feature-detects host messaging and sends one explicit
`ui/message` for Generate, using the [official host interface](https://developers.openai.com/plugins/build/chatgpt-ui).
Source and origin checks, bounded deduplication and no message retries prevent
replayed handoffs. No model call happens during selection polling. A host
acknowledgment means delivery, not finished code. Standalone browser panels lack
this host channel and display an instruction to continue in the current chat.

Model-visible `canvas_codegen_status` keeps the captured brief and baseline
available to Codex. After implementation and checks, `canvas_codegen_complete`
reads the primary UTF-8 source and rechecks request identity and design version.
It is excluded from browser RPC, as are arbitrary local file reads. App-only
`canvas_codegen_poll` returns changed results for preview/copy/download; supporting
files remain in the implementation project. Native standalone editor generation
continues using its existing provider pipeline.

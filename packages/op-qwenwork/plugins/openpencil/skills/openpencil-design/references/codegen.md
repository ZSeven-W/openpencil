# Code panel → QwenWork (千问办公) implementation

Use this workflow when the user requests conversion to code, clicks Generate in
the managed canvas's Code panel, or says “继续转代码” / “continue converting to code”.
Selection reminders alone never authorize implementation.

1. Read `canvas_codegen_status` when continuing a panel request. Its `requestId`,
   framework, page ID, node IDs and expected document version are fixed. Match its
   request ID to the triggering message; stop if superseded or cancelled. Read
   `canvas_status` and compare versions before work. If the design changed, ask
   the user to generate a fresh request; do not silently convert another target.
2. Read the captured nodes with `batch_get`, inspect their native screenshot,
   and inspect the current workspace. Follow the project's framework, components,
   tokens and asset conventions. With no implementation project, create a small
   runnable project in the current workspace without overwriting unrelated files.
3. The panel request includes deterministic `codegen_export` baseline source.
   Use it as a structural starting point, not a claim of production readiness.
   Inspect text, spacing, fonts, images, states, responsive behavior and accessibility.
   Reuse image assets from the design; export required local assets into the project.
4. Implement and run relevant checks. For a web project, inspect the rendered
   page against the native design screenshot. Do not modify the `.op` design as
   a side effect of implementation.
5. For a panel request, call `canvas_codegen_complete` with its `requestId` and
   the absolute `codePath` of the primary source file after verification. The
   extension must match the requested framework (`.tsx`/`.jsx`, `.vue`, `.svelte`,
   `.html`, `.dart`, `.swift`, or `.kt`) and be a regular UTF-8 file at most 2 MiB.
6. Return links to the runnable project and primary source, launch instructions,
   checks performed and remaining limitations.

The embedded MCP App wrapper sends an explicit Generate request through
`ui/message` when the host advertises it. An acknowledgment means the message
was handed off, not that implementation succeeded. Standalone browser panels and
hosts without messaging keep the baseline source and tell the user to say
“继续转代码” in the current QwenWork (千问办公) chat. Failed or uncertain messages are
never replayed automatically.

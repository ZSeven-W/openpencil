# apps/cli/ — OpenPencil CLI

The `op` command-line tool controls the OpenPencil desktop app or web server from the terminal.

## Structure

```text
apps/cli/
├── src/
│   ├── index.ts          Entry point — arg parsing, command dispatch, help text
│   ├── connection.ts     WebSocket connection to running app instance
│   ├── launcher.ts       Auto-detect and launch desktop app or web dev server
│   ├── output.ts         JSON output formatting (--pretty support)
│   └── commands/
│       ├── app.ts        start, stop, status
│       ├── design.ts     design, design:skeleton, design:content, design:refine
│       ├── document.ts   open, save, get, selection
│       ├── export.ts     export (react, html, vue, svelte, flutter, swiftui, compose, rn, css)
│       ├── import.ts     import:svg, import:figma
│       ├── layout.ts     layout, find-space
│       ├── nodes.ts      insert, update, delete, move, copy, replace
│       ├── pages.ts      page list/add/remove/rename/reorder/duplicate
│       └── variables.ts  vars, vars:set, themes, themes:set, theme:save/load/list
├── dist/                 Compiled output (openpencil-cli.cjs)
├── package.json          @zseven-w/openpencil, bin: { op }
└── README.md
```

## Commands

- **Compile:** `bun run cli:compile` (esbuild to `dist/openpencil-cli.cjs`)
- **Dev run:** `bun run cli:dev` (run from source via Bun)

## Key Patterns

- **Input methods:** Commands accepting JSON/DSL support inline string, `@filepath`, or `-` (stdin)
- **Connection:** WebSocket to running app instance (desktop or web server)
- **Launcher:** Auto-detects installed desktop app paths per platform (macOS, Windows, Linux)
- **esbuild:** Compiles with `--alias:@=src` to resolve web app imports, `--external:canvas --external:paper`
- **Output:** All commands output JSON; `--pretty` flag for human-readable formatting
- **Global flags:** `--file <path>` (target .op file), `--page <id>` (target page)

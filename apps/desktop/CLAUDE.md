# Desktop App

Electron desktop app for macOS, Windows, and Linux.

## Files

- **`main.ts`** — Main process: window creation, Nitro server fork, preferences, `.op` file association handling (`open-file` event on macOS, CLI args + single-instance lock on Windows/Linux)
- **`ipc-handlers.ts`** — IPC handler setup: native file dialogs (`dialog:openFile`, `dialog:saveFile`, `dialog:saveToPath`), theme sync for title bar overlay, renderer preferences (replaces origin-scoped localStorage), auto-updater IPC
- **`preload.ts`** — Context bridge for renderer <-> main IPC (file dialogs, menu actions, updater state, `onOpenFile`/`readFile` for file association)
- **`app-menu.ts`** — Native application menu configuration (File, Edit, View, Help)
- **`auto-updater.ts`** — Auto-updater: checks GitHub Releases on startup and periodically
- **`constants.ts`** — Electron-specific constants (port, window dimensions, platform padding)
- **`logger.ts`** — Main process logging
- **`dev.ts`** — Dev workflow: starts Vite -> waits for port 3000 -> compiles MCP -> compiles Electron -> launches Electron

## Build Flow

```text
BUILD_TARGET=electron bun run build
  -> bun run electron:compile (esbuild electron/ to out/desktop/)
  -> bun run mcp:compile
  -> npx electron-builder --config apps/desktop/electron-builder.yml
```

- **`electron-builder.yml`** — Packaging config: macOS (dmg/zip), Windows (nsis/portable), Linux (AppImage/deb), `.op` file association
- **`build/`** — Platform icons (.icns, .ico, .png)
- In production, Nitro server is forked as a child process on a random port; Electron loads `http://127.0.0.1:{port}/editor`

## File Association

`.op` files are registered as OpenPencil documents via `fileAssociations` in `electron-builder.yml`:
- macOS: `open-file` app event handles double-click/drag
- Windows/Linux: `requestSingleInstanceLock` + `second-instance` event forwards CLI args to existing window

## Auto-updater

Checks GitHub Releases on startup and every hour. `update-ready-banner.tsx` (in web app) shows download progress and "Restart & Install" prompt.

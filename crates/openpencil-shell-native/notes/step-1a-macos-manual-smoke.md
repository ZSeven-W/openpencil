# Step 1a macOS manual GPU smoke (per spec v19 §1.2 acceptance #1)

**Status**: PASS — exercised on Apple Silicon during Phase C Task 4
implementation (2026-05-05).

The macOS path through `gpu_smoke` / `gpu_chrome_stub_composition`
already runs in CI on `macos-latest` (`SharedSkiaContext::new_desktop`

- raster + chrome+stub composition). This note captures the
  maintainer-run `cargo run --example basic_window` smoke in addition
  to the automated checks (spec v19 §1.2 acceptance #1).

## Run on Apple Silicon (macos-latest, M-series)

```bash
cargo run -p openpencil-shell-native --example basic_window
```

### Expected window contents

- 800x600 window titled `OpenPencil — basic_window (Step 1a §1.2 acceptance #1)`.
- White background.
- Red filled rect at `(50, 50) — 100x100` (chrome).
- Black `Hello 你好` text at `(50, 200)` (chrome via Jian skia
  textlayout).
- Blue stroked rect outline at `(200, 50) — 200x150`.
- Closing the window via Cmd-W / red button → process exits with
  status 0 (no panic, no driver complaint).

### Verified

- Build: `cargo build --examples --workspace` succeeds clean.
- Process: launches without stderr output, holds the window until
  closed, no hangs on teardown.
- `SharedSkiaContext::teardown` runs once via `exiting()` and is a
  no-op on `Drop`.

## When to update this file

- After upgrading `vendor/jian` / `glutin` / `winit` / `skia-safe`.
- After reworking `paint_chrome` to call new `NativeBackend` methods.
- After macOS releases that change EAGL / Metal-translated bridging.

# Step 1a Windows manual GPU smoke (per spec v19 §8.1)

**Status**: PENDING — to be filled by a maintainer running on a Windows
desktop with a real GL driver.

The standard GitHub Actions `windows-latest` runner has no GPU driver
(`WINDOWS_GPU_DEFERRED_NO_RUNNER`), so spec v19 §8.1 requires the
following sequence to be exercised by a human on real hardware before
Step 1a can be declared "live on Windows".

## Prerequisites

- Windows 10/11 (x86_64 or aarch64) with a working OpenGL 3.3+ driver
  (default factory drivers on most modern GPUs satisfy this).
- Rust toolchain 1.85 (via `rustup toolchain install 1.85`).
- Submodules checked out (`git submodule update --init --recursive`) so
  `vendor/jian` is at `c4a794dc` (Step 1a freeze).

## Required commands

```pwsh
cargo run -p openpencil-shell-native --example basic_window
cargo test -p openpencil-shell-native --test gpu_smoke -- --include-ignored gpu_smoke
cargo test -p openpencil-shell-native --test gpu_chrome_stub_composition -- --include-ignored gpu_chrome_stub_composition
```

## Expected outcomes

- `basic_window` opens an 800x600 window showing:
  - White background.
  - Red filled rect at `(50, 50) — 100x100` (chrome).
  - Black `Hello 你好` text at `(50, 200)` (chrome via Jian skia
    textlayout).
  - Blue stroked rect outline at `(200, 50) — 200x150`.
  - Closing the window exits cleanly (no panic, no driver complaint
    in the console).
- `gpu_smoke` (`#[ignore]`'d on Windows by `WINDOWS_GPU_DEFERRED_NO_RUNNER`)
  passes when run with `--include-ignored` on a real-GPU host.
- `gpu_chrome_stub_composition` likewise passes — chrome pixel reads
  back red even after `CanvasViewportStub::render_into` pollutes
  `STENCIL_TEST` and `BlendFunc(ONE, ZERO)`.

## Where to record results

Append the run date, Windows build, GPU/driver, and output excerpt to
this file (replacing the `Status: PENDING` line) and commit on
`v0.8.0` with `docs(shell-native): record Windows manual GPU smoke`.

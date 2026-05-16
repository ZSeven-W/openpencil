# Step 1a Linux manual GPU smoke (per spec v19 §1.2 acceptance #1)

**Status**: PENDING — to be filled by a maintainer running on a Linux
desktop with a real or Mesa software-rendered GL driver.

The CI Linux runner ships Mesa llvmpipe via Xvfb but `gpu_smoke` /
`gpu_chrome_stub_composition` are currently `#[ignore]`'d under
`LINUX_GPU_SKIA_LOADER_TBD` (spec §3.1 mini-patch — `skia-safe`'s
`Interface::new_native` cannot resolve GL syms from EGL pbuffer +
llvmpipe; the proper fix is `new_load_with(eglGetProcAddress)` and is
deferred to Step 1f). The spec acceptance #1 on Linux therefore needs
a separate human runtime check.

## Prerequisites

- Ubuntu 22.04+ / Fedora 40+ / Arch with `mesa` / `libglvnd` /
  `xkbcommon` / `wayland-client` / `freetype` / `fontconfig` installed
  (the same set the CI `Install Linux GL prereqs` step provisions).
- X11 or Wayland session.
- Rust toolchain 1.85.
- Submodules at `vendor/jian@c4a794dc`.

## Required commands

```bash
cargo run -p op-host-native --example basic_window
# After Step 1f spec §3.1 mini-patch lands, also:
# cargo test -p op-host-native --test gpu_smoke -- --include-ignored gpu_smoke
# cargo test -p op-host-native --test gpu_chrome_stub_composition -- --include-ignored gpu_chrome_stub_composition
```

## Expected outcomes

- `basic_window` opens an 800x600 window showing:
  - White background.
  - Red filled rect at `(50, 50) — 100x100`.
  - Black `Hello 你好` text at `(50, 200)`.
  - Blue stroked rect outline at `(200, 50) — 200x150`.
  - Closing the window exits cleanly.
- Running under Xvfb + Mesa llvmpipe is acceptable for "no real GPU"
  hosts; the chrome must still render correctly.

## Where to record results

Append the run date, distro/version, GL renderer (`glxinfo | grep
OpenGL.renderer`), and output excerpt to this file (replacing the
`Status: PENDING` line) and commit on `v0.8.0` with
`docs(shell-native): record Linux manual GPU smoke`.

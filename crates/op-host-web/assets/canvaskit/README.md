# Vendored CanvasKit artifact

`canvaskit.js` (UMD loader) + `canvaskit.wasm` are the official skia WebAssembly
build, vendored here so the Rust web shell (`canvaskit` feature) renders without
any embedded skia / hand-rolled libc++ runtime and without depending on
`node_modules` (the TS app is being retired).

- **Version:** `canvaskit-wasm@0.40.0` (`bin/canvaskit.{js,wasm}`).
- **License:** BSD-3-Clause (skia / Google). See [`LICENSE`](./LICENSE), copied
  verbatim from the `canvaskit-wasm` package. The notice must ship alongside
  these binaries wherever the daemon serves them.
- **Served at:** `/canvaskit/canvaskit.{js,wasm}`. The daemon's web static server
  (`op-host-desktop/src/web_static.rs`) and any dev server must serve this
  directory at that URL prefix; `op_ck_bridge.js::opCkInit` loads
  `/canvaskit/canvaskit.js` then `CanvasKitInit({locateFile})`.
- **Update:** copy from `node_modules/.bun/canvaskit-wasm@<ver>/node_modules/canvaskit-wasm/bin/`
  after bumping the dependency, or fetch the matching release from the
  canvaskit-wasm npm package.

The Rust `CanvasKitBackend` drives this through the `op_ck_bridge.js` FFI; all
drawing/layout/widget logic stays in Rust.

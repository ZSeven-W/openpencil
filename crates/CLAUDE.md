# Rust Shell

Native + web editor chrome implemented in Rust against jian-skia. Goal: TS-equivalent editor UI surface so the backend can later swap underneath without UI regressions.

## Crate layout

```
crates/
├── openpencil-shell-core/      Platform-free widgets + Document model + RenderBackend trait
├── openpencil-shell-native/    Desktop runner: winit + skia-safe + accesskit
├── openpencil-shell-web/       Browser runner: wasm32-unknown-unknown + skia-safe-op fork
└── wasm-libc-shim/             ~95 env.* shims (libc / libm / libcxx) for the wasm32 build
```

`vendor/skia-safe-op/` is a fork of rust-skia that compiles for `wasm32-unknown-unknown` (no emscripten runtime); `vendor/jian/` is a submodule providing the rendering primitive layer (`jian-skia` Skia adapter, `jian-host-desktop` GL plumbing, `jian-core` event types). Both are referenced by path in workspace Cargo.toml.

## Key invariants

- **shell-core stays wasm32-clean** (spec v19 §1.2). No skia-safe / winit / accesskit_winit. The `RenderBackend` trait is the only seam between widget code and platform.
- **Widget code lives in shell-core only.** Hosts (shell-native `widget_host.rs`, shell-web `widget_host.rs`) are the ONLY files allowed to call `openpencil_shell_core::widgets::*`. Boundary script: `tools/check-widget-boundary.sh`.
- **Max 800 lines per file** — same rule as the TS workspace. `property_panel.rs` was split into `property_panel.rs` + `property_panel_sections.rs` to honor this.
- **Web bundle ceiling: 1 MiB gzip + 0 env.\* imports.** Enforced by `tools/check-wasm-bundle.sh`. Ceiling currently at ~916 KB after embedding Roboto + NotoSansCJK subset.

## Document model (`shell-core/src/document.rs`)

Single source of truth for editor state — mirrors TS `useCanvasStore` + `useDocumentStore` + `useAIStore` collapsed into one.

```text
Document
├── pages: Vec<Page>          (id + name + nodes)
├── active_page_index
├── selected: NodeId          (NONE = no selection)
├── tool: Tool                (Select/Rect/Text/Frame/Hand)
├── viewport: Viewport        (pan_x/pan_y/zoom + zoom_at + pan)
├── chat: ChatState           (messages, input, focused, anchor, collapsed)
└── ui: UiState               (sidebar_open)
```

`Node::aggregate_bounds` returns child-union bounds for container nodes (Group / unbounded Frame) so the property panel reports meaningful W/H.

## Widgets (`shell-core/src/widgets/`)

| Widget            | Section                                                            | File                                               |
| ----------------- | ------------------------------------------------------------------ | -------------------------------------------------- |
| TopBar            | Top — file name, agent chip, theme/i18n/fullscreen, sidebar toggle | `top_bar.rs`                                       |
| LayerPanel        | Left rail — Pages + Layers sections                                | `layer_panel.rs`                                   |
| Toolbar           | Vertical floating column — tool selection + actions                | `toolbar.rs`                                       |
| CanvasViewport    | Center — node tree + grid + viewport transform                     | `canvas_viewport.rs`                               |
| PropertyPanel     | Right rail — 设计/代码 tabs + 10 sections                          | `property_panel.rs` + `property_panel_sections.rs` |
| AIChatPlaceholder | Floating — chat with drag + 4-corner snap + collapse pill          | `ai_chat_panel.rs`                                 |
| StatusBar         | Floating bottom-right — zoom controls                              | `status_bar.rs`                                    |
| icons             | lucide d-string library (21 icons)                                 | `icons.rs`                                         |
| theme             | shadcn-dark palette tokens                                         | `theme.rs`                                         |

## RenderBackend trait

```rust
fill_rect / stroke_rect / draw_text / clip_rect
save / restore / translate
stroke_line / fill_round_rect / stroke_round_rect / stroke_svg_path
resize / dpi_scale
```

`stroke_svg_path` parses lucide d-strings via `skia_safe::utils::parse_path::from_svg`. PaintCap::Round + PaintJoin::Round to match lucide's stroke style.

## Native runner (`shell-native/`)

Entry: `examples/inspector_window.rs` — winit ApplicationHandler, GL surface via `jian-skia`. Logs:

- DPI scale via `canvas.scale((dpi, dpi))` per frame (preceded by `reset_matrix()` so it doesn't compound)
- LOGICAL viewport sizes (physical / dpi)
- Cursor position cached on `CursorMoved`, dispatched on `MouseInput`
- `MouseScrollDelta::PixelDelta` → trackpad pan; `LineDelta` / `PinchGesture` → zoom; modifier (Cmd/Ctrl) promotes pixel-delta to zoom

Native font path bypasses jian-skia's `textlayout` (which builds a fresh `FontCollection` per call → 605ms chrome frame): `NativeBackend` caches a Roboto Typeface + a system CJK Typeface (resolved via `FontMgr::match_family_style_character('一')`) and renders via `Canvas::draw_str`.

Run: `cargo run -p openpencil-shell-native --example inspector_window --release`.

## Web runner (`shell-web/`)

Single `mount(canvas_id)` entry point exposed to JS. Wires DOM listeners on the canvas + window:

- mousedown/mousemove/mouseup → apply_press / apply_cursor_move / apply_release
- wheel → apply_wheel
- keydown (window) → apply_text / apply_backspace / apply_send
- IME composition (hidden textarea) → apply_ime stubs

Skia surface: `wasm32-unknown-unknown` raster (N32_PREMUL) + `put_image_data`. Fonts: embedded Roboto-Regular.ttf (~35 KB) + NotoSansCJK-Subset.ttc (~8.7 KB) loaded via `FontMgr::custom_empty().new_from_data`.

Build: needs `EMSDK` env var pointing at an emsdk install (brew emscripten won't work — needs the real emsdk layout `$EMSDK/upstream/emscripten/llvm/bin/clang++`). Once set: `tools/check-wasm-bundle.sh` runs the full bundle gate (cargo → wasm-bindgen → wasm-opt -Oz, asserts 0 env.\* imports + ≤1 MiB gzip).

Smoke: `crates/openpencil-shell-web/smoke/step-1b.html` — start `python3 -m http.server 8000` from `crates/openpencil-shell-web/` and open http://localhost:8000/smoke/step-1b.html.

## Hit-test order

Hit-test runs in REVERSE paint order so the topmost overlay always wins:

1. TopBar (sidebar toggle button) — also eats other top-bar gaps
2. AI chat panel (DragHandle starts drag; FocusInput / Send / Example / ToggleCollapse defer to apply_click)
3. Toolbar (button hits dispatch tools; gaps inside the bounding rect eat clicks)
4. apply_click → LayerPanel rows / Page rows + chat-defocus
5. Empty canvas press → clear `selected` (collapses RightPanel) + start pan-drag

## Performance gotchas

- Native chrome paint: ~30 text draws × jian-skia textlayout's per-call `FontCollection::new()` = ~600ms/frame. Fix is the cached typeface path described above. Don't add new draw_text calls without cache awareness.
- skia canvas matrix is stateful across `with_frame` — `canvas.reset_matrix()` before applying DPI scale each frame, otherwise scale compounds.
- jian-skia's `DrawOp::Rect` / `DrawOp::Text` go through its image-cached path. `stroke_line` / `fill_round_rect` / `stroke_round_rect` / `stroke_svg_path` bypass jian and call skia canvas directly (necessary because jian doesn't have those DrawOp variants).

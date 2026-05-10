# Rust Shell

Native + web editor chrome implemented in Rust against jian-skia. Goal: TS-equivalent editor UI surface so the backend can later swap underneath without UI regressions.

## Crate layout

```
crates/
├── openpencil-shell-core/      Platform-free widgets + Document model + RenderBackend trait
├── openpencil-shell-native/    Native lib: WidgetHostNative + NativeBackend + SharedSkiaContext
├── openpencil-shell-web/       Browser runner: wasm32-unknown-unknown + skia-safe-op fork
├── openpencil-desktop/         Desktop binary: winit event loop + skia-safe GL surface
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

| Widget            | Section                                                            | File                                                           |
| ----------------- | ------------------------------------------------------------------ | -------------------------------------------------------------- |
| TopBar            | Top — file name, agent chip, theme/i18n/fullscreen, sidebar toggle | `top_bar.rs`                                                   |
| LayerPanel        | Left rail — Pages + Layers sections                                | `layer_panel.rs`                                               |
| Toolbar           | Vertical floating column — tool selection + actions (44×32)        | `toolbar.rs`                                                   |
| CanvasViewport    | Center — node tree + grid + viewport transform                     | `canvas_viewport.rs`                                           |
| PropertyPanel     | Right rail — 设计/代码 tabs + 10 sections                          | `property_panel.rs` + `property_panel_sections.rs`             |
| AIChatPlaceholder | Floating — chat with drag + 4-corner snap + collapse pill          | `ai_chat_panel.rs`                                             |
| LocalePicker      | TopBar Globe-button dropdown (15 native names + Check)             | `locale_picker.rs`                                             |
| StatusBar         | Floating bottom-right — zoom controls                              | `status_bar.rs`                                                |
| icons             | lucide d-string library (21 icons)                                 | `icons.rs`                                                     |
| theme             | shadcn-dark palette tokens (incl. `canvas_surface`)                | `theme.rs`                                                     |
| i18n              | 15 locale tables (706 keys each, TS-mirrored)                      | `i18n/{en,zh_cn,zh_tw,ja,ko,fr,es,de,pt,ru,hi,tr,th,vi,id}.rs` |

## Theme + i18n

`Document.ui` carries chrome-level state including `theme_mode` (Dark/Light), `locale`, and `locale_picker_open`:

- `Document::theme()` returns the active `Theme`. Widget builders read it instead of hardcoding `Theme::dark()`, so flipping the TopBar Sun icon reflows the entire chrome.
- `Document::t(key)` translates via `i18n::translate(self.ui.locale, key)`. Keys follow the TS `apps/web/src/i18n/locales/*.ts` dot.case convention (`common.untitled`, `pages.title`, `layers.title`, `ai.newChat`, `ai.tipSelectElements`, `rightPanel.design`, `layout.flexLayout`, `fill.title`, `stroke.title`, `effects.title`, `export.title`, `property.createComponent`, `topbar.agentsAndMcp`).
- 15 supported locales (matches TS dropdown order): EnUs / ZhCn / ZhTw / Ja / Ko / Fr / Es / De / Pt / Ru / Hi / Tr / Th / Vi / Id. Each carries a `display_name()` (English / 简体中文 / 繁體中文 / 日本語 / 한국어 / Français / Español / Deutsch / Português / Русский / हिन्दी / Türkçe / ไทย / Tiếng Việt / Bahasa Indonesia).
- TopBar Globe-button is a 44 px-wide compound (globe + chevron-down) opening a `LocalePicker` dropdown — clicking a row sets `Document.ui.locale` and closes; clicking outside (or the Globe again) closes silently. The picker paints as the top-most overlay so it covers chat / status / canvas.
- Multi-script chrome strings (한국어 / हिन्दी / ไทย / Tiếng Việt) render against per-codepoint typeface lookups (`FontMgr::match_family_style_character` cached per `i32` in `NativeBackend`), with each string broken into contiguous-typeface segments before draw.
- Tables are generated by `tools/convert-locales.py`. Re-run after changing TS locales:

  ```sh
  python3 tools/convert-locales.py
  ```

  Each locale file is ≤ 730 lines (under the 800-line ceiling). Cross-locale fallback: missing keys try EN before falling through to the key itself for debug visibility.

## RenderBackend trait

```rust
fill_rect / stroke_rect / draw_text / clip_rect
save / restore / translate
stroke_line / fill_round_rect / stroke_round_rect / stroke_svg_path
resize / dpi_scale
```

`stroke_svg_path` parses lucide d-strings via `skia_safe::utils::parse_path::from_svg`. PaintCap::Round + PaintJoin::Round to match lucide's stroke style.

## Desktop binary (`openpencil-desktop/`)

`crates/openpencil-desktop/src/main.rs` is the production desktop entry. It owns the winit `ApplicationHandler`, opens a GL window via `SharedSkiaContext::new_desktop`, and dispatches every `WindowEvent` onto `WidgetHostNative::apply_*`. Behaviour:

- DPI scale via `canvas.scale((dpi, dpi))` per frame (preceded by `reset_matrix()` so it doesn't compound)
- LOGICAL viewport sizes (physical / dpi)
- Cursor position cached on `CursorMoved`, dispatched on `MouseInput`
- `MouseScrollDelta::PixelDelta` → trackpad pan; `LineDelta` / `PinchGesture` → zoom; modifier (Cmd/Ctrl) promotes pixel-delta to zoom
- Cursor flips to `EwResize` when over a panel-resize gutter (`host.panel_resize_hover`)
- `WaitUntil(host.next_animation_deadline_ms())` pumps the caret-blink redraw

Native font path bypasses jian-skia's `textlayout` (which builds a fresh `FontCollection` per call → 605ms chrome frame): `NativeBackend` caches a Roboto Typeface + per-codepoint system fonts (resolved via `FontMgr::match_family_style_character`, cached per `i32`) so multi-script chrome (한국어 / हिन्दी / ไทย / Tiếng Việt) renders against the right font. `draw_text` segments each run by typeface and dispatches each segment via `Canvas::draw_str`.

Run: `cargo run -p openpencil-desktop --release`.

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
4. apply_click → LayerPanel rows / Page rows + chat-defocus (skipped when sidebar collapsed)
5. Empty canvas press → clear `selected` (collapses RightPanel) + start pan-drag

### Coordinate invariant

Every input path that reasons about the canvas region MUST derive its rects from `canvas_region(viewport_w, viewport_h)`. Never reuse `LAYER_PANEL_WIDTH` for hit-test — paint follows `canvas_region`, which collapses to `canvas_left = 0` when `Document.ui.sidebar_open == false`. Sites that proved this rule by violating it: `over_canvas`, `apply_wheel` cursor offset, toolbar hit rect in `apply_press` / `apply_click`. Web `apply_wheel` zoom anchor + `toolbar_rect()` helper follow the same rule.

## Performance gotchas

- Native chrome paint: ~30 text draws × jian-skia textlayout's per-call `FontCollection::new()` = ~600ms/frame. Fix is the cached typeface path described above. Don't add new draw_text calls without cache awareness.
- skia canvas matrix is stateful across `with_frame` — `canvas.reset_matrix()` before applying DPI scale each frame, otherwise scale compounds.
- jian-skia's `DrawOp::Rect` / `DrawOp::Text` go through its image-cached path. `stroke_line` / `fill_round_rect` / `stroke_round_rect` / `stroke_svg_path` bypass jian and call skia canvas directly (necessary because jian doesn't have those DrawOp variants).

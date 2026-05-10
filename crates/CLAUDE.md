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
├── tool: Tool                (Select / Rect / Ellipse / Polygon / Line / Pen / Text / Frame / Hand)
├── viewport: Viewport        (pan_x / pan_y / zoom + zoom_at + pan)
├── chat: ChatState           (messages, input, focused, anchor, collapsed)
└── ui: UiState               (sidebar_open, layer_panel_width, property_panel_width,
                               property_focus, property_input_draft, property_caret_anchor_ms,
                               theme_mode, locale, locale_picker_open,
                               shape_picker_open, shape_tool)
```

`Document::commit_property_edit(focus, value)` writes a parsed PropertyPanel input back into the selected node's bounds. `Tool::is_shape()` reports membership in the shape-slot group (Rect / Ellipse / Polygon / Line / Pen).

`Node::aggregate_bounds` returns child-union bounds for container nodes (Group / unbounded Frame) so the property panel reports meaningful W/H.

## Widgets (`shell-core/src/widgets/`)

| Widget            | Section                                                                             | File                                                           |
| ----------------- | ----------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| TopBar            | Top — file name, agent chip, theme/i18n/fullscreen, sidebar toggle                  | `top_bar.rs`                                                   |
| LayerPanel        | Left rail — Pages + Layers sections                                                 | `layer_panel.rs`                                               |
| Toolbar           | Vertical floating column — Select / shape slot / Text / Frame / Hand                | `toolbar.rs`                                                   |
| ShapePicker       | Toolbar shape-slot dropdown (Rect / Ellipse / Polygon / Line / Pen / Icon / Import) | `shape_picker.rs`                                              |
| CanvasViewport    | Center — node tree + grid + viewport transform                                      | `canvas_viewport.rs`                                           |
| PropertyPanel     | Right rail — 设计/代码 tabs + 10 sections + X/Y/W/H input editing                   | `property_panel.rs` + `property_panel_sections.rs`             |
| AIChatPlaceholder | Floating — chat with drag + 4-corner snap + collapse pill                           | `ai_chat_panel.rs`                                             |
| LocalePicker      | TopBar Globe-button dropdown (15 native names + Check)                              | `locale_picker.rs`                                             |
| StatusBar         | Floating bottom-right — zoom controls                                               | `status_bar.rs`                                                |
| icons             | lucide d-string library (35 icons)                                                  | `icons.rs`                                                     |
| theme             | shadcn-dark palette tokens (incl. `canvas_surface`)                                 | `theme.rs`                                                     |
| i18n              | 15 locale tables (706 keys each, TS-mirrored)                                       | `i18n/{en,zh_cn,zh_tw,ja,ko,fr,es,de,pt,ru,hi,tr,th,vi,id}.rs` |

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

## Toolbar shape-tool dropdown

The toolbar's compound `ShapeSlot` paints whichever shape variant is current (`ui.shape_tool`, default `Rect`) plus a small chevron-down in the gutter directly below the button (`SHAPE_SLOT_BOTTOM_EXTRA = 10 px`). Click anywhere on the slot — including the chevron — to toggle `ui.shape_picker_open`.

`ShapePicker::for_document(doc)` paints a 220 × 7-row dropdown anchored to the right of the slot. The seven rows mirror the TS shape-tool-dropdown verbatim:

- Rectangle / Ellipse / Polygon / Line / Pen → `ShapeChoice::Tool(Tool::*)` — the host writes `ui.shape_tool` + `doc.tool` and closes the panel.
- Icon → `ShapeChoice::OpenIconPicker` (host follow-up).
- Import Image or SVG… → `ShapeChoice::ImportImageOrSvg` (host follow-up).

Click anywhere outside the panel closes it silently. Locale lookups for the row labels (`shapes.rectangle / ellipse / polygon / line / icon / importImageSvg / pen`) come straight from the TS table; missing keys fall back to English literals.

## PropertyPanel input editing

`Document.ui` carries the focused property field, a draft buffer, and a caret-blink anchor:

- `property_focus: Option<PropertyFocus>` — `PositionX / PositionY / SizeW / SizeH / Rotation / Opacity / FillHex / StrokeHex / StrokeWidth`. Currently wired for X / Y / W / H editing; the others accept focus + clear cleanly but no-op at commit.
- `property_input_draft: String` — live keystrokes accumulate here; `apply_text` filters digits / leading minus / single decimal.
- `property_caret_anchor_ms: u64` — drives caret blink off the same `jian_core::anim::blink_visible` cadence as the chat input.

`PropertyPanel::for_selection_at(doc, now_ms)` is the entry point; the host calls `panel.hit_test(panel_rect, point)` to map clicks onto a `PropertyFocus`. Commit on Enter (parses f32, calls `Document::commit_property_edit`), discard on Escape, auto-commit on click outside the property panel.

`PropertyLabels::for_document(doc)` resolves every section title (位置/弹性布局/尺寸/图层/填充/描边/效果/导出), the 设计/代码 tabs, the 创建组件 button, and the size checkboxes (填充宽/高 / 适应宽/高 / 裁剪内容) via `Document::t`, falling back to English when a key isn't in the TS locale table.

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

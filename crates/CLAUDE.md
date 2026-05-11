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
- **Max 800 lines per file** — same rule as the TS workspace. `property_panel.rs` is split into 5 files (see PropertyPanel row in the widget table). `widget_host.rs` (shell-native) is split into a slim spine + 6 sibling submodules under `widget_host/` (see "Native widget_host layout" below).
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
                               property_draft_select_all,
                               theme_mode, locale, locale_picker_open,
                               shape_picker_open, shape_tool,
                               flex_layout, size_fill_width / fill_height / hug_width /
                               hug_height / clip_content, fill_type, fill_type_picker_open)
```

Mutators on `Document`:

- `commit_property_edit(focus, value)` — write parsed f32 to position / size / rotation / stroke width.
- `set_selected_color(is_fill, color)` — write hex-parsed `Color` to fill or stroke.
- `set_selected_bounds(rect)` — handle-drag resize.
- `set_selected_rotation(radians)` — rotation-ring drag.
- `translate_selected(dx, dy)` — node-drag move (recurses into descendants when the matched node is bounded so children don't detach).
- `delete_selected()` — remove the selected node from its parent's children (Delete / Backspace shortcut).
- `duplicate_selected(&mut next_id, offset_doc_px)` — deep-clone with fresh ids; lifts the allocator past `max_node_id() + 1` (`checked_add` so `u64::MAX` returns None instead of colliding).
- `reorder_selected(ReorderDirection::Up | Down)` — swap with next/prev sibling (`[` / `]`).
- `deselect_all()` — clear selection (Escape last tier).
- `max_node_id()` — largest raw id across pages + children, for the duplicate allocator guard.
- `node_at_doc_point(p)` — top-most-first hit-test honoring per-node rotation.

`Node.rotation: f32` (radians, cw +); paint applies `RenderBackend::rotate(radians, pivot)` around the node's centre. Bounded-Frame drag carries descendants — children's bounds are document-space-absolute.

`Node::aggregate_bounds` returns child-union bounds for container nodes (Group / unbounded Frame) so the property panel reports meaningful W/H.

`NodeKind` now spans Frame / Group / Rect / Ellipse / Polygon / Line / Text / Other; each has its own canvas paint (oval, triangle polygon, diagonal line, fill+stroke rect, draw_str). The `RenderBackend` trait grew `fill_oval` / `stroke_oval` / `fill_polygon` / `stroke_polygon` / `rotate` so both native + web backends can paint them.

`FillType { Solid, LinearGradient, RadialGradient, Image }` + `FlexLayout { Free, Vertical, Horizontal }` drive the property panel's dropdowns / button groups; both live on `Document.ui` so toggles persist across selection changes.

## Widgets (`shell-core/src/widgets/`)

| Widget            | Section                                                                                                                              | File                                                                                                                                    |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| TopBar            | Top — file name, agent chip, theme/i18n/fullscreen, sidebar toggle                                                                   | `top_bar.rs`                                                                                                                            |
| LayerPanel        | Left rail — Pages + Layers sections                                                                                                  | `layer_panel.rs`                                                                                                                        |
| Toolbar           | Vertical floating column — Select / shape slot / Text / Frame / Hand                                                                 | `toolbar.rs`                                                                                                                            |
| ShapePicker       | Toolbar shape-slot dropdown (Rect / Ellipse / Polygon / Line / Pen / Icon / Import)                                                  | `shape_picker.rs`                                                                                                                       |
| CanvasViewport    | Center — node tree + grid + viewport transform                                                                                       | `canvas_viewport.rs`                                                                                                                    |
| PropertyPanel     | Right rail — 设计/代码 tabs + 10 sections + interactive inputs (X/Y/W/H/R, hex, stroke width) + flex/size toggles + fill-type picker | `property_panel.rs` + `property_panel_sections.rs` + `property_panel_inputs.rs` + `property_panel_layout.rs` + `property_panel_fill.rs` |
| AIChatPlaceholder | Floating — chat with drag + 4-corner snap + collapse pill                                                                            | `ai_chat_panel.rs`                                                                                                                      |
| LocalePicker      | TopBar Globe-button dropdown (15 native names + Check)                                                                               | `locale_picker.rs`                                                                                                                      |
| StatusBar         | Floating bottom-right — zoom controls                                                                                                | `status_bar.rs`                                                                                                                         |
| icons             | lucide d-string library (35 icons)                                                                                                   | `icons.rs`                                                                                                                              |
| theme             | shadcn-dark palette tokens (incl. `canvas_surface`)                                                                                  | `theme.rs`                                                                                                                              |
| i18n              | 15 locale tables (706 keys each, TS-mirrored)                                                                                        | `i18n/{en,zh_cn,zh_tw,ja,ko,fr,es,de,pt,ru,hi,tr,th,vi,id}.rs`                                                                          |

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

- `property_focus: Option<PropertyFocus>` — `PositionX / PositionY / SizeW / SizeH / Rotation / Opacity / FillHex / StrokeHex / StrokeWidth`. **All variants are wired end-to-end:** numeric focuses go through `Document::commit_property_edit`, hex focuses through `set_selected_color(is_fill, color)`.
- `property_input_draft: String` — live keystrokes accumulate here. `apply_text` is focus-aware:
  - Numeric focuses (Position / Size / Rotation / Opacity / StrokeWidth) gate `[0-9]`, leading `-`, and a single `.`.
  - Hex focuses (FillHex / StrokeHex) preserve a sticky `#` prefix, accept `[0-9a-fA-F]` only, and cap the draft at 7 chars (`#RRGGBB`). No select-all-on-focus — backspace removes one char at a time, typing appends one.
- `property_caret_anchor_ms: u64` — drives caret blink off the same `jian_core::anim::blink_visible` cadence as the chat input.

Hex parsing is forgiving: `parse_hex_color` zero-pads 1-5 char inputs to 6 and expands CSS shorthand `#RGB` → `#RRGGBB`, so mid-edit commits don't visibly "reset" the colour.

`PropertyPanel::for_selection_at(doc, now_ms)` is the entry point. The host calls `panel.hit_test(panel_rect, point)` to map clicks onto a `PropertyFocus`, and `panel.hit_test_action(panel_rect, point)` to map clicks onto a `PropertyPanelAction`. Commit on Enter, discard on Escape, auto-commit on click outside the property panel.

### Buttons + checkboxes — `PropertyPanelAction`

```
PropertyPanelAction
├── SetFlexLayout(FlexLayout)        Free / Vertical / Horizontal
├── ToggleSizeFillWidth / FillHeight
├── ToggleSizeHugWidth / HugHeight
├── ToggleSizeClipContent
├── ToggleFillTypePicker             head-row dropdown
└── SetFillType(FillType)            Solid / LinearGradient / RadialGradient / Image
```

The hit-test walker `action_button_rects_with_fill_picker(panel_rect, visible, fill_picker_open)` lives in `property_panel_layout.rs` and emits one `Rect` per action. Same y-walk math as `editable_input_rects` so paint + hit-test stay in sync regardless of which sections are filtered.

### Fill-type dropdown

`FillType { Solid, LinearGradient, RadialGradient, Image }` lives on `Document.ui`. The Fill section head row paints `<swatch> <type-label ▾> <opacity%> <X>`; clicking the label opens an overlay popover with 4 rows. Body branches per type:

- **Solid** — hex input + caret.
- **LinearGradient** — Angle row + 色标 header + 2 default stops.
- **RadialGradient** — 色标 header + 2 stops (no angle).
- **Image** — 填充 row.

`fill_body_height(fill_type)` in `property_panel_layout.rs` returns the body height per variant; layout walkers thread it through `VisibleSections { …, fill_type }` so sections after Fill stay aligned with paint when the user flips type. Outside clicks close the picker via a dedicated swallow branch in `apply_press`, above all other property-panel hit-tests.

### Per-NodeKind section filtering

`SectionCapabilities::for_kind(NodeKind)` returns which sections paint for the current selection (Frame omits Stroke, Text omits Effects/Export, etc.). The returned `VisibleSections` is threaded through every paint routine _and_ both layout walkers so hidden sections cause subsequent rects to shift up by the right amount.

### File split

`property_panel_sections.rs` was split into 5 files to honor the 800-line ceiling:

- `property_panel.rs` — `PropertyPanel`, snapshot, `SectionCapabilities`, hit-test entry points.
- `property_panel_sections.rs` — section paint routines + `PropertyLabels` + `EditContext`.
- `property_panel_inputs.rs` — shared paint helpers (label / divider / input variants), layout constants, `format_color_hex`, `to_jian_color`.
- `property_panel_layout.rs` — `VisibleSections` / `SizeFlags` / `fill_body_height` + the two layout walkers.
- `property_panel_fill.rs` — fill-type label table, picker overlay, head row, all 4 body variants.

`PropertyLabels::for_document(doc)` resolves every section title (位置/弹性布局/尺寸/图层/填充/描边/效果/导出), the 设计/代码 tabs, the 创建组件 button, and the size checkboxes (填充宽/高 / 适应宽/高 / 裁剪内容) via `Document::t`, falling back to English when a key isn't in the TS locale table.

## RenderBackend trait

```rust
fill_rect / stroke_rect / draw_text / clip_rect
save / restore / translate
stroke_line / fill_round_rect / stroke_round_rect / stroke_svg_path
resize / dpi_scale
```

`stroke_svg_path` parses lucide d-strings via `skia_safe::utils::parse_path::from_svg`. PaintCap::Round + PaintJoin::Round to match lucide's stroke style.

## Native widget_host layout

`crates/openpencil-shell-native/src/widget_host.rs` is a slim spine (~265 lines) holding the public surface — `WidgetHostNative` struct, drag-state structs, `CursorHint` enum, `PanelResizeKind` enum, the constructor and tiny accessors (`set_now_ms` / `chat_focused` / `next_animation_deadline_ms`) — plus `mod` declarations for the sibling submodules under `widget_host/`:

| File                           | Purpose                                                                                                                                                                                                                |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `widget_host/frame_backend.rs` | `NativeFrameBackend` (`RenderBackend` impl over `NativeBackend` + `&Canvas`)                                                                                                                                           |
| `widget_host/helpers.rs`       | `parse_hex_color` / `color_to_hex` / `rect_contains` / `resize_bounds` + the inset / gutter / width constants                                                                                                          |
| `widget_host/geometry.rs`      | `impl WidgetHostNative` — canvas-region / panel-resize hover / cursor hint / picker rect math                                                                                                                          |
| `widget_host/input.rs`         | `impl WidgetHostNative` — `apply_wheel` / `_pan_gesture` / `_cursor_move` / `_release[_with_viewport]` / `_text` / `_backspace` / `_send` / `_escape` / `_property_action` / `commit_property_focus_if_any` / `_click` |
| `widget_host/press.rs`         | `impl WidgetHostNative` — `apply_press` + `create_node_for_active_tool` (largest single method; routes through 10 hit-test layers)                                                                                     |
| `widget_host/paint.rs`         | `impl WidgetHostNative::paint` — full editor-UI composition pass                                                                                                                                                       |

### Keyboard shortcuts

Native (`openpencil-desktop`) + web (`shell-web`) both dispatch the following P1 keyboard shortcuts through `WidgetHostNative` / `WidgetHost` methods. The desktop runner reads modifier state from `WindowEvent::ModifiersChanged` (`zoom_modifier` = Cmd/Ctrl, `shift_modifier` = Shift); the web shell reads `evt.meta_key() || evt.ctrl_key()` and `evt.shift_key()` from `KeyboardEvent`.

| Key                       | Method                | Behaviour (TS parity: `use-edit-shortcuts.ts` + `use-clipboard-shortcuts.ts`)                                                      |
| ------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `Backspace`               | `apply_backspace`     | Pops a char when an input is focused; else `delete_selected()`.                                                                    |
| `Delete`                  | `apply_delete`        | `delete_selected()` regardless of which non-text overlay is open.                                                                  |
| `Cmd/Ctrl+D`              | `apply_duplicate`     | `duplicate_selected(&mut next_node_id, 10.0)` and selects the clone.                                                               |
| `ArrowUp/Down/Left/Right` | `apply_nudge(dx, dy)` | Translates selection by 1 doc px, or 10 with `Shift`.                                                                              |
| `[`                       | `apply_reorder(Down)` | Swap with previous sibling (back in z-order).                                                                                      |
| `]`                       | `apply_reorder(Up)`   | Swap with next sibling (forward in z-order).                                                                                       |
| `Escape`                  | `apply_escape`        | One layer per press, in priority order: property-focus → locale picker → shape picker → fill-type picker → chat focus → selection. |
| `Enter`                   | `apply_send`          | Commits property edit or sends chat.                                                                                               |

All struct fields and intra-module helpers are scoped `pub(in crate::widget_host)` so submodule `impl` blocks can reach them while the public surface stays minimal. Each file is under 480 lines.

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

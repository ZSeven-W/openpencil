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
- **Max 800 lines per file** — same rule as the TS workspace. `property_panel.rs` is split into 5 files (see PropertyPanel row in the widget table). `widget_host.rs` (shell-native) is split into a slim spine + 8 sibling submodules under `widget_host/` (see "Native widget_host layout" below). `document.rs` is split into a spine + sibling submodules under `document/` — types in `document.rs`, `impl Document` in `mutators.rs`, free walkers + `ReorderDirection` in `walkers.rs`, chat types in `chat.rs`, pen-tool ops in `pen.rs`, color-picker state in `color_picker.rs`, group/ungroup ops in `grouping.rs`, page CRUD in `page_mutators.rs`, tests split into `tests_mutators.rs` + `tests_geometry.rs`.
- **Web bundle ceiling: 1 MiB gzip + 0 env.\* imports.** Enforced by `tools/check-wasm-bundle.sh`. Ceiling currently at ~916 KB after embedding Roboto + NotoSansCJK subset.

## Document model (`shell-core/src/document/`)

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
                               settings_input_draft,
                               agent_settings_open, agent_settings (focus, tab, connected[5],
                               mcp_server, mcp_cli_enabled[6], images_*, hover_provider),
                               color_picker, pen_in_progress, pen_cursor_doc,
                               pending_pen_history,
                               layer_context_menu, page_context_menu,
                               theme_mode, locale, locale_picker_open,
                               shape_picker_open, shape_tool,
                               flex_layout, size_fill_width / fill_height / hug_width /
                               hug_height / clip_content, fill_type, fill_type_picker_open,
                               property_tab (Design|Code))
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
- `start_pen_path` / `add_pen_point` / `finish_pen_path` (`document/pen.rs`) — multi-anchor `NodeKind::Path` builder; history snapshot captured BEFORE the first anchor so undo restores pre-pen state; finish strips 1-anchor (invisible) paths instead of polluting the undo stack.
- `group_selected` / `ungroup_selected` (`document/grouping.rs`) — Cmd+G / Cmd+Shift+G; group wraps selected siblings under a fresh `NodeKind::Group` whose `aggregate_bounds` covers the union.
- `add_page` / `duplicate_page` / `remove_page` / `rename_page_committed` (`document/page_mutators.rs`).
- `open_color_picker(target, click_y)` (`document/color_picker.rs`) — anchors a floating HSV picker; HSV stays anchored across the RGB-rounding cycle so dragging the hue slider doesn't visibly snap.

`Node.rotation: f32` (radians, cw +); paint applies `RenderBackend::rotate(radians, pivot)` around the node's centre. Bounded-Frame drag carries descendants — children's bounds are document-space-absolute. `Node.corner_radius: f32` (doc-px) is honored by Rect / Frame paint via `fill_round_rect` / `stroke_round_rect` when `radius * zoom > 0.5`; below that threshold paint collapses to `fill_rect` / `stroke_rect`.

`Node::aggregate_bounds` returns child-union bounds for container nodes (Group / unbounded Frame) so the property panel reports meaningful W/H.

`NodeKind` now spans Frame / Group / Rect / Ellipse / Polygon / Line / Text / Path / Other; each has its own canvas paint (oval, triangle polygon, diagonal line, fill+stroke rect, polyline through `node.points`, draw_str). The `RenderBackend` trait grew `fill_oval` / `stroke_oval` / `fill_polygon` / `stroke_polygon` / `rotate` / `fill_svg_path` so both native + web backends can paint them; `fill_svg_path` covers brand logos that ship as filled SVG paths in a non-24×24 viewBox.

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
| icons             | lucide d-string library (50+ icons; 24×24 viewBox stroke art)                                                                        | `icons.rs`                                                                                                                              |
| brand_icons       | Claude / OpenAI / Gemini / Copilot / OpenCode brand logos (filled SVG paths, non-24×24 viewBoxes)                                    | `brand_icons.rs`                                                                                                                        |
| ColorPicker       | HSV overlay (Cmd-Shift-C or fill/stroke swatch click) — sat/value box + hue strip + hex input                                        | `color_picker.rs`                                                                                                                       |
| LayerContextMenu  | Right-click overlay on layer rows + page tabs (Rename / Duplicate / Delete / Group / Ungroup / Lock / Hide; subset on page tabs)     | `layer_context_menu.rs`                                                                                                                 |
| AgentSettings     | Cmd+, modal — 880×640 with sidebar nav (Agents / MCP / Images / System) + scrollable right pane                                      | `agent_settings_panel.rs` + `agent_settings_{i18n,images,mcp,system}.rs`                                                                |
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

- `property_focus: Option<PropertyFocus>` — `PositionX / PositionY / Rotation / PositionR / SizeW / SizeH / Opacity / FillHex / StrokeHex / StrokeWidth`. **All 10 variants are wired end-to-end:** numeric focuses go through `Document::commit_property_edit` (`PositionR` writes `node.corner_radius`), hex focuses through `set_selected_color(is_fill, color)`.
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

## Settings modal (`Cmd+,`)

`agent_settings_panel.rs` + 4 tab modules render an 880×640 modal opened from the TopBar agent chip or `Cmd+,`. Sidebar nav: Agents / MCP / Images / System. Right pane scrolls; modal paints last (over dim scrim) so it covers every other widget.

- **Agents** — `+ 添加服务商` and `+ 添加 Agent` actions in two empty-state sections, then 5 provider cards (Claude / Codex / OpenCode / GitHub Copilot / Gemini) with real brand logos from `widgets/brand_icons.rs`. Hovering a connected card swaps the green `✓ Connected` row for a red `断开连接` button; both lifecycle actions toggle `agent_settings.connected[i]`.
- **MCP** — server status card with port input + Start/Stop button, then a 2×3 grid of CLI integration toggles (Claude Code / Codex / Gemini / OpenCode / Kiro / GitHub Copilot). Port input is editable (see "Settings input editing" below).
- **Images** — Image Search Ready/Not-configured indicator + collapsible Advanced section (Openverse OAuth Client ID / Secret + Register link + Test button), then Image Generation section with `+ Add` empty state.
- **System** — read-only Auto-update status card (no updater backend wired yet — a togglable switch would lie to the user; the row paints as informational text).

`agent_settings_i18n.rs` carries a hand-maintained EN/ZH key table (~50 keys, `settings.tab.*` / `settings.agents.*` / `settings.mcp.*` / `settings.images.*` / `settings.system.*` / `settings.provider.*`). The repo's main `i18n/{en,zh_cn,…}.rs` tables stay untouched (they're auto-generated from `apps/web/src/i18n/locales/*.ts`); when those TS tables grow `settings.*` keys, this hand-table collapses into per-locale `lookup` calls.

### Settings input editing

`SettingsFocus { McpPort }` is to settings inputs what `PropertyFocus` is to property-panel inputs. Click on the port field → `AgentSettingsHit::FocusMcpPort` → `agent_settings.focus = Some(McpPort)` + `UiState.settings_input_draft` seeded from current port. `apply_text` / `apply_backspace` / `apply_send` / `apply_escape` all route to the draft FIRST (swallowing every keystroke so non-digit chars don't leak into chat / rename / text-edit). Commit parses u16 and clamps ≥1024. Close / Outside / SelectTab / re-Focus all commit any pending draft first so a typed value isn't silently lost.

Mirrored on native (`widget_host/property_dispatch.rs::commit_settings_focus_if_any`) and web (`widget_host/keyboard.rs::commit_settings_focus`).

## Pen tool

`document/pen.rs` builds a `NodeKind::Path` with `points: Vec<Point2D>` (doc coords). State on `Document.ui`:

- `pen_in_progress: Option<NodeId>` — the path being authored (None when idle).
- `pen_cursor_doc: Option<Point2D>` — last cursor doc coord, drives the rubber-band preview from the last anchor to the cursor while authoring.
- `pending_pen_history: Option<DocumentSnapshot>` — snapshot captured BEFORE `start_pen_path` mutates the tree. Pushed onto the undo stack only when the path commits with ≥ 2 anchors; a lone-anchor (invisible) path is stripped without polluting history.

Press while `tool == Pen` calls `start_pen_path` or `add_pen_point` depending on `pen_in_progress`. Enter / Escape / tool change → `finish_pen_path`. Canvas paints the path through `node.points` as a polyline, plus a dashed preview line from `points.last()` to `pen_cursor_doc` while authoring.

## Color picker (HSV)

`document/color_picker.rs` + `widgets/color_picker.rs`. State: `ColorPickerState { target (Fill|Stroke), hue, sat, val, drag (None|SvBox|HueSlider), anchor_y }`. Open via `Cmd-Shift-C` or fill/stroke swatch click. **HSV stays anchored across the RGB-rounding cycle** — dragging the hue slider rewrites only `hue` and reconstructs RGB from the cached `(hue, sat, val)`, so the saturation+value crosshair doesn't visibly snap as round-trip rounding pulls a slightly different RGB out of the same HSV.

Hit-test order on the picker: hex input row → SvBox → HueSlider → close X. Outside-click closes silently. `apply_cursor_move` reads `state.drag` to feed live SvBox / HueSlider updates.

## Layer panel right-click + drag-into-container

`widgets/layer_context_menu.rs` paints a 200×N overlay on right-click of a layer row or page tab. Rows are gated on `LayerContextTarget`:

- **Layer**: Rename / Duplicate / Delete / Group / Ungroup / Lock / Hide (most rows route to dedicated `Document::*_selected` ops; Group/Ungroup live in `document/grouping.rs`).
- **Page**: Rename / Duplicate / Delete (`document/page_mutators.rs`).

Cursor-move feeds `hovered_row` so the menu highlights the row under the cursor. Outside-click closes silently. The menu paints AFTER the layer panel and BEFORE the settings modal so it sits below modals but above everything else.

Layer-panel drag now supports **cross-parent reparenting**: a drag whose drop-target falls inside a container row (Frame / Group with `children`) reparents the dragged node under that container instead of just reordering siblings. The walker in `widgets/layer_panel_walkers.rs` returns a `DropTarget { parent, before, into_container }` triple so the commit step can call `move_into_container` vs `reorder_in_place`.

## Performance gotchas

- Native chrome paint: ~30 text draws × jian-skia textlayout's per-call `FontCollection::new()` = ~600ms/frame. Fix is the cached typeface path described above. Don't add new draw_text calls without cache awareness.
- skia canvas matrix is stateful across `with_frame` — `canvas.reset_matrix()` before applying DPI scale each frame, otherwise scale compounds.
- jian-skia's `DrawOp::Rect` / `DrawOp::Text` go through its image-cached path. `stroke_line` / `fill_round_rect` / `stroke_round_rect` / `stroke_svg_path` / `fill_svg_path` bypass jian and call skia canvas directly (necessary because jian doesn't have those DrawOp variants).

### Hot-path optimizations (v0.8.0)

- **History via VecDeque** (`document/mutators.rs`) — `pop_front` + `push_back` capped at 100 entries; the old `Vec::remove(0)` was O(n) on every commit past the cap.
- **`Document::t` returns `&'static str`** — every locale value is a string literal, so chrome paint stores `&'static str` instead of cloning a `String` per frame. `PropertyLabels` is a `Copy` struct of static slices; widget builders propagate the `'static` lifetime so no per-paint allocations happen for labels.
- **Viewport culling** (`widgets/canvas_viewport.rs`) — `paint_node` takes a `cull: Rect` (canvas region + 64 px stroke/handle margin). Leaf nodes outside the cull skip paint entirely; containers always recurse so off-screen-parent / on-screen-child still renders.
- **Redraw scheduler** (`openpencil-desktop/src/main.rs`) — `request_redraw(dirty: bool)` + a `prepare_redraw` step that skips paint when only a tracked redraw fired and no visible state changed (kills the first-click chip flicker because macOS GL swap chain didn't perfectly hide `canvas.clear(BLACK)` between same-output frames).
- **Cursor-move coalescing** — `apply_cursor_move` cached as `pending_cursor_move`, drained on `RedrawRequested` AND right before `apply_press` / `apply_release` / `apply_right_press` (so the final drag-end frame isn't dropped). Without the press-time drain a fast drag-release could fire press before the queued cursor-move and the release saw stale hover state.
- **Font cache prewarm** (`openpencil-shell-native/src/backend/skia.rs`) — `NativeBackend::new` walks `PREWARM_CJK_CODEPOINTS` (~50 chars covering every CJK glyph used in the chrome + settings modal) through `FontMgr::match_family_style_character` at startup. Without this the first cross-tab paint stutters because each unseen CJK char triggers a synchronous system font scan.

## Native widget_host layout (expanded)

The shell-native `widget_host/` directory now houses 8 sibling submodules under the slim spine:

| File                               | Purpose                                                                                                                                                               |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `widget_host/frame_backend.rs`     | `NativeFrameBackend` (`RenderBackend` impl over `NativeBackend` + `&Canvas`)                                                                                          |
| `widget_host/helpers.rs`           | `parse_hex_color` / `color_to_hex` / `rect_contains` / `resize_bounds` + the inset / gutter / width constants                                                         |
| `widget_host/geometry.rs`          | Canvas region / panel-resize hover / cursor hint / picker rect math + `update_agent_settings_hover` (hover-card tracking for the agent cards)                         |
| `widget_host/input.rs`             | `apply_*` (non-press input handlers) including the settings-focus keyboard early-return                                                                               |
| `widget_host/press.rs`             | `apply_press` + `create_node_for_active_tool` (largest single method; routes through 10+ hit-test layers including the settings modal + color picker + layer context) |
| `widget_host/paint.rs`             | `WidgetHostNative::paint` — full editor-UI composition pass                                                                                                           |
| `widget_host/property_dispatch.rs` | Property-panel action dispatch + `commit_property_focus_if_any` + `commit_settings_focus_if_any`                                                                      |
| `widget_host/shortcuts.rs`         | Keyboard shortcut helpers (Cmd-Shift-C color picker, Cmd-Shift-G ungroup, etc.)                                                                                       |

## Canonical `.op` / `.pen` loader (v0.8.0+)

`openpencil-desktop/src/pen_doc_adapter.rs` bridges the canonical `jian_ops_schema::PenDocument` into the desktop's private `DocPayload` so files saved by the TS editor, Jian apps, or any other tool emitting the canonical schema load through the shared parser. Adapter responsibilities:

1. **Variant routing** — all 12 `PenNode` variants (frame / group / rectangle / ellipse / line / polygon / path / text / text_input / image / icon_font / ref) become `NodePayload`. Image / TextInput / IconFont degrade where their renderer isn't wired yet (image → grey placeholder rect; text_input → text node with seeded value / placeholder; icon_font → `NodeKind::Other("icon_font")` with `iconFontName` stashed in `text`).
2. **Layout via `jian_core::layout::LayoutEngine`** — taffy-backed flex solver, with the canonical `clipContent` / `justifyContent` / `alignItems` semantics. Each page-root gets its own engine pass; harvested rects are offset by `(base.x, base.y)` so multi-design files (e.g. `pencil-demo.op`'s 14 side-by-side mock-ups) spread across the infinite canvas instead of overlapping at origin.
3. **Text measurement via `jian_skia::SkiaMeasure`** — real skia paragraph shaper plugged in with `LayoutEngine::with_backend(Rc::new(SkiaMeasure::new()))`. Replaces the default `EstimateBackend` (~10% character-count heuristic) so `fit_content` text frames size against the same glyph advances `draw_text` paints with.
4. **Version tolerance** — `persistence::load_canonical` retries with `version` rewritten to `"1.0"` when the canonical schema rejects on a legacy major (e.g. pencil-demo.op's `version: "2.8"`), so older TS files still load best-effort.

`pen_doc_path_bounds.rs` ports `pen-core/src/path-anchors.ts::getPathBoundsFromAnchors` — cubic Bezier derivative roots + extrema sweep — so authored paths with handles/curves scale into their `width` / `height` box the same way the canonical renderer paints.

## Font weight + text wrapping

- **Numeric-string `fontWeight`** — `.op` files emit weights as JSON strings (`"700"`, `"600"`, `"normal"`). Both `vendor/jian/.../layout/mod.rs::resolve_weight` and `pen_doc_adapter::resolve_font_weight` parse numeric strings first, then fall back to lucide-style named weights (`bold`/`semibold`/`medium`/`light`/`thin`/`normal`/`regular`/`extralight`/`extrabold`/`black`/`heavy`/`demibold`/`ultralight`/`ultrabold`/`hairline`).
- **`Node.text_wrap: bool`** — set only when the schema authored `textGrowth: fixed-width` (or `fixed-width-and-height`). Canvas paint wraps text only when this flag is true; otherwise paints single-line at the authored width so font-fallback overshoot doesn't break lines the TS app shows on one line. `\n` characters split into multiple lines either way.
- **`wrap_text` (canvas_viewport_overlay.rs)** — greedy CJK-aware line breaker mirroring `pen-renderer/paint-utils.ts::wrapLine`. Per-character breaks for CJK codepoints (CJK Unified + Extension A, Hiragana/Katakana, Hangul, CJK symbols, full-width), word breaks for Latin runs, blank-line preservation on empty `\n` segments. Takes `weight: u16` so measurement matches the weighted typeface paint will use.
- **`RenderBackend::measure_text_weighted(text, font_size, weight)`** — every backend (`NativeBackend`, `WebBackend`, `NativeFrameBackend`) overrides this. Native keys its per-codepoint typeface cache on `(codepoint, weight)` and queries FontMgr with `FontStyle::new(Weight, Width::Normal, Slant::Upright)`. Web ships a single-weight bundle so `measure_text_weighted` forwards to `measure_text`. The trait default forwards to `measure_text` for the heuristic fallback.
- **Synthetic bold** — both native and web `draw_text` set `PaintStyle::StrokeAndFill` + `stroke_width = font_size * 0.06` for weights ≥ 600 so the bundled single-weight Roboto-Regular paints heavy when the file asks for bold.

## Icon coverage

`icons.rs` + `icons_data.rs` (sibling — keeps `icons.rs` under cap as the catalogue grows) cover ~75 lucide variants spanning chrome glyphs (Plus / Minus / Search / Settings / etc.) plus first-party `iconFontName` strings from `packages/pen-core/src/element-builders/` (`calendar`, `clock`, `map-pin`, `more-vertical`, `chevron-left`, `trending-up`/`-down`, `compass`, `refresh-cw`, `layout-dashboard`, `users`, `package`, `zap`, `sliders-horizontal`, `activity`, `loader`, `focus`, `chart-line`, `settings-2`, `arrow-right`, `check-circle`, `alert-triangle`, `alert-octagon`, `sticky-note`, `bar-chart-2`, `bold`/`italic`/`underline`/`strikethrough`, `shopping-cart`/`-bag`, `send`, `message-circle`, `rocket`, `menu`, `credit-card`, `x-circle`, `mail`, `smartphone`, `chrome`, `apple`, `user`). All d-strings copied verbatim from `node_modules/.bun/lucide-react@0.545.0/dist/esm/icons/*.js`.

`Icon::from_name(&str) -> Option<Icon>` resolves kebab-case glyph names plus common aliases (`back` / `forward` / `cart` / `bag` / `hamburger` / `card` / `cancel` / `house` / `chart-bar` / `like` / `team` / etc.). The unknown-name fallback strokes the canonical `FALLBACK_ICON_D` (`M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0`) — a small centred dot, matching `pen-renderer/node-renderer.ts::FALLBACK_ICON_D` — so unsupported icons read as "unknown glyph" instead of a solid block.

`paint_icon_font_node(backend, name, rect, fill)` in `icons.rs` is the canvas entry point: scales the 24×24 viewBox to `min(w, h)`, centres it, strokes at `(size / 24) * 2` widths to match lucide's 2 px reference stroke.

## Hover state across menus + panels

Every dropdown / sidebar / row list paints a per-row tint when the cursor is over it. State lives on `Document.ui` (or the appropriate sub-struct) and is updated by the host's `apply_cursor_move`:

| Surface                            | State                                                 | Tint                     |
| ---------------------------------- | ----------------------------------------------------- | ------------------------ |
| File menu (`file_menu.rs`)         | `Document.ui.file_menu_hover: Option<FileMenuChoice>` | `theme.muted` row wash   |
| Locale picker (`locale_picker.rs`) | `Document.ui.locale_picker_hover: Option<Locale>`     | `theme.muted`            |
| Shape picker (`shape_picker.rs`)   | `Document.ui.shape_picker_hover: Option<ShapeChoice>` | `theme.muted`            |
| Layer panel rows                   | `Document.ui.hovered_layer_id` / `hovered_page_index` | `theme.muted` row wash   |
| AgentSettings nav (sidebar tabs)   | `agent_settings.hover_nav: Option<AgentSettingsTab>`  | `theme.accent`           |
| AgentSettings provider cards       | `agent_settings.hover_provider: usize`                | `theme.accent` card wash |
| Layer context menu                 | `layer_context_menu.hovered_row: Option<u8>`          | row highlight            |

Every host close path clears its respective hover state so reopening starts un-hovered.

## File-menu polish

- Action labels strip the trailing `…` (Open file / Save As / Export image — pure label, no Mac-convention ellipsis).
- Recent file rows truncate long file names with a CJK-aware `truncate_to_width` helper that reserves space for the age column on the right.
- Compact row metrics (`ROW_HEIGHT = 30`, `HEADER_HEIGHT = 22`, `PAD_Y = 6`, `MENU_WIDTH = 300`) — the menu reads tighter than the default 36/28/8/320.

## Open / Save / Export error dialogs

`persistence.rs::show_error_dialog` pops a native `rfd::MessageDialog` on every failed load / save / export with a bilingual (EN / ZH per `Document.ui.locale`) title + path + detail body. `OpenRecent` failures additionally prune the stale entry from `Document.ui.recent_files`.

## MCP server (shell-core)

`crates/openpencil-shell-core/src/mcp.rs` (+ siblings under `mcp/`) carries the MCP wire layer that mirrors the TS `packages/pen-mcp` server. Serde-free hand-rolled JSON-RPC parser keeps the wasm32 bundle small.

### Tool catalog

Eight first-party tools registered today (v0.8.0+):

| Tool                    | Kind  | Args                                                                                               | Command emitted      | File                 |
| ----------------------- | ----- | -------------------------------------------------------------------------------------------------- | -------------------- | -------------------- |
| `get_document_info`     | read  | —                                                                                                  | —                    | `mcp/tools.rs`       |
| `get_selection`         | read  | —                                                                                                  | —                    | `mcp/tools.rs`       |
| `get_node`              | read  | `node_id`                                                                                          | —                    | `mcp/tools.rs`       |
| `list_pages`            | read  | —                                                                                                  | —                    | `mcp/tools.rs`       |
| `list_variables`        | read  | —                                                                                                  | —                    | `mcp/tools.rs`       |
| `get_active_theme`      | read  | —                                                                                                  | —                    | `mcp/tools.rs`       |
| `set_variable_color`    | write | `name`, `hex`                                                                                      | `SetVariableColor`   | `mcp/write_tools.rs` |
| `set_active_axis_value` | write | `axis`, `value`                                                                                    | `SetActiveAxisValue` | `mcp/write_tools.rs` |
| `insert_node`           | write | `kind`, `name`, `x`/`y`/`width`/`height`, optional `fill_hex`                                      | `InsertNode`         | `mcp/write_tools.rs` |
| `update_node`           | write | `node_id` + any of `x`/`y`/`width`/`height`/`name`/`fill_hex`                                      | `UpdateNode`         | `mcp/write_tools.rs` |
| `delete_node`           | write | `node_id`                                                                                          | `DeleteNode`         | `mcp/write_tools.rs` |
| `move_node`             | write | `node_id`, `target_parent_id` (0 = page root)                                                      | `MoveNode`           | `mcp/write_tools.rs` |
| `copy_node`             | write | `node_id`, `target_parent_id` (0 = page root)                                                      | `CopyNode`           | `mcp/write_tools.rs` |
| `replace_node`          | write | `node_id`, `kind`, `name`, `x`/`y`/`width`/`height`, optional `fill_hex`, optional `drop_children` | `ReplaceNode`        | `mcp/write_tools.rs` |

Read tools snapshot `Document` state at registration time. Write tools stay `&self`: they validate args and return `ToolOutcome::OkWithCommand(result, command)` for the host to apply via `Document::apply_mcp_command(command)`. The apply path follows pre-validate-then-mutate discipline (id space, target existence, geometry, hex, container-children consent) so a bad arg never leaves the document half-mutated.

`McpCommand::ReplaceNode.drop_children: bool` is a destructive-swap guard — replacing a container without explicit `drop_children=true` is refused at apply time so a Frame / Group can't silently lose its subtree.

### Wire-format hardening

`mcp/parser.rs::parse_tool_call` accepts both the real MCP `tools/call` envelope and the legacy direct-method shape. Multiple stop-gates baked in:

- **Structured args reject the parse.** `parse_flat_object_body` returns `None` on `{` or `[` for any value; `parse_tool_call` propagates that through a tri-state (`ParamsResult::Missing` / `Body` / `Malformed`). No scalar tool ever sees an object or array as a string-typed arg — earlier sentinel approaches were dropped because a literal `{...}` could collide with a real variable name.
- **`arguments` field is a top-level walker.** `arguments_field` iterates top-level key/value pairs in the params body so a nested `meta.arguments` can't shadow the real top-level field, and `"name":"arguments"` doesn't false-positive.
- **No client hangs on parse failure.** `run_stdio_with_applier` recovers the JSON-RPC `id` via `parser::extract_request_id` and writes a typed `ToolErrorCode::InvalidArgument` response so the client correlates + fails fast. Id-less lines drop silently — nothing to correlate against.
- **Read-only path refuses write tools.** `run_stdio` (no applier) demotes any `OkWithCommand` response to `ToolErrorCode::Internal`; clients can't see a "wrote: true" for an unapplied mutation.

### File layout

```
mcp.rs                       Spine: types + ToolRegistry + run_stdio* + JSON serializer
mcp/parser.rs                Wire parse — tri-state arguments_field, top-level walker
mcp/tools.rs                 Read tools (6)
mcp/tools_tests.rs           Read-tool tests
mcp/write_tools.rs           Write tools (8) + snapshot factories
mcp/write_tools_tests.rs     Write-tool tests (SetVariableColor / SetActiveAxisValue / Insert / Update / Delete / Move)
mcp/copy_node_tests.rs       Sibling — CopyNode tests
mcp/replace_node_tests.rs    Sibling — ReplaceNode tests (incl. destructive-swap guard)
mcp_tests.rs (in crate root) Cross-cutting: stdio dispatch, parser invariants, wire-shape regression
```

### Pending

- Wire `pen-server` (or the desktop binary) to register the `ToolRegistry` + spawn `run_stdio_with_applier` against the live document.
- Outstanding TS-parity write tools: `batch_design`, `design_skeleton`, `design_content`, `design_refine` — all require a JSON Node parser to accept full subtrees.
- HttpServer / streamable-http transport (currently the spawn-from-IPC scaffold has the lifecycle but not the wire format).

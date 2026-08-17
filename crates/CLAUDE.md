# Rust Shell

Native + web editor chrome implemented in Rust against jian-skia. This is the **canonical** OpenPencil implementation — the original TypeScript editor (`apps/web`, `apps/desktop`, `apps/cli`, `pen-*`) has been **retired**, so the Rust shell is the product, not a parity port. `// ported from pen-*` comments below name the retired TS as historical origin; that code lives only in git history now.

## Crate layout

> **Phase 7.3 reorg — read before trusting paths below.** The old
> `openpencil-shell-*` crates were dissolved/renamed; deeper sections in this doc
> may still cite pre-reorg paths. Map them: `openpencil-shell-native` →
> **`op-host-native`**, `openpencil-shell-web` → **`op-host-web`**,
> `openpencil-desktop` → **`op-host-desktop`**. `openpencil-shell-core` was
> **split**, not just renamed: widgets / Document / `RenderBackend` →
> **`op-editor-ui`**, the MCP wire layer → **`op-mcp`** (flat `op-mcp/src/*.rs`,
> no `mcp/` subdir). The `wasm-libc-shim` + `vendor/skia-safe-op` wasm-skia fork
> are **retired** — the browser renders through the official CanvasKit skia WASM
> (loaded separately), so the Rust wasm bundle is pure logic; the **"Web runner"
> section below is stale** (it describes the old wasm32 skia-raster + EMSDK path).
>
> **2026-07 dedupe + split campaign (`839f474a..d2d8104c`) — second stale-path
> layer.** Three structural changes ripple through the sections below. (1) The
> canonical editor state is `EditorState` in **`op-editor-core/src/state.rs`**;
> there is no `document/` directory in any crate, so every `shell-core/src/document/*.rs`
> path in the "Document model", pen-tool, colour-picker, and hot-path sections
> names a file that no longer exists — the behaviour they describe is still
> accurate, the paths are not. (2) The native↔web `widget_host` fork is
> **single-sourced** — see "Shared host logic" below for where the real logic
> lives. (3) The 800-line cap now holds **workspace-wide with zero violations**,
> so nearly every module named below is a spine plus sibling files; a path that
> resolves to a directory instead of a single file is the split, not a rename.

```
crates/
├── op-editor-ui/         Platform-free widgets + RenderBackend facade (wasm32-clean); re-exports the render scene from jian-scene + scene_vars from op-editor-core
├── op-editor-core/       Canonical `.op` (PenDocument) editor state + EditorCommand + scene_vars (design-variable resolution)
├── op-editor-host-core/  Transport-free host state machines shared by all hosts
├── op-collab/            Open, transport-free collaboration protocol + canonical hash + exact document apply (wasm32-clean)
├── op-collab-host/       Host-agnostic collaboration session runtime (`CollabRuntime` + `CollabHost`); desktop and the serve-web daemon both drive it
├── op-host-native/       Native host lib: WidgetHostNative + skia-safe GL backend (desktop + mobile)
├── op-engine-ffi/        Player C ABI: embed the canvas in iOS / Android shells (metal / gl / raster surfaces)
├── op-engine-jni/        Android JNI marshalling layer for op-engine-ffi (engine thread, registry, callbacks)
├── op-engine-napi/       OpenHarmony (OHOS) Node-API layer → `libopenpencil.so`; reuses op-engine-jni's engine thread + registry, everything else target-gated on `target_env = "ohos"` (see its README for the ArkTS API table)
├── op-host-web/          Browser bundle entry: wasm32-unknown-unknown cdylib, CanvasKit renderer
├── op-host-desktop/      Desktop binary `openpencil-desktop` (winit + skia-safe GL); also the `--serve-web` daemon
├── op-cli/               `op` command-line tool
├── op-util/              Dependency-free leaf: shared collaboration-id grammar + hex-colour / JSON / XML helpers
└── …                     op-mcp / op-ai / op-ai-skills / op-codegen / op-orchestrator / op-figma /
                          op-git / op-opmerge / op-pen-loader / op-design-lint / op-config-store /
                          op-process-io / op-acp / op-i18n / op-rpc-transport / op-smoke /
                          op-host-services / op-host-web-server / op-html / op-auth-bridge / op-web-sdk
```

`op-util` is the workspace's single source for the collaboration namespace/id
grammar and three helpers that had drifted
into many copies: `hex_color` (nine divergent implementations, one of which
panicked on non-ASCII input), `json_escape` (one copy was lossy), and
`xml_escape` (one copy omitted the quote entity — an attribute-injection gap).
It must stay dependency-free and wasm32-clean: it sits in the browser host's
build graph via op-editor-core / op-editor-ui. Reach for it instead of writing
a local `parse_hex` / `escape_json` again.

`vendor/jian/` is a submodule providing the rendering primitive layer (`jian-skia` Skia adapter, `jian-host-desktop` GL plumbing, `jian-core` event types + taffy layout, `jian-scene` canonical render scene — `LayoutScene` + hit-test + path geometry, consumed by OP and any jian app); `vendor/casement` is the winit fork; `vendor/agent` is the cross-product Rust agent runtime. All are referenced by path in the workspace Cargo.toml.

## Key invariants

- **op-host-native tests need `--features gl-host`.** `gl-host` is a non-default feature (keeps `skia-safe/gl` out of mobile builds), and the entire `widget_host` module is gated behind it — a default-feature `cargo test -p op-host-native` runs 55 tests instead of 676 and gives ZERO coverage of `widget_host/` changes. Always test native with `--features gl-host`.

- **op-editor-ui (widgets) stays wasm32-clean** (spec v19 §1.2). No skia-safe / winit / accesskit_winit. The `RenderBackend` trait is the only seam between widget code and platform.
- **Widget code lives in op-editor-ui only.** Hosts (op-host-native `widget_host.rs`, op-host-web `widget_host.rs`) are the ONLY files allowed to call `op_editor_ui::widgets::*`. Boundary script: `tools/check-widget-boundary.sh`.
- **Max 800 lines per file — zero violations workspace-wide.** As of `d2d8104c` no `.rs` file in `crates/` exceeds the cap, and the sibling-module split is the universal shape: a spine keeps the public surface and `mod` declarations, cohesive clusters move into siblings, and re-exports keep every import path and test name stable. Splits are pure code motion — when you split, do not also change behaviour. Test modules follow the same rule (`foo_tests.rs`, or a `foo/tests/` directory when the tests themselves outgrow the cap). Check with `find crates -name '*.rs' -exec wc -l {} + | awk '$1>800'`.
- **Blocking on a future from sync host code goes through `op_host_services::chat_runtime::block_on_anywhere`.** A bare `Runtime::block_on` (or a privately-built current-thread runtime) aborts with "runtime within runtime" when the caller happens to sit on a tokio worker. `block_on_anywhere` picks the safe strategy for whichever context it is called from; it is the only sanctioned entry point and is exercised by tests for the no-runtime, multi-thread-worker, and borrowing-non-`Send`-future cases.
- **Fallible paths carry typed error enums, not `String`.** The whole workspace is converted (80+ enums; `Result<_, String>` survives at exactly two documented boundary sites). Find the domain's enum in its `*_error.rs` / `error.rs` sibling module (e.g. `CliError`, `ProgramError`, `WebCanvasError`, `McpServeError`, `ExportError`, `McpLiveError`, `DocIoError`, `ImageGenerateError`). The pattern to copy: one enum per failure domain in its own sibling module, structured fields instead of pre-formatted text, a `Display` impl that reproduces the previous message **byte-identically** (so user-visible strings and their tests don't move), and `From` impls that collapse the `map_err` adapters at the call sites. New fallible code should introduce or reuse an enum rather than add another `Result<_, String>`.
- **Web bundle ceiling: 6 MiB gzip + 0 env.\* imports.** Enforced by `tools/check-wasm-bundle.sh`. The CanvasKit bundle carries the full app logic (codegen AI pipeline, Figma parser, AI/live-sync, collaboration) plus the still-embedded AI skill corpus (~1.1 MiB of markdown via `include_dir!`), so the ceiling sits well above the retired skia raster path's 1 MiB (**4.93 MiB today**; the ceiling is a runaway-regression tripwire, not a budget). Re-baselined from 8 MiB once the ~2.4 MiB of preview JPEGs moved out — see below. The 0 env.\* guard still holds — CanvasKit needs no libc shim.
- **Product assets are embedded on native and fetched on wasm.** `op-editor-core/src/web_assets.rs` is the platform-free half: a process-global `route -> bytes` registry with single-flight, a widget-records / host-drains request queue, and `Absent/Pending/Ready/Failed` states. The widget layer calls `web_assets::request(route)` from paint and draws a placeholder; `op-host-web/src/web_asset_fetch.rs` drains it once per frame over XHR and installs the bytes. Native keeps its `include_bytes!` unchanged behind `#[cfg(not(target_arch = "wasm32"))]`, so desktop behaviour is byte-identical. Assets are served from `/pkg/assets/…` (**not** `/assets/`, which belongs to the hub frontend) and staged into the bundle by `tools/stage-web-assets.sh`, which `check-wasm-bundle.sh` runs as step 4 and both the CI workflow and `Dockerfile.web-rust` inherit. Moved: the Prompt Center (~2.1 MiB) and Scene Template (~2.7 MiB) preview JPEGs, the 64 scene-template `.op` documents (~6.2 MiB), and `iconify-catalog-core.json` (~452 KiB) — ~11.3 MiB of staged assets in total. Still embedded everywhere: the `op-ai-skills` corpus (~1.1 MiB, `include_dir!`).
- **The icon-catalog split is opt-in, the rest is `cfg(target_arch = "wasm32")`.** `op-editor-ui`'s `runtime-icon-catalog` feature is enabled only by `op-host-web`, which has a daemon to fetch from and a frame pump to install on. `op-web-sdk` renders iconFont nodes but has no daemon, so it keeps the embed — for that viewer "not fetched yet" would mean "never". Bundle ceilings are per-bundle and measured: op-host-web 6 MiB (5.17 MiB measured), op-web-sdk 6 MiB (5.20 MiB measured).

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
                               property_focus, property_input: TextInputState,
                               settings_input: TextInputState,
                               agent_settings_open, agent_settings (focus, tab, connected[5],
                               mcp_server, mcp_cli_enabled[8], images_*, hover_provider),
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
| PropertyPanel     | Right rail — 设计/代码 tabs + 10 sections + interactive inputs (X/Y/W/H/R, hex, stroke width) + flex/size toggles + fill-type picker | `property_panel.rs` + the `property_panel_*.rs` family + `property_panel/`, `property_panel_snapshot/`, `property_panel_tests/` |
| AIChatPlaceholder | Floating — chat with drag + 4-corner snap + collapse pill                                                                            | `ai_chat_panel.rs`                                                                                                                      |
| LocalePicker      | TopBar Globe-button dropdown (15 native names + Check)                                                                               | `locale_picker.rs`                                                                                                                      |
| StatusBar         | Floating bottom-right — zoom controls                                                                                                | `status_bar.rs`                                                                                                                         |
| icons             | lucide d-string library (50+ icons; 24×24 viewBox stroke art)                                                                        | `icons.rs`                                                                                                                              |
| brand_icons       | Claude / OpenAI / Gemini / Copilot / Antigravity / Grok filled logos + OpenCode primitive logo                                      | `brand_icons.rs`                                                                                                                        |
| ColorPicker       | HSV overlay (Cmd-Shift-C or fill/stroke swatch click) — sat/value box + hue strip + hex input                                        | `color_picker.rs`                                                                                                                       |
| LayerContextMenu  | Right-click overlay on layer rows + page tabs (Rename / Duplicate / Delete / Group / Ungroup / Lock / Hide; subset on page tabs)     | `layer_context_menu.rs`                                                                                                                 |
| AgentSettings     | Cmd+, modal — 880×640 with sidebar nav (Agents / MCP / Images / System) + scrollable right pane                                      | `agent_settings_panel.rs` + `agent_settings_{i18n,images,mcp,system}.rs`                                                                |
| theme             | shadcn-dark palette tokens (incl. `canvas_surface`)                                                                                  | `theme.rs`                                                                                                                              |
| i18n              | 15 canonical locale catalogs, 1,526 direct keys each, sharded main → `_git` → `_panel` → `_collab` in the `op-i18n` crate                       | `op-i18n/src/i18n/{en,zh_cn,zh_tw,ja,ko,fr,es,de,pt,ru,hi,tr,th,vi,id}{,_git,_panel,_collab}.rs`                                                 |

## Theme + i18n

`Document.ui` carries chrome-level state including `theme_mode` (Dark/Light), `locale`, and `locale_picker_open`:

- `Document::theme()` returns the active `Theme`. Widget builders read it instead of hardcoding `Theme::dark()`, so flipping the TopBar Sun icon reflows the entire chrome.
- `Document::t(key)` translates via `i18n::translate(self.ui.locale, key)`. Keys retain the retired TS app's dot.case convention (`common.untitled`, `pages.title`, `layers.title`, `ai.newChat`, `ai.tipSelectElements`, `rightPanel.design`, `layout.flexLayout`, `fill.title`, `stroke.title`, `effects.title`, `export.title`, `property.createComponent`, `topbar.agentsAndMcp`), but the Rust tables are now the canonical source.
- 15 supported locales (matches TS dropdown order): EnUs / ZhCn / ZhTw / Ja / Ko / Fr / Es / De / Pt / Ru / Hi / Tr / Th / Vi / Id. Each carries a `display_name()` (English / 简体中文 / 繁體中文 / 日本語 / 한국어 / Français / Español / Deutsch / Português / Русский / हिन्दी / Türkçe / ไทย / Tiếng Việt / Bahasa Indonesia).
- TopBar Globe-button is a 44 px-wide compound (globe + chevron-down) opening a `LocalePicker` dropdown — clicking a row sets `Document.ui.locale` and closes; clicking outside (or the Globe again) closes silently. The picker paints as the top-most overlay so it covers chat / status / canvas.
- Multi-script chrome strings (한국어 / हिन्दी / ไทย / Tiếng Việt) render against per-codepoint typeface lookups (`FontMgr::match_family_style_character` cached per `i32` in `NativeBackend`), with each string broken into contiguous-typeface segments before draw.
- Locale tables are hand-maintained in Rust, and each locale is **four files, chained by fall-through**: the main table `<locale>.rs` ends its `match` with `_ => return super::<locale>_git::lookup(key)`, the Git shard ends with `_ => return super::<locale>_panel::lookup(key)`, the `_panel` shard ends with `_ => return super::<locale>_collab::lookup(key)`, and the `_collab` overflow shard ends with `_ => return None`. The catalog is 1,526 direct keys per locale (`catalog_integrity_tests.rs` asserts the exact number).

  **Adding a new key: put it in `<locale>_collab.rs` (the terminal overflow shard), in all 15 locales.** The main tables sit at or just under the repo's 800-line ceiling (`zh_cn.rs` is exactly 800, `en.rs` 798), so a new entry there does not fit — the `_panel` shards exist precisely to absorb overflow and have room. Only edit a main or `_git` table when you are changing the wording of a key that already lives there. Then bump the expected count in `catalog_integrity_tests.rs` and run:

  ```sh
  cargo test -p op-i18n
  ```

  Catalog integrity tests enforce the exact cross-locale key set across all four shards, reject duplicate or unsupported match arms, and require every translation to preserve the English placeholder set. The `i18n/mod.rs` tests were split into sibling modules alongside it (`tests.rs`, `catalog_integrity_tests.rs`, plus per-feature key guards: `figma_property_panel_key_tests.rs`, `html_import_key_tests.rs`, `missing_fonts_key_tests.rs`, `preview_device_key_tests.rs`, `vector_fidelity_property_keys.rs`) — add a feature's key guard next to those rather than growing `mod.rs`. `tools/convert-locales.py` is a deliberate failing shim because its retired TypeScript source no longer exists. Runtime lookup still falls back through English and then the raw key for debug visibility, while the tests prevent shipped locale gaps.

## Toolbar shape-tool dropdown

The toolbar's compound `ShapeSlot` paints whichever shape variant is current (`ui.shape_tool`, default `Rect`) plus a small chevron-down in the gutter directly below the button (`SHAPE_SLOT_BOTTOM_EXTRA = 10 px`). Click anywhere on the slot — including the chevron — to toggle `ui.shape_picker_open`.

`ShapePicker::for_document(doc)` paints a 220 × 7-row dropdown anchored to the right of the slot. The seven rows mirror the TS shape-tool-dropdown verbatim:

- Rectangle / Ellipse / Polygon / Line / Pen → `ShapeChoice::Tool(Tool::*)` — the host writes `ui.shape_tool` + `doc.tool` and closes the panel.
- Icon → `ShapeChoice::OpenIconPicker` (host follow-up).
- Import Image or SVG… → `ShapeChoice::ImportImageOrSvg` (host follow-up).

Click anywhere outside the panel closes it silently. Locale lookups for the row labels (`shapes.rectangle / ellipse / polygon / line / icon / importImageSvg / pen`) come straight from the TS table; missing keys fall back to English literals.

## PropertyPanel input editing

`Document.ui` carries the focused property field and a shared text input state:

- `property_focus: Option<PropertyFocus>` — `PositionX / PositionY / Rotation / PositionR / SizeW / SizeH / Opacity / FillHex / StrokeHex / StrokeWidth`. **All 10 variants are wired end-to-end:** numeric focuses go through `Document::commit_property_edit` (`PositionR` writes `node.corner_radius`), hex focuses through `set_selected_color(is_fill, color)`.
- `property_input: TextInputState` — live keystrokes, caret, select-all, blink, and composition state accumulate here. `apply_text` is focus-aware:
  - Numeric focuses (Position / Size / Rotation / Opacity / StrokeWidth) gate `[0-9]`, leading `-`, and a single `.`.
  - Hex focuses (FillHex / StrokeHex) preserve a sticky `#` prefix, accept `[0-9a-fA-F]` only, and cap the draft at 7 chars (`#RRGGBB`). No select-all-on-focus — backspace removes one char at a time, typing appends one.
- `property_input_draft` / caret fields are legacy mirrors while older preset-name and compatibility paint paths are retired.

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
- **Image** — trigger row (thumbnail + current mode name) opening a 220px popover (`property_panel_image_fill.rs`): mode toggle (Fill / Fit / Crop / Tile; Crop enables drag-to-reframe on canvas with edge clamping), Tile scale input, upload well, 7 color-adjust sliders + reset. Schema `ImageFillBody.transform` is honored by both skia and CanvasKit renderers.

`fill_body_height(fill_type)` in `property_panel_layout.rs` returns the body height per variant; layout walkers thread it through `VisibleSections { …, fill_type }` so sections after Fill stay aligned with paint when the user flips type. Outside clicks close the picker via a dedicated swallow branch in `apply_press`, above all other property-panel hit-tests.

### Per-NodeKind section filtering

`SectionCapabilities::for_kind(NodeKind)` returns which sections paint for the current selection (Frame omits Stroke, Text omits Effects/Export, etc.). The returned `VisibleSections` is threaded through every paint routine _and_ both layout walkers so hidden sections cause subsequent rects to shift up by the right amount.

### File split

The panel is a `property_panel_*.rs` family in `op-editor-ui/src/widgets/` (plus the `property_panel/`, `property_panel_snapshot/`, and `property_panel_tests/` directories) — one file per section or concern, all under the 800-line ceiling. The load-bearing entries:

- `property_panel.rs` — `PropertyPanel`, snapshot, `SectionCapabilities`, hit-test entry points.
- `property_panel_sections.rs` — section paint routines + `PropertyLabels` + `EditContext`.
- `property_panel_inputs.rs` — shared paint helpers (label / divider / input variants), layout constants, `format_color_hex`, `to_jian_color`.
- `property_panel_layout.rs` — `VisibleSections` / `SizeFlags` / `fill_body_height` + the two layout walkers.
- `property_panel_fill.rs` (+ `_fill_body`, `_fill_image_body`, `_fill_picker`) — fill-type label table, picker overlay, head row, body variants.
- `property_panel_dispatch.rs` (+ `_support`), `property_panel_commit.rs`, `property_panel_layout_ops.rs` — the **host-shared** action dispatch / draft lifecycle / layout writers; see "Shared host logic" below. These are not paint code and are called by both hosts.

Everything else (`_code*`, `_effects`, `_export`, `_stroke`, `_text`, `_typography`, `_image_*`, `_instance`, `_interactions`, `_icon`, `_compositing`, …) follows the same one-concern-per-file rule.

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

`crates/op-host-native/src/widget_host.rs` is a spine (~690 lines) holding the public surface — the `WidgetHostNative` struct, `CursorHint` / `PanelResizeKind` enums, the constructor and small accessors (`set_now_ms` / `chat_focused` / `next_animation_deadline_ms`) — plus the `mod` declarations for ~130 sibling submodules under `widget_host/`. Its web twin, `op-host-web/src/widget_host.rs` (~500 lines), has the same shape. The drag-state structs are **not** declared here any more: both hosts share them from `op_editor_core::host_drag_state`.

The directory is too large to table exhaustively; the naming scheme tells you where to look:

| Pattern                                             | Contents                                                                                                                                                        |
| --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `press.rs` + `press_ctx.rs` + `press_*_tiers.rs`     | The `apply_press` hit-test ladder — spine, per-event scratch struct, and the ordered tier bodies (see "Hit-test order")                                          |
| `input.rs` + `cursor_move_ctx.rs` + `cursor_move_*.rs` | The `apply_cursor_move` hover ladder, same spine/ctx/tier shape (native only; web's cursor path lives in `cursor_input.rs`)                                    |
| `keyboard.rs` + `keyboard_*.rs`                     | Key dispatch split by concern — caret, clipboard, delete, escape, send, IME                                                                                     |
| `*_dispatch.rs`                                     | Thin platform arms over the shared `op-editor-ui` dispatch flows (property, image panel, settings, font picker)                                                 |
| `paint.rs`, `paint_pan_cache.rs`, `scene_state.rs`  | The composition pass and its caches                                                                                                                             |
| `host_lifecycle.rs`, `host_requests.rs`             | Host boot / teardown and outbound request plumbing carved off the spine                                                                                         |
| `geometry.rs`, `overlay_rects.rs`, `helpers.rs`     | Thin forwarders to the shared geometry in `op-editor-ui` plus the few genuinely platform-specific rects                                                          |
| `*_tests.rs` / `*_tests/`                           | Per-concern test siblings; a directory means the tests themselves outgrew the cap                                                                               |

`NativeFrameBackend` (the `RenderBackend` impl over `NativeBackend` + `&Canvas`) moved out of `widget_host/` — it lives in `op-host-native/src/backend/frame_backend.rs` alongside `frame_offscreen.rs` and the `skia/` split.

### The three input ladders

`apply_cursor_move` (native) and both hosts' `apply_press` used to be single 900–1150 line methods. Each is now a short spine that calls `Option`-returning tier helpers in ascending priority order, with per-event scratch state in a `PressCtx` / `CursorMoveCtx` struct instead of a long argument list. A tier returning `Some(consumed)` ends the event; `None` falls through to the next tier.

**The call order in the spine *is* the behaviour.** The decomposition was verified mechanically — every tier body is byte-identical to its original line range and the spines invoke tiers in strictly ascending original order — so reordering a call is a behaviour change, not a refactor. Read "Hit-test order" below before touching either spine.

All struct fields and intra-module helpers are scoped `pub(in crate::widget_host)` so submodule `impl` blocks can reach them while the public surface stays minimal.

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

The shared implementation of these lives in `op_editor_core::host_keyboard_transitions` + `host_escape_transitions`; the host `keyboard*.rs` files are the platform arms.

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

**This section is the canonical statement of the ladder** — the tier modules point back here by name. Hit-test runs in REVERSE paint order so the topmost overlay always wins, and each spine's call order encodes it literally.

`apply_press` (native `widget_host/press.rs`) — the prelude first blur-commits an in-flight layer rename and text edit, with one exception: a press *inside* the edited text node places the caret instead of committing (TS textarea parity). Then:

1. `press_topmost_overlay_tiers` — modals, floating panels, context menus
2. commit-on-blur for property-panel inputs + the variable-row inline editor, then resolve Git-panel / chat-model-picker ownership (the floating Git panel paints over the right rail, so every rail tier below skips a click inside its rect)
3. `press_menu_modal_tiers` — shape picker; file / export / figma / login / account modals; import + locale dropdowns
4. `press_rail_overlay_tiers` — image-fill popover, StatusBar, resize gutter, the model-picker slice above the TopBar
5. `press_top_bar_tier` — TopBar chrome and its blank-press gaps
6. `press_preview_tier` — Preview (Play) mode swallows everything below
7. `press_property_overlay_tiers` → `press_panel_dispatch_tiers` → `press_property_panel_tier`
8. `press_git_and_chat_tiers`
9. `press_toolbar_tiers`
10. `press_layer_and_click_tiers` — LayerPanel rows / Page rows + chat-defocus
11. `press_canvas_tier` → `press_canvas_select_tier` — empty-canvas press clears selection (collapsing the right rail) and starts the pan / marquee drag

Web's `apply_press` runs the same order with two extra tiers split out (`press_import_locale_tiers`, `press_variables_tiers`, `press_font_and_picker_tiers`) because its overlay set differs; the relative priority is identical.

`apply_cursor_move` (native `widget_host/input.rs`) runs its own 10-tier ladder: modals → floating panels → a live rail-resize drag → in-flight property/text/crop/node/pen drags (pointer capture) → menus + Git panel → property-panel popovers → StatusBar/align/model-picker → TopBar → the left rail's slides tab → the single-shot chat probe (deliberately non-consuming, so the late drags still run) → late pointer-capture drags → base hover. The rail resize sits with the early pointer-capture drags rather than the late ones because its pointer travels back *over* the rail it is resizing, where the slides-tab hover tier would otherwise claim the move — which is exactly why the left rail could once be dragged wider but never narrower. It keeps one owned `PropertyPanel` snapshot per event in `CursorMoveCtx::property_panel_probe` so the expensive snapshot/i18n work happens at most once.

### Coordinate invariant

Every input path that reasons about the canvas region MUST derive its rects from `canvas_region(…)`. Never reuse `LAYER_PANEL_WIDTH` for hit-test — paint follows `canvas_region`, which collapses to `canvas_left = 0` when the sidebar is closed. Sites that proved this rule by violating it: `over_canvas`, `apply_wheel` cursor offset, toolbar hit rect in `apply_press` / `apply_click`. Web `apply_wheel` zoom anchor + `toolbar_rect()` follow the same rule.

The invariant now has **one implementation**: `op-editor-ui/src/widgets/host_canvas_geometry.rs` owns `canvas_origin` / `canvas_region` / `canvas_rect` / `over_canvas` / `canvas_doc_point[_unclamped]` / `canvas_centre_doc_point` / `layer_panel_rect` / `property_panel_rect` / `toolbar_rect*` / `status_bar_rect` / `marquee_rect` / `path_anchor_hit`, and each host's `widget_host/geometry.rs` is a thin forwarder. Both hosts previously carried byte-identical copies of this math — exactly the drift the invariant exists to prevent. Sixteen inline screen-to-doc conversions were routed through the shared helper as part of the same pass; do not add a seventeenth by hand.

## Settings modal (`Cmd+,`)

`agent_settings_panel.rs` + 4 tab modules render an 880×640 modal opened from the TopBar agent chip or `Cmd+,`. Sidebar nav: Agents / MCP / Images / System. Right pane scrolls; modal paints last (over dim scrim) so it covers every other widget.

- **Agents** — `+ 添加服务商` and `+ 添加 Agent` actions in two empty-state sections, then 6 provider cards (Claude / Codex / OpenCode / GitHub Copilot / Antigravity / Grok Build) with real brand logos from `widgets/brand_icons.rs`. Hovering a connected card swaps the green `✓ Connected` row for a red `断开连接` button; both lifecycle actions toggle `agent_settings.connected[i]`.
- **MCP** — server status card with port input + Start/Stop button, then a grid of 7 CLI integration toggles (Claude Code / Codex / OpenCode / Kiro / GitHub Copilot / Antigravity / Grok Build). Port input is editable (see "Settings input editing" below).
- **Images** — Image Search Ready/Not-configured indicator + collapsible Advanced section (Openverse OAuth Client ID / Secret + Register link + Test button), then Image Generation section with `+ Add` empty state.
- **System** — read-only Auto-update status card (no updater backend wired yet — a togglable switch would lie to the user; the row paints as informational text).

`agent_settings_i18n.rs` is a thin widget-layer adapter over the shared `op-i18n` lookup. All `settings.*` strings (`settings.tab.*` / `settings.agents.*` / `settings.mcp.*` / `settings.images.*` / `settings.system.*` / `settings.provider.*`) live directly in every one of the 15 canonical locale tables.

### Settings input editing

`SettingsFocus { McpPort }` is to settings inputs what `PropertyFocus` is to property-panel inputs. Click on the port field → `AgentSettingsHit::FocusMcpPort` → `agent_settings.focus = Some(McpPort)` + `EditorUiState.settings_input: TextInputState` seeded from current port. `apply_text` / `apply_backspace` / caret movement / select-all route through the shared text-input state FIRST (swallowing every keystroke so non-digit chars don't leak into chat / rename / text-edit). `apply_send` commits, parsing u16 and clamping ≥1024. `apply_escape` clears focus and the shared input. Close / Outside / SelectTab / re-Focus all commit any pending draft first so a typed value isn't silently lost.

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
- **Font cache prewarm** (`op-host-native/src/backend/skia.rs`) — `NativeBackend::new` walks `PREWARM_CJK_CODEPOINTS` (~50 chars covering every CJK glyph used in the chrome + settings modal) through `FontMgr::match_family_style_character` at startup. Without this the first cross-tab paint stutters because each unseen CJK char triggers a synchronous system font scan.

## Shared host logic (the native↔web `widget_host` fork is single-sourced)

`op-host-native/src/widget_host/` and `op-host-web/src/widget_host/` used to be a copy-pasted twin pair, and the copies had silently drifted apart — web's Interactions actions were dead no-ops, native's shift-drag skipped the undo snapshot, native painted two variables menus at once, web's marquee release never synced the entered container, and so on. Every one of those was a *real user-visible bug* found only because the twins were diffed. The fork is now single-sourced.

**Before adding logic to either host, check whether it belongs in one of these.** A host file should be a thin platform arm: hit-test, call the shared flow, run the platform tail. Genuinely platform-specific behaviour stays host-side **with a comment saying why**.

Pure state transitions — no widget types, wasm32-clean — live in **`op-editor-core/src/`**:

| Module                             | Owns                                                                                    |
| ---------------------------------- | ----------------------------------------------------------------------------------------- |
| `host_ui_transitions.rs`           | Chrome-state transitions (the byte-identical twin blocks)                                 |
| `host_press_transitions.rs`        | Press-path chrome transitions                                                             |
| `host_keyboard_transitions.rs`     | Keyboard-driven editor-state transitions                                                  |
| `host_escape_transitions.rs`       | The Escape ladder — exactly one layer dismissed per press                                 |
| `host_drag_state.rs`               | The transient pointer-drag structs both spines used to declare separately                 |
| `host_drag_transitions.rs`         | Canvas press/drag state transitions                                                       |
| `host_variables_transitions.rs`    | VariablesPanel press arms                                                                 |
| `host_variables_commit.rs`         | VariablesPanel draft commits (header renames + per-row Name/Number/String/Color drafts)   |
| `host_settings_commit.rs`          | Settings-modal draft commit                                                               |
| `host_image_panel_transitions.rs`  | Image-node panel transitions (Search / Generate popovers)                                 |
| `host_preset_name_draft.rs`        | The variables preset "save as name" input                                                 |
| `request_snapshot.rs`              | `narrowed_snapshot` — see below                                                           |
| `agent_reveals.rs`                 | Before/after node-id diff + staggered reveal bookkeeping for freshly inserted subtrees    |
| `auth_routes.rs`                   | Device-login HTTP route paths shared by the web shell (client) and serve-web daemon (server) |
| `collab_routes.rs` + `collab_wire.rs` | Collaboration is **not desktop-only**: the wasm bundle carries the UI but no transport, so the browser drives a daemon-hosted session over `/api/collab/{state,action,presence,avatar}` with versioned wire DTOs (client `op-host-web/src/collab_sync.rs`, server `op-host-services/src/web_canvas_server/collab_routes.rs`) |

Widget-typed shared logic lives in **`op-editor-ui/src/widgets/`**:

| Module                                                     | Owns                                                                              |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `press_flow.rs`                                            | Press dispatch — hosts map a hit-test result through it and run only the tail      |
| `chat_click_flow.rs`                                       | Chat-panel click dispatch (`apply_chat_hit`)                                       |
| `property_panel_dispatch.rs` + `_support.rs`               | PropertyPanel action dispatch (was two ~500-line near-verbatim copies)             |
| `property_panel_commit.rs`                                 | Both ends of the PropertyPanel draft lifecycle (`property_focus_initial` + commit) |
| `property_panel_layout_ops.rs`                             | Layout / sizing / typography writers behind the button groups                      |
| `drag_flow.rs`                                             | Canvas node-drag flow                                                              |
| `marquee_flow.rs`                                          | Marquee-release commit (screen→doc rect walk + additive/replace selection)          |
| `image_crop_flow.rs`                                       | Figma-style image-fill crop gesture                                                |
| `image_popover_input_flow.rs`                              | Mouse selection in the image Search / Generate text inputs                         |
| `scroll_flow.rs`                                           | Panel wheel / trackpad-pan scrolling                                               |
| `cursor_hover_flow.rs`                                     | Cursor-move hover resolution                                                       |
| `account_press_flow.rs`                                    | Sign-in modal + signed-in account-dropdown press flow                              |
| `missing_fonts_flow.rs`                                    | Missing-font prompt: scroll, press dispatch, detection/refresh bookkeeping         |
| `variables_panel_geometry_flow.rs`                         | Floating VariablesPanel placement + resize clamping                                |
| `agent_settings_press_flow.rs` + `_entries.rs` + `_focus.rs` | Agent-settings modal press dispatch, list-entry arms, and input focus seeding    |
| `host_canvas_geometry.rs`                                  | The coordinate invariant (see "Hit-test order")                                    |
| `host_overlay_geometry.rs`                                 | On-screen rects for dropdowns, the three draggable floating panels, StatusBar actions |
| `host_frame_bookkeeping.rs`                                | Per-frame work both spines do around paint (cache owner rotation, layout transitions, animation deadlines) |
| `menu_paint.rs`                                            | Shared dropdown row chrome (hover tint, hit rect, divider) for file + account menus |
| `settings_form.rs`                                         | Shared card-form chrome for the Agent settings sections                            |
| `test_capture_backend.rs`                                  | Test-only `RenderBackend` stub recording `fill_round_rect` calls                   |

`op-editor-ui/src/accessibility_regions.rs` (crate root, not `widgets/`) single-sources the a11y region walk behind native's `widget_host/a11y.rs` and web's `a11y_bridge.rs`.

Outside the widget hosts the same rule applies: settings payload serde lives in `op-editor-host-core/src/settings_payload.rs`, image generate / search in `op-host-services/src/web_image_{generate,search}.rs`, and the `--mcp` / `--mcp-http` / `--serve-web` argv dispatch in `op_host_services::cli_modes` (shared by `op-host-desktop` and `op-host-web-server`, which differ only in what an unknown mode means). Desktop no longer carries drifted copies of any of them.

### `EditorUiState` split + narrowed snapshots

`op-editor-core/src/editor_ui_state.rs` is a spine over seven siblings (`chrome`, `defaults`, `git_panel`, `groups`, `methods`, `pickers`, `tests`), with re-exports keeping every import path stable. Three measured field clusters became substructs in `editor_ui_state/groups.rs` — `PreviewState`, `SizeToggleState`, `DesignMdPanelState` (192 → 176 flat fields). Larger clusters were rejected on workspace touch-count, so don't group further without measuring.

`request_snapshot::narrowed_snapshot(&mut EditorState)` is the sanctioned way to hand editor state to a worker thread. The MCP request path, the chat-launch path, and the desktop design pump's per-apply-ack all use it: it drops `chat` / `codegen` / `theme_presets`, which grow with session length and are read by no consumer on the worker side. A full `EditorState::clone()` on a request path is a bug.

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

## MCP server (`op-mcp`)

The **`op-mcp`** crate carries the MCP wire layer that mirrors the retired TS `packages/pen-mcp` server: a flat `op-mcp/src/*.rs` module set with `lib.rs` as the spine (types + `ToolRegistry` + `run_stdio*`). There is no `mcp/` subdirectory. A serde-free hand-rolled JSON-RPC parser keeps the wasm32 bundle small.

### Tool catalog

**123 static-schema tools** registered today. The authoritative list
is `op-host-services/src/mcp_serve/schemas.rs` — `TOOL_SCHEMAS`
(JSON inputSchemas; `DEBUG_TOOL_SCHEMAS` is gated out of the
production catalog) — plus the `register_tool!` calls in
`op-host-services/src/mcp_serve.rs`, guarded by an exact-count
assertion test in `mcp_serve/tests.rs`
(`tools_list_response_includes_all_registered_tools`). A further
**37 kit-component tools** (`insert_<component>`, one per built-in
UIKit component — 6 starter-kit + 31 shadcn) are appended dynamically
via `op_mcp::element_tools::insert_kit_component_tools` and ride
alongside the static schemas in the `tools/list` response. These 37
tools are what remains of the old TS element ecosystem — the ~188
`add_*` element-alias tools, their per-category builders/shards, and
the layered element manifest (`emit_elements`) are retired; callers
that need an arbitrary tree reach for `batch_design` instead.

By category:

- **Read (18)** — `get_document_info` / `get_selection` / `get_node`
  / `get_node_children` / `get_node_parent` / `list_pages` /
  `list_variables` / `get_active_theme` / `list_components` /
  `get_component` / `snapshot_layout` / `get_canvas_bounds` /
  `find_node_by_name` / `count_nodes` / `list_node_kinds` /
  `get_history_depth` / `get_viewport` / `get_selection_set`.
  Each snapshots `Document` state at registration.
- **Node write (insert/update/delete/move/copy/replace)** —
  `op-mcp/src/write_tools.rs`.
- **Per-node attribute writers** — `set_node_rotation` / `_text` /
  `_corner_radius` / `_font_size` / `_font_weight` / `_stroke_hex` /
  `_stroke_width` / `_fill_hex` / `_name` (`op-mcp/src/node_attr_tools.rs`).
- **Selection / clipboard / canvas ops** — `set_selection` /
  `set_selection_set` / `toggle_node_selection` / `clear_selection`
  / `duplicate` / `delete` / `nudge` / `group` / `ungroup` /
  `reorder` / `align_selected` / `copy_selected` / `cut_selected`
  / `paste_clipboard` / `set_active_tool` / `set_viewport` /
  `set_node_hidden|locked|collapsed` / `undo` / `redo`
  (`op-mcp/src/selected_ops_tools.rs` + `op-mcp/src/component_tools.rs`).
- **Pages + components** — `add_page` / `rename_page` /
  `delete_page` / `duplicate_page` / `reorder_page` /
  `set_active_page` / `instantiate_component` / `create_component`
  / `delete_component` / `rename_component` (`op-mcp/src/component_tools.rs`).
- **Variables + themes** — `set_variable_color|number|string|boolean`
  / `create_variable` / `delete_variable` / `rename_variable` /
  `set_active_axis_value` / `cycle_active_axis_value`
  (`op-mcp/src/scalar_vars.rs` + `op-mcp/src/component_tools.rs`).
- **Batch design** — `batch_design` / `design_skeleton` /
  `design_content` / `design_refine` (`op-mcp/src/batch_design.rs`).
  `batch_design` accepts exactly one of `nodes_json` (a flat
  descriptor array), `operations` (the insert-only `I(parent, node)`
  program DSL), or `script` — a JavaScript program sandboxed through
  `op_mcp::script_runner` (feature `script`, native-only; 64 MiB
  memory / 2 s wall-clock / 4096 recorded-line / 256 KiB source-size
  limits) whose only effect is calling `I(parent, node)` to emit the
  same program. The orchestrator's subagents share this exact runner:
  script-gen (a real JS program driving `I(...)`) is THE generation
  protocol on every subagent rung. The reduced-complexity /
  minimal-skills retry rungs only narrow the loaded skill set; they do
  not switch output protocol. `parse_nodes` remains available for the
  separate modify/chat paths that still consume flat node JSON.

Read tools snapshot `Document` state at registration time. Write tools stay `&self`: they validate args and return `ToolOutcome::OkWithCommand(result, command)` for the host to apply via `Document::apply_mcp_command(command)`. The apply path follows pre-validate-then-mutate discipline (id space, target existence, geometry, hex, container-children consent) so a bad arg never leaves the document half-mutated.

`McpCommand::ReplaceNode.drop_children: bool` is a destructive-swap guard — replacing a container without explicit `drop_children=true` is refused at apply time so a Frame / Group can't silently lose its subtree.

### Wire-format hardening

`op-mcp/src/parser.rs::parse_tool_call` accepts both the real MCP `tools/call` envelope and the legacy direct-method shape. Multiple stop-gates baked in:

- **Structured args reject the parse.** `parse_flat_object_body` returns `None` on `{` or `[` for any value; `parse_tool_call` propagates that through a tri-state (`ParamsResult::Missing` / `Body` / `Malformed`). No scalar tool ever sees an object or array as a string-typed arg — earlier sentinel approaches were dropped because a literal `{...}` could collide with a real variable name.
- **`arguments` field is a top-level walker.** `arguments_field` iterates top-level key/value pairs in the params body so a nested `meta.arguments` can't shadow the real top-level field, and `"name":"arguments"` doesn't false-positive.
- **No client hangs on parse failure.** `run_stdio_with_applier` recovers the JSON-RPC `id` via `parser::extract_request_id` and writes a typed `ToolErrorCode::InvalidArgument` response so the client correlates + fails fast. Id-less lines drop silently — nothing to correlate against.
- **Read-only path refuses write tools.** `run_stdio` (no applier) demotes any `OkWithCommand` response to `ToolErrorCode::Internal`; clients can't see a "wrote: true" for an unapplied mutation.

### File layout

All paths below are `op-mcp/src/` — flat, no subdirectory:

```
lib.rs                       Spine: types + ToolRegistry + run_stdio*
json_serializer.rs           JSON-RPC wire serializer (response_to_json + helpers)
parser.rs                    Wire parse — tri-state arguments_field, top-level walker
tools.rs                     Core read tools
read_tools.rs / read_nodes.rs / extra_read_tools.rs / read_tools_extra.rs
                             Read-tool families carved off tools.rs at the 800-cap
write_tools.rs               Core node write tools (insert/update/delete/move/copy/replace)
write_tools_import_svg.rs    SVG import arm of write_tools
write_tools_variables.rs     Variable-write arm of write_tools
node_attr_tools.rs           Per-node attribute writers (rotation/text/font/stroke/fill/name)
selected_ops_tools.rs        Selection ops (dup/delete/nudge/group/align/clipboard)
component_tools.rs           Components + selection/viewport/flag/tool/undo tools
page_tools.rs                Page CRUD tools
batch_design.rs              BatchDesign tool + hand-rolled nodes_json parser
batch_design_{dsl,normalize,fill_normalize,result,wire}.rs
                             The batch_design DSL, normalizers, and wire shapes
batch_program*.rs            The `I(parent, node)` program DSL: parse / resolve / exec
batch_program_error.rs       `ProgramError` (typed InvalidValue / ValueOutOfRange variants)
batch_layered.rs             Layered design workflow
batch_layered_guidelines.rs  Its guideline text (split off at the cap)
scalar_vars.rs               Scalar + Color variables + create/delete/rename_variable
script_runner.rs             Sandboxed JS runner behind `batch_design { script }`
element_tools.rs             insert_<component> kit-component tools
*_tests.rs                   Per-module sibling test files (800-line cap convention)
mcp_tests.rs                 Cross-cutting: stdio dispatch, parser invariants
```

### Host wiring

`openpencil-desktop --mcp <path>` (`crates/op-host-services/src/mcp_serve.rs`, shared by the desktop binary and `op-host-web-server` via `op_host_services::cli_modes`) runs a JSON-RPC stdio MCP server backed by the .op file at `<path>`. External CLIs (Claude Code / Codex / OpenCode / Kiro / Copilot / Antigravity / Grok Build) spawn the binary in this mode to drive the Rust editor. The module is itself a spine over `mcp_serve/` siblings: `schemas.rs` (the `TOOL_SCHEMAS` catalog), `wire.rs`, `sniff.rs` (top-level method / id sniffing), `error.rs` (`McpServeError`, which separates protocol faults from socket failures), `file_path.rs`, `doc_sync.rs`, and the export / screenshot tool arms.

- Handshake: `initialize` returns protocol version + capabilities + serverInfo; `tools/list` enumerates the full catalog with JSON inputSchemas; `notifications/initialized` + `ping` handled inline.
- Per-call lifecycle: re-build the `ToolRegistry` against the live document (so read-tool snapshots reflect prior writes) → dispatch through `run_stdio_with_applier` → applier closure mutates the doc + saves to disk on each successful write.
- Top-level method / id sniffing uses the same key-walker discipline as the wire parser so nested keys can't shadow the real top-level fields.

### Pending

- A real JSON Node parser would unlock `replace_node`'s subtree path + grow batch_design / design_skeleton beyond leaf-only.
- Per-phase apply semantics for the design\_\* workflow (e.g. design_refine emitting UpdateNode batches against existing nodes instead of fresh inserts).
- HttpServer / streamable-http MCP transport (lifecycle scaffold exists in `chat_http_server.rs`; wire protocol unverified).

## AI chat (real provider integration)

The floating chat panel (`widgets/ai_chat_panel.rs`) is wired to
real CLI agents, not a stub:

- `ChatState::begin_send` (shell-core `document/chat.rs`) — the
  native send path: pushes the user message + an empty assistant
  bubble, raises `chat.pending_send`. The web shell (with the
  `codegen` feature — the production bundle) drains it into
  `web_chat.rs` and streams the turn through the daemon's
  `/api/ai/stream`; a transport-less build reports an honest
  per-send error (the old `send()` echo stub is retired from the
  web send path).
- `ChatProvider` (shell-core `chat_provider.rs`) is the
  transport-free trait; real impls live desktop-side:
  `chat_runtime.rs` (`BuiltInProvider`, agent-rs), `chat_claude.rs`
  (`ClaudeCodeProvider`, `anthropic-agent-sdk`), `chat_subprocess.rs`
  (`SubprocessProvider` for Codex/Antigravity/Grok Build),
  `chat_copilot.rs`, `chat_http_server.rs` (OpenCode).
- `chat_session.rs` — `ChatSession` runs a turn on a worker thread
  (`ChatProvider::send` is a blocking iterator). `launch_if_pending`
  drains `pending_send` → `provider_for_agent(chat_selected_agent)`;
  `pump` streams deltas into the transcript each frame. The winit
  loop wakes ~30 fps while a turn runs.
- Model chip — the chat panel's bottom-left chip shows the selected
  agent (`AgentProvider::name`); clicking it cycles
  `ui.chat_selected_agent` through `agent_settings.connected` via
  `Document::cycle_chat_agent` (`AIChatHit::CycleModel`).
- Codex / OpenCode have no `ChatProvider` bridge yet — selecting
  them writes an explicit `error: … not wired yet` into the
  transcript rather than silently rerouting to another agent.

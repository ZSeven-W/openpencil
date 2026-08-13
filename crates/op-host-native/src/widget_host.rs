//! Step 4 native widget glue — the only file in shell-native allowed
//! to call into `op_editor_ui::widgets`. Mirrors the
//! shell-web `widget_host.rs` so the editor-UI composition is
//! cross-platform: same widget code, same paint output.
//!
//! ### Module layout
//!
//! This file is the public spine. Implementation methods are split
//! across sibling submodules (per the 800-line-per-file ceiling):
//! - `NativeFrameBackend` (`RenderBackend` impl) — moved to `crate::backend`
//! - [`helpers`] — hex parsing + resize-bounds math + constants
//! - [`geometry`] — canvas region / cursor hint / picker rect helpers
//! - [`input`] — cursor-move / release / panel-resize input handlers
//! - [`scroll`] — wheel + trackpad-pan (zoom / canvas + diff scroll)
//! - [`press`] — `apply_press` + new-node spawn (largest method)
//! - [`git_press`] — Git-panel press dispatch

use op_editor_core::PreviewDeviceKind;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::host_frame_bookkeeping as bookkeeping;
use op_editor_ui::widgets::SelectionHandle;
use op_editor_ui::{Rect, Theme};

mod a11y;
mod account_press;
#[cfg(test)]
mod account_press_tests;
#[cfg(test)]
mod agent_settings_acp_tests;
#[cfg(test)]
mod agent_settings_compact_press_tests;
#[cfg(test)]
mod agent_settings_form_press_tests;
mod agent_settings_geometry;
#[cfg(test)]
mod agent_settings_image_gen_tests;
#[cfg(test)]
mod agent_settings_tests;
mod agent_settings_touch;
#[cfg(test)]
mod agent_settings_touch_tests;
mod ai_chat_geometry;
#[cfg(test)]
mod ai_chat_minimized_tests;
mod arc_drag;
mod auth_flow;
mod blur_inputs;
#[cfg(test)]
mod blur_inputs_tests;
mod canvas_pan_cache;
mod canvas_select_drag;
#[cfg(test)]
mod canvas_select_drag_tests;
mod chat_design_apply;
#[cfg(test)]
mod chat_design_apply_tests;
mod chat_design_hover;
#[cfg(test)]
mod chat_design_hover_tests;
#[cfg(test)]
mod chat_input_caret_tests;
mod chat_model_picker_caret;
#[cfg(test)]
mod chat_model_picker_caret_tests;
#[cfg(test)]
mod chat_send_tests;
#[cfg(test)]
mod chat_style_card_tests;
mod click;
#[cfg(test)]
mod codegen_framework_tests;
#[cfg(feature = "gl-host")]
mod collab_host_impl;
#[cfg(test)]
mod collab_input_tests;
#[cfg(test)]
mod collaboration_id_tests;
#[cfg(test)]
mod collaboration_install_tests;
mod collaboration_policy;
mod color_picker_press;
mod component_browser_press;
mod cursor_hint;
mod cursor_move_chrome;
mod cursor_move_ctx;
mod cursor_move_drags;
mod cursor_move_modals;
mod cursor_move_overlays;
mod cursor_move_panels;
#[cfg(test)]
mod deferred_press_tests;
mod design_md_press;
#[cfg(test)]
mod design_md_press_tests;
#[cfg(test)]
mod document_epoch_tests;
#[cfg(test)]
mod export_quick_menu_tests;
mod figma_import_scroll;
#[cfg(test)]
mod figma_import_tests;
#[cfg(test)]
mod file_menu_press_tests;
#[cfg(test)]
mod font_generation_scene_tests;
mod font_picker_dispatch;
#[cfg(test)]
mod font_picker_keyboard_tests;
mod geometry;
mod geometry_settings_hover;
#[cfg(test)]
mod git_panel_placement_tests;
mod git_press;
mod helpers;
mod history_guard;
mod host_lifecycle;
mod host_requests;
mod html_import_diagnostics_dispatch;
mod icon_picker_press;
#[cfg(test)]
mod icon_picker_press_tests;
mod image_crop_drag;
#[cfg(test)]
mod image_crop_drag_tests;
mod image_drop_target;
mod image_panel_dispatch;
#[cfg(test)]
mod image_panel_overlay_tests;
mod image_panel_selection;
#[cfg(test)]
mod image_panel_selection_tests;
mod ime;
#[cfg(test)]
mod ime_punctuation_tests;
mod input;
#[cfg(test)]
mod input_clipboard_tests;
#[cfg(test)]
mod input_drag_tests;
#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod instance_panel_tests;
mod keyboard;
mod keyboard_caret;
mod keyboard_clipboard;
mod keyboard_delete;
mod keyboard_escape;
mod keyboard_send;
mod missing_fonts_dispatch;
#[cfg(test)]
mod mobile_touch_tests;
mod mode_transition_host;
#[cfg(test)]
mod overlay_cursor_tests;
mod overlay_rects;
#[cfg(test)]
mod page_switch_center_tests;
mod paint;
mod paint_chrome_menus;
mod paint_floating_panels;
mod paint_mobile;
mod paint_pan_cache;
mod paint_rail;
mod paint_topmost_overlays;
#[cfg(test)]
mod pan_cache_scroll_tests;
#[cfg(test)]
mod pan_cache_tests;
#[cfg(test)]
mod panel_history_tests;
mod pen_press;
#[cfg(test)]
mod pen_press_tests;
mod press;
mod press_canvas_tiers;
mod press_chrome_tiers;
mod press_ctx;
mod press_helpers;
mod press_overlay_tiers;
mod press_property_tiers;
mod press_surface_tiers;
mod preview_edge_swipe;
#[cfg(all(test, not(target_os = "windows")))]
mod preview_edge_swipe_tests;
mod preview_frame;
#[cfg(test)]
mod preview_frame_geometry_tests;
#[cfg(all(test, not(target_os = "windows")))]
mod preview_frame_tests;
mod preview_input;
mod preview_slideshow;
#[cfg(test)]
mod preview_slideshow_tests;
#[cfg(test)]
mod prompt_center_host_tests;
mod prompt_center_press;
#[cfg(test)]
mod property_compositing_tests;
mod property_dispatch;
#[cfg(test)]
mod property_keyboard_occlusion_tests;
mod property_layout_dispatch;
#[cfg(test)]
mod property_panel_interactions_tests;
#[cfg(test)]
mod property_panel_press_tests;
mod property_popovers;
mod property_scroll;
mod release;
mod release_feedback;
mod responsive_geometry;
mod scene_state;
#[cfg(test)]
mod scene_template_host_tests;
#[cfg(test)]
mod scene_template_ime_tests;
mod scene_template_install;
mod scene_template_press;
#[cfg(test)]
mod scene_template_touch_tests;
mod screen_switcher;
#[cfg(all(test, not(target_os = "windows")))]
mod screen_switcher_tests;
mod scroll;
#[cfg(test)]
mod scroll_direction_tests;
#[cfg(test)]
mod scroll_tests;
mod settings_caret;
#[cfg(test)]
mod settings_caret_tests;
mod settings_dispatch;
mod shape_picker_press;
#[cfg(test)]
mod shortcut_surface_tests;
mod shortcuts;
mod slides_panel;
mod slides_panel_thumbs;
mod text_edit_press;
// Windows CI DirectWrite/Skia text layout aborts inside these text-edit
// fixtures before Rust can report an assertion. macOS + Linux keep coverage.
#[cfg(all(test, not(target_os = "windows")))]
mod text_edit_press_tests;
#[cfg(test)]
mod theme_tests;
mod toolbar_actions;
mod toolbar_hover;
#[cfg(test)]
mod top_bar_tooltip_tests;
mod touch_panel_gesture;
#[cfg(test)]
mod touch_panel_gesture_tests;
#[cfg(test)]
mod variables_panel_add_tests;
mod variables_panel_commit;
mod variables_panel_geometry;
mod variables_panel_press;
mod variables_panel_row_press;
#[cfg(test)]
mod variables_panel_tests;
#[cfg(test)]
mod variables_panel_ux_tests;
mod variables_preset_press;
mod viewport_fit;

/// Cursor affordance the host suggests for a given screen point — re-exported
/// from `jian-core` so widgets (via `cursor_at`) and hosts share one vocabulary.
/// The runner maps each variant to its native cursor (`CursorIcon` on desktop,
/// CSS `cursor:` string on web). Domain/canvas cursor decisions (active tool,
/// selection handles, resize gutters) stay host-side — see `cursor_for_handle`
/// and `geometry::cursor_hint`.
pub use jian_core::CursorHint;

/// Map a selection handle to its resize cursor. Stays host-side because it
/// depends on the OP `SelectionHandle` type, which is not a jian-atomic concern.
pub(in crate::widget_host) fn cursor_for_handle(h: SelectionHandle) -> CursorHint {
    match h {
        SelectionHandle::Left | SelectionHandle::Right => CursorHint::ResizeEw,
        SelectionHandle::Top | SelectionHandle::Bottom => CursorHint::ResizeNs,
        SelectionHandle::TopLeft | SelectionHandle::BottomRight => CursorHint::ResizeNwse,
        SelectionHandle::TopRight | SelectionHandle::BottomLeft => CursorHint::ResizeNesw,
    }
}

/// Native counterpart of shell-web's `WidgetHost`. Owns the
/// canonical-model `EditorState` as its single source of truth +
/// composes the editor UI per frame in the TS-equivalent layout.
pub struct WidgetHostNative {
    /// **The host's single source of truth.** All input handlers
    /// mutate this; paint + the input hit-test read the derived
    /// `layout_scene` rebuilt from it (see `refresh_layout_scene`).
    pub(in crate::widget_host) editor_state: op_editor_core::EditorState,
    /// Derived paint-only `LayoutScene` of `editor_state` — the
    /// layout-resolved render tree the `CanvasViewport` paints AND
    /// the host's canvas hit-test queries. Rebuilt lazily by
    /// `refresh_layout_scene()` whenever `editor_state_dirty` is set.
    pub(in crate::widget_host) layout_scene: op_editor_ui::layout_scene::LayoutScene,
    /// Paint-only interpolation from the previous layout scene to the
    /// current one. Used by canvas node drag/reorder previews.
    pub(in crate::widget_host) layout_transition:
        Option<op_editor_ui::widgets::CanvasLayoutTransition>,
    /// Skips the `layout_scene` rebuild when the document / active theme /
    /// active page are unchanged. `editor_state_dirty` fires on nearly every
    /// interaction (hover, scroll, selection, caret drafts, chat streaming),
    /// but most leave the scene inputs identical — this guards the rebuild.
    pub(in crate::widget_host) scene_cache: op_pen_loader::SceneBuildCache,
    /// Set whenever `editor_state` is mutated. Drives the lazy
    /// rebuild of `layout_scene` — `refresh_layout_scene()` rebuilds
    /// + clears the flag, so a sequence of mutations re-derives once.
    pub(in crate::widget_host) editor_state_dirty: bool,
    /// Monotonic counter bumped every time the WHOLE `editor_state` is
    /// replaced (Open / New / import) — never on an in-place edit or a
    /// save. Async work captured against a document (e.g. a clipboard
    /// paste decode on a worker thread) reads this at dispatch and
    /// re-checks it before applying its result, so a result decoded
    /// for a document that has since been replaced is dropped instead
    /// of landing in the wrong document.
    pub(in crate::widget_host) document_epoch: u64,
    /// The `jian_skia` font-registry generation the current `layout_scene`
    /// was built against. A runtime font import/removal bumps that
    /// generation WITHOUT dirtying `editor_state`, so `refresh_layout_scene`
    /// watches this to force a rebuild — otherwise an already-open document
    /// keeps its stale fallback-font layout until an unrelated dirty event.
    pub(in crate::widget_host) layout_scene_font_generation: u64,
    pub(in crate::widget_host) theme: Theme,
    /// Active canvas pan-drag state — left-button press → motion
    /// → release.
    pub(in crate::widget_host) drag: Option<DragState>,
    /// Pending one-finger gesture inside the touch Agent Settings body.
    /// Body actions commit on release; motion past the touch slop promotes
    /// the press to scrolling without leaking through to the canvas.
    pub(in crate::widget_host) agent_settings_touch_gesture:
        Option<agent_settings_touch::AgentSettingsTouchGesture>,
    /// Pending one-finger gesture inside a touch Property / Layers / Slides surface.
    /// A stationary release replays the press; crossing touch slop scrolls.
    pub(in crate::widget_host) touch_panel_gesture: Option<touch_panel_gesture::TouchPanelGesture>,
    /// True while Space is held — transient pan mode (TS parity):
    /// canvas presses pan regardless of the active tool.
    pub(in crate::widget_host) space_pan: bool,
    /// Last cursor position the canvas-hover hit-test ran at —
    /// sub-3px moves skip the tree walk (cost guard).
    pub(in crate::widget_host) last_hover_probe: Option<(f32, f32)>,
    /// Active chat-panel drag state — present while the user
    /// drags the floating AI chat panel by its header. Holds the
    /// transient panel top-left position so paint can place the
    /// panel at the cursor instead of its anchor; on release the
    /// host computes the nearest corner via `ChatAnchor::nearest`
    /// and snaps.
    pub(in crate::widget_host) chat_drag: Option<ChatDragState>,
    /// Active chat-panel resize — mirrors the TS panel's invisible
    /// edge/corner handles.
    pub(in crate::widget_host) chat_resize: Option<ChatResizeState>,
    /// Active Design-MD panel drag — present while the user drags the
    /// floating panel by its header bar. The live top-left is written
    /// straight back into `editor_ui.design_md_panel.pos`.
    pub(in crate::widget_host) design_md_drag: Option<DesignMdDragState>,
    /// Active Component-Browser panel drag.
    pub(in crate::widget_host) component_browser_drag: Option<ComponentBrowserDragState>,
    /// Active Icon-picker panel drag.
    pub(in crate::widget_host) icon_picker_drag: Option<IconPickerDragState>,
    /// Active image-fill adjustment slider drag in the floating
    /// property popover.
    pub(in crate::widget_host) image_adjustment_drag: Option<op_editor_core::ImageAdjustmentField>,
    /// Active bitmap pan while the selected image fill is in crop edit mode.
    pub(in crate::widget_host) image_crop_drag: Option<image_crop_drag::ImageCropDragState>,
    pub(in crate::widget_host) effect_radius_drag: Option<usize>,
    /// Active generated-code preview text selection drag.
    pub(in crate::widget_host) code_selection_drag: Option<CodeSelectionDragState>,
    /// Active chat input text selection drag.
    pub(in crate::widget_host) chat_input_selection_drag: Option<ChatInputSelectionDragState>,
    /// Active Search / Generate popover input selection drag.
    pub(in crate::widget_host) image_input_selection_drag:
        Option<image_panel_selection::ImageInputSelectionDragState>,
    /// Latest Search / Generate input geometry measured by the real painter.
    pub(in crate::widget_host) image_input_geometry:
        Option<op_editor_ui::widgets::property_panel_image_assets::ImagePopoverInputGeometry>,
    /// Active chat transcript text selection drag.
    pub(in crate::widget_host) chat_text_selection_drag: Option<ChatTextSelectionDragState>,
    /// Active inline canvas text-edit selection drag — press inside
    /// the edited Text node placed the caret; dragging extends the
    /// selection from the press offset.
    pub(in crate::widget_host) text_edit_selection_drag: Option<TextEditSelectionDragState>,
    /// Lazily-created measure-only Skia backend for text-edit
    /// hit-testing OUTSIDE the paint pass (press / drag / arrow-key
    /// line mapping). Same `measure_text_weighted` implementation the
    /// paint backend uses, so hit geometry matches painted glyphs. Interior
    /// mutability also lets read-only TopBar geometry (hover + Git popover
    /// placement) use the same metrics without widening those APIs to `&mut`.
    pub(in crate::widget_host) text_measure: std::cell::RefCell<Option<crate::NativeBackend>>,
    /// Active panel-resize drag — set when the cursor is pressed
    /// within the resize gutter of LayerPanel's right edge or
    /// PropertyPanel's left edge.
    pub(in crate::widget_host) panel_resize: Option<PanelResize>,
    /// Active floating-VariablesPanel resize drag (right / bottom /
    /// corner edge, TS pointer-capture handles). The live size is
    /// written into `editor_ui.variables_panel_size`.
    pub(in crate::widget_host) variables_resize:
        Option<op_editor_ui::widgets::variables_panel::VariablesResizeEdge>,
    /// Active node-drag — set when the user presses on a node in
    /// the canvas with the Select tool. Tracks the document-space
    /// cursor anchor so each `apply_cursor_move` translates the
    /// selected node by the delta.
    pub(in crate::widget_host) node_drag: Option<NodeDragState>,
    /// Original selected ids for an active Option-drag clone move.
    /// Drop hit-testing skips these so a fresh clone does not
    /// immediately reparent back into the source it overlaps.
    pub(in crate::widget_host) option_drag_source_ids: Vec<op_editor_core::NodeId>,
    /// Active handle-drag — set when the user pressed on one of
    /// the 8 selection handles. Carries the start screen anchor +
    /// the original bounds so each move computes a fresh
    /// `new_bounds = start_bounds + delta`.
    pub(in crate::widget_host) handle_drag: Option<HandleDragState>,
    /// Active rotation drag — set when the user pressed in the
    /// rotation ring just outside one of the four corners.
    pub(in crate::widget_host) rotate_drag: Option<RotateDragState>,
    /// Active shape-create drag — set when the user presses
    /// empty canvas with a shape tool selected.
    pub(in crate::widget_host) create_drag: Option<CreateDragState>,
    /// Active marquee rect-select drag — set when the user
    /// presses empty canvas with the Select tool. On release,
    /// every top-level node whose bounds intersect the marquee
    /// joins the selection (replaces if `additive == false`,
    /// toggles each if `additive == true`).
    pub(in crate::widget_host) marquee_drag: Option<MarqueeDragState>,
    /// Active layer drag-to-reorder gesture — set when the user
    /// presses a LayerPanel row with the press-y exceeding a
    /// small threshold during move. Carries the source NodeId and
    /// the live cursor y in panel-local space. Resolved on
    /// release into `Document::reorder_before` /
    /// `reorder_after` via `LayerPanel::drop_target_at`.
    pub(in crate::widget_host) layer_drag: Option<LayerDragState>,
    /// Active path-anchor drag — set when the user presses on a
    /// per-anchor handle of a selected Path node with the Pen tool.
    /// Each `apply_cursor_move` snaps the anchor to the current
    /// document-space cursor; release commits a history snapshot.
    pub(in crate::widget_host) path_anchor_drag: Option<PathAnchorDragState>,
    /// Active ellipse arc-handle drag — set when the user presses on
    /// a start / sweep / inner-radius handle of a selected Ellipse.
    /// Each move re-applies `SetEllipseArc`; release commits history.
    pub(in crate::widget_host) arc_handle_drag: Option<ArcHandleDragState>,
    /// Counter for minting fresh `NodeId`s for newly-created nodes.
    /// Bumped past the highest sample id so new + sample nodes
    /// never collide on the same key.
    pub(in crate::widget_host) next_node_id: u64,
    /// Session-owned allocator for collaboration-authored ids. Standalone
    /// paths keep using `next_node_id`; while this is `Some`, every supported
    /// document creation path must use this allocator instead.
    pub(in crate::widget_host) collab_id_allocator: Option<op_editor_core::DocumentIdAllocator>,
    /// Host-supplied frame timestamp in milliseconds. Focused
    /// `TextInputState`s use this for caret blink. The
    /// inspector_window runner refreshes this once per
    /// `RedrawRequested` from a single `Instant` start anchor;
    /// any other host (mobile / browser) installs its own clock.
    pub(in crate::widget_host) now_ms: u64,
    /// Millisecond deadline until which a pan/zoom gesture counts as
    /// live. While `now_ms` is before it, the canvas paints in
    /// interactive-degrade mode (effect layers + sub-pixel leaves
    /// skip); the animation scheduler wakes at the deadline so the
    /// gesture-end frame repaints at full quality.
    pub(in crate::widget_host) interaction_hot_until_ms: u64,
    /// Offscreen canvas layer serving pure-pan frames during a live
    /// gesture. See `widget_host/canvas_pan_cache.rs`.
    pub(in crate::widget_host) pan_cache: Option<canvas_pan_cache::CanvasPanCache>,
    /// Progressive gesture-end quality restore over `pan_cache`.
    pub(in crate::widget_host) pan_cache_restore: Option<canvas_pan_cache::PanCacheRestore>,
    /// Frames served by a pan-cache blit (test observability).
    pub(in crate::widget_host) pan_cache_blits: u64,
    /// In-place scroll refreshes performed (test observability).
    pub(in crate::widget_host) pan_cache_scrolls: u64,
    /// Full expanded-layer builds performed (test observability).
    pub(in crate::widget_host) pan_cache_builds: u64,
    /// Whether the most recent gesture tick was a zoom — zoom frames
    /// never build the pan cache (each tick would invalidate it).
    pub(in crate::widget_host) last_gesture_was_zoom: bool,
    /// Whether the shift key is currently held. Runners update
    /// this via `set_modifier_shift` on every modifier change.
    /// Drives shift+click multi-select in `apply_press`.
    pub(in crate::widget_host) shift_held: bool,
    /// Whether Alt/Option is currently held. Node dragging uses this
    /// to duplicate the current selection before moving it.
    pub(in crate::widget_host) alt_held: bool,
    /// Last viewport size seen by paint/press. Used by handlers
    /// that don't receive viewport dims (e.g. apply_cursor_move
    /// driving the color-picker drag).
    /// In-flight browser device-login flow handle from `op-auth-bridge`
    /// (`None` = no login running). Polled by the desktop event loop via
    /// [`Self::poll_auth`].
    pub(in crate::widget_host) auth_login_handle: Option<u64>,
    /// Verification URL waiting for the desktop host to open in the
    /// system browser (drained by `take_pending_browser_url`).
    pub(in crate::widget_host) auth_pending_browser_url: Option<String>,
    /// Whether the current flow's verification URL was already queued —
    /// the flow reports `WaitingApproval` every poll, but the browser
    /// must open exactly once.
    pub(in crate::widget_host) auth_browser_opened: bool,
    /// Short post-restore window during which the desktop keeps polling the
    /// session handle. The private runtime revalidates persisted credentials
    /// asynchronously, so profile fields can become fresher after startup.
    pub(in crate::widget_host) auth_session_refresh_deadline: Option<std::time::Instant>,
    /// Opaque process-local hash of the last mirrored avatar source. This keeps
    /// repeated session polls idempotent without retaining another signed URL.
    pub(in crate::widget_host) auth_account_avatar_revision: Option<u64>,
    /// Rect the toast banner painted last frame, or `None` when nothing
    /// painted. Cached because the banner's width is measured in the font it
    /// paints with, which only the paint pass can do — the press arm re-checks
    /// the toast is still live before trusting it (see
    /// `op_editor_ui::widgets::editor_toast_flow::press`).
    pub(in crate::widget_host) toast_rect: Option<op_editor_ui::Rect>,
    pub(in crate::widget_host) last_viewport_w: f32,
    pub(in crate::widget_host) last_viewport_h: f32,
    pub(in crate::widget_host) keyboard_occlusion: f32,
    /// Live canvas Preview (Play) session — `Some` while
    /// `editor_state.editor_ui.preview.mode` is set. Owns the jian
    /// `Runtime` (host-local; `!Send`). Built on enter from the
    /// document JSON, dropped on exit so the document stays untouched.
    pub(in crate::widget_host) preview: Option<crate::preview::PreviewSession>,
    pub(in crate::widget_host) preview_device_frame: Option<preview_frame::DeviceFrame>,
    pub(in crate::widget_host) preview_scroll_y: f32,
    pub(in crate::widget_host) preview_manual_pick: Option<PreviewDeviceKind>,
    pub(in crate::widget_host) preview_surface_capture: Option<preview_frame::PreviewSurface>,
    /// Track M-1: an in-flight canvas ↔ device-frame merge animation —
    /// `Some(Enter)` for the brief window right after `enter_preview`
    /// installed `self.preview`; `Some(Exit)` for the window between
    /// `exit_preview` being CALLED and the runtime actually dropping
    /// (see `exit_preview`'s doc — the drop is deferred to
    /// `settle_mode_transition` so the merge-back animation has
    /// something live to paint). `None` the rest of the time.
    pub(in crate::widget_host) preview_mode_transition: Option<crate::preview::ModeTransition>,
    /// Live preview pointer-drag state: `true` between a canvas Down
    /// and its Up, so cursor moves dispatch as drags (slider knob)
    /// instead of hovers.
    pub(in crate::widget_host) preview_press_active: bool,
    /// Last preview pointer position in DOCUMENT space — the release
    /// dispatches its Up here (the OS reports release without coords).
    pub(in crate::widget_host) preview_last_doc: Option<(f32, f32)>,
    /// Screen-space point a presenting press landed on, held until the
    /// release decides whether the gesture was a click (advance the deck) or
    /// a drag (do nothing). `None` outside a slideshow press.
    ///
    /// Screen space, not document space: the presenting stage covers the
    /// hidden rails, which the editor's screen-to-document mapping still
    /// treats as off-canvas, and a press there would have no document point
    /// at all. Screen coordinates also make the slop comparison the same
    /// numbers the threshold is written in.
    pub(in crate::widget_host) slideshow_press_screen: Option<(f32, f32)>,
    /// Last cursor position seen while presenting — the release's other half
    /// of the click-versus-drag comparison.
    pub(in crate::widget_host) slideshow_cursor: Option<(f32, f32)>,
    /// Track C-4: the SCREEN-space x a preview press started at, when
    /// that press began within the edge-swipe dead zone (device-frame
    /// content-local x < 24px) — the iOS-style "swipe from the left
    /// edge to go back" candidate. `None` when no candidate is being
    /// tracked (press started elsewhere, or preview isn't in App Mode /
    /// there's nowhere to pop to).
    pub(in crate::widget_host) preview_edge_swipe_start_x: Option<f32>,
    /// Stable, process-unique id scoping this host's chat-panel transcript
    /// cache. Allocated once at construction and stamped onto every
    /// `AIChatPlaceholder` this host builds (`.owned_by`), so the display-frame
    /// cursor-shape hint (`hit_test_current_build`) only reads a build THIS
    /// panel resolved — never one a different panel left in the thread-local
    /// slot after a tab/host switch.
    pub(in crate::widget_host) chat_panel_owner: u64,
    /// Stable, process-unique id scoping this host's LayerPanel row-model
    /// cache. Stamped onto the per-frame paint build (`from_editor_owned`)
    /// so the thread-local slot is owned by this panel and never
    /// cross-served to another host on a revision-counter collision. Page
    /// switches don't rotate (`active_page_index` is in the key), but
    /// whole-document replacements do — see
    /// `force_rotate_layer_panel_owner` (revision counters restart at 0,
    /// so the key alone can alias across documents).
    pub(in crate::widget_host) layer_panel_owner: u64,
    /// Rendered board thumbnails for the left rail's slides tab. Its own
    /// cache, keyed on `(board id, document revision)` — deliberately
    /// disjoint from the scene cache and the canvas pan cache.
    pub(in crate::widget_host) slide_thumbs: slides_panel_thumbs::SlideThumbCache,
    /// Active chat-session (tab) index observed at the last owner rotation.
    /// When the active session changes, [`Self::rotate_chat_owner_if_session_changed`]
    /// rotates `chat_panel_owner` so the new tab's transcript never reads the
    /// previous tab's cached geometry via the 0-hash cursor-shape hint (it reads
    /// `None` — the default arrow — until the new tab's first paint re-stores the
    /// slot under the rotated owner).
    pub(in crate::widget_host) last_chat_session_index: usize,
}

// Transient pointer-drag records that carry no platform types are
// single-sourced in `op_editor_core::host_drag_state` (the web spine
// declared byte-identical copies). Re-exported here under their
// historical `widget_host` names so every submodule's `use super::{…}`
// and struct-literal site stays unchanged. The three floating-panel
// header drags collapse onto one shape — they only ever carried the
// grab offset.
pub(in crate::widget_host) use op_editor_core::host_drag_state::{
    AnchorDragTarget, ChatDragState, ChatInputSelectionDragState, ChatTextSelectionDragState,
    CodeSelectionDragState, DragState, LayerDragState, MarqueeDragState, PathAnchorDragState,
    TextEditSelectionDragState,
};
pub(in crate::widget_host) use op_editor_core::host_drag_state::{
    PanelDragState as ComponentBrowserDragState, PanelDragState as DesignMdDragState,
    PanelDragState as IconPickerDragState,
};

/// Which panel edge is being dragged, plus the press anchor +
/// the panel width at press time. Live width is computed as
/// `start_width + (live_x - start_x) * sign`.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct PanelResize {
    pub(in crate::widget_host) kind: PanelResizeKind,
    pub(in crate::widget_host) start_x: f32,
    pub(in crate::widget_host) start_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelResizeKind {
    LayerRight,
    PropertyLeft,
}

/// Active node-drag — tracks the previous cursor position in
/// SCREEN coordinates. Each `apply_cursor_move` divides the
/// screen-space delta by the active zoom to get a doc-space
/// translation, which sidesteps canvas_region offset math (the
/// offset cancels for incremental deltas).
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct NodeDragState {
    pub(in crate::widget_host) last_screen_x: f32,
    pub(in crate::widget_host) last_screen_y: f32,
    /// Press point in screen px — the drag-threshold anchor.
    pub(in crate::widget_host) press_screen_x: f32,
    pub(in crate::widget_host) press_screen_y: f32,
    /// Latches true once the cursor travels past `NODE_DRAG_THRESHOLD_PX`
    /// so a pure click with sub-pixel jitter never moves anything.
    pub(in crate::widget_host) moved: bool,
    /// Net cursor travel since the press in DOC px (`(cursor - press)
    /// / zoom`, refreshed each move). Flex-flow children never
    /// doc-translate during the drag, so the release commit adds this
    /// to their scene bounds to find the dropped position.
    pub(in crate::widget_host) total_dx: f64,
    pub(in crate::widget_host) total_dy: f64,
    /// Cursor-following resolved bounds for same-container flex
    /// drags. Paint hides the in-flow selected node and draws a
    /// floating copy at these bounds.
    pub(in crate::widget_host) overlay_bounds: Option<Rect>,
}

/// Active handle-drag — captures the press cursor anchor + the
/// node's starting bounds. Each `apply_cursor_move` computes the
/// new bounds from a screen-space delta inverted by zoom and
/// writes it via `Document::set_selected_bounds`.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct HandleDragState {
    pub(in crate::widget_host) handle: SelectionHandle,
    pub(in crate::widget_host) start_screen_x: f32,
    pub(in crate::widget_host) start_screen_y: f32,
    pub(in crate::widget_host) start_bounds: Rect,
    /// Authored parent-relative origin at press time. Kept separate from the
    /// resolved absolute scene bounds so left/top drags of nested free nodes
    /// do not jump into document coordinates.
    pub(in crate::widget_host) start_authored_x: Option<f64>,
    pub(in crate::widget_host) start_authored_y: Option<f64>,
}

/// Active rotation drag — `center_screen` is the screen-space
/// centre of the selected node's bounds at press time; the
/// rotation angle is `atan2(y - cy, x - cx)`. We snapshot the
/// initial cursor angle and the node's starting rotation so each
/// move computes `rotation = start_rotation + (cursor_angle -
/// start_cursor_angle)`.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct RotateDragState {
    pub(in crate::widget_host) center_screen_x: f32,
    pub(in crate::widget_host) center_screen_y: f32,
    pub(in crate::widget_host) start_cursor_angle: f32,
    pub(in crate::widget_host) start_rotation: f32,
}

/// Active shape-create drag — set when the user presses an empty
/// canvas point with a shape tool (Rect / Ellipse / Polygon /
/// Line / Pen / Frame / Text) selected. The new node is created
/// at press time and resized on each move; the document's
/// selection still points at it, so we don't need to carry the
/// id on the drag state itself.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct CreateDragState {
    pub(in crate::widget_host) start_doc_x: f32,
    pub(in crate::widget_host) start_doc_y: f32,
}

/// Ellipse arc-handle drag — tracks which arc handle of which
/// Ellipse is being dragged. Move re-applies `SetEllipseArc`;
/// release commits a history snapshot only when the arc changed.
#[derive(Debug, Clone)]
pub(in crate::widget_host) struct ArcHandleDragState {
    pub(in crate::widget_host) node_id: op_editor_core::NodeId,
    pub(in crate::widget_host) handle: op_editor_ui::widgets::ArcHandle,
    /// Press cursor doc point — the move handler gates `moved` on
    /// real motion from here so a press-release pushes no undo entry.
    pub(in crate::widget_host) start_doc: op_editor_ui::Point2D,
    /// Set true on the first cursor-move that actually moves the arc.
    pub(in crate::widget_host) moved: bool,
    /// Snapshot captured at drag-start; pushed only if `moved`.
    pub(in crate::widget_host) pre_drag_snapshot: op_editor_core::EditorSnapshot,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct ChatResizeState {
    pub(in crate::widget_host) edge: op_editor_ui::widgets::ChatResizeEdge,
    pub(in crate::widget_host) start_x: f32,
    pub(in crate::widget_host) start_y: f32,
    pub(in crate::widget_host) start_rect: Rect,
}

impl WidgetHostNative {
    /// Refresh host-level theme tokens from the canonical editor UI
    /// state. Most widgets derive their theme directly, but a few
    /// paint-layer affordances still read this host cache.
    pub(in crate::widget_host) fn sync_theme_from_editor(&mut self) {
        self.theme =
            op_editor_ui::widgets::editor_state_ext::theme_for(&self.editor_state.editor_ui);
    }

    /// Run a path boolean op on the active selection (Union /
    /// Subtract / Intersect / Exclude). Backed by skia's `Path::op`.
    /// Returns true when the op committed (≥ 2 Path nodes were
    /// selected + the result yielded a non-empty polyline).
    #[cfg(feature = "gl-host")]
    pub fn apply_boolean_op(&mut self, op: op_editor_core::BooleanOp) -> bool {
        // Codex stop-gate: boolean op shortcuts (Cmd+Alt+U/S/I/X)
        // mutate the document — commit any pending variable-row
        // edit first so the dirty draft lands before this op runs.
        self.commit_variable_row_focus_if_any();
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::NodeReplacement,
            ),
        ) {
            return true;
        }
        // The skia `Path::op` math runs against the layout-resolved
        // `LayoutScene` + the editor selection; the result polyline
        // is committed back through an `EditorState` mutator so the
        // host never edits the canonical tree directly.
        self.refresh_layout_scene();
        let selected: Vec<String> = self
            .editor_state
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        let outcome = crate::boolean_ops::compute_boolean_op(&self.layout_scene, &selected, op);
        let Some(result) = outcome else {
            return false;
        };
        // Scene ids are the canonical `.op` ids — wrap straight into
        // `op_editor_core::NodeId`.
        let source_ids: Vec<op_editor_core::NodeId> = result
            .source_ids
            .iter()
            .map(op_editor_core::NodeId::new)
            .collect();
        let pre = self.editor_state.snapshot_for_history();
        let new_id = if let Some(allocator) = self.collab_id_allocator.as_mut() {
            self.editor_state
                .replace_paths_with_polyline_with_allocator(
                    &source_ids,
                    &result.contours,
                    allocator,
                )
        } else {
            Ok(self.editor_state.replace_paths_with_polyline(
                &source_ids,
                &result.contours,
                &mut self.next_node_id,
            ))
        };
        match new_id {
            Err(error) => {
                self.show_collab_id_error(error);
                true
            }
            Ok(Some(id)) => {
                self.editor_state.history_push_past(pre);
                self.editor_state.set_single_selection(id);
                self.mark_dirty();
                true
            }
            Ok(None) => false,
        }
    }
}

/// How long after the last pan/zoom tick the canvas keeps painting in
/// interactive-degrade mode. Long enough to cover trackpad event gaps,
/// short enough that full quality returns imperceptibly after release.
const INTERACTION_HOT_MS: u64 = 150;

impl Default for WidgetHostNative {
    fn default() -> Self {
        Self::new()
    }
}

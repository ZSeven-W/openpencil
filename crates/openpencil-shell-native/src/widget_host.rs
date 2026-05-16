//! Step 4 native widget glue — the only file in shell-native allowed
//! to call into `openpencil_shell_core::widgets`. Mirrors the
//! shell-web `widget_host.rs` so the editor-UI composition is
//! cross-platform: same widget code, same paint output.
//!
//! Layout matches `apps/web/src/components/editor/editor-layout.tsx`
//! — TopBar / LayerPanel / Toolbar (vertical floating) / Canvas
//! (fills) / RightPanel (only with selection) / StatusBar (floating
//! bottom-right) / AIChatPlaceholder (floating bottom-center).
//!
//! ### Mobile (iOS / Android) — Step 1f path
//!
//! Spec §11 — shell-native is desktop-gated (`backend` /
//! `widget_host` modules cfg-gated to `macos | linux | windows`).
//! Mobile widget rendering lands in Step 1f via `context::EaglProvider`
//! / `context::AndroidEglProvider`. Per the 2026-05-10 directive
//! ("安卓和ios 不需要 ipc / 本地 cli — 只需要 custom provider"):
//! mobile rendering is a custom-provider plugin point on the
//! `GlContextProvider` trait, not a separate IPC / CLI pipeline.
//!
//! ### Module layout
//!
//! This file is the public spine. Implementation methods are split
//! across sibling submodules (per the 800-line-per-file ceiling):
//! - [`frame_backend`] — `NativeFrameBackend` (`RenderBackend` impl)
//! - [`helpers`] — hex parsing + resize-bounds math + constants
//! - [`geometry`] — canvas region / cursor hint / picker rect helpers
//! - [`input`] — wheel / pan / cursor-move / release / keyboard / click
//! - [`press`] — `apply_press` + new-node spawn (largest method)
//! - [`paint`] — full editor-UI composition paint pass

use openpencil_shell_core::document::Document;
use openpencil_shell_core::widgets::SelectionHandle;
use openpencil_shell_core::{Rect, Theme};

mod click;
mod frame_backend;
mod geometry;
mod helpers;
mod input;
#[cfg(test)]
mod input_tests;
mod keyboard;
mod paint;
mod press;
mod press_helpers;
mod property_dispatch;
mod shortcuts;

pub use frame_backend::NativeFrameBackend;

/// Cursor affordance the host suggests for a given screen point.
/// The runner maps each variant to its native cursor (`CursorIcon`
/// on desktop, CSS `cursor:` string on web).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorHint {
    Default,
    Move,
    Grabbing,
    /// Open hand — Hand tool over canvas, ready to grab + pan.
    Grab,
    /// Crosshair — shape / frame / text tool ready to draw on
    /// canvas. Distinct from Default so the user has a clear
    /// signal the click will spawn a new node.
    Crosshair,
    /// Text cursor — only shown by the Text tool over canvas.
    Text,
    ResizeEw,
    ResizeNs,
    ResizeNwse,
    ResizeNesw,
    Rotate,
}

impl CursorHint {
    pub(in crate::widget_host) fn for_handle(h: SelectionHandle) -> Self {
        match h {
            SelectionHandle::Left | SelectionHandle::Right => CursorHint::ResizeEw,
            SelectionHandle::Top | SelectionHandle::Bottom => CursorHint::ResizeNs,
            SelectionHandle::TopLeft | SelectionHandle::BottomRight => CursorHint::ResizeNwse,
            SelectionHandle::TopRight | SelectionHandle::BottomLeft => CursorHint::ResizeNesw,
        }
    }
}

/// Native counterpart of shell-web's `WidgetHost`. Owns the
/// canonical-model `EditorState` as its single source of truth +
/// composes the editor UI per frame in the TS-equivalent layout.
pub struct WidgetHostNative {
    /// **The host's single source of truth.** All input handlers
    /// mutate this; the paint pass derives a read-only `Document`
    /// snapshot from it (see `paint_doc` / `paint_document`).
    pub(in crate::widget_host) editor_state: op_editor_core::EditorState,
    /// Derived paint-only `Document` snapshot of `editor_state`.
    /// shell-core's ~30 widgets + hit-test helpers are `&Document`-
    /// bound and read-only; they are fed this snapshot, never
    /// `editor_state`. Rebuilt lazily by `paint_document()` whenever
    /// `editor_state_dirty` is set.
    pub(in crate::widget_host) paint_doc: Document,
    /// Set whenever `editor_state` is mutated. Drives the lazy
    /// rebuild of `paint_doc` — `paint_document()` rebuilds + clears
    /// the flag, so a sequence of mutations only re-derives once.
    pub(in crate::widget_host) editor_state_dirty: bool,
    pub(in crate::widget_host) theme: Theme,
    /// Active canvas pan-drag state — left-button press → motion
    /// → release.
    pub(in crate::widget_host) drag: Option<DragState>,
    /// Active chat-panel drag state — present while the user
    /// drags the floating AI chat panel by its header. Holds the
    /// transient panel top-left position so paint can place the
    /// panel at the cursor instead of its anchor; on release the
    /// host computes the nearest corner via `ChatAnchor::nearest`
    /// and snaps.
    pub(in crate::widget_host) chat_drag: Option<ChatDragState>,
    /// Active panel-resize drag — set when the cursor is pressed
    /// within the resize gutter of LayerPanel's right edge or
    /// PropertyPanel's left edge.
    pub(in crate::widget_host) panel_resize: Option<PanelResize>,
    /// Active node-drag — set when the user presses on a node in
    /// the canvas with the Select tool. Tracks the document-space
    /// cursor anchor so each `apply_cursor_move` translates the
    /// selected node by the delta.
    pub(in crate::widget_host) node_drag: Option<NodeDragState>,
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
    /// Counter for minting fresh `NodeId`s for newly-created nodes.
    /// Bumped past the highest sample id so new + sample nodes
    /// never collide on the same key.
    pub(in crate::widget_host) next_node_id: u64,
    /// Host-supplied frame timestamp in milliseconds. Drives the
    /// caret blink via `jian_core::anim::blink_visible`. The
    /// inspector_window runner refreshes this once per
    /// `RedrawRequested` from a single `Instant` start anchor;
    /// any other host (mobile / browser) installs its own clock.
    pub(in crate::widget_host) now_ms: u64,
    /// Whether the shift key is currently held. Runners update
    /// this via `set_modifier_shift` on every modifier change.
    /// Drives shift+click multi-select in `apply_press`.
    pub(in crate::widget_host) shift_held: bool,
    /// Last viewport size seen by paint/press. Used by handlers
    /// that don't receive viewport dims (e.g. apply_cursor_move
    /// driving the color-picker drag).
    pub(in crate::widget_host) last_viewport_w: f32,
    pub(in crate::widget_host) last_viewport_h: f32,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct DragState {
    pub(in crate::widget_host) last_x: f32,
    pub(in crate::widget_host) last_y: f32,
}

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

/// Active marquee rect-select drag. Endpoints are in SCREEN
/// coordinates so paint can draw the rect without re-deriving
/// the canvas→screen transform; release converts to doc space
/// once to ask the document which nodes overlap.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct MarqueeDragState {
    pub(in crate::widget_host) start_screen_x: f32,
    pub(in crate::widget_host) start_screen_y: f32,
    pub(in crate::widget_host) current_screen_x: f32,
    pub(in crate::widget_host) current_screen_y: f32,
    /// Whether shift was held at press time. Drives whether
    /// release REPLACES the selection or toggles each
    /// intersecting node into / out of the existing set.
    pub(in crate::widget_host) additive: bool,
}

/// Active LayerPanel drag-to-reorder gesture.
#[derive(Debug, Clone)]
pub(in crate::widget_host) struct LayerDragState {
    /// NodeId of the row the user pressed on — what gets moved on
    /// release.
    pub(in crate::widget_host) source: openpencil_shell_core::document::NodeId,
    /// Cursor y at press time, panel-local. Used to suppress drag
    /// activation until the cursor has moved a few pixels (avoids
    /// promoting a regular click into a drag).
    pub(in crate::widget_host) start_y: f32,
    /// Live cursor x / y for the drop-target hit-test.
    pub(in crate::widget_host) current_x: f32,
    pub(in crate::widget_host) current_y: f32,
    /// Whether the cursor has moved past the activation threshold.
    /// False = still a candidate click; True = committed drag (paint
    /// the drop indicator).
    pub(in crate::widget_host) active: bool,
}

/// Path-anchor drag — tracks which anchor of which Path node is
/// being dragged by the pen tool. Move dispatches snap the anchor
/// to the cursor; release commits a history snapshot ONLY when the
/// anchor actually moved (codex CONCERN: a press-release without
/// motion pushed a no-op snapshot that polluted the undo stack).
#[derive(Debug, Clone)]
pub(in crate::widget_host) struct PathAnchorDragState {
    pub(in crate::widget_host) node_id: op_editor_core::NodeId,
    pub(in crate::widget_host) anchor_index: usize,
    /// Anchor position at drag-start (doc coords) — compared against
    /// the final position on release to decide whether to push the
    /// snapshot.
    pub(in crate::widget_host) start_doc: openpencil_shell_core::Point2D,
    /// Set to true on the first cursor-move that mutates the anchor.
    pub(in crate::widget_host) moved: bool,
    /// Snapshot captured at drag-start; pushed only if `moved`.
    pub(in crate::widget_host) pre_drag_snapshot: op_editor_core::EditorSnapshot,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct ChatDragState {
    /// Pointer offset within the panel rect when the drag began.
    /// Subtracting from the live cursor position gives the panel
    /// top-left, so the panel doesn't visually jump on press.
    pub(in crate::widget_host) grab_dx: f32,
    pub(in crate::widget_host) grab_dy: f32,
    /// Live panel top-left (logical px, viewport-relative).
    pub(in crate::widget_host) pos_x: f32,
    pub(in crate::widget_host) pos_y: f32,
}

impl WidgetHostNative {
    pub fn new() -> Self {
        let editor_state = op_editor_core::EditorState::sample();
        // Seed the paint snapshot once up front; subsequent frames
        // re-derive only when `editor_state_dirty` is set.
        let paint_doc = derive_paint_doc(&editor_state);
        Self {
            editor_state,
            paint_doc,
            editor_state_dirty: false,
            theme: Theme::dark(),
            drag: None,
            chat_drag: None,
            panel_resize: None,
            node_drag: None,
            path_anchor_drag: None,
            handle_drag: None,
            rotate_drag: None,
            create_drag: None,
            marquee_drag: None,
            layer_drag: None,
            next_node_id: 100,
            now_ms: 0,
            shift_held: false,
            last_viewport_w: 0.0,
            last_viewport_h: 0.0,
        }
    }

    /// Push the host's current shift-key state. Runners call this
    /// on every modifier-change event so `apply_press` can branch
    /// on shift+click semantics.
    pub fn set_modifier_shift(&mut self, held: bool) {
        self.shift_held = held;
    }

    /// Push the host's monotonic millisecond timestamp into the
    /// host. Drives caret blink + any future time-based
    /// animations via `jian_core::anim`.
    pub fn set_now_ms(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    /// Run a path boolean op on the active selection (Union /
    /// Subtract / Intersect / Exclude). Backed by skia's `Path::op`.
    /// Returns true when the op committed (≥ 2 Path nodes were
    /// selected + the result yielded a non-empty polyline).
    pub fn apply_boolean_op(
        &mut self,
        op: openpencil_shell_core::document::BooleanOp,
    ) -> bool {
        // Codex stop-gate: boolean op shortcuts (Cmd+Alt+U/S/I/X)
        // mutate the document — commit any pending variable-row
        // edit first so the dirty draft lands before this op runs.
        self.commit_variable_row_focus_if_any();
        // The skia `Path::op` math runs against the derived paint
        // `Document`; the result polyline is committed back through
        // an `EditorState` mutator so the host never edits the
        // canonical tree directly.
        self.refresh_paint_doc();
        let outcome =
            crate::boolean_ops::compute_boolean_op(&self.paint_doc, op);
        let Some(result) = outcome else {
            return false;
        };
        let source_ids: Vec<op_editor_core::NodeId> = result
            .source_ids
            .iter()
            .map(op_pen_loader::rev::node_id)
            .collect();
        let pre = self.editor_state.snapshot_for_history();
        let new_id = self.editor_state.replace_paths_with_polyline(
            &source_ids,
            &result.points,
            &mut self.next_node_id,
        );
        match new_id {
            Some(id) => {
                self.editor_state.history_push_past(pre);
                self.editor_state.set_single_selection(id);
                self.mark_dirty();
                true
            }
            None => false,
        }
    }

    /// Derive a fresh paint-only `Document` snapshot from
    /// `editor_state` if `editor_state_dirty` is set; clear the flag.
    /// Cheap no-op when the snapshot is already current.
    pub(in crate::widget_host) fn refresh_paint_doc(&mut self) {
        if self.editor_state_dirty {
            self.paint_doc = derive_paint_doc(&self.editor_state);
            self.editor_state_dirty = false;
        }
    }

    /// The read-only paint `Document` snapshot of the live
    /// `EditorState`. Rebuilt on demand when the state changed since
    /// the last derive. Every widget paint + `&Document`-bound
    /// hit-test reads through this.
    pub fn paint_document(&mut self) -> &Document {
        self.refresh_paint_doc();
        &self.paint_doc
    }

    /// Mark `editor_state` as mutated so the next `paint_document()`
    /// re-derives the paint snapshot. Call after any direct mutation
    /// of `self.editor_state`.
    pub(in crate::widget_host) fn mark_dirty(&mut self) {
        self.editor_state_dirty = true;
    }

    /// Test-only: flag the paint snapshot stale after a test mutated
    /// `editor_state` directly through `editor_state_mut()`.
    #[cfg(test)]
    pub(in crate::widget_host) fn mark_paint_dirty_for_test(&mut self) {
        self.editor_state_dirty = true;
    }

    /// Read-only paint `Document` accessor for file-I/O / export
    /// code in the desktop binary. Forces a fresh derive.
    pub fn document(&mut self) -> &Document {
        self.paint_document()
    }

    /// Borrow the canonical-model editor state — the host's single
    /// source of truth.
    pub fn editor_state(&self) -> &op_editor_core::EditorState {
        &self.editor_state
    }

    /// Mutable borrow of the canonical-model editor state. Callers
    /// that mutate through this MUST call `mark_editor_state_dirty()`
    /// afterwards, else the paint snapshot goes stale.
    pub fn editor_state_mut(&mut self) -> &mut op_editor_core::EditorState {
        &mut self.editor_state
    }

    /// Public dirty-flag — desktop-side code that mutates
    /// `editor_state` through `editor_state_mut()` (settings I/O,
    /// `.op` load, chat streaming, model discovery) calls this so the
    /// next paint re-derives the snapshot.
    pub fn mark_editor_state_dirty(&mut self) {
        self.editor_state_dirty = true;
    }

    /// Commit any in-progress settings-modal input draft (currently
    /// the MCP port). Used by the desktop runner before persisting
    /// settings on quick-quit so a focused-but-uncommitted port edit
    /// isn't silently dropped.
    pub fn flush_settings_input(&mut self) {
        self.commit_settings_focus_if_any();
    }

    /// Whether the chat input is focused — runner uses this to
    /// decide whether to schedule a periodic wake-up for caret
    /// blink.
    pub fn chat_focused(&self) -> bool {
        self.editor_state.chat.focused
    }

    /// Next millisecond at which the host should wake to repaint
    /// the caret blink phase. `None` = no animation pending.
    pub fn next_animation_deadline_ms(&self) -> Option<u64> {
        let ui = &self.editor_state.ui;
        if ui.text_editing.is_some() {
            return Some(jian_core::anim::next_blink_flip_ms(
                self.now_ms,
                ui.text_edit_caret_anchor_ms,
                500,
            ));
        }
        if ui.layer_rename.is_some() {
            return Some(jian_core::anim::next_blink_flip_ms(
                self.now_ms,
                self.editor_state.editor_ui.rename_caret_anchor_ms,
                500,
            ));
        }
        if ui.property_focus.is_some() {
            return Some(jian_core::anim::next_blink_flip_ms(
                self.now_ms,
                ui.property_caret_anchor_ms,
                500,
            ));
        }
        if self.editor_state.chat.focused {
            return Some(jian_core::anim::next_blink_flip_ms(
                self.now_ms,
                self.editor_state.chat.caret_anchor_ms,
                500,
            ));
        }
        None
    }
}

/// Derive a faithful paint-only `Document` from an `EditorState`:
/// node tree + geometry from the canonical doc, then the chrome /
/// chat / components / variable / scalar state layered on.
pub(in crate::widget_host) fn derive_paint_doc(
    state: &op_editor_core::EditorState,
) -> Document {
    let mut d = op_pen_loader::pen_document_to_document(&state.doc);
    op_pen_loader::apply_editor_state_ui(&mut d, state);
    d
}

impl Default for WidgetHostNative {
    fn default() -> Self {
        Self::new()
    }
}

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

mod frame_backend;
mod geometry;
mod helpers;
mod input;
#[cfg(test)]
mod input_tests;
mod paint;
mod press;

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

/// Native counterpart of shell-web's `WidgetHost`. Owns a
/// `Document` + composes the editor UI per frame in the
/// TS-equivalent layout (Step 4 visual lift).
pub struct WidgetHostNative {
    pub(in crate::widget_host) document: Document,
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
#[derive(Debug, Clone, Copy)]
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
        Self {
            document: Document::sample(),
            theme: Theme::dark(),
            drag: None,
            chat_drag: None,
            panel_resize: None,
            node_drag: None,
            handle_drag: None,
            rotate_drag: None,
            create_drag: None,
            marquee_drag: None,
            layer_drag: None,
            next_node_id: 100,
            now_ms: 0,
            shift_held: false,
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

    /// Whether the chat input is focused — runner uses this to
    /// decide whether to schedule a periodic wake-up for caret
    /// blink.
    pub fn chat_focused(&self) -> bool {
        self.document.chat.focused
    }

    /// Next millisecond at which the host should wake to repaint
    /// the caret blink phase. `None` = no animation pending.
    pub fn next_animation_deadline_ms(&self) -> Option<u64> {
        if self.document.ui.property_focus.is_some() {
            return Some(jian_core::anim::next_blink_flip_ms(
                self.now_ms,
                self.document.ui.property_caret_anchor_ms,
                500,
            ));
        }
        if self.document.chat.focused {
            return Some(jian_core::anim::next_blink_flip_ms(
                self.now_ms,
                self.document.chat.caret_anchor_ms,
                500,
            ));
        }
        None
    }
}

impl Default for WidgetHostNative {
    fn default() -> Self {
        Self::new()
    }
}

//! Pointer input + the viewer's touch gestures.
//!
//! The engine owns interaction so both shells stay thin: single-finger
//! tap selects the topmost scene node under the finger (with a
//! highlight painted by [`crate::render`]), single-finger drag pans the
//! viewport, and a two-finger pinch zooms around the pinch midpoint.
//! Points arrive in surface-logical coordinates with a top-left origin,
//! exactly as the shells deliver them from `UITouch.location(in:)` /
//! `MotionEvent` (no dpr multiplication — dpr affects the drawable
//! only, never the pointer stream).

use crate::desc::OpPointerPhase;
use crate::error::{FfiError, FfiResult};
use crate::lifecycle::Session;
use op_editor_ui::layout_scene::LayoutScene;
use op_editor_ui::{Point2D, Rect};

/// A pan only starts once the finger has moved this many logical
/// points — below it the gesture is still a tap candidate.
const PAN_SLOP: f32 = 6.0;
/// Zoom bounds (logical): 2% ..= 16x.
pub(crate) const MIN_ZOOM: f32 = 0.02;
pub(crate) const MAX_ZOOM: f32 = 16.0;

/// The mutable viewport fields a gesture writes. Split out of
/// [`Session`] so the tracker never double-borrows the session.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Viewport {
    pub origin: Point2D,
    pub zoom: f32,
}

impl Viewport {
    pub fn view_to_doc(&self, p: Point2D) -> Point2D {
        Point2D::new(
            (p.x - self.origin.x) / self.zoom,
            (p.y - self.origin.y) / self.zoom,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct TouchPoint {
    id: u32,
    x: f32,
    y: f32,
}

/// Multi-touch tracker feeding the viewer gestures.
#[derive(Default)]
pub(crate) struct GestureTracker {
    touches: Vec<TouchPoint>,
    /// A single-finger drag has passed the slop and is panning.
    panning: bool,
    /// Anchor for an active two-finger pinch.
    pinch: Option<PinchAnchor>,
}

#[derive(Clone, Copy, Debug)]
struct PinchAnchor {
    dist_start: f32,
    zoom_start: f32,
    /// Doc-space point that stays under the moving midpoint.
    doc_anchor: Point2D,
}

/// Outcome of one gesture event.
#[derive(Default)]
pub(crate) struct GestureOutcome {
    /// The viewport changed (pan / pinch).
    pub viewport_changed: bool,
    /// A tap completed; the topmost node id under the finger (or `None`
    /// for dead space — clear the selection).
    pub tap_hit: Option<String>,
}

/// Route one pointer event into the session's gesture tracker.
///
/// Returns `true` when the event mutated visible state (viewport,
/// selection, or text edit), so the caller can fire the redraw upcall.
pub(crate) fn handle_pointer(
    session: &mut Session,
    id: u32,
    phase: OpPointerPhase,
    x: f32,
    y: f32,
) -> FfiResult<bool> {
    let mut view = Viewport {
        origin: session.viewport_origin,
        zoom: session.zoom,
    };
    let logical = session.logical;
    let outcome = {
        // Split borrows: the tracker mutates the viewport while the scene
        // is only read (hit-testing); the borrows end before the tap is
        // applied to the whole session.
        let Session { gesture, scene, .. } = session;
        gesture.handle(&mut view, scene, logical, id, phase, x, y)?
    };
    if view.origin != session.viewport_origin || view.zoom != session.zoom {
        session.user_interacted = true;
    }
    session.viewport_origin = view.origin;
    session.zoom = view.zoom;
    // A completed tap carries the hit node id (or `None` for dead space);
    // apply it with full session access (text edits may rebuild the scene).
    // Dead space still commits an active text edit (the desktop commits on
    // any outside click).
    if outcome.tap_hit.is_some() || session.state.ui.text_editing.is_some() {
        return apply_tap(session, outcome.tap_hit.as_deref(), x, y);
    }
    Ok(outcome.viewport_changed)
}

/// Apply a completed tap: text nodes enter inline edit mode (or place the
/// caret when already editing that node); anything else (including dead
/// space) commits an active edit and selects the hit.
fn apply_tap(session: &mut Session, hit: Option<&str>, x: f32, y: f32) -> FfiResult<bool> {
    if let Some(editing) = session
        .state
        .ui
        .text_editing
        .as_ref()
        .map(|id| id.as_str().to_owned())
    {
        if Some(editing.as_str()) == hit {
            // Tap inside the edited node: place the caret at the tapped
            // glyph (the same line layout the painter uses).
            let Some(hit) = hit else {
                return Ok(false);
            };
            let Some(page) = session.scene.active_page() else {
                return Ok(false);
            };
            let Some(node) = crate::render::find_node(&page.children, hit) else {
                return Ok(false);
            };
            let doc = Point2D::new(
                (x - session.viewport_origin.x) / session.zoom,
                (y - session.viewport_origin.y) / session.zoom,
            );
            let mut measure = crate::measure::MeasureOnly {
                inner: &mut session.backend,
            };
            let layout =
                op_editor_ui::widgets::canvas_text_edit::text_edit_layout(&mut measure, node);
            let offset = layout.offset_at_point(&mut measure, doc);
            let before = session.state.ui.text_edit_input.caret();
            session
                .state
                .text_edit_set_caret(offset, false, session.now_ms);
            let moved = session.state.ui.text_edit_input.caret() != before;
            if moved {
                session.request_redraw();
            }
            return Ok(moved);
        }
        // Tap elsewhere (or dead space): commit the active edit, then
        // select the hit (None → clear the selection).
        session.state.text_edit_commit();
        session.rebuild_scene();
        session.focus_changed(false, 0, 0);
        session.selected = hit.map(str::to_owned);
        session.request_redraw();
        return Ok(true);
    }

    // No active edit: a Text node tap enters edit mode; anything else
    // selects. Dead space without an edit leaves everything untouched.
    let Some(page) = session.scene.active_page() else {
        return Ok(false);
    };
    let Some(hit) = hit else {
        return Ok(false);
    };
    let Some(node) = crate::render::find_node(&page.children, hit) else {
        session.selected = None;
        session.request_redraw();
        return Ok(true);
    };
    if matches!(node.kind, op_editor_ui::layout_scene::NodeKind::Text)
        && session
            .state
            .start_text_edit(op_editor_core::NodeId::new(hit))
    {
        session.rebuild_scene();
        session.selected = None;
        session.focus_changed(true, 0, 0);
        session.request_redraw();
        return Ok(true);
    }
    if session.selected.as_deref() != Some(hit) {
        session.selected = Some(hit.to_owned());
        session.request_redraw();
        return Ok(true);
    }
    Ok(false)
}

impl GestureTracker {
    #[allow(clippy::too_many_arguments)]
    fn handle(
        &mut self,
        view: &mut Viewport,
        scene: &LayoutScene,
        logical: (f32, f32),
        id: u32,
        phase: OpPointerPhase,
        x: f32,
        y: f32,
    ) -> FfiResult<GestureOutcome> {
        let _ = logical;
        let mut outcome = GestureOutcome::default();
        let point = Point2D::new(x, y);
        match phase {
            OpPointerPhase::Down => {
                self.touches.push(TouchPoint { id, x, y });
                if self.touches.len() == 1 {
                    // Fresh single touch: re-arm the tap candidate.
                    self.panning = false;
                }
            }
            OpPointerPhase::Move => {
                let Some(touch) = self.touches.iter_mut().find(|t| t.id == id) else {
                    return Err(FfiError::invalid("pointer id not tracked"));
                };
                let (old_x, old_y) = (touch.x, touch.y);
                touch.x = x;
                touch.y = y;
                match self.touches.len() {
                    1 => {
                        if self.panning {
                            view.origin.x += x - old_x;
                            view.origin.y += y - old_y;
                            outcome.viewport_changed = true;
                        } else {
                            let dist = ((x - old_x).powi(2) + (y - old_y).powi(2)).sqrt();
                            if dist > PAN_SLOP {
                                self.panning = true;
                                view.origin.x += x - old_x;
                                view.origin.y += y - old_y;
                                outcome.viewport_changed = true;
                            }
                        }
                    }
                    2 => {
                        let (a, b) = self.pinch_pair();
                        let mid = Point2D::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
                        let dist = ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt().max(1.0);
                        let anchor = self.pinch.get_or_insert_with(|| PinchAnchor {
                            dist_start: dist,
                            zoom_start: view.zoom,
                            doc_anchor: view.view_to_doc(mid),
                        });
                        anchor.dist_start = dist;
                        let zoom = (anchor.zoom_start * dist / anchor.dist_start)
                            .clamp(MIN_ZOOM, MAX_ZOOM);
                        view.origin = Point2D::new(
                            mid.x - anchor.doc_anchor.x * zoom,
                            mid.y - anchor.doc_anchor.y * zoom,
                        );
                        view.zoom = zoom;
                        outcome.viewport_changed = true;
                    }
                    _ => {}
                }
            }
            OpPointerPhase::Up | OpPointerPhase::Cancel => {
                let was_pinch = self.pinch.is_some();
                self.touches.retain(|t| t.id != id);
                if was_pinch {
                    self.pinch = None;
                }
                match self.touches.len() {
                    0 => {
                        if !self.panning && !was_pinch {
                            // A tap: select the topmost node under the
                            // point (dead space reports `None` so the
                            // caller clears the selection).
                            let doc = view.view_to_doc(point);
                            outcome.tap_hit = scene.node_at_doc_point(doc, view.zoom);
                        }
                        self.panning = false;
                    }
                    1 => {
                        // Pinch lifted to a single finger; re-arm pan
                        // tracking without a jump (the remaining finger's
                        // position was already updated by its own Move).
                        self.panning = false;
                    }
                    _ => {}
                }
            }
        }
        Ok(outcome)
    }

    fn pinch_pair(&self) -> (Point2D, Point2D) {
        debug_assert_eq!(self.touches.len(), 2);
        (
            Point2D::new(self.touches[0].x, self.touches[0].y),
            Point2D::new(self.touches[1].x, self.touches[1].y),
        )
    }

    /// Reset all tracking (used by suspend).
    pub(crate) fn reset(&mut self) {
        self.touches.clear();
        self.panning = false;
        self.pinch = None;
    }
}

/// Cull rect covering the current viewport in doc space (with a small
/// stroke margin so a node half-offscreen still paints).
pub(crate) fn viewport_cull(origin: Point2D, zoom: f32, logical: (f32, f32)) -> Rect {
    let (w, h) = logical;
    let margin = 64.0 / zoom.max(0.0001);
    let to_doc = |p: Point2D| Point2D::new((p.x - origin.x) / zoom, (p.y - origin.y) / zoom);
    let top_left = to_doc(Point2D::new(-margin, -margin));
    let bottom_right = to_doc(Point2D::new(w + margin, h + margin));
    Rect {
        origin: top_left,
        size: Point2D::new(
            (bottom_right.x - top_left.x).max(0.0),
            (bottom_right.y - top_left.y).max(0.0),
        ),
    }
}

//! `CanvasViewport` — center-canvas widget that renders the editor's
//! resolved render scene as visual primitives.
//!
//! PAINT path: the widget reads editor state (`viewport` / selection /
//! `tool` / pen-draft) from `op_editor_core::EditorState` and the
//! resolved render-node tree from a [`LayoutScene`]. The per-kind node
//! painter lives in the sibling [`super::canvas_viewport_paint`]
//! module to keep this file under the 800-line ceiling.
//!
//! INPUT path: the host-input hit-test helpers
//! [`rotation_corner_at_point`] / [`selection_handle_at_point`] read
//! the layout-resolved [`LayoutScene`] + the editor's selection /
//! viewport state — they serve the hosts' input dispatch, not widget
//! paint.
//!
//! Per-kind paint:
//! - Frame: fill (if any) at `bounds`, optional stroke, then recurse.
//! - Group / Other(_): no own paint, just recurse into children.
//! - Rect / Ellipse / Polygon / Line / Path / Text: per-kind paint.
//! - `Other("icon_font")`: lucide glyph from `text`.
//!
//! Selection overlay (outlines + handles), grid, pen rubber-band and
//! per-anchor Path handles are layered on top of the resolved scene.

use crate::document::{NodeKind, Viewport as DocViewport};
use crate::layout_scene::LayoutScene;
use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect};

use op_editor_core::EditorState;

/// One of the 8 selection handles (corners + edge midpoints) the
/// selection overlay paints. Used by the host to dispatch resize
/// drags: each variant fixes the corresponding edge / corner of
/// the selected bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

/// Radius (screen px) of the rotation ring that sits OUTSIDE the
/// 4 selection corners. Matches the TS `ROTATE_OUTER_RADIUS`.
const ROTATE_OUTER_RADIUS: f32 = 16.0;

/// The single resolved scene node the editor's selection anchor
/// points at, or `None` when the selection isn't a single node that
/// resolves on the active page. Shared by the two selection-overlay
/// hit-tests below — they only fire on single-select.
fn selected_scene_node<'a>(
    scene: &'a LayoutScene,
    state: &EditorState,
) -> Option<&'a crate::layout_scene::SceneNode> {
    if state.selection_count() != 1 {
        return None;
    }
    let anchor = state.selection.anchor.as_str();
    scene.active_page()?.find(anchor)
}

/// Hit-test the rotation ring that sits just outside the four
/// corner handles. Returns the nearest corner (so the runner can
/// hint which way the rotation drag is anchored) or `None` if the
/// cursor isn't in a rotation zone.
///
/// The rotation zone is an annulus around each corner — beyond
/// the 6 px handle slop and inside the 16 px outer radius. Matches
/// the TS `hitTestRotation` logic.
///
/// INPUT path — reads the layout-resolved [`LayoutScene`] (selected
/// node geometry) + the editor's selection / viewport state.
pub fn rotation_corner_at_point(
    canvas_rect: Rect,
    scene: &LayoutScene,
    state: &EditorState,
    point: Point2D,
) -> Option<SelectionHandle> {
    // Rotation rings are only painted on single-select (the
    // multi-select overlay is outline-only), so gate the hit-test
    // to match — otherwise non-anchor "rotation zones" would
    // intercept clicks on dead air.
    let node = selected_scene_node(scene, state)?;
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    let viewport = DocViewport {
        pan_x: state.viewport.pan_x,
        pan_y: state.viewport.pan_y,
        zoom: state.viewport.zoom,
    };
    let left = canvas_rect.origin.x + viewport.pan_x + bounds.origin.x * viewport.zoom;
    let top = canvas_rect.origin.y + viewport.pan_y + bounds.origin.y * viewport.zoom;
    let right = left + bounds.size.x * viewport.zoom;
    let bottom = top + bounds.size.y * viewport.zoom;
    // Inverse-rotate the cursor into the node's local space so the
    // hit-test annulus tracks the rendered (rotated) corners.
    let cx = (left + right) / 2.0;
    let cy = (top + bottom) / 2.0;
    let local = inverse_rotate(point, Point2D::new(cx, cy), node.rotation);
    let inner = 6.0_f32;
    let outer = ROTATE_OUTER_RADIUS;
    let corners = [
        (SelectionHandle::TopLeft, left, top),
        (SelectionHandle::TopRight, right, top),
        (SelectionHandle::BottomLeft, left, bottom),
        (SelectionHandle::BottomRight, right, bottom),
    ];
    for (kind, cx, cy) in corners {
        let dx = local.x - cx;
        let dy = local.y - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > inner && dist <= outer {
            return Some(kind);
        }
    }
    None
}

/// Apply the inverse of a rotation about `pivot` to `point`. Used
/// by hit-tests so a rotated selection's handles + rotation ring
/// + body all match the rendered (rotated) geometry.
fn inverse_rotate(point: Point2D, pivot: Point2D, radians: f32) -> Point2D {
    if radians.abs() < f32::EPSILON {
        return point;
    }
    let dx = point.x - pivot.x;
    let dy = point.y - pivot.y;
    let cos_t = (-radians).cos();
    let sin_t = (-radians).sin();
    Point2D::new(
        pivot.x + dx * cos_t - dy * sin_t,
        pivot.y + dx * sin_t + dy * cos_t,
    )
}

/// Hit-test the 8 selection handles around the currently-selected
/// node. Returns the handle at `point` (a small slop around each
/// handle center counts) or `None` if no selection / no handle.
///
/// `canvas_rect` is the on-screen rect the canvas widget paints
/// into (same value passed to `CanvasViewport::paint`). The
/// transform from document → screen is identical to paint so a
/// handle the user clicks is the handle they see.
///
/// INPUT path — reads the layout-resolved [`LayoutScene`] + the
/// editor's selection / viewport state (see [`rotation_corner_at_point`]).
pub fn selection_handle_at_point(
    canvas_rect: Rect,
    scene: &LayoutScene,
    state: &EditorState,
    point: Point2D,
) -> Option<SelectionHandle> {
    // Handles are only painted on single-select (the multi-select
    // overlay is outline-only — Figma parity), so gate the hit-
    // test to match. Otherwise the "anchor's handles" would hit-
    // test even though no handles are visible anywhere.
    let node = selected_scene_node(scene, state)?;
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    let viewport = DocViewport {
        pan_x: state.viewport.pan_x,
        pan_y: state.viewport.pan_y,
        zoom: state.viewport.zoom,
    };
    let left = canvas_rect.origin.x + viewport.pan_x + bounds.origin.x * viewport.zoom;
    let top = canvas_rect.origin.y + viewport.pan_y + bounds.origin.y * viewport.zoom;
    let right = left + bounds.size.x * viewport.zoom;
    let bottom = top + bounds.size.y * viewport.zoom;
    let mid_x = (left + right) / 2.0;
    let mid_y = (top + bottom) / 2.0;
    // Inverse-rotate the cursor so handle hit-test tracks rendered
    // (rotated) handle positions.
    let local = inverse_rotate(point, Point2D::new(mid_x, mid_y), node.rotation);
    let slop = 6.0;
    let anchors = [
        (SelectionHandle::TopLeft, left, top),
        (SelectionHandle::Top, mid_x, top),
        (SelectionHandle::TopRight, right, top),
        (SelectionHandle::Right, right, mid_y),
        (SelectionHandle::BottomRight, right, bottom),
        (SelectionHandle::Bottom, mid_x, bottom),
        (SelectionHandle::BottomLeft, left, bottom),
        (SelectionHandle::Left, left, mid_y),
    ];
    for (kind, hx, hy) in anchors {
        if (local.x - hx).abs() <= slop && (local.y - hy).abs() <= slop {
            return Some(kind);
        }
    }
    None
}

/// Caret-blink descriptor for the text node currently being edited.
/// `pub` so the sibling `canvas_viewport_paint` module can name it in
/// the public `paint_node` signature.
#[derive(Clone)]
pub struct EditCaret {
    /// The node id (scene-space string) being edited.
    pub editing: String,
    pub anchor_ms: u64,
    pub now_ms: u64,
}

pub struct CanvasViewport<'a> {
    pub id: WidgetId,
    /// Pan / zoom of the infinite canvas — read from `EditorState`.
    pub(super) viewport: DocViewport,
    /// The resolved render scene — the node tree the painter walks.
    pub(super) scene: &'a LayoutScene,
    /// Anchor-selected node id (scene-space string). Empty = none.
    pub(super) selected: String,
    /// Full selection set (scene-space string ids).
    pub(super) selected_set: Vec<String>,
    /// Active canvas tool — gates the per-anchor Path handles.
    pub(super) tool: op_editor_core::Tool,
    /// Pen-tool draft: the in-progress path id + last cursor doc
    /// coord, used to paint the rubber-band preview.
    pub(super) pen_in_progress: Option<String>,
    pub(super) pen_cursor_doc: Option<Point2D>,
    /// Text node being edited (scene-space string id) + its caret
    /// blink anchor.
    pub(super) text_editing: Option<String>,
    pub(super) text_edit_caret_anchor_ms: u64,
    /// Background fill outside any Frame.
    pub canvas_background: Color,
    pub theme: Theme,
    /// Host ms clock — text-edit caret blink.
    pub now_ms: u64,
}

/// Spacing (document px) between major grid dots at 100% zoom.
/// Infinite canvas — adapted from `apps/web` canvas grid.
const GRID_SPACING: f32 = 32.0;

impl<'a> CanvasViewport<'a> {
    /// Build the canvas widget for a paint pass.
    ///
    /// Editor state — `viewport` / selection / `tool` / pen-draft /
    /// text-edit — is read from `state`; the resolved render-node tree
    /// is read from `scene`. The host builds `scene` via
    /// `op_pen_loader::editor_state_to_layout_scene` and caches it,
    /// refreshing on `editor_state_dirty`.
    pub fn from_editor(state: &EditorState, scene: &'a LayoutScene) -> Self {
        let theme = theme_for(&state.editor_ui);
        let viewport = DocViewport {
            pan_x: state.viewport.pan_x,
            pan_y: state.viewport.pan_y,
            zoom: state.viewport.zoom,
        };
        Self {
            id: WidgetId::new(4000),
            viewport,
            scene,
            selected: state.selection.anchor.as_str().to_string(),
            selected_set: state
                .selection
                .set
                .iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            tool: state.tool,
            pen_in_progress: state
                .ui
                .pen_in_progress
                .as_ref()
                .map(|id| id.as_str().to_string()),
            pen_cursor_doc: state
                .ui
                .pen_cursor_doc
                .map(|p| Point2D::new(p.x, p.y)),
            text_editing: state
                .ui
                .text_editing
                .as_ref()
                .map(|id| id.as_str().to_string()),
            text_edit_caret_anchor_ms: state.ui.text_edit_caret_anchor_ms,
            canvas_background: theme.canvas_surface,
            theme,
            now_ms: 0,
        }
    }
}

impl<'a> Widget for CanvasViewport<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Host sizes via the paint rect; we just report a default.
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, cx.available_width, cx.available_width.max(400.0)),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
            return;
        }
        cx.backend.save();
        cx.backend.clip_rect(rect);

        // 1. Paint the canvas background INSIDE the clip region.
        cx.backend.fill_rect(rect, self.canvas_background);

        // 2. Dotted grid — canvas-local, scales with pan/zoom.
        let viewport = &self.viewport;
        paint_grid(cx, rect, viewport, &self.theme);

        // 3. Walk the active page; clip enforces widget bounds.
        if let Some(page) = self.scene.active_page() {
            let viewport_origin = Point2D::new(
                rect.origin.x + viewport.pan_x,
                rect.origin.y + viewport.pan_y,
            );
            let edit_caret = self.text_editing.as_ref().map(|id| EditCaret {
                editing: id.clone(),
                anchor_ms: self.text_edit_caret_anchor_ms,
                now_ms: self.now_ms,
            });
            // Cull rect — anything fully outside this rect (with a
            // generous margin for stroke widths / rotated handles /
            // text overhang) can skip paint entirely.
            const CULL_MARGIN: f32 = 64.0;
            let cull = Rect {
                origin: Point2D::new(rect.origin.x - CULL_MARGIN, rect.origin.y - CULL_MARGIN),
                size: Point2D::new(
                    rect.size.x + CULL_MARGIN * 2.0,
                    rect.size.y + CULL_MARGIN * 2.0,
                ),
            };
            for child in &page.children {
                super::canvas_viewport_paint::paint_node(
                    cx,
                    child,
                    viewport_origin,
                    viewport.zoom,
                    edit_caret.clone(),
                    cull,
                );
            }
        }

        // 3b. Pen tool rubber-band from last anchor to cursor.
        super::canvas_viewport_overlay::paint_pen_rubber_band(
            cx,
            self.scene,
            self.pen_in_progress.as_deref(),
            self.pen_cursor_doc,
            rect,
            viewport,
        );

        // 4. Selection overlay — outlines + handles (single-select only).
        let show_handles = self.selected_set.len() == 1;
        if let Some(page) = self.scene.active_page() {
            for id in &self.selected_set {
                let Some(node) = page.find(id) else {
                    continue;
                };
                if node.hidden {
                    continue;
                }
                let bounds = node.aggregate_bounds();
                if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
                    continue;
                }
                let world_rect = Rect {
                    origin: Point2D::new(
                        rect.origin.x + viewport.pan_x + bounds.origin.x * viewport.zoom,
                        rect.origin.y + viewport.pan_y + bounds.origin.y * viewport.zoom,
                    ),
                    size: Point2D::new(
                        bounds.size.x * viewport.zoom,
                        bounds.size.y * viewport.zoom,
                    ),
                };
                let is_container = matches!(
                    node.kind,
                    NodeKind::Frame | NodeKind::Group | NodeKind::Other(_)
                );
                let rotated = node.rotation.abs() > f32::EPSILON;
                if rotated {
                    let pivot = Point2D::new(
                        world_rect.origin.x + world_rect.size.x / 2.0,
                        world_rect.origin.y + world_rect.size.y / 2.0,
                    );
                    cx.backend.save();
                    cx.backend.rotate(node.rotation, pivot);
                }
                super::canvas_viewport_overlay::paint_selection_overlay(
                    cx,
                    world_rect,
                    &self.theme,
                    is_container,
                    show_handles,
                );
                if rotated {
                    cx.backend.restore();
                }
            }
        }

        // 4b. Per-anchor handles for the selected Path node when the
        //     Pen tool is active — surfaces the drag target that
        //     `path_anchor_drag` consumes.
        if matches!(self.tool, op_editor_core::Tool::Pen) && self.selected_set.len() == 1 {
            if let Some(page) = self.scene.active_page() {
                if let Some(node) = page.find(&self.selected) {
                    if matches!(node.kind, NodeKind::Path) {
                        let r = 4.0; // screen-px radius
                        for p in &node.points {
                            let center = Point2D::new(
                                rect.origin.x + viewport.pan_x + p.x * viewport.zoom,
                                rect.origin.y + viewport.pan_y + p.y * viewport.zoom,
                            );
                            let bounds = Rect {
                                origin: Point2D::new(center.x - r, center.y - r),
                                size: Point2D::new(r * 2.0, r * 2.0),
                            };
                            cx.backend.fill_oval(bounds, self.theme.background);
                            cx.backend.stroke_oval(bounds, self.theme.primary, 1.5);
                        }
                    }
                }
            }
        }

        cx.backend.restore();
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Canvas);
        node.set_label("Canvas");
        node
    }
}

/// Paint a dotted grid across the canvas widget rect. The grid is
/// drawn in canvas-local coordinates and offset by `viewport.pan`
/// so it scrolls with the document content. Dots get sparser as
/// zoom decreases (skipping every other dot at low zoom) so they
/// stay visually airy.
fn paint_grid(cx: &mut PaintCx<'_>, rect: Rect, viewport: &DocViewport, theme: &Theme) {
    let zoom = viewport.zoom.max(0.0001);
    let mut step = GRID_SPACING * zoom;
    // Skip rendering when dots would be packed tighter than 8 px
    // (visual noise at deep zoom-out), and double the step so the
    // dot density stays roughly constant.
    while step < 8.0 {
        step *= 2.0;
    }

    let dot_color = Color {
        r: theme.muted_foreground.r,
        g: theme.muted_foreground.g,
        b: theme.muted_foreground.b,
        a: 0.18,
    };
    let dot_size = (1.5 * zoom.sqrt()).clamp(1.0, 2.5);

    // Align grid origin to (0, 0) in document space, shifted by
    // pan, then shifted into widget space.
    let origin_x = rect.origin.x + (viewport.pan_x.rem_euclid(step));
    let origin_y = rect.origin.y + (viewport.pan_y.rem_euclid(step));

    let mut y = origin_y - step;
    while y < rect.origin.y + rect.size.y + step {
        let mut x = origin_x - step;
        while x < rect.origin.x + rect.size.x + step {
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(x - dot_size / 2.0, y - dot_size / 2.0),
                    size: Point2D::new(dot_size, dot_size),
                },
                dot_size / 2.0,
                dot_color,
            );
            x += step;
        }
        y += step;
    }
}


#[cfg(test)]
#[path = "canvas_viewport_tests.rs"]
mod tests;

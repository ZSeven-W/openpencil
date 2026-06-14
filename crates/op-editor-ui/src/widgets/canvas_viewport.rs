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

use crate::layout_scene::LayoutScene;
use crate::layout_scene::NodeKind;
use crate::layout_scene::{SceneAnchor, SceneNode};
use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect};
use jian_core::text_input::TextInputState;
use op_editor_core::Viewport as DocViewport;

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

/// Rotate `p` by `radians` (clockwise, screen y-down) about `center`.
/// Used by the host to un-rotate a cursor point into a rotated
/// node's local frame before hit-testing its handles.
pub fn rotate_point(p: Point2D, center: Point2D, radians: f32) -> Point2D {
    let (s, c) = radians.sin_cos();
    let dx = p.x - center.x;
    let dy = p.y - center.y;
    Point2D::new(center.x + dx * c - dy * s, center.y + dx * s + dy * c)
}

/// Screen-px offset of a "ghost" handle dot from its anchor when the
/// handle is unset — far enough from the anchor body to grab.
pub const PATH_HANDLE_GHOST_PX: f32 = 26.0;

/// Doc-space positions of a path anchor's incoming + outgoing bezier
/// control handles. An unset handle is given a "ghost" position
/// offset from the anchor (scaled to `zoom`) so the user can grab it
/// to create the handle. Returns `(handle_in, handle_out)`. Shared by
/// the overlay painter and the host's handle hit-test.
pub fn path_handle_positions(anchor: &SceneAnchor, zoom: f32) -> (Point2D, Point2D) {
    let ghost = PATH_HANDLE_GHOST_PX / zoom.max(0.0001);
    let hin = anchor
        .handle_in
        .unwrap_or(Point2D::new(anchor.pos.x - ghost, anchor.pos.y));
    let hout = anchor
        .handle_out
        .unwrap_or(Point2D::new(anchor.pos.x + ghost, anchor.pos.y));
    (hin, hout)
}

/// The three arc-edit handles on a selected Ellipse — start angle,
/// sweep (end) angle, and the donut inner-radius.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcHandle {
    /// Perimeter handle at the arc's start angle.
    Start,
    /// Perimeter handle at the arc's end angle (start + sweep).
    Sweep,
    /// Radial handle controlling the donut inner-radius fraction.
    Inner,
}

/// Doc-space positions of the three arc handles for an Ellipse
/// `SceneNode`. `None` for non-Ellipse kinds or a zero-size node.
/// Shared by the overlay painter and the host's arc-handle hit-test
/// so both agree on handle placement.
pub fn arc_handle_positions(node: &SceneNode) -> Option<[(ArcHandle, Point2D); 3]> {
    if !matches!(node.kind, NodeKind::Ellipse) {
        return None;
    }
    let b = node.bounds;
    if b.size.x <= 0.0 || b.size.y <= 0.0 {
        return None;
    }
    let cx = b.origin.x + b.size.x / 2.0;
    let cy = b.origin.y + b.size.y / 2.0;
    let rx = b.size.x / 2.0;
    let ry = b.size.y / 2.0;
    let start = node.arc_start_angle.unwrap_or(0.0);
    let sweep = node.arc_sweep_angle.unwrap_or(360.0);
    let inner = node.arc_inner_radius.unwrap_or(0.0).clamp(0.0, 1.0);
    let at = |deg: f32, scale: f32| -> Point2D {
        let a = deg.to_radians();
        Point2D::new(cx + rx * scale * a.cos(), cy + ry * scale * a.sin())
    };
    Some([
        (ArcHandle::Start, at(start, 1.0)),
        (ArcHandle::Sweep, at(start + sweep, 1.0)),
        (ArcHandle::Inner, at(start, inner)),
    ])
}

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
    pub input: TextInputState,
    pub now_ms: u64,
    /// Pre-resolved selection wash color (theme isn't reachable in the
    /// `paint_node` walker, so the viewport stashes it here).
    pub selection_color: Color,
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
    /// True while the Pen press-drag is minting handles — hides the
    /// rubber band (TS `isDraggingHandle`).
    pub(super) pen_dragging_handle: bool,
    /// Smart-guide alignment lines to paint during a node drag —
    /// doc-space, computed by the host's `align_guides` pass.
    pub(super) active_guides: Vec<op_editor_core::align_guides::AlignmentGuide>,
    /// Text node being edited (scene-space string id) and its shared
    /// draft/caret/selection state.
    pub(super) text_editing: Option<String>,
    pub(super) text_edit_input: TextInputState,
    /// Background fill outside any Frame.
    pub canvas_background: Color,
    pub theme: Theme,
    /// Host ms clock — text-edit caret blink.
    pub now_ms: u64,
    /// Node under the cursor (excluding selected nodes) — paints the
    /// dashed hover outline (TS `drawHoverOutline`).
    pub(super) hovered: Option<String>,
    /// Top-level frame labels: (scene id, display name, label colour).
    /// Collected from the canonical tree at build time (the scene
    /// carries no node names); painted screen-space above each root
    /// frame (TS `drawFrameLabelColored`).
    pub(super) frame_labels: Vec<(String, String, Color)>,
}

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
            pen_cursor_doc: state.ui.pen_cursor_doc.map(|p| Point2D::new(p.x, p.y)),
            pen_dragging_handle: state.ui.pen_dragging_handle,
            active_guides: state.editor_ui.active_guides.clone(),
            text_editing: state
                .ui
                .text_editing
                .as_ref()
                .map(|id| id.as_str().to_string()),
            text_edit_input: state.ui.text_edit_input.clone(),
            canvas_background: theme.canvas_surface,
            theme,
            now_ms: 0,
            hovered: state
                .editor_ui
                .canvas_hover_node
                .as_ref()
                // The outline is a Select-tool affordance; a stale id
                // from a previous tool must not paint.
                .filter(|_| matches!(state.tool, op_editor_core::Tool::Select))
                .filter(|id| !state.selection.set.iter().any(|s| s == *id))
                .map(|id| id.as_str().to_string()),
            frame_labels: collect_frame_labels(state),
        }
    }
}

/// Root-frame name labels (TS `drawFrameLabelColored` over
/// `renderNodes` with an empty clip stack): every named top-level
/// Frame gets a grey label; reusable components purple; instances
/// (Ref nodes — expanded to frames in the scene) the indigo
/// instance tint.
fn collect_frame_labels(state: &EditorState) -> Vec<(String, String, Color)> {
    use op_editor_core::PenNodeExt;
    const FRAME_LABEL: Color = Color {
        r: 0.6,
        g: 0.6,
        b: 0.6,
        a: 1.0,
    }; // #999999
    const COMPONENT: Color = Color {
        r: 0.658,
        g: 0.333,
        b: 0.969,
        a: 1.0,
    }; // #a855f7
    const INSTANCE: Color = Color {
        r: 0.573,
        g: 0.506,
        b: 0.969,
        a: 1.0,
    }; // #9281f7
    state
        .active_children()
        .iter()
        .filter_map(|node| {
            use jian_ops_schema::node::PenNode;
            let name = node.base().name.clone().unwrap_or_default();
            if name.is_empty() {
                return None;
            }
            let color = match node {
                PenNode::Frame(f) if f.reusable == Some(true) => COMPONENT,
                PenNode::Frame(_) => FRAME_LABEL,
                PenNode::Ref(_) => INSTANCE,
                _ => return None,
            };
            Some((node.base().id.clone(), name, color))
        })
        .collect()
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
        super::canvas_viewport_grid::paint_grid(cx, rect, viewport, &self.theme);
        let indicators = op_editor_core::agent_indicators::snapshot_at_if_active(self.now_ms);

        // 3. Walk the active page; clip enforces widget bounds.
        if let Some(page) = self.scene.active_page() {
            let viewport_origin = Point2D::new(
                rect.origin.x + viewport.pan_x,
                rect.origin.y + viewport.pan_y,
            );
            let edit_caret = self.text_editing.as_ref().map(|id| EditCaret {
                editing: id.clone(),
                input: self.text_edit_input.clone(),
                now_ms: self.now_ms,
                selection_color: crate::widgets::text_selection::selection_color(&self.theme),
            });
            const CULL_MARGIN: f32 = 64.0;
            let cull = Rect {
                origin: Point2D::new(rect.origin.x - CULL_MARGIN, rect.origin.y - CULL_MARGIN),
                size: Point2D::new(
                    rect.size.x + CULL_MARGIN * 2.0,
                    rect.size.y + CULL_MARGIN * 2.0,
                ),
            };
            let reveal_schedule = indicators
                .as_ref()
                .and_then(|indicators| reveal_schedule_for_paint(&indicators.reveals, self.now_ms));
            for child in page.children.iter().rev() {
                if let Some(reveals) = reveal_schedule {
                    super::canvas_viewport_paint::paint_node_with_reveals(
                        cx,
                        child,
                        viewport_origin,
                        viewport.zoom,
                        edit_caret.clone(),
                        cull,
                        reveals,
                    );
                } else {
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
            if let Some(indicators) = indicators.as_ref() {
                super::canvas_agent_overlay::paint_agent_frame_indicators_with_snapshot(
                    cx,
                    &page.children,
                    viewport_origin,
                    viewport.zoom,
                    self.now_ms,
                    indicators,
                );
            }
            if let Some(hovered) = self.hovered.as_ref() {
                if let Some(node) = page.find(hovered) {
                    const HOVER: Color = Color {
                        r: 0.231,
                        g: 0.51,
                        b: 0.965,
                        a: 1.0,
                    };
                    let b = node.aggregate_bounds();
                    let screen = Rect {
                        origin: Point2D::new(
                            viewport_origin.x + b.origin.x * viewport.zoom,
                            viewport_origin.y + b.origin.y * viewport.zoom,
                        ),
                        size: Point2D::new(b.size.x * viewport.zoom, b.size.y * viewport.zoom),
                    };
                    paint_dashed_rect(cx, screen, HOVER, 1.5);
                }
            }
            for (id, name, color) in &self.frame_labels {
                let Some(node) = page.find(id) else { continue };
                let b = node.aggregate_bounds();
                let sx = viewport_origin.x + b.origin.x * viewport.zoom;
                let sy = viewport_origin.y + b.origin.y * viewport.zoom;
                if sy < rect.origin.y || sy > rect.origin.y + rect.size.y + 18.0 {
                    continue;
                }
                // Horizontal cull — generous width allowance so a
                // label whose start is left of the clip still paints
                // its visible tail.
                if sx > rect.origin.x + rect.size.x || sx + 600.0 < rect.origin.x {
                    continue;
                }
                let layout = crate::TextLayout::single_run(
                    name,
                    "system-ui",
                    12.0,
                    jian_core::scene::Color::rgba(
                        (color.r * 255.0) as u8,
                        (color.g * 255.0) as u8,
                        (color.b * 255.0) as u8,
                        255,
                    ),
                    Point2D::new(0.0, 0.0),
                )
                .with_font_weight(500);
                cx.backend.draw_text(&layout, Point2D::new(sx, sy - 18.0));
            }
        }

        // 3a. Smart-guide alignment lines (magenta) — painted over the
        // nodes during a node drag, cleared on release.
        if !self.active_guides.is_empty() {
            const GUIDE_COLOR: Color = Color {
                r: 0.93,
                g: 0.12,
                b: 0.55,
                a: 1.0,
            };
            for g in &self.active_guides {
                let (from, to) = if g.vertical {
                    let x = rect.origin.x + viewport.pan_x + (g.pos as f32) * viewport.zoom;
                    let y0 = rect.origin.y + viewport.pan_y + (g.start as f32) * viewport.zoom;
                    let y1 = rect.origin.y + viewport.pan_y + (g.end as f32) * viewport.zoom;
                    (Point2D::new(x, y0), Point2D::new(x, y1))
                } else {
                    let y = rect.origin.y + viewport.pan_y + (g.pos as f32) * viewport.zoom;
                    let x0 = rect.origin.x + viewport.pan_x + (g.start as f32) * viewport.zoom;
                    let x1 = rect.origin.x + viewport.pan_x + (g.end as f32) * viewport.zoom;
                    (Point2D::new(x0, y), Point2D::new(x1, y))
                };
                cx.backend.stroke_line(from, to, GUIDE_COLOR, 1.0);
            }
        }

        // 4. Selection overlay — outlines + handles (single-select only).
        let show_handles = self.selected_set.len() == 1;
        let active_page = self.scene.active_page();
        let single_selected_id = self.selected_set.first().map(String::as_str);
        let single_selected_node = if show_handles {
            active_page.and_then(|page| single_selected_id.and_then(|id| page.find(id)))
        } else {
            None
        };
        let anchor_selected_node = if single_selected_id == Some(self.selected.as_str()) {
            single_selected_node
        } else {
            None
        };
        if let Some(page) = active_page {
            let mut paint_selected_node = |node: &SceneNode| {
                if node.hidden
                    || indicators.as_ref().is_some_and(|indicators| {
                        indicators
                            .reveals
                            .get(&node.id)
                            .is_some_and(|started_at| self.now_ms < *started_at)
                    })
                {
                    return;
                }
                let bounds = node.aggregate_bounds();
                if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
                    return;
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
            };
            if let Some(node) = single_selected_node {
                paint_selected_node(node);
            } else {
                for id in &self.selected_set {
                    let Some(node) = page.find(id) else {
                        continue;
                    };
                    paint_selected_node(node);
                }
            }
        }

        // 4b. Pen + path-editing overlays (`canvas_path_overlay.rs`):
        //     - TS `drawPenPreview` while a pen session is authoring
        //       (blue segments + dashed rubber band + anchor dots);
        //     - the ghost-handle edit overlay when the Pen tool has a
        //       single Path selected (pre-existing Rust superset);
        //     - TS `drawPathEditor` for a single selected Path under
        //       any other tool (Select-tool bezier editing, #5).
        super::canvas_path_overlay::paint_path_overlays(
            cx,
            self.scene,
            &self.theme,
            self.tool,
            self.pen_in_progress.as_deref(),
            self.pen_cursor_doc,
            self.pen_dragging_handle,
            &self.selected,
            self.selected_set.len(),
            anchor_selected_node,
            rect,
            viewport,
        );

        // 4c. Arc-edit handles for a single-selected Ellipse with the
        //     Select tool — start / sweep / inner-radius grab dots.
        if matches!(self.tool, op_editor_core::Tool::Select) && self.selected_set.len() == 1 {
            if let Some(node) = anchor_selected_node {
                if let Some(handles) = arc_handle_positions(node) {
                    let zoom = viewport.zoom;
                    let to_screen = |p: Point2D| {
                        Point2D::new(
                            rect.origin.x + viewport.pan_x + p.x * zoom,
                            rect.origin.y + viewport.pan_y + p.y * zoom,
                        )
                    };
                    // Rotate the overlay to match a rotated ellipse.
                    let rotated = node.rotation.abs() > f32::EPSILON;
                    if rotated {
                        let b = node.bounds;
                        let pivot = to_screen(Point2D::new(
                            b.origin.x + b.size.x / 2.0,
                            b.origin.y + b.size.y / 2.0,
                        ));
                        cx.backend.save();
                        cx.backend.rotate(node.rotation, pivot);
                    }
                    let r = 4.5; // screen-px radius
                    for (_, p) in handles {
                        let center = to_screen(p);
                        let bounds = Rect {
                            origin: Point2D::new(center.x - r, center.y - r),
                            size: Point2D::new(r * 2.0, r * 2.0),
                        };
                        // Filled primary dot — distinct from the white
                        // square resize handles.
                        cx.backend.fill_oval(bounds, self.theme.primary);
                        cx.backend.stroke_oval(bounds, self.theme.background, 1.5);
                    }
                    if rotated {
                        cx.backend.restore();
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

pub(crate) fn reveal_schedule_for_paint<'a>(
    reveals: &'a std::collections::HashMap<String, u64>,
    now_ms: u64,
) -> Option<super::canvas_viewport_paint::RevealSchedule<'a>> {
    (!reveals.is_empty()).then_some(super::canvas_viewport_paint::RevealSchedule {
        starts: reveals,
        now_ms,
    })
}

/// Stroke a dashed rectangle as 4 dashed edges (4 px on / 4 px off,
/// screen-space) — the `RenderBackend` trait has no path-effect
/// surface, so the dash is segmented by hand; backend-agnostic.
pub(super) fn paint_dashed_rect(cx: &mut PaintCx<'_>, rect: Rect, color: Color, width: f32) {
    const ON: f32 = 4.0;
    const OFF: f32 = 4.0;
    let mut dash_line = |from: Point2D, to: Point2D| {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= f32::EPSILON {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        let mut t = 0.0;
        while t < len {
            let end = (t + ON).min(len);
            cx.backend.stroke_line(
                Point2D::new(from.x + ux * t, from.y + uy * t),
                Point2D::new(from.x + ux * end, from.y + uy * end),
                color,
                width,
            );
            t = end + OFF;
        }
    };
    let tl = rect.origin;
    let tr = Point2D::new(rect.origin.x + rect.size.x, rect.origin.y);
    let br = Point2D::new(rect.origin.x + rect.size.x, rect.origin.y + rect.size.y);
    let bl = Point2D::new(rect.origin.x, rect.origin.y + rect.size.y);
    dash_line(tl, tr);
    dash_line(tr, br);
    dash_line(br, bl);
    dash_line(bl, tl);
}

#[cfg(test)]
#[path = "canvas_viewport_tests.rs"]
mod tests;

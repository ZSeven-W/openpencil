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
//! [`rotation_corner_at_point`] / [`selection_handle_at_point`] still
//! consume the shell-core `Document` — they serve the hosts' input
//! dispatch, not widget paint, and a later task migrates them.
//!
//! Per-kind paint:
//! - Frame: fill (if any) at `bounds`, optional stroke, then recurse.
//! - Group / Other(_): no own paint, just recurse into children.
//! - Rect / Ellipse / Polygon / Line / Path / Text: per-kind paint.
//! - `Other("icon_font")`: lucide glyph from `text`.
//!
//! Selection overlay (outlines + handles), grid, pen rubber-band and
//! per-anchor Path handles are layered on top of the resolved scene.

use crate::document::{Document, NodeKind, Viewport as DocViewport};
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

/// Hit-test the rotation ring that sits just outside the four
/// corner handles. Returns the nearest corner (so the runner can
/// hint which way the rotation drag is anchored) or `None` if the
/// cursor isn't in a rotation zone.
///
/// The rotation zone is an annulus around each corner — beyond
/// the 6 px handle slop and inside the 16 px outer radius. Matches
/// the TS `hitTestRotation` logic.
///
/// INPUT path — stays on `&Document` (the host input dispatch still
/// reasons in `Document` space; a later task migrates it).
pub fn rotation_corner_at_point(
    canvas_rect: Rect,
    doc: &Document,
    point: Point2D,
) -> Option<SelectionHandle> {
    // Rotation rings are only painted on single-select (the
    // multi-select overlay is outline-only), so gate the hit-test
    // to match — otherwise non-anchor "rotation zones" would
    // intercept clicks on dead air.
    if doc.selection_count() != 1 {
        return None;
    }
    let node = doc.selected_node()?;
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    let viewport = &doc.viewport;
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
/// INPUT path — stays on `&Document` (see [`rotation_corner_at_point`]).
pub fn selection_handle_at_point(
    canvas_rect: Rect,
    doc: &Document,
    point: Point2D,
) -> Option<SelectionHandle> {
    // Handles are only painted on single-select (the multi-select
    // overlay is outline-only — Figma parity), so gate the hit-
    // test to match. Otherwise the "anchor's handles" would hit-
    // test even though no handles are visible anywhere.
    if doc.selection_count() != 1 {
        return None;
    }
    let node = doc.selected_node()?;
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    let viewport = &doc.viewport;
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
    /// `op_pen_loader::editor_state_to_layout_scene` and caches it the
    /// same way `paint_doc` is cached.
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
mod tests {
    use super::*;
    use crate::layout_scene::{LayoutScene, SceneFillType, SceneNode, ScenePage, SceneStroke};
    use crate::{Color, Point2D, Rect, TextLayout};

    /// Records op order; clip-isolated paint = `Save, Clip, Fill, …, Restore`.
    #[derive(Debug, PartialEq, Eq)]
    enum Op {
        Save,
        Restore,
        Clip,
        Fill,
        Stroke,
        Text,
    }

    #[derive(Default)]
    struct RecordingBackend {
        ops: Vec<Op>,
        rects: usize,
        strokes: usize,
        text: usize,
    }

    impl crate::RenderBackend for RecordingBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {
            self.rects += 1;
            self.ops.push(Op::Fill);
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {
            self.strokes += 1;
            self.ops.push(Op::Stroke);
        }
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {
            self.text += 1;
            self.ops.push(Op::Text);
        }
        fn clip_rect(&mut self, _: Rect) {
            self.ops.push(Op::Clip);
        }
        fn save(&mut self) {
            self.ops.push(Op::Save);
        }
        fn restore(&mut self) {
            self.ops.push(Op::Restore);
        }
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {
            self.strokes += 1;
            self.ops.push(Op::Stroke);
        }
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
            self.rects += 1;
            self.ops.push(Op::Fill);
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {
            self.strokes += 1;
            self.ops.push(Op::Stroke);
        }
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {
            self.strokes += 1;
            self.ops.push(Op::Stroke);
        }
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    /// A leaf scene node with bounds + optional fill.
    fn leaf(id: &str, kind: NodeKind, bounds: Rect, fill: Option<Color>) -> SceneNode {
        let mut n = SceneNode::leaf(id, kind);
        n.bounds = bounds;
        n.fill = fill;
        n
    }

    /// A one-page scene mirroring `Document::sample`: a Frame with a
    /// stroke, a filled Rect child, and two Text nodes.
    fn sample_scene() -> LayoutScene {
        let mut frame = SceneNode::leaf("n1", NodeKind::Frame);
        frame.bounds = Rect::xywh(40.0, 40.0, 320.0, 200.0);
        frame.fill = Some(Color { r: 0.16, g: 0.16, b: 0.2, a: 1.0 });
        frame.stroke = Some(SceneStroke { color: Color::WHITE, width: 1.0 });
        frame.fill_type = SceneFillType::Solid;
        let mut button = leaf(
            "n2",
            NodeKind::Rect,
            Rect::xywh(60.0, 80.0, 120.0, 40.0),
            Some(Color::BLUE),
        );
        button.stroke = None;
        let mut title = SceneNode::leaf("n3", NodeKind::Text);
        title.bounds = Rect::xywh(60.0, 60.0, 200.0, 20.0);
        title.text = Some("Title".to_string());
        let mut label = SceneNode::leaf("n4", NodeKind::Text);
        label.bounds = Rect::xywh(70.0, 90.0, 100.0, 16.0);
        label.text = Some("Button".to_string());
        frame.children = vec![button, title, label];
        LayoutScene {
            pages: vec![ScenePage {
                id: "p1".into(),
                name: "Page 1".into(),
                children: vec![frame],
            }],
            active_page_index: 0,
        }
    }

    fn sample_state() -> EditorState {
        EditorState::sample()
    }

    #[test]
    fn from_sample_scene_paints_expected_primitives() {
        let state = sample_state();
        let scene = sample_scene();
        let mut viewport = CanvasViewport::from_editor(&state, &scene);
        // Select the Frame so the overlay stroke paints.
        viewport.selected = "n1".into();
        viewport.selected_set = vec!["n1".into()];
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
        }
        // ≥3 fills (canvas bg, frame fill, button rect), ≥2 strokes
        // (frame outline + selection overlay), 2 text draws.
        assert!(backend.rects >= 3, "expected ≥3 fills, got {}", backend.rects);
        assert!(
            backend.strokes >= 2,
            "expected ≥2 strokes (frame + selection overlay), got {}",
            backend.strokes
        );
        assert_eq!(backend.text, 2, "two text nodes draw two text runs");
    }

    #[test]
    fn empty_scene_paints_canvas_background_and_grid_only() {
        let state = sample_state();
        let scene = LayoutScene::default();
        let viewport = CanvasViewport::from_editor(&state, &scene);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 100.0, 100.0));
        }
        // Infinite-canvas: bg + grid dots, no document-side strokes
        // / text.
        assert!(backend.rects >= 1, "canvas bg + grid dots");
        assert_eq!(backend.strokes, 0);
        assert_eq!(backend.text, 0);
    }

    #[test]
    fn unselected_scene_skips_overlay_stroke() {
        let state = sample_state();
        let scene = sample_scene();
        // No selection — only the frame's own stroke paints.
        let viewport = CanvasViewport::from_editor(&state, &scene);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
        }
        assert_eq!(backend.strokes, 1, "no selection => only the frame stroke");
    }

    #[test]
    fn access_node_advertises_canvas_role() {
        let state = sample_state();
        let scene = sample_scene();
        let viewport = CanvasViewport::from_editor(&state, &scene);
        let node = viewport.access_node();
        assert_eq!(node.role(), accesskit::Role::Canvas);
        assert_eq!(node.label(), Some("Canvas"));
    }

    #[test]
    fn paint_is_clip_isolated_save_clip_then_restore() {
        let state = sample_state();
        let scene = sample_scene();
        let viewport = CanvasViewport::from_editor(&state, &scene);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
        }
        // First three ops: Save, Clip, Fill (the canvas bg).
        assert_eq!(
            &backend.ops[..3],
            &[Op::Save, Op::Clip, Op::Fill],
            "canvas paint must open with Save → Clip → bg Fill"
        );
        assert_eq!(
            backend.ops.last(),
            Some(&Op::Restore),
            "canvas paint must close with Restore"
        );
        let saves = backend.ops.iter().filter(|o| **o == Op::Save).count();
        let restores = backend.ops.iter().filter(|o| **o == Op::Restore).count();
        assert_eq!(saves, restores, "balanced save/restore");
        assert_eq!(saves, 1);
    }

    #[test]
    fn paint_with_zero_size_rect_skips_entirely() {
        let state = sample_state();
        let scene = sample_scene();
        let viewport = CanvasViewport::from_editor(&state, &scene);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 0.0, 0.0));
        }
        assert!(backend.ops.is_empty(), "zero-size rect must paint nothing");
    }

    #[test]
    fn group_kind_recurses_without_own_paint() {
        let state = sample_state();
        let inner = leaf(
            "n2",
            NodeKind::Rect,
            Rect::xywh(0.0, 0.0, 50.0, 50.0),
            Some(Color::RED),
        );
        let mut group = SceneNode::leaf("n3", NodeKind::Group);
        group.bounds = Rect::xywh(10.0, 10.0, 80.0, 80.0);
        group.fill = Some(Color::BLUE); // fill on group should be ignored
        group.children = vec![inner];
        let scene = LayoutScene {
            pages: vec![ScenePage {
                id: "n1".into(),
                name: "p".into(),
                children: vec![group],
            }],
            active_page_index: 0,
        };
        let viewport = CanvasViewport::from_editor(&state, &scene);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 200.0, 200.0));
        }
        // canvas bg (1) + grid dots (variable) + leaf rect fill (1)
        // — group fill skipped.
        assert!(backend.rects >= 2, "canvas bg + at least the leaf");
    }
}

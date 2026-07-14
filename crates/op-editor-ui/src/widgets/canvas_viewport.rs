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
use crate::widgets::canvas_frame_labels::FrameLabel;
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

impl SelectionHandle {
    /// Authored dimensions changed by this handle.
    pub fn resize_axes(self) -> op_editor_core::drag_mutators::ResizeAxes {
        use op_editor_core::drag_mutators::ResizeAxes;
        match (self.resizes_width(), self.resizes_height()) {
            (true, true) => ResizeAxes::Both,
            (true, false) => ResizeAxes::Width,
            (false, true) => ResizeAxes::Height,
            (false, false) => unreachable!("every selection handle resizes at least one axis"),
        }
    }

    /// Whether this handle authors the selected node's width.
    pub fn resizes_width(self) -> bool {
        !matches!(self, Self::Top | Self::Bottom)
    }

    /// Whether this handle authors the selected node's height.
    pub fn resizes_height(self) -> bool {
        !matches!(self, Self::Left | Self::Right)
    }

    /// Whether dragging this handle moves the selected node's left edge.
    pub fn moves_left_edge(self) -> bool {
        matches!(self, Self::Left | Self::TopLeft | Self::BottomLeft)
    }

    /// Whether dragging this handle moves the selected node's top edge.
    pub fn moves_top_edge(self) -> bool {
        matches!(self, Self::Top | Self::TopLeft | Self::TopRight)
    }
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
    /// Pencil-style selected element/count label painted near the overlay.
    pub(super) selection_label: Option<String>,
    /// Active canvas tool — gates the per-anchor Path handles.
    pub(super) tool: op_editor_core::Tool,
    /// Pen-tool draft: the in-progress path id + last cursor doc
    /// coord, used to paint the rubber-band preview.
    pub(super) pen_in_progress: Option<String>,
    /// Ghost of the blank starter frame `(x, y, w, h)` doc-px — painted
    /// after a design prompt clears the real node, until the generated
    /// design's sized root arrives, so the canvas never flashes empty.
    /// User-selected pencil-cursor silhouette (Settings > System).
    pub(super) pencil_cursor_style: op_editor_core::PencilCursorStyle,
    pub(super) starter_ghost: Option<[f32; 4]>,
    pub(super) pen_cursor_doc: Option<Point2D>,
    /// True while the Pen press-drag is minting handles — hides the
    /// rubber band (TS `isDraggingHandle`).
    pub(super) pen_dragging_handle: bool,
    /// Smart-guide alignment lines to paint during a node drag —
    /// doc-space, computed by the host's `align_guides` pass.
    pub(super) active_guides: Vec<op_editor_core::align_guides::AlignmentGuide>,
    /// Drop-target preview during canvas node dragging.
    pub(super) drop_indicator: Option<op_editor_core::editor_ui_state::CanvasDropIndicator>,
    /// True while a selected canvas node is actively being dragged.
    /// Selection chrome is hidden in that state so the dragged
    /// element itself is the only moving visual affordance.
    pub(super) node_drag_active: bool,
    /// Optional floating copy of the dragged node. The base scene can
    /// still be reflowed to preview sibling avoidance while this copy
    /// follows the cursor.
    pub(super) node_drag_overlay: Option<CanvasNodeDragOverlay>,
    /// Text node being edited (scene-space string id) and its shared
    /// draft/caret/selection state.
    pub(super) text_editing: Option<String>,
    pub(super) text_edit_input: TextInputState,
    /// Background fill outside any Frame.
    pub canvas_background: Color,
    pub theme: Theme,
    /// Host ms clock — text-edit caret blink.
    pub now_ms: u64,
    /// Hierarchy focus under the cursor. An unselected focus paints a
    /// solid outline; its direct visible children paint dashed hints.
    pub(super) hovered: Option<String>,
    /// Top-level frame labels, including transient generation state.
    /// Collected from the canonical tree at build time (the scene
    /// carries no node names); painted screen-space above each root
    /// frame (TS `drawFrameLabelColored`).
    pub(super) frame_labels: Vec<FrameLabel>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasNodeDragOverlay {
    pub node_id: String,
    pub target_origin_doc: Point2D,
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
            selection_label: selection_size_label(state, scene),
            tool: state.tool,
            pen_in_progress: state
                .ui
                .pen_in_progress
                .as_ref()
                .map(|id| id.as_str().to_string()),
            starter_ghost: state.editor_ui.starter_ghost,
            pencil_cursor_style: state.editor_ui.pencil_cursor_style,
            pen_cursor_doc: state.ui.pen_cursor_doc.map(|p| Point2D::new(p.x, p.y)),
            pen_dragging_handle: state.ui.pen_dragging_handle,
            active_guides: state.editor_ui.active_guides.clone(),
            drop_indicator: state.editor_ui.canvas_drop_indicator.clone(),
            node_drag_active: false,
            node_drag_overlay: None,
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
                .map(|id| id.as_str().to_string()),
            frame_labels: collect_frame_labels(state),
        }
    }

    /// Read-only construction for the embedding SDK: paints `scene` at
    /// `viewport` with no selection / tool / editing affordances. Frame
    /// labels are derived from the scene's top-level node names rather
    /// than an `EditorState`, so no editor is required to render.
    ///
    /// All editor-specific fields (selection, pen draft, text-edit,
    /// hover, guides) are left empty/default so the widget paints a
    /// clean viewer layer — no overlays, no interactive affordances.
    pub fn from_scene(scene: &'a LayoutScene, viewport: DocViewport, theme: Theme) -> Self {
        let canvas_background = theme.canvas_surface;
        Self {
            id: WidgetId::new(4000),
            viewport,
            scene,
            selected: String::new(),
            selected_set: Vec::new(),
            selection_label: None,
            tool: op_editor_core::Tool::Select,
            pen_in_progress: None,
            starter_ghost: None,
            pencil_cursor_style: Default::default(),
            pen_cursor_doc: None,
            pen_dragging_handle: false,
            active_guides: Vec::new(),
            drop_indicator: None,
            node_drag_active: false,
            node_drag_overlay: None,
            text_editing: None,
            text_edit_input: Default::default(),
            canvas_background,
            theme,
            now_ms: 0,
            hovered: None,
            frame_labels: Vec::new(),
        }
    }

    pub fn frame_label_at_point(&self, rect: Rect, point: Point2D) -> Option<String> {
        let page = self.scene.active_page()?;
        let viewport_origin = Point2D::new(
            rect.origin.x + self.viewport.pan_x,
            rect.origin.y + self.viewport.pan_y,
        );
        super::canvas_frame_labels::frame_label_at_point(
            &page.children,
            &self.frame_labels,
            viewport_origin,
            &self.viewport,
            rect,
            point,
        )
    }

    pub fn set_node_drag_active(&mut self, active: bool) {
        self.node_drag_active = active;
    }

    pub fn set_node_drag_overlay(&mut self, overlay: Option<CanvasNodeDragOverlay>) {
        self.node_drag_overlay = overlay;
    }
}

/// Root-frame name labels (TS `drawFrameLabelColored` over
/// `renderNodes` with an empty clip stack): every named top-level
/// Frame gets a grey label; reusable components purple; instances
/// (Ref nodes — expanded to frames in the scene) the indigo
/// instance tint.
fn collect_frame_labels(state: &EditorState) -> Vec<FrameLabel> {
    use op_editor_core::PenNodeExt;
    let theme = theme_for(&state.editor_ui);
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
            let id = op_editor_core::NodeId::new(node.base().id.clone());
            let generating = matches!(node, PenNode::Frame(_))
                && op_editor_core::agent_indicators::is_frame_generating(node.base().id.as_str());
            let color = if generating || state.selection.contains(&id) {
                theme.primary
            } else {
                match node {
                    PenNode::Frame(f) if f.reusable == Some(true) => COMPONENT,
                    PenNode::Frame(_) => FRAME_LABEL,
                    PenNode::Ref(_) => INSTANCE,
                    _ => return None,
                }
            };
            Some(FrameLabel::new(
                node.base().id.clone(),
                generating_label_text(&name, generating),
                color,
                generating,
            ))
        })
        .collect()
}

fn generating_label_text(base_name: &str, generating: bool) -> String {
    if generating {
        format!("Generating: {base_name}")
    } else {
        base_name.to_string()
    }
}

fn selection_size_label(state: &EditorState, scene: &LayoutScene) -> Option<String> {
    if state.selection_count() == 0 {
        return None;
    }
    let page = scene.active_page()?;
    let mut union: Option<Rect> = None;
    for id in &state.selection.set {
        if !id.is_real() {
            continue;
        }
        let Some(node) = page.find(id.as_str()) else {
            continue;
        };
        union = match union {
            Some(rect) => Some(union_rects(rect, node.aggregate_bounds())),
            None => Some(node.aggregate_bounds()),
        };
    }
    let rect: Rect = union?;
    if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
        return None;
    }
    Some(format_dimension_label(rect.size.x, rect.size.y))
}

fn format_dimension_label(width: f32, height: f32) -> String {
    format!(
        "{} × {}",
        width.round().max(0.0) as i32,
        height.round().max(0.0) as i32
    )
}

fn union_rects(a: Rect, b: Rect) -> Rect {
    let min_x = a.origin.x.min(b.origin.x);
    let min_y = a.origin.y.min(b.origin.y);
    let max_x = (a.origin.x + a.size.x).max(b.origin.x + b.size.x);
    let max_y = (a.origin.y + a.size.y).max(b.origin.y + b.size.y);
    Rect::xywh(min_x, min_y, max_x - min_x, max_y - min_y)
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
        let selection_chrome_visible = !self.node_drag_active;
        let show_handles = selection_chrome_visible && self.selected_set.len() == 1;
        let single_selected_id = self.selected_set.first().map(String::as_str);
        let selected_lookup = if show_handles {
            single_selected_id
        } else {
            None
        };
        let hovered_lookup = if selection_chrome_visible {
            self.hovered.as_deref()
        } else {
            None
        };
        let hovered_focus_selected = hovered_lookup
            .is_some_and(|hovered| self.selected_set.iter().any(|selected| selected == hovered));
        // Root labels carry their durable generating/selection colour
        // from construction. Apply the transient hover tint here so
        // `node_drag_active` suppresses it together with every other
        // hierarchy-hover affordance.
        let hovered_frame_labels = hovered_lookup.map(|hovered| {
            self.frame_labels
                .iter()
                .map(|label| {
                    let mut label = label.clone();
                    if label.id == hovered {
                        label.color = self.theme.primary;
                    }
                    label
                })
                .collect::<Vec<_>>()
        });
        let frame_labels = hovered_frame_labels
            .as_deref()
            .unwrap_or(&self.frame_labels);
        let selected_root_frame_label = selection_chrome_visible
            && show_handles
            && self.scene.active_page().is_some_and(|page| {
                single_selected_id.is_some_and(|id| page.children.iter().any(|node| node.id == id))
            });
        let mut paint_hits = super::canvas_viewport_paint::PaintNodeHits::default();

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
            let hidden_drag_node = self
                .node_drag_overlay
                .as_ref()
                .map(|overlay| overlay.node_id.as_str());
            let generation_sets = super::canvas_generation_scan::generating_paint_sets(
                &page.children,
                indicators.as_ref(),
                self.now_ms,
            );
            // Starter ghost: after a design prompt clears the blank starter
            // frame, keep painting its silhouette (white artboard + name
            // label) until the generated design's root lands — the canvas
            // must never flash empty between prompt and first batch.
            if let Some([gx, gy, gw, gh]) = self.starter_ghost {
                let ghost = Rect {
                    origin: Point2D::new(
                        viewport_origin.x + gx * viewport.zoom,
                        viewport_origin.y + gy * viewport.zoom,
                    ),
                    size: Point2D::new(gw * viewport.zoom, gh * viewport.zoom),
                };
                cx.backend.fill_rect(ghost, Color::WHITE);
                cx.backend
                    .stroke_rect(ghost, self.theme.border.with_alpha(0.8), 1.0);
                let mf = self.theme.muted_foreground;
                let label = crate::TextLayout::single_run(
                    "Frame",
                    "system-ui",
                    11.0,
                    jian_core::scene::Color::rgba(
                        (mf.r * 255.0) as u8,
                        (mf.g * 255.0) as u8,
                        (mf.b * 255.0) as u8,
                        255,
                    ),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend
                    .draw_text(&label, Point2D::new(ghost.origin.x, ghost.origin.y - 6.0));
            }
            for child in page.children.iter().rev() {
                let child_hits = super::canvas_viewport_paint::paint_node_with_options_hiding(
                    cx,
                    child,
                    viewport_origin,
                    viewport.zoom,
                    edit_caret.clone(),
                    cull,
                    reveal_schedule,
                    hovered_lookup,
                    selected_lookup,
                    self.pen_in_progress.as_deref(),
                    hidden_drag_node,
                    self.now_ms,
                    generation_sets.as_ref().map(|sets| &sets.scan),
                    generation_sets
                        .as_ref()
                        .map(|_| super::canvas_generation_scan::SKELETON_BLUE),
                    generation_sets.as_ref().map(|sets| &sets.queued),
                );
                paint_hits.merge_missing(child_hits);
            }
            if let Some(overlay) = self.node_drag_overlay.as_ref() {
                if let Some(node) = page.find(overlay.node_id.as_str()) {
                    let mut floating = node.clone();
                    let current = floating.aggregate_bounds();
                    super::canvas_layout_transition::translate_scene_subtree(
                        &mut floating,
                        overlay.target_origin_doc.x - current.origin.x,
                        overlay.target_origin_doc.y - current.origin.y,
                    );
                    let _ = super::canvas_viewport_paint::paint_node_with_options(
                        cx,
                        &floating,
                        viewport_origin,
                        viewport.zoom,
                        None,
                        cull,
                        reveal_schedule,
                        None,
                        None,
                        None,
                    );
                }
            }
            if let Some(indicators) = indicators.as_ref() {
                super::canvas_agent_cursor::paint_agent_cursors(
                    cx,
                    &page.children,
                    viewport_origin,
                    viewport.zoom,
                    self.now_ms,
                    indicators,
                    self.pencil_cursor_style,
                    Point2D::new(
                        rect.origin.x + rect.size.x / 2.0,
                        rect.origin.y + rect.size.y / 2.0,
                    ),
                );
            }
            const HOVER: Color = Color {
                r: 0.231,
                g: 0.51,
                b: 0.965,
                a: 1.0,
            };
            if !hovered_focus_selected {
                if let Some(screen) = paint_hits.hover_rect {
                    // Replay the focus node's root→node flip/rotation
                    // chain so its solid outline lands on the rendered
                    // geometry, not the unrotated doc-space bounds.
                    let hover_transformed = super::canvas_overlay_transform::replay_on_backend(
                        cx,
                        &paint_hits.hover_transforms,
                    );
                    cx.backend.stroke_rect(screen, HOVER, 1.5);
                    if hover_transformed {
                        cx.backend.restore();
                    }
                }
            }
            for (screen, transforms) in &paint_hits.hover_child_rects {
                // A direct child can add its own flip/rotation after
                // the focus node's ancestor chain, so replay each hint
                // independently instead of sharing the focus transform.
                let hover_transformed =
                    super::canvas_overlay_transform::replay_on_backend(cx, transforms);
                paint_dashed_rect(cx, *screen, HOVER, 1.5);
                if hover_transformed {
                    cx.backend.restore();
                }
            }
            super::canvas_frame_labels::paint_frame_labels(
                cx,
                &page.children,
                frame_labels,
                if selected_root_frame_label {
                    &[]
                } else {
                    &self.selected_set
                },
                viewport_origin,
                viewport,
                rect,
            );
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

        // 3b. Drag/drop preview — transient target highlight, ghost
        // bounds, and flex insertion line. It is painted over nodes
        // but below the final selection handles.
        if let Some(indicator) = self.drop_indicator.as_ref() {
            paint_drop_indicator(
                cx,
                rect,
                viewport,
                &self.theme,
                indicator,
                !self.node_drag_active,
            );
        }

        // 4. Selection overlay — outlines + handles (single-select only).
        let active_page = self.scene.active_page();
        let single_selected_node = if show_handles {
            paint_hits.selected_node
        } else {
            None
        };
        let anchor_selected_node = if single_selected_id == Some(self.selected.as_str()) {
            single_selected_node
        } else {
            None
        };
        if selection_chrome_visible {
            if let Some(page) = active_page {
                let selection_input = super::canvas_selection_overlay::SelectionPaintInput {
                    theme: &self.theme,
                    indicators: indicators.as_ref(),
                    now_ms: self.now_ms,
                    canvas_rect: rect,
                    viewport,
                    selection_label: self.selection_label.as_deref(),
                };
                if let Some(node) = single_selected_node {
                    super::canvas_selection_overlay::paint_selected_node(
                        cx,
                        node,
                        &selection_input,
                        show_handles,
                        &paint_hits.selected_transforms,
                    );
                } else if !self.selected_set.is_empty() {
                    super::canvas_selection_overlay::paint_multi_selection_overlays(
                        cx,
                        &page.children,
                        &self.selected_set,
                        &selection_input,
                    );
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
            &self.theme,
            self.tool,
            self.pen_in_progress.is_some(),
            paint_hits.pen_node,
            self.pen_cursor_doc,
            self.pen_dragging_handle,
            self.selected_set.len(),
            anchor_selected_node,
            &paint_hits.selected_transforms,
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
                    // Replay the selected ellipse's transform chain so
                    // arc handles follow rotated/flipped ancestors too.
                    let transformed = super::canvas_overlay_transform::replay_on_backend(
                        cx,
                        &paint_hits.selected_transforms,
                    ) || {
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
                        rotated
                    };
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
                    if transformed {
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

fn paint_drop_indicator(
    cx: &mut PaintCx<'_>,
    canvas_rect: Rect,
    viewport: &DocViewport,
    theme: &Theme,
    indicator: &op_editor_core::editor_ui_state::CanvasDropIndicator,
    paint_ghost: bool,
) {
    let to_screen_rect = |r: op_editor_core::editor_ui_state::CanvasOverlayRect| {
        Rect::xywh(
            canvas_rect.origin.x + viewport.pan_x + r.x as f32 * viewport.zoom,
            canvas_rect.origin.y + viewport.pan_y + r.y as f32 * viewport.zoom,
            r.w as f32 * viewport.zoom,
            r.h as f32 * viewport.zoom,
        )
    };
    let primary = theme.primary;
    if let Some(target) = indicator.target {
        let rect = to_screen_rect(target);
        let fill = Color { a: 0.08, ..primary };
        let stroke = Color { a: 0.45, ..primary };
        cx.backend.fill_rect(rect, fill);
        cx.backend.stroke_rect(rect, stroke, 1.0);
    }
    if paint_ghost {
        let ghost = to_screen_rect(indicator.ghost);
        let ghost_fill = Color { a: 0.10, ..primary };
        let ghost_stroke = Color { a: 0.85, ..primary };
        cx.backend.fill_rect(ghost, ghost_fill);
        paint_dashed_rect(cx, ghost, ghost_stroke, 1.25);
    }
    if let Some(line) = indicator.insertion {
        let from = Point2D::new(
            canvas_rect.origin.x + viewport.pan_x + line.x1 as f32 * viewport.zoom,
            canvas_rect.origin.y + viewport.pan_y + line.y1 as f32 * viewport.zoom,
        );
        let to = Point2D::new(
            canvas_rect.origin.x + viewport.pan_x + line.x2 as f32 * viewport.zoom,
            canvas_rect.origin.y + viewport.pan_y + line.y2 as f32 * viewport.zoom,
        );
        cx.backend.stroke_line(from, to, primary, 2.0);
    }
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

#[cfg(test)]
#[path = "canvas_viewport_from_scene_tests.rs"]
mod from_scene_tests;

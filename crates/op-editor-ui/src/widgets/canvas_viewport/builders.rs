//! [`super::CanvasViewport`] constructors plus the derived chrome data
//! they collect — root-frame labels and the selection dimension label.
//!
//! Split out of `canvas_viewport.rs` to keep that spine under the
//! repository's 800-line cap.

use super::{CanvasNodeDragOverlay, CanvasViewport};
use crate::layout_scene::LayoutScene;
use crate::theme::Theme;
use crate::widgets::canvas_frame_labels::FrameLabel;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::WidgetId;
use crate::{Color, Point2D, Rect};
use op_editor_core::EditorState;
use op_editor_core::Viewport as DocViewport;
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
        let (canvas_background, show_grid) =
            crate::widgets::canvas_viewport_background::resolve(state, theme.canvas_surface);
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
            image_crop_edit_active: state
                .editor_ui
                .image_crop_editing
                .as_ref()
                .is_some_and(|id| id == &state.selection.anchor),
            node_drag_overlay: None,
            text_editing: state
                .ui
                .text_editing
                .as_ref()
                .map(|id| id.as_str().to_string()),
            text_edit_input: state.ui.text_edit_input.clone(),
            canvas_background,
            show_grid,
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
            collab_presence: crate::widgets::canvas_collab_presence::snapshot(
                &state.editor_ui.collab,
            ),
            fast_interaction: false,
            cull_override: None,
        }
    }

    /// Shift the paint origin by `(dx, dy)` logical px while keeping the
    /// doc↔screen mapping fixed. The host's pan bitmap cache paints into
    /// an offscreen layer whose rect is grown by a margin on every side;
    /// adding the margin here cancels the shifted `rect.origin` so nodes
    /// land at the same logical coordinates as an on-window paint.
    pub fn offset_paint_origin(&mut self, dx: f32, dy: f32) {
        self.viewport.pan_x += dx;
        self.viewport.pan_y += dy;
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
            image_crop_edit_active: false,
            node_drag_overlay: None,
            text_editing: None,
            text_edit_input: Default::default(),
            canvas_background,
            show_grid: true,
            theme,
            now_ms: 0,
            hovered: None,
            frame_labels: Vec::new(),
            collab_presence: Vec::new(),
            fast_interaction: false,
            cull_override: None,
        }
    }

    pub fn frame_label_at_point(&self, rect: Rect, point: Point2D) -> Option<String> {
        let page = self.scene.active_page()?;
        let viewport_origin = Point2D::new(
            rect.origin.x + self.viewport.pan_x,
            rect.origin.y + self.viewport.pan_y,
        );
        crate::widgets::canvas_frame_labels::frame_label_at_point(
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

pub(super) fn generating_label_text(base_name: &str, generating: bool) -> String {
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

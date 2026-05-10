//! `PropertyPanel` — right-rail node inspector (Step 6).
//!
//! Mirrors `apps/web/src/components/panels/right-panel.tsx` and the
//! per-section TS files (`*-section.tsx`). The bulk of the paint
//! logic lives in [`super::property_panel_sections`] — this file
//! holds the `PropertyPanel` struct, the `Widget` impl, and the
//! per-frame snapshot extraction. Splitting the file keeps both
//! pieces under the openpencil 800-line ceiling.
//!
//! Sections (top → bottom, mirroring TS order):
//!   1. Tab strip (设计 / 代码)
//!   2. Header (kind label) + 创建组件 button
//!   3. 位置 — X / Y / rotation / R
//!   4. 弹性布局 — 3 layout-mode buttons
//!   5. 尺寸 — W / H + 5 sizing checkboxes
//!   6. 图层 — opacity row
//!   7. 填充 — solid color rows + add affordance
//!   8. 描边 — color + width row
//!   9. 效果 — empty list + add affordance
//!  10. 导出 — scale + format dropdowns
//!
//! Conditional rendering: TS app does `{hasSelection && <RightPanel/>}`.
//! Host calls [`PropertyPanel::for_selection`] which returns
//! `Option<Self>`; `None` = panel hidden entirely.

use crate::document::{Document, Node, Stroke};
use crate::theme::Theme;
use crate::widgets::property_panel_sections as sections;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect};

pub const PROPERTY_PANEL_WIDTH: f32 = 280.0;

/// Snapshot of the selected node's editable fields, formatted for
/// display. Built once per `for_selection` call so all paint
/// helpers can read pre-computed strings instead of re-formatting.
#[derive(Debug, Clone)]
pub struct NodeSnapshot {
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub fill: Option<Color>,
    pub stroke: Option<Stroke>,
}

impl NodeSnapshot {
    fn from_node(node: &Node) -> Self {
        // Use `aggregate_bounds` so Group / unbounded container
        // nodes (Frame without explicit bounds, Other(_)) report
        // the visual extent of their subtree instead of "0 × 0"
        // (codex Step 6 stop-hook fix).
        let bounds = node.aggregate_bounds();
        Self {
            kind: node.kind.label().to_string(),
            name: node.name.clone(),
            x: bounds.origin.x.round() as i32,
            y: bounds.origin.y.round() as i32,
            width: bounds.size.x.round() as i32,
            height: bounds.size.y.round() as i32,
            fill: node.fill,
            stroke: node.stroke,
        }
    }
}

pub struct PropertyPanel {
    pub id: WidgetId,
    pub snapshot: NodeSnapshot,
    pub theme: Theme,
}

impl PropertyPanel {
    /// Conditional builder — returns `Some` only when the document
    /// has an active selection. Mirrors TS `{hasSelection && ...}`.
    pub fn for_selection(doc: &Document) -> Option<Self> {
        let node = doc.selected_node()?;
        Some(Self {
            id: WidgetId::new(2000),
            snapshot: NodeSnapshot::from_node(node),
            theme: Theme::dark(),
        })
    }
}

impl Widget for PropertyPanel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Vertical extent is "as much as you give me" — the host
        // clips at the rail rect. Reporting 800 here is just a
        // placeholder for the abstract widget tree.
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, 800.0),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, self.theme.card);
        cx.backend.fill_rect(
            Rect {
                origin: rect.origin,
                size: Point2D::new(1.0, rect.size.y),
            },
            self.theme.border,
        );

        let x = rect.origin.x;
        let w = rect.size.x;
        let mut y = rect.origin.y;
        y = sections::paint_tab_strip(cx, &self.theme, x, y, w);
        y = sections::paint_node_header(cx, &self.theme, &self.snapshot, x, y, w);
        y = sections::paint_create_component(cx, &self.theme, x, y, w);
        y = sections::paint_position_section(cx, &self.theme, &self.snapshot, x, y, w);
        y = sections::paint_flex_section(cx, &self.theme, x, y, w);
        y = sections::paint_size_section(cx, &self.theme, &self.snapshot, x, y, w);
        y = sections::paint_layer_section(cx, &self.theme, x, y, w);
        y = sections::paint_fill_section(cx, &self.theme, &self.snapshot, x, y, w);
        y = sections::paint_stroke_section(cx, &self.theme, &self.snapshot, x, y, w);
        y = sections::paint_effects_section(cx, &self.theme, x, y, w);
        let _ = sections::paint_export_section(cx, &self.theme, x, y, w);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(self.snapshot.kind.clone());
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::NodeId;

    #[test]
    fn for_selection_with_real_node_builds_snapshot() {
        let doc = Document::sample();
        let panel = PropertyPanel::for_selection(&doc).expect("sample doc has a selection");
        assert_eq!(panel.snapshot.kind, "Text");
        assert_eq!(panel.snapshot.name, "Title");
        // Title node bounds: (60, 60, 240, 28).
        assert_eq!(panel.snapshot.x, 60);
        assert_eq!(panel.snapshot.y, 60);
        assert_eq!(panel.snapshot.width, 240);
        assert_eq!(panel.snapshot.height, 28);
    }

    #[test]
    fn for_selection_without_selection_returns_none() {
        let doc = Document::empty();
        assert!(PropertyPanel::for_selection(&doc).is_none());
    }

    #[test]
    fn for_selection_with_stale_selection_returns_none() {
        let mut doc = Document::sample();
        doc.selected = NodeId::new(9999);
        assert!(PropertyPanel::for_selection(&doc).is_none());
    }

    #[test]
    fn access_node_advertises_group_with_kind_label() {
        let doc = Document::sample();
        let panel = PropertyPanel::for_selection(&doc).unwrap();
        let node = panel.access_node();
        assert_eq!(node.role(), accesskit::Role::Group);
        assert_eq!(node.label(), Some("Text"));
    }

    #[test]
    fn group_snapshot_aggregates_child_bounds() {
        // Codex Step 6 stop-hook fix: a Group has bounds = ZERO,
        // so `from_node` must derive W/H from children — else
        // the panel shows "0 × 0" for any container.
        let doc = Document::sample();
        // Select the "Button" group (id 12). Its children:
        //   - Button background rect (60, 130, 180, 36)
        //   - Click me text       (76, 152, 160, 16)
        // Aggregate bounds: (60, 130, 240-60=180, 168-130=38).
        let mut doc = doc;
        doc.selected = NodeId::new(12);
        let panel = PropertyPanel::for_selection(&doc).unwrap();
        assert_eq!(panel.snapshot.kind, "Group");
        assert_eq!(panel.snapshot.x, 60);
        assert_eq!(panel.snapshot.y, 130);
        assert!(panel.snapshot.width > 0);
        assert!(panel.snapshot.height > 0);
    }

    #[test]
    fn format_color_hex_pads_to_six_chars() {
        use sections::format_color_hex;
        assert_eq!(format_color_hex(Color::WHITE), "#FFFFFF");
        assert_eq!(format_color_hex(Color::BLACK), "#000000");
        assert_eq!(format_color_hex(Color::RED), "#FF0000");
    }
}

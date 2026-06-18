//! Platform-free accessibility-tree assembler (#67).
//!
//! Every [`Widget`](crate::widgets::Widget) already exposes an
//! `accesskit::Node` via `Widget::access_node()`, but nothing assembled
//! those per-widget nodes into a single `accesskit::TreeUpdate` for an
//! OS accessibility adapter to consume — so the whole editor read as one
//! opaque canvas to VoiceOver / Narrator / Orca.
//!
//! This module is that missing seam. It is deliberately platform-free
//! (no winit / no skia / no platform adapter): it depends only on
//! `accesskit` + the widget facade, so it stays wasm32-clean and is
//! shared verbatim by the native + web hosts. Hosts collect the same
//! ordered set of top-level widgets they paint, pair each with the
//! screen-space rect they placed it at, and hand the slice here.
//!
//! ## Why the host supplies bounds
//!
//! `Widget::access_node()` sets a node's `role` + `label` but NOT its
//! bounds — a widget paints relative to a host-provided rect and has no
//! standing knowledge of where the host placed it. So the assembler
//! takes a `(rect, node)` pairing per widget and injects the rect as the
//! node's `bounds`. The host is the single source of truth for layout
//! (it reuses the very same enumeration its paint pass walks), so the
//! a11y tree and the painted frame never drift.
//!
//! ## NodeId convention
//!
//! `accesskit::NodeId(widget_id.0)` — the documented mapping on
//! [`WidgetId`](crate::widgets::WidgetId). The root host frame uses
//! [`ROOT_WIDGET_ID`](crate::widgets::ROOT_WIDGET_ID) (`WidgetId(0)`).

use crate::widgets::{Widget, WidgetId, ROOT_WIDGET_ID};
use crate::Rect;

/// A top-level widget placed at a host-resolved screen rect.
///
/// Hosts build one of these per region they paint (top bar, toolbar,
/// layer panel, canvas, property panel, chat, status bar, plus any open
/// overlays). The assembler reads `widget.id()` + `widget.access_node()`
/// and stamps `bounds` from `rect`.
pub struct PlacedWidget<'a> {
    /// The widget — read for its stable id + its `access_node()`.
    pub widget: &'a dyn Widget,
    /// Screen-space rectangle the host painted the widget into.
    pub bounds: Rect,
}

impl<'a> PlacedWidget<'a> {
    /// Convenience constructor.
    pub fn new(widget: &'a dyn Widget, bounds: Rect) -> Self {
        Self { widget, bounds }
    }
}

/// Map a `WidgetId` to its `accesskit::NodeId` per the documented
/// convention (`NodeId(WidgetId.0)`).
#[inline]
pub fn node_id(id: WidgetId) -> accesskit::NodeId {
    accesskit::NodeId(id.0)
}

/// Convert a facade [`Rect`] (origin + size, `f32`) into an
/// `accesskit::Rect` (min/max corners, `f64`). AccessKit bounds are in
/// the same screen-space coordinate system the host paints in.
#[inline]
fn to_accesskit_rect(r: Rect) -> accesskit::Rect {
    let x0 = r.origin.x as f64;
    let y0 = r.origin.y as f64;
    accesskit::Rect {
        x0,
        y0,
        x1: x0 + r.size.x as f64,
        y1: y0 + r.size.y as f64,
    }
}

/// Build the root host node — a `Window`-roled frame whose children are
/// the supplied widget node ids, sized to the window bounds.
fn build_root(window_bounds: Rect, children: &[accesskit::NodeId]) -> accesskit::Node {
    let mut root = accesskit::Node::new(accesskit::Role::Window);
    root.set_label("OpenPencil");
    root.set_bounds(to_accesskit_rect(window_bounds));
    root.set_children(children.to_vec());
    root
}

/// Assemble a complete `accesskit::TreeUpdate` for the editor.
///
/// * `window_bounds` — the host window's screen rect (root node bounds).
/// * `widgets` — the ordered top-level widgets the host painted, each
///   paired with its placement rect. Order is preserved as the root's
///   child order (i.e. reading order for the screen reader).
/// * `focus` — the `WidgetId` that should hold keyboard focus. If it is
///   not present among `widgets` (or is the root), focus falls back to
///   the root node so the update is always self-consistent (accesskit
///   requires `focus` to name a node that exists in the tree).
///
/// The returned update carries the full tree (`tree: Some(..)`) so it is
/// valid as an initial publish; hosts may also re-send it on every dirty
/// frame (accesskit suppresses no-op events).
pub fn assemble_tree_update(
    window_bounds: Rect,
    widgets: &[PlacedWidget<'_>],
    focus: WidgetId,
) -> accesskit::TreeUpdate {
    let root_id = node_id(ROOT_WIDGET_ID);

    // Child id list (preserves host paint / reading order).
    let child_ids: Vec<accesskit::NodeId> =
        widgets.iter().map(|w| node_id(w.widget.id())).collect();

    let mut nodes: Vec<(accesskit::NodeId, accesskit::Node)> =
        Vec::with_capacity(widgets.len() + 1);
    nodes.push((root_id, build_root(window_bounds, &child_ids)));

    for placed in widgets {
        let id = node_id(placed.widget.id());
        let mut node = placed.widget.access_node();
        // The widget set role + label; the host owns layout, so the
        // bounds come from where it was painted.
        node.set_bounds(to_accesskit_rect(placed.bounds));
        nodes.push((id, node));
    }

    // Resolve focus: it must name a real node in this update. A focus
    // request for a widget the host did not include this frame (e.g. a
    // closed overlay) degrades to the root rather than producing an
    // invalid update.
    let focus_id = node_id(focus);
    let focus_present =
        focus != ROOT_WIDGET_ID && widgets.iter().any(|w| node_id(w.widget.id()) == focus_id);
    let focus = if focus_present { focus_id } else { root_id };

    accesskit::TreeUpdate {
        nodes,
        tree: Some(accesskit::Tree::new(root_id)),
        tree_id: accesskit::TreeId::ROOT,
        focus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::{LayoutBox, LayoutCx, PaintCx};
    use crate::Rect;

    /// Minimal fake widget so the assembler tests don't depend on any
    /// real widget's construction surface.
    struct FakeWidget {
        id: WidgetId,
        role: accesskit::Role,
        label: &'static str,
    }

    impl Widget for FakeWidget {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
            LayoutBox {
                rect: crate::widgets::rect(0.0, 0.0, 1.0, 1.0),
            }
        }
        fn paint(&self, _cx: &mut PaintCx<'_>, _rect: Rect) {}
        fn access_node(&self) -> accesskit::Node {
            let mut node = accesskit::Node::new(self.role);
            node.set_label(self.label);
            node
        }
    }

    fn placed(
        id: u64,
        role: accesskit::Role,
        label: &'static str,
        bounds: Rect,
    ) -> (FakeWidget, Rect) {
        (
            FakeWidget {
                id: WidgetId::new(id),
                role,
                label,
            },
            bounds,
        )
    }

    fn window() -> Rect {
        crate::widgets::rect(0.0, 0.0, 1280.0, 800.0)
    }

    #[test]
    fn root_node_is_a_window_with_window_bounds() {
        let update = assemble_tree_update(window(), &[], ROOT_WIDGET_ID);
        let (root_id, root) = &update.nodes[0];
        assert_eq!(*root_id, node_id(ROOT_WIDGET_ID));
        assert_eq!(root.role(), accesskit::Role::Window);
        let b = root.bounds().expect("root has bounds");
        assert_eq!((b.x0, b.y0, b.x1, b.y1), (0.0, 0.0, 1280.0, 800.0));
    }

    #[test]
    fn child_count_matches_input_and_every_child_has_a_node() {
        let (top, top_r) = placed(5000, accesskit::Role::Header, "Title bar", window());
        let (tool, tool_r) = placed(
            3000,
            accesskit::Role::Toolbar,
            "Toolbar",
            crate::widgets::rect(8.0, 48.0, 44.0, 300.0),
        );
        let (canvas, canvas_r) = placed(
            4000,
            accesskit::Role::Canvas,
            "Canvas",
            crate::widgets::rect(0.0, 40.0, 1280.0, 760.0),
        );
        let widgets = [
            PlacedWidget::new(&top, top_r),
            PlacedWidget::new(&tool, tool_r),
            PlacedWidget::new(&canvas, canvas_r),
        ];
        let update = assemble_tree_update(window(), &widgets, WidgetId::new(4000));

        // Root + 3 children.
        assert_eq!(update.nodes.len(), 4);

        // Root child list matches input order and length.
        let (_, root) = &update.nodes[0];
        let child_ids: Vec<_> = root.children().to_vec();
        assert_eq!(
            child_ids,
            vec![
                node_id(WidgetId::new(5000)),
                node_id(WidgetId::new(3000)),
                node_id(WidgetId::new(4000))
            ]
        );

        // Every advertised child id maps to a real node in `nodes`.
        for child in &child_ids {
            assert!(
                update.nodes.iter().any(|(id, _)| id == child),
                "child {child:?} has no node entry"
            );
        }
    }

    #[test]
    fn each_child_node_carries_its_widget_role_label_and_host_bounds() {
        let bounds = crate::widgets::rect(10.0, 20.0, 30.0, 40.0);
        let (canvas, _) = placed(4000, accesskit::Role::Canvas, "Canvas", bounds);
        let widgets = [PlacedWidget::new(&canvas, bounds)];
        let update = assemble_tree_update(window(), &widgets, WidgetId::new(4000));

        let (_, node) = update
            .nodes
            .iter()
            .find(|(id, _)| *id == node_id(WidgetId::new(4000)))
            .expect("canvas node present");
        assert_eq!(node.role(), accesskit::Role::Canvas);
        assert_eq!(node.label(), Some("Canvas"));
        let b = node.bounds().expect("canvas has host bounds");
        assert_eq!((b.x0, b.y0, b.x1, b.y1), (10.0, 20.0, 40.0, 60.0));
    }

    #[test]
    fn focus_resolves_to_a_real_node_in_the_tree() {
        let (canvas, r) = placed(4000, accesskit::Role::Canvas, "Canvas", window());
        let widgets = [PlacedWidget::new(&canvas, r)];
        let update = assemble_tree_update(window(), &widgets, WidgetId::new(4000));
        assert_eq!(update.focus, node_id(WidgetId::new(4000)));
        // The focus id must be one of the emitted nodes.
        assert!(update.nodes.iter().any(|(id, _)| *id == update.focus));
    }

    #[test]
    fn focus_on_absent_widget_degrades_to_root() {
        let (canvas, r) = placed(4000, accesskit::Role::Canvas, "Canvas", window());
        let widgets = [PlacedWidget::new(&canvas, r)];
        // Ask focus for a widget that wasn't included this frame.
        let update = assemble_tree_update(window(), &widgets, WidgetId::new(7000));
        assert_eq!(update.focus, node_id(ROOT_WIDGET_ID));
        assert!(update.nodes.iter().any(|(id, _)| *id == update.focus));
    }

    #[test]
    fn tree_root_is_the_root_widget_id() {
        let update = assemble_tree_update(window(), &[], ROOT_WIDGET_ID);
        let tree = update.tree.expect("initial update carries tree");
        assert_eq!(tree.root, node_id(ROOT_WIDGET_ID));
        assert_eq!(update.tree_id, accesskit::TreeId::ROOT);
    }
}

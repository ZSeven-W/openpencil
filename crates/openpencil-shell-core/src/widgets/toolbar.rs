//! `Toolbar` — vertical icon-only tool column anchored to the left
//! edge of the canvas (Step 4 visual lift).
//!
//! Layout matches `apps/web/src/components/editor/toolbar.tsx`:
//! tools at the top (Select / Rect / Text / Frame / Hand), a hairline
//! separator, undo/redo, another separator, then panel toggles
//! (Code / Design system).
//!
//! Active tool gets a `theme.primary` filled rounded square + the
//! white foreground icon. Inactive items render the icon in
//! `theme.muted_foreground` with a transparent background.
//!
//! Click events are wired by the host in P6; the toolbar exposes
//! [`Toolbar::hit_test`] so a `(x, y)` mouse position resolves to
//! either a `Tool` change or an `Action` (Undo / Redo / TogglePanel).

use crate::document::{Document, Tool};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect};

/// Outer column width (matches the TS app's `w-12` toolbar).
pub const TOOLBAR_WIDTH: f32 = 44.0;
const BUTTON_SIZE: f32 = 32.0;
const BUTTON_RADIUS: f32 = 8.0;
const ICON_SIZE: f32 = 18.0;
const STROKE_W: f32 = 1.6;
const BUTTON_GAP: f32 = 4.0;
/// Extra vertical room reserved AFTER the shape slot so the
/// chevron-down affordance has space to sit below the button
/// without overlapping the next item (matches the TS layout).
const SHAPE_SLOT_BOTTOM_EXTRA: f32 = 10.0;
const SECTION_GAP: f32 = 12.0;
const PAD_TOP: f32 = 8.0;
const PAD_BOTTOM: f32 = 8.0;

/// Each entry in the toolbar — either a tool button (selectable),
/// an action button (one-shot), a separator that paints a
/// hairline, or the shape-tool dropdown slot (icon driven by
/// `Document.ui.shape_tool`, click toggles the picker).
#[derive(Debug, Clone, Copy)]
pub enum ToolbarItem {
    Tool(Tool, Icon),
    Action(ToolbarAction, Icon),
    Separator,
    /// Compound shape slot. Paints the icon for whichever shape
    /// variant the user last picked; click toggles the dropdown
    /// listing all shape options.
    ShapeSlot,
}

/// One-shot action a toolbar button can dispatch. Wired in P6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    Undo,
    Redo,
    ToggleCodePanel,
    ToggleDesignPanel,
}

/// Hit-test result for a mouse click inside the toolbar rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarHit {
    Tool(Tool),
    Action(ToolbarAction),
    /// User clicked the shape slot — host should toggle the
    /// shape-tool picker (`Document.ui.shape_picker_open`).
    ToggleShapePicker,
}

pub struct Toolbar {
    pub id: WidgetId,
    pub items: Vec<ToolbarItem>,
    pub active: Tool,
    pub theme: Theme,
    /// Which shape variant the shape slot paints. Read from
    /// `Document.ui.shape_tool` so the icon flips after the user
    /// picks a shape from the dropdown.
    pub shape_tool: Tool,
}

impl Toolbar {
    /// Default Step 4 set — TS app order:
    /// Select / Rect / Text / Frame / Hand · Undo / Redo · Code / Design.
    pub fn default_set() -> Self {
        Self::for_document(&Document::empty())
    }

    /// Build the toolbar bound to the document's active tool +
    /// theme. The active highlight reads `doc.tool`; theme reads
    /// `doc.theme()` so the toolbar flips with TopBar Sun click.
    pub fn for_document(doc: &Document) -> Self {
        Self {
            id: WidgetId::new(3000),
            items: vec![
                ToolbarItem::Tool(Tool::Select, Icon::Cursor),
                ToolbarItem::ShapeSlot,
                ToolbarItem::Tool(Tool::Text, Icon::Type),
                ToolbarItem::Tool(Tool::Frame, Icon::Frame),
                ToolbarItem::Tool(Tool::Hand, Icon::Hand),
                ToolbarItem::Separator,
                ToolbarItem::Action(ToolbarAction::Undo, Icon::Undo),
                ToolbarItem::Action(ToolbarAction::Redo, Icon::Redo),
                ToolbarItem::Separator,
                ToolbarItem::Action(ToolbarAction::ToggleCodePanel, Icon::Braces),
                ToolbarItem::Action(ToolbarAction::ToggleDesignPanel, Icon::BookOpen),
            ],
            active: doc.tool,
            theme: doc.theme(),
            shape_tool: doc.ui.shape_tool,
        }
    }

    /// Total intrinsic height = padding + each item's slot.
    fn intrinsic_height(&self) -> f32 {
        let mut h = PAD_TOP;
        let mut prev_was_item = false;
        for item in &self.items {
            match item {
                ToolbarItem::Separator => {
                    h += if prev_was_item { SECTION_GAP } else { 0.0 };
                    prev_was_item = false;
                }
                ToolbarItem::Tool(_, _) | ToolbarItem::Action(_, _) => {
                    if prev_was_item {
                        h += BUTTON_GAP;
                    }
                    h += BUTTON_SIZE;
                    prev_was_item = true;
                }
                ToolbarItem::ShapeSlot => {
                    if prev_was_item {
                        h += BUTTON_GAP;
                    }
                    h += BUTTON_SIZE + SHAPE_SLOT_BOTTOM_EXTRA;
                    prev_was_item = true;
                }
            }
        }
        h + PAD_BOTTOM
    }

    /// Returns the on-screen rect of the shape slot. Used by the
    /// host to anchor the shape-tool picker dropdown immediately
    /// to the right of this button. `None` if the toolbar wasn't
    /// built with a shape slot (e.g. test fixtures).
    pub fn shape_slot_rect(&self, rect: Rect) -> Option<Rect> {
        let button_x = rect.origin.x + (rect.size.x - BUTTON_SIZE) / 2.0;
        let mut y = rect.origin.y + PAD_TOP;
        let mut prev_was_item = false;
        for item in &self.items {
            match item {
                ToolbarItem::Separator => {
                    if prev_was_item {
                        y += SECTION_GAP;
                    }
                    prev_was_item = false;
                }
                ToolbarItem::ShapeSlot => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    return Some(Rect {
                        origin: Point2D::new(button_x, y),
                        size: Point2D::new(BUTTON_SIZE, BUTTON_SIZE),
                    });
                }
                _ => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    y += BUTTON_SIZE;
                    prev_was_item = true;
                }
            }
        }
        None
    }

    /// Resolve a pointer at `point` (host-coordinates, top-left of
    /// the toolbar rect at `rect.origin`) to a tool / action.
    /// Returns `None` for a click outside any button or on a
    /// separator gap.
    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<ToolbarHit> {
        if point.x < rect.origin.x
            || point.x > rect.origin.x + rect.size.x
            || point.y < rect.origin.y
            || point.y > rect.origin.y + rect.size.y
        {
            return None;
        }
        let button_x = rect.origin.x + (rect.size.x - BUTTON_SIZE) / 2.0;
        let mut y = rect.origin.y + PAD_TOP;
        let mut prev_was_item = false;
        for item in &self.items {
            match item {
                ToolbarItem::Separator => {
                    if prev_was_item {
                        y += SECTION_GAP;
                    }
                    prev_was_item = false;
                }
                ToolbarItem::Tool(tool, _) => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    let button_rect = Rect {
                        origin: Point2D::new(button_x, y),
                        size: Point2D::new(BUTTON_SIZE, BUTTON_SIZE),
                    };
                    if hit(button_rect, point) {
                        return Some(ToolbarHit::Tool(*tool));
                    }
                    y += BUTTON_SIZE;
                    prev_was_item = true;
                }
                ToolbarItem::Action(action, _) => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    let button_rect = Rect {
                        origin: Point2D::new(button_x, y),
                        size: Point2D::new(BUTTON_SIZE, BUTTON_SIZE),
                    };
                    if hit(button_rect, point) {
                        return Some(ToolbarHit::Action(*action));
                    }
                    y += BUTTON_SIZE;
                    prev_was_item = true;
                }
                ToolbarItem::ShapeSlot => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    // Hit area covers the button + its chevron
                    // gutter so a click on the chevron itself
                    // also opens the picker.
                    let button_rect = Rect {
                        origin: Point2D::new(button_x, y),
                        size: Point2D::new(BUTTON_SIZE, BUTTON_SIZE + SHAPE_SLOT_BOTTOM_EXTRA),
                    };
                    if hit(button_rect, point) {
                        return Some(ToolbarHit::ToggleShapePicker);
                    }
                    y += BUTTON_SIZE + SHAPE_SLOT_BOTTOM_EXTRA;
                    prev_was_item = true;
                }
            }
        }
        None
    }
}

/// Lucide icon for a shape variant. Used by the toolbar shape
/// slot AND the dropdown rows so both stay visually aligned.
pub fn icon_for_shape(tool: Tool) -> Icon {
    match tool {
        Tool::Rect => Icon::Square,
        Tool::Ellipse => Icon::Circle,
        Tool::Polygon => Icon::Triangle,
        Tool::Line => Icon::Minus,
        Tool::Pen => Icon::PenTool,
        _ => Icon::Square,
    }
}

fn hit(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

impl Widget for Toolbar {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(TOOLBAR_WIDTH, self.intrinsic_height()),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        // Floating column on top of the canvas — paint a translucent
        // popover-ish background so it reads as a panel against any
        // canvas content beneath.
        cx.backend.fill_round_rect(rect, 12.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(rect, 12.0, self.theme.border, 1.0);

        let button_x = rect.origin.x + (rect.size.x - BUTTON_SIZE) / 2.0;
        let mut y = rect.origin.y + PAD_TOP;
        let mut prev_was_item = false;
        for item in &self.items {
            match item {
                ToolbarItem::Separator => {
                    if prev_was_item {
                        y += SECTION_GAP / 2.0;
                    }
                    let sep_x = rect.origin.x + 10.0;
                    let sep_w = rect.size.x - 20.0;
                    cx.backend.fill_rect(
                        Rect {
                            origin: Point2D::new(sep_x, y),
                            size: Point2D::new(sep_w, 1.0),
                        },
                        self.theme.border,
                    );
                    y += SECTION_GAP / 2.0;
                    prev_was_item = false;
                }
                ToolbarItem::Tool(tool, icon) => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    let active = *tool == self.active;
                    paint_button(cx, &self.theme, button_x, y, *icon, active);
                    y += BUTTON_SIZE;
                    prev_was_item = true;
                }
                ToolbarItem::Action(_, icon) => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    paint_button(cx, &self.theme, button_x, y, *icon, false);
                    y += BUTTON_SIZE;
                    prev_was_item = true;
                }
                ToolbarItem::ShapeSlot => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    let active = self.active.is_shape();
                    paint_button(
                        cx,
                        &self.theme,
                        button_x,
                        y,
                        icon_for_shape(self.shape_tool),
                        active,
                    );
                    // Chevron-down sits just BELOW the button,
                    // horizontally centered — matches the TS
                    // shape-tool-dropdown affordance (caret in the
                    // gutter, not overlapping the icon).
                    let chev_size = 10.0;
                    draw_icon(
                        cx.backend,
                        Icon::ChevronDown,
                        Point2D::new(button_x + (BUTTON_SIZE - chev_size) / 2.0, y + BUTTON_SIZE),
                        chev_size,
                        self.theme.muted_foreground,
                        1.4,
                    );
                    y += BUTTON_SIZE + SHAPE_SLOT_BOTTOM_EXTRA;
                    prev_was_item = true;
                }
            }
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Toolbar);
        node.set_label("Toolbar");
        node
    }
}

fn paint_button(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32, icon: Icon, active: bool) {
    let button_rect = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(BUTTON_SIZE, BUTTON_SIZE),
    };
    let icon_color: Color;
    if active {
        cx.backend
            .fill_round_rect(button_rect, BUTTON_RADIUS, theme.primary);
        icon_color = theme.primary_foreground;
    } else {
        icon_color = theme.muted_foreground;
    }
    let icon_origin = Point2D::new(
        x + (BUTTON_SIZE - ICON_SIZE) / 2.0,
        y + (BUTTON_SIZE - ICON_SIZE) / 2.0,
    );
    draw_icon(
        cx.backend,
        icon,
        icon_origin,
        ICON_SIZE,
        icon_color,
        STROKE_W,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_set_has_four_tools_plus_shape_slot_plus_actions() {
        let toolbar = Toolbar::default_set();
        let tool_count = toolbar
            .items
            .iter()
            .filter(|i| matches!(i, ToolbarItem::Tool(..)))
            .count();
        let action_count = toolbar
            .items
            .iter()
            .filter(|i| matches!(i, ToolbarItem::Action(..)))
            .count();
        let shape_slot_count = toolbar
            .items
            .iter()
            .filter(|i| matches!(i, ToolbarItem::ShapeSlot))
            .count();
        // Select / Text / Frame / Hand are direct tool buttons;
        // Rect / Ellipse / Polygon / Line / Pen live behind the
        // single ShapeSlot dropdown.
        assert_eq!(tool_count, 4);
        assert_eq!(shape_slot_count, 1);
        assert_eq!(action_count, 4);
        assert_eq!(toolbar.active, Tool::Select);
    }

    #[test]
    fn intrinsic_height_accommodates_all_items() {
        let toolbar = Toolbar::default_set();
        let h = toolbar.intrinsic_height();
        // 5 tool buttons + 4 action buttons = 9 buttons; their total
        // is at minimum 9 * BUTTON_SIZE = 324, plus padding +
        // separator gaps. Sanity-check the height is in a
        // reasonable band.
        assert!(h > 9.0 * BUTTON_SIZE, "toolbar shorter than its buttons");
        assert!(h < 9.0 * BUTTON_SIZE + 200.0, "toolbar bloated: {h}");
    }

    #[test]
    fn for_document_picks_up_active_tool() {
        let mut doc = Document::sample();
        doc.tool = Tool::Frame;
        let toolbar = Toolbar::for_document(&doc);
        assert_eq!(toolbar.active, Tool::Frame);
    }

    #[test]
    fn hit_test_inside_first_button_returns_select() {
        let toolbar = Toolbar::default_set();
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar.intrinsic_height()),
        };
        // Center of the first button.
        let center = Point2D::new((TOOLBAR_WIDTH) / 2.0, PAD_TOP + BUTTON_SIZE / 2.0);
        assert_eq!(
            toolbar.hit_test(rect, center),
            Some(ToolbarHit::Tool(Tool::Select))
        );
    }

    #[test]
    fn hit_test_outside_returns_none() {
        let toolbar = Toolbar::default_set();
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar.intrinsic_height()),
        };
        assert_eq!(toolbar.hit_test(rect, Point2D::new(-10.0, -10.0)), None);
        assert_eq!(toolbar.hit_test(rect, Point2D::new(1000.0, 1000.0)), None);
    }

    #[test]
    fn hit_test_resolves_action_button() {
        let toolbar = Toolbar::default_set();
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar.intrinsic_height()),
        };
        // First Action item is Undo. After 5 tool buttons + 4 button
        // gaps + 1 separator + button advance, hit-test the center
        // of the Undo button.
        let undo_y =
            PAD_TOP + 5.0 * BUTTON_SIZE + 4.0 * BUTTON_GAP + SECTION_GAP + BUTTON_SIZE / 2.0;
        let center = Point2D::new(TOOLBAR_WIDTH / 2.0, undo_y);
        assert_eq!(
            toolbar.hit_test(rect, center),
            Some(ToolbarHit::Action(ToolbarAction::Undo))
        );
    }

    #[test]
    fn access_node_advertises_toolbar_role() {
        let toolbar = Toolbar::default_set();
        let node = toolbar.access_node();
        assert_eq!(node.role(), accesskit::Role::Toolbar);
        assert_eq!(node.label(), Some("Toolbar"));
    }
}

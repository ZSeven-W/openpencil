//! `Toolbar` — vertical icon-only tool column anchored to the left
//! edge of the canvas (Step 4 visual lift).
//!
//! Layout matches `apps/web/src/components/editor/toolbar.tsx`:
//! tools at the top (Select / Rect / Text / Frame / Hand), a hairline
//! separator, undo/redo, another separator, then panel toggles
//! (Variables / Design system).
//!
//! Active tool gets a `theme.primary` filled rounded square + the
//! white foreground icon. Inactive items render the icon in
//! `theme.muted_foreground` with a transparent background.
//!
//! Click events are wired by the host in P6; the toolbar exposes
//! [`Toolbar::hit_test`] so a `(x, y)` mouse position resolves to
//! either a `Tool` change or an `Action` (Undo / Redo / TogglePanel).

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use op_editor_core::EditorState;
use op_editor_core::Tool;

/// Outer column width (matches the TS app's `w-12` toolbar).
pub const TOOLBAR_WIDTH: f32 = 44.0;
const BUTTON_SIZE: f32 = 32.0;
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
    ToggleVariablesPanel,
    ToggleDesignPanel,
}

/// Hit-test result for a mouse click inside the toolbar rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarHit {
    Tool(Tool),
    Action(ToolbarAction),
    /// User clicked the shape slot — host should toggle the
    /// shape-tool picker (`Document.ui.shape_picker.open`).
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
    /// Which item the cursor is over — drives the per-button hover
    /// wash. `None` = no hover (cursor off the bar or over an
    /// active item where the active fill already reads).
    pub hover: Option<op_editor_core::ToolbarHover>,
    pub pressed: Option<op_editor_core::ToolbarHover>,
}

impl Toolbar {
    /// Default Step 4 set — TS app order:
    /// Select / Rect / Text / Frame / Hand · Undo / Redo · Variables / Design.
    pub fn default_set() -> Self {
        Self::for_editor(&EditorState::new())
    }

    /// Build the toolbar bound to the editor's active tool + theme.
    /// The active highlight reads `state.tool`; theme reads
    /// `state.editor_ui.theme_mode` so the toolbar flips with the
    /// TopBar Sun click.
    pub fn for_editor(state: &EditorState) -> Self {
        // Form-widget tools are intentionally NOT in the toolbar: widget
        // nodes are authored via the component kit (uikit) / AI+MCP and
        // matched by the jian runtime, not dropped as primitive tools.
        let items = vec![
            ToolbarItem::Tool(Tool::Select, Icon::Cursor),
            ToolbarItem::ShapeSlot,
            ToolbarItem::Tool(Tool::Text, Icon::Type),
            ToolbarItem::Tool(Tool::Frame, Icon::Frame),
            ToolbarItem::Tool(Tool::Hand, Icon::Hand),
            ToolbarItem::Separator,
            ToolbarItem::Action(ToolbarAction::Undo, Icon::Undo),
            ToolbarItem::Action(ToolbarAction::Redo, Icon::Redo),
            ToolbarItem::Separator,
            ToolbarItem::Action(ToolbarAction::ToggleVariablesPanel, Icon::Braces),
            ToolbarItem::Action(ToolbarAction::ToggleDesignPanel, Icon::BookOpen),
        ];
        Self {
            id: WidgetId::new(3000),
            items,
            active: state.tool,
            theme: theme_for(&state.editor_ui),
            shape_tool: state.editor_ui.shape_tool,
            hover: state.editor_ui.toolbar_hover,
            pressed: match state.editor_ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::Toolbar(button)) => Some(button),
                _ => None,
            },
        }
    }

    /// True when `hover` matches `item`. The active state takes
    /// visual precedence (active button paints the primary fill,
    /// not the hover wash), so an active+hovered item is
    /// considered not-hovered here.
    fn item_hovered(&self, item: &ToolbarItem) -> bool {
        use op_editor_core::ToolbarHover as H;
        let Some(hover) = self.hover else {
            return false;
        };
        match item {
            ToolbarItem::Tool(tool, _) => {
                matches!(hover, H::Tool(t) if t == *tool) && *tool != self.active
            }
            ToolbarItem::Action(action, _) => {
                use crate::widgets::editor_state_ext::toolbar_action;
                matches!(hover, H::Action(a) if a == toolbar_action(*action))
            }
            ToolbarItem::ShapeSlot => matches!(hover, H::ShapeSlot) && !self.active.is_shape(),
            ToolbarItem::Separator => false,
        }
    }

    fn item_pressed(&self, item: &ToolbarItem) -> bool {
        use op_editor_core::ToolbarHover as H;
        let Some(pressed) = self.pressed else {
            return false;
        };
        match item {
            ToolbarItem::Tool(tool, _) => {
                matches!(pressed, H::Tool(t) if t == *tool) && *tool != self.active
            }
            ToolbarItem::Action(action, _) => {
                use crate::widgets::editor_state_ext::toolbar_action;
                matches!(pressed, H::Action(a) if a == toolbar_action(*action))
            }
            ToolbarItem::ShapeSlot => matches!(pressed, H::ShapeSlot) && !self.active.is_shape(),
            ToolbarItem::Separator => false,
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
                    if button_rect.contains(point) {
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
                    if button_rect.contains(point) {
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
                    if button_rect.contains(point) {
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
                    let hovered = self.item_hovered(item);
                    let pressed = self.item_pressed(item);
                    paint_button(
                        cx,
                        &self.theme,
                        button_x,
                        y,
                        *icon,
                        active,
                        hovered,
                        pressed,
                    );
                    y += BUTTON_SIZE;
                    prev_was_item = true;
                }
                ToolbarItem::Action(_, icon) => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    let hovered = self.item_hovered(item);
                    let pressed = self.item_pressed(item);
                    paint_button(cx, &self.theme, button_x, y, *icon, false, hovered, pressed);
                    y += BUTTON_SIZE;
                    prev_was_item = true;
                }
                ToolbarItem::ShapeSlot => {
                    if prev_was_item {
                        y += BUTTON_GAP;
                    }
                    let active = self.active.is_shape();
                    let hovered = self.item_hovered(item);
                    let pressed = self.item_pressed(item);
                    paint_button(
                        cx,
                        &self.theme,
                        button_x,
                        y,
                        icon_for_shape(self.shape_tool),
                        active,
                        hovered,
                        pressed,
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

#[allow(clippy::too_many_arguments)]
fn paint_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    icon: Icon,
    active: bool,
    hovered: bool,
    pressed: bool,
) {
    let button_rect = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(BUTTON_SIZE, BUTTON_SIZE),
    };
    crate::widgets::button::paint_icon_button(
        cx.backend,
        theme,
        button_rect,
        icon,
        hovered,
        pressed,
        active,
        ICON_SIZE,
        STROKE_W,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_set_has_base_tools_shape_slot_and_actions_no_widgets() {
        let toolbar = Toolbar::default_set();
        let tool_count = toolbar
            .items
            .iter()
            .filter(|i| matches!(i, ToolbarItem::Tool(..)))
            .count();
        let widget_tool_count = toolbar
            .items
            .iter()
            .filter(|i| matches!(i, ToolbarItem::Tool(t, _) if t.is_widget()))
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
        // Rect / Ellipse / Polygon / Line / Pen live behind the single
        // ShapeSlot dropdown. Form widgets are NOT toolbar tools — they
        // are authored via the component kit / AI+MCP, not primitive
        // drop tools.
        assert_eq!(tool_count, 4);
        assert_eq!(widget_tool_count, 0);
        assert_eq!(shape_slot_count, 1);
        assert_eq!(action_count, 4);
        assert_eq!(toolbar.active, Tool::Select);
    }

    #[test]
    fn intrinsic_height_accommodates_all_items() {
        let toolbar = Toolbar::default_set();
        let h = toolbar.intrinsic_height();
        // 4 direct tools + shape slot + 4 action buttons = 9 button
        // slots; total is at least 9 * BUTTON_SIZE plus padding + gaps.
        let buttons = 9.0;
        assert!(
            h > buttons * BUTTON_SIZE,
            "toolbar shorter than its buttons"
        );
        assert!(h < buttons * BUTTON_SIZE + 200.0, "toolbar bloated: {h}");
    }

    #[test]
    fn for_editor_picks_up_active_tool() {
        let mut state = EditorState::new();
        state.tool = op_editor_core::Tool::Frame;
        let toolbar = Toolbar::for_editor(&state);
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
    fn for_editor_picks_up_pressed_button() {
        let mut state = EditorState::new();
        state.editor_ui.pressed_button = Some(op_editor_core::ButtonPressTarget::Toolbar(
            op_editor_core::ToolbarHover::Action(op_editor_core::ToolbarAction::Undo),
        ));
        let toolbar = Toolbar::for_editor(&state);
        assert_eq!(
            toolbar.pressed,
            Some(op_editor_core::ToolbarHover::Action(
                op_editor_core::ToolbarAction::Undo
            ))
        );
    }

    #[test]
    fn hit_test_resolves_action_button() {
        let toolbar = Toolbar::default_set();
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar.intrinsic_height()),
        };
        // The Undo button sits below the (now longer) tool sections;
        // rather than re-derive its y by hand, scan every row down the
        // bar centre for the first Undo hit. Decouples the test from
        // the exact item layout.
        let cx = TOOLBAR_WIDTH / 2.0;
        let undo_hit = (0..(toolbar.intrinsic_height() as i32)).find(|y| {
            toolbar.hit_test(rect, Point2D::new(cx, *y as f32))
                == Some(ToolbarHit::Action(ToolbarAction::Undo))
        });
        assert!(
            undo_hit.is_some(),
            "expected an Undo action button somewhere down the bar"
        );
    }

    #[test]
    fn no_widget_tools_in_toolbar() {
        // Widgets are authored via the component kit / AI+MCP, never as
        // toolbar drop-tools — so none appear in the bar.
        let toolbar = Toolbar::default_set();
        assert!(!toolbar
            .items
            .iter()
            .any(|i| matches!(i, ToolbarItem::Tool(t, _) if t.is_widget())));
    }

    #[test]
    fn access_node_advertises_toolbar_role() {
        let toolbar = Toolbar::default_set();
        let node = toolbar.access_node();
        assert_eq!(node.role(), accesskit::Role::Toolbar);
        assert_eq!(node.label(), Some("Toolbar"));
    }
}

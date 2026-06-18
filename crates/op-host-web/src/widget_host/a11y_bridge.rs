//! Host-side bridge for the hidden accessibility DOM mirror (#57).
//!
//! Two responsibilities, mirroring `op-host-native/src/widget_host/a11y.rs`:
//!
//! 1. **Enumerate** the editor's always-present regions (top bar, layer
//!    panel, canvas, property panel, toolbar, chat, status bar) at the
//!    rects the web paint pass (`widget_host/paint.rs`) places them, and
//!    hand them to the platform-free assembler
//!    (`op_editor_ui::accessibility`) to build an `accesskit::TreeUpdate`.
//!    The CanvasKit mount renders that update into the hidden DOM mirror
//!    (`crate::a11y_dom`) so screen readers can read the opaque canvas.
//! 2. **Route** incoming DOM accessibility events (focus / click on a
//!    mirror node) back into editor state — focusing the chat input,
//!    blurring it when focus moves to the canvas / a panel, or activating
//!    a tool. These keep the painted frame in lock-step with the screen
//!    reader's focus.
//!
//! The actual tree shape / NodeId mapping / focus resolution lives in the
//! assembler; this file is only the host-side enumeration + action
//! handlers (the `EditorState`-aware half that can't live in the
//! platform-free `a11y_dom` module).

use super::WidgetHost;
use op_editor_ui::accessibility::{assemble_tree_update, PlacedWidget};
use op_editor_ui::widgets::{
    AIChatPlaceholder, CanvasViewport, LayerPanel, LayoutCx, PropertyPanel, StatusBar, Toolbar,
    Widget, WidgetId, ROOT_WIDGET_ID, STATUS_BAR_HEIGHT, STATUS_BAR_WIDTH, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

use super::{STATUS_INSET, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};

// Stable widget ids of the always-present regions, mirrored from each
// widget's constructor (`WidgetId::new(..)` in op-editor-ui). Used for
// focus targeting + action routing without constructing the widget twice.
// Kept in sync with the native host's a11y constants.
const AI_CHAT_WIDGET_ID: u64 = 7000;
const PROPERTY_PANEL_WIDGET_ID: u64 = 2000;
const CANVAS_WIDGET_ID: u64 = 4000;
const TOOLBAR_WIDGET_ID: u64 = 3000;

impl WidgetHost {
    /// Assemble the accessibility tree for the current editor frame.
    ///
    /// Enumerates the SAME always-present regions the web paint pass
    /// composes (`widget_host/paint.rs`), pairs each with the rect it is
    /// painted at, and hands the ordered slice to the platform-free
    /// assembler. Order = screen-reader reading order. Transient overlays
    /// (pickers, modals, context menus) are intentionally omitted for v1.
    ///
    /// Takes `&mut self` because the canvas region reads the
    /// layout-resolved scene, which `refresh_layout_scene` lazily rebuilds
    /// when editor state is dirty — same contract as `paint`.
    pub(crate) fn accessibility_tree_update(
        &mut self,
        viewport_width: f32,
        viewport_height: f32,
    ) -> accesskit::TreeUpdate {
        // Keep the canvas scene in sync with editor state (cheap no-op when
        // not dirty) so the CanvasViewport widget matches what paint draws.
        self.refresh_layout_scene();
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;

        let window_bounds = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, viewport_height),
        };

        let ui = &self.editor_state.editor_ui;
        let dpi = 1.0; // Toolbar layout is dpi-independent (fixed metrics).

        // 1. TopBar — full-width top strip.
        let top_bar = self.top_bar();
        let top_bar_rect = self.top_bar_rect(viewport_width);

        // 2. LayerPanel — left rail, only when the sidebar is open.
        let layer_panel = LayerPanel::from_editor(&self.editor_state);
        let layer_panel_rect = self.layer_panel_rect(viewport_height);

        // 3. CanvasViewport — middle band (sidebar / right-rail aware).
        let (canvas_left, _canvas_y, canvas_w, canvas_h) =
            self.canvas_region(viewport_width, viewport_height);
        let canvas = CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
        let canvas_rect = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };

        // 4. PropertyPanel — right rail, only with a selection.
        let property_panel = PropertyPanel::for_selection_at(&self.editor_state, self.now_ms);
        let property_panel_width = ui.property_panel_width;
        let property_rect = Rect {
            origin: Point2D::new(viewport_width - property_panel_width, TOP_BAR_HEIGHT),
            size: Point2D::new(
                property_panel_width,
                (viewport_height - TOP_BAR_HEIGHT).max(0.0),
            ),
        };

        // 5. Toolbar — floating vertical column over the canvas.
        let toolbar = Toolbar::for_editor(&self.editor_state);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(
                canvas_left + TOOLBAR_INSET_X,
                TOP_BAR_HEIGHT + TOOLBAR_INSET_Y,
            ),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
        let toolbar_visible = canvas_w > TOOLBAR_WIDTH + TOOLBAR_INSET_X * 2.0;

        // 6. AIChatPlaceholder — floating chat panel.
        let chat = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms);
        let chat_rect = self.ai_chat_rect(viewport_width, viewport_height);

        // 7. StatusBar — floating bottom-right zoom pill (same geometry the
        //    web paint pass derives inline).
        let status = StatusBar::for_editor(&self.editor_state);
        let canvas_right = canvas_left + canvas_w;
        let status_rect = if canvas_w > STATUS_BAR_WIDTH + STATUS_INSET * 2.0 {
            Some(Rect {
                origin: Point2D::new(
                    canvas_right - STATUS_BAR_WIDTH - STATUS_INSET,
                    TOP_BAR_HEIGHT + canvas_h - STATUS_BAR_HEIGHT - STATUS_INSET,
                ),
                size: Point2D::new(STATUS_BAR_WIDTH, STATUS_BAR_HEIGHT),
            })
        } else {
            None
        };

        // Assemble the ordered, present set. Order = reading order.
        let mut placed: Vec<PlacedWidget<'_>> = Vec::with_capacity(8);
        placed.push(PlacedWidget::new(&top_bar, top_bar_rect));
        if ui.sidebar_open {
            placed.push(PlacedWidget::new(&layer_panel, layer_panel_rect));
        }
        if canvas_w > 0.0 && canvas_h > 0.0 {
            placed.push(PlacedWidget::new(&canvas, canvas_rect));
        }
        if let Some(panel) = property_panel.as_ref() {
            placed.push(PlacedWidget::new(panel, property_rect));
        }
        if toolbar_visible {
            placed.push(PlacedWidget::new(&toolbar, toolbar_rect));
        }
        if let Some(rect) = chat_rect {
            placed.push(PlacedWidget::new(&chat, rect));
        }
        if let Some(rect) = status_rect {
            placed.push(PlacedWidget::new(&status, rect));
        }

        let focus = self.accessibility_focus_target(canvas_w, canvas_h, property_panel.is_some());

        assemble_tree_update(window_bounds, &placed, focus)
    }

    /// Pick a sensible default focus target for the a11y tree.
    ///
    /// Order: focused chat input → property panel (when an editable
    /// selection is up) → canvas (the editor's primary work surface) →
    /// root. The chosen id must be a region actually present this frame,
    /// which the assembler re-checks before emitting.
    fn accessibility_focus_target(
        &self,
        canvas_w: f32,
        canvas_h: f32,
        property_panel_present: bool,
    ) -> WidgetId {
        if self.editor_state.chat.focused {
            return WidgetId::new(AI_CHAT_WIDGET_ID);
        }
        if property_panel_present && self.editor_state.ui.property_focus.is_some() {
            return WidgetId::new(PROPERTY_PANEL_WIDGET_ID);
        }
        if canvas_w > 0.0 && canvas_h > 0.0 {
            return WidgetId::new(CANVAS_WIDGET_ID);
        }
        ROOT_WIDGET_ID
    }

    /// Route an accessibility action targeting a known editor region back
    /// into host state. Returns `true` when the action changed state (so
    /// the mount repaints + re-publishes the tree). Mirrors the native
    /// host's `apply_a11y_action`.
    ///
    /// `target` is the raw `accesskit::NodeId.0` (== `WidgetId.0`), and
    /// `is_focus` distinguishes a `focus` event from a `click` activation.
    pub(crate) fn apply_a11y_action(&mut self, target: u64, is_focus: bool) -> bool {
        match target {
            // AIChat panel — focus or click both focus + ready the chat
            // input (mirrors `click.rs` `AIChatHit::FocusInput`).
            AI_CHAT_WIDGET_ID => {
                self.a11y_focus_chat_input();
                true
            }
            // Canvas / Toolbar / Property panel — moving a11y focus off the
            // chat blurs the chat input so caret + send routing follow the
            // screen reader's focus. Only meaningful when the chat holds it.
            CANVAS_WIDGET_ID | TOOLBAR_WIDGET_ID | PROPERTY_PANEL_WIDGET_ID if is_focus => {
                if self.editor_state.chat.focused {
                    self.editor_state.chat.focused = false;
                    self.mark_dirty();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Activate a tool from the hidden a11y toolbar — mirrors the painted
    /// toolbar's `ToolbarHit::Tool` arm (tool write + shape-picker close).
    /// Retained for a future per-tool mirror node set; the v1 mirror only
    /// surfaces the toolbar as a single region (so this has no caller yet —
    /// it is parity surface mirrored from the native host + exercised by the
    /// tests below).
    #[allow(dead_code)]
    pub(crate) fn a11y_set_tool(&mut self, tool: op_editor_core::Tool) {
        self.editor_state.tool = tool;
        self.editor_state.editor_ui.shape_picker.open = false;
        self.editor_state.editor_ui.shape_picker.hover = None;
        self.editor_state.editor_ui.shape_picker.pressed = None;
        self.mark_dirty();
    }

    /// Focus the chat input from the hidden a11y mirror — mirrors
    /// `widget_host/click.rs` `AIChatHit::FocusInput` (focus + clear stale
    /// selections), plus the caret-blink anchor reset so the painted caret
    /// restarts its phase like a real click. Callers should `set_now_ms`
    /// first so the anchor is current.
    pub(crate) fn a11y_focus_chat_input(&mut self) {
        self.editor_state.chat.focus_input_at_end(self.now_ms);
        self.editor_state.chat.transcript_selection = None;
        self.mark_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::accessibility::node_id;

    fn host() -> WidgetHost {
        WidgetHost::new()
    }

    #[test]
    fn tree_includes_always_present_regions() {
        let mut h = host();
        let update = h.accessibility_tree_update(1280.0, 800.0);
        let ids: Vec<_> = update.nodes.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&node_id(ROOT_WIDGET_ID)));
        assert!(ids.contains(&node_id(WidgetId::new(5000))), "top bar");
        assert!(
            ids.contains(&node_id(WidgetId::new(CANVAS_WIDGET_ID))),
            "canvas"
        );
        assert!(
            ids.contains(&node_id(WidgetId::new(AI_CHAT_WIDGET_ID))),
            "chat"
        );
        // Root advertises every emitted child.
        let (_, root) = &update.nodes[0];
        for child in root.children() {
            assert!(
                update.nodes.iter().any(|(id, _)| id == child),
                "root child {child:?} missing a node"
            );
        }
    }

    #[test]
    fn focus_defaults_to_canvas() {
        let mut h = host();
        let update = h.accessibility_tree_update(1280.0, 800.0);
        assert_eq!(update.focus, node_id(WidgetId::new(CANVAS_WIDGET_ID)));
    }

    #[test]
    fn focused_chat_input_takes_focus() {
        let mut h = host();
        h.editor_state.chat.focused = true;
        let update = h.accessibility_tree_update(1280.0, 800.0);
        assert_eq!(update.focus, node_id(WidgetId::new(AI_CHAT_WIDGET_ID)));
    }

    #[test]
    fn a11y_action_on_chat_focuses_input() {
        let mut host = host();
        host.set_now_ms(1234);
        let changed = host.apply_a11y_action(AI_CHAT_WIDGET_ID, true);
        assert!(changed);
        assert!(host.editor_state.chat.focused);
    }

    #[test]
    fn a11y_focus_on_canvas_blurs_chat() {
        let mut host = host();
        host.editor_state.chat.focused = true;
        let changed = host.apply_a11y_action(CANVAS_WIDGET_ID, true);
        assert!(changed);
        assert!(!host.editor_state.chat.focused);
    }

    #[test]
    fn a11y_action_on_unknown_region_is_noop() {
        let mut host = host();
        assert!(!host.apply_a11y_action(99999, true));
    }

    #[test]
    fn a11y_set_tool_switches_tool_and_closes_shape_picker() {
        let mut host = host();
        host.editor_state.editor_ui.shape_picker.open = true;
        host.a11y_set_tool(op_editor_core::Tool::Frame);
        assert_eq!(host.editor_state.tool, op_editor_core::Tool::Frame);
        assert!(!host.editor_state.editor_ui.shape_picker.open);
        assert!(host.editor_state_dirty);
    }

    #[test]
    fn a11y_focus_chat_input_focuses_and_clears_selections() {
        let mut host = host();
        host.set_now_ms(1234);
        host.editor_state.chat.set_input_text("hello");
        host.editor_state.chat.select_all_input(0);
        host.a11y_focus_chat_input();
        let chat = &host.editor_state.chat;
        assert!(chat.focused);
        assert!(chat.input.highlight_range().is_none());
        assert!(chat.transcript_selection.is_none());
        assert_eq!(chat.input.next_blink_flip_ms(1234), 1734);
    }

    #[test]
    fn collapsed_sidebar_drops_layer_panel_region() {
        let mut h = host();
        h.editor_state.editor_ui.sidebar_open = false;
        let update = h.accessibility_tree_update(1280.0, 800.0);
        let ids: Vec<_> = update.nodes.iter().map(|(id, _)| *id).collect();
        assert!(
            !ids.contains(&node_id(WidgetId::new(1000))),
            "layer panel hidden"
        );
    }
}

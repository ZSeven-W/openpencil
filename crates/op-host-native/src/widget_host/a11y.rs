//! Accessibility region tree for the native host (#67).
//!
//! Builds an `accesskit::TreeUpdate` over the SAME top-level widgets the
//! paint pass (`widget_host/paint.rs`) composes, so a screen reader sees
//! the editor's always-present regions — top bar, layer panel, toolbar,
//! canvas, property panel, chat, status bar — plus any cheap open
//! overlays, each at the rect the host painted it at.
//!
//! The actual tree shape / node-id mapping / focus resolution lives in
//! the platform-free assembler `op_editor_ui::accessibility`; this file
//! is only the host-side enumeration that pairs each widget with its
//! placement rect. Keeping the geometry here (reusing the same
//! `canvas_region` / `*_rect` helpers paint uses) means the a11y tree and
//! the painted frame never drift.

use super::helpers::{TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::WidgetHostNative;
use op_editor_ui::accessibility::{assemble_tree_update, PlacedWidget};
use op_editor_ui::widgets::{
    AIChatPlaceholder, CanvasViewport, LayerPanel, LayoutCx, PropertyPanel, StatusBar, Toolbar,
    TopBar, Widget, WidgetId, ROOT_WIDGET_ID, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// Assemble the accessibility tree for the current editor frame.
    ///
    /// Hosts call this on the same cadence they paint (initial publish +
    /// every dirty frame); the assembler suppresses no-op events on the
    /// adapter side. The widget set mirrors `paint.rs`'s always-present
    /// regions; transient overlays (pickers, modals, context menus) are
    /// intentionally omitted for v1 — they come and go every frame and
    /// their `access_node()`s are not yet richly labelled, so adding them
    /// would churn the tree without improving navigability.
    ///
    /// Takes `&mut self` because the canvas region reads the
    /// layout-resolved scene, which `refresh_layout_scene` lazily rebuilds
    /// when the editor state is dirty — same contract as `paint`.
    pub fn accessibility_tree_update(
        &mut self,
        viewport_width: f32,
        viewport_height: f32,
    ) -> accesskit::TreeUpdate {
        // Keep the canvas scene in sync with editor state (cheap no-op
        // when not dirty) so the CanvasViewport widget is consistent
        // with what paint draws.
        self.refresh_layout_scene();

        let window_bounds = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, viewport_height),
        };

        let ui = &self.editor_state.editor_ui;
        let dpi = self.dpi_scale_hint();

        // 1. TopBar — full-width top strip.
        let top_bar = TopBar::for_editor_ui(ui);
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };

        // 2. LayerPanel — left rail, only when the sidebar is open.
        let layer_panel = LayerPanel::from_editor(&self.editor_state);
        let layer_panel_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                ui.layer_panel_width,
                (viewport_height - TOP_BAR_HEIGHT).max(0.0),
            ),
        };

        // 3. CanvasViewport — middle band (sidebar/right-rail aware).
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

        // 7. StatusBar — floating bottom-right zoom pill.
        let status = StatusBar::for_editor(&self.editor_state);
        let status_rect = self.status_bar_rect(viewport_width, viewport_height);

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
    /// top bar → root. The chosen id must be a region actually present
    /// this frame, which the assembler re-checks before emitting.
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

    /// DPI scale used for the toolbar layout pass. The toolbar layout is
    /// dpi-independent (fixed button metrics), so a 1.0 fallback is
    /// exact; the real value is only threaded for parity with paint.
    fn dpi_scale_hint(&self) -> f32 {
        1.0
    }

    /// Route an accesskit action targeting a known editor region back
    /// into host state. Returns `true` when the action changed state (so
    /// the runner repaints + re-publishes the tree). Mirrors the web
    /// `a11y_bridge` action handlers.
    ///
    /// `target` is the raw `accesskit::NodeId.0` (== `WidgetId.0`), and
    /// `is_focus` distinguishes a `Focus` request from a `Click` /
    /// `Default` activation. v1 handles the two regions the web bridge
    /// covered — focusing the chat input and activating it — plus
    /// blurring the chat when focus moves to the canvas / a panel.
    pub fn apply_a11y_action(&mut self, target: u64, is_focus: bool) -> bool {
        match target {
            // AIChat panel — Focus or Click/Default both focus + ready
            // the chat input (TS click.rs `AIChatHit::FocusInput`).
            AI_CHAT_WIDGET_ID => {
                let now = self.now_ms;
                self.editor_state.chat.focus_input_at_end(now);
                self.editor_state.chat.transcript_selection = None;
                self.mark_editor_state_dirty();
                true
            }
            // Canvas / Toolbar / Property panel — moving a11y focus off
            // the chat blurs the chat input so caret + send routing
            // follow the screen reader's focus. Only meaningful when the
            // chat currently holds focus.
            CANVAS_WIDGET_ID | TOOLBAR_WIDGET_ID | PROPERTY_PANEL_WIDGET_ID if is_focus => {
                if self.editor_state.chat.focused {
                    self.editor_state.chat.focused = false;
                    self.mark_editor_state_dirty();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

// Stable widget ids of the always-present regions, mirrored from each
// widget's constructor (`WidgetId::new(..)`). Used for focus targeting
// without constructing the widget twice.
const AI_CHAT_WIDGET_ID: u64 = 7000;
const PROPERTY_PANEL_WIDGET_ID: u64 = 2000;
const CANVAS_WIDGET_ID: u64 = 4000;
const TOOLBAR_WIDGET_ID: u64 = 3000;

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::accessibility::node_id;

    fn host() -> WidgetHostNative {
        WidgetHostNative::new()
    }

    #[test]
    fn tree_includes_always_present_regions() {
        let mut h = host();
        let update = h.accessibility_tree_update(1280.0, 800.0);
        // Root + at least: top bar, layer panel, canvas, toolbar,
        // chat, status bar (property panel only with a selection).
        let ids: Vec<_> = update.nodes.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&node_id(ROOT_WIDGET_ID)));
        assert!(ids.contains(&node_id(WidgetId::new(5000))), "top bar");
        assert!(ids.contains(&node_id(WidgetId::new(4000))), "canvas");
        assert!(ids.contains(&node_id(WidgetId::new(7000))), "chat");
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
        h.editor_state_mut().chat.focused = true;
        let update = h.accessibility_tree_update(1280.0, 800.0);
        assert_eq!(update.focus, node_id(WidgetId::new(AI_CHAT_WIDGET_ID)));
    }

    #[test]
    fn a11y_action_on_chat_focuses_input() {
        let mut h = host();
        h.set_now_ms(1234);
        let changed = h.apply_a11y_action(AI_CHAT_WIDGET_ID, true);
        assert!(changed);
        assert!(h.editor_state().chat.focused);
    }

    #[test]
    fn a11y_focus_on_canvas_blurs_chat() {
        let mut h = host();
        h.editor_state_mut().chat.focused = true;
        let changed = h.apply_a11y_action(CANVAS_WIDGET_ID, true);
        assert!(changed);
        assert!(!h.editor_state().chat.focused);
    }

    #[test]
    fn a11y_action_on_unknown_region_is_noop() {
        let mut h = host();
        assert!(!h.apply_a11y_action(99999, true));
    }

    #[test]
    fn collapsed_sidebar_drops_layer_panel_region() {
        let mut h = host();
        h.editor_state_mut().editor_ui.sidebar_open = false;
        let update = h.accessibility_tree_update(1280.0, 800.0);
        let ids: Vec<_> = update.nodes.iter().map(|(id, _)| *id).collect();
        assert!(
            !ids.contains(&node_id(WidgetId::new(1000))),
            "layer panel hidden"
        );
    }
}

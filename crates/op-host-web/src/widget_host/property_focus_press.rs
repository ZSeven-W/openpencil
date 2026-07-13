use super::WidgetHost;
use op_editor_core::PropertyFocus;
use op_editor_ui::widgets::PropertyPanel;
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    pub(in crate::widget_host) fn focus_property_input_from_press(
        &mut self,
        focus: PropertyFocus,
        property_rect: Rect,
        point: Point2D,
    ) -> bool {
        self.commit_property_focus_if_any();
        // The previous property commit may have changed the selected node's
        // resolved Fill/Hug size. Refresh before taking the next input seed.
        self.refresh_layout_scene();
        let (focus, initial) = if let Some(panel) =
            PropertyPanel::for_selection_with_scene(&self.editor_state, &self.layout_scene)
        {
            let focus = panel.hit_test(property_rect, point).unwrap_or(focus);
            (
                focus,
                super::property_dispatch::property_focus_initial(focus, &panel),
            )
        } else {
            (focus, String::new())
        };
        self.editor_state.ui.property_focus = Some(focus);
        self.editor_state
            .ui
            .property_input
            .set_text(initial.clone());
        self.editor_state.ui.property_input.touch(self.now_ms);
        self.editor_state.ui.property_input_draft = initial;
        self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        self.editor_state.ui.property_draft_select_all = false;
        self.editor_state.chat.focused = false;
        self.mark_dirty();
        true
    }
}

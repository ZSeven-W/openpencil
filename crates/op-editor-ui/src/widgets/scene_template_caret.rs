//! Where the caret is in the Asset Center's focused field.
//!
//! Only one consumer, and it is not paint: the hosts hand this rect to the
//! platform so an IME candidate window opens under the text being composed
//! rather than at the pointer or at the window origin. Paint derives its own
//! caret from the same `TextInputState`, so the two cannot disagree about the
//! character position — only about which field is live, and that question is
//! answered here by the same `field_focused` predicate paint uses.

use op_editor_core::SceneTemplateFocus;

use super::scene_template_panel::{
    SceneTemplatePanel, GENERATE_INPUT_PAD_X, GENERATE_TEXT_SIZE, SEARCH_PAD_X, SEARCH_TEXT_SIZE,
};
use super::text_input::single_line_caret_rect;
use crate::Rect;

impl SceneTemplatePanel<'_> {
    /// Caret rect of whichever field the keyboard is writing into.
    ///
    /// Falls back to the search field rather than returning `None`: the panel
    /// always has a focused field while it is open, and a `None` here would
    /// send the candidate window back to the pointer-position fallback for a
    /// field that is right there on screen.
    pub fn focused_input_caret_rect(&self, panel: Rect) -> Rect {
        let center = &self.state.editor_ui.scene_template_center;
        // `field_focused` already resolves the one case where the stored
        // focus and the painted panel disagree — a topic field whose row the
        // scene filter has hidden hands focus back to search.
        // The paste box is a layer over the panel, so while it is up it owns
        // the keyboard whatever the fields below it think.
        if self.style_import_open() {
            let rect = self.style_import_text_rect(panel);
            return crate::Rect::xywh(rect.origin.x + 10.0, rect.origin.y + 10.0, 1.0, 16.0);
        }
        if self.field_focused(SceneTemplateFocus::Generate) {
            if let Some(input) = self.generate_input_rect(panel) {
                return single_line_caret_rect(
                    &center.generate,
                    input,
                    GENERATE_TEXT_SIZE,
                    GENERATE_INPUT_PAD_X,
                );
            }
        }
        single_line_caret_rect(
            &center.search,
            self.search_rect_for(panel),
            SEARCH_TEXT_SIZE,
            SEARCH_PAD_X,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::scene_template_panel::test_rects::MEDIUM as PANEL;
    use op_editor_core::EditorState;

    fn open_state() -> EditorState {
        let mut state = EditorState::default();
        state.editor_ui.scene_template_generate_supported = true;
        state.editor_ui.open_scene_template_center(0);
        state
    }

    /// The caret tracks the focused field, not a fixed one — anchoring the
    /// candidate window over the search box while the user types a topic is
    /// the bug this rect exists to prevent.
    #[test]
    fn the_caret_follows_focus_between_the_two_fields() {
        let mut state = open_state();
        let panel = SceneTemplatePanel::for_editor(&state).expect("open");
        let search = panel.focused_input_caret_rect(PANEL);
        assert!(
            SceneTemplatePanel::search_rect(PANEL).contains(search.origin),
            "search focus must anchor in the search field"
        );

        state.editor_ui.scene_template_center.focus = SceneTemplateFocus::Generate;
        let panel = SceneTemplatePanel::for_editor(&state).expect("open");
        let topic = panel.focused_input_caret_rect(PANEL);
        let input = panel.generate_input_rect(PANEL).expect("the row paints");
        assert!(
            input.contains(topic.origin),
            "topic focus must anchor in the topic field"
        );
        // Both fields start at the same x (same content column, same glyph
        // inset), so the row is what separates them.
        assert!(topic.origin.y > search.origin.y);
    }

    /// A focus on a row the scene filter has hidden falls back to search,
    /// matching what paint does with the same predicate.
    #[test]
    fn a_hidden_topic_row_anchors_at_the_search_field() {
        let mut state = open_state();
        state.editor_ui.scene_template_center.focus = SceneTemplateFocus::Generate;
        state.editor_ui.scene_template_center.filter = op_editor_core::SceneFilter::Scene(
            op_editor_core::scene_template_catalog::TemplateScene::Card,
        );

        let panel = SceneTemplatePanel::for_editor(&state).expect("open");
        assert!(panel.generate_input_rect(PANEL).is_none());
        let caret = panel.focused_input_caret_rect(PANEL);
        assert!(SceneTemplatePanel::search_rect(PANEL).contains(caret.origin));
    }

    /// The caret advances as text is typed, so the candidate window follows
    /// the composition instead of sitting at the field's left edge.
    #[test]
    fn the_caret_advances_with_the_text() {
        let mut state = open_state();
        let empty = SceneTemplatePanel::for_editor(&state)
            .expect("open")
            .focused_input_caret_rect(PANEL);

        state
            .editor_ui
            .scene_template_center
            .search
            .set_text("presentation");
        let typed = SceneTemplatePanel::for_editor(&state)
            .expect("open")
            .focused_input_caret_rect(PANEL);
        assert!(typed.origin.x > empty.origin.x);
    }
}

//! Normal-mode chat begins with the delivered work, rather than a new design.
use super::ai_chat_panel::ExampleCard;
use op_editor_core::EditorState;

pub(crate) struct WorkspaceCopy {
    pub hint: String,
    pub placeholder: String,
    pub tip: String,
    pub examples: [ExampleCard; 4],
}

pub(crate) fn for_editor(state: &EditorState) -> Option<WorkspaceCopy> {
    let workspace = &state.editor_ui.workspace;
    if !workspace.visible
        || !workspace.active
        || op_editor_core::preview_slideshow::active_page_boards(state).is_empty()
    {
        return None;
    }
    let locale = state.editor_ui.effective_locale();
    let tr = |key| op_i18n::translate(locale, key).to_string();
    let pairs = [
        ("ai.work.editTitle", "ai.work.editTitlePrompt"),
        ("ai.work.simplify", "ai.work.simplifyPrompt"),
        ("ai.work.colors", "ai.work.colorsPrompt"),
        ("ai.work.review", "ai.work.reviewPrompt"),
    ];
    Some(WorkspaceCopy {
        hint: tr("ai.work.hint"),
        placeholder: tr("ai.work.placeholder"),
        tip: tr("ai.work.tip"),
        examples: pairs.map(|(title, prompt)| ExampleCard {
            title: tr(title),
            prompt: tr(prompt),
            subtitle: String::new(),
            emoji: "",
        }),
    })
}

/// Localise shared progress labels without changing provider transcripts.
pub(crate) fn progress_label(locale: op_i18n::Locale, text: &str) -> String {
    let key = match text {
        "Checking guidelines" => "ai.work.prepare",
        "Analyzing modification request..." => "ai.work.prepareDetail",
        "Read components" => "ai.work.read",
        "Checked layout" => "ai.work.layout",
        "Designed" => "ai.work.update",
        _ => return text.to_string(),
    };
    op_i18n::translate(locale, key).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::{AIChatHit, AIChatPlaceholder};
    use crate::{Point2D, Rect};
    fn work() -> EditorState {
        let source = r#"{"version":"1.0.0","children":[{"type":"frame","id":"home","name":"首页","width":375,"height":812,"children":[]}]}"#;
        let mut state =
            EditorState::from_document(jian_ops_schema::load_str(source).unwrap().value);
        state.editor_ui.locale = op_i18n::Locale::ZhCn;
        state
            .editor_ui
            .workspace
            .open_for_reading(op_editor_core::HomeFamily::AppUi, 1);
        state
    }
    #[test]
    fn existing_work_shows_edit_actions_and_clicks_fill_the_same_scoped_prompts() {
        let state = work();
        let panel = AIChatPlaceholder::from_editor(&state);
        assert_eq!(panel.label_start_with_ai, "你想怎么改这份作品？");
        assert!(!panel.label_input_placeholder.contains("Agent"));
        let rect = Rect::xywh(0.0, 0.0, 360.0, 700.0);
        for (index, card) in crate::widgets::ai_chat_panel_paint::example_card_rects(rect)
            .iter()
            .enumerate()
        {
            let point = Point2D::new(card.origin.x + 12.0, card.origin.y + 12.0);
            let Some(AIChatHit::Example { prompt, .. }) = panel.hit_test(rect, point) else {
                panic!("action not clickable");
            };
            assert_eq!(prompt, panel.examples[index].prompt);
            assert!(prompt.contains("这一页"));
        }
        assert!(panel.examples[3].prompt.contains("不要修改设计"));
    }
    #[test]
    fn leaving_normal_workspace_restores_creation_context() {
        let mut state = work();
        state.editor_ui.workspace.visible = false;
        assert!(for_editor(&state).is_none());
    }
    #[test]
    fn progress_wording_is_localised_without_rewriting_unknown_provider_labels() {
        assert_eq!(
            progress_label(op_i18n::Locale::ZhCn, "Checking guidelines"),
            "准备修改"
        );
        assert_eq!(
            progress_label(op_i18n::Locale::ZhCn, "provider-specific status"),
            "provider-specific status"
        );
    }
}

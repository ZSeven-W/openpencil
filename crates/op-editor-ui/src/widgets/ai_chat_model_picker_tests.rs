use super::ai_chat_model_picker::*;
use crate::{Point2D, Rect};
use op_editor_core::chat::{AgentProvider, ModelEntry};

fn entry(p: AgentProvider, v: &str) -> ModelEntry {
    ModelEntry::new(p, v, v)
}

#[test]
fn content_height_counts_groups_and_rows() {
    let models = vec![
        entry(AgentProvider::ClaudeCode, "a"),
        entry(AgentProvider::ClaudeCode, "b"),
        entry(AgentProvider::CodexCli, "c"),
    ];
    let expected =
        MODEL_SEARCH_H + 2.0 * MODEL_GROUP_H + 3.0 * MODEL_ROW_H + MODEL_PICKER_PAD_Y * 2.0;
    assert!((picker_content_height(&models, "") - expected).abs() < 0.01);
}

#[test]
fn content_height_groups_noncontiguous_models_by_provider_like_ts_model_groups() {
    let models = vec![
        entry(AgentProvider::ClaudeCode, "claude-a"),
        entry(AgentProvider::CodexCli, "gpt-a"),
        entry(AgentProvider::ClaudeCode, "claude-b"),
    ];

    let expected =
        MODEL_SEARCH_H + 2.0 * MODEL_GROUP_H + 3.0 * MODEL_ROW_H + MODEL_PICKER_PAD_Y * 2.0;
    assert!((picker_content_height(&models, "") - expected).abs() < 0.01);
}

#[test]
fn model_at_resolves_row_and_skips_headers() {
    let models = vec![
        entry(AgentProvider::ClaudeCode, "a"),
        entry(AgentProvider::CodexCli, "b"),
    ];
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(200.0, picker_content_height(&models, "")),
    };
    let first_row_y = MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0;
    assert_eq!(
        model_at(rect, Point2D::new(100.0, first_row_y), &models, 0.0, ""),
        Some(0)
    );
    let header_y = MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H / 2.0;
    assert_eq!(
        model_at(rect, Point2D::new(100.0, header_y), &models, 0.0, ""),
        None
    );
}

#[test]
fn model_at_honors_scroll_offset() {
    let models: Vec<ModelEntry> = (0..40)
        .map(|i| entry(AgentProvider::OpenCode, &format!("m{i}")))
        .collect();
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(200.0, picker_view_height(&models, "")),
    };
    let probe = Point2D::new(
        100.0,
        MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0,
    );
    let unscrolled = model_at(rect, probe, &models, 0.0, "");
    let scrolled = model_at(rect, probe, &models, MODEL_ROW_H * 3.0, "");
    assert_eq!(unscrolled, Some(0));
    assert_eq!(scrolled, Some(3));
    assert!(max_picker_scroll(&models, "") > 0.0);
}

#[test]
fn long_catalog_height_is_capped_to_ts_dropdown_height_and_still_scrolls() {
    let models: Vec<ModelEntry> = (0..40)
        .map(|i| entry(AgentProvider::OpenCode, &format!("m{i}")))
        .collect();

    assert!((picker_view_height(&models, "") - 288.0).abs() < 0.01);
    assert!(max_picker_scroll(&models, "") > 0.0);
}

#[test]
fn model_at_filters_by_search_and_returns_original_index() {
    let models = vec![
        entry(AgentProvider::ClaudeCode, "opus"),
        entry(AgentProvider::CodexCli, "gpt-5.5"),
        entry(AgentProvider::CodexCli, "gpt-4.1"),
    ];
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(220.0, picker_view_height(&models, "5.5")),
    };
    let first_filtered_row_y =
        MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0;

    assert_eq!(
        model_at(
            rect,
            Point2D::new(100.0, first_filtered_row_y),
            &models,
            0.0,
            "5.5"
        ),
        Some(1)
    );
}

#[test]
fn builtin_group_header_prefers_retained_provider_display_name() {
    let mut entry = ModelEntry::builtin(
        AgentProvider::CodexCli,
        "builtin-1",
        "builtin:builtin-1:MiniMax-M2.7",
        "MiniMax-M2.7",
    );
    entry.builtin_provider_display_name = Some("MiniMax".into());

    assert_eq!(group_label_for_entry(&entry), "MINIMAX");
}

#[test]
fn search_matches_retained_builtin_provider_display_name() {
    let entry = ModelEntry::builtin_with_display_name(
        AgentProvider::CodexCli,
        "builtin-bailian",
        "百炼CP",
        "builtin:builtin-bailian:qwen3-coder-plus",
        "qwen3-coder-plus",
    );

    assert_eq!(visible_model_indices(&[entry], "百炼"), vec![0]);
}

#[test]
fn builtin_search_does_not_match_api_key_badge_text() {
    let builtin = ModelEntry::builtin(
        AgentProvider::CodexCli,
        "builtin-1",
        "builtin:builtin-1:deepseek-v4-pro",
        "deepseek-v4-pro",
    );
    let provider_model = entry(AgentProvider::CodexCli, "gpt-5.5");

    assert_eq!(
        visible_model_indices(std::slice::from_ref(&builtin), "api key"),
        Vec::<usize>::new()
    );
    assert_eq!(
        visible_model_indices(std::slice::from_ref(&builtin), "deepseek"),
        vec![0]
    );
    assert_eq!(visible_model_indices(&[provider_model], "openai"), vec![0]);
}

#[test]
fn builtin_group_header_falls_back_to_generic_label_without_retained_name() {
    let entry = ModelEntry::builtin(
        AgentProvider::CodexCli,
        "builtin-1",
        "builtin:builtin-1:deepseek-v4-pro",
        "deepseek-v4-pro",
    );

    assert_eq!(group_label_for_entry(&entry), "OPENAI API KEY");
}

#[test]
fn gemini_provider_group_label_matches_ts_provider_name() {
    assert_eq!(provider_label(AgentProvider::GeminiCli), "GOOGLE GEMINI");
}

#[test]
fn builtin_groups_stay_separate_when_ids_differ_but_provider_matches() {
    let models = vec![
        ModelEntry::builtin(
            AgentProvider::CodexCli,
            "builtin-1",
            "builtin:builtin-1:MiniMax-M2.7",
            "MiniMax-M2.7",
        ),
        ModelEntry::builtin(
            AgentProvider::CodexCli,
            "builtin-2",
            "builtin:builtin-2:deepseek-v4-pro",
            "deepseek-v4-pro",
        ),
    ];

    let expected =
        MODEL_SEARCH_H + 2.0 * MODEL_GROUP_H + 2.0 * MODEL_ROW_H + MODEL_PICKER_PAD_Y * 2.0;
    assert!((picker_content_height(&models, "") - expected).abs() < 0.01);
}

#[test]
fn acp_models_with_same_placeholder_provider_stay_in_separate_groups() {
    let models = vec![
        ModelEntry::new(AgentProvider::CodexCli, "acp:acp-1", "Local ACP"),
        ModelEntry::new(AgentProvider::CodexCli, "acp:acp-2", "Remote ACP"),
    ];

    let expected =
        MODEL_SEARCH_H + 2.0 * MODEL_GROUP_H + 2.0 * MODEL_ROW_H + MODEL_PICKER_PAD_Y * 2.0;
    assert!((picker_content_height(&models, "") - expected).abs() < 0.01);
}

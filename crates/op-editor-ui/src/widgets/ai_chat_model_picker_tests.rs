use super::ai_chat_model_picker::*;
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use jian_core::text_input::TextInputState;
use jian_widgets::components::select::{SelectHit, SelectState};
use op_editor_core::chat::{AgentProvider, ModelEntry};

fn entry(p: AgentProvider, v: &str) -> ModelEntry {
    ModelEntry::new(p, v, v)
}

#[derive(Default)]
struct RoundFillBackend {
    fills: Vec<(Rect, f32, Color)>,
}

impl RenderBackend for RoundFillBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.fills.push((rect, radius, color));
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn color_close(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 1e-6
        && (a.g - b.g).abs() < 1e-6
        && (a.b - b.b).abs() < 1e-6
        && (a.a - b.a).abs() < 1e-6
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
fn model_picker_hit_uses_shared_select_state_protocol() {
    let models = vec![
        entry(AgentProvider::ClaudeCode, "a"),
        entry(AgentProvider::CodexCli, "b"),
    ];
    let state = SelectState {
        open: true,
        ..Default::default()
    };
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(220.0, picker_view_height(&models, "")),
    };
    let first_row_y = MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0;

    assert_eq!(
        model_picker_hit(&state, rect, Point2D::new(100.0, first_row_y), &models, ""),
        SelectHit::Row(0)
    );
    assert_eq!(
        model_picker_hit(&state, rect, Point2D::new(100.0, 12.0), &models, ""),
        SelectHit::Inside
    );
    assert_eq!(
        model_picker_hit(&state, rect, Point2D::new(-1.0, 12.0), &models, ""),
        SelectHit::Outside
    );
}

#[test]
fn pressed_model_row_uses_shared_select_feedback() {
    let models = vec![entry(AgentProvider::ClaudeCode, "claude-sonnet")];
    let rect = Rect {
        origin: Point2D::new(10.0, 20.0),
        size: Point2D::new(240.0, picker_view_height(&models, "")),
    };
    let state = SelectState {
        open: true,
        pressed: Some(0),
        ..Default::default()
    };
    let theme = Theme::dark();
    let input = TextInputState::default();
    let mut backend = RoundFillBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    let row_y = rect.origin.y + MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H;
    let expected_rect = Rect {
        origin: Point2D::new(rect.origin.x + 4.0, row_y + 1.0),
        size: Point2D::new(rect.size.x - 8.0, MODEL_ROW_H - 2.0),
    };
    let expected = theme.button_hover.with_alpha(theme.button_hover.a * 1.8);

    paint_model_picker(
        &mut cx,
        &theme,
        rect,
        &models,
        usize::MAX,
        &state,
        &input,
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend.fills.iter().any(|(fill, radius, color)| {
            *fill == expected_rect && (*radius - 6.0).abs() < 0.01 && color_close(*color, expected)
        }),
        "pressed model row should paint the shared pressed feedback token"
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
fn builtin_search_uses_display_group_label_not_backing_provider_like_ts() {
    let entry = ModelEntry::builtin_with_display_name(
        AgentProvider::CodexCli,
        "builtin-minimax",
        "MiniMax",
        "builtin:builtin-minimax:MiniMax-M2.7",
        "MiniMax-M2.7",
    );

    assert_eq!(
        visible_model_indices(std::slice::from_ref(&entry), "minimax"),
        vec![0]
    );
    assert_eq!(
        visible_model_indices(std::slice::from_ref(&entry), "openai"),
        Vec::<usize>::new()
    );
}

#[test]
fn acp_search_uses_acp_group_label_not_backing_provider_like_ts() {
    let entry = ModelEntry::acp("local-agent", "Local Agent");

    assert_eq!(
        visible_model_indices(std::slice::from_ref(&entry), "local"),
        vec![0]
    );
    assert_eq!(
        visible_model_indices(std::slice::from_ref(&entry), "acp"),
        vec![0]
    );
    assert_eq!(
        visible_model_indices(std::slice::from_ref(&entry), "openai"),
        Vec::<usize>::new()
    );
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

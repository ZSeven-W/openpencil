//! Discovered-model dropdown for the built-in provider form's Model
//! field — the "select" half of a combobox. The field itself stays a
//! free-text input, so a provider that cannot be queried (or a failed
//! fetch) falls back to typing the model id by hand.
//!
//! The menu opens on the Model field of the expanded built-in form
//! (saved agent or add-provider draft), lists the runtime catalog
//! discovered for that credential, and toggles picked rows in the
//! newline-delimited Model field. It only paints while that field
//! owns the settings focus, so any focus move hides it without a
//! dedicated close call.

use crate::theme::Theme;
use crate::widgets::agent_settings_builtin_layout::field_input_rect_for_ui;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::settings_form::{draw_text, ellipsize};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::agent_settings::{
    AgentSettings, BuiltinAgentConfig, BuiltinAgentField, BuiltinModelMenuTarget, SettingsFocus,
};
use op_editor_core::agent_settings_builtin_models::{
    BuiltinModelCatalog, BuiltinModelCatalogPhase, BuiltinModelCatalogTarget,
};
use op_editor_core::editor_ui_state::EditorUiState;

/// Row index of the Model field among DisplayName / ApiKey / Model / BaseUrl.
pub const MODEL_FIELD_ROW: usize = 2;
const MODEL_ROW_H: f32 = 24.0;
const TOUCH_MODEL_ROW_H: f32 = 44.0;
const MODEL_MENU_PAD: f32 = 4.0;
const TOUCH_MODEL_MENU_PAD: f32 = 6.0;
const MODEL_MENU_MAX_VISIBLE: usize = 6;
const TOUCH_MODEL_MENU_MAX_VISIBLE: usize = 5;
const MODEL_MENU_GAP: f32 = 4.0;
const TOUCH_MODEL_MENU_GAP: f32 = 8.0;

/// Menu target for a form card: saved agent by index, or the draft.
pub fn menu_target(index: Option<usize>) -> BuiltinModelMenuTarget {
    match index {
        Some(index) => BuiltinModelMenuTarget::Agent(index),
        None => BuiltinModelMenuTarget::Draft,
    }
}

/// Whether the menu should paint and take hits: its flag is set AND the
/// Model field of that card still owns the settings focus. Commit,
/// Escape, and re-focusing another field all move focus and hide the
/// menu through this predicate alone.
pub fn model_menu_visible(settings: &AgentSettings) -> bool {
    match settings.builtin_model_menu_open {
        Some(BuiltinModelMenuTarget::Agent(index)) => matches!(
            settings.focus,
            Some(SettingsFocus::BuiltinAgent {
                index: focused,
                field: BuiltinAgentField::Model,
            }) if focused == index
        ),
        Some(BuiltinModelMenuTarget::Draft) => matches!(
            settings.focus,
            Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model))
        ),
        None => false,
    }
}

/// Extra card height the open model menu reserves — 0 when closed or
/// hidden. The viewport hugs the current content up to the row cap, so
/// loading, empty, and short catalogs do not leave a large blank panel.
/// Layout (card heights + the fields below Model) and hit-test all derive
/// from this so they cannot drift from the paint.
pub fn model_menu_height(settings: &AgentSettings, index: Option<usize>, touch: bool) -> f32 {
    if settings.builtin_model_menu_open == Some(menu_target(index)) && model_menu_visible(settings)
    {
        menu_view_height(settings, index, touch) + menu_gap(touch)
    } else {
        0.0
    }
}

fn menu_gap(touch: bool) -> f32 {
    if touch {
        TOUCH_MODEL_MENU_GAP
    } else {
        MODEL_MENU_GAP
    }
}

fn menu_metrics(touch: bool) -> (f32, f32, usize) {
    if touch {
        (
            TOUCH_MODEL_MENU_PAD,
            TOUCH_MODEL_ROW_H,
            TOUCH_MODEL_MENU_MAX_VISIBLE,
        )
    } else {
        (MODEL_MENU_PAD, MODEL_ROW_H, MODEL_MENU_MAX_VISIBLE)
    }
}

fn menu_row_count(settings: &AgentSettings, index: Option<usize>, touch: bool) -> usize {
    let (_, _, max_visible) = menu_metrics(touch);
    match catalog(settings, index) {
        Some(catalog) if !catalog.models.is_empty() => catalog.models.len().min(max_visible),
        Some(catalog)
            if matches!(
                catalog.phase,
                BuiltinModelCatalogPhase::Error | BuiltinModelCatalogPhase::Unsupported
            ) && catalog
                .error
                .as_deref()
                .is_some_and(|error| !error.is_empty()) =>
        {
            2
        }
        _ => 1,
    }
}

fn menu_view_height(settings: &AgentSettings, index: Option<usize>, touch: bool) -> f32 {
    let (pad, row_h, _) = menu_metrics(touch);
    pad * 2.0 + menu_row_count(settings, index, touch) as f32 * row_h
}

/// The menu is anchored under the Model field's input rect, same width.
pub fn model_menu_rect(
    settings: &AgentSettings,
    card: Rect,
    index: Option<usize>,
    touch: bool,
) -> Rect {
    let input = field_input_rect_for_ui(settings, card, index, MODEL_FIELD_ROW, touch);
    let gap = menu_gap(touch);
    Rect {
        origin: Point2D::new(input.origin.x, input.origin.y + input.size.y + gap),
        size: Point2D::new(input.size.x, menu_view_height(settings, index, touch)),
    }
}

fn catalog_target(
    settings: &AgentSettings,
    index: Option<usize>,
) -> Option<BuiltinModelCatalogTarget> {
    match index {
        Some(index) => settings
            .builtin_agents
            .get(index)
            .map(|agent| BuiltinModelCatalogTarget::Agent(agent.id.clone())),
        None => Some(BuiltinModelCatalogTarget::Draft),
    }
}

fn catalog(settings: &AgentSettings, index: Option<usize>) -> Option<&BuiltinModelCatalog> {
    settings.builtin_model_catalog(&catalog_target(settings, index)?)
}

/// Whether one row is selected in the field's live newline-delimited model
/// list. While focused the shared input is authoritative; otherwise the
/// normalized saved config is.
fn effective_model_selected<'a>(
    settings: &'a AgentSettings,
    ui: &'a EditorUiState,
    agent: &'a BuiltinAgentConfig,
    index: Option<usize>,
) -> impl Fn(&str) -> bool + 'a {
    let focus = match index {
        Some(index) => SettingsFocus::BuiltinAgent {
            index,
            field: BuiltinAgentField::Model,
        },
        None => SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model),
    };
    move |candidate| {
        if settings.focus == Some(focus) {
            ui.settings_input
                .text()
                .lines()
                .any(|line| line.trim() == candidate)
        } else {
            agent.has_model(candidate)
        }
    }
}

fn row_rect(menu: Rect, row: usize, touch: bool) -> Rect {
    let (pad, row_h) = if touch {
        (TOUCH_MODEL_MENU_PAD, TOUCH_MODEL_ROW_H)
    } else {
        (MODEL_MENU_PAD, MODEL_ROW_H)
    };
    Rect {
        origin: Point2D::new(
            menu.origin.x + pad,
            menu.origin.y + pad + row as f32 * row_h,
        ),
        size: Point2D::new(menu.size.x - pad * 2.0, row_h),
    }
}

fn content_height(options: usize, touch: bool) -> f32 {
    let (pad, row_h) = if touch {
        (TOUCH_MODEL_MENU_PAD, TOUCH_MODEL_ROW_H)
    } else {
        (MODEL_MENU_PAD, MODEL_ROW_H)
    };
    pad * 2.0 + options as f32 * row_h
}

/// Largest valid scroll offset for the open menu's option list.
pub fn model_scroll_max(settings: &AgentSettings, index: Option<usize>, touch: bool) -> f32 {
    let options = catalog(settings, index)
        .map(|catalog| catalog.models.len())
        .unwrap_or(0);
    (content_height(options, touch) - menu_view_height(settings, index, touch)).max(0.0)
}

fn effective_model_scroll(settings: &AgentSettings, index: Option<usize>, touch: bool) -> f32 {
    settings
        .builtin_model_menu_scroll
        .offset
        .clamp(0.0, model_scroll_max(settings, index, touch))
}

/// Whether `point` sits inside the open menu for this card.
pub fn model_menu_contains(
    settings: &AgentSettings,
    card: Rect,
    index: Option<usize>,
    point: Point2D,
    touch: bool,
) -> bool {
    settings.builtin_model_menu_open == Some(menu_target(index))
        && model_menu_visible(settings)
        && model_menu_rect(settings, card, index, touch).contains(point)
}

/// Index into the catalog option list under `point`, honoring scroll.
pub fn model_row_at(
    settings: &AgentSettings,
    card: Rect,
    index: Option<usize>,
    point: Point2D,
    touch: bool,
) -> Option<usize> {
    if !model_menu_contains(settings, card, index, point, touch) {
        return None;
    }
    let menu = model_menu_rect(settings, card, index, touch);
    let options = catalog(settings, index)
        .map(|catalog| catalog.models.len())
        .unwrap_or(0);
    if options == 0 {
        return None;
    }
    let (pad, row_h) = if touch {
        (TOUCH_MODEL_MENU_PAD, TOUCH_MODEL_ROW_H)
    } else {
        (MODEL_MENU_PAD, MODEL_ROW_H)
    };
    let local_y = point.y - menu.origin.y - pad + effective_model_scroll(settings, index, touch);
    if local_y < 0.0 {
        return None;
    }
    let row = (local_y / row_h).floor() as usize;
    let inside_row = local_y - row as f32 * row_h <= row_h;
    (inside_row && row < options).then_some(row)
}

/// Paint the dropdown for the built-in form's Model field. No-op unless
/// the menu is open AND visible for this card.
#[allow(clippy::too_many_arguments)]
pub fn paint_model_menu(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: Option<usize>,
    card: Rect,
    touch: bool,
) {
    if settings.builtin_model_menu_open != Some(menu_target(index)) || !model_menu_visible(settings)
    {
        return;
    }
    let menu = model_menu_rect(settings, card, index, touch);
    let radius = if touch { 10.0 } else { 6.0 };
    cx.backend.fill_round_rect(menu, radius, theme.card);
    cx.backend
        .stroke_round_rect(menu, radius, theme.border, 1.0);
    cx.backend.save();
    cx.backend.clip_rect(menu);
    let scroll_offset = effective_model_scroll(settings, index, touch);
    cx.backend.translate(Point2D::new(0.0, -scroll_offset));
    let mut scrollbar_options = None;
    let catalog = catalog(settings, index);
    let state_row = state_rows(ui, catalog);
    if let Some((hint, detail)) = state_row {
        paint_state_rows(cx, theme, menu, &hint, detail.as_deref(), touch);
    } else if let Some(options) = catalog.map(|catalog| catalog.models.as_slice()) {
        let selected = effective_model_selected(settings, ui, agent, index);
        for (row, option) in options.iter().enumerate() {
            let item = row_rect(menu, row, touch);
            if settings.builtin_model_menu_hover == Some(row) {
                cx.backend
                    .fill_round_rect(item, if touch { 8.0 } else { 5.0 }, theme.button_hover);
            }
            if selected(&option.id) {
                let icon_size = if touch { 18.0 } else { 12.0 };
                draw_icon(
                    cx.backend,
                    Icon::Check,
                    Point2D::new(
                        item.origin.x + if touch { 12.0 } else { 8.0 },
                        item.origin.y + (item.size.y - icon_size) / 2.0,
                    ),
                    icon_size,
                    theme.foreground,
                    1.7,
                );
            }
            let font_size = if touch { 15.0 } else { 11.0 };
            let text_x = item.origin.x + if touch { 44.0 } else { 28.0 };
            let shown = ellipsize(
                cx,
                &option.display_name,
                (item.origin.x + item.size.x - text_x - 8.0).max(0.0),
                font_size,
            );
            draw_text(
                cx,
                &shown,
                font_size,
                theme.foreground,
                text_x,
                if touch {
                    jian_widgets::centered_text_baseline_y(item, font_size)
                } else {
                    item.origin.y + 16.0
                },
            );
        }
        scrollbar_options = Some(options.len());
    }
    cx.backend.restore();
    // The scrollbar belongs to the fixed viewport, not the translated model
    // content. Painting after restore keeps the thumb visible at every offset.
    if let Some(options) = scrollbar_options {
        paint_scrollbar(cx, theme, menu, options, touch, scroll_offset);
    }
}

/// The message row(s) shown while the catalog is empty: loading, no
/// models, or — for Idle/Error/Unsupported and unconfigured providers —
/// the type-manually fallback (with the fetch error underneath when
/// there is one). Rows are informational and never selectable.
fn state_rows(
    ui: &EditorUiState,
    catalog: Option<&BuiltinModelCatalog>,
) -> Option<(String, Option<String>)> {
    match catalog {
        None => Some((t_settings(ui, "builtin.modelsOnePerLine").to_string(), None)),
        Some(catalog) if !catalog.models.is_empty() => None,
        Some(catalog) => match catalog.phase {
            BuiltinModelCatalogPhase::Loading => {
                Some((t_settings(ui, "ai.loadingModels").to_string(), None))
            }
            BuiltinModelCatalogPhase::Ready | BuiltinModelCatalogPhase::Idle => {
                Some((t_settings(ui, "builtin.modelsOnePerLine").to_string(), None))
            }
            BuiltinModelCatalogPhase::Error | BuiltinModelCatalogPhase::Unsupported => Some((
                t_settings(ui, "builtin.typeModelManually").to_string(),
                catalog.error.clone(),
            )),
        },
    }
}

fn paint_state_rows(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    menu: Rect,
    hint: &str,
    detail: Option<&str>,
    touch: bool,
) {
    let font_size = if touch { 15.0 } else { 11.0 };
    let hint_row = row_rect(menu, 0, touch);
    let hint = ellipsize(cx, hint, (hint_row.size.x - 16.0).max(0.0), font_size);
    draw_text(
        cx,
        &hint,
        font_size,
        theme.muted_foreground,
        hint_row.origin.x + 8.0,
        if touch {
            jian_widgets::centered_text_baseline_y(hint_row, font_size)
        } else {
            hint_row.origin.y + 16.0
        },
    );
    if let Some(detail) = detail {
        let detail_row = row_rect(menu, 1, touch);
        let detail = ellipsize(cx, detail, (detail_row.size.x - 16.0).max(0.0), font_size);
        draw_text(
            cx,
            &detail,
            font_size,
            theme.muted_foreground,
            detail_row.origin.x + 8.0,
            if touch {
                jian_widgets::centered_text_baseline_y(detail_row, font_size)
            } else {
                detail_row.origin.y + 16.0
            },
        );
    }
}

fn paint_scrollbar(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    menu: Rect,
    options: usize,
    touch: bool,
    offset: f32,
) {
    let content_h = content_height(options, touch);
    let track_h = (menu.size.y - 8.0).max(0.0);
    let scroll = jian_core::scroll::ScrollState { offset };
    let Some(thumb_geom) = scroll.thumb(
        track_h,
        content_h,
        menu.size.y,
        if touch { 44.0 } else { 24.0 },
    ) else {
        return;
    };
    let thumb = Rect {
        origin: Point2D::new(
            menu.origin.x + menu.size.x - 5.0,
            menu.origin.y + 4.0 + thumb_geom.offset,
        ),
        size: Point2D::new(2.0, thumb_geom.len),
    };
    cx.backend.fill_round_rect(
        thumb,
        1.0,
        Color {
            a: 0.55,
            ..theme.muted_foreground
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_height_is_zero_unless_the_model_field_owns_focus() {
        let mut settings = AgentSettings::default();
        assert_eq!(model_menu_height(&settings, Some(0), false), 0.0);

        settings.builtin_model_menu_open = Some(menu_target(Some(0)));
        assert_eq!(
            model_menu_height(&settings, Some(0), false),
            0.0,
            "the open flag alone must not reserve height without focus"
        );

        settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        });
        assert_eq!(
            model_menu_height(&settings, Some(0), false),
            0.0,
            "a different field on the same card hides the menu"
        );

        settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::Model,
        });
        assert!(
            model_menu_height(&settings, Some(0), false) > 0.0,
            "the Model focus shows the menu"
        );
    }

    #[test]
    fn row_at_maps_scrolled_clicks_onto_catalog_rows() {
        let mut settings = AgentSettings::default();
        settings.builtin_agents.push(BuiltinAgentConfig {
            id: "a1".into(),
            preset: op_editor_core::BuiltinAgentPresetKey::Anthropic,
            display_name: "Anthropic".into(),
            kind: op_editor_core::agent_settings::BuiltinAgentKind::Anthropic,
            api_key: "sk-test".into(),
            models: Vec::new(),
            base_url: "https://api.anthropic.com".into(),
            enabled: true,
        });
        let request = settings
            .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent("a1".into()), 0)
            .expect("discovery request");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("resolvable");
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            op_editor_core::BuiltinModelCatalogRefreshOutcome::Success {
                models: (0..10)
                    .map(|i| {
                        op_editor_core::BuiltinModelOption::new(
                            format!("model-{i}"),
                            format!("Model {i}"),
                        )
                    })
                    .collect(),
            },
        );
        settings.builtin_model_menu_open = Some(menu_target(Some(0)));
        settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::Model,
        });
        let card = Rect {
            origin: Point2D::new(10.0, 10.0),
            size: Point2D::new(500.0, 200.0),
        };
        let menu = model_menu_rect(&settings, card, Some(0), false);
        let first_row = Point2D::new(
            menu.origin.x + 40.0,
            menu.origin.y + MODEL_MENU_PAD + MODEL_ROW_H / 2.0,
        );
        assert_eq!(
            model_row_at(&settings, card, Some(0), first_row, false),
            Some(0)
        );
        assert_eq!(
            model_row_at(
                &settings,
                card,
                Some(0),
                Point2D::new(menu.origin.x + 40.0, menu.origin.y + 1.0),
                false,
            ),
            None,
            "a click on the top padding maps to nothing"
        );
        settings.builtin_model_menu_scroll.offset = MODEL_ROW_H;
        assert_eq!(
            model_row_at(&settings, card, Some(0), first_row, false),
            Some(1),
            "scrolling shifts which row the same screen point hits"
        );
        assert_eq!(
            model_scroll_max(&settings, Some(0), false),
            content_height(10, false) - menu_view_height(&settings, Some(0), false)
        );

        let request = settings
            .force_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent("a1".into()), 2)
            .expect("forced refresh");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("resolvable");
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            op_editor_core::BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![
                    op_editor_core::BuiltinModelOption::new("short-a", "Short A"),
                    op_editor_core::BuiltinModelOption::new("short-b", "Short B"),
                ],
            },
        );
        settings.builtin_model_menu_scroll.offset = 10_000.0;
        assert_eq!(effective_model_scroll(&settings, Some(0), false), 0.0);
        assert_eq!(
            model_row_at(&settings, card, Some(0), first_row, false),
            Some(0),
            "a shorter async result clamps paint and hit-testing to the same first row"
        );
    }

    #[test]
    fn menu_height_hugs_status_and_short_catalog_rows() {
        let mut settings = AgentSettings {
            builtin_agent_draft: Some(BuiltinAgentConfig {
                id: String::new(),
                preset: op_editor_core::BuiltinAgentPresetKey::Anthropic,
                display_name: "Anthropic".into(),
                kind: op_editor_core::agent_settings::BuiltinAgentKind::Anthropic,
                api_key: String::new(),
                models: Vec::new(),
                base_url: "https://api.anthropic.com".into(),
                enabled: true,
            }),
            builtin_model_menu_open: Some(menu_target(None)),
            focus: Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model)),
            ..AgentSettings::default()
        };

        let compact = MODEL_MENU_PAD * 2.0 + MODEL_ROW_H;
        assert_eq!(menu_view_height(&settings, None, false), compact);
        assert_eq!(
            menu_view_height(&settings, None, true),
            TOUCH_MODEL_MENU_PAD * 2.0 + TOUCH_MODEL_ROW_H,
            "touch keeps the guidance row compact while preserving a 44pt target"
        );
        assert_eq!(
            model_menu_height(&settings, None, false),
            compact + MODEL_MENU_GAP,
            "an unconfigured provider shows one compact guidance row"
        );

        settings
            .builtin_agent_draft
            .as_mut()
            .expect("draft")
            .api_key = "sk-test".into();
        let request = settings
            .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Draft, 1)
            .expect("discovery request");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("resolvable");
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            op_editor_core::BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![
                    op_editor_core::BuiltinModelOption::new("model-a", "Model A"),
                    op_editor_core::BuiltinModelOption::new("model-b", "Model B"),
                ],
            },
        );
        assert_eq!(
            menu_view_height(&settings, None, false),
            MODEL_MENU_PAD * 2.0 + MODEL_ROW_H * 2.0,
            "a two-model catalog reserves exactly two rows"
        );
        assert_eq!(
            menu_view_height(&settings, None, true),
            TOUCH_MODEL_MENU_PAD * 2.0 + TOUCH_MODEL_ROW_H * 2.0,
            "touch reserves exactly the discovered rows below its five-row cap"
        );
    }

    #[test]
    fn error_detail_reserves_two_rows_but_never_the_full_catalog_cap() {
        let mut settings = AgentSettings {
            builtin_agent_draft: Some(BuiltinAgentConfig {
                id: String::new(),
                preset: op_editor_core::BuiltinAgentPresetKey::Anthropic,
                display_name: "Anthropic".into(),
                kind: op_editor_core::agent_settings::BuiltinAgentKind::Anthropic,
                api_key: "sk-test".into(),
                models: Vec::new(),
                base_url: "https://api.anthropic.com".into(),
                enabled: true,
            }),
            builtin_model_menu_open: Some(menu_target(None)),
            focus: Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model)),
            ..AgentSettings::default()
        };
        let request = settings
            .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Draft, 1)
            .expect("discovery request");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("resolvable");
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            op_editor_core::BuiltinModelCatalogRefreshOutcome::Error {
                error: "request failed".into(),
            },
        );

        assert_eq!(
            menu_view_height(&settings, None, false),
            MODEL_MENU_PAD * 2.0 + MODEL_ROW_H * 2.0
        );
    }

    #[test]
    fn hidden_by_focus_change_menus_take_no_hits() {
        let settings = AgentSettings {
            builtin_model_menu_open: Some(menu_target(None)),
            focus: Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey)),
            ..AgentSettings::default()
        };
        let card = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(500.0, 300.0),
        };
        assert!(!model_menu_contains(
            &settings,
            card,
            None,
            Point2D::new(200.0, 200.0),
            false,
        ));
    }
}

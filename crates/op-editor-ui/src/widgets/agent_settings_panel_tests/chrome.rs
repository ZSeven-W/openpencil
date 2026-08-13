//! Modal chrome: the close button and the top tab strip.
//!
//! Split out of `agent_settings_panel_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;
use crate::widgets::agent_settings_panel_geometry::{close_rect, nav_item_rect};
use op_editor_core::agent_settings::{BuiltinAgentConfig, BuiltinAgentKind};
use op_editor_core::agent_settings_builtin_presets::BuiltinAgentPresetKey;
use op_editor_core::size_class::EditorSizeClass;

fn tab_center(panel: Rect, index: usize, count: usize) -> Point2D {
    let pill = nav_item_rect(panel, index, count);
    Point2D::new(
        pill.origin.x + pill.size.x / 2.0,
        pill.origin.y + pill.size.y / 2.0,
    )
}

fn rect_center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn touch_state(class: EditorSizeClass) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = class;
    state
}

#[test]
fn close_button_paints_after_scrollable_content() {
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let close_origin = close_rect(rect).origin;
    let close_idx = backend
        .icon_strokes
        .iter()
        .find_map(|(at, size, idx)| {
            ((at.x - close_origin.x).abs() < 0.01
                && (at.y - close_origin.y).abs() < 0.01
                && (*size - 16.0).abs() < 0.01)
                .then_some(*idx)
        })
        .expect("close icon should paint");
    let restore_idx = backend
        .ops
        .iter()
        .rposition(|op| *op == "restore")
        .expect("content clip should restore");

    assert!(
        close_idx > restore_idx,
        "close button must paint above clipped, scrollable content"
    );
}

#[test]
fn close_button_hover_paints_visible_wash() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.hover_agent_settings_close = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let close = close_rect(rect);
    assert!(
        backend
            .round_fills
            .iter()
            .any(|(fill, color)| *fill == close && color_eq(*color, panel.theme.button_hover)),
        "hovered close button should paint a visible hover wash"
    );
}

#[test]
fn pressed_close_button_uses_shared_button_feedback() {
    let mut state = EditorState::default();
    state.editor_ui.pressed_button =
        Some(ButtonPressTarget::AgentSettings(AgentSettingsButton::Close));
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let close = close_rect(rect);
    let expected = panel
        .theme
        .button_hover
        .with_alpha(panel.theme.button_hover.a * 1.8);
    assert!(
        backend
            .round_fills
            .iter()
            .any(|(fill, color)| *fill == close && color_eq(*color, expected)),
        "pressed close button should paint the shared pressed feedback token"
    );
}

#[test]
fn agents_tab_icon_uses_ts_pen_glyph_not_pencil() {
    const PEN_PATH: &str =
        "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z";
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    // The glyph is centred with its label inside the pill, so assert it
    // lands anywhere inside the first pill rather than re-deriving the
    // centring maths here.
    let pill = nav_item_rect(rect, 0, 5);
    let strokes: Vec<_> = backend
        .svg_strokes
        .iter()
        .filter(|(_, at, size)| (*size - 16.0).abs() < 0.01 && pill.contains(*at))
        .collect();

    assert_eq!(strokes.len(), 1, "TS settings nav uses lucide Pen");
    assert_eq!(strokes[0].0, PEN_PATH);
}

#[test]
fn top_tab_strip_selects_by_pill() {
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);

    assert_eq!(
        panel.hit_test(rect, tab_center(rect, 0, 5)),
        AgentSettingsHit::SelectTab(AgentSettingsTab::Agents)
    );
    assert_eq!(
        panel.hit_test(rect, tab_center(rect, 1, 5)),
        AgentSettingsHit::SelectTab(AgentSettingsTab::Mcp)
    );
    // The strip is centred with clear space on both flanks — a press out
    // there stays inside the modal without selecting anything.
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(rect.origin.x + 12.0, rect.origin.y + 30.0)
        ),
        AgentSettingsHit::Inside
    );
}

#[test]
fn compact_touch_settings_hide_mcp_and_fall_back_to_agents() {
    let mut state = touch_state(EditorSizeClass::Compact);
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(390.0, 844.0);
    let layout = panel.resolved_layout(rect);

    assert_eq!(rect, Rect::xywh(0.0, 0.0, 390.0, 844.0));
    assert_eq!(
        panel.navigation_tabs(),
        &[
            AgentSettingsTab::Agents,
            AgentSettingsTab::Images,
            AgentSettingsTab::Fonts,
            AgentSettingsTab::System,
        ]
    );
    assert_eq!(panel.active_tab(), AgentSettingsTab::Agents);
    assert_eq!(layout.close_target.size, Point2D::new(44.0, 44.0));
    for index in 0..panel.navigation_tabs().len() {
        let tab = layout.nav_item_rect(index, panel.navigation_tabs().len());
        assert!(tab.size.x >= 44.0 && tab.size.y >= 44.0);
        assert_eq!(
            panel.hit_test(rect, rect_center(tab)),
            AgentSettingsHit::SelectTab(panel.navigation_tabs()[index])
        );
    }
    assert!(layout.content.origin.y >= layout.navigation.origin.y + layout.navigation.size.y);
}

#[test]
fn compact_account_tabs_wrap_without_overflow_at_320_points() {
    let mut state = touch_state(EditorSizeClass::Compact);
    state.editor_ui.account_ui_available = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(320.0, 568.0);
    let layout = panel.resolved_layout(rect);
    let tabs = panel.navigation_tabs();

    assert_eq!(tabs.len(), 5);
    assert!(!tabs.contains(&AgentSettingsTab::Mcp));
    let item_rects: Vec<_> = (0..tabs.len())
        .map(|index| layout.nav_item_rect(index, tabs.len()))
        .collect();
    assert!(
        layout.navigation.size.y > 44.0,
        "five tabs wrap to two rows"
    );
    for item in &item_rects {
        assert!(item.size.x >= 44.0 && item.size.y >= 44.0);
        assert!(item.origin.x >= layout.navigation.origin.x);
        assert!(
            item.origin.x + item.size.x
                <= layout.navigation.origin.x + layout.navigation.size.x + 0.01
        );
        assert!(
            item.origin.y + item.size.y
                <= layout.navigation.origin.y + layout.navigation.size.y + 0.01
        );
    }
    for (index, a) in item_rects.iter().enumerate() {
        for b in item_rects.iter().skip(index + 1) {
            let overlap_x =
                a.origin.x < b.origin.x + b.size.x && b.origin.x < a.origin.x + a.size.x;
            let overlap_y =
                a.origin.y < b.origin.y + b.size.y && b.origin.y < a.origin.y + a.size.y;
            assert!(!(overlap_x && overlap_y), "touch tabs must not overlap");
        }
    }
}

#[test]
fn medium_touch_settings_use_a_centered_680_point_column() {
    let state = touch_state(EditorSizeClass::Medium);
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(834.0, 1_112.0);
    let layout = panel.resolved_layout(rect);

    assert_eq!(rect, Rect::xywh(12.0, 12.0, 810.0, 1_088.0));
    assert_eq!(layout.content.size.x, 680.0);
    assert_eq!(layout.navigation.size.x, 680.0);
    assert_eq!(layout.close_target.size, Point2D::new(44.0, 44.0));
    assert!(!layout.vertical_navigation);
}

#[test]
fn expanded_touch_settings_use_a_bounded_surface_and_vertical_rail() {
    let state = touch_state(EditorSizeClass::Expanded);
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1_194.0, 834.0);
    let layout = panel.resolved_layout(rect);
    let tabs = panel.navigation_tabs();

    assert_eq!(rect.size, Point2D::new(960.0, 800.0));
    assert_eq!(rect.origin, Point2D::new(117.0, 17.0));
    assert!(layout.vertical_navigation);
    assert!(layout.content.size.x <= 680.0);
    assert!(!tabs.contains(&AgentSettingsTab::Mcp));
    for index in 0..tabs.len() {
        let item = layout.nav_item_rect(index, tabs.len());
        assert!(item.size.x >= 44.0 && item.size.y >= 44.0);
    }
}

#[test]
fn touch_saved_provider_actions_are_visible_and_have_44_point_targets() {
    let mut state = touch_state(EditorSizeClass::Compact);
    state
        .editor_ui
        .agent_settings
        .builtin_agents
        .push(BuiltinAgentConfig {
            id: "builtin-1".into(),
            preset: BuiltinAgentPresetKey::Custom,
            display_name: "OpenAI compatible".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: "sk-test".into(),
            model: "model".into(),
            base_url: "https://example.test/v1".into(),
            enabled: true,
        });
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(390.0, 844.0);
    let content = panel.resolved_content_viewport(rect);
    let card = Rect::xywh(
        content.origin.x,
        content.origin.y
            + crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
            + crate::widgets::agent_settings_metrics::SECTION_HEADER_H
            + crate::widgets::agent_settings_metrics::SECTION_SUBTITLE_H,
        content.size.x,
        crate::widgets::agent_settings_metrics::ROW_H_TWO_LINE,
    );
    let edit = crate::widgets::agent_settings_builtin_layout::compact_touch_edit_target(card);
    let remove = crate::widgets::agent_settings_builtin_layout::compact_touch_remove_target(card);
    assert_eq!(edit.size, Point2D::new(44.0, 44.0));
    assert_eq!(remove.size, Point2D::new(44.0, 44.0));
    assert_eq!(
        panel.hit_test(rect, rect_center(edit)),
        AgentSettingsHit::EditBuiltinAgent(0)
    );
    assert_eq!(
        panel.hit_test(rect, rect_center(remove)),
        AgentSettingsHit::RemoveBuiltinAgent(0)
    );

    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint(&mut cx, rect);
    for icon in [Icon::Pencil, Icon::Trash] {
        assert!(
            backend
                .svg_strokes
                .iter()
                .any(|(path, _, _)| icon.paths().contains(&path.as_str())),
            "touch rows always paint their management action"
        );
    }
}

#[test]
fn touch_empty_provider_card_has_a_44_point_add_target() {
    let state = touch_state(EditorSizeClass::Compact);
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(390.0, 844.0);
    let content = panel.resolved_content_viewport(rect);
    let y = content.origin.y + crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT;
    let target = crate::widgets::agent_settings_builtin_layout::touch_empty_cta_rect(
        content,
        y + crate::widgets::agent_settings_metrics::SECTION_HEADER_H
            + crate::widgets::agent_settings_metrics::SECTION_SUBTITLE_H,
    );

    assert_eq!(target.size.y, 44.0);
    assert_eq!(
        panel.hit_test(rect, rect_center(target)),
        AgentSettingsHit::AddProvider
    );
}

#[test]
fn touch_empty_provider_target_tracks_the_sync_error_row() {
    let mut state = touch_state(EditorSizeClass::Compact);
    state.editor_ui.agent_settings.web_credential_sync_error = Some("offline".into());
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(390.0, 844.0);
    let content = panel.resolved_content_viewport(rect);
    let y = content.origin.y
        + crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
        + crate::widgets::agent_settings_metrics::SECTION_HEADER_H
        + crate::widgets::agent_settings_metrics::SECTION_SUBTITLE_H
        + crate::widgets::agent_settings_builtin_layout::sync_error_height(
            &state.editor_ui.agent_settings,
        );
    let target = crate::widgets::agent_settings_builtin_layout::touch_empty_cta_rect(content, y);

    assert_eq!(
        panel.hit_test(rect, rect_center(target)),
        AgentSettingsHit::AddProvider
    );
}

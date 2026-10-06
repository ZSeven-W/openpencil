use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{AssetCenterTab, EditorState};

use super::host_overlay_geometry::scene_template_panel_rect;
use super::scene_template_panel::{grid_columns, SceneTemplatePanel};
use crate::Rect;

fn touch_state(size_class: EditorSizeClass) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = size_class;
    state.editor_ui.scene_template_generate_supported = true;
    state.editor_ui.open_scene_template_center(0);
    state
}

fn assert_touch_targets(state: &EditorState, viewport_w: f32, viewport_h: f32) {
    let rect = scene_template_panel_rect(state, viewport_w, viewport_h).expect("open panel");
    let panel = SceneTemplatePanel::for_editor(state).expect("panel model");

    assert!(panel.close_rect_for(rect).size.x >= 44.0);
    assert!(panel.search_rect_for(rect).size.y >= 44.0);
    for (tab, _) in panel.tab_chip_rects(rect) {
        assert!(tab.size.y >= 44.0);
    }
    let content = SceneTemplatePanel::content_rect(rect);
    for (chip, _) in panel.filter_chip_rects(rect) {
        assert!(chip.size.y >= 44.0);
        assert!(chip.origin.x >= content.origin.x);
        assert!(chip.origin.x + chip.size.x <= content.origin.x + content.size.x + 0.01);
    }
    assert!(panel.generate_input_rect(rect).expect("topic input").size.y >= 44.0);
    assert!(
        panel
            .generate_button_rect(rect)
            .expect("generate button")
            .size
            .y
            >= 44.0
    );

    let (primary, secondary) = panel.card_action_rects_for(
        Rect::xywh(rect.origin.x + 24.0, rect.origin.y + 300.0, 300.0, 260.0),
        true,
    );
    assert!(primary.size.y >= 44.0);
    assert!(secondary.expect("secondary action").size.y >= 44.0);
    assert!(panel.card_actions_visible(0));
}

#[test]
fn compact_and_medium_asset_centers_use_touch_sized_controls() {
    // card_actions_visible(0) is a shipped-card assertion: a saved template
    // left in the process-global registry by a concurrently running Asset
    // Center test would make index 0 a saved card (which never shows the
    // action strip) and fail it. Hold the registry lock like every other
    // test that depends on the saved/shipped split.
    let _templates =
        super::asset_center_template_cards::template_test_support::exclusive_user_templates();
    for (class, width, height) in [
        (EditorSizeClass::Compact, 390.0_f32, 844.0_f32),
        (EditorSizeClass::Medium, 834.0_f32, 1_112.0_f32),
    ] {
        let state = touch_state(class);
        assert_touch_targets(&state, width, height);
    }
}

#[test]
fn compact_filters_wrap_and_leave_the_cards_below_them() {
    let state = touch_state(EditorSizeClass::Compact);
    let rect = scene_template_panel_rect(&state, 390.0, 844.0).expect("open panel");
    let panel = SceneTemplatePanel::for_editor(&state).expect("panel model");
    let chips = panel.filter_chip_rects(rect);
    assert!(
        chips
            .windows(2)
            .any(|pair| pair[1].0.origin.y > pair[0].0.origin.y),
        "the complete filter set should wrap on a 390pt phone"
    );
    let chip_bottom = chips
        .iter()
        .map(|(chip, _)| chip.origin.y + chip.size.y)
        .fold(0.0, f32::max);
    assert!(panel.cards_viewport(rect).origin.y > chip_bottom);
    assert_eq!(panel.grid_metrics(rect).0, 1);
}

#[test]
fn compact_landscape_keeps_a_useful_card_viewport_on_both_tabs() {
    let mut state = touch_state(EditorSizeClass::Compact);
    let rect = scene_template_panel_rect(&state, 844.0, 390.0).expect("landscape panel");
    for tab in [AssetCenterTab::Templates, AssetCenterTab::Styles] {
        state.editor_ui.scene_template_center.tab = tab;
        let panel = SceneTemplatePanel::for_editor(&state).expect("panel model");
        let cards = panel.cards_viewport(rect);
        assert!(
            cards.size.y >= 150.0,
            "{tab:?} left only {}pt for cards",
            cards.size.y
        );
        assert!(rect.contains(panel.close_rect_for(rect).origin));
        assert!(panel.close_rect_for(rect).size.x >= 44.0);
        let track = panel.tab_track_rect(rect);
        assert!(track.origin.y >= rect.origin.y);
        assert!(track.origin.y + track.size.y <= cards.origin.y);
    }
}

#[test]
fn touch_style_actions_are_visible_and_at_least_44_points() {
    let mut state = touch_state(EditorSizeClass::Compact);
    state.editor_ui.scene_template_center.tab = AssetCenterTab::Styles;
    let rect = scene_template_panel_rect(&state, 390.0, 844.0).expect("open panel");
    let panel = SceneTemplatePanel::for_editor(&state).expect("panel model");

    assert!(
        panel
            .style_import_button_rect(rect)
            .expect("style import")
            .size
            .y
            >= 44.0
    );
    assert!(panel.style_import_confirm_rect(rect).size.y >= 44.0);
    assert!(panel.style_import_cancel_rect(rect).size.y >= 44.0);
    assert!(panel.style_delete_visible(0));
    assert!(
        panel
            .style_delete_rect_for(Rect::xywh(20.0, 20.0, 300.0, 128.0))
            .size
            .x
            >= 44.0
    );
}

#[test]
fn mouse_layout_keeps_legacy_density_even_with_compact_or_medium_size_classes() {
    for class in [EditorSizeClass::Compact, EditorSizeClass::Medium] {
        let mut state = EditorState::default();
        state.editor_ui.touch = false;
        state.editor_ui.size_class = class;
        state.editor_ui.open_scene_template_center(0);
        let panel = SceneTemplatePanel::for_editor(&state).expect("panel model");
        let rect = Rect::xywh(24.0, 64.0, 342.0, 760.0);

        assert_eq!(
            panel.search_rect_for(rect),
            SceneTemplatePanel::search_rect(rect)
        );
        assert_eq!(
            panel.grid_metrics(rect).0,
            grid_columns(panel.cards_viewport(rect).size.x)
        );
    }
}

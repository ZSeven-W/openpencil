use super::*;
use jian_ops_schema::node::PenNode;
use op_editor_core::motion_presets::MotionPreset;
use op_editor_core::Locale;

fn summary(json: serde_json::Value) -> MotionSummary {
    let node: PenNode = serde_json::from_value(json).expect("node");
    MotionSummary::of(&node)
}

fn fade() -> serde_json::Value {
    serde_json::to_value(
        op_editor_core::motion_presets::MotionPreset::FadeIn
            .animation(jian_ops_schema::motion::MotionTrigger::Mount),
    )
    .expect("animation json")
}

#[test]
fn an_empty_node_shows_add_and_transition_rows() {
    let summary = summary(serde_json::json!({
        "type": "rectangle", "id": "r", "width": 10, "height": 10
    }));
    assert_eq!(
        rows(&summary),
        [MotionRowKind::Add, MotionRowKind::Transition]
    );
    assert_eq!(
        motion_section_height(&summary),
        SECTION_HEADER_HEIGHT + 2.0 * ROW_STEP + 6.0 + SECTION_GAP
    );
    let geometry = row_geometry(&summary, 0.0, 0.0, 280.0);
    assert!(geometry.iter().all(|row| row.button.is_none()));
}

#[test]
fn animations_stagger_and_a_transition_add_rows_and_remove_buttons() {
    let summary = summary(serde_json::json!({
        "type": "frame", "id": "list", "width": 10, "height": 10,
        "animations": [fade(), fade()],
        "transition": {"durationMs": 150, "easing": "standard"},
        "children": [{"type": "rectangle", "id": "a", "width": 1, "height": 1}]
    }));
    assert_eq!(
        rows(&summary),
        [
            MotionRowKind::Animation(0),
            MotionRowKind::Animation(1),
            MotionRowKind::Add,
            MotionRowKind::Stagger,
            MotionRowKind::Transition,
        ]
    );
    let geometry = row_geometry(&summary, 0.0, 0.0, 280.0);
    let buttons: Vec<bool> = geometry.iter().map(|row| row.button.is_some()).collect();
    assert_eq!(buttons, [true, true, false, false, true]);
    // Rows are stacked one step apart under the header.
    assert_eq!(
        geometry[1].main.origin.y - geometry[0].main.origin.y,
        ROW_STEP
    );
}

#[test]
fn labels_name_trigger_preset_duration_and_loops() {
    let mut pulse = serde_json::to_value(
        op_editor_core::motion_presets::MotionPreset::Pulse
            .animation(jian_ops_schema::motion::MotionTrigger::InView),
    )
    .expect("animation json");
    pulse["delayMs"] = serde_json::json!(80);
    let summary = summary(serde_json::json!({
        "type": "rectangle", "id": "r", "width": 10, "height": 10,
        "animations": [pulse],
        "transition": {"durationMs": 150, "easing": "standard"}
    }));
    assert_eq!(
        animation_label(Locale::EnUs, &summary.rows[0]),
        "In view · Pulse · 1200 ms · ∞"
    );
    assert_eq!(
        transition_label(Locale::EnUs, &summary),
        "Transition · 150 ms · Standard"
    );
    assert_eq!(
        animation_label(Locale::ZhCn, &summary.rows[0]),
        "进入视口时 · 脉冲 · 1200 ms · ∞"
    );
}

fn actions(summary: &MotionSummary) -> Vec<PropertyPanelAction> {
    let mut out = Vec::new();
    push_motion_action_rects(&mut out, summary, 0.0, 0.0, 280.0);
    out.into_iter().map(|(action, _)| action).collect()
}

#[test]
fn rows_open_menus_and_buttons_remove() {
    let with_animation = summary(serde_json::json!({
        "type": "rectangle", "id": "r", "width": 10, "height": 10,
        "animations": [fade()]
    }));
    assert_eq!(
        actions(&with_animation),
        [
            menu(MotionMenuTarget::Animation(0)),
            edit(MotionPanelEdit::Remove(0)),
            menu(MotionMenuTarget::AddPreset),
            edit(MotionPanelEdit::AddTransition),
        ]
    );
    let with_transition = summary(serde_json::json!({
        "type": "rectangle", "id": "r", "width": 10, "height": 10,
        "transition": {"durationMs": 150, "easing": "standard"}
    }));
    assert_eq!(
        actions(&with_transition)[1..],
        [
            menu(MotionMenuTarget::TransitionEasing),
            edit(MotionPanelEdit::RemoveTransition),
        ]
    );
}

#[test]
fn menus_list_presets_triggers_and_curves() {
    let summary = summary(serde_json::json!({
        "type": "rectangle", "id": "r", "width": 10, "height": 10,
        "animations": [fade(), fade()]
    }));
    let locale = Locale::EnUs;
    assert_eq!(
        motion_menu_rows(locale, MotionMenuTarget::AddPreset, &summary).len(),
        MotionPreset::ALL.len()
    );
    // Staggering an empty pair of keyframes would do nothing.
    assert_eq!(
        motion_menu_rows(locale, MotionMenuTarget::StaggerPreset, &summary).len(),
        MotionPreset::ALL.len() - 1
    );
    let row_menu: Vec<String> = motion_menu_rows(locale, MotionMenuTarget::Animation(1), &summary)
        .into_iter()
        .map(|(_, label)| label)
        .collect();
    assert_eq!(
        row_menu,
        ["✓ On load", "In view", "Manual", "Loop forever", "Move up"]
    );
    let easings = motion_menu_rows(locale, MotionMenuTarget::Easing(0), &summary);
    assert_eq!(easings[0].1, "✓ Standard");
    assert_eq!(
        easings[4].0,
        edit(MotionPanelEdit::SetEasing(
            0,
            jian_ops_schema::motion::Easing::Linear
        ))
    );
    assert!(motion_menu_rows(locale, MotionMenuTarget::Animation(5), &summary).is_empty());
}

#[test]
fn the_interactions_section_moves_down_by_the_motion_rows() {
    use crate::widgets::property_panel_interactions::InteractionSummary;
    use crate::widgets::property_panel_layout::action_button_rects;
    use crate::widgets::property_panel_visibility::VisibleSections;
    let add_interaction_y = |motion: MotionSummary| {
        let interactions = InteractionSummary {
            motion,
            ..InteractionSummary::default()
        };
        action_button_rects(
            Rect::xywh(0.0, 0.0, 280.0, 2000.0),
            VisibleSections::ALL,
            &[],
            &[],
            &interactions,
        )
        .into_iter()
        .find(|(action, _)| {
            *action == PropertyPanelAction::ToggleInteractionMenu(InteractionMenuTarget::AddTrigger)
        })
        .map(|(_, rect)| rect.origin.y)
        .expect("add interaction row")
    };
    let empty = summary(serde_json::json!({
        "type": "rectangle", "id": "r", "width": 10, "height": 10
    }));
    let two = summary(serde_json::json!({
        "type": "rectangle", "id": "r", "width": 10, "height": 10,
        "animations": [fade(), fade()]
    }));
    assert_eq!(
        add_interaction_y(two) - add_interaction_y(empty),
        2.0 * ROW_STEP
    );
}

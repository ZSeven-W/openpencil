use super::*;
use jian_ops_schema::node::PenNode;
use op_editor_core::motion_presets::MotionPreset;
use op_editor_core::Locale;
use op_editor_core::{EditorState, PropertyFocus};

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
            MotionRowKind::TransitionTiming,
        ]
    );
    let geometry = row_geometry(&summary, 0.0, 0.0, 280.0);
    let buttons: Vec<bool> = geometry.iter().map(|row| row.button.is_some()).collect();
    assert_eq!(buttons, [true, true, false, false, true, false]);
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
        "In view · Pulse ∞ · 1200 ms"
    );
    assert_eq!(
        transition_label(Locale::EnUs, &summary),
        "Transition · 150 ms · Standard"
    );
    assert_eq!(
        animation_label(Locale::ZhCn, &summary.rows[0]),
        "进入视口时 · 脉冲 ∞ · 1200 ms"
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
            PropertyPanelAction::ToggleMotionRow(0),
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

fn expanded(mut summary: MotionSummary, index: usize) -> MotionSummary {
    summary.expanded = Some(index);
    summary
}

#[test]
fn an_open_row_shows_its_fields() {
    let mut manual_loop = serde_json::to_value(
        op_editor_core::motion_presets::MotionPreset::Spin
            .animation(jian_ops_schema::motion::MotionTrigger::Manual),
    )
    .expect("animation json");
    manual_loop["id"] = serde_json::json!("spin");
    let base = summary(serde_json::json!({
        "type": "rectangle", "id": "r", "width": 10, "height": 10,
        "animations": [fade(), manual_loop]
    }));
    // A finite mount animation: options, timing, curve and repeat.
    let open = expanded(base.clone(), 0);
    assert_eq!(
        rows(&open)[..7],
        [
            MotionRowKind::Animation(0),
            MotionRowKind::Options(0),
            MotionRowKind::Timing(0),
            MotionRowKind::Easing(0),
            MotionRowKind::Curve(0),
            MotionRowKind::Fill(0),
            MotionRowKind::Repeat(0),
        ]
    );
    let mut inputs = Vec::new();
    push_motion_input_rects(&mut inputs, &open, 0.0, 0.0, 280.0);
    let focuses: Vec<PropertyFocus> = inputs.iter().map(|(focus, _)| *focus).collect();
    assert_eq!(
        focuses,
        [
            PropertyFocus::MotionField(0, MotionField::Duration),
            PropertyFocus::MotionField(0, MotionField::Delay),
            PropertyFocus::MotionField(0, MotionField::Curve(0)),
            PropertyFocus::MotionField(0, MotionField::Curve(1)),
            PropertyFocus::MotionField(0, MotionField::Curve(2)),
            PropertyFocus::MotionField(0, MotionField::Curve(3)),
            PropertyFocus::MotionField(0, MotionField::Repeat),
        ]
    );
    // The four controls share one row after the thumbnail.
    assert!(inputs[2..6]
        .windows(2)
        .all(|pair| pair[0].1.origin.y == pair[1].1.origin.y
            && pair[1].1.origin.x > pair[0].1.origin.x + pair[0].1.size.x));
    // Duration and delay share a row, side by side and indented.
    assert_eq!(inputs[0].1.origin.y, inputs[1].1.origin.y);
    assert!(inputs[1].1.origin.x > inputs[0].1.origin.x + inputs[0].1.size.x);
    let actions: Vec<PropertyPanelAction> = actions(&open);
    assert!(actions.contains(&menu(MotionMenuTarget::Animation(0))));
    assert!(actions.contains(&menu(MotionMenuTarget::Easing(0))));
    assert!(actions.contains(&menu(MotionMenuTarget::Fill(0))));

    // An endless manual loop has no repeat count but needs its id.
    let open = expanded(base, 1);
    let rows = rows(&open);
    assert!(rows.contains(&MotionRowKind::Id(1)));
    assert!(!rows.contains(&MotionRowKind::Repeat(1)));
    assert_eq!(
        motion_section_height(&open),
        SECTION_HEADER_HEIGHT + rows.len() as f32 * ROW_STEP + 6.0 + SECTION_GAP
    );
}

#[test]
fn fields_seed_from_the_row_and_commit_through_motion_edits() {
    let doc = jian_ops_schema::load_str(
        &serde_json::json!({
            "version": "1.0",
            "children": [{
                "type": "rectangle", "id": "r", "width": 10, "height": 10,
                "animations": [fade()],
                "transition": {"durationMs": 150, "easing": "standard"}
            }]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    let mut state = EditorState::from_document(doc);
    assert!(state.apply(op_editor_core::EditorCommand::SetSelection {
        node_id: op_editor_core::NodeId::new("r"),
    }));
    let summary_now = |state: &EditorState| MotionSummary::of(state.selected_node().unwrap());
    let duration = PropertyFocus::MotionField(0, MotionField::Duration);
    assert_eq!(field_initial(duration, &summary_now(&state)), "300");
    assert_eq!(
        field_initial(PropertyFocus::TransitionDuration, &summary_now(&state)),
        "150"
    );
    let summary = summary_now(&state);
    commit_field(&mut state, &summary, duration, " 640 ");
    commit_field(
        &mut state,
        &summary,
        PropertyFocus::MotionField(0, MotionField::Delay),
        "oops",
    );
    commit_field(
        &mut state,
        &summary,
        PropertyFocus::TransitionDuration,
        "90",
    );
    let summary = summary_now(&state);
    assert_eq!(
        (summary.rows[0].duration_ms, summary.rows[0].delay_ms),
        (640, 0)
    );
    assert_eq!(summary.transition.map(|t| t.duration_ms), Some(90));
}

#[test]
fn curve_fields_show_the_runtime_controls_and_write_a_bezier() {
    let doc = jian_ops_schema::load_str(
        &serde_json::json!({
            "version": "1.0",
            "children": [{
                "type": "rectangle", "id": "r", "width": 10, "height": 10,
                "animations": [{"trigger": "mount", "durationMs": 400, "easing": "standard", "keyframes": [
                    {"offset": 0, "values": {"opacity": 0}},
                    {"offset": 1, "values": {"opacity": 1}}
                ]}]
            }]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    let mut state = EditorState::from_document(doc);
    assert!(state.apply(op_editor_core::EditorCommand::SetSelection {
        node_id: op_editor_core::NodeId::new("r"),
    }));
    let summary = MotionSummary::of(state.selected_node().unwrap());
    let x1 = PropertyFocus::MotionField(0, MotionField::Curve(0));
    let y2 = PropertyFocus::MotionField(0, MotionField::Curve(3));
    assert_eq!(field_initial(x1, &summary), "0.2");
    assert_eq!(field_initial(y2, &summary), "1");
    commit_field(&mut state, &summary, y2, "1.25");
    let easing = MotionSummary::of(state.selected_node().unwrap()).rows[0]
        .easing
        .clone();
    assert_eq!(
        easing,
        jian_ops_schema::motion::Easing::CubicBezier(0.2, 0.0, 0.0, 1.25)
    );
}

#[test]
fn a_selected_keyframe_turns_the_section_into_its_values() {
    let mut summary = summary(serde_json::json!({
        "type": "rectangle", "id": "r", "width": 10, "height": 10, "animations": [fade()]
    }));
    summary.keyframe = Some(op_editor_core::motion_summary::KeyframeSummary {
        animation: 0,
        keyframe: 1,
        count: 2,
        time_ms: 300.0,
        values: vec![("opacity".into(), serde_json::json!(1))],
        addable: vec!["translateY"],
    });
    assert_eq!(
        rows(&summary),
        [
            MotionRowKind::KeyframeHeader,
            MotionRowKind::KeyframeValue(0),
            MotionRowKind::KeyframeAdd,
        ]
    );
    let actions = actions(&summary);
    assert!(actions.contains(&PropertyPanelAction::LeaveMotionKeyframe));
    assert!(actions.contains(&menu(MotionMenuTarget::KeyframeProperty)));
    assert!(
        actions.contains(&edit(MotionPanelEdit::RemoveKeyframeValue {
            animation: 0,
            keyframe: 1,
            property: "opacity".into(),
        }))
    );
    let mut inputs = Vec::new();
    push_motion_input_rects(&mut inputs, &summary, 0.0, 0.0, 280.0);
    assert_eq!(inputs[0].0, PropertyFocus::MotionKeyframeValue(0));
    assert_eq!(field_initial(inputs[0].0, &summary), "1");
    let menu_rows = motion_menu_rows(
        op_editor_core::Locale::EnUs,
        MotionMenuTarget::KeyframeProperty,
        &summary,
    );
    assert_eq!(menu_rows.len(), 1);
    assert_eq!(menu_rows[0].1, "translateY");
    assert_eq!(
        crate::widgets::property_panel_motion_curve::keyframe_title(
            op_editor_core::Locale::EnUs,
            summary.keyframe.as_ref().unwrap()
        ),
        "Keyframe 2 of 2 · 0.30 s"
    );
}

#[test]
fn a_manual_row_offers_the_screens_ids() {
    let mut manual = serde_json::to_value(
        op_editor_core::motion_presets::MotionPreset::FadeIn
            .animation(jian_ops_schema::motion::MotionTrigger::Manual),
    )
    .expect("animation json");
    manual["id"] = serde_json::json!("intro");
    let mut open = expanded(
        summary(serde_json::json!({
            "type": "rectangle", "id": "r", "width": 10, "height": 10, "animations": [manual]
        })),
        0,
    );
    assert!(!actions(&open).contains(&menu(MotionMenuTarget::ManualId(0))));
    open.manual_ids = vec!["intro".into(), "outro".into()];
    assert!(actions(&open).contains(&menu(MotionMenuTarget::ManualId(0))));
    let rows = motion_menu_rows(
        op_editor_core::Locale::EnUs,
        MotionMenuTarget::ManualId(0),
        &open,
    );
    let labels: Vec<&str> = rows.iter().map(|(_, label)| label.as_str()).collect();
    assert_eq!(labels, ["✓ intro", "outro"]);
    assert_eq!(rows[1].0, edit(MotionPanelEdit::SetId(0, "outro".into())));
}

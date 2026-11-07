//! Tests for the Interactions section's data parsing + geometry +
//! menu helpers.

use super::*;
use crate::widgets::property_panel_visibility::SectionCapabilities;
use jian_ops_schema::node::PenNode;
use op_editor_core::interaction_edit::InteractionActionKind;

fn node_from(json: &str) -> PenNode {
    serde_json::from_str(json).expect("test fixture parses")
}

// ── Section capability filtering ────────────────────────────────────

#[test]
fn interactions_capability_is_on_for_every_single_select_kind() {
    use crate::layout_scene::NodeKind as K;
    for kind in [
        K::Frame,
        K::Group,
        K::Rect,
        K::Ellipse,
        K::Polygon,
        K::Line,
        K::Path,
        K::Text,
        K::Other("image".to_string()),
        K::Other("icon_font".to_string()),
        K::Other("ref".to_string()),
        K::Other("widget".to_string()),
    ] {
        assert!(
            SectionCapabilities::for_kind(&kind).interactions,
            "{kind:?} should show the Interactions section"
        );
    }
}

#[test]
fn interactions_capability_is_off_for_multi_select() {
    assert!(!SectionCapabilities::for_multi().interactions);
}

// ── Summary ─────────────────────────────────────────────────────────

#[test]
fn interactions_of_summarizes_every_trigger() {
    let node = node_from(
        r#"{"type":"frame","id":"f1","events":{
            "onTap":[{"push":"\"/a\""},{"pop":null}],
            "onHoverEnter":[{"show":"tip"}]}}"#,
    );
    let summary = interactions_of(&node, false);
    assert_eq!(summary.triggers.len(), 2);
    assert_eq!(summary.param(InteractionTrigger::Tap, 0), Some("/a"));
    assert_eq!(summary.action_count(InteractionTrigger::Tap), 2);
    assert_eq!(
        summary.param(InteractionTrigger::HoverEnter, 0),
        Some("tip")
    );
}

#[test]
fn interactions_of_reports_no_triggers_for_bare_node() {
    let node = node_from(r#"{"type":"frame","id":"f1"}"#);
    let summary = interactions_of(&node, false);
    assert!(summary.triggers.is_empty());
    assert_eq!(summary.screen, None);
}

// ── screen marker, gated by `is_top_level` ──────────────────────────

#[test]
fn interactions_of_shows_screen_only_when_top_level() {
    let node = node_from(r#"{"type":"frame","id":"f1","screen":"/home"}"#);
    assert_eq!(
        interactions_of(&node, true).screen,
        Some("/home".to_string())
    );
    assert_eq!(interactions_of(&node, false).screen, None);
}

#[test]
fn interactions_of_never_shows_screen_for_non_frame() {
    // `screen` isn't a field on Rectangle at all — even `is_top_level`
    // true must not fabricate one.
    let node = node_from(r#"{"type":"rectangle","id":"r1"}"#);
    assert_eq!(interactions_of(&node, true).screen, None);
}

// ── Geometry ────────────────────────────────────────────────────────

fn summary_of(json: &str) -> InteractionSummary {
    interactions_of(&node_from(json), true)
}

fn actions(summary: &InteractionSummary) -> Vec<(PropertyPanelAction, Rect)> {
    let mut out = Vec::new();
    push_interaction_action_rects(&mut out, summary, 0.0, 0.0, 280.0);
    out
}

#[test]
fn empty_state_has_only_the_add_row() {
    let summary = InteractionSummary::default();
    let rects = actions(&summary);
    assert_eq!(rects.len(), 1);
    assert_eq!(
        rects[0].0,
        PropertyPanelAction::ToggleInteractionMenu(InteractionMenuTarget::AddTrigger)
    );
    assert_eq!(
        interactions_section_height(&summary),
        SECTION_HEADER_HEIGHT + ROW_STEP + 6.0 + SECTION_GAP
    );
}

#[test]
fn rows_cover_trigger_action_param_and_add() {
    let summary = summary_of(
        r#"{"type":"frame","id":"f1","screen":"/home",
            "events":{"onTap":[{"push":"\"/a\""},{"toast":"\"hi\""},{"pop":null}]}}"#,
    );
    let rects = actions(&summary);
    let kinds: Vec<_> = rects.iter().map(|(a, _)| a.clone()).collect();
    use InteractionMenuTarget as T;
    let tap = InteractionTrigger::Tap;
    assert_eq!(
        kinds,
        vec![
            PropertyPanelAction::ToggleInteractionMenu(T::Trigger("onTap".into())),
            PropertyPanelAction::EditInteraction(InteractionEdit::AddAction(tap)),
            PropertyPanelAction::ToggleInteractionMenu(T::Action {
                trigger: tap,
                index: 0
            }),
            PropertyPanelAction::EditInteraction(InteractionEdit::RemoveAction {
                trigger: tap,
                index: 0
            }),
            PropertyPanelAction::ToggleInteractionMenu(T::Param {
                trigger: tap,
                index: 0
            }),
            PropertyPanelAction::ToggleInteractionMenu(T::Action {
                trigger: tap,
                index: 1
            }),
            PropertyPanelAction::EditInteraction(InteractionEdit::RemoveAction {
                trigger: tap,
                index: 1
            }),
            PropertyPanelAction::ToggleInteractionMenu(T::Action {
                trigger: tap,
                index: 2
            }),
            PropertyPanelAction::EditInteraction(InteractionEdit::RemoveAction {
                trigger: tap,
                index: 2
            }),
            PropertyPanelAction::ToggleInteractionMenu(T::AddTrigger),
        ]
    );
    // Screen + trigger + 3 actions + 2 params + add = 8 rows.
    assert_eq!(
        interactions_section_height(&summary),
        SECTION_HEADER_HEIGHT + 8.0 * ROW_STEP + 6.0 + SECTION_GAP
    );
    // The toast's text param is an input, not an action.
    let mut inputs = Vec::new();
    push_interaction_input_rects(&mut inputs, &summary, 0.0, 0.0, 280.0);
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].0, PropertyFocus::InteractionParam(tap, 1));
    let add = rects.last().unwrap().1;
    assert!(inputs[0].1.origin.y < add.origin.y);
}

#[test]
fn unknown_trigger_is_read_only_but_removable() {
    let summary = summary_of(
        r#"{"type":"frame","id":"f1","events":{"onPanStart":[{"set":{"$app.x":"1"}}]}}"#,
    );
    let rects = actions(&summary);
    assert!(rects.iter().any(|(a, _)| *a
        == PropertyPanelAction::EditInteraction(InteractionEdit::RemoveTrigger(
            "onPanStart".into()
        ))));
    assert!(!rects.iter().any(|(a, _)| matches!(
        a,
        PropertyPanelAction::ToggleInteractionMenu(InteractionMenuTarget::Action { .. })
    )));
}

// ── Menus ───────────────────────────────────────────────────────────

fn ctx() -> InteractionContext {
    InteractionContext {
        screens: vec!["/home".into(), "/detail".into()],
        own_screen: Some("/home".into()),
        nodes: vec![
            op_editor_core::interaction_edit_apply::InteractionTargetNode {
                id: "sheet".into(),
                name: "Sheet".into(),
                hidden: true,
            },
        ],
    }
}

#[test]
fn add_trigger_menu_skips_present_triggers() {
    let summary = summary_of(r#"{"type":"frame","id":"f1","events":{"onTap":[{"pop":null}]}}"#);
    let rows = interaction_menu_rows(
        op_editor_core::Locale::EnUs,
        &InteractionMenuTarget::AddTrigger,
        &summary,
        &ctx(),
    );
    assert_eq!(rows.len(), InteractionTrigger::ALL.len() - 1);
    assert!(!rows.iter().any(|(a, _)| *a
        == PropertyPanelAction::EditInteraction(InteractionEdit::AddTrigger(
            InteractionTrigger::Tap
        ))));
}

#[test]
fn action_menu_lists_kinds_then_reorder_then_remove() {
    let summary =
        summary_of(r#"{"type":"frame","id":"f1","events":{"onTap":[{"pop":null},{"pop":null}]}}"#);
    let target = InteractionMenuTarget::Action {
        trigger: InteractionTrigger::Tap,
        index: 0,
    };
    let rows = interaction_menu_rows(op_editor_core::Locale::EnUs, &target, &summary, &ctx());
    // 13 kinds + Move down + Remove (first row has no Move up).
    assert_eq!(rows.len(), InteractionActionKind::ALL.len() + 2);
    assert!(
        rows[1].1.starts_with("✓ "),
        "Back is current: {:?}",
        rows[1]
    );
    assert_eq!(rows.last().unwrap().1.trim(), "Remove");
}

#[test]
fn param_menus_list_screens_and_layers() {
    let summary = summary_of(
        r#"{"type":"frame","id":"f1","events":{"onTap":[{"push":"\"/detail\""},{"show":"sheet"}]}}"#,
    );
    let screen_rows = interaction_menu_rows(
        op_editor_core::Locale::EnUs,
        &InteractionMenuTarget::Param {
            trigger: InteractionTrigger::Tap,
            index: 0,
        },
        &summary,
        &ctx(),
    );
    assert_eq!(screen_rows.len(), 2);
    assert!(screen_rows[1].1.contains("✓ /detail"));
    let layer_rows = interaction_menu_rows(
        op_editor_core::Locale::EnUs,
        &InteractionMenuTarget::Param {
            trigger: InteractionTrigger::Tap,
            index: 1,
        },
        &summary,
        &ctx(),
    );
    assert_eq!(layer_rows.len(), 1);
    assert!(layer_rows[0].1.contains("Sheet (hidden)"));
}

#[test]
fn menu_flips_above_when_it_would_overflow() {
    let bounds = Rect::xywh(0.0, 0.0, 300.0, 400.0);
    let anchor = Rect::xywh(20.0, 350.0, 240.0, 32.0);
    let menu = interaction_menu_rect(anchor, 6, bounds);
    assert!(menu.origin.y + menu.size.y <= anchor.origin.y + 0.01);
    let below = interaction_menu_rect(Rect::xywh(20.0, 20.0, 240.0, 32.0), 3, bounds);
    assert_eq!(below.origin.y, 52.0);
}

#[test]
fn menu_row_rects_stack_from_the_menu_top() {
    let rows = vec![
        (PropertyPanelAction::CloseInteractionMenu, "a".to_string()),
        (PropertyPanelAction::CloseInteractionMenu, "b".to_string()),
    ];
    let menu = Rect::xywh(0.0, 100.0, 200.0, 64.0);
    let rects = interaction_menu_row_rects(menu, &rows);
    assert_eq!(rects[0].1.origin.y, 104.0);
    assert_eq!(rects[1].1.origin.y, 104.0 + 28.0);
}

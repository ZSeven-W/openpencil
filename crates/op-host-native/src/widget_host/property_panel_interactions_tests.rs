//! Host-side dispatch tests for the Interactions section's write
//! path — `apply_property_action` + `EditorCommand::PatchNodeData` +
//! undo. Hit-test / popover-row / row-parsing coverage lives in
//! `op-editor-ui`'s `property_panel_interactions_tests.rs`; these
//! tests cover what only the host can exercise (real `EditorState`
//! mutation + history).

use super::WidgetHostNative;
use op_editor_core::interaction_edit::{InteractionActionKind, InteractionTrigger};
use op_editor_core::interaction_edit_apply::{InteractionEdit, InteractionMenuTarget};
use op_editor_core::NodeId;
use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn node_events_json(host: &WidgetHostNative, id: &str) -> serde_json::Value {
    let node =
        op_editor_core::walkers::find_node(host.editor_state().active_children(), &NodeId::new(id))
            .expect("node present");
    serde_json::to_value(node).expect("node serializes")
}

/// Brute-force scan the property rail for a point the open
/// Interactions menu maps to row `want_row` — mirrors the
/// `point_for_action` scan in `property_panel_press_tests.rs`.
fn point_for_menu_row(host: &WidgetHostNative, want_row: usize) -> Point2D {
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let rect = host.property_rect(VIEWPORT_W, VIEWPORT_H);
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x < rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if panel.interaction_menu_row_at(rect, point) == Some(want_row) {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no point maps to interaction-menu row {want_row}");
}

fn update_interaction_hover(host: &mut WidgetHostNative, point: Point2D) -> bool {
    let panel = PropertyPanel::for_selection(host.editor_state());
    host.update_interaction_menu_hover(point.x, point.y, VIEWPORT_W, VIEWPORT_H, panel.as_ref())
}

/// Find a point where an Effects popover row visually covers one of the
/// inspector body's action buttons. This is the exact geometry that used to
/// leave the covered button's stale hover wash visible through the popover.
fn point_where_effect_menu_overlaps_action(host: &WidgetHostNative) -> (Point2D, usize, usize) {
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let rect = host.property_rect(VIEWPORT_W, VIEWPORT_H);
    let popup = panel
        .effect_add_menu_rect(rect)
        .expect("open effect menu has painted bounds");
    let point = Point2D::new(
        popup.origin.x + popup.size.x / 2.0,
        popup.origin.y + popup.size.y / 2.0,
    );
    let row = panel
        .effect_add_menu_row_at(rect, point)
        .expect("popup center is the middle effect row");
    let action = panel
        .action_hover_index(rect, point)
        .expect("middle effect row overlaps an inspector action");
    (point, action, row)
}

const TWO_SCREENS: &str = r#"{ "version": "1.0.0", "children": [
    {"type":"frame","id":"home","name":"Home","screen":"/home","width":300,"height":600,
     "children":[
        {"type":"rectangle","id":"btn","name":"Button","width":100,"height":40},
        {"type":"rectangle","id":"sheet","name":"Sheet","visible":false,"width":300,"height":200}
     ]},
    {"type":"frame","id":"detail","name":"Detail","screen":"/detail","width":300,"height":600}
]}"#;

fn host_with_button() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_SCREENS);
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("btn"));
    host
}

fn scroll_rail_to_bottom(host: &mut WidgetHostNative) {
    let property_rect = host.property_rect(VIEWPORT_W, VIEWPORT_H);
    let _ = host.apply_wheel(
        property_rect.origin.x + property_rect.size.x / 2.0,
        property_rect.origin.y + property_rect.size.y / 2.0,
        -10_000.0,
        VIEWPORT_W,
        VIEWPORT_H,
    );
}

#[test]
fn toggle_interaction_menu_opens_swaps_and_closes() {
    let mut host = host_with_button();
    host.apply_property_action(PropertyPanelAction::ToggleInteractionMenu(
        InteractionMenuTarget::AddTrigger,
    ));
    assert!(host.editor_state().editor_ui.interaction_menu_open);
    let trig = InteractionMenuTarget::Trigger("onTap".into());
    host.apply_property_action(PropertyPanelAction::ToggleInteractionMenu(trig.clone()));
    assert_eq!(
        host.editor_state().editor_ui.interaction_menu_target,
        Some(trig.clone())
    );
    host.apply_property_action(PropertyPanelAction::ToggleInteractionMenu(trig));
    assert!(!host.editor_state().editor_ui.interaction_menu_open);
    assert_eq!(host.editor_state().editor_ui.interaction_menu_target, None);
}

#[test]
fn add_trigger_writes_push_to_the_other_screen_and_undoes_in_one_step() {
    let mut host = host_with_button();
    host.apply_property_action(PropertyPanelAction::EditInteraction(
        InteractionEdit::AddTrigger(InteractionTrigger::Tap),
    ));
    let value = node_events_json(&host, "btn");
    assert_eq!(value["events"]["onTap"][0]["push"], "\"/detail\"");
    assert!(!host.editor_state().editor_ui.interaction_menu_open);

    host.editor_state_mut().undo();
    let value = node_events_json(&host, "btn");
    assert!(
        value.get("events").is_none(),
        "undo restores no events: {value}"
    );
}

#[test]
fn show_layer_action_targets_the_hidden_sibling() {
    let mut host = host_with_button();
    host.apply_property_action(PropertyPanelAction::EditInteraction(
        InteractionEdit::AddTrigger(InteractionTrigger::Tap),
    ));
    host.apply_property_action(PropertyPanelAction::EditInteraction(
        InteractionEdit::SetActionKind {
            trigger: InteractionTrigger::Tap,
            index: 0,
            kind: InteractionActionKind::Show,
        },
    ));
    let value = node_events_json(&host, "btn");
    assert_eq!(value["events"]["onTap"][0]["show"], "sheet");
}

#[test]
fn remove_trigger_preserves_a_sibling_event_handler() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{ "version": "1.0.0", "children": [
            {"type":"frame","id":"f1","name":"Frame",
             "events":{"onTap":[{"pop":null}],"onPanStart":[{"set":{"$state.x":"1"}}]}}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("f1"));
    host.apply_property_action(PropertyPanelAction::EditInteraction(
        InteractionEdit::RemoveTrigger("onTap".into()),
    ));
    let value = node_events_json(&host, "f1");
    assert!(value["events"].get("onTap").is_none());
    assert!(value["events"]["onPanStart"].is_array());
}

#[test]
fn text_param_input_commits_through_the_focus_lifecycle() {
    let mut host = host_with_button();
    host.apply_property_action(PropertyPanelAction::EditInteraction(
        InteractionEdit::AddTrigger(InteractionTrigger::LongPress),
    ));
    host.apply_property_action(PropertyPanelAction::EditInteraction(
        InteractionEdit::SetActionKind {
            trigger: InteractionTrigger::LongPress,
            index: 0,
            kind: InteractionActionKind::Toast,
        },
    ));
    scroll_rail_to_bottom(&mut host);
    let focus = op_editor_core::PropertyFocus::InteractionParam(InteractionTrigger::LongPress, 0);
    let rect = host.property_rect(VIEWPORT_W, VIEWPORT_H);
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("panel");
    // Find the painted input and press it like a user would.
    let mut hit = None;
    let mut y = rect.origin.y;
    'scan: while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x < rect.origin.x + rect.size.x {
            if panel.hit_test(rect, Point2D::new(x, y)) == Some(focus) {
                hit = Some(Point2D::new(x, y));
                break 'scan;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = hit.expect("toast text input is hit-testable");
    host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H);
    assert_eq!(host.editor_state().ui.property_focus, Some(focus));
    assert_eq!(host.editor_state().ui.property_input_draft, "Done");
    host.editor_state_mut()
        .ui
        .property_input
        .set_text("Saved!".to_string());
    host.editor_state_mut().ui.property_input_draft = "Saved!".into();
    host.commit_property_focus_if_any();
    let value = node_events_json(&host, "btn");
    assert_eq!(value["events"]["onLongPress"][0]["toast"], "\"Saved!\"");
}

#[test]
fn interaction_menu_hover_tracks_the_row_under_the_cursor() {
    let mut host = host_with_button();
    scroll_rail_to_bottom(&mut host);
    host.apply_property_action(PropertyPanelAction::ToggleInteractionMenu(
        InteractionMenuTarget::AddTrigger,
    ));
    assert!(host.editor_state().editor_ui.interaction_menu_open);
    assert_eq!(host.editor_state().editor_ui.interaction_menu_hover, None);

    let point = point_for_menu_row(&host, 0);
    assert!(update_interaction_hover(&mut host, point));
    assert_eq!(
        host.editor_state().editor_ui.interaction_menu_hover,
        Some(0)
    );
    assert!(!update_interaction_hover(&mut host, point));
    assert!(update_interaction_hover(
        &mut host,
        Point2D::new(-500.0, -500.0)
    ));
    assert_eq!(host.editor_state().editor_ui.interaction_menu_hover, None);

    // Pressing row 0 ("On tap") writes the trigger and closes the menu.
    host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H);
    assert!(!host.editor_state().editor_ui.interaction_menu_open);
    let value = node_events_json(&host, "btn");
    assert!(value["events"]["onTap"].is_array(), "{value}");
}

#[test]
fn interaction_menu_hover_is_a_no_op_while_closed() {
    let mut host = host_with_button();
    assert!(!host.editor_state().editor_ui.interaction_menu_open);
    assert!(!update_interaction_hover(&mut host, Point2D::new(0.0, 0.0)));
}

#[test]
fn effect_popover_hover_clears_the_covered_inspector_action_hover() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{ "version": "1.0.0", "children": [
            {"type":"frame","id":"f1","name":"Frame"}
        ]}"#,
    );
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("f1"));
    host.apply_property_action(PropertyPanelAction::ToggleEffectAddPicker);

    let (point, covered_action, effect_row) = point_where_effect_menu_overlaps_action(&host);
    host.editor_state_mut().editor_ui.property_action_hover = Some(covered_action);

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().editor_ui.effect_add_menu_hover,
        Some(effect_row),
        "the top popover row should own hover"
    );
    assert_eq!(
        host.editor_state().editor_ui.property_action_hover,
        None,
        "the covered inspector action must not retain a hover wash"
    );

    // Even when the popover's own row is unchanged, it remains an owning
    // floating surface and must clear any stale lower hover before returning.
    host.editor_state_mut().editor_ui.property_action_hover = Some(covered_action);
    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state().editor_ui.property_action_hover, None);
}

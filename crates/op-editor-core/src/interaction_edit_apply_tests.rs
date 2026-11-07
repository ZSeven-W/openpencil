use super::*;
use crate::interaction_edit::summarize_events;
use serde_json::json;

fn ctx() -> InteractionContext {
    InteractionContext {
        screens: vec!["/home".into(), "/detail".into()],
        own_screen: Some("/home".into()),
        nodes: vec![
            InteractionTargetNode {
                id: "title".into(),
                name: "Title".into(),
                hidden: false,
            },
            InteractionTargetNode {
                id: "sheet".into(),
                name: "Sheet".into(),
                hidden: true,
            },
        ],
    }
}

fn map(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn add_trigger_seeds_navigate_to_another_screen() {
    let mut m = Map::new();
    assert!(apply_interaction_edit(
        &mut m,
        &InteractionEdit::AddTrigger(InteractionTrigger::Tap),
        &ctx()
    ));
    assert_eq!(
        Value::Object(m),
        json!({"onTap": [{"push": "\"/detail\""}]})
    );
}

#[test]
fn add_trigger_without_screens_seeds_toast() {
    let mut m = Map::new();
    apply_interaction_edit(
        &mut m,
        &InteractionEdit::AddTrigger(InteractionTrigger::LongPress),
        &InteractionContext::default(),
    );
    assert_eq!(
        Value::Object(m),
        json!({"onLongPress": [{"toast": "\"Done\""}]})
    );
}

#[test]
fn set_kind_reseeds_param_from_context() {
    let mut m = map(json!({"onTap": [{"pop": null}]}));
    apply_interaction_edit(
        &mut m,
        &InteractionEdit::SetActionKind {
            trigger: InteractionTrigger::Tap,
            index: 0,
            kind: InteractionActionKind::Show,
        },
        &ctx(),
    );
    assert_eq!(Value::Object(m), json!({"onTap": [{"show": "sheet"}]}));
}

#[test]
fn set_param_keeps_authored_replace_verb() {
    let mut m = map(json!({"onTap": [{"replace": "\"/home\""}]}));
    apply_interaction_edit(
        &mut m,
        &InteractionEdit::SetActionParam {
            trigger: InteractionTrigger::Tap,
            index: 0,
            value: "/detail".into(),
        },
        &ctx(),
    );
    assert_eq!(
        Value::Object(m),
        json!({"onTap": [{"replace": "\"/detail\""}]})
    );
}

#[test]
fn change_trigger_merges_into_existing_list() {
    let mut m = map(json!({
        "onTap": [{"pop": null}],
        "onDoubleTap": [{"toast": "\"x\""}],
    }));
    apply_interaction_edit(
        &mut m,
        &InteractionEdit::ChangeTrigger {
            from: InteractionTrigger::DoubleTap,
            to: InteractionTrigger::Tap,
        },
        &ctx(),
    );
    assert_eq!(
        Value::Object(m),
        json!({"onTap": [{"pop": null}, {"toast": "\"x\""}]})
    );
}

#[test]
fn removing_last_action_drops_trigger_and_unknown_keys_survive() {
    let mut m = map(json!({
        "onTap": [{"pop": null}],
        "onPanStart": [{"set": {"$app.x": "1"}}],
    }));
    apply_interaction_edit(
        &mut m,
        &InteractionEdit::RemoveAction {
            trigger: InteractionTrigger::Tap,
            index: 0,
        },
        &ctx(),
    );
    assert_eq!(
        Value::Object(m),
        json!({"onPanStart": [{"set": {"$app.x": "1"}}]})
    );
}

#[test]
fn move_action_swaps_and_rejects_out_of_range() {
    let mut m = map(json!({"onTap": [{"pop": null}, {"haptic": {"style": "medium"}}]}));
    let up = InteractionEdit::MoveAction {
        trigger: InteractionTrigger::Tap,
        index: 1,
        up: true,
    };
    assert!(apply_interaction_edit(&mut m, &up, &ctx()));
    assert_eq!(m["onTap"][0], json!({"haptic": {"style": "medium"}}));
    let bad = InteractionEdit::MoveAction {
        trigger: InteractionTrigger::Tap,
        index: 0,
        up: true,
    };
    assert!(!apply_interaction_edit(&mut m, &bad, &ctx()));
}

fn state_with_screens() -> EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [
            {"type": "frame", "id": "home", "screen": "/home", "width": 300, "height": 600,
             "children": [
                {"type": "rectangle", "id": "btn", "name": "Button", "width": 100, "height": 40},
                {"type": "rectangle", "id": "sheet", "name": "Sheet", "visible": false,
                 "width": 300, "height": 200}
             ]},
            {"type": "frame", "id": "detail", "screen": "/detail", "width": 300, "height": 600}
        ]
    }))
    .unwrap();
    let mut s = EditorState::from_document(doc);
    s.selection.anchor = crate::NodeId::new("btn");
    s
}

#[test]
fn context_lists_screens_and_sibling_layers() {
    let s = state_with_screens();
    let c = s.interaction_context();
    assert_eq!(c.screens, vec!["/home", "/detail"]);
    assert_eq!(c.own_screen.as_deref(), Some("/home"));
    assert_eq!(c.nodes.len(), 1);
    assert_eq!(c.nodes[0].id, "sheet");
    assert!(c.nodes[0].hidden);
}

#[test]
fn editor_edit_is_one_undo_step_and_declares_toggle_state() {
    let mut s = state_with_screens();
    assert!(
        s.apply_selected_interaction_edit(&InteractionEdit::AddTrigger(InteractionTrigger::Tap))
    );
    assert!(
        s.apply_selected_interaction_edit(&InteractionEdit::SetActionKind {
            trigger: InteractionTrigger::Tap,
            index: 0,
            kind: InteractionActionKind::ToggleState,
        })
    );
    let views = summarize_events(s.selected_node().unwrap().events());
    assert_eq!(views[0].actions[0].param, "open");
    let state = s.doc.state.as_ref().unwrap();
    assert_eq!(state["open"].default, Some(Value::Bool(false)));

    s.undo();
    let views = summarize_events(s.selected_node().unwrap().events());
    assert_eq!(
        views[0].actions[0].kind,
        Some(InteractionActionKind::Navigate)
    );
    assert_eq!(views[0].actions[0].param, "/detail");
}

#[test]
fn removing_everything_clears_events_field() {
    let mut s = state_with_screens();
    s.apply_selected_interaction_edit(&InteractionEdit::AddTrigger(InteractionTrigger::Tap));
    s.apply_selected_interaction_edit(&InteractionEdit::RemoveTrigger("onTap".into()));
    assert!(s.selected_node().unwrap().events().is_none());
}

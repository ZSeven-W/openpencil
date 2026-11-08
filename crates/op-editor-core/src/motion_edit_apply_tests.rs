use super::*;
use crate::collab_gate::{CollabDocumentMutation, CollabGateAction, CollabUnsupportedFeature};
use crate::motion_edit::MotionRetime;
use crate::motion_timeline_projection::keyframe_ms;
use jian_ops_schema::motion::Easing;
use serde_json::json;

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [{
                "type": "frame", "id": "list", "width": 390, "height": 600,
                "children": [
                    {"type": "rectangle", "id": "a", "width": 10, "height": 10},
                    {"type": "rectangle", "id": "b", "width": 10, "height": 10},
                    {"type": "rectangle", "id": "c", "width": 10, "height": 10}
                ]
            }, {
                "type": "rectangle", "id": "lonely", "width": 10, "height": 10
            }]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    EditorState::from_document(doc)
}

fn id(value: &str) -> NodeId {
    NodeId::new(value)
}

fn animations(state: &EditorState, node: &str) -> Vec<NodeAnimation> {
    find_node(state.active_children(), &id(node))
        .expect("node")
        .motion_declarations()
        .1
        .cloned()
        .unwrap_or_default()
}

fn fade() -> NodeAnimation {
    MotionPreset::FadeIn.animation(MotionTrigger::Mount)
}

#[test]
fn adding_updating_and_removing_are_single_undo_steps() {
    let mut state = state();
    let add = MotionCommand::AddAnimation {
        node_id: id("a"),
        animation: fade(),
        index: None,
    };
    assert_eq!(state.apply_motion_edits(vec![add]), Ok(true));
    assert_eq!(animations(&state, "a"), vec![fade()]);

    let mut slower = fade();
    slower.duration_ms = 900;
    let update = MotionCommand::UpdateAnimation {
        node_id: id("a"),
        index: 0,
        animation: slower.clone(),
    };
    assert_eq!(state.apply_motion_edits(vec![update]), Ok(true));
    assert_eq!(animations(&state, "a")[0].duration_ms, 900);

    let remove = MotionCommand::RemoveAnimation {
        node_id: id("a"),
        index: 0,
    };
    assert_eq!(state.apply_motion_edits(vec![remove]), Ok(true));
    assert!(animations(&state, "a").is_empty());
    // The empty list is cleared from the node, not saved as [].
    let node = find_node(state.active_children(), &id("a")).expect("node");
    assert_eq!(node.motion_declarations().1, None);

    assert!(state.undo());
    assert_eq!(animations(&state, "a"), vec![slower]);
    assert!(state.undo());
    assert_eq!(animations(&state, "a"), vec![fade()]);
    assert!(state.undo());
    assert!(animations(&state, "a").is_empty());
}

#[test]
fn a_refused_list_changes_nothing() {
    let mut state = state();
    let mut broken = fade();
    broken.keyframes.reverse();
    let commands = vec![
        MotionCommand::AddAnimation {
            node_id: id("a"),
            animation: fade(),
            index: None,
        },
        MotionCommand::AddAnimation {
            node_id: id("b"),
            animation: broken,
            index: None,
        },
    ];
    let error = state
        .apply_motion_edits(commands)
        .expect_err("invalid keyframes");
    assert!(
        matches!(error, MotionEditError::Invalid { ref node_id, .. } if node_id.as_str() == "b")
    );
    assert!(animations(&state, "a").is_empty());
    assert_eq!(
        state.check_motion_command(&MotionCommand::RemoveAnimation {
            node_id: id("a"),
            index: 0,
        }),
        Err(MotionEditError::AnimationMissing {
            node_id: id("a"),
            index: 0
        })
    );
    assert_eq!(
        state.check_motion_command(&MotionCommand::SetTransition {
            node_id: id("ghost"),
            transition: None,
        }),
        Err(MotionEditError::NodeMissing(id("ghost")))
    );
}

#[test]
fn presets_apply_to_every_selected_node_and_manual_ones_need_an_id() {
    let mut state = state();
    let preset = |id: Option<&str>, trigger| MotionCommand::ApplyPreset {
        node_ids: vec![NodeId::new("a"), NodeId::new("b")],
        preset: MotionPreset::Pulse,
        trigger,
        id: id.map(str::to_owned),
    };
    assert!(matches!(
        state.apply_motion_edits(vec![preset(None, MotionTrigger::Manual)]),
        Err(MotionEditError::Invalid { .. })
    ));
    assert_eq!(
        state.apply_motion_edits(vec![preset(Some(" beat "), MotionTrigger::Manual)]),
        Ok(true)
    );
    assert_eq!(animations(&state, "b")[0].id.as_deref(), Some("beat"));
    // The same id twice on one node is refused.
    assert!(matches!(
        state.apply_motion_edits(vec![preset(Some("beat"), MotionTrigger::Manual)]),
        Err(MotionEditError::Invalid { .. })
    ));
    // One undo step removes the preset from both nodes.
    assert!(state.undo());
    assert!(animations(&state, "a").is_empty() && animations(&state, "b").is_empty());
}

#[test]
fn stagger_offsets_each_child_by_the_step() {
    let mut state = state();
    let stagger = MotionCommand::Stagger {
        container_id: id("list"),
        preset: MotionPreset::SlideUp,
        trigger: MotionTrigger::InView,
        id: None,
        base_delay_ms: 100,
        step_ms: 60,
    };
    assert_eq!(state.apply_motion_edits(vec![stagger]), Ok(true));
    let delays: Vec<u64> = ["a", "b", "c"]
        .iter()
        .map(|node| animations(&state, node)[0].delay_ms)
        .collect();
    assert_eq!(delays, [100, 160, 220]);
    assert!(animations(&state, "list").is_empty());
    assert_eq!(
        state.check_motion_command(&MotionCommand::Stagger {
            container_id: id("lonely"),
            preset: MotionPreset::FadeIn,
            trigger: MotionTrigger::Mount,
            id: None,
            base_delay_ms: 0,
            step_ms: 60,
        }),
        Err(MotionEditError::NothingToStagger(id("lonely")))
    );
}

#[test]
fn reorder_retime_and_curve_changes_write_in_one_step() {
    let mut state = state();
    let mut spin = MotionPreset::Spin.animation(MotionTrigger::Mount);
    spin.keyframes.insert(
        1,
        jian_ops_schema::motion::Keyframe {
            offset: 0.25,
            values: [("rotation".to_owned(), json!(90))].into_iter().collect(),
            extra: Default::default(),
        },
    );
    let setup = vec![
        MotionCommand::AddAnimation {
            node_id: id("a"),
            animation: fade(),
            index: None,
        },
        MotionCommand::AddAnimation {
            node_id: id("a"),
            animation: spin.clone(),
            index: Some(0),
        },
    ];
    assert_eq!(state.apply_motion_edits(setup), Ok(true));
    assert_eq!(animations(&state, "a")[0], spin);

    let reorder = MotionCommand::ReorderAnimation {
        node_id: id("a"),
        from: 0,
        to: 9,
    };
    let retime = MotionCommand::RetimeMany {
        retimes: vec![MotionRetime {
            node_id: id("a"),
            index: 0,
            delay_ms: 50,
            duration_ms: 600,
        }],
    };
    assert_eq!(state.apply_motion_edits(vec![reorder, retime]), Ok(true));
    let list = animations(&state, "a");
    assert_eq!(list[1], spin);
    assert_eq!((list[0].delay_ms, list[0].duration_ms), (50, 600));

    let time_before = keyframe_ms(&list[1], list[1].keyframes[1].offset);
    let ease = MotionCommand::SetEasingKeepTimes {
        node_id: id("a"),
        index: 1,
        easing: Easing::EaseInOut,
    };
    assert_eq!(state.apply_motion_edits(vec![ease]), Ok(true));
    let eased = &animations(&state, "a")[1];
    assert_eq!(eased.easing, Easing::EaseInOut);
    assert!((keyframe_ms(eased, eased.keyframes[1].offset) - time_before).abs() < 0.5);
}

#[test]
fn a_no_op_list_adds_no_undo_step() {
    let mut state = state();
    let clear = MotionCommand::SetTransition {
        node_id: id("a"),
        transition: None,
    };
    assert_eq!(state.apply_motion_edits(vec![clear]), Ok(false));
    assert!(!state.undo());
}

#[test]
fn collaboration_refuses_motion_edits_like_interaction_parameters() {
    let cmd = EditorCommand::Motion(MotionCommand::SetTransition {
        node_id: id("a"),
        transition: None,
    });
    assert_eq!(
        cmd.collab_gate_action(),
        CollabGateAction::Document(CollabDocumentMutation::Unsupported(
            CollabUnsupportedFeature::UnsupportedNodeProperty
        ))
    );
}

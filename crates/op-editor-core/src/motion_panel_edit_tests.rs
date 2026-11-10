use super::*;
use crate::EditorCommand;
use serde_json::json;

fn state_with_selected(node: &str) -> EditorState {
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [{
                "type": "frame", "id": "list", "width": 390, "height": 600,
                "children": [
                    {"type": "rectangle", "id": "a", "width": 10, "height": 10},
                    {"type": "rectangle", "id": "b", "width": 10, "height": 10}
                ]
            }]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    let mut state = EditorState::from_document(doc);
    assert!(state.apply(EditorCommand::SetSelection {
        node_id: NodeId::new(node),
    }));
    state
}

fn animations(state: &EditorState) -> Vec<NodeAnimation> {
    state
        .selected_node()
        .and_then(|node| node.motion_declarations().1.cloned())
        .unwrap_or_default()
}

#[test]
fn presets_add_rows_that_reorder_and_remove() {
    let mut state = state_with_selected("a");
    assert!(state.apply_selected_motion_edit(&MotionPanelEdit::AddPreset(MotionPreset::FadeIn)));
    assert!(state.apply_selected_motion_edit(&MotionPanelEdit::AddPreset(MotionPreset::Spin)));
    assert_eq!(animations(&state)[1].duration_ms, 1000);
    assert!(state.apply_selected_motion_edit(&MotionPanelEdit::MoveUp(1)));
    assert_eq!(animations(&state)[0].duration_ms, 1000);
    assert!(!state.apply_selected_motion_edit(&MotionPanelEdit::MoveUp(0)));
    assert!(state.apply_selected_motion_edit(&MotionPanelEdit::MoveDown(0)));
    assert!(state.apply_selected_motion_edit(&MotionPanelEdit::Remove(0)));
    assert_eq!(animations(&state).len(), 1);
    assert_eq!(animations(&state)[0].duration_ms, 1000);
}

#[test]
fn switching_to_manual_names_the_animation() {
    let mut state = state_with_selected("a");
    state.apply_selected_motion_edit(&MotionPanelEdit::AddPreset(MotionPreset::Pulse));
    state.apply_selected_motion_edit(&MotionPanelEdit::AddPreset(MotionPreset::Pop));
    assert!(
        state.apply_selected_motion_edit(&MotionPanelEdit::SetTrigger(0, MotionTrigger::Manual))
    );
    assert!(
        state.apply_selected_motion_edit(&MotionPanelEdit::SetTrigger(1, MotionTrigger::Manual))
    );
    let ids: Vec<_> = animations(&state).into_iter().map(|a| a.id).collect();
    assert_eq!(ids, [Some("motion-1".into()), Some("motion-2".into())]);
}

#[test]
fn row_toggles_flip_loop_once_and_fill() {
    let mut state = state_with_selected("a");
    state.apply_selected_motion_edit(&MotionPanelEdit::AddPreset(MotionPreset::FadeIn));
    state.apply_selected_motion_edit(&MotionPanelEdit::ToggleLoop(0));
    assert_eq!(animations(&state)[0].iterations, Iterations::Infinite);
    state.apply_selected_motion_edit(&MotionPanelEdit::ToggleLoop(0));
    assert_eq!(animations(&state)[0].iterations, Iterations::Count(1));
    state.apply_selected_motion_edit(&MotionPanelEdit::ToggleOnce(0));
    assert!(!animations(&state)[0].once);
    state.apply_selected_motion_edit(&MotionPanelEdit::SetFill(0, NodeAnimationFillMode::None));
    assert_eq!(animations(&state)[0].fill_mode, NodeAnimationFillMode::None);
    state.apply_selected_motion_edit(&MotionPanelEdit::SetEasing(0, Easing::EaseIn));
    assert_eq!(animations(&state)[0].easing, Easing::EaseIn);
}

#[test]
fn stagger_and_transition_edits_write_the_selected_container() {
    let mut state = state_with_selected("list");
    assert!(
        state.apply_selected_motion_edit(&MotionPanelEdit::StaggerPreset(MotionPreset::SlideUp))
    );
    let child = crate::walkers::find_node(state.active_children(), &NodeId::new("b"))
        .and_then(|node| node.motion_declarations().1.cloned())
        .expect("staggered child");
    assert_eq!(child[0].delay_ms, STAGGER_STEP_MS);

    assert!(
        !state.apply_selected_motion_edit(&MotionPanelEdit::SetTransitionEasing(Easing::Linear))
    );
    assert!(state.apply_selected_motion_edit(&MotionPanelEdit::AddTransition));
    assert!(state.apply_selected_motion_edit(&MotionPanelEdit::SetTransitionEasing(Easing::Linear)));
    let transition = state
        .selected_node()
        .and_then(|node| node.motion_declarations().0.cloned())
        .expect("transition");
    assert_eq!(
        (transition.duration_ms, transition.easing),
        (DEFAULT_TRANSITION_MS, Easing::Linear)
    );
    assert!(state.apply_selected_motion_edit(&MotionPanelEdit::RemoveTransition));
    assert!(state
        .selected_node()
        .unwrap()
        .motion_declarations()
        .0
        .is_none());
    // Each edit was its own undo step.
    assert!(state.undo());
    assert!(state
        .selected_node()
        .unwrap()
        .motion_declarations()
        .0
        .is_some());
}

#[test]
fn text_fields_write_timing_repeat_and_id() {
    let mut state = state_with_selected("a");
    state.apply_selected_motion_edit(&MotionPanelEdit::AddPreset(MotionPreset::FadeIn));
    state.apply_selected_motion_edit(&MotionPanelEdit::SetDuration(0, 640));
    state.apply_selected_motion_edit(&MotionPanelEdit::SetDelay(0, 90));
    state.apply_selected_motion_edit(&MotionPanelEdit::SetRepeat(0, 0));
    let animation = &animations(&state)[0];
    assert_eq!((animation.duration_ms, animation.delay_ms), (640, 90));
    assert_eq!(animation.iterations, Iterations::Count(1));

    state.apply_selected_motion_edit(&MotionPanelEdit::SetId(0, " intro ".into()));
    assert_eq!(animations(&state)[0].id.as_deref(), Some("intro"));
    state.apply_selected_motion_edit(&MotionPanelEdit::SetTrigger(0, MotionTrigger::Manual));
    // A manual animation cannot lose its id: the edit is refused.
    assert!(!state.apply_selected_motion_edit(&MotionPanelEdit::SetId(0, "  ".into())));
    assert_eq!(animations(&state)[0].id.as_deref(), Some("intro"));

    state.apply_selected_motion_edit(&MotionPanelEdit::AddTransition);
    assert!(
        state.apply_selected_motion_edit_untracked(&MotionPanelEdit::SetTransitionDuration(220))
    );
    let transition = state
        .selected_node()
        .and_then(|node| node.motion_declarations().0.cloned())
        .expect("transition");
    assert_eq!(transition.duration_ms, 220);
    // The untracked write added no undo step of its own: one undo removes
    // the transition the tracked edit added.
    assert!(state.undo());
    assert!(state
        .selected_node()
        .unwrap()
        .motion_declarations()
        .0
        .is_none());
}

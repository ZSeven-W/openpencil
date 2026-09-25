//! Remote (Pending) generator flow: the editor half of the browser's
//! daemon-backed runtime, driven by a thread-local stand-in cache.

use std::cell::RefCell;

use jian_ops_schema::node::PenNode;
use serde_json::json;

use super::*;
use crate::generator::{GeneratorError, GeneratorRequest, GENERATOR_STARTERS};
use crate::{EditorState, NodeId};

thread_local! {
    /// The "daemon answer" the fake remote runner serves, once ready.
    static READY: RefCell<Option<Vec<PenNode>>> = const { RefCell::new(None) };
    /// Every request the fake runner saw while not ready.
    static ASKED: RefCell<Vec<GeneratorRunWireRequest>> = const { RefCell::new(Vec::new()) };
}

fn rect(name: &str) -> PenNode {
    serde_json::from_value(json!({
        "type": "rectangle", "id": "raw", "name": name, "width": 8, "height": 8
    }))
    .expect("rect")
}

fn remote_runner(request: &GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError> {
    if let Some(children) = READY.with(|ready| ready.borrow().clone()) {
        return Ok(children);
    }
    ASKED.with(|asked| {
        asked
            .borrow_mut()
            .push(GeneratorRunWireRequest::from_request(request))
    });
    Err(GeneratorError::Pending)
}

fn local_runner(_: &GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError> {
    Ok(vec![rect("local")])
}

fn reset(ready: Option<Vec<PenNode>>) {
    READY.with(|slot| *slot.borrow_mut() = ready);
    ASKED.with(|asked| asked.borrow_mut().clear());
}

fn last_asked() -> GeneratorRunWireRequest {
    ASKED.with(|asked| asked.borrow().last().cloned().expect("a remote request"))
}

fn starter_state() -> (EditorState, NodeId) {
    let mut state = EditorState::new();
    let id = state
        .insert_generator_starter(&GENERATOR_STARTERS[0], Some(local_runner))
        .expect("inserted");
    (state, id)
}

#[test]
fn pending_is_not_parked_as_an_error_and_leaves_the_document_alone() {
    reset(None);
    let (mut state, id) = starter_state();
    let doc = serde_json::to_string(&state.doc).unwrap();
    let depth = state.history.past.len();
    assert_eq!(
        state.regenerate_generator(&id, Some(remote_runner)),
        Err(GeneratorError::Pending)
    );
    assert_eq!(serde_json::to_string(&state.doc).unwrap(), doc);
    assert_eq!(state.history.past.len(), depth);
    assert!(
        state.ui.generator_error.is_none(),
        "pending is not a failure"
    );
    assert!(state.generator_is_pending(&id));
    assert_eq!(last_asked().generator_id, id.as_str());
}

#[test]
fn remote_result_applies_as_one_undo_step_and_reapply_is_a_no_op() {
    reset(None);
    let (mut state, id) = starter_state();
    let depth = state.history.past.len();
    let _ = state.set_generator_param_input(&id, 2, "4", Some(remote_runner));
    assert!(state.generator_is_pending(&id));
    let asked = last_asked();
    assert_eq!(state.history.past.len(), depth, "nothing written yet");

    reset(Some(vec![rect("a"), rect("b")]));
    let request_id = NodeId::new_opt(&asked.generator_id).unwrap();
    assert_eq!(
        state.apply_remote_generator_result(&request_id, &asked.spec, Some(remote_runner)),
        Ok(true)
    );
    assert_eq!(state.history.past.len(), depth + 1, "exactly one undo step");
    assert!(!state.generator_is_pending(&id));
    let stored = state.generator_spec(&id).unwrap();
    assert_eq!(
        stored.params[2].display_value(),
        "4",
        "the edited param landed"
    );
    let doc = serde_json::to_string(&state.doc).unwrap();

    assert_eq!(
        state.apply_remote_generator_result(&request_id, &asked.spec, Some(remote_runner)),
        Ok(false),
        "a byte-identical re-apply writes nothing"
    );
    assert_eq!(state.history.past.len(), depth + 1);
    assert_eq!(serde_json::to_string(&state.doc).unwrap(), doc);

    assert!(state.undo());
    assert_eq!(
        state.generator_spec(&id).unwrap().params[2].display_value(),
        GENERATOR_STARTERS[0].spec().params[2].display_value(),
        "one undo restores the pre-edit spec"
    );
}

#[test]
fn remote_failure_is_reported_once_it_lands() {
    reset(None);
    let (mut state, id) = starter_state();
    let _ = state.regenerate_generator(&id, Some(remote_runner));
    let asked = last_asked();
    fn failing(_: &GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError> {
        Err(GeneratorError::Remote(
            "generator program failed: boom".into(),
        ))
    }
    assert!(state
        .apply_remote_generator_result(&id, &asked.spec, Some(failing))
        .is_err());
    assert!(!state.generator_is_pending(&id));
    let parked = state.ui.generator_error.as_ref().expect("parked");
    assert_eq!(parked.message, "generator program failed: boom");
}

#[test]
fn a_pending_starter_is_inserted_when_its_result_lands() {
    reset(None);
    let mut state = EditorState::new();
    let depth = state.history.past.len();
    assert_eq!(
        state.insert_generator_starter(&GENERATOR_STARTERS[1], Some(remote_runner)),
        Err(GeneratorError::Pending)
    );
    assert_eq!(state.history.past.len(), depth);
    let asked = last_asked();
    let id = NodeId::new_opt(&asked.generator_id).unwrap();
    assert!(state.generator_is_pending(&id));
    assert!(state.locate_generator_node(&id).is_none());

    reset(Some(vec![rect("one")]));
    assert_eq!(
        state.apply_remote_generator_result(&id, &asked.spec, Some(remote_runner)),
        Ok(true)
    );
    assert_eq!(state.selection.anchor, id);
    assert_eq!(state.history.past.len(), depth + 1);
    assert!(!state.generator_is_pending(&id));
    assert_eq!(
        state.generator_spec(&id).unwrap().starter.as_deref(),
        Some(GENERATOR_STARTERS[1].id)
    );
}

#[test]
fn a_result_for_a_deleted_generator_is_ignored() {
    reset(None);
    let (mut state, id) = starter_state();
    let _ = state.regenerate_generator(&id, Some(remote_runner));
    let asked = last_asked();
    assert!(state.apply(crate::EditorCommand::DeleteNode {
        node_id: id.clone(),
        page_id: None,
    }));
    reset(Some(vec![rect("late")]));
    assert_eq!(
        state.apply_remote_generator_result(&id, &asked.spec, Some(remote_runner)),
        Ok(false)
    );
    assert!(!state.generator_is_pending(&id));
}

#[test]
fn wire_request_and_reply_round_trip() {
    reset(None);
    let (mut state, id) = starter_state();
    let _ = state.regenerate_generator(&id, Some(remote_runner));
    let asked = last_asked();
    let json = serde_json::to_string(&asked).unwrap();
    assert!(json.contains("\"generatorId\""));
    let back: GeneratorRunWireRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(back, asked);

    let ok = parse_generator_run_reply(200, &generator_run_reply_body(&Ok(vec![rect("x")])));
    assert_eq!(ok.result.unwrap().len(), 1);
    assert!(!ok.transient);
    let failed = parse_generator_run_reply(
        422,
        &generator_run_reply_body(&Err(GeneratorError::TimeLimit { budget_ms: 1500 })),
    );
    assert_eq!(
        failed.result,
        Err(GeneratorError::Remote(
            GeneratorError::TimeLimit { budget_ms: 1500 }.to_string()
        ))
    );
    assert!(!failed.transient, "a program failure is deterministic");
    assert!(parse_generator_run_reply(0, "").transient);
    assert!(parse_generator_run_reply(429, r#"{"error":"busy"}"#).transient);
    assert!(parse_generator_run_reply(502, "<html>").result.is_err());
}

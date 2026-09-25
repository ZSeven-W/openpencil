//! `/api/generator/run` round-trips through the real QuickJS runtime.

use op_editor_core::generator::{parse_generator_run_reply, GeneratorSpec, GENERATOR_STARTERS};

use super::*;

fn wire(spec: GeneratorSpec) -> String {
    let starter = &GENERATOR_STARTERS[0];
    serde_json::to_string(&GeneratorRunWireRequest {
        generator_id: "gen1".into(),
        spec,
        frame: starter.frame(0.0, 0.0),
    })
    .expect("serializes")
}

#[test]
fn a_valid_spec_returns_its_children() {
    let (status, body) = serve(&wire(GENERATOR_STARTERS[0].spec()));
    assert_eq!(status, "200 OK", "{body}");
    let reply = parse_generator_run_reply(200, &body);
    let children = reply.result.expect("children");
    assert!(!children.is_empty());
    assert!(!reply.transient);
}

#[test]
fn a_failing_program_is_a_422_with_the_runtime_message() {
    let mut spec = GENERATOR_STARTERS[0].spec();
    spec.program = "throw new Error('boom');".into();
    let (status, body) = serve(&wire(spec));
    assert_eq!(status, "422 Unprocessable Entity");
    let reply = parse_generator_run_reply(422, &body);
    let error = reply.result.expect_err("refused").to_string();
    assert!(error.contains("boom"), "{error}");
    assert!(!reply.transient, "a program failure is not retried");
}

#[test]
fn malformed_and_oversized_bodies_are_refused_before_running() {
    let never = |_: &GeneratorRequest<'_>| -> Result<Vec<PenNode>, GeneratorError> {
        panic!("the runtime must not run")
    };
    let (status, _) = serve_with("{\"generatorId\":1}", never);
    assert_eq!(status, "400 Bad Request");
    let mut spec = GENERATOR_STARTERS[0].spec();
    spec.program = format!("// {}", "x".repeat(MAX_REQUEST_BYTES));
    let (status, body) = serve_with(&wire(spec), never);
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("too large"), "{body}");
}

#[test]
fn every_slot_busy_is_a_429_and_a_freed_slot_serves_again() {
    let slots = AtomicUsize::new(0);
    let held: Vec<_> = (0..MAX_CONCURRENT_RUNS)
        .map(|_| RunSlot::acquire(&slots).expect("free slot"))
        .collect();
    let body = wire(GENERATOR_STARTERS[0].spec());
    let (status, body_text) = serve_counted(&body, &slots, |_| Ok(Vec::new()));
    assert_eq!(status, "429 Too Many Requests");
    assert!(parse_generator_run_reply(429, &body_text).transient);
    drop(held);
    let (status, _) = serve_counted(&body, &slots, |_| Ok(Vec::new()));
    assert_eq!(status, "200 OK");
    assert_eq!(slots.load(Ordering::Acquire), 0, "the slot was released");
}

#[test]
fn a_local_daemon_advertises_the_generators_capability() {
    use crate::web_canvas_server::{handle_web_canvas_request, WebCanvasState};
    let mut state = WebCanvasState::new_with_path(op_editor_core::EditorState::new(), 3100, None);
    let server = handle_web_canvas_request("GET", "/api/mcp/server", "", &mut state);
    let body: serde_json::Value = serde_json::from_str(&server.body).expect("json");
    assert_eq!(body["generators"], true, "{}", server.body);
}

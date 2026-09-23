//! Runtime generation failures degrade to a stock search — once.
//!
//! Config-missing Generate slots keep their visible failure (the
//! placeholder), but a generation that was ATTEMPTED and failed with a
//! degradable error (Busy / Timeout / Upstream / network) re-runs the slot
//! as a search using its query, and the session counts it.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::super::*;
use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::{ImageNode, PenNode};
use jian_ops_schema::sizing::SizingBehavior;
use op_editor_core::agent_settings::{ImageGenProfile, ImageGenProvider, ImageTestStatus};

/// A mock Workbench that answers every request with `reply` and counts the
/// submissions it saw. `None` body → `{"error":"qwen_mode_active"}`.
fn busy_workbench() -> (String, Arc<Mutex<u32>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind mock workbench");
    let port = listener.local_addr().expect("addr").port();
    let submits = Arc::new(Mutex::new(0u32));
    let counter = Arc::clone(&submits);
    std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            let mut seen = Vec::new();
            let mut byte = [0u8; 1];
            while stream.read(&mut byte).map(|n| n == 1).unwrap_or(false) {
                seen.push(byte[0]);
                if seen.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let head = String::from_utf8_lossy(&seen).into_owned();
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.trim().split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            let mut body = vec![0u8; length];
            if length > 0 {
                let _ = stream.read_exact(&mut body);
            }
            if head.starts_with("POST") {
                *counter.lock().unwrap() += 1;
            }
            let body = r#"{"error":"qwen_mode_active"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{}",
                body.len(),
                body,
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (format!("http://127.0.0.1:{port}"), submits)
}

/// An Image node bound to explicit generation (prompt only, no query) —
/// the shape `G(…, "generate")` emits.
fn generate_node(id: &str, prompt: &str, name: &str) -> PenNode {
    PenNode::Image(ImageNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.into()),
            ..Default::default()
        },
        src: "".into(),
        object_fit: None,
        width: Some(SizingBehavior::Number(240.0)),
        height: Some(SizingBehavior::Number(160.0)),
        corner_radius: None,
        effects: None,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
        image_prompt: Some(prompt.into()),
        image_search_query: None,
        video: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: Default::default(),
    })
}

fn workbench_profile(base_url: &str) -> ImageGenProfile {
    ImageGenProfile {
        id: "igp-wb".into(),
        name: "Workbench".into(),
        provider: ImageGenProvider::Workbench,
        api_key: "wb-key".into(),
        model: "Qwen-Image-2.1".into(),
        base_url: Some(base_url.to_string()),
        test_status: ImageTestStatus::Idle,
    }
}

fn state_with_generate_slot(profile: Option<ImageGenProfile>) -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.image_gen_enabled = true;
    if let Some(profile) = profile {
        state.editor_ui.agent_settings.image_gen_profiles = vec![profile];
        state.editor_ui.agent_settings.active_image_gen_profile_id = Some("igp-wb".into());
    }
    state.active_children_mut().clear();
    state.active_children_mut().push(generate_node(
        "gen-1",
        "a watercolor kitten",
        "kitten photo",
    ));
    state
}

/// Drive `poll_into` until the session drains or `budget` elapses.
fn drain(session: &mut ImageSearchSession, state: &mut EditorState, budget_ms: u64) {
    let deadline = std::time::Instant::now() + Duration::from_millis(budget_ms);
    while session.is_pending() && std::time::Instant::now() < deadline {
        session.poll_into(state);
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn node_src(state: &EditorState, id: &str) -> String {
    walkers::find_node(state.active_children(), &NodeId::new(id.to_string()))
        .and_then(|node| match node {
            PenNode::Image(image) => Some(image.src.as_str().to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn a_busy_generation_falls_back_to_search_for_the_same_slot() {
    let (base, _submits) = busy_workbench();
    let mut state = state_with_generate_slot(Some(workbench_profile(&base)));
    let mut session = ImageSearchSession::new();
    // Pre-seed the memo so the fallback search resolves deterministically
    // with no network: one Ready photo for the slot's query intent.
    let seeded = "data:image/png;base64,SEARBRA==".to_string();
    session.search_memo.lock().unwrap().insert(
        search_intent_key("kitten photo", Some(ImageAspectRatio::Wide)),
        SearchMemoEntry::Ready(seeded.clone()),
    );

    assert!(session.enqueue_missing(&state));
    drain(&mut session, &mut state, 8_000);

    assert_eq!(
        node_src(&state, "gen-1"),
        seeded,
        "the slot's final src must come from the fallback search"
    );
    let stats = session.stats();
    assert_eq!(stats.fell_back, 1, "exactly one fallback must happen");
    assert_eq!(stats.searched, 1);
    assert_eq!(stats.generated, 0);
}

#[test]
fn a_config_missing_generate_slot_fails_visibly_without_a_search() {
    let mut state = state_with_generate_slot(None);
    let mut session = ImageSearchSession::new();

    assert!(session.enqueue_missing(&state));
    // The unavailable-gen job is synchronous: one poll settles it.
    session.poll_into(&mut state);

    assert!(
        is_image_fallback(
            walkers::find_node(state.active_children(), &NodeId::new("gen-1".to_string()))
                .expect("node")
        ),
        "the slot must land the visible placeholder, not a stock photo"
    );
    let stats = session.stats();
    assert_eq!(stats.fell_back, 0, "config missing must NOT degrade");
    assert_eq!(
        stats.searched, 0,
        "no search job may run for a config fault"
    );
    assert!(!session.is_pending());
}

#[test]
fn the_fallback_happens_once_and_never_retries_generation() {
    let (base, submits) = busy_workbench();
    let mut state = state_with_generate_slot(Some(workbench_profile(&base)));
    let mut session = ImageSearchSession::new();
    // The fallback search answers from the memo; whatever it returns, the
    // slot is finished — the property under test is that the session never
    // goes BACK to the (still-busy) generator for a second attempt.
    let seeded = "data:image/png;base64,ONCEONLY==".to_string();
    session.search_memo.lock().unwrap().insert(
        search_intent_key("kitten photo", Some(ImageAspectRatio::Wide)),
        SearchMemoEntry::Ready(seeded.clone()),
    );

    assert!(session.enqueue_missing(&state));
    drain(&mut session, &mut state, 8_000);
    // Extra rescan + drain: applying the result bumped the revision, so a
    // fresh walk runs — a completed slot must not re-enqueue generation.
    session.enqueue_missing(&state);
    drain(&mut session, &mut state, 8_000);

    assert_eq!(node_src(&state, "gen-1"), seeded);
    let stats = session.stats();
    assert_eq!(stats.fell_back, 1, "one slot, one fallback");
    assert_eq!(
        *submits.lock().unwrap(),
        1,
        "the machine saw exactly one submission — no retry back to generation"
    );
}

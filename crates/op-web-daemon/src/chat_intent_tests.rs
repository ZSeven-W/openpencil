//! Tests for the CLI standard-mode intent router (GAP #33).

use std::sync::mpsc;
use std::time::Duration;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, StopReason};
use op_editor_core::EditorState;
use op_orchestrator::DesignRequest;

use super::*;
use crate::chat_canvas_tools::{apply_design_modification, chat_tool_channel};

// ---------------------------------------------------------------------------
// Keyword + tag classification
// ---------------------------------------------------------------------------

#[test]
fn keyword_classifier_matches_ts_rule_order() {
    // MODIFY keywords win unless a CHAT keyword fires without one.
    assert_eq!(
        classify_by_keywords("change the title color"),
        DesignIntent::Modify
    );
    assert_eq!(
        classify_by_keywords("make it smaller please"),
        DesignIntent::Modify
    );
    assert_eq!(
        classify_by_keywords("add to the header"),
        DesignIntent::Modify
    );
    // CHAT keyword without a modify keyword.
    assert_eq!(classify_by_keywords("what is a frame?"), DesignIntent::Chat);
    assert_eq!(
        classify_by_keywords("explain auto layout"),
        DesignIntent::Chat
    );
    // CHAT + MODIFY both present → modify (TS: chat requires !modify).
    assert_eq!(
        classify_by_keywords("can you fix the button"),
        DesignIntent::Modify
    );
    // Neither → new.
    assert_eq!(
        classify_by_keywords("design a login page"),
        DesignIntent::New
    );
    assert_eq!(classify_by_keywords("a pricing table"), DesignIntent::New);
}

#[test]
fn keyword_phrases_respect_word_boundaries() {
    // "moved" must not match \bmove\b; "exchange" must not match change.
    assert_eq!(
        classify_by_keywords("the moved exchange"),
        DesignIntent::New
    );
    // Multi-word phrase across extra whitespace still matches.
    assert_eq!(
        classify_by_keywords("what   is a vector?"),
        DesignIntent::Chat
    );
}

#[test]
fn classification_tag_parsing_matches_ts() {
    assert_eq!(parse_classified("DESIGN_MODIFY"), DesignIntent::Modify);
    assert_eq!(parse_classified("  design_modify  "), DesignIntent::Modify);
    assert_eq!(parse_classified("DESIGN_NEW"), DesignIntent::New);
    // Bare DESIGN counts as new (TS `upper.includes('DESIGN')`).
    assert_eq!(parse_classified("This is a DESIGN task"), DesignIntent::New);
    assert_eq!(parse_classified("CHAT"), DesignIntent::Chat);
    // Unknown / empty → new.
    assert_eq!(parse_classified("gibberish"), DesignIntent::New);
    assert_eq!(parse_classified(""), DesignIntent::New);
}

// ---------------------------------------------------------------------------
// Scripted provider
// ---------------------------------------------------------------------------

struct Scripted {
    deltas: Vec<ChatDelta>,
    delay: Duration,
}

impl Scripted {
    fn text(s: &str) -> Self {
        Self {
            deltas: vec![
                ChatDelta::TextDelta(s.to_string()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
            delay: Duration::ZERO,
        }
    }

    fn error(msg: &str) -> Self {
        Self {
            deltas: vec![
                ChatDelta::Error(msg.to_string()),
                ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                },
            ],
            delay: Duration::ZERO,
        }
    }

    fn slow(s: &str, delay: Duration) -> Self {
        let mut p = Self::text(s);
        p.delay = delay;
        p
    }
}

impl ChatProvider for Scripted {
    fn provider_label(&self) -> &str {
        "scripted"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let deltas = self.deltas.clone();
        let delay = self.delay;
        Box::new(deltas.into_iter().inspect(move |_| {
            std::thread::sleep(delay);
        }))
    }
}

// ---------------------------------------------------------------------------
// LLM classification
// ---------------------------------------------------------------------------

#[test]
fn llm_classifier_parses_provider_reply() {
    let provider = Scripted::text("DESIGN_MODIFY");
    assert_eq!(
        classify_intent_llm(&provider, "make it red", None),
        DesignIntent::Modify
    );
    let provider = Scripted::text("CHAT");
    assert_eq!(
        classify_intent_llm(&provider, "what is a frame", None),
        DesignIntent::Chat
    );
}

#[test]
fn llm_classifier_falls_back_to_new_on_error() {
    // TS: classify failure → { intent: 'new' }.
    let provider = Scripted::error("boom");
    assert_eq!(
        classify_intent_llm(&provider, "anything", None),
        DesignIntent::New
    );
}

#[test]
fn llm_classifier_falls_back_to_new_on_timeout() {
    let provider = Scripted::slow("CHAT", Duration::from_millis(300));
    let got =
        classify_intent_llm_with_timeout(&provider, "anything", None, Duration::from_millis(30));
    assert_eq!(got, DesignIntent::New, "timeout mirrors the TS abort → new");
}

// ---------------------------------------------------------------------------
// Fixture nodes
// ---------------------------------------------------------------------------

/// Build a canonical frame node from JSON — immune to schema field
/// growth, and exactly the shape `parse_nodes` consumes.
fn frame(id: &str, name: &str, width: f64, children: Vec<PenNode>) -> PenNode {
    let mut node: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": id,
        "name": name,
        "x": 0.0,
        "y": 0.0,
        "width": width,
        "height": 800.0,
        "children": [],
    }))
    .expect("valid frame json");
    if let Some(kids) = node.children_mut() {
        *kids = children;
    }
    node
}

fn rect(id: &str, name: &str) -> PenNode {
    serde_json::from_value(serde_json::json!({
        "type": "rectangle",
        "id": id,
        "name": name,
        "x": 0.0,
        "y": 0.0,
        "width": 120.0,
        "height": 40.0,
    }))
    .expect("valid rectangle json")
}

/// Page frame with a status bar + two content sections.
fn state_with_page() -> EditorState {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "page-1",
        "Home",
        375.0,
        vec![
            frame("sb", "Status Bar", 375.0, vec![]),
            frame("hero", "Hero", 375.0, vec![]),
            frame("features", "Features", 375.0, vec![]),
        ],
    ));
    state
}

// ---------------------------------------------------------------------------
// Append-intent detection
// ---------------------------------------------------------------------------

#[test]
fn append_intent_requires_a_keyword() {
    let state = state_with_page();
    assert!(detect_append_intent(&state, "make a dashboard").is_none());
    assert!(detect_append_intent(&state, "").is_none());
}

#[test]
fn append_intent_detects_and_filters_status_bar() {
    let state = state_with_page();
    let ctx = detect_append_intent(&state, "continue with a pricing section").expect("append");
    assert_eq!(ctx.target_parent_id, "page-1");
    assert_eq!(ctx.target_width, 375.0);
    assert_eq!(
        ctx.existing_section_labels,
        vec!["Hero".to_string(), "Features".to_string()],
        "status-bar sections are filtered from the labels"
    );
    assert!(ctx.is_mobile, "375px page is mobile (≤480)");
}

#[test]
fn append_intent_prefers_a_content_named_root() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "page-1",
        "Home",
        1200.0,
        vec![
            frame("sb", "Status Bar", 1200.0, vec![]),
            frame(
                "content-1",
                "Main Content",
                1100.0,
                vec![
                    frame("hero", "Hero", 1100.0, vec![]),
                    frame("about", "About", 1100.0, vec![]),
                ],
            ),
        ],
    ));
    let ctx = detect_append_intent(&state, "also add a testimonials section").expect("append");
    assert_eq!(ctx.target_parent_id, "content-1");
    assert_eq!(ctx.target_width, 1100.0);
    assert_eq!(
        ctx.existing_section_labels,
        vec!["Hero".to_string(), "About".to_string()]
    );
    assert!(!ctx.is_mobile);
}

#[test]
fn append_intent_vetoed_by_new_screen_phrases() {
    let state = state_with_page();
    assert!(
        detect_append_intent(&state, "continue, but on a new page").is_none(),
        "new-screen phrasing suppresses append mode"
    );
    assert!(detect_append_intent(&state, "继续，做一个全新设计").is_none());
}

#[test]
fn append_intent_needs_existing_content() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    // Page frame whose only child is a status bar — no real content.
    state.active_children_mut().push(frame(
        "page-1",
        "Home",
        375.0,
        vec![frame("sb", "status_bar", 375.0, vec![])],
    ));
    assert!(detect_append_intent(&state, "continue the design").is_none());
    // No page frame at all.
    let mut empty = EditorState::new();
    empty.active_children_mut().clear();
    assert!(detect_append_intent(&empty, "continue the design").is_none());
}

#[test]
fn append_intent_matches_cjk_phrases() {
    let state = state_with_page();
    assert!(detect_append_intent(&state, "再加一个定价区块").is_some());
    assert!(detect_append_intent(&state, "接着补充内容").is_some());
}

#[test]
fn status_bar_matcher_covers_separator_runs() {
    assert!(is_status_bar_like_text("Status Bar"));
    assert!(is_status_bar_like_text("status_bar"));
    assert!(is_status_bar_like_text("status--bar"));
    assert!(is_status_bar_like_text("System  Chrome"));
    assert!(is_status_bar_like_text("状态栏"));
    assert!(!is_status_bar_like_text("Status Mention"));
}

// ---------------------------------------------------------------------------
// Variable context + modify plan
// ---------------------------------------------------------------------------

fn seed_variables(state: &mut EditorState) {
    use jian_ops_schema::variable::{
        ThemedValue, VariableDefinition, VariableKind, VariableScalar, VariableValue,
    };
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(
        "color-1".to_string(),
        VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#112233".into()),
                    theme: None,
                },
                ThemedValue {
                    value: VariableScalar::Str("#aabbcc".into()),
                    theme: None,
                },
            ]),
        },
    );
    vars.insert(
        "spacing-1".to_string(),
        VariableDefinition {
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(8.0)),
        },
    );
    state.doc.variables = Some(vars);
    let mut themes = std::collections::BTreeMap::new();
    themes.insert("Theme-1".to_string(), vec!["Light".into(), "Dark".into()]);
    state.doc.themes = Some(themes);
}

#[test]
fn variable_context_matches_ts_format() {
    let mut state = EditorState::new();
    assert!(build_variable_context(&state).is_none());
    seed_variables(&mut state);
    let ctx = build_variable_context(&state).expect("variables present");
    assert!(ctx.starts_with(
        "DOCUMENT VARIABLES (use \"$name\" to reference, e.g. fill color \"$color-1\"):"
    ));
    assert!(ctx.contains("  - color-1 (color): #112233 [themed]"));
    assert!(ctx.contains("  - spacing-1 (number): 8"));
    assert!(ctx.contains("Themes: Theme-1: [Light, Dark]"));
}

#[test]
fn modify_plan_targets_selection_when_present() {
    let mut state = state_with_page();
    state.selection.set = vec![op_editor_core::NodeId::new("page-1")];
    state.selection.anchor = op_editor_core::NodeId::new("page-1");
    let plan = build_modify_plan(&state, "make it red").expect("plan");
    assert!(plan.user_message.starts_with("CONTEXT NODES:\n"));
    assert!(plan.user_message.contains("\"id\":\"page-1\""));
    assert!(plan.user_message.contains("\n\nINSTRUCTION:\nmake it red"));
}

#[test]
fn modify_plan_falls_back_to_last_frame_then_last_child() {
    // No selection → last top-level frame.
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(frame("f1", "One", 375.0, vec![]));
    state
        .active_children_mut()
        .push(frame("f2", "Two", 375.0, vec![]));
    state.active_children_mut().push(rect("r1", "Loose"));
    let plan = build_modify_plan(&state, "tweak").expect("plan");
    assert!(plan.user_message.contains("\"id\":\"f2\""));
    assert!(!plan.user_message.contains("\"id\":\"f1\""));

    // No frames at all → last child.
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(rect("r1", "Only"));
    let plan = build_modify_plan(&state, "tweak").expect("plan");
    assert!(plan.user_message.contains("\"id\":\"r1\""));
}

#[test]
fn modify_plan_appends_variable_context() {
    let mut state = state_with_page();
    seed_variables(&mut state);
    let plan = build_modify_plan(&state, "recolor with variables").expect("plan");
    assert!(plan.user_message.contains("DOCUMENT VARIABLES"));
    assert!(
        !plan.system_prompt.is_empty(),
        "maintenance skills resolve into the system prompt"
    );
}

#[test]
fn modify_plan_is_none_for_an_empty_page() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    assert!(build_modify_plan(&state, "tweak").is_none());
}

// ---------------------------------------------------------------------------
// apply_design_modification (extractAndApplyDesignModification port)
// ---------------------------------------------------------------------------

#[test]
fn apply_modification_updates_existing_and_inserts_new() {
    let mut state = state_with_page();
    let nodes = vec![
        // Existing id → update in place.
        serde_json::json!({ "id": "hero", "name": "Hero Updated" }),
        // Unknown id → insert under the primary frame (canonical
        // TextNode carries `content`).
        serde_json::json!({
            "id": "fresh-1",
            "type": "text",
            "name": "New Caption",
            "content": "Hello",
        }),
    ];
    let (count, mutated) = apply_design_modification(&mut state, &nodes);
    assert_eq!(count, 2, "both nodes applied");
    assert!(mutated);
    let doc = serde_json::to_string(&state.doc).unwrap();
    assert!(doc.contains("Hero Updated"), "existing node updated");
    assert!(doc.contains("New Caption"), "new node inserted");
    // The insert landed inside the page frame, not at the page root.
    let page = state
        .active_children()
        .iter()
        .find(|n| n.id_str() == "page-1")
        .unwrap();
    let kids = page.children().unwrap();
    assert!(
        kids.iter()
            .any(|k| k.base().name.as_deref().is_some_and(|n| n == "New Caption")),
        "implied-new node parents to the active page's primary frame"
    );
}

// ---------------------------------------------------------------------------
// run_cli_turn routing
// ---------------------------------------------------------------------------

fn test_design_request() -> DesignRequest {
    DesignRequest {
        prompt: "p".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
    }
}

fn drain_chat(rx: &mpsc::Receiver<ChatDelta>) -> Vec<ChatDelta> {
    let mut out = Vec::new();
    while let Ok(delta) = rx.recv_timeout(Duration::from_secs(10)) {
        let done = matches!(delta, ChatDelta::Done { .. });
        out.push(delta);
        if done {
            break;
        }
    }
    out
}

#[test]
fn cli_turn_chat_route_streams_provider_deltas() {
    let plan = CliTurnPlan {
        user_text: "what is a frame?".into(),
        page_children_empty: false,
        classify_provider: Box::new(Scripted::text("CHAT")),
        chat_provider: Box::new(Scripted::text("a frame is a container")),
        design_provider: Box::new(Scripted::text("unused")),
        chat_request: ChatRequest::default(),
        modify_request: Some(ChatRequest::default()),
        design_request: test_design_request(),
        initial_state: EditorState::new(),
        indicator_epoch: 0,
        model: None,
    };
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, _tool_rx) = chat_tool_channel();
    let (delta_tx, delta_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let worker =
        std::thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));
    let deltas = drain_chat(&chat_rx);
    worker.join().unwrap();
    assert_eq!(
        deltas,
        vec![
            ChatDelta::TextDelta("a frame is a container".into()),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            },
        ]
    );
    // Design channels were dropped — the design pump would retire.
    assert!(delta_rx.recv().is_err());
    assert!(cmd_rx.recv().is_err());
}

#[test]
fn cli_turn_modify_route_applies_nodes_and_marks_applied() {
    let response = r##"[{"id":"hero","type":"frame","name":"Hero Red","fill":[{"type":"solid","color":"#ff0000"}]}]"##;
    let plan = CliTurnPlan {
        user_text: "make the hero red".into(),
        page_children_empty: false,
        classify_provider: Box::new(Scripted::text("DESIGN_MODIFY")),
        chat_provider: Box::new(Scripted::text("unused")),
        design_provider: Box::new(Scripted::text(response)),
        chat_request: ChatRequest::default(),
        modify_request: Some(ChatRequest::default()),
        design_request: test_design_request(),
        initial_state: EditorState::new(),
        indicator_epoch: 0,
        model: None,
    };
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, tool_rx) = chat_tool_channel();
    let (delta_tx, delta_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let worker =
        std::thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));

    // Act as the UI pump: execute the internal apply op.
    let req = tool_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("modify route forwards the apply op");
    assert_eq!(req.name, APPLY_MODIFICATION_OP);
    let nodes = serde_json::from_str::<serde_json::Value>(&req.args_json)
        .unwrap()
        .get("nodes")
        .and_then(|n| n.as_array().cloned())
        .unwrap();
    let mut state = state_with_page();
    let (count, mutated) = apply_design_modification(&mut state, &nodes);
    assert_eq!(count, 1);
    assert!(mutated);
    req.ack
        .send(op_ai::chat_provider::ChatToolResult {
            content: serde_json::json!({ "success": true, "count": count }).to_string(),
            is_error: false,
        })
        .unwrap();

    let deltas = drain_chat(&chat_rx);
    worker.join().unwrap();
    // Step → raw response → APPLIED marker → Done.
    assert_eq!(
        deltas[0],
        ChatDelta::TextDelta(
            r#"<step title="Checking guidelines">Analyzing modification request...</step>"#.into()
        )
    );
    assert_eq!(deltas[1], ChatDelta::TextDelta(format!("\n{response}")));
    assert_eq!(
        deltas[2],
        ChatDelta::TextDelta("\n\n<!-- APPLIED -->".into())
    );
    assert!(matches!(deltas[3], ChatDelta::Done { .. }));
    // The doc was recolored through the apply path.
    let doc = serde_json::to_string(&state.doc).unwrap().to_lowercase();
    assert!(doc.contains("#ff0000"));
    // Design channels dropped.
    assert!(delta_rx.recv().is_err());
    assert!(cmd_rx.recv().is_err());
}

#[test]
fn cli_turn_modify_parse_failure_surfaces_ts_error() {
    let plan = CliTurnPlan {
        user_text: "make the hero red".into(),
        page_children_empty: false,
        classify_provider: Box::new(Scripted::text("DESIGN_MODIFY")),
        chat_provider: Box::new(Scripted::text("unused")),
        design_provider: Box::new(Scripted::text("sorry, I cannot help with that")),
        chat_request: ChatRequest::default(),
        modify_request: Some(ChatRequest::default()),
        design_request: test_design_request(),
        initial_state: EditorState::new(),
        indicator_epoch: 0,
        model: None,
    };
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, _tool_rx) = chat_tool_channel();
    let (delta_tx, cmd_tx) = (mpsc::channel().0, mpsc::channel().0);
    let worker =
        std::thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));
    let deltas = drain_chat(&chat_rx);
    worker.join().unwrap();
    let error = deltas
        .iter()
        .find_map(|d| match d {
            ChatDelta::Error(msg) => Some(msg.clone()),
            _ => None,
        })
        .expect("parse failure surfaces an error");
    assert!(
        error.starts_with("Could not parse design nodes from model response. Model output: "),
        "got: {error}"
    );
}

#[test]
fn route_resolution_degrades_modify_like_ts() {
    use DesignIntent::*;
    // TS: modify on an empty page → new.
    assert_eq!(resolve_route(Modify, true, true), New);
    // Belt-and-braces: modify without a usable target plan → new.
    assert_eq!(resolve_route(Modify, false, false), New);
    // Healthy modify survives.
    assert_eq!(resolve_route(Modify, false, true), Modify);
    // Chat / new pass through untouched.
    assert_eq!(resolve_route(Chat, true, false), Chat);
    assert_eq!(resolve_route(New, false, true), New);
}

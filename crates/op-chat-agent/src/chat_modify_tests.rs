use super::*;
use op_ai::chat_provider::ChatToolResult;
use std::sync::mpsc;
use std::time::Duration;

struct EditProvider;
impl ChatProvider for EditProvider {
    fn provider_label(&self) -> &str {
        "fixture"
    }
    fn send(&self, _: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(
            vec![
                ChatDelta::TextDelta(
                    r#"I(null,{"id":"title","type":"text","content":"weekend coffee"});"#.into(),
                ),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]
            .into_iter(),
        )
    }
}

#[test]
fn rejected_zero_or_malformed_acknowledgements_never_claim_an_applied_edit() {
    for (content, is_error) in [
        (r#"{"success":true,"count":1}"#, true),
        (r#"{"success":false,"count":1}"#, false),
        (r#"{"success":true,"count":0}"#, false),
        (r#"{"success":true}"#, false),
        (r#"{"count":1}"#, false),
        ("not JSON", false),
    ] {
        let (chat_tx, chat_rx) = mpsc::channel();
        let (executor, tool_rx) = crate::chat_canvas_tools::chat_tool_channel();
        let worker = std::thread::spawn(move || {
            run_modify_turn(
                &EditProvider,
                ChatRequest::default(),
                &chat_tx,
                &executor,
                vec!["board".into()],
            )
        });
        let request = tool_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        request
            .ack
            .send(ChatToolResult {
                content: content.into(),
                is_error,
            })
            .unwrap();
        let mut deltas = Vec::new();
        loop {
            let delta = chat_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            let done = matches!(delta, ChatDelta::Done { .. });
            deltas.push(delta);
            if done {
                break;
            }
        }
        worker.join().unwrap();
        assert!(
            deltas.iter().any(|d| matches!(d, ChatDelta::Error(_))),
            "{content}"
        );
        assert!(
            matches!(
                deltas.last(),
                Some(ChatDelta::Done {
                    stop_reason: StopReason::Aborted
                })
            ),
            "{content}"
        );
        assert!(!deltas.iter().any(|d| matches!(d, ChatDelta::TextDelta(text) if text.contains("I(null") || text.contains("APPLIED"))), "{content}");
    }
}

#[test]
fn rejected_or_nonexecuted_javascript_cannot_resurrect_literal_nodes() {
    for response in [
        r#"I(null,{"type":"text","id":"title","content":"wrong"}); throw new Error("stop");"#,
        r#"if (false) I(null,{"type":"text","id":"title","content":"wrong"});"#,
        r#"const unused = {"type":"text","id":"title","content":"wrong"};"#,
        "```js\nI(null,{\"type\":\"text\",\"content\":\"wrong\"}); throw new Error(\"stop\");\n```",
    ] {
        assert!(parse_modify_nodes(response).is_empty(), "{response}");
    }
    let oversized = format!(
        "[{{\"type\":\"text\",\"content\":\"{}\"}}]",
        "x".repeat(op_mcp::script_runner::MAX_SCRIPT_BYTES)
    );
    assert!(
        parse_modify_nodes(&oversized).is_empty(),
        "pure JSON also respects the response-size limit"
    );
}

#[test]
fn a_scoped_height_edit_can_still_change_authored_geometry() {
    let mut state = EditorState::from_document(serde_json::from_value(serde_json::json!({
        "version":"1.0.0", "children":[{
            "type":"frame","id":"board","width":390,"height":844,
            "children":[{"type":"text","id":"title","name":"Hero title","content":"Coffee","height":40}]
        }]
    })).unwrap());
    let operations = parse_modify_nodes(
        r#"I(null,{"type":"text","id":"title","name":"Hero title","content":"Coffee","height":72});"#,
    );
    let (count, changed) = crate::chat_canvas_tools::apply_design_modification(
        &mut state,
        &operations,
        &["board".into()],
    );
    assert_eq!((count, changed), (1, true));
    let title = state.active_children()[0].children().unwrap()[0].clone();
    assert_eq!(
        serde_json::to_value(title).unwrap()["height"].as_f64(),
        Some(72.0)
    );
}

#[test]
fn standalone_json_and_jsonl_keep_the_legacy_modification_route() {
    for response in [
        r#"[{"type":"text","id":"title","content":"correct"}]"#,
        "```json\n[{\"type\":\"text\",\"id\":\"title\",\"content\":\"correct\"}]\n```",
        "{\"type\":\"text\",\"id\":\"a\",\"content\":\"A\"}\n{\"type\":\"text\",\"id\":\"b\",\"content\":\"B\"}",
    ] {
        assert!(!parse_modify_nodes(response).is_empty(), "{response}");
    }
}

#[test]
fn a_malformed_nested_icon_rejects_the_entire_script_before_apply() {
    let result = parse_modify_response(
        r#"
        I(null,{type:"text",id:"valid",content:"first edit"});
        I(null,{type:"frame",id:"board",children:[{type:"icon_font",id:"broken"}]});
    "#,
    );
    assert!(result.nodes.is_empty(), "no partial script writes");
    assert!(result.diagnostic.unwrap().contains("iconFontName"));
}

#[test]
fn preflight_accepts_host_assigned_ids_and_the_canonical_mcp_dialect() {
    let parsed = parse_modify_response(
        r#"I(null,{type:"frame",children:[
        {type:"text",content:"步骤 1"},
        {type:"icon_font",iconFontName:"arrow-right",width:24,height:24},
        {type:"frame",layout:{type:"vertical"},children:[{type:"text",content:"说明"}]}
    ]});"#,
    );
    assert!(!parsed.nodes.is_empty(), "{:?}", parsed.diagnostic);
    assert!(
        parsed.nodes[0].1.get("id").is_none(),
        "preflight assigns no live ids"
    );
}

#[test]
fn invalid_node_schema_gets_one_feedback_retry_and_one_host_apply() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };
    struct RetryProvider {
        calls: AtomicUsize,
        feedback: Mutex<String>,
    }
    impl ChatProvider for RetryProvider {
        fn provider_label(&self) -> &str {
            "schema-retry-fixture"
        }
        fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            let response = if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                r#"I(null,{type:"frame",id:"board",children:[{type:"icon_font",id:"bad"}]});"#
            } else {
                *self.feedback.lock().unwrap() = request.user_message;
                r#"I(null,{type:"text",id:"title",content:"corrected"});"#
            };
            Box::new(
                vec![
                    ChatDelta::TextDelta(response.into()),
                    ChatDelta::Done {
                        stop_reason: StopReason::EndTurn,
                    },
                ]
                .into_iter(),
            )
        }
    }
    let provider = Arc::new(RetryProvider {
        calls: AtomicUsize::new(0),
        feedback: Mutex::new(String::new()),
    });
    let worker_provider = Arc::clone(&provider);
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, tool_rx) = crate::chat_canvas_tools::chat_tool_channel();
    let worker = std::thread::spawn(move || {
        run_modify_turn(
            worker_provider.as_ref(),
            ChatRequest::default(),
            &chat_tx,
            &executor,
            vec!["board".into()],
        )
    });
    let request = tool_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(request.args_json.contains("corrected"));
    assert!(!request.args_json.contains("icon_font"));
    request
        .ack
        .send(ChatToolResult {
            content: r#"{"success":true,"count":1}"#.into(),
            is_error: false,
        })
        .unwrap();
    worker.join().unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert!(provider.feedback.lock().unwrap().contains("iconFontName"));
    assert!(
        tool_rx.try_recv().is_err(),
        "one host apply after validation"
    );
    let deltas: Vec<_> = chat_rx.try_iter().collect();
    assert!(!deltas.iter().any(|d| matches!(d, ChatDelta::Error(_))));
    assert!(deltas
        .iter()
        .any(|d| matches!(d,ChatDelta::TextDelta(t) if t.contains("APPLIED"))));
}

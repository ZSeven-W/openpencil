use super::*;
use std::net::TcpListener;

#[test]
fn cancel_interrupts_a_silent_structured_edit_transport() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (accepted_tx, accepted_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        accepted_tx.send(()).unwrap();
        let _ = release_rx.recv_timeout(Duration::from_secs(5));
        drop(stream);
    });
    let provider = MobileModifyProvider {
        runtime: chat_runtime().unwrap(),
        turn: BuiltinChatTurn {
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: "test".into(),
            model: "glm-5.3-flash".into(),
            base_url: format!("http://{address}"),
            system_prompt: String::new(),
            history: Vec::new(),
            prompt: String::new(),
            max_output_tokens: 8192,
        },
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let mut stream = provider.send_cancellable(ChatRequest::default(), Arc::clone(&cancel));
    let (done_tx, done_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        done_tx.send(stream.next()).unwrap();
    });
    accepted_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    cancel.store(true, Ordering::Release);
    assert!(done_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap()
        .is_none());
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    server.join().unwrap();
}

#[test]
fn replacing_the_document_drops_queued_edits_and_deltas_before_applying_or_finalizing() {
    for design_loop in [false, true] {
        for host_open in [false, true] {
            let document = |title: &str| {
                serde_json::from_value(serde_json::json!({
                "version":"1.0.0", "children":[{
                    "type":"frame", "id":"board", "name":"Home", "width":390, "height":844,
                    "children":[{"type":"text","id":"title","content":title,"height":40,"fontSize":28}]
                }]
            })).unwrap()
            };
            let mut host = WidgetHostNative::new();
            host.replace_editor_state(EditorState::from_document(document("old")));
            let identity = (
                host.document_epoch(),
                host.editor_state().document_generation(),
            );
            let cancel = Arc::new(AtomicBool::new(false));
            let (delta_tx, delta_rx) = mpsc::channel();
            let (tool_tx, tool_rx) = mpsc::channel();
            let (ack_tx, ack_rx) = mpsc::sync_channel(1);
            tool_tx
                .send(ChatToolRequest {
                    name: op_chat_agent::chat_modify::APPLY_MODIFICATION_OP.into(),
                    args_json: serde_json::json!({
                        "targetFrameIds":["board"],
                        "nodes":[["null",{"type":"text","id":"title","content":"stale edit"}]]
                    })
                    .to_string(),
                    ack: ack_tx,
                })
                .unwrap();
            delta_tx
                .send(ChatDelta::TextDelta("stale completion".into()))
                .unwrap();
            let session = ChatSession::from_channels_with_cancel(
                delta_rx,
                Some(tool_rx),
                Arc::clone(&cancel),
            );
            let session = if design_loop {
                session.into_design_loop()
            } else {
                session
            };
            let mut chat = MobileChatHost {
                turn: Some(ChatTurnJob {
                    session,
                    document_identity: identity,
                    running_tab: 0,
                    abort: None,
                    indicator_epoch: None,
                    page_edit_fence: Some(vec!["board".into()]),
                    document_mutated: false,
                }),
            };
            if host_open {
                host.install_open_document(document("new"), None, Some("new.op".into()))
                    .unwrap();
            } else {
                host.editor_state_mut().replace_document(document("new"));
            }
            host.editor_state_mut().chat.messages = vec![op_editor_core::ChatMessage::assistant(
                "new document transcript",
            )];
            let before = host.editor_state().doc.clone();
            assert!(chat.pump(&mut host, 10, (390.0, 844.0)).is_none());
            assert!(chat.turn.is_none());
            assert!(cancel.load(Ordering::Acquire));
            assert!(matches!(
                ack_rx.try_recv(),
                Err(mpsc::TryRecvError::Disconnected)
            ));
            assert_eq!(
                host.editor_state().doc,
                before,
                "a colliding id must not admit old edits or finalize"
            );
            assert_eq!(
                host.editor_state().chat.messages[0].content,
                "new document transcript"
            );
        }
    }
}

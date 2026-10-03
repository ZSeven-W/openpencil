use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct ScriptedProvider {
    requests: Mutex<Vec<ChatRequest>>,
}

impl ChatProvider for ScriptedProvider {
    fn provider_label(&self) -> &str {
        "scripted-modify"
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.requests.lock().unwrap().push(request);
        let script = r##"
            I("screen", {
                type: "frame",
                id: "state-board",
                name: "Interaction States",
                x: 420,
                y: 0,
                width: 360,
                height: 640,
                layout: "vertical",
                gap: 16,
                padding: 24,
                fill: [{type: "solid", color: "#F8FAFC"}],
                children: []
            });
        "##;
        Box::new(
            vec![
                ChatDelta::TextDelta(script.into()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]
            .into_iter(),
        )
    }
}

fn state_with_roots(ids: &[&str]) -> EditorState {
    let roots = ids
        .iter()
        .map(|id| {
            serde_json::json!({
                "type": "frame",
                "id": id,
                "name": format!("Screen {id}"),
                "x": 0,
                "y": 0,
                "width": 390,
                "height": 844,
                "fill": [{"type": "solid", "color": "#FFFFFF"}],
                "children": [{
                    "type": "text",
                    "id": format!("{id}-title"),
                    "name": "Title",
                    "x": 24,
                    "y": 24,
                    "width": 300,
                    "height": 40,
                    "content": "Checkout",
                    "fontSize": 28,
                    "fontWeight": 700,
                    "fill": [{"type": "solid", "color": "#111827"}]
                }]
            })
        })
        .collect::<Vec<_>>();
    let source = serde_json::json!({
        "version": "1.0.0",
        "children": roots,
    })
    .to_string();
    op_host_services::doc_io::load_editor_state_from_source(&source, op_editor_core::Locale::EnUs)
        .unwrap()
}

fn contains_node_name(nodes: &[jian_ops_schema::node::PenNode], name: &str) -> bool {
    nodes.iter().any(|node| {
        node.base().name.as_deref() == Some(name)
            || node
                .children()
                .is_some_and(|children| contains_node_name(children, name))
    })
}

#[test]
fn modify_mode_runs_the_production_plan_turn_and_scoped_apply_path() {
    let provider = ScriptedProvider::default();
    let execution = run_loaded_modify(
        state_with_roots(&["screen"]),
        "Complete the missing interaction states.",
        ModifyScope::One(None),
        "kimi-code/k3-256k",
        ThinkingMode::Disabled,
        &provider,
        vec![ChatAttachment {
            name: "source.png".into(),
            media_type: "image/png".into(),
            data: vec![1, 2, 3],
        }],
    )
    .unwrap();

    assert_eq!(execution.target_frame_ids, vec!["screen"]);
    assert_eq!(execution.applied_count, 1);
    assert!(contains_node_name(
        execution.state.active_children(),
        "Interaction States"
    ));

    let requests = provider.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].max_output_tokens, 8192);
    assert_eq!(requests[0].attachments.len(), 1);
    assert_eq!(requests[0].attachments[0].data, vec![1, 2, 3]);
    assert_eq!(requests[0].thinking, ThinkingMode::Disabled);
    assert_eq!(requests[0].model.as_deref(), Some("kimi-code/k3-256k"));
    assert!(requests[0].user_message.contains("CONTEXT NODES:"));
    assert!(requests[0]
        .user_message
        .contains("Complete the missing interaction states."));
}

#[test]
fn keep_thinking_opt_in_uses_adaptive_mode_for_k3_provenance() {
    assert_eq!(modify_thinking(false), ThinkingMode::Disabled);
    assert_eq!(modify_thinking(true), ThinkingMode::Adaptive);
    assert!(truthy("true"));
    assert!(!truthy("0"));
}

#[test]
fn modify_provider_selection_accepts_antigravity_aliases() {
    assert_eq!(
        ModifyProviderKind::parse("antigravity").unwrap(),
        ModifyProviderKind::Antigravity
    );
    assert_eq!(
        ModifyProviderKind::parse("AGY").unwrap(),
        ModifyProviderKind::Antigravity
    );
    assert_eq!(
        ModifyProviderKind::parse("openai-compat").unwrap(),
        ModifyProviderKind::OpenAiCompat
    );

    let provider = build_provider(
        ModifyProviderKind::Antigravity,
        "gemini-3.6-flash-high",
        None,
        None,
    )
    .unwrap();
    assert_eq!(provider.provider_label(), "Antigravity");

    let provider = build_provider(
        ModifyProviderKind::OpenAiCompat,
        "custom-model",
        Some("http://127.0.0.1:1234/v1".into()),
        Some("secret".into()),
    )
    .unwrap();
    assert_eq!(provider.provider_label(), "smoke-modify");
}

#[test]
fn modify_provider_selection_rejects_unknown_and_missing_http_credentials() {
    let error = ModifyProviderKind::parse("anthropic").unwrap_err();
    assert!(error.contains("unknown OPENPENCIL_LLM_PROVIDER"));
    assert!(error.contains("antigravity"));

    let error = build_provider(
        ModifyProviderKind::OpenAiCompat,
        "model",
        None,
        Some("secret".into()),
    )
    .err()
    .expect("missing base URL must fail");
    assert!(error.contains("OPENPENCIL_LLM_BASE_URL is required"));

    let error = build_provider(
        ModifyProviderKind::OpenAiCompat,
        "model",
        Some("http://127.0.0.1:1234/v1".into()),
        None,
    )
    .err()
    .expect("missing API key must fail");
    assert!(error.contains("OPENPENCIL_LLM_API_KEY is required"));
}

#[test]
fn implicit_target_requires_exactly_one_top_level_frame() {
    let mut empty = state_with_roots(&[]);
    assert!(select_target_frame(&mut empty, None)
        .unwrap_err()
        .contains("found 0"));

    let mut ambiguous = state_with_roots(&["screen-a", "screen-b"]);
    assert!(select_target_frame(&mut ambiguous, None)
        .unwrap_err()
        .contains("found 2"));
    assert_eq!(
        select_target_frame(&mut ambiguous, Some("screen-b")).unwrap(),
        "screen-b"
    );
    assert_eq!(ambiguous.selection.set, vec![NodeId::new("screen-b")]);
}

#[test]
fn explicit_all_roots_scope_builds_the_same_multi_board_refine_plan_as_home() {
    let mut state = state_with_roots(&["card-a", "card-b", "card-c"]);
    let before = state.doc.clone();
    select_modify_scope(&mut state, ModifyScope::AllRoots).unwrap();
    let plan =
        build_modify_plan(&state, "Add clear tutorial labels to these three cards.").unwrap();
    assert_eq!(plan.target_frame_ids, vec!["card-a", "card-b", "card-c"]);
    assert_eq!(
        state.doc, before,
        "selecting the scope must not edit the baseline"
    );
    assert!(select_modify_scope(&mut state_with_roots(&[]), ModifyScope::AllRoots).is_err());
}

#[test]
fn modify_attachments_reject_invalid_or_excessive_inputs_before_the_provider_runs() {
    assert!(load_attachments(None).unwrap().is_empty());
    assert!(load_attachments(Some("not json")).is_err());
    assert!(load_attachments(Some(r#"["a.png","b.png","c.png","d.png","e.png"]"#)).is_err());
    assert!(load_attachments(Some(r#"["not-an-image.txt"]"#)).is_err());
}

#[test]
fn explicit_target_rejects_missing_and_non_frame_nodes() {
    let mut state = state_with_roots(&["screen"]);
    assert!(select_target_frame(&mut state, Some("missing"))
        .unwrap_err()
        .contains("was not found"));
    assert!(select_target_frame(&mut state, Some("screen-title"))
        .unwrap_err()
        .contains("is not a Frame"));
}

#[test]
fn hash_is_stable_and_baseline_output_alias_is_rejected() {
    assert_eq!(
        sha256_hex(b"fixed baseline"),
        "5264c104ad63e29e86586bb556c24ee3c835440664b4becb22a4e697eb84d307"
    );
    // The thread name is the full test path ("modify_mode::tests::…"), and
    // Windows rejects ':' in file names, so it must be sanitized before it
    // can key a temp file.
    let thread = std::thread::current();
    let discriminator: String = thread
        .name()
        .unwrap_or("test")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let path = std::env::temp_dir().join(format!(
        "op-smoke-modify-alias-{}-{discriminator}.op",
        std::process::id(),
    ));
    std::fs::write(&path, br#"{"version":"1.0.0","children":[]}"#).unwrap();
    let error = reject_input_overwrite(&path, &path).unwrap_err();
    assert!(error.contains("must not overwrite"));
    std::fs::remove_file(path).unwrap();
}

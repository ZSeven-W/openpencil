use super::*;
use op_editor_core::{EditorState, NodeId};

#[test]
fn codegen_lifecycle_explicitly_clears_request_credentials() {
    let source = include_str!("codegen_web.rs");
    let implementation = source
        .split("#[cfg(test)]")
        .next()
        .expect("codegen implementation");
    assert!(
        implementation.matches("s.0.credential = None;").count() >= 2,
        "cancel and terminal completion must both clear the captured credential"
    );
}

fn two_rect_state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"n1","name":"A","x":0,"y":0,"width":10,"height":10},
            {"type":"rectangle","id":"n2","name":"B","x":20,"y":0,"width":10,"height":10}
        ]}"#,
    )
    .expect("fixture parses")
    .value;
    EditorState::from_document(doc)
}

#[test]
fn build_input_resolves_the_selection_subtrees() {
    let mut state = two_rect_state();
    state.set_single_selection(NodeId::new("n1"));
    let input = build_codegen_input(&state).expect("input");
    assert!(input.nodes_json.contains("n1"));
    assert!(!input.nodes_json.contains("n2"));
    assert_eq!(input.framework, state.codegen.framework);
}

#[test]
fn empty_selection_falls_back_to_active_page_children() {
    // TS getTargetNodes (code-panel.tsx:137-142): no selection → ALL
    // active-page children. Desktop codegen_input parity.
    let mut state = two_rect_state();
    state.clear_selection();
    let input = build_codegen_input(&state).expect("page fallback");
    assert!(input.nodes_json.contains("n1"));
    assert!(input.nodes_json.contains("n2"));
}

#[test]
fn empty_page_and_unresolvable_selection_are_none() {
    let empty = EditorState::new();
    assert!(build_codegen_input(&empty).is_none());
    let mut ghost = two_rect_state();
    ghost.set_single_selection(NodeId::new("ghost"));
    assert!(build_codegen_input(&ghost).is_none());
}

#[test]
fn framework_ext_maps_every_framework() {
    assert_eq!(framework_ext(Framework::React), "tsx");
    assert_eq!(framework_ext(Framework::ReactNative), "tsx");
    assert_eq!(framework_ext(Framework::Vue), "vue");
    assert_eq!(framework_ext(Framework::Svelte), "svelte");
    assert_eq!(framework_ext(Framework::Html), "html");
    assert_eq!(framework_ext(Framework::Flutter), "dart");
    assert_eq!(framework_ext(Framework::SwiftUi), "swift");
    assert_eq!(framework_ext(Framework::Compose), "kt");
}

#[test]
fn codegen_proxy_body_carries_the_selected_request_scoped_credential() {
    let req = PendingRequest {
        id: RequestId(1),
        kind: op_codegen::ai::types::RequestKind::Planning,
        skills: vec!["codegen-plan"],
        user_message: "plan".into(),
        max_output_tokens: 1024,
        thinking: op_ai::chat_provider::ThinkingMode::Disabled,
        effort: op_ai::chat_provider::EffortLevel::Low,
    };
    let credential = serde_json::json!({"api_key":"sk-codegen"});

    let body: serde_json::Value = serde_json::from_str(&build_body_json(
        &req,
        Some(AgentProvider::CodexCli),
        Some("provider-codegen"),
        "private-model",
        Some(&credential),
    ))
    .unwrap();

    assert_eq!(body["provider"], "codex-cli");
    assert_eq!(body["builtinProviderId"], "provider-codegen");
    assert_eq!(body["model"], "private-model");
    assert_eq!(body["credential"]["api_key"], "sk-codegen");
}

fn asset(zip_path: &str, bytes: &[u8]) -> AssetFile {
    AssetFile {
        id: zip_path.into(),
        relative_path: format!("./{zip_path}"),
        zip_path: zip_path.into(),
        mime_type: "image/png".into(),
        bytes: bytes.to_vec(),
        source_node_id: "n1".into(),
    }
}

#[test]
fn code_zip_carries_component_and_each_asset() {
    let assets = vec![
        asset("assets/img-1.png", &[1, 2, 3]),
        asset("assets/img-2.png", &[4, 5, 6]),
    ];
    let bytes = build_code_zip("export default function X(){}", "vue", &assets);
    // STORED zip magic + every entry name present.
    assert_eq!(&bytes[0..4], &[0x50, 0x4B, 0x03, 0x04]);
    let hay = String::from_utf8_lossy(&bytes);
    assert!(hay.contains("component.vue"));
    assert!(hay.contains("assets/img-1.png"));
    assert!(hay.contains("assets/img-2.png"));
}

#[test]
fn download_cache_returns_html_after_generating_react_and_switching_back() {
    RESULTS.with(|results| *results.borrow_mut() = WebCodegenResults::default());
    let mut state = EditorState::new();
    let identity = document_identity(0, &state);
    state.codegen.framework = Framework::Html;
    cache_completed_result(
        identity,
        Framework::Html,
        "<main>html source</main>".into(),
        vec![asset("assets/html.png", &[0xde, 0xad, 0xbe, 0xef])],
    );
    state.codegen.framework = Framework::React;
    cache_completed_result(
        identity,
        Framework::React,
        "export default function ReactSource() {}".into(),
        vec![asset("assets/react.png", &[9, 9, 9])],
    );
    state.codegen.framework = Framework::Html;

    RESULTS.with(|results| {
        let results = results.borrow();
        let result = results
            .get(identity, state.codegen.framework)
            .expect("HTML result");
        assert_eq!(result.code, "<main>html source</main>");
        assert_eq!(result.framework_ext, "html");
        assert_eq!(result.assets[0].bytes, [0xde, 0xad, 0xbe, 0xef]);
        let zip = build_code_zip(&result.code, result.framework_ext, &result.assets);
        let hay = String::from_utf8_lossy(&zip);
        assert!(hay.contains("component.html") && hay.contains("assets/html.png"));
        assert!(!hay.contains("component.tsx") && !hay.contains("assets/react.png"));
        assert!(zip
            .windows(4)
            .any(|bytes| bytes == [0xde, 0xad, 0xbe, 0xef]));
    });
}

#[test]
fn cancel_before_applying_queued_done_preserves_previous_download_result() {
    RESULTS.with(|results| *results.borrow_mut() = WebCodegenResults::default());
    let identity = document_identity(0, &EditorState::new());
    cache_completed_result(
        identity,
        Framework::Html,
        "<main>painted result</main>".into(),
        vec![asset("assets/painted.png", &[1, 2, 3])],
    );

    let mut queue = VecDeque::new();
    enqueue_completed_result(
        &mut queue,
        Framework::Html,
        "<main>canceled result</main>".into(),
        false,
        vec![asset("assets/canceled.png", &[9, 9, 9])],
    );
    assert_eq!(queue.len(), 1, "terminal result is waiting for the pump");
    discard_queued_deltas(&mut queue);
    assert!(queue.is_empty());

    RESULTS.with(|results| {
        let results = results.borrow();
        let result = results
            .get(identity, Framework::Html)
            .expect("painted result");
        assert_eq!(result.code, "<main>painted result</main>");
        assert_eq!(result.framework_ext, "html");
        assert_eq!(result.assets[0].zip_path, "assets/painted.png");
        assert_eq!(result.assets[0].bytes, [1, 2, 3]);
    });
}

#[test]
fn failed_regeneration_keeps_previous_code_and_target_snapshot_together() {
    let mut state = EditorState::new();
    state.codegen.phase = CodegenPhase::Generating;
    state.codegen.code = "<main>previous result</main>".into();
    state.codegen.selection_snapshot = vec!["old-node".into()];
    let identity = document_identity(0, &state);
    let mut run_snapshot = vec!["new-node".into()];

    assert!(apply_codegen_delta(
        &mut state.codegen,
        WebCodegenDelta::Failed("regeneration failed".into()),
        identity,
        &mut run_snapshot,
    ));
    assert_eq!(state.codegen.phase, CodegenPhase::Error);
    assert_eq!(state.codegen.code, "<main>previous result</main>");
    assert_eq!(state.codegen.selection_snapshot, ["old-node"]);
    assert_eq!(
        run_snapshot,
        ["new-node"],
        "a failed run must not commit its targets"
    );
}

#[test]
fn successful_regeneration_commits_its_run_target_snapshot() {
    RESULTS.with(|results| *results.borrow_mut() = WebCodegenResults::default());
    let mut state = EditorState::new();
    state.codegen.phase = CodegenPhase::Generating;
    state.codegen.code = "previous result".into();
    state.codegen.selection_snapshot = vec!["old-node".into()];
    let identity = document_identity(0, &state);
    let mut run_snapshot = vec!["new-node".into()];

    assert!(apply_codegen_delta(
        &mut state.codegen,
        WebCodegenDelta::Done {
            code: "new result".into(),
            degraded: false,
            framework: Framework::React,
            assets: Vec::new(),
        },
        identity,
        &mut run_snapshot,
    ));
    assert_eq!(state.codegen.phase, CodegenPhase::Complete);
    assert_eq!(state.codegen.code, "new result");
    assert_eq!(state.codegen.selection_snapshot, ["new-node"]);
    assert!(run_snapshot.is_empty());
    RESULTS.with(|results| {
        assert_eq!(
            results
                .borrow()
                .get(identity, Framework::React)
                .expect("successful raw result")
                .code,
            "new result"
        );
    });
}

#[test]
fn document_replacement_discards_queued_completion_and_hides_raw_cache() {
    RESULTS.with(|results| *results.borrow_mut() = WebCodegenResults::default());
    let mut state = two_rect_state();
    state.codegen.framework = Framework::Html;
    state.codegen.phase = CodegenPhase::Complete;
    state.codegen.code = "<main>painted old document</main>".into();
    let old_identity = document_identity(7, &state);
    cache_completed_result(
        old_identity,
        Framework::Html,
        state.codegen.code.clone(),
        vec![asset("assets/old.png", &[1, 2, 3])],
    );

    let mut queue = VecDeque::new();
    enqueue_completed_result(
        &mut queue,
        Framework::Html,
        "<main>late old completion</main>".into(),
        false,
        vec![asset("assets/late.png", &[9, 9, 9])],
    );

    // Models an MCP/live-sync replacement: the host epoch is stable while
    // EditorState's generation advances and clears document-scoped codegen.
    state.replace_document(EditorState::new().doc);
    let live_identity = document_identity(7, &state);
    assert_ne!(live_identity, old_identity);
    assert!(discard_if_stale_document(
        old_identity,
        live_identity,
        &mut queue
    ));
    assert!(queue.is_empty(), "late completion is never applied");
    assert!(state.codegen.code.is_empty(), "painted old code is cleared");

    RESULTS.with(|results| {
        assert!(
            results
                .borrow()
                .get(live_identity, Framework::Html)
                .is_none(),
            "old raw assets are not visible to the replacement document"
        );
    });
}

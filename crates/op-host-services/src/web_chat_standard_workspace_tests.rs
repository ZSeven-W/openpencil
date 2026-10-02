use super::*;

fn request(normal: bool, user: &str) -> WebStandardTurnRequest {
    parse_standard_turn_body(
        &serde_json::json!({
            "model": "test-model", "user": user,
            "workspaceVisible": normal, "workspaceSelected": 1,
            "document": {"version": "1.0.0", "children": [
                {"type": "frame", "id": "home", "name": "Home", "width": 390, "height": 844},
                {"type": "frame", "id": "orders", "name": "Orders", "width": 390, "height": 844}
            ]}
        })
        .to_string(),
    )
    .expect("request")
}

#[test]
fn normal_mode_context_is_request_local_and_resolves_a_named_page() {
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let req = request(true, "把首页标题改成咖啡");
    let snapshot = apply_request_snapshot(&req, &state, &SseHub::default(), None).unwrap();
    assert_eq!(
        crate::chat_intent::build_modify_plan(&snapshot, &req.ai.user)
            .unwrap()
            .target_frame_ids,
        ["home"]
    );
    assert!(!state.lock().unwrap().editor.editor_ui.workspace.visible);
    let req = request(false, "把首页标题改成咖啡");
    let snapshot = apply_request_snapshot(&req, &state, &SseHub::default(), None).unwrap();
    assert!(crate::chat_intent::build_modify_plan(&snapshot, &req.ai.user).is_none());
}

#[test]
fn ambiguous_normal_mode_edit_finishes_before_provider_or_document_write() {
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let req = request(true, "改一下标题");
    apply_request_snapshot(&req, &state, &SseHub::default(), None).unwrap();
    let before = state.lock().unwrap().editor.doc.clone();
    let mut out = Vec::new();
    stream_standard_turn(&mut out, req, &state, &SseHub::default(), None, None).unwrap();
    let response = String::from_utf8(out).unwrap();
    assert!(
        response.contains("Choose the page") || response.contains("请先选择"),
        "{response}"
    );
    assert_eq!(state.lock().unwrap().editor.doc, before);
}

#[test]
fn pinned_ppt_refine_retains_explicit_multi_board_scope_in_normal_mode() {
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let prompt = op_editor_core::refine_prompt(
        op_editor_core::HomeFamily::Presentation,
        "为 OpenPencil 做一份 5 页产品介绍 PPT，包含封面和结束页。",
    );
    let mut req = request(true, &prompt);
    req.launch_route = op_editor_core::LaunchRoute::Refine;
    req.selected_ids = vec!["home".into(), "orders".into()];
    let snapshot = apply_request_snapshot(&req, &state, &SseHub::default(), None).unwrap();
    assert!(snapshot.editor_ui.workspace.visible);
    let plan =
        crate::chat_intent::build_modify_plan_for_route(&snapshot, &prompt, req.launch_route)
            .unwrap();
    assert_eq!(plan.target_frame_ids, ["home", "orders"]);
    assert_eq!(
        pinned_intent(req.launch_route, true),
        Some(crate::chat_intent::DesignIntent::Modify)
    );
    assert!(
        crate::chat_intent::build_modify_plan(&snapshot, &prompt).is_none(),
        "ordinary text alone must not authorize every board"
    );
}

#[test]
fn novice_quick_actions_keep_the_selected_page_and_review_never_writes() {
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    for locale in [
        op_i18n::Locale::EnUs,
        op_i18n::Locale::ZhCn,
        op_i18n::Locale::ZhTw,
    ] {
        for key in [
            "ai.work.editTitlePrompt",
            "ai.work.simplifyPrompt",
            "ai.work.colorsPrompt",
        ] {
            let prompt = op_i18n::translate(locale, key);
            let req = request(true, prompt);
            let snapshot = apply_request_snapshot(&req, &state, &SseHub::default(), None).unwrap();
            let plan = crate::chat_intent::build_modify_plan(&snapshot, prompt)
                .expect("the UI action must be an edit, not a new design");
            assert_eq!(plan.target_frame_ids, ["orders"], "{locale:?}: {prompt}");
        }
        let prompt = op_i18n::translate(locale, "ai.work.reviewPrompt");
        let req = request(true, prompt);
        let snapshot = apply_request_snapshot(&req, &state, &SseHub::default(), None).unwrap();
        assert!(
            crate::chat_intent::workspace_edit::is_workspace_question(&snapshot, prompt),
            "{locale:?}: {prompt}"
        );
        assert!(crate::chat_intent::build_modify_plan(&snapshot, prompt).is_none());
    }
}

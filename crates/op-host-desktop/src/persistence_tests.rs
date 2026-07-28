//! Tests for the desktop Save / Open / Export persistence flow, split
//! from `persistence.rs` to keep both files under the repo's 800-line
//! cap. Wired in as `persistence::tests` via `#[path]` so `use super::*`
//! still resolves against `persistence` itself.

use super::*;
// `save_to_path` arrives via `super::*`; `sidecar_path` is only
// needed for legacy-sidecar cleanup in tests, so import it directly.
use op_host_services::doc_io::sidecar_path;

/// A unique temp path under the OS temp dir for a round-trip test.
fn temp_op_path(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!("openpencil-test-{tag}-{pid}-{nanos}.op"));
    p
}

fn bind_collaboration_for_save_as(
    host: &mut WidgetHostNative,
    phase: op_editor_core::CollabConnectionPhase,
    role: op_editor_core::CollabUiRole,
    pending_edit: op_editor_core::CollabPendingEditUi,
) {
    let collab = &mut host.editor_state_mut().editor_ui.collab;
    assert!(collab.set_authenticated_session(
        phase,
        op_editor_core::AuthenticatedCollabSession {
            session_name: "Shared design".into(),
            role,
            share_endpoint: None,
        },
        Vec::new(),
    ));
    collab.pending_edit = pending_edit;
}

#[test]
fn synchronous_save_as_requires_background_fork_for_active_owner_and_guest() {
    for role in [
        op_editor_core::CollabUiRole::Owner,
        op_editor_core::CollabUiRole::Editor,
    ] {
        let mut host = WidgetHostNative::new();
        bind_collaboration_for_save_as(
            &mut host,
            op_editor_core::CollabConnectionPhase::Active,
            role,
            op_editor_core::CollabPendingEditUi::None,
        );
        let original = PathBuf::from("shared-source.op");
        let mut current_path = Some(original.clone());
        let mut synchronous_writer_called = false;

        let outcome = handle_save_as_with(
            &mut host,
            &mut current_path,
            None,
            |_| {
                synchronous_writer_called = true;
                Ok(Some(PathBuf::from("unsafe-fork.op")))
            },
            |_, _| {},
        );

        assert_eq!(outcome, SaveActionOutcome::BackgroundForkRequired);
        assert!(!synchronous_writer_called);
        assert_eq!(current_path, Some(original));
        assert!(host
            .editor_state()
            .editor_ui
            .collab
            .authenticated_session()
            .is_some());
    }
}

#[test]
fn ended_guest_with_pending_edit_still_requires_background_fork() {
    let mut host = WidgetHostNative::new();
    bind_collaboration_for_save_as(
        &mut host,
        op_editor_core::CollabConnectionPhase::Ended,
        op_editor_core::CollabUiRole::Editor,
        op_editor_core::CollabPendingEditUi::Submitting,
    );
    let mut current_path = Some(PathBuf::from("shared-source.op"));
    let mut synchronous_writer_called = false;

    let outcome = handle_save_as_with(
        &mut host,
        &mut current_path,
        None,
        |_| {
            synchronous_writer_called = true;
            Ok(Some(PathBuf::from("unsafe-fork.op")))
        },
        |_, _| {},
    );

    assert_eq!(outcome, SaveActionOutcome::BackgroundForkRequired);
    assert!(!synchronous_writer_called);
    assert_eq!(
        host.editor_state().editor_ui.collab.pending_edit,
        op_editor_core::CollabPendingEditUi::Submitting
    );
    assert!(host
        .editor_state()
        .editor_ui
        .collab
        .authenticated_session()
        .is_some());
}

#[test]
fn run_action_preserves_the_typed_background_fork_requirement() {
    let mut host = WidgetHostNative::new();
    bind_collaboration_for_save_as(
        &mut host,
        op_editor_core::CollabConnectionPhase::Active,
        op_editor_core::CollabUiRole::Owner,
        op_editor_core::CollabPendingEditUi::None,
    );
    let mut current_path = Some(PathBuf::from("shared-source.op"));

    assert_eq!(
        run_action(
            op_editor_core::editor_ui_state::FileAction::SaveAs,
            &mut host,
            &mut current_path,
            None,
        ),
        ActionOutcome::SaveAsForkRequired
    );
    assert_eq!(current_path, Some(PathBuf::from("shared-source.op")));

    current_path = None;
    assert_eq!(
        run_action(
            op_editor_core::editor_ui_state::FileAction::Save,
            &mut host,
            &mut current_path,
            None,
        ),
        ActionOutcome::SaveAsForkRequired
    );
    assert!(current_path.is_none());
}

#[test]
fn standalone_synchronous_save_as_success_rebinds_and_marks_saved() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().mark_document_changed();
    let target = PathBuf::from("standalone-fork.op");
    let mut current_path = Some(PathBuf::from("old.op"));

    let outcome = handle_save_as_with(
        &mut host,
        &mut current_path,
        None,
        |_| Ok(Some(target.clone())),
        |_, _| panic!("successful Save As must not report an error"),
    );

    assert_eq!(outcome, SaveActionOutcome::Saved);
    assert_eq!(current_path.as_deref(), Some(target.as_path()));
    assert_eq!(
        host.editor_state().editor_ui.file_name_display.as_deref(),
        Some("standalone-fork.op")
    );
    assert!(!host.editor_state().is_dirty());
}

#[test]
fn standalone_synchronous_save_as_cancel_does_not_rebind() {
    let mut host = WidgetHostNative::new();
    let original = PathBuf::from("old.op");
    let mut current_path = Some(original.clone());

    let outcome = handle_save_as_with(
        &mut host,
        &mut current_path,
        None,
        |_| Ok(None),
        |_, _| panic!("cancelled Save As must not report an error"),
    );

    assert_eq!(outcome, SaveActionOutcome::Cancelled);
    assert_eq!(current_path, Some(original));
}

#[test]
fn standalone_synchronous_save_as_failure_does_not_rebind() {
    let mut host = WidgetHostNative::new();
    let original = PathBuf::from("old.op");
    let mut current_path = Some(original.clone());
    let mut reported = false;

    let outcome = handle_save_as_with(
        &mut host,
        &mut current_path,
        None,
        |_| Err(DocIoError::Io("synthetic write failure".into())),
        |_, error| {
            reported = true;
            assert_eq!(error, &DocIoError::Io("synthetic write failure".into()));
        },
    );

    assert_eq!(outcome, SaveActionOutcome::Failed);
    assert!(reported);
    assert_eq!(current_path, Some(original));
}

#[test]
fn new_file_action_resets_to_starter_frame() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().doc.children.clear();
    host.editor_state_mut().viewport.pan_x = -5000.0;
    host.editor_state_mut().viewport.pan_y = -5000.0;
    host.editor_state_mut().viewport.zoom = 0.2;
    let mut current_path = Some(PathBuf::from("/tmp/old.op"));

    let outcome = run_action(
        op_editor_core::editor_ui_state::FileAction::New,
        &mut host,
        &mut current_path,
        None,
    );

    assert_eq!(outcome, ActionOutcome::Saved);
    assert!(current_path.is_none());
    assert_eq!(host.editor_state().doc.children.len(), 1);
    assert!(host.editor_state().selection.is_empty());
    let frame = match &host.editor_state().doc.children[0] {
        jian_ops_schema::node::PenNode::Frame(frame) => frame,
        other => panic!(
            "new file should create the blank starter frame, got {:?}",
            other
        ),
    };
    assert_eq!(frame.base.x, Some(0.0));
    assert_eq!(frame.base.y, Some(0.0));
    assert!(matches!(
        frame.container.width,
        Some(jian_ops_schema::sizing::SizingBehavior::Number(1200.0))
    ));
    assert!(matches!(
        frame.container.height,
        Some(jian_ops_schema::sizing::SizingBehavior::Number(800.0))
    ));
    let v = host.editor_state().viewport;
    assert!((v.zoom - 0.8933333).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - 64.0).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - 72.66669).abs() < 1e-2, "pan_y {}", v.pan_y);
}

#[test]
fn new_file_action_preserves_builtin_agent_models() {
    let mut host = WidgetHostNative::new();
    let builtin_id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_config(
            "DS",
            "sk-test",
            "deepseek-v4-pro",
            op_editor_core::BuiltinAgentKind::OpenAiCompat,
            "https://api.deepseek.com/v1",
        );
    host.editor_state_mut().rebuild_chat_models();
    let mut current_path = Some(PathBuf::from("/tmp/old.op"));

    let outcome = run_action(
        op_editor_core::editor_ui_state::FileAction::New,
        &mut host,
        &mut current_path,
        None,
    );

    assert_eq!(outcome, ActionOutcome::Saved);
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .builtin_agents
            .len(),
        1
    );
    assert!(host
        .editor_state()
        .chat
        .available_models
        .iter()
        .any(|m| m.builtin_provider_id.as_deref() == Some(builtin_id.as_str())));
}

#[test]
fn opening_document_preserves_builtin_agent_models() {
    let mut host = WidgetHostNative::new();
    let builtin_id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_config(
            "MINIMAX",
            "sk-test",
            "MiniMax-M2.7",
            op_editor_core::BuiltinAgentKind::OpenAiCompat,
            "https://api.minimaxi.com/v1",
        );
    host.editor_state_mut().rebuild_chat_models();
    assert!(host
        .editor_state()
        .chat
        .available_models
        .iter()
        .any(|m| m.builtin_provider_id.as_deref() == Some(builtin_id.as_str())));

    let state_to_open = EditorState::new();
    let path = temp_op_path("open-preserves-builtins");
    save_to_path(&state_to_open, &path).expect("save succeeds");
    let mut current_path = None;

    assert!(open_path(&mut host, path.clone(), &mut current_path, None));

    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .builtin_agents
            .len(),
        1
    );
    assert!(host
        .editor_state()
        .chat
        .available_models
        .iter()
        .any(|m| m.builtin_provider_id.as_deref() == Some(builtin_id.as_str())));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(sidecar_path(&path));
}

#[test]
fn opening_op_preserves_font_catalog_and_detects_only_real_missing_families() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.font_import_supported = true;
        ui.system_fonts_loaded = true;
        ui.system_font_families = std::sync::Arc::new(vec!["PingFang SC".into()]);
        ui.bundled_font_families = std::sync::Arc::new(vec!["Inter".into()]);
        ui.imported_font_families = std::sync::Arc::new(vec!["Brand Sans".into()]);
    }
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"text","id":"system","content":"系统字体","fontFamily":"pingfang sc"},
            {"type":"text","id":"bundled","content":"Bundled","fontFamily":"INTER"},
            {"type":"text","id":"imported","content":"Imported","fontFamily":"brand sans"},
            {"type":"text","id":"css-stack","content":"Stack","fontFamily":"Inter,ui-sans-serif,system-ui,-apple-system,\"PingFang SC\",sans-serif"},
            {"type":"text","id":"generic","content":"Generic","fontFamily":"sans-serif"},
            {"type":"text","id":"missing","content":"Missing","fontFamily":"__MissingOpFont__"}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    let path = temp_op_path("open-font-matching");
    save_to_path(&EditorState::from_document(doc), &path).expect("save succeeds");
    let mut current_path = None;
    assert!(open_path(&mut host, path.clone(), &mut current_path, None));
    let ui = &host.editor_state().editor_ui;
    assert!(ui.font_import_supported);
    assert!(ui.system_fonts_loaded);
    assert_eq!(&*ui.system_font_families, &["PingFang SC"]);
    assert_eq!(&*ui.bundled_font_families, &["Inter"]);
    assert_eq!(&*ui.imported_font_families, &["Brand Sans"]);
    assert!(ui.missing_fonts_modal_open);
    let entries = &ui.missing_fonts_prompt.as_ref().expect("prompt").entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].family, "__MissingOpFont__");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(sidecar_path(&path));
}

#[test]
fn opening_document_leaves_nothing_selected() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(op_editor_core::NodeId::new("n10"));

    let mut state_to_open = EditorState::starter();
    state_to_open.set_single_selection(op_editor_core::NodeId::new("n10"));
    let path = temp_op_path("open-clears-selection");
    save_to_path(&state_to_open, &path).expect("save succeeds");
    let mut current_path = None;

    assert!(open_path(&mut host, path.clone(), &mut current_path, None));

    assert!(host.editor_state().selection.is_empty());
    assert_eq!(host.editor_state().doc.children.len(), 1);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(sidecar_path(&path));
}

#[test]
fn selected_node_svg_export_routes_through_desktop_dispatch() {
    let doc = jian_ops_schema::load_str(
        r##"{
          "version":"1.0.0",
          "pages":[{"id":"page","name":"Page","children":[
            {"type":"text","id":"selected-card","name":"Selected Card",
             "x":10,"y":20,"width":120,"height":24,"content":"Selected Card"},
            {"type":"text","id":"sibling-card","name":"Sibling Card",
             "x":300,"y":400,"width":100,"height":24,"content":"Sibling Card"}
          ]}],
          "children":[]
        }"##,
    )
    .expect("fixture JSON parses")
    .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.export_format = op_editor_core::editor_ui_state::ExportFormat::Svg;
    state.set_single_selection(op_editor_core::NodeId::new("selected-card"));
    let path = temp_op_path("desktop-selected-svg-route").with_extension("svg");

    export_editor_state_to_path(&state, &path).expect("desktop SVG export succeeds");

    let svg = std::fs::read_to_string(&path).expect("desktop SVG export writes a file");
    assert!(
        svg.contains("selected-card"),
        "selected node missing: {svg}"
    );
    assert!(
        svg.contains("Selected Card"),
        "selected name missing: {svg}"
    );
    assert!(
        !svg.contains("sibling-card"),
        "unselected sibling leaked into export: {svg}"
    );
    assert!(
        !svg.contains("Sibling Card"),
        "unselected sibling name leaked into export: {svg}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn opening_document_fits_and_centers_multiple_root_nodes() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r#"{
          "version":"1.0.0",
          "children":[
            {"type":"frame","id":"left","name":"Left","x":900,"y":120,"width":240,"height":320},
            {"type":"frame","id":"right","name":"Right","x":1320,"y":220,"width":260,"height":280}
          ]
        }"#,
    )
    .expect("fixture JSON parses")
    .value;
    let state_to_open = EditorState::from_document(doc);
    let path = temp_op_path("open-centers-multi-root");
    save_to_path(&state_to_open, &path).expect("save succeeds");
    let mut current_path = None;

    assert!(open_path(&mut host, path.clone(), &mut current_path, None));

    let (min_x, min_y, max_x, max_y) =
        active_page_bbox(host.editor_state()).expect("opened content has bounds");
    let content_center_x = ((min_x + max_x) / 2.0) as f32;
    let content_center_y = ((min_y + max_y) / 2.0) as f32;
    let (canvas_w, canvas_h) = op_host_services::design_session::design_canvas_size(
        host.editor_state(),
        super::super::INITIAL_VIEWPORT_W,
        super::super::INITIAL_VIEWPORT_H,
    );
    let screen_center_x =
        host.editor_state().viewport.pan_x + content_center_x * host.editor_state().viewport.zoom;
    let screen_center_y =
        host.editor_state().viewport.pan_y + content_center_y * host.editor_state().viewport.zoom;

    assert!(
        (screen_center_x - canvas_w / 2.0).abs() < 0.5,
        "opened content should be horizontally centered: screen_center_x={screen_center_x}, canvas_w={canvas_w}"
    );
    assert!(
        (screen_center_y - canvas_h / 2.0).abs() < 0.5,
        "opened content should be vertically centered: screen_center_y={screen_center_y}, canvas_h={canvas_h}"
    );

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(sidecar_path(&path));
}

#[test]
fn opening_document_fits_and_centers_fit_content_root() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r#"{
          "version":"1.0.0",
          "children":[{
            "type":"frame","id":"root","name":"Explore",
            "x":12,"y":24,"width":390,"height":"fit_content",
            "layout":"vertical",
            "children":[
              {"type":"frame","id":"header","width":"fill_container","height":62},
              {"type":"frame","id":"content","width":"fill_container","height":616},
              {"type":"frame","id":"tabs","width":"fill_container","height":72}
            ]
          }]
        }"#,
    )
    .expect("fixture JSON parses")
    .value;
    let state_to_open = EditorState::from_document(doc);
    let path = temp_op_path("open-centers-fit-content-root");
    save_to_path(&state_to_open, &path).expect("save succeeds");
    let mut current_path = None;

    assert!(open_path(&mut host, path.clone(), &mut current_path, None));

    let (min_x, min_y, max_x, max_y) =
        active_page_bbox(host.editor_state()).expect("resolved content has bounds");
    assert!((max_y - min_y - 750.0).abs() < 0.01);
    let content_center_x = ((min_x + max_x) / 2.0) as f32;
    let content_center_y = ((min_y + max_y) / 2.0) as f32;
    let (canvas_w, canvas_h) = op_host_services::design_session::design_canvas_size(
        host.editor_state(),
        super::super::INITIAL_VIEWPORT_W,
        super::super::INITIAL_VIEWPORT_H,
    );
    let screen_center_x =
        host.editor_state().viewport.pan_x + content_center_x * host.editor_state().viewport.zoom;
    let screen_center_y =
        host.editor_state().viewport.pan_y + content_center_y * host.editor_state().viewport.zoom;

    assert!((screen_center_x - canvas_w / 2.0).abs() < 0.5);
    assert!((screen_center_y - canvas_h / 2.0).abs() < 0.5);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(sidecar_path(&path));
}

//! `~/.openpencil/templates/` round-trip tests.
//!
//! These touch two process-global things — the config root and the runtime
//! template registry — so they run under one lock and rebuild both from
//! scratch. The config root is redirected once per process by the test
//! harness, which is what keeps them off the developer's real `~/.openpencil`.

use super::*;
use std::sync::{Mutex, MutexGuard};

fn exclusive() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    crate::test_config_root::guard_user_config();
    op_editor_core::user_scene_templates::set_user_scene_templates(Vec::new());
    if let Ok(dir) = templates_dir() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    guard
}

fn entry(id: &str, name: &str) -> UserSceneTemplate {
    UserSceneTemplate {
        id: id.to_string(),
        name: name.to_string(),
        frames: 2,
        frame_width: 1920,
        frame_height: 1080,
        document: "{\"version\":\"1.0.0\",\"children\":[]}".to_string(),
        preview_jpeg: vec![0xFF, 0xD8, 0xFF, 0xD9],
    }
}

/// A saved template is three files in one directory, and a fresh scan
/// rebuilds the same entry under the same id — which is what makes a saved
/// template survive a restart.
#[test]
fn a_saved_template_is_stored_as_a_directory_and_reloads_with_the_same_id() {
    let _guard = exclusive();
    let saved = op_editor_core::user_scene_templates::load_user_scene_template(entry(
        "user:my-deck",
        "My Deck",
    ))
    .expect("saves");
    let dir = persist_user_scene_template(&saved.id).expect("persists");

    assert_eq!(dir.file_name().unwrap(), "my-deck");
    assert!(dir.join("document.op").exists());
    assert!(dir.join("preview.jpg").exists());
    assert!(dir.join("meta.json").exists());

    // A fresh session sees the same template under the same id.
    op_editor_core::user_scene_templates::set_user_scene_templates(Vec::new());
    load_user_scene_templates();
    let loaded = op_editor_core::user_scene_templates::user_scene_templates();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "user:my-deck");
    assert_eq!(loaded[0].name, "My Deck");
    assert_eq!(loaded[0].frames, 2);
    assert_eq!(loaded[0].frame_width, 1920);
    assert_eq!(loaded[0].frame_height, 1080);
    assert_eq!(loaded[0].preview_jpeg, vec![0xFF, 0xD8, 0xFF, 0xD9]);
}

#[test]
fn persist_writes_a_template_the_save_flow_registered() {
    let _guard = exclusive();
    let saved = op_editor_core::user_scene_templates::load_user_scene_template(entry(
        "user:my-deck",
        "My Deck",
    ))
    .expect("saves");
    let dir = persist_user_scene_template(&saved.id).expect("persists");
    assert_eq!(
        std::fs::read_to_string(dir.join("document.op")).expect("read"),
        "{\"version\":\"1.0.0\",\"children\":[]}"
    );

    assert!(
        persist_user_scene_template("user:never-registered").is_err(),
        "an unknown id has no content to write"
    );
}

#[test]
fn a_missing_preview_does_not_cost_the_template() {
    let _guard = exclusive();
    let saved = op_editor_core::user_scene_templates::load_user_scene_template(entry(
        "user:no-preview",
        "No Preview",
    ))
    .expect("saves");
    let dir = persist_user_scene_template(&saved.id).expect("persists");
    std::fs::remove_file(dir.join("preview.jpg")).expect("remove");

    op_editor_core::user_scene_templates::set_user_scene_templates(Vec::new());
    load_user_scene_templates();
    let loaded = op_editor_core::user_scene_templates::user_scene_templates();
    assert_eq!(loaded.len(), 1);
    assert!(
        loaded[0].preview_jpeg.is_empty(),
        "a missing preview loads as an empty preview"
    );
}

#[test]
fn delete_removes_the_directory_and_is_idempotent() {
    let _guard = exclusive();
    let saved = op_editor_core::user_scene_templates::load_user_scene_template(entry(
        "user:my-deck",
        "My Deck",
    ))
    .expect("saves");
    let dir = persist_user_scene_template(&saved.id).expect("persists");
    assert!(dir.exists());

    delete_user_scene_template(&saved.id).expect("deletes");
    assert!(!dir.exists());
    // The user asked for it gone and it is; a second pass must not error.
    delete_user_scene_template(&saved.id).expect("a missing directory is success");
}

/// The id reaches this through editor state, so it is re-validated rather
/// than trusted — a delete must not be able to walk out of the directory.
#[test]
fn delete_refuses_a_path_traversing_id() {
    let _guard = exclusive();
    let victim = templates_dir().expect("dir").join("keep-me");
    std::fs::create_dir_all(&victim).expect("create");

    for id in [
        "user:../keep-me",
        "user:..",
        "user:sub/keep-me",
        "user:",
        "user:keep me",
    ] {
        assert!(
            delete_user_scene_template(id).is_err(),
            "{id} must be refused"
        );
    }
    assert!(victim.exists(), "nothing outside the directory was touched");
}

#[test]
fn failed_delete_restores_the_registry_clears_the_queue_and_warns() {
    let _guard = exclusive();
    let saved = op_editor_core::user_scene_templates::load_user_scene_template(entry(
        "user:retry-me",
        "Retry Me",
    ))
    .expect("registers");
    // Mirror the shared press flow: memory disappears before the desktop gets
    // its chance to remove the directory.
    assert!(op_editor_core::user_scene_templates::remove_user_scene_template(&saved.id).is_some());
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .scene_template_center
        .queue_template_delete(&saved.id);

    let mut reloaded = false;
    let restore = saved.as_ref().clone();
    assert!(drain_pending_template_delete_with(
        &mut host,
        7_777,
        |_| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
        || {
            reloaded = true;
            op_editor_core::user_scene_templates::load_user_scene_template(restore.clone())
                .expect("restores from disk");
        },
    ));

    assert!(reloaded, "a failed disk delete must trigger recovery");
    assert_eq!(
        op_editor_core::user_scene_templates::user_scene_templates()[0].id,
        saved.id,
        "the card must return while its directory still exists"
    );
    assert!(
        host.editor_state()
            .editor_ui
            .scene_template_center
            .pending_template_delete
            .is_empty(),
        "a redraw must not immediately retry a persistent disk error"
    );
    let toast = host
        .editor_state()
        .editor_ui
        .visible_toast(7_777)
        .expect("delete failure is user-visible");
    assert_eq!(toast.i18n_key, "sceneTemplate.deleteFailed");
    assert_eq!(toast.level, EditorToastLevel::Warn);
}

#[test]
fn file_menu_template_request_commits_a_focused_property_draft_before_snapshotting() {
    let _guard = exclusive();
    // The drained save renders a template preview through the shared export
    // painter, whose discovery pass DRAINS the process-global pending-decode
    // registry (`ensure_images_decoded` → `take_pending_decodes(usize::MAX)`).
    // Hold the decode-test lock so that drain cannot steal an entry a
    // concurrently running `image_decode_host` test just queued (stolen
    // avatar decode, linux-aarch64 CI 2026-08-29).
    let _decode_guard = crate::image_decode_host::lock_decode_test_registry();
    let mut app = crate::DesktopApp::new(None);
    let state = app.host.editor_state_mut();
    state.set_single_selection(op_editor_core::NodeId::new("n10"));
    state.ui.property_focus = Some(op_editor_core::PropertyFocus::SizeW);
    state.ui.property_input.set_text("321");

    assert!(app
        .host
        .editor_state_mut()
        .editor_ui
        .scene_template_center
        .request_save_current());
    assert!(app.drain_save_current_template_request());
    assert!(!app.drain_save_current_template_request());

    let state = app.host.editor_state();
    assert!(state.ui.property_focus.is_none());
    assert_eq!(
        op_editor_core::own_bounds(state.selected_node().expect("selected starter node")).w,
        321.0
    );
    let saved = op_editor_core::user_scene_templates::user_scene_templates()
        .into_iter()
        .find(|template| template.name == "untitled-template")
        .expect("saved template is registered");
    let document = jian_ops_schema::load_str(&saved.document)
        .expect("saved template document parses")
        .value;
    let saved_state = op_editor_core::EditorState::from_document(document);
    let saved_node = op_editor_core::walkers::find_node(
        saved_state.active_children(),
        &op_editor_core::NodeId::new("n10"),
    )
    .expect("starter node is present in the template snapshot");
    assert_eq!(op_editor_core::own_bounds(saved_node).w, 321.0);
}

/// The directory is user-editable, so one incomplete save must not cost the
/// rest of the collection.
#[test]
fn a_half_written_directory_is_skipped_and_the_rest_of_the_scan_survives() {
    let _guard = exclusive();
    let dir = templates_dir().expect("dir");
    std::fs::create_dir_all(dir.join("good-one")).expect("create");
    std::fs::write(
        dir.join("good-one/meta.json"),
        r#"{"name":"Good","frames":1,"frameWidth":10,"frameHeight":10}"#,
    )
    .expect("write");
    std::fs::write(dir.join("good-one/document.op"), "{}").expect("write");
    // A directory with a document but no meta, and one with a meta but no
    // document, and a stray file — all skipped, none fatal.
    std::fs::create_dir_all(dir.join("no-meta")).expect("create");
    std::fs::write(dir.join("no-meta/document.op"), "{}").expect("write");
    std::fs::create_dir_all(dir.join("no-document")).expect("create");
    std::fs::write(
        dir.join("no-document/meta.json"),
        r#"{"name":"No Doc","frames":1,"frameWidth":10,"frameHeight":10}"#,
    )
    .expect("write");
    std::fs::write(dir.join("notes.txt"), "hello").expect("write");

    load_user_scene_templates();
    let loaded = op_editor_core::user_scene_templates::user_scene_templates();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "user:good-one");
}

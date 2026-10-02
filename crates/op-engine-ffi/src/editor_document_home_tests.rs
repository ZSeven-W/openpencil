//! Regressions for Home -> draft -> save/reopen through mobile ABI drains.

use super::*;
use crate::desc::{Callbacks, CreateOptions};
use crate::editor::op_editor_take_shell_action;
use crate::lifecycle::{call_session, OpEngine};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    engine: OpEngine,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "openpencil-home-save-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let private =
            std::env::temp_dir().join(format!("openpencil-ffi-doc-save-{}", std::process::id()));
        std::fs::create_dir_all(&private).unwrap();
        op_config_store::redirect_user_root_for_tests(&private);
        let mut engine = OpEngine::new(
            Session::new(CreateOptions {
                document: include_str!(
                    "../../op-editor-core/assets/scene_templates/daily-sign-card.op"
                )
                .to_owned(),
                width: 390.0,
                height: 844.0,
                dpr: 1.0,
                callbacks: Callbacks::default(),
                asset_base: None,
                editor_mode: true,
                documents_root: None,
            })
            .unwrap(),
        );
        // Avoid migrating another test's process-global fallback directory.
        engine.session_mut_for_test().document_save.root = Some(root.clone());
        Self { engine, root }
    }

    fn session(&mut self) -> &mut Session {
        self.engine.session_mut_for_test()
    }

    fn edit(&mut self, name: &str) {
        let state = self.session().editor_mut().unwrap().editor_state_mut();
        state.doc.name = Some(name.to_owned());
        state.chat.title = "Same title".into();
        state.mark_document_changed();
    }

    fn swap_home(&mut self) {
        // Real taps run inside call_session. Successive calls may precede a
        // shell-action drain, so capture must happen at this boundary too.
        assert_eq!(
            unsafe {
                call_session(&mut self.engine, |session| {
                    assert!(session.editor_mut()?.start_fresh_document_for_home());
                    Ok(())
                })
            },
            OpStatus::Ok
        );
    }

    fn drain(&mut self, action: Option<op_editor_core::FileAction>) -> OpStatus {
        self.session()
            .editor_mut()
            .unwrap()
            .editor_state_mut()
            .editor_ui
            .pending_file_action = action;
        let mut shell_action = -1;
        unsafe { op_editor_take_shell_action(&mut self.engine, &mut shell_action) }
    }

    fn recent_paths(&mut self) -> Vec<PathBuf> {
        self.session()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui
            .recent_files
            .iter()
            .map(|entry| PathBuf::from(&entry.path))
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn saved_name(path: &Path) -> String {
    let source = std::fs::read_to_string(path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&source).unwrap();
    value["name"].as_str().unwrap().to_owned()
}

#[test]
fn home_draft_never_overwrites_a_bound_file_and_rescue_reopens() {
    let mut fixture = Fixture::new();
    fixture.edit("A saved");
    let original = fixture.root.join("original.op");
    write_current_document(fixture.session(), &original).unwrap();
    finish_successful_save(fixture.session(), original.clone(), false);
    fixture.edit("A unsaved edits");

    fixture.swap_home();
    let player_name = fixture.session().state.doc.name.clone();
    assert_eq!(
        player_name,
        fixture
            .session()
            .editor_mut()
            .unwrap()
            .editor_state()
            .doc
            .name
    );
    fixture.edit("B new work");
    assert!(fixture.session().document_save.bound_path().is_none());
    assert_eq!(
        fixture.drain(Some(op_editor_core::FileAction::Save)),
        OpStatus::Ok
    );
    assert_eq!(saved_name(&original), "A saved");
    assert!(
        fixture
            .session()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui
            .save_name_dialog
            .open
    );
    let rescued = fixture.recent_paths()[0].clone();
    assert_eq!(saved_name(&rescued), "A unsaved edits");
    assert_eq!(
        fixture.drain(Some(op_editor_core::FileAction::OpenRecent(0))),
        OpStatus::Ok
    );
    assert_eq!(
        fixture.session().document_save.bound_path(),
        Some(rescued.as_path())
    );
    assert_eq!(
        fixture
            .session()
            .editor_mut()
            .unwrap()
            .editor_state()
            .doc
            .name
            .as_deref(),
        Some("A unsaved edits")
    );
    assert!(!fixture
        .session()
        .editor_mut()
        .unwrap()
        .editor_state()
        .is_dirty());
    assert!(fixture
        .recent_paths()
        .iter()
        .any(|path| saved_name(path) == "B new work"));
    assert_eq!(saved_name(&original), "A saved");
}

#[test]
fn repeated_drafts_and_a_failed_rescue_keep_every_snapshot_until_retry() {
    let mut fixture = Fixture::new();
    fixture.edit("first draft");
    fixture.swap_home();
    fixture.edit("second draft");
    fixture.swap_home();
    fixture.edit("third draft");
    assert_eq!(fixture.session().document_save.rescued.len(), 2);

    let blocked = fixture.root.join("not-a-directory");
    std::fs::write(&blocked, b"block the save").unwrap();
    fixture.session().document_save.root = Some(blocked);
    assert_eq!(
        fixture.drain(Some(op_editor_core::FileAction::Save)),
        OpStatus::Ok
    );
    assert_eq!(fixture.session().document_save.rescued.len(), 2);
    assert!(fixture.session().document_save.recovery_error_reported);
    assert!(
        fixture
            .session()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui
            .save_name_dialog
            .open,
        "a failed recovery must not block saving the current work elsewhere"
    );
    assert!(fixture.session().document_save.bound_path().is_none());

    fixture.session().document_save.root = Some(fixture.root.clone());
    assert_eq!(fixture.drain(None), OpStatus::Ok);
    assert!(fixture.session().document_save.rescued.is_empty());
    let paths = fixture.recent_paths();
    assert_eq!(paths.len(), 2);
    assert_ne!(
        paths[0], paths[1],
        "same-title drafts are never overwritten"
    );
    let mut names: Vec<_> = paths.iter().map(|path| saved_name(path)).collect();
    names.sort();
    assert_eq!(names, ["first draft", "second draft"]);
    assert_eq!(fixture.drain(None), OpStatus::Ok);
    assert_eq!(
        fixture.recent_paths().len(),
        2,
        "successful drain is one-shot"
    );
}

#[test]
fn suspend_captures_a_home_swap_before_writing_the_old_binding() {
    let mut fixture = Fixture::new();
    fixture.edit("saved original");
    let original = fixture.root.join("original.op");
    write_current_document(fixture.session(), &original).unwrap();
    finish_successful_save(fixture.session(), original.clone(), false);
    // Exercise the drain fallback without the usual ABI post-call capture.
    fixture
        .session()
        .editor_mut()
        .unwrap()
        .start_fresh_document_for_home();
    fixture.edit("incoming unsaved draft");
    flush_on_suspend(fixture.session());
    assert_eq!(saved_name(&original), "saved original");
    assert!(fixture.session().document_save.bound_path().is_none());
    assert!(fixture
        .session()
        .editor_mut()
        .unwrap()
        .editor_state()
        .is_dirty());
}

#[test]
fn pending_recovery_does_not_change_which_recent_row_was_tapped() {
    let mut fixture = Fixture::new();
    fixture.edit("saved original");
    let original = fixture.root.join("original.op");
    write_current_document(fixture.session(), &original).unwrap();
    finish_successful_save(fixture.session(), original.clone(), false);
    fixture.edit("unsaved original changes");
    fixture.swap_home();
    fixture.edit("incoming draft");
    assert_eq!(
        fixture.recent_paths().as_slice(),
        std::slice::from_ref(&original)
    );
    assert_eq!(
        fixture.drain(Some(op_editor_core::FileAction::OpenRecent(0))),
        OpStatus::Ok
    );
    assert_eq!(
        fixture.session().document_save.bound_path(),
        Some(original.as_path())
    );
    assert_eq!(
        fixture
            .session()
            .editor_mut()
            .unwrap()
            .editor_state()
            .doc
            .name
            .as_deref(),
        Some("saved original")
    );
    assert_eq!(
        fixture.recent_paths().len(),
        3,
        "both displaced revisions were rescued"
    );
}

#[test]
fn invalid_recent_preserves_the_current_document_and_consumes_the_request() {
    let mut fixture = Fixture::new();
    fixture.edit("keep this draft");
    let corrupt = fixture.root.join("corrupt.op");
    std::fs::write(&corrupt, b"not a document").unwrap();
    home::touch_recent(fixture.session(), &corrupt);
    assert_eq!(
        fixture.drain(Some(op_editor_core::FileAction::OpenRecent(0))),
        OpStatus::BadDocument
    );
    let state = fixture.session().editor_mut().unwrap().editor_state();
    assert_eq!(state.doc.name.as_deref(), Some("keep this draft"));
    assert!(state.is_dirty());
    assert!(state.editor_ui.pending_file_action.is_none());
}

#[test]
fn recovery_uses_the_outgoing_image_thumbnail_snapshot() {
    let mut fixture = Fixture::new();
    let source = format!(
        "https://example.invalid/home-recovery-{}.png",
        std::process::id()
    );
    let paint_id = jian_ops_schema::node::image_src::paint_image_id(&source);
    let document = serde_json::json!({
        "version": "1.0.0",
        "children": [{"type": "image", "id": "photo", "src": source, "width": 200, "height": 200}]
    });
    crate::editor::install_document_source(fixture.session(), &document.to_string(), None).unwrap();
    fixture.edit("image document");
    jian_ops_schema::image_thumbs::store_thumb(paint_id, vec![0xff, 0xd8, 0xff, 0xd9]);
    fixture.swap_home();
    // The live registry is free to change after the old work is parked.
    jian_ops_schema::image_thumbs::store_thumb(paint_id, vec![1, 2, 3]);
    assert_eq!(fixture.drain(None), OpStatus::Ok);
    let rescued = fixture.recent_paths()[0].clone();
    let bytes = std::fs::read_to_string(rescued).unwrap();
    let value: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    assert_eq!(value["imageThumbs"][paint_id.to_string()], "/9j/2Q==");
}

#[test]
fn home_page_choice_starts_the_correct_example_through_pointer_abi() {
    use op_editor_core::{size_class::EditorSizeClass, AppPages};
    for (pages, expected) in [(AppPages::Single, 1), (AppPages::Multiple, 3)] {
        let mut fixture = Fixture::new();
        let rect = {
            let state = fixture.session().editor_mut().unwrap().editor_state_mut();
            state.editor_ui.touch = true;
            state.editor_ui.size_class = EditorSizeClass::Compact;
            state.editor_ui.home.visible = true;
            state.editor_ui.home.set_app_pages(pages);
            op_editor_ui::widgets::HomeSurface::for_editor(state)
                .unwrap()
                .layout(390.0, 844.0)
                .send
        };
        assert_eq!(
            unsafe {
                crate::editor_pointer::op_editor_press_at(
                    &mut fixture.engine,
                    rect.origin.x + rect.size.x / 2.0,
                    rect.origin.y + rect.size.y / 2.0,
                    10_000,
                )
            },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe {
                crate::op_editor_release_at(
                    &mut fixture.engine,
                    rect.origin.x + rect.size.x / 2.0,
                    rect.origin.y + rect.size.y / 2.0,
                    10_001,
                )
            },
            OpStatus::Ok
        );
        let state = fixture.session().editor_mut().unwrap().editor_state();
        assert!(
            !state.editor_ui.home.visible,
            "{pages:?} must start the example"
        );
        assert_eq!(
            op_editor_core::preview_slideshow::active_page_boards(state).len(),
            expected
        );
    }
}

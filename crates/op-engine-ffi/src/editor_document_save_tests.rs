//! Mobile Save / Save As, driven through the real FFI surface the shells
//! call: the More-menu press queues `FileAction::Save`/`SaveAs`,
//! `op_editor_take_shell_action` opens the engine-painted name dialog,
//! `op_editor_text` / `op_editor_key(KEY_ENTER)` name and confirm it, and
//! the next drain writes a canonical `.op` file into the destination
//! directory — the shell's user-visible documents root when it passed one,
//! else the private `<storage_root>/documents` fallback.

use crate::desc::{Callbacks, CreateOptions};
use crate::editor::{op_editor_key, op_editor_take_shell_action, op_editor_text, KEY_ENTER};
use crate::editor_auth::SHELL_ACTION_NONE;
use crate::lifecycle::{OpEngine, Session};
use crate::OpStatus;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The private storage root is a process-wide first-write-wins redirect, so
/// every case in this file shares one `documents/` fallback directory.
/// Migration cases deliberately drain that directory, which would move
/// another case's file out from under it — serialize the whole file.
static SANDBOX: Mutex<()> = Mutex::new(());

fn exclusive() -> MutexGuard<'static, ()> {
    SANDBOX.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A fresh, empty directory standing in for a shell-provided user-visible
/// documents root (iOS `NSDocumentDirectory`).
fn visible_root(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "openpencil-ffi-visible-{}-{label}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("visible root");
    dir
}

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

fn scratch_root() -> PathBuf {
    // First redirect wins process-wide; every test installs the same
    // scratch directory so no case can ever write ~/.openpencil.
    let root = std::env::temp_dir().join(format!("openpencil-ffi-doc-save-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("scratch root");
    op_config_store::redirect_user_root_for_tests(&root).to_path_buf()
}

fn phone_engine() -> OpEngine {
    phone_engine_with_documents_root(None)
}

fn phone_engine_with_documents_root(documents_root: Option<PathBuf>) -> OpEngine {
    scratch_root();
    OpEngine::new(
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: 390.0,
            height: 844.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: documents_root
                .map(|root| root.to_str().expect("utf-8 path").to_owned()),
        })
        .expect("editor session"),
    )
}

/// Drive the name dialog end to end and return the bound path.
fn save_as_named(engine: &mut OpEngine, name: &str) -> PathBuf {
    let pointer = engine as *mut OpEngine;
    queue_file_action(engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    type_text(pointer, name);
    assert_eq!(unsafe { op_editor_key(pointer, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    saved_path(engine).expect("save binds a path")
}

fn queue_file_action(engine: &mut OpEngine, action: op_editor_core::FileAction) {
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state_mut()
        .editor_ui
        .pending_file_action = Some(action);
}

fn drain(pointer: *mut OpEngine) -> i32 {
    let mut action = -1;
    assert_eq!(
        unsafe { op_editor_take_shell_action(pointer, &mut action) },
        OpStatus::Ok
    );
    action
}

fn type_text(pointer: *mut OpEngine, text: &str) {
    assert_eq!(
        unsafe { op_editor_text(pointer, text.as_ptr(), text.len()) },
        OpStatus::Ok
    );
}

fn saved_path(engine: &mut OpEngine) -> Option<PathBuf> {
    engine
        .session_mut_for_test()
        .document_save
        .bound_path()
        .map(std::path::Path::to_path_buf)
}

#[test]
fn first_save_prompts_for_a_name_and_writes_a_canonical_file() {
    let _guard = exclusive();
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue_file_action(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    {
        let ui = &engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui;
        assert!(
            ui.save_name_dialog.open,
            "first save must prompt for a name"
        );
        assert!(!ui.save_name_dialog.save_as);
        // Untitled doc on the default zh-CN locale seeds 未命名, selected.
        assert_eq!(ui.save_name_dialog.input.text(), "未命名");
    }

    // Typing replaces the selected seed; Enter confirms.
    type_text(pointer, "周报海报");
    assert_eq!(unsafe { op_editor_key(pointer, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);

    let path = saved_path(&mut engine).expect("save binds a sandbox path");
    assert!(path.ends_with("documents/周报海报.op"), "path: {path:?}");
    let written = std::fs::read_to_string(&path).expect("saved file");
    let loaded = jian_ops_schema::load_str(&written).expect("canonical round-trip");
    jian_ops_schema::image_thumbs::discard_for_document(&loaded.value);
    let value: serde_json::Value = serde_json::from_str(&written).expect("json");
    assert!(value.get("editorMeta").is_some(), "editorMeta is persisted");

    let host = engine.session_mut_for_test().editor_mut().unwrap();
    let state = host.editor_state();
    assert!(!state.editor_ui.save_name_dialog.open);
    assert_eq!(
        state.editor_ui.file_name_display.as_deref(),
        Some("周报海报.op")
    );
    assert!(!state.is_dirty(), "a fresh save leaves the document clean");
}

#[test]
fn resave_overwrites_in_place_without_prompting() {
    let _guard = exclusive();
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;

    // First save through the dialog.
    queue_file_action(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    type_text(pointer, "resave-fixture");
    assert_eq!(unsafe { op_editor_key(pointer, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    let path = saved_path(&mut engine).expect("bound path");

    // Edit the document, then Save again: no dialog, same file updated.
    {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        let state = host.editor_state_mut();
        state.doc.name = Some("edited-in-place".into());
        state.mark_document_changed();
        assert!(state.is_dirty());
    }
    queue_file_action(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        assert!(!host.editor_state().editor_ui.save_name_dialog.open);
        assert!(!host.editor_state().is_dirty());
    }
    assert_eq!(saved_path(&mut engine).as_deref(), Some(path.as_path()));
    let written = std::fs::read_to_string(&path).expect("resaved file");
    assert!(written.contains("edited-in-place"));
}

#[test]
fn save_as_writes_a_copy_and_switches_the_binding() {
    let _guard = exclusive();
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue_file_action(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    type_text(pointer, "saveas-fixture");
    assert_eq!(unsafe { op_editor_key(pointer, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    let original = saved_path(&mut engine).expect("original path");

    // Save As with the seeded (same) name: the dialog opens pre-filled with
    // the current stem and the write dedupes to "<stem> 2.op".
    queue_file_action(&mut engine, op_editor_core::FileAction::SaveAs);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    {
        let ui = &engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui;
        assert!(ui.save_name_dialog.open);
        assert!(ui.save_name_dialog.save_as);
        assert_eq!(ui.save_name_dialog.input.text(), "saveas-fixture");
    }
    assert_eq!(unsafe { op_editor_key(pointer, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);

    let copy = saved_path(&mut engine).expect("copy path");
    assert_ne!(copy, original, "Save As never overwrites the original");
    assert!(copy.ends_with("documents/saveas-fixture 2.op"), "{copy:?}");
    assert!(original.exists(), "the original file survives");
    assert!(copy.exists(), "the copy exists");
    assert_eq!(
        engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui
            .file_name_display
            .as_deref(),
        Some("saveas-fixture 2.op")
    );
}

#[test]
fn blank_name_cannot_confirm_and_escape_cancels() {
    let _guard = exclusive();
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue_file_action(&mut engine, op_editor_core::FileAction::SaveAs);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    // Clear the seeded name (it opens selected) and try to confirm.
    type_text(pointer, " ");
    assert_eq!(unsafe { op_editor_key(pointer, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    {
        let ui = &engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui;
        assert!(
            ui.save_name_dialog.open,
            "a blank name must not confirm the dialog"
        );
    }
    assert_eq!(saved_path(&mut engine), None, "nothing was written");

    // Escape cancels the prompt without saving.
    assert_eq!(
        unsafe { op_editor_key(pointer, crate::editor::KEY_ESCAPE) },
        OpStatus::Ok
    );
    {
        let ui = &engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui;
        assert!(!ui.save_name_dialog.open);
    }
    assert_eq!(saved_path(&mut engine), None);
}

#[test]
fn suspend_flushes_a_bound_dirty_document_but_never_an_unsaved_one() {
    let _guard = exclusive();
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;

    // Unsaved document: suspend must not invent a file.
    engine.session_mut_for_test().suspend();
    assert_eq!(saved_path(&mut engine), None);
    engine
        .session_mut_for_test()
        .resume(None)
        .expect("resume without surface");

    // Bind a sandbox file, dirty the doc, then suspend.
    queue_file_action(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    type_text(pointer, "suspend-flush");
    assert_eq!(unsafe { op_editor_key(pointer, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    let path = saved_path(&mut engine).expect("bound path");
    {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        let state = host.editor_state_mut();
        state.doc.name = Some("flushed-on-suspend".into());
        state.mark_document_changed();
    }
    engine.session_mut_for_test().suspend();
    let written = std::fs::read_to_string(&path).expect("flushed file");
    assert!(written.contains("flushed-on-suspend"));
    assert!(!engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .is_dirty());
}

#[test]
fn replacing_the_document_drops_the_sandbox_binding() {
    let _guard = exclusive();
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue_file_action(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    type_text(pointer, "replaced-doc");
    assert_eq!(unsafe { op_editor_key(pointer, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    assert!(saved_path(&mut engine).is_some());

    // File ▸ New replaces the document; the binding must not leak onto it.
    queue_file_action(&mut engine, op_editor_core::FileAction::New);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    assert_eq!(saved_path(&mut engine), None);
    assert!(
        !engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui
            .save_name_dialog
            .open
    );
}

#[test]
fn a_documents_root_redirects_saves_out_of_the_private_storage_root() {
    let _guard = exclusive();
    let visible = visible_root("save-target");
    let mut engine = phone_engine_with_documents_root(Some(visible.clone()));

    let path = save_as_named(&mut engine, "visible-poster");
    assert_eq!(path, visible.join("visible-poster.op"));
    assert!(path.is_file(), "the document landed in the visible root");
    assert!(
        !path.starts_with(scratch_root()),
        "a shell-provided documents root must not live under the private storage root: {path:?}"
    );
    // Save As dedupes inside the visible root, exactly as it does in the
    // private fallback.
    let pointer = &mut engine as *mut OpEngine;
    queue_file_action(&mut engine, op_editor_core::FileAction::SaveAs);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    assert_eq!(unsafe { op_editor_key(pointer, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    assert_eq!(
        saved_path(&mut engine),
        Some(visible.join("visible-poster 2.op"))
    );
}

#[test]
fn without_a_documents_root_saves_stay_in_the_private_fallback() {
    let _guard = exclusive();
    let mut engine = phone_engine_with_documents_root(None);

    let path = save_as_named(&mut engine, "fallback-poster");
    assert_eq!(
        path,
        scratch_root().join("documents").join("fallback-poster.op"),
        "shells that pass no documents root keep today's private location"
    );
}

#[test]
fn migration_moves_legacy_documents_and_dedupes_collisions() {
    let legacy = visible_root("migrate-legacy");
    let target = visible_root("migrate-target");
    std::fs::write(legacy.join("poster.op"), b"legacy-poster").expect("legacy doc");
    std::fs::write(legacy.join("deck.OP"), b"legacy-deck").expect("legacy deck");
    // Not documents: a stray temp file and an unrelated sibling.
    std::fs::write(legacy.join(".poster.op.tmp"), b"partial").expect("temp");
    std::fs::write(legacy.join("notes.txt"), b"notes").expect("txt");
    // The visible root already holds a same-named document.
    std::fs::write(target.join("poster.op"), b"already-here").expect("existing");

    let moved = crate::editor_document::migrate_documents(&legacy, &target).expect("migrate");
    assert_eq!(moved, 2, "both .op documents moved, nothing else");
    assert_eq!(
        std::fs::read_to_string(target.join("poster.op")).expect("existing survives"),
        "already-here",
        "migration never clobbers a same-named document"
    );
    assert_eq!(
        std::fs::read_to_string(target.join("poster 2.op")).expect("deduped copy"),
        "legacy-poster"
    );
    assert_eq!(
        std::fs::read_to_string(target.join("deck.op")).expect("case-insensitive extension"),
        "legacy-deck"
    );
    assert!(!legacy.join("poster.op").exists(), "the legacy file moved");
    assert!(
        legacy.join("notes.txt").is_file(),
        "non-documents are left alone"
    );

    // Idempotent: a second pass has nothing left to move.
    assert_eq!(
        crate::editor_document::migrate_documents(&legacy, &target).expect("second pass"),
        0
    );
    assert!(!target.join("poster 3.op").exists());
}

#[test]
fn migration_is_a_no_op_without_a_legacy_directory_or_onto_itself() {
    let target = visible_root("migrate-noop");
    let missing = target.join("never-created");
    assert_eq!(
        crate::editor_document::migrate_documents(&missing, &target).expect("absent legacy"),
        0
    );

    std::fs::write(target.join("poster.op"), b"same-dir").expect("doc");
    assert_eq!(
        crate::editor_document::migrate_documents(&target, &target).expect("same directory"),
        0,
        "a shell whose documents root IS the legacy directory must not churn"
    );
    assert!(target.join("poster.op").is_file());
    assert!(!target.join("poster 2.op").exists());
}

#[test]
fn creating_an_engine_migrates_private_saves_into_the_visible_root() {
    let _guard = exclusive();
    // A document saved by the previous build, in the private location.
    let mut old_engine = phone_engine();
    let old_path = save_as_named(&mut old_engine, "carried-over");
    assert!(old_path.starts_with(scratch_root()));
    drop(old_engine);

    // The upgraded shell now passes a visible documents root.
    let visible = visible_root("migrate-on-create");
    let engine = phone_engine_with_documents_root(Some(visible.clone()));
    assert!(
        visible.join("carried-over.op").is_file(),
        "startup carried the private document into the visible root"
    );
    assert!(!old_path.exists(), "the private copy moved rather than sat");
    drop(engine);

    // Second launch: nothing left behind, no duplicate.
    let _second = phone_engine_with_documents_root(Some(visible.clone()));
    assert!(!visible.join("carried-over 2.op").exists());
}

/// The migration helper is reachable from the shell-facing entry point too:
/// a session with no documents root never touches the legacy directory.
#[test]
fn migrate_legacy_documents_is_inert_without_a_documents_root() {
    let state = crate::editor_document::DocumentSaveShellState::with_root(None);
    assert_eq!(
        crate::editor_document::migrate_legacy_documents(&state).expect("inert"),
        0
    );
}

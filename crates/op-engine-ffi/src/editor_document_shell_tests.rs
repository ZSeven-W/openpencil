//! Picker-backed Save / Save As, driven through the real C ABI the three
//! mobile shells call.
//!
//! Each case walks the whole round trip — `SHELL_ACTION_SAVE_DOCUMENT`,
//! `op_editor_copy_save_file_name` / `op_editor_copy_save_target`,
//! `op_editor_stage_save_to_path`, then `op_editor_commit_save` or
//! `op_editor_cancel_save` — because the interesting bugs live in what the
//! engine believes about the document *between* those calls.

use crate::desc::{Callbacks, CreateOptions};
use crate::editor::op_editor_take_shell_action;
use crate::editor_auth::SHELL_ACTION_NONE;
use crate::editor_document_shell::{
    op_editor_cancel_save, op_editor_commit_save, op_editor_configure_save_picker,
    op_editor_copy_save_file_name, op_editor_copy_save_target, op_editor_stage_save_to_path,
    SHELL_ACTION_SAVE_DOCUMENT,
};
use crate::lifecycle::{OpEngine, Session};
use crate::OpStatus;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// A picker shell's app-private staging directory: created by the shell,
/// removed by the shell, exactly as `MainActivity` / `DocumentShell` /
/// `DocumentSaveCoordinator` do.
struct Staging(PathBuf);

impl Staging {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "openpencil-ffi-save-staging-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("staging dir");
        Self(dir)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Redirect the private storage root, which is a process-wide
/// first-write-wins setting. Same scratch directory the path-based save
/// tests install, so whichever file wins the race both stay out of
/// `~/.openpencil`.
fn scratch_root() {
    let root = std::env::temp_dir().join(format!("openpencil-ffi-doc-save-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("scratch root");
    op_config_store::redirect_user_root_for_tests(&root);
}

/// An editor engine whose shell declared the save picker. `documents_root`
/// is `None` by default — the Android / HarmonyOS shape, where no
/// user-visible directory exists at all and the flow must still work.
fn picker_engine() -> OpEngine {
    picker_engine_with_documents_root(None)
}

fn picker_engine_with_documents_root(documents_root: Option<PathBuf>) -> OpEngine {
    scratch_root();
    let mut engine = OpEngine::new(
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
    );
    let pointer = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { op_editor_configure_save_picker(pointer, true) },
        OpStatus::Ok
    );
    engine
}

fn queue(engine: &mut OpEngine, action: op_editor_core::FileAction) {
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

/// The shell's "size query, then copy" read of an engine string.
fn copy_string(
    pointer: *mut OpEngine,
    read: unsafe extern "C" fn(*mut OpEngine, *mut u8, usize, *mut usize) -> OpStatus,
) -> Option<String> {
    let mut required = 0usize;
    assert_eq!(
        unsafe { read(pointer, std::ptr::null_mut(), 0, &mut required) },
        OpStatus::Ok
    );
    if required == 0 {
        return None;
    }
    let mut bytes = vec![0_u8; required];
    assert_eq!(
        unsafe { read(pointer, bytes.as_mut_ptr(), bytes.len(), &mut required) },
        OpStatus::Ok
    );
    Some(String::from_utf8(bytes).expect("utf-8"))
}

fn stage(pointer: *mut OpEngine, path: &Path) -> OpStatus {
    let path = path.to_str().expect("utf-8 path");
    unsafe { op_editor_stage_save_to_path(pointer, path.as_ptr(), path.len()) }
}

fn commit(pointer: *mut OpEngine, handle: &str, name: &str) -> OpStatus {
    unsafe {
        op_editor_commit_save(
            pointer,
            handle.as_ptr(),
            handle.len(),
            name.as_ptr(),
            name.len(),
        )
    }
}

fn dirty(engine: &mut OpEngine) -> bool {
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .is_dirty()
}

fn touch(engine: &mut OpEngine, name: &str) {
    let state = engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state_mut();
    state.doc.name = Some(name.into());
    state.mark_document_changed();
}

/// The full happy path: prompt, stage canonical bytes, commit.
#[test]
fn a_first_save_prompts_and_a_commit_binds_the_picked_destination() {
    let staging = Staging::new();
    let mut engine = picker_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    // The engine-painted name dialog stays shut: the picker owns naming.
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

    let name = copy_string(pointer, op_editor_copy_save_file_name).expect("suggested name");
    assert!(name.ends_with(".op"), "name: {name}");
    assert_eq!(
        copy_string(pointer, op_editor_copy_save_target),
        None,
        "an unbound document must make the shell prompt"
    );

    let staged = staging.file(&name);
    assert_eq!(stage(pointer, &staged), OpStatus::Ok);
    let written = std::fs::read_to_string(&staged).expect("staged bytes");
    let loaded = jian_ops_schema::load_str(&written).expect("canonical round-trip");
    jian_ops_schema::image_thumbs::discard_for_document(&loaded.value);
    let value: serde_json::Value = serde_json::from_str(&written).expect("json");
    assert!(value.get("editorMeta").is_some(), "editorMeta is persisted");

    // Staging alone must not consume the round trip: the destination has not
    // received anything yet, so the shell still owes a commit or a cancel.
    assert!(engine
        .session_mut_for_test()
        .document_save
        .pending
        .is_some());
    assert!(engine
        .session_mut_for_test()
        .document_save
        .shell_binding()
        .is_none());

    assert_eq!(
        commit(pointer, "content://docs/tree/9/document/42", "周报海报.op"),
        OpStatus::Ok
    );
    assert!(!dirty(&mut engine), "a committed save leaves it clean");
    assert!(engine
        .session_mut_for_test()
        .document_save
        .pending
        .is_none());
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    assert_eq!(
        host.editor_state().editor_ui.file_name_display.as_deref(),
        Some("周报海报.op")
    );
}

/// The whole point of persisting the handle: the second Save rewrites the
/// same destination instead of asking again.
#[test]
fn a_second_plain_save_rewrites_the_bound_handle_without_prompting() {
    let staging = Staging::new();
    let mut engine = picker_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    let name = copy_string(pointer, op_editor_copy_save_file_name).expect("name");
    assert_eq!(stage(pointer, &staging.file(&name)), OpStatus::Ok);
    assert_eq!(
        commit(pointer, "content://picked/1", "poster.op"),
        OpStatus::Ok
    );

    touch(&mut engine, "edited-after-first-save");
    assert!(dirty(&mut engine));

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(
        copy_string(pointer, op_editor_copy_save_target).as_deref(),
        Some("content://picked/1"),
        "a bound document rewrites its handle silently"
    );
    assert_eq!(
        copy_string(pointer, op_editor_copy_save_file_name).as_deref(),
        Some("poster.op"),
        "the destination keeps the name the picker gave it"
    );
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

    let staged = staging.file("poster.op");
    assert_eq!(stage(pointer, &staged), OpStatus::Ok);
    assert!(std::fs::read_to_string(&staged)
        .expect("restaged bytes")
        .contains("edited-after-first-save"));
    assert_eq!(
        commit(pointer, "content://picked/1", "poster.op"),
        OpStatus::Ok
    );
    assert!(!dirty(&mut engine));
}

/// Save As is the escape hatch: it must re-prompt even when bound.
#[test]
fn save_as_always_prompts_even_with_a_binding() {
    let staging = Staging::new();
    let mut engine = picker_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    let name = copy_string(pointer, op_editor_copy_save_file_name).expect("name");
    assert_eq!(stage(pointer, &staging.file(&name)), OpStatus::Ok);
    assert_eq!(
        commit(pointer, "content://picked/2", "first.op"),
        OpStatus::Ok
    );

    queue(&mut engine, op_editor_core::FileAction::SaveAs);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(
        copy_string(pointer, op_editor_copy_save_target),
        None,
        "Save As must ask for a new destination"
    );

    assert_eq!(stage(pointer, &staging.file("first.op")), OpStatus::Ok);
    assert_eq!(
        commit(pointer, "content://picked/3", "second.op"),
        OpStatus::Ok
    );
    let binding = engine
        .session_mut_for_test()
        .document_save
        .shell_binding()
        .map(|b| (b.handle.clone(), b.display_name.clone()));
    assert_eq!(
        binding,
        Some(("content://picked/3".into(), "second.op".into())),
        "Save As rebinds to the new destination"
    );
}

/// A dismissed picker is not a save. Nothing may be marked clean, the old
/// binding must survive, and the next Save must be able to start over.
#[test]
fn a_cancelled_picker_leaves_the_document_dirty_and_retryable() {
    let mut engine = picker_engine();
    let pointer = &mut engine as *mut OpEngine;

    touch(&mut engine, "unsaved-work");
    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(
        unsafe { op_editor_cancel_save(pointer, false) },
        OpStatus::Ok
    );

    assert!(dirty(&mut engine), "a cancelled save keeps the changes");
    assert!(engine
        .session_mut_for_test()
        .document_save
        .pending
        .is_none());
    // A second cancel has nothing to cancel — the shell must not be able to
    // silently double-consume a round trip.
    assert_eq!(
        unsafe { op_editor_cancel_save(pointer, false) },
        OpStatus::NotReady
    );

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(
        drain(pointer),
        SHELL_ACTION_SAVE_DOCUMENT,
        "the next Save starts a fresh round trip"
    );
}

/// The shell got a destination but could not write it. Same outcome as a
/// cancel, plus a diagnostic — never a document that believes it is saved.
#[test]
fn a_failed_shell_write_keeps_the_previous_binding() {
    let staging = Staging::new();
    let mut engine = picker_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    let name = copy_string(pointer, op_editor_copy_save_file_name).expect("name");
    assert_eq!(stage(pointer, &staging.file(&name)), OpStatus::Ok);
    assert_eq!(commit(pointer, "content://good", "bound.op"), OpStatus::Ok);

    touch(&mut engine, "changes-that-must-not-vanish");
    queue(&mut engine, op_editor_core::FileAction::SaveAs);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(stage(pointer, &staging.file("bound.op")), OpStatus::Ok);
    // The staged bytes exist, but the copy into the picked file blew up.
    assert_eq!(
        unsafe { op_editor_cancel_save(pointer, true) },
        OpStatus::Ok
    );

    assert!(
        dirty(&mut engine),
        "a failed write keeps the document dirty"
    );
    assert_eq!(
        engine
            .session_mut_for_test()
            .document_save
            .shell_binding()
            .map(|b| b.handle.clone()),
        Some("content://good".into()),
        "a failed Save As must not clobber the working binding"
    );
}

/// Backgrounding a shell-bound dirty document: the bytes go to a private
/// shadow copy, the document stays honestly dirty, and the next drain asks
/// the shell to catch the picked destination up.
#[test]
fn suspend_shadows_a_shell_bound_document_and_reconciles_on_the_next_drain() {
    let staging = Staging::new();
    // The shadow copy lands in the engine's documents directory; point that
    // at a private temp dir so the case cannot collide with the migration
    // cases that drain the shared private fallback.
    let documents = std::env::temp_dir().join(format!(
        "openpencil-ffi-save-shadow-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&documents).expect("documents root");
    let mut engine = picker_engine_with_documents_root(Some(documents.clone()));
    let pointer = &mut engine as *mut OpEngine;

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    let name = copy_string(pointer, op_editor_copy_save_file_name).expect("name");
    assert_eq!(stage(pointer, &staging.file(&name)), OpStatus::Ok);
    assert_eq!(
        commit(pointer, "content://suspend/1", "backgrounded.op"),
        OpStatus::Ok
    );

    touch(&mut engine, "typed-just-before-backgrounding");
    crate::editor_document::flush_on_suspend(engine.session_mut_for_test());

    assert!(
        dirty(&mut engine),
        "the picked destination is stale, so the document must stay dirty"
    );
    let shadow = documents.join(".backgrounded.autosave.op");
    assert!(
        std::fs::read_to_string(&shadow)
            .expect("shadow copy")
            .contains("typed-just-before-backgrounding"),
        "backgrounding must not lose the delta"
    );

    // Resumed: the engine asks the shell to rewrite the same destination,
    // with no user interaction.
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(
        copy_string(pointer, op_editor_copy_save_target).as_deref(),
        Some("content://suspend/1")
    );
    assert_eq!(
        stage(pointer, &staging.file("backgrounded.op")),
        OpStatus::Ok
    );
    assert_eq!(
        commit(pointer, "content://suspend/1", "backgrounded.op"),
        OpStatus::Ok
    );
    assert!(!dirty(&mut engine));
    // One reconcile only: the flag is consumed even though it fired.
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    let _ = std::fs::remove_dir_all(&documents);
}

/// Guard rails on the staging handshake itself.
#[test]
fn staging_rejects_a_mismatched_name_an_existing_file_and_a_double_stage() {
    let staging = Staging::new();
    let mut engine = picker_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    let name = copy_string(pointer, op_editor_copy_save_file_name).expect("name");

    assert_eq!(
        stage(pointer, &staging.file("something-else.op")),
        OpStatus::InvalidArg
    );
    assert_eq!(
        stage(pointer, Path::new("relative.op")),
        OpStatus::InvalidArg
    );

    let occupied = staging.file(&name);
    std::fs::write(&occupied, b"existing").expect("occupy");
    assert_eq!(stage(pointer, &occupied), OpStatus::InvalidArg);
    assert_eq!(
        std::fs::read(&occupied).expect("untouched"),
        b"existing",
        "a refused stage must not overwrite"
    );
    std::fs::remove_file(&occupied).expect("clear");

    assert_eq!(stage(pointer, &occupied), OpStatus::Ok);
    assert_eq!(
        stage(pointer, &staging.file(&name)),
        OpStatus::Busy,
        "one stage per round trip"
    );

    // A commit needs a real destination handle and a real name.
    assert_eq!(commit(pointer, "  ", "ok.op"), OpStatus::InvalidArg);
    assert_eq!(
        commit(pointer, "content://x", "a/b.op"),
        OpStatus::InvalidArg
    );
    assert_eq!(commit(pointer, "content://x", "ok.op"), OpStatus::Ok);
}

/// Committing without staging would mark a document saved whose bytes never
/// left the engine.
#[test]
fn commit_without_staging_is_refused() {
    let mut engine = picker_engine();
    let pointer = &mut engine as *mut OpEngine;

    assert_eq!(commit(pointer, "content://x", "x.op"), OpStatus::NotReady);
    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(commit(pointer, "content://x", "x.op"), OpStatus::NotReady);
}

/// Replacing the document must not leave the incoming one bound to the
/// outgoing one's destination.
#[test]
fn replacing_the_document_drops_a_shell_binding() {
    let staging = Staging::new();
    let mut engine = picker_engine();
    let pointer = &mut engine as *mut OpEngine;

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    let name = copy_string(pointer, op_editor_copy_save_file_name).expect("name");
    assert_eq!(stage(pointer, &staging.file(&name)), OpStatus::Ok);
    assert_eq!(commit(pointer, "content://old", "old.op"), OpStatus::Ok);

    queue(&mut engine, op_editor_core::FileAction::New);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    assert!(engine
        .session_mut_for_test()
        .document_save
        .shell_binding()
        .is_none());

    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(
        copy_string(pointer, op_editor_copy_save_target),
        None,
        "a fresh document must be placed by the user, not inherited"
    );
    assert_eq!(
        unsafe { op_editor_cancel_save(pointer, false) },
        OpStatus::Ok
    );
}

/// A shell that never declares the picker keeps the engine-owned name
/// dialog — the ABI addition must be opt-in.
#[test]
fn a_shell_without_the_picker_capability_still_gets_the_name_dialog() {
    let mut engine = OpEngine::new(
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: 390.0,
            height: 844.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session"),
    );
    let pointer = &mut engine as *mut OpEngine;
    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_NONE);
    assert!(
        engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui
            .save_name_dialog
            .open,
        "without the capability the engine still names the file itself"
    );
}

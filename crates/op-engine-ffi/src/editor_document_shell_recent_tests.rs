//! Successful picker saves remain in Works across settings reloads.

use super::*;

fn recent_paths(engine: &mut OpEngine) -> Vec<PathBuf> {
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .editor_ui
        .recent_files
        .iter()
        .map(|entry| PathBuf::from(&entry.path))
        .collect()
}

#[test]
fn picker_save_indexes_exact_staged_bytes_and_resave_reuses_the_local_copy() {
    let staging = Staging::new();
    let saved = Staging::new();
    let mut engine = picker_engine();
    engine.session_mut_for_test().document_save.root = Some(saved.0.clone());
    let pointer = &mut engine as *mut OpEngine;
    touch(&mut engine, "saved revision");
    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    let name = copy_string(pointer, op_editor_copy_save_file_name).unwrap();
    let staged = staging.file(&name);
    assert_eq!(stage(pointer, &staged), OpStatus::Ok);
    assert!(
        recent_paths(&mut engine).is_empty(),
        "staging is not a successful save"
    );
    touch(&mut engine, "edit after staging");
    assert_eq!(
        commit(pointer, "content://docs/persisted", &name),
        OpStatus::Ok
    );
    let cached = recent_paths(&mut engine)[0].clone();
    assert_eq!(
        std::fs::read(&cached).unwrap(),
        std::fs::read(&staged).unwrap()
    );
    assert!(dirty(&mut engine));

    let next_staging = Staging::new();
    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(stage(pointer, &next_staging.file(&name)), OpStatus::Ok);
    assert_eq!(
        commit(pointer, "content://docs/persisted", &name),
        OpStatus::Ok
    );
    assert_eq!(
        recent_paths(&mut engine).as_slice(),
        std::slice::from_ref(&cached)
    );
    assert!(std::fs::read_to_string(&cached)
        .unwrap()
        .contains("edit after staging"));
    assert!(!dirty(&mut engine));

    // Unit engines keep automatic settings I/O inert to protect the user's
    // real preferences. Exercise the same save/load functions explicitly;
    // the ABI persistence integration test covers their per-call wiring.
    op_editor_host_core::settings_io::save(
        engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state(),
    );
    drop(engine);
    let mut reopened = picker_engine();
    reopened.session_mut_for_test().document_save.root = Some(saved.0.clone());
    op_editor_host_core::settings_io::load(
        reopened
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state_mut(),
    );
    assert_eq!(recent_paths(&mut reopened), [cached]);
    queue(&mut reopened, op_editor_core::FileAction::OpenRecent(0));
    assert_eq!(drain(&mut reopened), SHELL_ACTION_NONE);
    let state = reopened
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state();
    assert_eq!(state.doc.name.as_deref(), Some("edit after staging"));
    assert!(!state.is_dirty());

    touch(&mut reopened, "edit after reopening");
    queue(&mut reopened, op_editor_core::FileAction::Save);
    assert_eq!(drain(&mut reopened), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(
        copy_string(&mut reopened, op_editor_copy_save_target).as_deref(),
        Some("content://docs/persisted"),
        "reopening the private copy must preserve the acknowledged external destination",
    );
}

#[test]
fn legacy_picker_copy_prompts_instead_of_silently_saving_only_the_private_mirror() {
    let root = Staging::new();
    let mut engine = picker_engine();
    engine.session_mut_for_test().document_save.root = Some(root.0.clone());
    let path = root.0.join("Saved/legacy.op");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, SAMPLE_DOC).unwrap();
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state_mut()
        .editor_ui
        .touch_recent_file(path.to_string_lossy().into_owned(), 1);
    queue(&mut engine, op_editor_core::FileAction::OpenRecent(0));
    assert_eq!(drain(&mut engine), SHELL_ACTION_NONE);
    touch(&mut engine, "changed legacy copy");
    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(&mut engine), SHELL_ACTION_SAVE_DOCUMENT);
    assert_eq!(copy_string(&mut engine, op_editor_copy_save_target), None);
    assert_eq!(std::fs::read_to_string(path).unwrap(), SAMPLE_DOC);
}

#[test]
fn failed_local_mirror_does_not_undo_a_successful_external_save() {
    let staging = Staging::new();
    let mut engine = picker_engine();
    let blocked = staging.file("not-a-directory");
    std::fs::write(&blocked, b"block directory creation").unwrap();
    engine.session_mut_for_test().document_save.root = Some(blocked);
    let pointer = &mut engine as *mut OpEngine;
    touch(&mut engine, "external file is current");
    queue(&mut engine, op_editor_core::FileAction::Save);
    assert_eq!(drain(pointer), SHELL_ACTION_SAVE_DOCUMENT);
    let name = copy_string(pointer, op_editor_copy_save_file_name).unwrap();
    assert_eq!(stage(pointer, &staging.file(&name)), OpStatus::Ok);
    assert_eq!(
        commit(pointer, "content://docs/external", &name),
        OpStatus::Ok
    );
    assert!(!dirty(&mut engine));
    let binding = engine
        .session_mut_for_test()
        .document_save
        .shell_binding()
        .unwrap();
    assert_eq!(binding.handle, "content://docs/external");
    assert!(binding.recent_copy.is_none());
    assert!(recent_paths(&mut engine).is_empty());
}

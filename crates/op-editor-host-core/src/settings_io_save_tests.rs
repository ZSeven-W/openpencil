use super::super::*;

fn settings_save_test_root(case: &str) -> std::path::PathBuf {
    let sequence = SETTINGS_TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "openpencil-settings-save-{}-{sequence}-{case}",
        std::process::id()
    ))
}

#[test]
fn checked_save_reports_an_unwritable_parent() {
    let root = settings_save_test_root("unwritable-parent");
    let blocking_parent = root.join("not-a-directory");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(&blocking_parent, b"block").unwrap();
    let path = blocking_parent.join("settings.json");

    let result = save_checked_to_path(&EditorState::new(), &path);

    assert!(result.is_err());
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn checked_save_does_not_reuse_an_existing_temporary_path() {
    let root = settings_save_test_root("unique-temp");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("settings.json");
    let tmp = root.join("settings.json.tmp");
    std::fs::write(&tmp, b"do not replace").unwrap();

    let result = save_checked_to_path(&EditorState::new(), &path);

    assert!(result.is_ok());
    assert_eq!(std::fs::read(&tmp).unwrap(), b"do not replace");
    assert!(path.is_file());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn checked_save_removes_the_temporary_file_after_a_replace_failure() {
    let root = settings_save_test_root("replace-failure");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("settings.json");
    std::fs::create_dir(&path).unwrap();

    let result = save_checked_to_path(&EditorState::new(), &path);

    assert!(result.is_err());
    let remaining = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(remaining, vec![path]);
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn checked_save_enforces_private_permissions_on_the_final_file() {
    use std::os::unix::fs::PermissionsExt;

    let root = settings_save_test_root("private-mode");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("settings.json");
    let old_tmp = root.join("settings.json.tmp");
    std::fs::write(&old_tmp, b"old temporary contents").unwrap();
    std::fs::set_permissions(&old_tmp, std::fs::Permissions::from_mode(0o666)).unwrap();

    save_checked_to_path(&EditorState::new(), &path).unwrap();

    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let _ = std::fs::remove_dir_all(&root);
}

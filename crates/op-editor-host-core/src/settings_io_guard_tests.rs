//! The corrupt-file guard: an unreadable settings file is backed up and
//! never overwritten by this process.

use super::{load_lenient_from_path, save_checked_to_path, SettingsIoError};
use op_editor_core::EditorState;

fn temp_settings_path(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "op-settings-guard-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.join("settings.json")
}

#[test]
fn an_unparseable_settings_file_is_backed_up_and_never_overwritten() {
    let path = temp_settings_path("corrupt");
    let garbage = b"{ \"version\": 1, \"builtin_agents\": [ truncated";
    std::fs::write(&path, garbage).expect("write garbage");

    let mut state = EditorState::new();
    assert!(
        !load_lenient_from_path(&mut state, &path),
        "the file must be rejected"
    );

    let backups: Vec<_> = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("settings.json.corrupt-")
        })
        .collect();
    assert_eq!(backups.len(), 1, "exactly one corrupt backup is written");
    assert_eq!(std::fs::read(backups[0].path()).unwrap(), garbage);

    assert!(matches!(
        save_checked_to_path(&state, &path),
        Err(SettingsIoError::RejectedLoad)
    ));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        garbage,
        "the original bytes stay on disk for the user to recover"
    );
}

#[test]
fn a_missing_or_valid_settings_file_keeps_saves_working() {
    let path = temp_settings_path("clean");
    let mut state = EditorState::new();
    assert!(
        load_lenient_from_path(&mut state, &path),
        "a missing file is a first run"
    );
    save_checked_to_path(&state, &path).expect("first save writes the file");
    assert!(
        load_lenient_from_path(&mut state, &path),
        "the file we wrote loads back"
    );
    save_checked_to_path(&state, &path).expect("saving a valid file is still allowed");
}

#![cfg(feature = "editor")]

//! Mobile settings persistence: an editor engine created with a private
//! storage root must restore persisted user settings on the next create
//! (the iOS/Android/OHOS "provider settings survive an app restart" path).
//!
//! This lives in its own integration-test binary on purpose: the config
//! root is a process-global first-call-wins `OnceLock`, so configuring it
//! here must not leak into the other test binaries' engines (which create
//! without a storage root and keep persistence inert).

use op_engine_ffi::{op_create, op_destroy, op_editor_locale_code, op_editor_set_locale};
use op_engine_ffi::{OpCreateDesc, OpEngine, OpStatus};
use std::path::{Path, PathBuf};
use std::ptr;

fn temp_storage_root() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "op-engine-ffi-settings-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create temp storage root");
    root
}

fn create_editor_engine(storage_root: &Path) -> *mut OpEngine {
    let storage = storage_root.to_str().expect("utf-8 temp path").as_bytes();
    let desc = OpCreateDesc {
        size: std::mem::size_of::<OpCreateDesc>(),
        doc_ptr: ptr::null(),
        doc_len: 0,
        width: 390.0,
        height: 844.0,
        dpr: 2.0,
        callbacks: ptr::null(),
        asset_base_ptr: ptr::null(),
        asset_base_len: 0,
        mode: 1,
        storage_root_ptr: storage.as_ptr(),
        storage_root_len: storage.len(),
        documents_root_ptr: ptr::null(),
        documents_root_len: 0,
    };
    let mut engine: *mut OpEngine = ptr::null_mut();
    let status = unsafe { op_create(&desc, &mut engine) };
    assert_eq!(status, OpStatus::Ok);
    assert!(!engine.is_null());
    engine
}

fn locale_code_of(engine: *mut OpEngine) -> String {
    let mut required = 0usize;
    let status = unsafe { op_editor_locale_code(engine, ptr::null_mut(), 0, &mut required) };
    assert_eq!(status, OpStatus::Ok);
    let mut bytes = vec![0u8; required];
    let status =
        unsafe { op_editor_locale_code(engine, bytes.as_mut_ptr(), bytes.len(), &mut required) };
    assert_eq!(status, OpStatus::Ok);
    String::from_utf8(bytes).expect("locale code is utf-8")
}

#[test]
fn settings_survive_an_engine_restart_via_the_storage_root() {
    let root = temp_storage_root();

    // First launch: change a persisted setting through the public ABI.
    // Thai is not seeded by any CI environment locale, so a pass can only
    // come from the settings file round trip.
    let engine = create_editor_engine(&root);
    let tag = "th";
    let status = unsafe { op_editor_set_locale(engine, tag.as_ptr(), tag.len()) };
    assert_eq!(status, OpStatus::Ok);
    assert_eq!(locale_code_of(engine), "th");
    let status = unsafe { op_destroy(engine) };
    assert_eq!(status, OpStatus::Ok);

    // The edit was flushed into the shell-provided sandbox root, not a
    // desktop config directory.
    let settings_file = root.join("settings.json");
    let json = std::fs::read_to_string(&settings_file).expect("settings.json written");
    assert!(json.contains("\"locale\": \"th\""), "unexpected: {json}");

    // Second launch (same process; the same canonical root is idempotent):
    // the persisted choice must be live again without any shell replay.
    let engine = create_editor_engine(&root);
    assert_eq!(locale_code_of(engine), "th");
    let status = unsafe { op_destroy(engine) };
    assert_eq!(status, OpStatus::Ok);

    let _ = std::fs::remove_dir_all(&root);
}

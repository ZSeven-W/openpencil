//! Tests for app control helpers, split out to keep the implementation below 800 lines.

use super::{
    default_document_path_in, editor_will_open, live_port_file_path_in, preflight_document,
    MINIMAL_DOCUMENT,
};
use std::fs;

#[test]
fn preflight_rejects_malformed_documents_and_accepts_a_valid_one() {
    let dir = std::env::temp_dir().join(format!("op-preflight-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);

    // A real `.fig` / `.pen` is a binary ZIP archive — `PK\x03\x04` then
    // bytes that are not valid UTF-8. This must be rejected as a wrong file
    // type, not reported as an unreadable file, and never reach the server
    // (which would exit(1) before binding → opaque "connection refused").
    let archive = dir.join("archive.op");
    fs::write(&archive, b"PK\x03\x04\xff\xfe\x00\x01binary\x80\x81").expect("write archive");
    let err = preflight_document(&archive).expect_err("binary archive must be rejected");
    assert!(
        err.contains("is not a valid OpenPencil document"),
        "unexpected error text: {err}"
    );

    // Valid UTF-8 that isn't a canonical document — exercises the loader's
    // JSON parse-error path (distinct from the invalid-UTF-8 path above).
    let garbage = dir.join("garbage.op");
    fs::write(&garbage, b"this is not a .op document").expect("write garbage");
    let err = preflight_document(&garbage).expect_err("non-document text must be rejected");
    assert!(
        err.contains("is not a valid OpenPencil document"),
        "unexpected error text: {err}"
    );

    // The exact starter template `ensure_document_file` writes must load,
    // so a fresh session is never falsely rejected (loader parity).
    let good = dir.join("good.op");
    fs::write(&good, MINIMAL_DOCUMENT).expect("write good doc");
    assert!(
        preflight_document(&good).is_ok(),
        "the minimal starter document must pass preflight"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reports_only_files_the_editor_actually_opens() {
    // Mirrors the desktop `initial_file_from_argv` gate so `op start
    // --file` never claims a documentPath the editor silently ignored.
    let dir = std::env::temp_dir().join(format!("op-start-file-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let op = dir.join("doc.op");
    fs::write(&op, "{}").expect("write .op");
    let txt = dir.join("note.txt");
    fs::write(&txt, "x").expect("write .txt");

    // Existing supported document → editor opens it.
    assert!(editor_will_open(op.to_str().unwrap()));
    // Existing but unsupported extension → editor ignores it.
    assert!(!editor_will_open(txt.to_str().unwrap()));
    // Supported extension but missing file → editor ignores it.
    assert!(!editor_will_open(dir.join("missing.op").to_str().unwrap()));
    // A directory is not a file.
    assert!(!editor_will_open(dir.to_str().unwrap()));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn config_store_paths_keep_live_port_and_session_names() {
    let root = std::env::temp_dir().join(format!("op-cli-config-paths-{}", std::process::id()));
    let store = op_config_store::ConfigStore::at(&root);

    assert_eq!(
        live_port_file_path_in(&store).unwrap(),
        root.join(".op-mcp-port")
    );
    assert_eq!(
        default_document_path_in(&store).unwrap(),
        root.join("cli-session.op")
    );
}

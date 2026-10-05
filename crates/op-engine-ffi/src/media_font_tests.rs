//! Imported mobile font bytes and the editor's availability inventory must
//! agree both immediately and after opening a saved document.

use super::op_register_font;
use crate::desc::{Callbacks, CreateOptions};
use crate::lifecycle::{OpEngine, Session};
use crate::OpStatus;
use op_editor_core::missing_fonts::detect_missing_fonts;
use std::sync::Arc;

const DOCUMENT: &str = r#"{"version":"1.0.0","children":[{"type":"text","id":"title","content":"Saved work","fontFamily":"Roboto","fontSize":20}]}"#;
fn engine(document: &str) -> OpEngine {
    OpEngine::new(
        Session::new(CreateOptions {
            document: document.into(),
            width: 390.0,
            height: 844.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("mobile editor"),
    )
}

#[test]
fn registering_mobile_font_resolves_prompt_and_survives_document_reopen() {
    // Font registration changes the process-global Skia registry. Keep the
    // real ABI assertions in an isolated process so concurrently running
    // render regressions cannot change fonts between their pixel snapshots.
    const CHILD: &str = "OPENPENCIL_FFI_FONT_TEST_CHILD";
    const FULL_TEST: &str = concat!(
        module_path!(),
        "::registering_mobile_font_resolves_prompt_and_survives_document_reopen"
    );
    // libtest names omit the crate component included by module_path!().
    let test = FULL_TEST.split_once("::").expect("qualified test name").1;
    if std::env::var(CHILD).as_deref() != Ok(test) {
        let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", test, "--nocapture"])
            .env(CHILD, test)
            .output()
            .expect("run isolated font registration regression");
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("1 passed;"),
            "isolated font regression failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        return;
    }
    for (family, path) in [
        ("Roboto", "../op-host-native/assets/Roboto-Regular.ttf"),
        ("Inter", "../op-host-desktop/assets/fonts/Inter-VF.ttf"),
        (
            "Noto Sans SC",
            "../../packaging/shared/fonts/NotoSansSC-VF.ttf",
        ),
        (
            "Plus Jakarta Sans",
            "../../packaging/shared/fonts/PlusJakartaSans-VF.ttf",
        ),
    ] {
        let font = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
            .expect("shipped mobile font asset");
        assert_eq!(
            jian_skia::parse_imported_font_meta(&font).unwrap().family,
            family
        );
        register_and_reopen(family, &font);
    }
}

fn register_and_reopen(family: &str, font: &[u8]) {
    let document = DOCUMENT.replace("Roboto", family);
    let mut engine = engine(&document);
    {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        let state = host.editor_state_mut();
        // Isolate the editor snapshot from fonts installed on the test host.
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families = Arc::default();
        state.editor_ui.bundled_font_families = Arc::default();
        state.editor_ui.imported_font_families = Arc::default();
        state.editor_ui.missing_fonts_prompt = detect_missing_fonts(state);
        state.editor_ui.missing_fonts_modal_open = true;
        assert!(state.editor_ui.missing_fonts_prompt.is_some());
    }
    assert_eq!(
        unsafe { op_register_font(&mut engine, font.as_ptr(), font.len()) },
        OpStatus::Ok
    );
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    let ui = &host.editor_state().editor_ui;
    assert!(ui.imported_font_families.iter().any(|name| name == family));
    assert!(ui.missing_fonts_prompt.is_none());
    assert!(!ui.missing_fonts_modal_open);

    let document = jian_ops_schema::load_str(&document).unwrap().value;
    assert!(host
        .install_open_document(document, None, Some("Saved work.op".into()))
        .is_ok());
    assert!(host.editor_state().editor_ui.missing_fonts_prompt.is_none());
    assert!(!host.editor_state().editor_ui.missing_fonts_modal_open);
}

#[test]
fn invalid_mobile_font_does_not_claim_the_family_is_available() {
    let mut engine = engine(DOCUMENT);
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state_mut()
        .editor_ui
        .imported_font_families = Arc::default();
    let invalid = b"not a font";
    assert_eq!(
        unsafe { op_register_font(&mut engine, invalid.as_ptr(), invalid.len()) },
        OpStatus::InvalidArg
    );
    assert!(engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .editor_ui
        .imported_font_families
        .is_empty());
}

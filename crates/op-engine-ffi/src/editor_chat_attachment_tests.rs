//! The mobile attachment-picker contract, end to end through the C ABI:
//! the Home 添加截图 tap surfaces as a shell action exactly once, and the
//! bytes the shell hands back land on the same staged-attachment list the
//! desktop picker fills.

use super::*;
use crate::desc::{Callbacks, CreateOptions};
use crate::editor_auth::SHELL_ACTION_NONE;
use crate::lifecycle::OpEngine;
use crate::{op_editor_press_at, op_editor_release_at, op_editor_take_shell_action};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::EntrySurface;
use op_editor_ui::widgets::HomeSurface;

const PHONE: (f32, f32) = (402.0, 874.0);
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nmobile-screenshot-payload";
const JPEG: &[u8] = b"\xFF\xD8\xFF\xE0mobile-photo";

/// Compact-phone session with the Studio Home takeover up.
fn home_engine() -> OpEngine {
    let mut engine = OpEngine::new(
        Session::new(CreateOptions {
            document: String::new(),
            width: PHONE.0,
            height: PHONE.1,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session"),
    );
    let host = engine.session_mut_for_test().editor_mut().expect("host");
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = true;
    ui.size_class = EditorSizeClass::Compact;
    ui.entry_surface = EntrySurface::Home;
    ui.home.visible = true;
    engine
}

fn take_action(engine: &mut OpEngine) -> i32 {
    let mut action = -1;
    assert_eq!(
        unsafe { op_editor_take_shell_action(engine, &mut action) },
        OpStatus::Ok
    );
    action
}

fn attach(engine: &mut OpEngine, bytes: &[u8], media_type: &str, name: &str) -> OpStatus {
    unsafe {
        op_editor_attach_chat_image(
            engine,
            bytes.as_ptr(),
            bytes.len(),
            media_type.as_ptr(),
            media_type.len(),
            name.as_ptr(),
            name.len(),
        )
    }
}

fn staged(engine: &mut OpEngine) -> Vec<(String, String, usize)> {
    engine
        .session_mut_for_test()
        .editor()
        .expect("host")
        .editor_state()
        .chat
        .pending_attachments
        .iter()
        .map(|a| (a.name.clone(), a.media_type.clone(), a.data.len()))
        .collect()
}

#[test]
fn the_home_screenshot_tap_asks_the_shell_for_a_picker_exactly_once() {
    let mut engine = home_engine();
    let (x, y) = {
        let session = engine.session_mut_for_test();
        let (w, h) = session.editor_viewport();
        let host = session.editor().expect("host");
        let home = HomeSurface::for_editor_at(host.editor_state(), session.now_ms).expect("home");
        let rect = home.layout(w, h).screenshot;
        (
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        )
    };
    let pointer = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { op_editor_press_at(pointer, x, y, 1_000) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release_at(pointer, x, y, 1_050) },
        OpStatus::Ok
    );

    assert_eq!(take_action(&mut engine), SHELL_ACTION_PICK_CHAT_ATTACHMENT);
    // Consumed: a cancelled picker never reopens on the next poll.
    assert_eq!(take_action(&mut engine), SHELL_ACTION_NONE);

    // The shell's pick lands on the list Home and the next send read.
    assert_eq!(
        attach(&mut engine, PNG, "image/png", "IMG_0042.PNG"),
        OpStatus::Ok
    );
    assert_eq!(
        staged(&mut engine),
        vec![(
            "IMG_0042.PNG".to_string(),
            "image/png".to_string(),
            PNG.len()
        )]
    );
    let host = engine.session_mut_for_test().editor().expect("host");
    let home = HomeSurface::for_editor_at(host.editor_state(), 2_000).expect("home");
    assert_eq!(home.attachment_names, vec!["IMG_0042.PNG".to_string()]);
}

#[test]
fn the_media_type_is_sniffed_when_the_shell_does_not_declare_one() {
    let mut engine = home_engine();
    assert_eq!(attach(&mut engine, JPEG, "", ""), OpStatus::Ok);
    assert_eq!(
        staged(&mut engine),
        vec![(
            "attachment.jpg".to_string(),
            "image/jpeg".to_string(),
            JPEG.len()
        )]
    );
    // `image/jpg` is the same type, spelled the way some pickers spell it.
    assert_eq!(
        attach(&mut engine, JPEG, "image/jpg", "a.jpg"),
        OpStatus::Ok
    );
    assert_eq!(staged(&mut engine)[1].1, "image/jpeg");
}

#[test]
fn a_declared_svg_is_accepted_and_contradictions_are_refused() {
    let mut engine = home_engine();
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
    assert_eq!(
        attach(&mut engine, svg, "image/svg+xml", "logo.svg"),
        OpStatus::Ok
    );
    assert_eq!(staged(&mut engine)[0].1, "image/svg+xml");

    assert_eq!(
        attach(&mut engine, PNG, "image/jpeg", "x.jpg"),
        OpStatus::InvalidArg
    );
    assert_eq!(
        attach(&mut engine, PNG, "application/pdf", "x.pdf"),
        OpStatus::InvalidArg
    );
    assert_eq!(
        attach(&mut engine, b"plain text", "", "notes.txt"),
        OpStatus::InvalidArg
    );
    assert_eq!(
        attach(&mut engine, PNG, "", "../x.png"),
        OpStatus::InvalidArg
    );
    assert_eq!(attach(&mut engine, &[], "", ""), OpStatus::InvalidArg);
    assert_eq!(staged(&mut engine).len(), 1, "refusals stage nothing");
}

#[test]
fn oversized_images_and_a_full_turn_stage_nothing() {
    let mut engine = home_engine();
    let mut big = PNG.to_vec();
    big.resize(MAX_ATTACHMENT_BYTES + 1, 0);
    assert_eq!(attach(&mut engine, &big, "", ""), OpStatus::InvalidArg);

    for _ in 0..MAX_ATTACHMENTS {
        assert_eq!(attach(&mut engine, PNG, "", ""), OpStatus::Ok);
    }
    assert_eq!(attach(&mut engine, PNG, "", ""), OpStatus::Busy);
    assert_eq!(staged(&mut engine).len(), MAX_ATTACHMENTS);
}

#[test]
fn a_null_pointer_with_a_length_is_rejected() {
    let mut engine = home_engine();
    let status = unsafe {
        op_editor_attach_chat_image(
            &mut engine,
            std::ptr::null(),
            8,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
        )
    };
    assert_eq!(status, OpStatus::InvalidArg);
}

#![cfg(feature = "editor")]

//! Editor-mode tests: `OpCreateDesc.mode == 1` drives the full desktop
//! chrome through op-host-native's widget host.

use op_engine_ffi::{
    op_create, op_destroy, op_editor_begin_transform, op_editor_cancel_gesture,
    op_editor_ime_focused, op_editor_key, op_editor_move, op_editor_open_document, op_editor_pan,
    op_editor_pinch, op_editor_press, op_editor_release, op_editor_right_press, op_editor_text,
    op_frame_cpu, op_get_page_count, op_get_pixel_size, op_last_error, op_set_keyboard,
    OpCreateDesc, OpEngine, OpStatus, KEY_ARROW_DOWN, KEY_BACKSPACE, KEY_DELETE, KEY_DUPLICATE,
    KEY_ENTER, KEY_ESCAPE, KEY_UNDO,
};
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");
const TWO_PAGE_DOC: &str =
    include_str!("../../../packaging/android-player/app/src/main/assets/two-page.op");

fn with_second_page_active(source: &str) -> String {
    let end = source.rfind('}').expect("top-level object close");
    format!(
        "{},\"editorMeta\":{{\"activePageIndex\":1}}{}",
        &source[..end],
        &source[end..]
    )
}

fn last_error_of(engine: *mut OpEngine) -> String {
    let mut required = 0usize;
    let status = unsafe { op_last_error(engine, ptr::null_mut(), 0, &mut required) };
    assert_eq!(status, OpStatus::Ok);
    if required == 0 {
        return String::new();
    }
    let mut bytes = vec![0u8; required];
    let status = unsafe { op_last_error(engine, bytes.as_mut_ptr(), bytes.len(), &mut required) };
    assert_eq!(status, OpStatus::Ok);
    String::from_utf8(bytes).unwrap()
}

unsafe extern "C" fn needs_redraw_cb(
    user_data: *mut c_void,
    _has_next_wake: bool,
    _next_wake_ms: u64,
) {
    // SAFETY: `user_data` is the live boxed CbCtx (freed in Drop, after
    // the engine is destroyed).
    unsafe {
        (&*(user_data as *const CbCtx))
            .redraws
            .fetch_add(1, Ordering::SeqCst);
    }
}

/// Callback-recording context (the C `user_data`); boxed so its address
/// is stable for the engine's lifetime.
struct CbCtx {
    redraws: AtomicUsize,
}

impl Default for CbCtx {
    fn default() -> Self {
        Self {
            redraws: AtomicUsize::new(0),
        }
    }
}

struct EditorHarness {
    engine: *mut OpEngine,
    ctx: *mut CbCtx,
}

impl EditorHarness {
    fn create(doc: &str) -> Self {
        let ctx = Box::into_raw(Box::new(CbCtx::default()));
        let callbacks = OpCallbacks {
            size: std::mem::size_of::<OpCallbacks>(),
            user_data: ctx as *mut c_void,
            needs_redraw: Some(needs_redraw_cb),
            runtime_error: None,
            input_focus_changed: None,
            remote_image_request: None,
            credential_load: None,
            credential_store_if_absent: None,
        };
        let mut callbacks_slot = callbacks;
        let doc_bytes = doc.as_bytes();
        let desc = OpCreateDesc {
            size: std::mem::size_of::<OpCreateDesc>(),
            doc_ptr: doc_bytes.as_ptr(),
            doc_len: doc_bytes.len(),
            width: 800.0,
            height: 600.0,
            dpr: 1.0,
            callbacks: &mut callbacks_slot,
            asset_base_ptr: ptr::null(),
            asset_base_len: 0,
            mode: 1,
            storage_root_ptr: ptr::null(),
            storage_root_len: 0,
        };
        let mut engine: *mut OpEngine = ptr::null_mut();
        let status = unsafe { op_create(&desc, &mut engine) };
        assert_eq!(
            status,
            OpStatus::Ok,
            "editor create failed: {}",
            last_error_of(ptr::null_mut())
        );
        Self { engine, ctx }
    }

    fn frame(&self) -> Vec<u8> {
        let mut width = 0u32;
        let mut height = 0u32;
        assert_eq!(
            unsafe { op_get_pixel_size(self.engine, &mut width, &mut height) },
            OpStatus::Ok
        );
        let stride = width as usize * 4;
        let mut buffer = vec![0u8; height as usize * stride];
        let status = unsafe {
            op_frame_cpu(
                self.engine,
                1_000,
                buffer.as_mut_ptr(),
                buffer.len(),
                stride,
            )
        };
        assert_eq!(
            status,
            OpStatus::Ok,
            "frame failed: {}",
            last_error_of(self.engine)
        );
        buffer
    }

    fn redraws(&self) -> usize {
        // SAFETY: `ctx` is the live boxed CbCtx.
        unsafe { (&*self.ctx).redraws.load(Ordering::SeqCst) }
    }
}

impl Drop for EditorHarness {
    fn drop(&mut self) {
        unsafe { op_destroy(self.engine) };
        // SAFETY: `ctx` is the live box created in `create`; freed once.
        unsafe { drop(Box::from_raw(self.ctx)) };
    }
}

use op_engine_ffi::OpCallbacks;

#[test]
fn editor_mode_renders_the_desktop_chrome() {
    let h = EditorHarness::create(SAMPLE_DOC);
    let buffer = h.frame();
    // The editor chrome is a dark theme — the top bar region must not be
    // the viewer's white backdrop.
    let top_bar_pixel = pixel_at(&buffer, 400, 20, 800);
    assert!(
        top_bar_pixel[0] < 200,
        "top bar should be dark, got {top_bar_pixel:?}"
    );
    // The frame is not a flat color — panels + canvas + doc content.
    let first = buffer[0];
    assert!(
        buffer.iter().any(|&b| b != first),
        "editor frame is a single flat color"
    );
}

#[test]
fn editor_keyboard_occlusion_keeps_the_unfocused_frame_stable() {
    let h = EditorHarness::create(SAMPLE_DOC);
    let _ = h.frame();
    let before = h.frame();
    assert_eq!(unsafe { op_set_keyboard(h.engine, 120.0) }, OpStatus::Ok);

    let after = h.frame();
    assert_eq!(
        after, before,
        "an unfocused keyboard update must not resize or translate the editor frame"
    );
}

#[test]
fn editor_open_document_replaces_pages_and_renders_the_new_active_page() {
    let h = EditorHarness::create(SAMPLE_DOC);
    let before = h.frame();
    let document = with_second_page_active(TWO_PAGE_DOC);
    let file_name = b"two-page.op";

    assert_eq!(
        unsafe {
            op_editor_open_document(
                h.engine,
                document.as_ptr(),
                document.len(),
                file_name.as_ptr(),
                file_name.len(),
            )
        },
        OpStatus::Ok,
        "open failed: {}",
        last_error_of(h.engine)
    );
    let mut page_count = 0;
    assert_eq!(
        unsafe { op_get_page_count(h.engine, &mut page_count) },
        OpStatus::Ok
    );
    assert_eq!(page_count, 2);

    let after = h.frame();
    assert_ne!(
        before, after,
        "the newly active page must replace the frame"
    );
    assert!(
        pixel_at(&after, 400, 20, 800)[0] < 200,
        "editor chrome must remain present after opening a document"
    );
}

#[test]
fn editor_open_document_rejection_keeps_the_previous_render_and_page_count() {
    let h = EditorHarness::create(TWO_PAGE_DOC);
    let before = h.frame();
    let bad = br#"{"version":"1.0.0","children":[{"type":"nonsense"}]}"#;

    assert_eq!(
        unsafe { op_editor_open_document(h.engine, bad.as_ptr(), bad.len(), ptr::null(), 0) },
        OpStatus::BadDocument
    );
    let mut page_count = 0;
    assert_eq!(
        unsafe { op_get_page_count(h.engine, &mut page_count) },
        OpStatus::Ok
    );
    assert_eq!(page_count, 2);
    assert_eq!(before, h.frame(), "a bad file must not replace live pixels");
}

#[test]
fn editor_press_release_and_keys_are_safe_and_drive_redraws() {
    let h = EditorHarness::create(SAMPLE_DOC);
    h.frame();
    let before = h.redraws();

    // Press + release on the canvas area.
    assert_eq!(
        unsafe { op_editor_press(h.engine, 400.0, 300.0) },
        OpStatus::Ok
    );
    assert_eq!(unsafe { op_editor_cancel_gesture(h.engine) }, OpStatus::Ok);
    assert_eq!(
        unsafe { op_editor_release(h.engine, 400.0, 300.0) },
        OpStatus::Ok
    );
    assert!(h.redraws() > before, "press/release must request redraws");

    // Escape closes anything transient; safe either way.
    let before = h.redraws();
    assert_eq!(unsafe { op_editor_key(h.engine, KEY_ESCAPE) }, OpStatus::Ok);
    assert_eq!(
        unsafe { op_editor_key(h.engine, KEY_BACKSPACE) },
        OpStatus::Ok
    );
    assert_eq!(unsafe { op_editor_key(h.engine, KEY_DELETE) }, OpStatus::Ok);
    assert_eq!(unsafe { op_editor_key(h.engine, KEY_ENTER) }, OpStatus::Ok);
    assert_eq!(
        unsafe { op_editor_key(h.engine, KEY_DUPLICATE) },
        OpStatus::Ok
    );
    assert_eq!(unsafe { op_editor_key(h.engine, KEY_UNDO) }, OpStatus::Ok);
    assert_eq!(
        unsafe { op_editor_key(h.engine, KEY_ARROW_DOWN) },
        OpStatus::Ok
    );
    h.frame();
    assert!(h.redraws() >= before);

    // Right-press (long-press) opens/refreshes context menus — safe.
    assert_eq!(
        unsafe { op_editor_right_press(h.engine, 400.0, 300.0) },
        OpStatus::Ok
    );

    // Text into an unfocused state is a safe no-op.
    let status = unsafe {
        let text = b"hello";
        op_editor_text(h.engine, text.as_ptr(), text.len())
    };
    assert_eq!(status, OpStatus::Ok);

    // Two-finger pan + pinch are safe.
    assert_eq!(
        unsafe { op_editor_begin_transform(h.engine, 400.0, 300.0) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_pan(h.engine, 400.0, 300.0, 5.0, 8.0) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_pinch(h.engine, 400.0, 300.0, 2.0) },
        OpStatus::Ok
    );
    h.frame();

    // Cursor moves are safe.
    assert_eq!(
        unsafe { op_editor_move(h.engine, 410.0, 310.0) },
        OpStatus::Ok
    );

    // No IME focus before any input interaction.
    let mut focused = true;
    assert_eq!(
        unsafe { op_editor_ime_focused(h.engine, &mut focused) },
        OpStatus::Ok
    );
    assert!(!focused);
}

#[test]
fn unknown_editor_key_is_rejected() {
    let h = EditorHarness::create(SAMPLE_DOC);
    let status = unsafe { op_editor_key(h.engine, 999) };
    assert_eq!(status, OpStatus::InvalidArg);
}

fn pixel_at(buffer: &[u8], x: usize, y: usize, width: usize) -> [u8; 4] {
    let i = y * width * 4 + x * 4;
    [buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]]
}

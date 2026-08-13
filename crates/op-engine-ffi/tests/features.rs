//! Tests for the player feature ABI additions: text editing + IME,
//! remote images, page switching, and imported fonts. Everything runs
//! through the public C surface on the raster CPU path.

use op_engine_ffi::{
    op_create, op_destroy, op_frame_cpu, op_get_page_count, op_get_pixel_size,
    op_ime_cancel_composition, op_ime_commit_composition, op_ime_set_composing_text, op_last_error,
    op_pointer, op_register_font, op_remote_image_result, op_set_active_page, op_set_safe_area,
    op_text_backspace, op_text_begin, op_text_caret_rect, op_text_end, op_text_get_state,
    op_text_insert, OpCallbacks, OpCreateDesc, OpEngine, OpStatus, OpTextState,
};
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// A canonical v1.0.0 document with ONE text node at a known absolute
/// position (layout-none frame at the origin, text at 20,20).
const TEXT_DOC: &str = r##"{
  "version": "1.0.0",
  "name": "text",
  "children": [
    {
      "type": "frame",
      "id": "f1",
      "name": "card",
      "x": 0,
      "y": 0,
      "width": 400,
      "height": 300,
      "layout": "none",
      "fill": [{ "type": "solid", "color": "#FFFFFF" }],
      "children": [
        {
          "type": "text",
          "id": "t1",
          "name": "title",
          "content": "Hello World",
          "fontSize": 28,
          "fontWeight": 400,
          "x": 20,
          "y": 20,
          "textGrowth": "auto",
          "width": "fit_content"
        }
      ]
    }
  ]
}"##;

/// A tiny canonical document with TWO pages (distinct background colors).
const TWO_PAGE_DOC: &str = r##"{
  "version": "1.0.0",
  "name": "two-page",
  "children": [],
  "pages": [
    {
      "id": "p1",
      "name": "Page 1",
      "children": [
        {
          "type": "frame",
          "id": "pg1-bg",
          "name": "bg",
          "width": 400,
          "height": 600,
          "layout": "none",
          "fill": [{ "type": "solid", "color": "#FF0000" }]
        }
      ]
    },
    {
      "id": "p2",
      "name": "Page 2",
      "children": [
        {
          "type": "frame",
          "id": "pg2-bg",
          "name": "bg",
          "width": 400,
          "height": 600,
          "layout": "none",
          "fill": [{ "type": "solid", "color": "#00FF00" }]
        }
      ]
    }
  ]
}"##;

/// A document with a remote (https) image node.
const REMOTE_IMAGE_DOC: &str = r##"{
  "version": "1.0.0",
  "name": "remote",
  "children": [
    {
      "type": "frame",
      "id": "f1",
      "name": "holder",
      "width": 320,
      "height": 240,
      "layout": "none",
      "children": [
        {
          "type": "image",
          "id": "img1",
          "name": "remote image",
          "src": "https://example.invalid/photo.png",
          "width": 320,
          "height": 240,
          "objectFit": "fill"
        }
      ]
    }
  ]
}"##;

/// A 1×1 red PNG (for `op_remote_image_result`).
const RED_PIXEL_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x01, 0x25, 0x84, 0x63, 0xf2, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// The demo imported font (copied from the jian player's assets).
const DEMO_FONT: &[u8] = include_bytes!("fixtures/player-font.ttf");

/// Callback-recording context (the C `user_data`).
struct CbCtx {
    redraws: AtomicUsize,
    focus_events: AtomicUsize,
    last_focused: AtomicUsize,
    image_requests: AtomicUsize,
    last_image_request_id: AtomicU64,
}

impl Default for CbCtx {
    fn default() -> Self {
        Self {
            redraws: AtomicUsize::new(0),
            focus_events: AtomicUsize::new(0),
            last_focused: AtomicUsize::new(0),
            image_requests: AtomicUsize::new(0),
            last_image_request_id: AtomicU64::new(0),
        }
    }
}

/// Engine + callback harness.
struct Harness {
    engine: *mut OpEngine,
    ctx: *mut CbCtx,
}

impl Harness {
    fn create(doc: &str) -> Self {
        let ctx = Box::into_raw(Box::new(CbCtx::default()));
        let callbacks = OpCallbacks {
            size: std::mem::size_of::<OpCallbacks>(),
            user_data: ctx as *mut c_void,
            needs_redraw: Some(needs_redraw_cb),
            runtime_error: None,
            input_focus_changed: Some(input_focus_changed_cb),
            remote_image_request: Some(remote_image_request_cb),
        };
        let mut callbacks_slot = callbacks;
        let doc_bytes = doc.as_bytes();
        let desc = OpCreateDesc {
            size: std::mem::size_of::<OpCreateDesc>(),
            doc_ptr: doc_bytes.as_ptr(),
            doc_len: doc_bytes.len(),
            width: 400.0,
            height: 600.0,
            dpr: 1.0,
            callbacks: &mut callbacks_slot,
            asset_base_ptr: ptr::null(),
            asset_base_len: 0,
            mode: 0,
        };
        let mut engine: *mut OpEngine = ptr::null_mut();
        let status = unsafe { op_create(&desc, &mut engine) };
        assert_eq!(
            status,
            OpStatus::Ok,
            "create failed: {}",
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
                self.now_ms(),
                buffer.as_mut_ptr(),
                buffer.len(),
                stride,
            )
        };
        assert_eq!(status, OpStatus::Ok, "frame failed: {}", self.last_error());
        buffer
    }

    fn now_ms(&self) -> u64 {
        1_000
    }

    fn last_error(&self) -> String {
        last_error_of(self.engine)
    }

    fn focus_events(&self) -> usize {
        // SAFETY: `ctx` is the live boxed CbCtx.
        unsafe { (&*self.ctx).focus_events.load(Ordering::SeqCst) }
    }

    fn last_focused(&self) -> bool {
        // SAFETY: `ctx` is the live boxed CbCtx.
        unsafe { (&*self.ctx).last_focused.load(Ordering::SeqCst) != 0 }
    }

    fn image_requests(&self) -> usize {
        // SAFETY: `ctx` is the live boxed CbCtx.
        unsafe { (&*self.ctx).image_requests.load(Ordering::SeqCst) }
    }

    fn last_image_request_id(&self) -> u64 {
        // SAFETY: `ctx` is the live boxed CbCtx.
        unsafe { (&*self.ctx).last_image_request_id.load(Ordering::SeqCst) }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        unsafe { op_destroy(self.engine) };
        // SAFETY: `ctx` is the live box created in `create`; freed once.
        unsafe { drop(Box::from_raw(self.ctx)) };
    }
}

unsafe extern "C" fn needs_redraw_cb(
    user_data: *mut c_void,
    _has_next_wake: bool,
    _next_wake_ms: u64,
) {
    // SAFETY: `user_data` is the live boxed CbCtx.
    unsafe {
        (&*(user_data as *const CbCtx))
            .redraws
            .fetch_add(1, Ordering::SeqCst)
    };
}

unsafe extern "C" fn input_focus_changed_cb(
    user_data: *mut c_void,
    focused: bool,
    _kind: i32,
    _hint: i32,
) {
    // SAFETY: `user_data` is the live boxed CbCtx.
    unsafe {
        let ctx = &*(user_data as *const CbCtx);
        ctx.focus_events.fetch_add(1, Ordering::SeqCst);
        ctx.last_focused.store(focused as usize, Ordering::SeqCst);
    }
}

unsafe extern "C" fn remote_image_request_cb(
    user_data: *mut c_void,
    request_id: u64,
    _url_ptr: *const u8,
    _url_len: usize,
) {
    // SAFETY: `user_data` is the live boxed CbCtx.
    unsafe {
        let ctx = &*(user_data as *const CbCtx);
        ctx.image_requests.fetch_add(1, Ordering::SeqCst);
        ctx.last_image_request_id
            .store(request_id, Ordering::SeqCst);
    }
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

fn tap(h: &Harness, id: u32, x: f32, y: f32) {
    let now = h.now_ms();
    assert_eq!(
        unsafe { op_pointer(h.engine, id, 0, x, y, now) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(h.engine, id, 2, x, y, now + 1) },
        OpStatus::Ok
    );
}

/// The fitted TEXT_DOC maps doc (30, 32) — inside the "Hello World" text —
/// to viewport (44, 191) at the 400×600 creation size.
const TEXT_TAP: (f32, f32) = (44.0, 191.0);

// ---- Text editing + IME ------------------------------------------------

#[test]
fn tapping_a_text_node_enters_edit_mode_and_fires_focus() {
    let h = Harness::create(TEXT_DOC);
    let before = h.focus_events();
    tap(&h, 1, TEXT_TAP.0, TEXT_TAP.1);
    assert_eq!(h.focus_events(), before + 1);
    assert!(h.last_focused());
    h.frame();
    unsafe { op_text_end(h.engine) };
    assert!(!h.last_focused());
}

#[test]
fn viewer_single_pointer_cancel_never_becomes_a_tap() {
    let h = Harness::create(TEXT_DOC);
    let before = h.frame();

    assert_eq!(
        unsafe { op_pointer(h.engine, 1, 0, TEXT_TAP.0, TEXT_TAP.1, 1_000) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(h.engine, 1, 3, TEXT_TAP.0, TEXT_TAP.1, 1_001) },
        OpStatus::Ok
    );
    assert!(!h.last_focused(), "Cancel must not enter text edit mode");
    assert_eq!(
        h.frame(),
        before,
        "Cancel must not mutate the camera or selection"
    );

    // UIKit/Android should not send Up after Cancel, but a stale delivery must
    // remain harmless rather than replaying the canceled touch as a tap.
    assert_eq!(
        unsafe { op_pointer(h.engine, 1, 2, TEXT_TAP.0, TEXT_TAP.1, 1_002) },
        OpStatus::Ok
    );
    assert!(!h.last_focused(), "stale Up after Cancel must not tap");

    tap(&h, 2, TEXT_TAP.0, TEXT_TAP.1);
    assert!(h.last_focused(), "a fresh pointer stream must still work");
}

#[test]
fn viewer_multi_pointer_cancel_resets_every_tracked_touch() {
    let h = Harness::create(TEXT_DOC);
    let insets = (47.0, 11.0, 31.0, 23.0);
    assert_eq!(
        unsafe { op_set_safe_area(h.engine, insets.0, insets.1, insets.2, insets.3) },
        OpStatus::Ok
    );
    let before = h.frame();
    let local_zoom = (366.0_f32 / 400.0).min(522.0 / 300.0) * 0.92;
    let text_x = insets.3 + (366.0 - 400.0 * local_zoom) * 0.5 + 30.0 * local_zoom;
    let text_y = insets.0 + (522.0 - 300.0 * local_zoom) * 0.5 + 32.0 * local_zoom;

    assert_eq!(
        unsafe { op_pointer(h.engine, 1, 0, text_x, text_y, 1_000) },
        OpStatus::Ok
    );
    // Pointer 2 starts in the left safe band and is suppressed, but Android
    // may report it as ACTION_CANCEL's action index. Cancel must still reset
    // accepted pointer 1 inside the viewer gesture tracker.
    assert_eq!(
        unsafe { op_pointer(h.engine, 2, 0, 10.0, 300.0, 1_001) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(h.engine, 2, 3, 10.0, 300.0, 1_002) },
        OpStatus::Ok
    );
    assert_eq!(
        h.frame(),
        before,
        "multi-touch Cancel must not move the camera"
    );

    // Cancel clears both ids, so neither stale Up may complete a tap.
    assert_eq!(
        unsafe { op_pointer(h.engine, 1, 2, text_x, text_y, 1_003) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(h.engine, 2, 2, 300.0, 400.0, 1_004) },
        OpStatus::Ok
    );
    assert!(!h.last_focused(), "stale multi-touch Up must not tap");

    tap(&h, 3, text_x, text_y);
    assert!(
        h.last_focused(),
        "Cancel must fully re-arm the next gesture"
    );
}

#[test]
fn viewer_safe_area_refits_paint_and_pointer_to_one_local_viewport() {
    let h = Harness::create(TEXT_DOC);
    let insets = (47.0, 11.0, 31.0, 23.0);
    assert_eq!(
        unsafe { op_set_safe_area(h.engine, insets.0, insets.1, insets.2, insets.3) },
        OpStatus::Ok
    );

    // TEXT_DOC is 400×300. Fitting the 366×522 safe-local viewport yields
    // 0.8418 zoom and centers it at local (14.6, 134.7). Tap inside t1 after
    // adding the full-surface safe-area origin.
    let local_zoom = (366.0_f32 / 400.0).min(522.0 / 300.0) * 0.92;
    let local_origin_x = (366.0 - 400.0 * local_zoom) * 0.5;
    let local_origin_y = (522.0 - 300.0 * local_zoom) * 0.5;
    let surface_x = insets.3 + local_origin_x + 30.0 * local_zoom;
    let surface_y = insets.0 + local_origin_y + 32.0 * local_zoom;
    tap(&h, 1, surface_x, surface_y);
    assert!(
        h.last_focused(),
        "viewer input must subtract left/top insets before hit-testing the refitted scene"
    );

    let mut caret = [0.0_f32; 4];
    assert_eq!(
        unsafe { op_text_caret_rect(h.engine, caret.as_mut_ptr()) },
        OpStatus::Ok
    );
    assert!(
        caret[0] >= insets.3 && caret[1] >= insets.0,
        "public caret geometry must add the safe-area origin back: {caret:?}"
    );

    assert_eq!(unsafe { op_text_end(h.engine) }, OpStatus::Ok);
    assert_eq!(
        unsafe { op_pointer(h.engine, 2, 0, 200.0, 350.0, 2_000) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(h.engine, 2, 1, 200.0, 100.0, 2_001) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(h.engine, 2, 2, 200.0, 100.0, 2_002) },
        OpStatus::Ok
    );
    let frame = h.frame();
    assert_eq!(
        pixel_at(&frame, 200, 20, 400),
        [245, 245, 247, 255],
        "panned viewer content must stay clipped out of the top safe band"
    );
}

#[test]
fn viewer_ignores_streams_starting_in_all_safe_area_bands() {
    let h = Harness::create(TEXT_DOC);
    assert_eq!(
        unsafe { op_set_safe_area(h.engine, 47.0, 11.0, 31.0, 23.0) },
        OpStatus::Ok
    );
    let before = h.frame();
    let focus_events = h.focus_events();
    let band_starts = [(11.0, 300.0), (200.0, 23.0), (395.0, 300.0), (200.0, 584.0)];

    for (index, (band_x, band_y)) in band_starts.into_iter().enumerate() {
        let id = index as u32 + 10;
        assert_eq!(
            unsafe { op_pointer(h.engine, id, 0, band_x, band_y, id as u64 * 10) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { op_pointer(h.engine, id, 1, 200.0, 300.0, id as u64 * 10 + 1) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { op_pointer(h.engine, id, 2, 200.0, 300.0, id as u64 * 10 + 2) },
            OpStatus::Ok
        );
        assert_eq!(
            h.frame(),
            before,
            "a pointer starting in safe-area band {index} must not pan or select"
        );
        assert_eq!(
            h.focus_events(),
            focus_events,
            "a pointer starting in safe-area band {index} must not change focus"
        );
    }
}

#[test]
fn viewer_keeps_capture_after_moving_into_safe_area_band() {
    let h = Harness::create(TEXT_DOC);
    assert_eq!(
        unsafe { op_set_safe_area(h.engine, 47.0, 11.0, 31.0, 23.0) },
        OpStatus::Ok
    );
    let before = h.frame();

    assert_eq!(
        unsafe { op_pointer(h.engine, 50, 0, 200.0, 300.0, 1_000) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(h.engine, 50, 1, 0.0, 300.0, 1_001) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(h.engine, 50, 2, 0.0, 300.0, 1_002) },
        OpStatus::Ok
    );
    assert_ne!(
        h.frame(),
        before,
        "a content-owned drag must keep pointer capture outside the safe-local viewport"
    );
}

#[test]
fn text_insert_backspace_and_state_round_trip() {
    let h = Harness::create(TEXT_DOC);
    tap(&h, 1, TEXT_TAP.0, TEXT_TAP.1);
    let status = unsafe {
        let text = b"Hello";
        op_text_insert(h.engine, text.as_ptr(), text.len())
    };
    assert_eq!(status, OpStatus::Ok);

    let mut state = empty_state();
    assert_eq!(
        unsafe { op_text_get_state(h.engine, &mut state) },
        OpStatus::Ok
    );
    let text = unsafe { std::slice::from_raw_parts(state.text_ptr, state.text_len) };
    // Insert lands at the caret, which starts at the end of the content.
    assert_eq!(String::from_utf8_lossy(text), "Hello WorldHello");

    assert_eq!(unsafe { op_text_backspace(h.engine) }, OpStatus::Ok);
    assert_eq!(
        unsafe { op_text_get_state(h.engine, &mut state) },
        OpStatus::Ok
    );
    let text = unsafe { std::slice::from_raw_parts(state.text_ptr, state.text_len) };
    assert_eq!(String::from_utf8_lossy(text), "Hello WorldHell");

    // Backspace without a session is NoFocus.
    unsafe { op_text_end(h.engine) };
    let status = unsafe { op_text_backspace(h.engine) };
    assert_eq!(status, OpStatus::NoFocus);

    // Beginning on a non-text node id is InvalidArg.
    let status = unsafe { op_text_begin(h.engine, b"f1".as_ptr(), 2) };
    assert_eq!(status, OpStatus::InvalidArg);
}

#[test]
fn ime_composition_commit_and_cancel() {
    let h = Harness::create(TEXT_DOC);
    tap(&h, 1, TEXT_TAP.0, TEXT_TAP.1);

    // setComposingText("中文", cursor at end = 2 UTF-16 units)
    let text = "中文";
    let status = unsafe { op_ime_set_composing_text(h.engine, text.as_ptr(), text.len(), 2, 2) };
    assert_eq!(status, OpStatus::Ok);

    let mut state = empty_state();
    assert_eq!(
        unsafe { op_text_get_state(h.engine, &mut state) },
        OpStatus::Ok
    );
    assert!(state.has_composing);
    // The composition region covers the inserted text at the caret (the
    // caret starts at the end of the content: 11 UTF-16 units in).
    assert_eq!((state.composing_start, state.composing_end), (11, 13));

    // Commit the composition → the draft contains the CJK chars.
    assert_eq!(unsafe { op_ime_commit_composition(h.engine) }, OpStatus::Ok);
    assert_eq!(
        unsafe { op_text_get_state(h.engine, &mut state) },
        OpStatus::Ok
    );
    let draft = unsafe { std::slice::from_raw_parts(state.text_ptr, state.text_len) };
    assert_eq!(String::from_utf8_lossy(draft), "Hello World中文");
    assert!(!state.has_composing);

    // Cancel path: a new composition is discarded.
    let status = unsafe { op_ime_set_composing_text(h.engine, text.as_ptr(), text.len(), 2, 2) };
    assert_eq!(status, OpStatus::Ok);
    assert_eq!(unsafe { op_ime_cancel_composition(h.engine) }, OpStatus::Ok);
    assert_eq!(
        unsafe { op_text_get_state(h.engine, &mut state) },
        OpStatus::Ok
    );
    let draft = unsafe { std::slice::from_raw_parts(state.text_ptr, state.text_len) };
    assert_eq!(String::from_utf8_lossy(draft), "Hello World中文");
    unsafe { op_text_end(h.engine) };
}

#[test]
fn caret_rect_is_finite_and_in_viewport() {
    let h = Harness::create(TEXT_DOC);
    tap(&h, 1, TEXT_TAP.0, TEXT_TAP.1);
    let mut rect = [f32::NAN; 4];
    assert_eq!(
        unsafe { op_text_caret_rect(h.engine, rect.as_mut_ptr()) },
        OpStatus::Ok
    );
    for value in rect {
        assert!(value.is_finite() && value >= 0.0, "caret rect {rect:?}");
    }
    // Without a session the rect query reports NoFocus.
    unsafe { op_text_end(h.engine) };
    let status = unsafe { op_text_caret_rect(h.engine, rect.as_mut_ptr()) };
    assert_eq!(status, OpStatus::NoFocus);
}

#[test]
fn tapping_outside_commits_the_edit_and_selects() {
    let h = Harness::create(TEXT_DOC);
    tap(&h, 1, TEXT_TAP.0, TEXT_TAP.1);
    assert!(h.last_focused());
    let status = unsafe {
        let text = b"X";
        op_text_insert(h.engine, text.as_ptr(), text.len())
    };
    assert_eq!(status, OpStatus::Ok);
    // Tap far from the page (viewport corner) → commit + clear selection.
    tap(&h, 2, 5.0, 5.0);
    assert!(!h.last_focused());
    // The committed text is in the document: re-enter edit and check the
    // draft starts with the inserted char.
    let status = unsafe { op_text_begin(h.engine, b"t1".as_ptr(), 2) };
    assert_eq!(status, OpStatus::Ok);
    let mut state = empty_state();
    assert_eq!(
        unsafe { op_text_get_state(h.engine, &mut state) },
        OpStatus::Ok
    );
    let draft = unsafe { std::slice::from_raw_parts(state.text_ptr, state.text_len) };
    assert_eq!(String::from_utf8_lossy(draft), "Hello WorldX");
    unsafe { op_text_end(h.engine) };
}

// ---- Page switching -----------------------------------------------------

#[test]
fn page_count_and_switch_change_rendering() {
    let h = Harness::create(TWO_PAGE_DOC);
    let mut count = 0u32;
    assert_eq!(
        unsafe { op_get_page_count(h.engine, &mut count) },
        OpStatus::Ok
    );
    assert_eq!(count, 2);

    let page1 = h.frame();
    // The fitted red page covers the viewport centre.
    let center = pixel_at(&page1, 200, 300, 400);
    assert!(center[0] > 200, "page 1 should be red-dominant: {center:?}");

    assert_eq!(unsafe { op_set_active_page(h.engine, 1) }, OpStatus::Ok);
    let page2 = h.frame();
    let center = pixel_at(&page2, 200, 300, 400);
    assert!(
        center[1] > 200,
        "page 2 should be green-dominant: {center:?}"
    );
    assert_ne!(page1, page2);

    // Out-of-range index is rejected.
    let status = unsafe { op_set_active_page(h.engine, 2) };
    assert_eq!(status, OpStatus::InvalidArg);
}

// ---- Remote images ------------------------------------------------------

#[test]
fn remote_image_requests_are_drained_and_results_stored() {
    let h = Harness::create(REMOTE_IMAGE_DOC);
    h.frame();
    // The paint recorded the https image miss; the frame drain fired the
    // upcall with the source id.
    assert_eq!(h.image_requests(), 1);
    let request_id = h.last_image_request_id();
    assert!(request_id > 0);

    // Push a real PNG back: the next paint decodes it (no crash).
    let status = unsafe {
        op_remote_image_result(
            h.engine,
            request_id,
            RED_PIXEL_PNG.as_ptr(),
            RED_PIXEL_PNG.len(),
        )
    };
    assert_eq!(status, OpStatus::Ok);
    h.frame();

    // A failed fetch (empty bytes) is also accepted and terminates the
    // request cleanly.
    let status = unsafe { op_remote_image_result(h.engine, request_id, ptr::null(), 0) };
    assert_eq!(status, OpStatus::Ok);
}

// ---- Imported fonts -----------------------------------------------------

#[test]
fn register_font_accepts_valid_ttf_and_rejects_garbage() {
    let h = Harness::create(TEXT_DOC);
    let status = unsafe { op_register_font(h.engine, DEMO_FONT.as_ptr(), DEMO_FONT.len()) };
    assert_eq!(
        status,
        OpStatus::Ok,
        "font register failed: {}",
        h.last_error()
    );

    let garbage = b"not a font";
    let status = unsafe { op_register_font(h.engine, garbage.as_ptr(), garbage.len()) };
    assert_eq!(status, OpStatus::InvalidArg);
}

// ---- helpers ------------------------------------------------------------

fn empty_state() -> OpTextState {
    OpTextState {
        text_ptr: ptr::null(),
        text_len: 0,
        selection_start: 0,
        selection_end: 0,
        has_composing: false,
        composing_start: 0,
        composing_end: 0,
    }
}

/// Read one RGBA pixel from a tight RGBA buffer (row-major, width known).
fn pixel_at(buffer: &[u8], x: usize, y: usize, width: usize) -> [u8; 4] {
    let i = y * width * 4 + x * 4;
    [buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]]
}

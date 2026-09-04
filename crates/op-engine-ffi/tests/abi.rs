//! ABI-level tests for `op-engine-ffi` — everything goes through the C
//! surface (`op_create` / `op_frame_cpu` / …), so the tests double as a
//! contract check for the shells.

use op_engine_ffi::{
    op_create, op_destroy, op_frame, op_frame_cpu, op_get_pixel_size, op_last_error, op_pointer,
    op_prefers_light_system_icons, op_resize, op_resize_with_safe_area, op_resume, op_suspend,
    OpCreateDesc, OpEngine, OpStatus,
};
use std::ptr;

/// A canonical v1.0.0 document: a 1080×1440 "daily sign card" with a
/// painted outer frame and layered content (bundled with op-editor-core
/// as a scene template).
const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

fn create_engine(width: f32, height: f32, dpr: f32) -> Result<*mut OpEngine, OpStatus> {
    let doc = SAMPLE_DOC.as_bytes();
    let desc = OpCreateDesc {
        size: std::mem::size_of::<OpCreateDesc>(),
        doc_ptr: doc.as_ptr(),
        doc_len: doc.len(),
        width,
        height,
        dpr,
        callbacks: ptr::null(),
        asset_base_ptr: ptr::null(),
        asset_base_len: 0,
        mode: 0,
        storage_root_ptr: ptr::null(),
        storage_root_len: 0,
        documents_root_ptr: ptr::null(),
        documents_root_len: 0,
    };
    let mut engine: *mut OpEngine = ptr::null_mut();
    let status = unsafe { op_create(&desc, &mut engine) };
    if status != OpStatus::Ok {
        return Err(status);
    }
    Ok(engine)
}

fn last_error(engine: *mut OpEngine) -> String {
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

/// Render one CPU frame into a tight RGBA buffer.
fn cpu_frame(engine: *mut OpEngine, now_ms: u64) -> Vec<u8> {
    let mut width = 0u32;
    let mut height = 0u32;
    let status = unsafe { op_get_pixel_size(engine, &mut width, &mut height) };
    assert_eq!(status, OpStatus::Ok);
    let stride = width as usize * 4;
    let mut buffer = vec![0u8; height as usize * stride];
    let status = unsafe { op_frame_cpu(engine, now_ms, buffer.as_mut_ptr(), buffer.len(), stride) };
    assert_eq!(
        status,
        OpStatus::Ok,
        "op_frame_cpu failed: {}",
        last_error(engine)
    );
    buffer
}

#[test]
fn create_and_render_cpu_frame() {
    let engine = create_engine(400.0, 533.0, 2.0).expect("engine creation");
    let buffer = cpu_frame(engine, 0);
    // 400×533 logical @2 → 800×1066 physical.
    assert_eq!(buffer.len(), 800 * 1066 * 4);

    // The page must actually paint: at least some pixels differ from the
    // backdrop white-ish fill, and not every pixel is identical.
    let first = buffer[0];
    assert!(
        buffer.iter().any(|&b| b != first),
        "frame is a single flat color — nothing painted"
    );

    let status = unsafe { op_destroy(engine) };
    assert_eq!(status, OpStatus::Ok);
}

#[test]
fn bad_document_reports_error_through_last_error() {
    let garbage = br#"{"version":"1.0.0","children":[{"type":"nonsense"}]"#;
    let desc = OpCreateDesc {
        size: std::mem::size_of::<OpCreateDesc>(),
        doc_ptr: garbage.as_ptr(),
        doc_len: garbage.len(),
        width: 100.0,
        height: 100.0,
        dpr: 1.0,
        callbacks: ptr::null(),
        asset_base_ptr: ptr::null(),
        asset_base_len: 0,
        mode: 0,
        storage_root_ptr: ptr::null(),
        storage_root_len: 0,
        documents_root_ptr: ptr::null(),
        documents_root_len: 0,
    };
    let mut engine: *mut OpEngine = ptr::null_mut();
    let status = unsafe { op_create(&desc, &mut engine) };
    assert_eq!(status, OpStatus::BadDocument);
    assert!(engine.is_null());
    let message = last_error(ptr::null_mut());
    assert!(
        !message.is_empty(),
        "op_last_error must report the schema rejection"
    );
}

#[test]
fn invalid_desc_is_rejected() {
    let status = unsafe { op_create(ptr::null(), ptr::null_mut()) };
    assert_eq!(status, OpStatus::InvalidArg);
}

#[test]
fn viewer_system_icon_preference_is_dark_and_validates_output() {
    let engine = create_engine(200.0, 200.0, 1.0).expect("engine creation");
    assert_eq!(
        unsafe { op_prefers_light_system_icons(engine, ptr::null_mut()) },
        OpStatus::InvalidArg
    );

    let mut prefers_light = true;
    assert_eq!(
        unsafe { op_prefers_light_system_icons(engine, &mut prefers_light) },
        OpStatus::Ok
    );
    assert!(!prefers_light);
    assert_eq!(unsafe { op_destroy(engine) }, OpStatus::Ok);
}

#[test]
fn engine_is_bound_to_its_creator_thread() {
    let engine = create_engine(200.0, 200.0, 1.0).expect("engine creation");
    // The raw pointer is deliberately !Send; a usize copy rides into the
    // spawned thread so the ABI itself can reject the cross-thread call.
    let raw = engine as usize;
    let handle = std::thread::spawn(move || {
        let engine = raw as *mut OpEngine;
        let mut buffer = vec![0u8; 200 * 200 * 4];
        unsafe { op_frame_cpu(engine, 0, buffer.as_mut_ptr(), buffer.len(), 200 * 4) }
    });
    let status = handle.join().unwrap();
    assert_eq!(status, OpStatus::WrongThread);
    // Destroying from the wrong thread is also refused.
    let handle = std::thread::spawn(move || unsafe { op_destroy(raw as *mut OpEngine) });
    assert_eq!(handle.join().unwrap(), OpStatus::WrongThread);
    // The owner thread can still destroy it.
    let status = unsafe { op_destroy(engine) };
    assert_eq!(status, OpStatus::Ok);
}

#[test]
fn resize_updates_pixel_size() {
    let engine = create_engine(300.0, 400.0, 1.0).expect("engine creation");
    let mut width = 0u32;
    let mut height = 0u32;
    let status = unsafe { op_get_pixel_size(engine, &mut width, &mut height) };
    assert_eq!(status, OpStatus::Ok);
    assert_eq!((width, height), (300, 400));

    let status = unsafe { op_resize(engine, 120.0, 240.0, 2.0) };
    assert_eq!(status, OpStatus::Ok);
    let status = unsafe { op_get_pixel_size(engine, &mut width, &mut height) };
    assert_eq!(status, OpStatus::Ok);
    assert_eq!((width, height), (240, 480));

    let status = unsafe { op_destroy(engine) };
    assert_eq!(status, OpStatus::Ok);
}

#[test]
fn atomic_resize_with_safe_area_updates_one_valid_tuple() {
    let engine = create_engine(300.0, 400.0, 1.0).expect("engine creation");
    let mut width = 0u32;
    let mut height = 0u32;

    assert_eq!(
        unsafe { op_resize_with_safe_area(engine, 120.0, 240.0, 2.0, 24.0, 10.0, 20.0, 12.0) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_get_pixel_size(engine, &mut width, &mut height) },
        OpStatus::Ok
    );
    assert_eq!((width, height), (240, 480));

    assert_eq!(
        unsafe { op_resize_with_safe_area(engine, 150.0, 250.0, 3.0, 24.0, 10.0, 20.0, -1.0) },
        OpStatus::InvalidArg
    );
    assert_eq!(
        unsafe { op_get_pixel_size(engine, &mut width, &mut height) },
        OpStatus::Ok
    );
    assert_eq!((width, height), (240, 480));

    assert_eq!(unsafe { op_destroy(engine) }, OpStatus::Ok);
}

#[test]
fn tap_selects_and_dead_space_does_not_change_pixels() {
    let engine = create_engine(400.0, 533.0, 1.0).expect("engine creation");
    let baseline = cpu_frame(engine, 0);

    // A tap far from the page (viewport corner) is dead space: nothing
    // changes.
    let status = unsafe { op_pointer(engine, 1, 0, 2.0, 2.0, 0) };
    assert_eq!(status, OpStatus::Ok);
    let status = unsafe { op_pointer(engine, 1, 2, 2.0, 2.0, 1) };
    assert_eq!(status, OpStatus::Ok);
    let after_dead_tap = cpu_frame(engine, 2);
    assert_eq!(baseline, after_dead_tap, "dead-space tap must not repaint");

    // A tap at the page center (the fitted 1080×1440 page fills most of
    // the 400×533 viewport; its centre is ~(200, 266) and lands on the
    // painted outer frame at minimum) selects a node → selection stroke
    // changes pixels.
    let status = unsafe { op_pointer(engine, 2, 0, 200.0, 266.0, 3) };
    assert_eq!(status, OpStatus::Ok);
    let status = unsafe { op_pointer(engine, 2, 2, 200.0, 266.0, 4) };
    assert_eq!(status, OpStatus::Ok);
    let after_tap = cpu_frame(engine, 5);
    assert_ne!(
        baseline, after_tap,
        "tap on a node must paint the selection overlay"
    );

    let status = unsafe { op_destroy(engine) };
    assert_eq!(status, OpStatus::Ok);
}

#[test]
fn suspend_blocks_pointer_and_frame_until_resume() {
    let engine = create_engine(200.0, 300.0, 1.0).expect("engine creation");

    let status = unsafe { op_suspend(engine) };
    assert_eq!(status, OpStatus::Ok);
    // A stale platform display callback must not advance runtime work after
    // suspend; only `op_background_tick` is legal until resume.
    let status = unsafe { op_frame(engine, 0) };
    assert_eq!(status, OpStatus::Suspended);
    // Pointer input is refused while suspended.
    let status = unsafe { op_pointer(engine, 1, 0, 10.0, 10.0, 0) };
    assert_eq!(status, OpStatus::Suspended);

    let status = unsafe { op_resume(engine, ptr::null()) };
    assert_eq!(status, OpStatus::Ok);
    let status = unsafe { op_pointer(engine, 1, 0, 10.0, 10.0, 1) };
    assert_eq!(status, OpStatus::Ok);

    let status = unsafe { op_destroy(engine) };
    assert_eq!(status, OpStatus::Ok);
}

#[test]
fn gpu_frame_without_surface_is_not_ready() {
    let engine = create_engine(100.0, 100.0, 1.0).expect("engine creation");
    // On a host build without a GPU feature the surface backend is not
    // compiled; with one, the handle is still never attached here.
    let status = unsafe { op_frame(engine, 0) };
    assert_eq!(status, OpStatus::NotReady);
    let status = unsafe { op_destroy(engine) };
    assert_eq!(status, OpStatus::Ok);
}

#[test]
fn null_destroy_is_invalid() {
    // A null engine is rejected before any dereference; destroying a
    // freed engine is UB by contract (the handle must be live), so only
    // the null path is testable.
    let status = unsafe { op_destroy(ptr::null_mut()) };
    assert_eq!(status, OpStatus::InvalidArg);
}

#[cfg(feature = "editor")]
#[test]
fn image_import_action_code_and_abi_entrypoint_are_stable() {
    assert_eq!(op_engine_ffi::SHELL_ACTION_IMPORT_IMAGE_OR_SVG, 12);
    let desc = OpCreateDesc {
        size: std::mem::size_of::<OpCreateDesc>(),
        doc_ptr: ptr::null(),
        doc_len: 0,
        width: 1_024.0,
        height: 768.0,
        dpr: 1.0,
        callbacks: ptr::null(),
        asset_base_ptr: ptr::null(),
        asset_base_len: 0,
        mode: 1,
        storage_root_ptr: ptr::null(),
        storage_root_len: 0,
        documents_root_ptr: ptr::null(),
        documents_root_len: 0,
    };
    let mut engine = ptr::null_mut();
    assert_eq!(unsafe { op_create(&desc, &mut engine) }, OpStatus::Ok);
    let png = b"\x89PNG\r\n\x1a\nabi-image";
    let name = b"abi.png";
    assert_eq!(
        unsafe {
            op_engine_ffi::op_editor_import_image_or_svg(
                engine,
                png.as_ptr(),
                png.len(),
                name.as_ptr(),
                name.len(),
            )
        },
        OpStatus::Ok
    );
    assert_eq!(unsafe { op_destroy(engine) }, OpStatus::Ok);
}

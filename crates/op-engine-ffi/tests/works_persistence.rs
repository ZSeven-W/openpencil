#![cfg(feature = "editor")]

//! Public-ABI regression for the phone Save -> terminate -> Works path.
//! A separate binary isolates the process-wide private storage root.

use op_editor_core::{EditorState, HomeFamily, TaskDraft, Tool, WorkspacePhase};
use op_editor_ui::widgets::mobile_chrome::{more_entry_rect, more_panel_rect, MobileMoreEntry};
use op_editor_ui::widgets::{HomeSurface, MobileAppBar, WorksReader};
use op_editor_ui::Rect;
use op_engine_ffi::*;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

const WIDTH: f32 = 390.0;
const HEIGHT: f32 = 844.0;
const DOCUMENT: &str = r##"{
  "version":"1.0.0", "name":"Three phone screens",
  "pages":[
    {"id":"phones", "name":"Phone screens", "children":[
      {"type":"frame","id":"one","name":"One","x":0,"y":0,"width":375,"height":812,"fill":[{"type":"solid","color":"#12C874"}],"children":[]},
      {"type":"frame","id":"two","name":"Two","x":450,"y":0,"width":375,"height":812,"fill":[{"type":"solid","color":"#234FC9"}],"children":[]},
      {"type":"frame","id":"three","name":"Three","x":900,"y":0,"width":375,"height":812,"fill":[{"type":"solid","color":"#DD3344"}],"children":[]}
    ]},
    {"id":"notes", "name":"Notes", "children":[{"type":"rectangle","id":"note","width":200,"height":100}]}
  ], "children":[]
}"##;
static NOW_MS: AtomicU64 = AtomicU64::new(1_000);

fn create(root: &Path, documents_root: &Path, source: &str) -> *mut OpEngine {
    let storage = root.to_str().unwrap().as_bytes();
    let documents = documents_root.to_str().unwrap().as_bytes();
    let desc = OpCreateDesc {
        size: std::mem::size_of::<OpCreateDesc>(),
        doc_ptr: source.as_ptr(),
        doc_len: source.len(),
        width: WIDTH,
        height: HEIGHT,
        dpr: 1.0,
        callbacks: ptr::null(),
        asset_base_ptr: ptr::null(),
        asset_base_len: 0,
        mode: 1,
        storage_root_ptr: storage.as_ptr(),
        storage_root_len: storage.len(),
        documents_root_ptr: documents.as_ptr(),
        documents_root_len: documents.len(),
    };
    let mut engine = ptr::null_mut();
    assert_eq!(unsafe { op_create(&desc, &mut engine) }, OpStatus::Ok);
    assert_eq!(
        unsafe { op_editor_configure_save_picker(engine, true) },
        OpStatus::Ok
    );
    engine
}

fn tap(engine: *mut OpEngine, rect: Rect) {
    assert!(rect.size.x > 0.0 && rect.size.y > 0.0);
    let x = rect.origin.x + rect.size.x / 2.0;
    let y = rect.origin.y + rect.size.y / 2.0;
    let now = NOW_MS.fetch_add(500, Ordering::Relaxed);
    assert_eq!(
        unsafe { op_editor_press_at(engine, x, y, now) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release_at(engine, x, y, now + 20) },
        OpStatus::Ok
    );
    // Run the same frame pump as the shell so deferred touch transitions and
    // layout settle before the next visible control is pressed.
    let stride = WIDTH as usize * 4;
    let mut pixels = vec![0_u8; stride * HEIGHT as usize];
    assert_eq!(
        unsafe { op_frame_cpu(engine, now + 400, pixels.as_mut_ptr(), pixels.len(), stride) },
        OpStatus::Ok
    );
}

fn phone_layout_state() -> EditorState {
    let mut state = EditorState::starter();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = op_editor_core::size_class::EditorSizeClass::Compact;
    state.editor_ui.home.visible = true;
    state
}

fn drain(engine: *mut OpEngine) -> i32 {
    let mut action = -1;
    assert_eq!(
        unsafe { op_editor_take_shell_action(engine, &mut action) },
        OpStatus::Ok
    );
    action
}

fn file_name(engine: *mut OpEngine) -> String {
    let mut required = 0;
    assert_eq!(
        unsafe { op_editor_copy_save_file_name(engine, ptr::null_mut(), 0, &mut required) },
        OpStatus::Ok
    );
    let mut bytes = vec![0; required];
    assert_eq!(
        unsafe {
            op_editor_copy_save_file_name(engine, bytes.as_mut_ptr(), bytes.len(), &mut required)
        },
        OpStatus::Ok
    );
    String::from_utf8(bytes).unwrap()
}

fn page_count(engine: *mut OpEngine) -> u32 {
    let mut count = 0;
    assert_eq!(
        unsafe { op_get_page_count(engine, &mut count) },
        OpStatus::Ok
    );
    count
}

/// Measure the unique green selected-board fill in an actual ABI-rendered
/// frame. This verifies viewport geometry without adding a test-only ABI.
fn reader_board_width(engine: *mut OpEngine, reference: &EditorState) -> i32 {
    let stage = WorksReader::for_editor(reference)
        .unwrap()
        .layout(WIDTH, HEIGHT)
        .stage;
    let stride = WIDTH as usize * 4;
    let mut pixels = vec![0_u8; stride * HEIGHT as usize];
    let now = NOW_MS.fetch_add(500, Ordering::Relaxed);
    assert_eq!(
        unsafe { op_frame_cpu(engine, now, pixels.as_mut_ptr(), pixels.len(), stride) },
        OpStatus::Ok
    );
    let y = (stage.origin.y + stage.size.y / 2.0) as usize;
    let green: Vec<_> = (0..WIDTH as usize)
        .filter(|x| {
            let p = &pixels[y * stride + x * 4..][..3];
            (16..=20).contains(&p[0]) && (198..=202).contains(&p[1]) && (114..=118).contains(&p[2])
        })
        .collect();
    (green.last().expect("the selected board must be visible") - green.first().unwrap() + 1) as i32
}

#[test]
fn picker_save_is_persisted_before_destroy_and_cold_create_reopens_it_from_works() {
    let root = std::env::temp_dir().join(format!("op-ffi-works-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let containers = root.join("Containers/Data/Application");
    let old_documents = containers.join("11111111-1111-1111-1111-111111111111/Documents");
    let new_documents = containers.join("22222222-2222-2222-2222-222222222222/Documents");
    std::fs::create_dir_all(&old_documents).unwrap();
    let first = create(&root, &old_documents, DOCUMENT);
    assert_eq!(page_count(first), 2);
    // Navigate through the ordinary Works reader, then its real More -> Save
    // controls. Reference state supplies geometry only, never engine state.
    tap(
        first,
        MobileAppBar::home_rect(Rect::xywh(0.0, 0.0, WIDTH, 52.0)),
    );
    let mut layout_state = phone_layout_state();
    let home = HomeSurface::for_editor(&layout_state).unwrap();
    tap(first, home.layout(WIDTH, HEIGHT).nav_items[1]);
    let loaded = jian_ops_schema::load_str(DOCUMENT).unwrap().value;
    layout_state.replace_document(loaded);
    layout_state.editor_ui.home.works_open = true;
    let home = HomeSurface::for_editor(&layout_state).unwrap();
    tap(first, home.works_layout(WIDTH, HEIGHT).current.unwrap());
    layout_state.editor_ui.home.visible = false;
    layout_state.editor_ui.open_workspace_for_generation(
        HomeFamily::AppUi,
        "",
        TaskDraft::default(),
        0,
        0,
        Some(Tool::Select),
    );
    layout_state.editor_ui.workspace.phase = WorkspacePhase::Done;
    let initial_width = reader_board_width(first, &layout_state);
    assert!(
        initial_width > 200,
        "the selected phone board should fill the reader stage"
    );
    tap(
        first,
        WorksReader::for_editor(&layout_state)
            .unwrap()
            .layout(WIDTH, HEIGHT)
            .more,
    );
    let index = MobileMoreEntry::visible(&layout_state)
        .iter()
        .position(|entry| *entry == MobileMoreEntry::SaveFile)
        .unwrap();
    tap(
        first,
        more_entry_rect(
            &layout_state,
            more_panel_rect(&layout_state, WIDTH, HEIGHT),
            index,
        ),
    );
    assert_eq!(drain(first), SHELL_ACTION_SAVE_DOCUMENT);
    let name = file_name(first);
    let staging = root.join(&name);
    let staging_string = staging.to_str().unwrap();
    assert_eq!(
        unsafe {
            op_editor_stage_save_to_path(first, staging_string.as_ptr(), staging_string.len())
        },
        OpStatus::Ok
    );
    let handle = "content://test/saved-work";
    assert_eq!(
        unsafe {
            op_editor_commit_save(
                first,
                handle.as_ptr(),
                handle.len(),
                name.as_ptr(),
                name.len(),
            )
        },
        OpStatus::Ok
    );

    // Check BEFORE suspend/destroy: an OS termination may skip both, and those
    // unconditional flushes previously masked the missing fingerprint field.
    let settings: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("settings.json"))
            .expect("Save itself must persist the Works index"),
    )
    .unwrap();
    let recent = settings["recent_files"]
        .as_array()
        .expect("persisted recents");
    assert_eq!(recent.len(), 1);
    let saved_path = recent[0]["path"].as_str().unwrap();
    assert!(Path::new(saved_path).is_file());

    // Recreate through op_create while the first handle has not been flushed
    // or destroyed. Startup must read its own settings; no manual load call.
    let second = create(&root, &old_documents, "");
    assert_eq!(page_count(second), 1);
    let mut cold_layout = phone_layout_state();
    let home = HomeSurface::for_editor(&cold_layout).unwrap();
    tap(second, home.layout(WIDTH, HEIGHT).nav_items[1]);
    cold_layout.editor_ui.home.works_open = true;
    cold_layout
        .editor_ui
        .touch_recent_file(saved_path.to_owned(), 0);
    let home = HomeSurface::for_editor(&cold_layout).unwrap();
    tap(second, home.works_layout(WIDTH, HEIGHT).rows[0]);
    assert_eq!(drain(second), SHELL_ACTION_NONE);
    assert_eq!(
        page_count(second),
        2,
        "cold Works opens the saved two-page document"
    );
    let reopened_width = reader_board_width(second, &layout_state);
    assert!((initial_width - reopened_width).abs() <= 2, "cold reader must frame one board, not shrink to fit all three: {initial_width} -> {reopened_width}");

    // An iOS update can relocate the app's Documents container. Its copied
    // Saved file must be found using the new root on the next op_create.
    let saved_path = Path::new(saved_path);
    let relative = saved_path.strip_prefix(&old_documents).unwrap();
    let relocated = new_documents.join(relative);
    std::fs::create_dir_all(relocated.parent().unwrap()).unwrap();
    std::fs::copy(saved_path, &relocated).unwrap();
    std::fs::remove_file(saved_path).unwrap();
    let upgraded = create(&root, &new_documents, "");
    let relocated_settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        relocated_settings["recent_files"][0]["path"].as_str(),
        relocated.to_str()
    );
    let home = HomeSurface::for_editor(&cold_layout).unwrap();
    tap(upgraded, home.layout(WIDTH, HEIGHT).nav_items[1]);
    tap(upgraded, home.works_layout(WIDTH, HEIGHT).rows[0]);
    assert_eq!(drain(upgraded), SHELL_ACTION_NONE);
    assert_eq!(
        page_count(upgraded),
        2,
        "upgrade keeps the saved work reopenable"
    );
    assert!((initial_width - reader_board_width(upgraded, &layout_state)).abs() <= 2);
    assert_eq!(unsafe { op_destroy(first) }, OpStatus::Ok);
    assert_eq!(unsafe { op_destroy(second) }, OpStatus::Ok);
    assert_eq!(unsafe { op_destroy(upgraded) }, OpStatus::Ok);
    std::fs::remove_dir_all(root).unwrap();
}

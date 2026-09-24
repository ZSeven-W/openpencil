//! Offscreen phone works-reader and 作品-page screenshots through the
//! production paint path (the same harness shape as `home_shots`).
//!
//! Run with:
//! ```text
//! cargo run -p op-host-native --features gl-host --example reader_shots -- <out_dir>
//! ```

use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{HomeFamily, TaskDraft, Tool, WorkspacePhase};
use op_editor_ui::widgets::WorksReader;
use op_host_native::backend::{NativeBackend, NativeFrameBackend};
use op_host_native::widget_host::WidgetHostNative;

const W: f32 = 390.0;
const H: f32 = 844.0;

fn resource(name: &str) -> String {
    format!(
        "{}/../../packaging/ios/Resources/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// A phone host over `file`, reading it as a finished `family` work.
fn phone_host(file: &str, family: HomeFamily, brief: &str) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let src = std::fs::read_to_string(resource(file)).expect("read sample document");
    let loaded = op_pen_loader::load_canonical(&src).expect("parse sample document");
    let mut state = op_editor_core::EditorState::from_document(loaded.value);
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.locale = op_i18n::Locale::ZhCn;
    assert!(host.replace_editor_state(state));
    host.set_now_ms(1_000);
    host.editor_state_mut()
        .editor_ui
        .open_workspace_for_generation(
            family,
            brief,
            TaskDraft::default(),
            0,
            1_000,
            Some(Tool::Select),
        );
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Done;
    host.frame_reader_board(W, H);
    host
}

fn shoot(host: &mut WidgetHostNative, out_dir: &str, name: &str) {
    let mut backend = NativeBackend::with_dpi(2.0);
    let mut surface =
        skia_safe::surfaces::raster_n32_premul((W as i32, H as i32)).expect("raster surface");
    for frame in 0..6 {
        host.set_now_ms(2_000 + frame * 300);
        {
            let mut frame_backend = NativeFrameBackend::new(&mut backend, surface.canvas());
            host.paint(&mut frame_backend, W, H);
        }
        for pending in
            op_editor_ui::widgets::canvas_viewport_image::take_pending_decodes(usize::MAX)
        {
            match op_editor_ui::widgets::canvas_viewport_image::cached_bytes_for(pending.id)
                .and_then(|bytes| op_host_native::decode_raster_capped(&bytes, pending.max_edge_px))
            {
                Some((image, covers)) => {
                    backend.install_raster_image(pending.id, image, covers);
                    op_editor_ui::widgets::canvas_viewport_image::mark_decode_done(pending.id);
                }
                None => {
                    op_editor_ui::widgets::canvas_viewport_image::mark_decode_failed(pending.id)
                }
            }
        }
    }
    let data = surface
        .image_snapshot()
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .expect("encode png");
    let target = format!("{out_dir}/{name}.png");
    std::fs::write(&target, data.as_bytes()).expect("write png");
    println!("reader-shots: wrote {target}");
}

fn tap(host: &mut WidgetHostNative, rect: op_editor_ui::Rect) {
    let (x, y) = (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    );
    host.apply_press(x, y, W, H);
    host.apply_release_with_viewport(W, H);
}

fn reader_layout(host: &WidgetHostNative) -> op_editor_ui::widgets::ReaderLayout {
    WorksReader::for_editor(host.editor_state())
        .expect("reader visible")
        .layout(W, H)
}

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/reader-shots".into());
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    // A finished deck, page 1, then page 2 via the real pager.
    let brief = "为 OpenPencil 做一份产品介绍 PPT";
    let mut deck = phone_host("ppt-demo.op", HomeFamily::Presentation, brief);
    shoot(&mut deck, &out_dir, "reader-deck-page1");
    let next = reader_layout(&deck).next.expect("paged");
    tap(&mut deck, next);
    shoot(&mut deck, &out_dir, "reader-deck-page2");

    // 改这一页 → the chat sheet opens over the reader, bound to page 2.
    let edit = reader_layout(&deck).edit_page;
    tap(&mut deck, edit);
    shoot(&mut deck, &out_dir, "reader-deck-edit-page-sheet");

    // A page edit in flight: status names the page, Stop is offered.
    let mut running = phone_host("ppt-demo.op", HomeFamily::Presentation, brief);
    {
        let boards = op_editor_core::preview_slideshow::active_page_boards(running.editor_state());
        let workspace = &mut running.editor_state_mut().editor_ui.workspace;
        workspace.select_board(1, boards.len());
        workspace.stage_page_edit(boards[1].clone(), 1);
        workspace.begin_page_edit_turn();
        workspace.phase = WorkspacePhase::Generating;
    }
    running.frame_reader_board(W, H);
    shoot(&mut running, &out_dir, "reader-deck-generating");

    // A failed run offers Retry.
    running.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Failed;
    running
        .editor_state_mut()
        .editor_ui
        .workspace
        .page_edit_running = None;
    shoot(&mut running, &out_dir, "reader-deck-failed");

    // An app work (phone screens) read page by page.
    let mut app = phone_host("sample.op", HomeFamily::AppUi, "咖啡外带预约 App");
    shoot(&mut app, &out_dir, "reader-app");

    // 专业 from the reader: the full mobile canvas on the same page.
    let professional = reader_layout(&app).mode_professional;
    tap(&mut app, professional);
    shoot(&mut app, &out_dir, "reader-app-professional");

    // Back to Home, then the 作品 page listing the live work + recents.
    let mut works = phone_host("sample.op", HomeFamily::AppUi, "咖啡外带预约 App");
    works.editor_state_mut().editor_ui.recent_files =
        ["季度复盘.op", "活动海报-春季.op", "咖啡 App.op"]
            .iter()
            .map(|name| op_editor_core::RecentFile {
                path: format!("/Users/me/Documents/OpenPencil/{name}"),
                modified_at: 0,
            })
            .collect();
    let back = reader_layout(&works).back;
    tap(&mut works, back);
    shoot(&mut works, &out_dir, "home-back-to-workspace");
    let nav = op_editor_ui::widgets::HomeSurface::for_editor(works.editor_state())
        .expect("home")
        .layout(W, H)
        .nav_items[1];
    tap(&mut works, nav);
    shoot(&mut works, &out_dir, "works-list");
}

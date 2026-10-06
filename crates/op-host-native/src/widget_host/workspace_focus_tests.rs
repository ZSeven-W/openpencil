//! Exercise single-page scope through native chrome actions and cached pan frames.

use super::*;
use crate::backend::{NativeBackend, NativeFrameBackend};
use op_editor_core::{EditorState, HomeFamily};

const W: i32 = 1440;
const H: i32 = 900;

fn frame_colors(host: &mut WidgetHostNative, backend: &mut NativeBackend) -> (usize, usize) {
    let mut surface = skia_safe::surfaces::raster_n32_premul((W, H)).unwrap();
    {
        let mut frame = NativeFrameBackend::new(backend, surface.canvas());
        host.paint(&mut frame, W as f32, H as f32);
    }
    let mut pixels = vec![0; (W * H * 4) as usize];
    let info = skia_safe::ImageInfo::new(
        (W, H),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    assert!(surface.read_pixels(&info, &mut pixels, (W * 4) as usize, (0, 0)));
    let mut red = 0;
    let mut blue = 0;
    // Exclude chrome, thumbnails and the chat rail from the pixel probes.
    for y in 120..H - 120 {
        for x in 340..W {
            let offset = ((y * W + x) * 4) as usize;
            let pixel = &pixels[offset..offset + 4];
            red += usize::from(pixel[0] > 240 && pixel[1] < 20 && pixel[2] < 20);
            blue += usize::from(pixel[2] > 240 && pixel[0] < 20 && pixel[1] < 20);
        }
    }
    (red, blue)
}

fn action(host: &mut WidgetHostNative, hit: WorkspaceHit) {
    let surface = WorkspaceSurface::for_editor(host.editor_state()).unwrap();
    let layout = surface.layout(W as f32, H as f32);
    host.run_workspace_action(hit, &layout, 0.0);
    host.mark_dirty();
}

#[test]
fn native_single_page_pager_and_pan_cache_keep_neighbors_hidden() {
    let _guard = crate::agent_indicator_test_support::read();
    let doc = jian_ops_schema::load_str(r##"{"version":"1.0.0","children":[
        {"type":"frame","id":"a","name":"First","width":375,"height":500,"fill":[{"type":"solid","color":"#ff0000"}],"children":[]},
        {"type":"frame","id":"b","name":"Second","x":420,"width":375,"height":500,"fill":[{"type":"solid","color":"#0000ff"}],"children":[]}
    ]}"##).unwrap().value;
    let original = doc.clone();
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(doc));
    {
        let state = host.editor_state_mut();
        state
            .editor_ui
            .workspace
            .open_for_reading(HomeFamily::KnowledgeCards, 2);
        state.tool = Tool::Hand;
        state.chat.focused = false;
    }
    host.set_now_ms(2_000);
    let mut backend = NativeBackend::with_dpi(1.0);
    action(
        &mut host,
        WorkspaceHit::View(WorkspaceView::Single { index: 0 }),
    );
    let (red, blue) = frame_colors(&mut host, &mut backend);
    assert!(red > 10_000);
    assert_eq!(blue, 0, "neighbor must not be painted in single-page mode");
    let fonts_before_pan = jian_skia::font_generation();
    for delta in [12.0, 8.0] {
        assert!(host.apply_pan_gesture(800.0, 400.0, delta, 0.0, W as f32, H as f32));
        let (red, blue) = frame_colors(&mut host, &mut backend);
        assert!(red > 10_000);
        assert_eq!(blue, 0, "a cached pan frame must retain its board scope");
    }
    // Other native tests install fonts globally and correctly invalidate the cache.
    // An isolated run must prove that at least one frame actually used a cached blit.
    if jian_skia::font_generation() == fonts_before_pan {
        assert!(host.pan_cache_blits_for_test() > 0);
    } else {
        eprintln!("cache lifecycle inconclusive under concurrent font installs");
    }
    action(&mut host, WorkspaceHit::Next);
    let (red, blue) = frame_colors(&mut host, &mut backend);
    assert_eq!(red, 0, "paging must drop the previous board's cached paint");
    assert!(blue > 10_000);
    action(&mut host, WorkspaceHit::Prev);
    let (red, blue) = frame_colors(&mut host, &mut backend);
    assert!(red > 10_000);
    assert_eq!(blue, 0);
    action(&mut host, WorkspaceHit::View(WorkspaceView::AllBoards));
    let (red, blue) = frame_colors(&mut host, &mut backend);
    assert!(red > 10_000 && blue > 10_000);
    assert_eq!(host.editor_state().doc, original);
}

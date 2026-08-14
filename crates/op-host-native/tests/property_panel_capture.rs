//! Raster capture coverage for a selected Frame's Property Panel.
//!
//! Set `OPENPENCIL_PROPERTY_PANEL_SHOT_DIR` to keep the rendered PNGs. Without
//! it, the test still paints every viewport and verifies that Skia produced
//! non-empty pixels.

#![cfg(all(feature = "gl-host", not(target_os = "windows")))]

use std::path::{Path, PathBuf};

use op_editor_core::size_class::{size_class, MobileSheetKind};
use op_editor_core::NodeId;
use op_host_native::backend::{NativeBackend, NativeFrameBackend};
use op_host_native::WidgetHostNative;

const VIEWPORTS: [(&str, i32, i32, bool); 6] = [
    ("compact.png", 390, 844, false),
    ("compact-bottom.png", 390, 844, true),
    ("medium.png", 834, 1_112, false),
    ("medium-bottom.png", 834, 1_112, true),
    ("expanded.png", 1_194, 834, false),
    ("expanded-bottom.png", 1_194, 834, true),
];

#[test]
fn selected_frame_property_panel_paints_at_every_touch_size_class() {
    let shot_dir = std::env::var_os("OPENPENCIL_PROPERTY_PANEL_SHOT_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    if let Some(dir) = shot_dir.as_deref() {
        std::fs::create_dir_all(dir).expect("create Property Panel shot directory");
    }

    for (file_name, width, height, scroll_to_bottom) in VIEWPORTS {
        paint_viewport(
            file_name,
            width,
            height,
            scroll_to_bottom,
            shot_dir.as_deref(),
        );
    }
}

fn paint_viewport(
    file_name: &str,
    width: i32,
    height: i32,
    scroll_to_bottom: bool,
    shot_dir: Option<&Path>,
) {
    let mut host = WidgetHostNative::new();
    {
        let state = host.editor_state_mut();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = size_class(width as f32, height as f32);
        state.editor_ui.mobile_sheet = Some(MobileSheetKind::Properties);
        if scroll_to_bottom {
            state.editor_ui.property_panel_scroll.offset = f32::MAX;
        }
        state.set_single_selection(NodeId::new("n10"));
    }
    host.mark_editor_state_dirty();
    let panel = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .expect("selected starter Frame resolves a Property Panel");
    assert_eq!(panel.snapshot.kind, "Frame");

    let mut backend = NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((width, height))
        .expect("allocate Property Panel raster surface");
    {
        let mut frame = NativeFrameBackend::new(&mut backend, surface.canvas());
        host.paint(&mut frame, width as f32, height as f32);
    }

    let stride = width as usize * 4;
    let mut pixels = vec![0u8; stride * height as usize];
    let info = skia_safe::ImageInfo::new(
        (width, height),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    assert!(
        surface.read_pixels(&info, &mut pixels, stride, (0, 0)),
        "read {file_name} raster pixels"
    );
    assert!(
        pixels.chunks_exact(4).any(|pixel| pixel[3] != 0),
        "{file_name} must contain at least one non-transparent pixel"
    );

    if let Some(dir) = shot_dir {
        let data = surface
            .image_snapshot()
            .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
            .expect("encode Property Panel PNG");
        let path = dir.join(file_name);
        std::fs::write(&path, data.as_bytes()).expect("write Property Panel PNG");
        eprintln!("Property Panel shot written to {}", path.display());
    }
}

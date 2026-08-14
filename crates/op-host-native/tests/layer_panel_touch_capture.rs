//! Raster capture coverage for the responsive touch Layers surface.
//!
//! Set `OPENPENCIL_LAYER_PANEL_SHOT_DIR` to keep the rendered PNGs. Without
//! it, the test still paints every viewport and verifies non-empty output.

#![cfg(all(feature = "gl-host", not(target_os = "windows")))]

use std::path::{Path, PathBuf};

use op_editor_core::size_class::{size_class, MobileSheetKind};
use op_host_native::backend::{NativeBackend, NativeFrameBackend};
use op_host_native::WidgetHostNative;

const VIEWPORTS: [(&str, i32, i32); 3] = [
    ("compact.png", 390, 844),
    ("medium.png", 834, 1_112),
    ("expanded.png", 1_194, 834),
];

#[test]
fn touch_layers_surface_paints_at_every_size_class() {
    let shot_dir = std::env::var_os("OPENPENCIL_LAYER_PANEL_SHOT_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    if let Some(dir) = shot_dir.as_deref() {
        std::fs::create_dir_all(dir).expect("create Layers shot directory");
    }

    for (file_name, width, height) in VIEWPORTS {
        paint_viewport(file_name, width, height, shot_dir.as_deref());
    }
}

fn paint_viewport(file_name: &str, width: i32, height: i32, shot_dir: Option<&Path>) {
    let mut host = WidgetHostNative::new();
    {
        let state = host.editor_state_mut();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = size_class(width as f32, height as f32);
        state.editor_ui.sidebar_open = state.editor_ui.expanded_touch_layout();
        state.editor_ui.mobile_sheet =
            (!state.editor_ui.expanded_touch_layout()).then_some(MobileSheetKind::Layers);
    }
    host.mark_editor_state_dirty();

    let mut backend = NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((width, height))
        .expect("allocate Layers raster surface");
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
            .expect("encode Layers PNG");
        let path = dir.join(file_name);
        std::fs::write(&path, data.as_bytes()).expect("write Layers PNG");
        eprintln!("Layers shot written to {}", path.display());
    }
}

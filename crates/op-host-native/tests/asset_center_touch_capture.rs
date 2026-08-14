//! Raster capture coverage for the Asset Center's phone and tablet layouts.
//!
//! Set `OPENPENCIL_ASSET_CENTER_SHOT_DIR` to keep the rendered PNGs. Without
//! it, the test still paints each viewport and verifies non-empty output.

#![cfg(all(feature = "gl-host", not(target_os = "windows")))]

use std::path::{Path, PathBuf};

use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::AssetCenterTab;
use op_host_native::backend::{NativeBackend, NativeFrameBackend};
use op_host_native::WidgetHostNative;

const VIEWPORTS: [(&str, i32, i32, EditorSizeClass, AssetCenterTab); 2] = [
    (
        "compact-templates.png",
        390,
        844,
        EditorSizeClass::Compact,
        AssetCenterTab::Templates,
    ),
    (
        "medium-styles.png",
        834,
        1_112,
        EditorSizeClass::Medium,
        AssetCenterTab::Styles,
    ),
];

#[test]
fn asset_center_paints_non_empty_touch_frames() {
    let shot_dir = std::env::var_os("OPENPENCIL_ASSET_CENTER_SHOT_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    if let Some(dir) = shot_dir.as_deref() {
        std::fs::create_dir_all(dir).expect("create Asset Center shot directory");
    }

    for (file_name, width, height, size_class, tab) in VIEWPORTS {
        paint_viewport(
            file_name,
            width,
            height,
            size_class,
            tab,
            shot_dir.as_deref(),
        );
    }
}

fn paint_viewport(
    file_name: &str,
    width: i32,
    height: i32,
    size_class: EditorSizeClass,
    tab: AssetCenterTab,
    shot_dir: Option<&Path>,
) {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.touch = true;
        ui.size_class = size_class;
        ui.sidebar_open = false;
        ui.open_scene_template_center(0);
        ui.scene_template_center.select_tab(tab);
    }
    host.mark_editor_state_dirty();
    assert!(host.editor_state().editor_ui.scene_template_center.open);
    assert_eq!(host.editor_state().editor_ui.scene_template_center.tab, tab);

    let mut backend = NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((width, height))
        .expect("allocate Asset Center raster surface");
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
            .expect("encode Asset Center PNG");
        let path = dir.join(file_name);
        std::fs::write(&path, data.as_bytes()).expect("write Asset Center PNG");
        eprintln!("Asset Center shot written to {}", path.display());
    }
}

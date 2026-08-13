//! Raster capture coverage for Agent Settings across the three touch layouts.
//!
//! Set `OPENPENCIL_AGENT_SETTINGS_SHOT_DIR` to keep the rendered PNGs. Without
//! it, the test still paints every viewport and verifies that Skia produced
//! non-empty pixels.

#![cfg(all(feature = "gl-host", not(target_os = "windows")))]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use op_editor_core::agent_settings::{AgentSettingsTab, ImageGenField, SettingsFocus};
use op_editor_core::missing_fonts::{MissingFontEntry, MissingFontsPrompt};
use op_editor_core::size_class::size_class;
use op_host_native::backend::{NativeBackend, NativeFrameBackend};
use op_host_native::WidgetHostNative;

const CAPTURES: [(&str, i32, i32, AgentSettingsTab); 7] = [
    ("compact.png", 390, 844, AgentSettingsTab::Agents),
    ("medium.png", 834, 1_112, AgentSettingsTab::Agents),
    ("expanded.png", 1_194, 834, AgentSettingsTab::Agents),
    ("images-compact.png", 390, 844, AgentSettingsTab::Images),
    ("images-medium.png", 834, 1_112, AgentSettingsTab::Images),
    ("fonts-compact.png", 390, 844, AgentSettingsTab::Fonts),
    ("fonts-medium.png", 834, 1_112, AgentSettingsTab::Fonts),
];

#[test]
fn agent_settings_paints_non_empty_frames_for_every_touch_size_class() {
    let shot_dir = std::env::var_os("OPENPENCIL_AGENT_SETTINGS_SHOT_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    if let Some(dir) = shot_dir.as_deref() {
        std::fs::create_dir_all(dir).expect("create Agent Settings shot directory");
    }

    for (file_name, width, height, tab) in CAPTURES {
        paint_viewport(file_name, width, height, tab, shot_dir.as_deref());
    }
}

fn paint_viewport(
    file_name: &str,
    width: i32,
    height: i32,
    tab: AgentSettingsTab,
    shot_dir: Option<&Path>,
) {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.touch = true;
        ui.size_class = size_class(width as f32, height as f32);
        ui.agent_settings_open = true;
        ui.agent_settings.tab = tab;
        match tab {
            AgentSettingsTab::Agents => ui.agent_settings.begin_builtin_agent_draft(),
            AgentSettingsTab::Images => {
                ui.agent_settings.images_advanced_open = true;
                ui.agent_settings.add_image_gen_profile();
                ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
                    index: 0,
                    field: ImageGenField::Name,
                });
            }
            AgentSettingsTab::Fonts => {
                ui.missing_fonts_prompt = Some(MissingFontsPrompt {
                    entries: vec![MissingFontEntry {
                        family: "Source Han Sans".into(),
                        run_count: 3,
                        mismatch_note: None,
                        resolved: false,
                    }],
                });
                ui.imported_font_families = Arc::new(vec!["Inter".into()]);
                ui.system_font_families = Arc::new(vec!["Arial".into(), "Helvetica Neue".into()]);
                ui.font_import_supported = true;
                ui.open_missing_font_picker(0, op_editor_core::MissingFontSurface::Settings);
            }
            _ => unreachable!("capture table only contains Agents, Images, and Fonts"),
        }
    }
    host.mark_editor_state_dirty();

    let mut backend = NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((width, height))
        .expect("allocate Agent Settings raster surface");
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
            .expect("encode Agent Settings PNG");
        let path = dir.join(file_name);
        std::fs::write(&path, data.as_bytes()).expect("write Agent Settings PNG");
        eprintln!("Agent Settings shot written to {}", path.display());
    }
}

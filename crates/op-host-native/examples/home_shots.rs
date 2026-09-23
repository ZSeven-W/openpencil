//! Offscreen Studio Home screenshots through the production paint path.
//!
//! Renders the Home surface at the two reference viewports into raster
//! surfaces and writes PNGs, so the design can be compared against the
//! approved prototype captures without opening a window. Multi-frame
//! painting lets the lazy image decodes (brand mark, template previews)
//! resolve before the capture frame.
//!
//! Run with:
//! ```text
//! cargo run -p op-host-native --features gl-host --example home_shots -- <out_dir>
//! ```

use op_host_native::backend::{NativeBackend, NativeFrameBackend};
use op_host_native::widget_host::WidgetHostNative;

/// `(width, height, name, compact)` — `compact` is the phone
/// composition (touch chrome + Compact size class), the same predicate
/// `EditorUiState::compact_layout()` gates the layout branch on.
const VIEWPORTS: [(f32, f32, &str, bool); 3] = [
    (1440.0, 900.0, "home-1440x900", false),
    (1180.0, 820.0, "home-1180x820", false),
    (390.0, 844.0, "home-390x844-phone", true),
];

/// Both themes, so the pair can be compared at the same viewport.
const THEMES: [(op_editor_core::ThemeMode, &str); 2] = [
    (op_editor_core::ThemeMode::Light, "light"),
    (op_editor_core::ThemeMode::Dark, "dark"),
];

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/home-shots".into());
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    for ((w, h, base_name, compact), (theme, theme_name)) in VIEWPORTS
        .into_iter()
        .flat_map(|vp| THEMES.into_iter().map(move |t| (vp, t)))
    {
        let name: &str = &format!("{base_name}-{theme_name}");
        let mut host = WidgetHostNative::new();
        // The approved prototype capture's state: light theme, a usable
        // chat model selected (the primary button reads 开始设计).
        host.editor_state_mut().editor_ui.theme_mode = theme;
        host.editor_state_mut().editor_ui.home.visible = true;
        if compact {
            host.editor_state_mut().editor_ui.touch = true;
            host.editor_state_mut().editor_ui.size_class =
                op_editor_core::size_class::EditorSizeClass::Compact;
            // A real document behind the Home takeover, so the 专业 shot
            // proves the phone hands over to the full canvas rather than
            // to an empty placeholder.
            if let Ok(src) = std::fs::read_to_string("packaging/ios/Resources/sample.op") {
                if let Ok(loaded) = op_pen_loader::load_canonical(&src) {
                    host.editor_state_mut().doc = loaded.value;
                    host.editor_state_mut().mark_document_changed();
                }
            }
        }
        host.editor_state_mut().editor_ui.agent_settings.connected[0] = true;
        host.editor_state_mut().chat.available_models = vec![op_editor_core::ModelEntry::new(
            op_editor_core::AgentProvider::ClaudeCode,
            "glm-5.3-flash",
            "GLM 5.3 Flash",
        )];
        host.editor_state_mut().chat.selected_model = 0;
        host.set_now_ms(1_000);
        let mut backend = NativeBackend::with_dpi(2.0);
        let mut surface = skia_safe::surfaces::raster_n32_premul((w as i32, h as i32))
            .expect("raster surface allocated");
        // A few frames: the first stamps the entrance clock and queues the
        // template decodes; between frames the queued ids decode + install
        // synchronously (the desktop frame loop's ImageDecodeHost thread
        // does this in the app), and the next frame paints them resident.
        for frame in 0..6 {
            host.set_now_ms(1_000 + frame * 300);
            {
                let mut frame_backend = NativeFrameBackend::new(&mut backend, surface.canvas());
                host.paint(&mut frame_backend, w, h);
            }
            for pending in
                op_editor_ui::widgets::canvas_viewport_image::take_pending_decodes(usize::MAX)
            {
                match op_editor_ui::widgets::canvas_viewport_image::cached_bytes_for(pending.id)
                    .and_then(|bytes| {
                        op_host_native::decode_raster_capped(&bytes, pending.max_edge_px)
                    }) {
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
        write_png(&surface.image_snapshot(), &out_dir, name);

        // The phone also documents the other half of the mode switch:
        // press 专业 and capture the real canvas it hands over to.
        if compact {
            let professional = op_editor_ui::widgets::HomeSurface::for_editor(host.editor_state())
                .expect("home visible")
                .layout(w, h)
                .professional;
            assert!(host.apply_press(
                professional.origin.x + professional.size.x / 2.0,
                professional.origin.y + professional.size.y / 2.0,
                w,
                h,
            ));
            assert!(!host.editor_state().editor_ui.home.visible);
            // The app bar's 适应 (fit) target, pressed through the real
            // touch chrome.
            let bar = op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_rect(
                host.editor_state(),
                w,
            );
            let fit = op_editor_ui::widgets::MobileAppBar::fit_rect(bar);
            host.apply_press(
                fit.origin.x + fit.size.x / 2.0,
                fit.origin.y + fit.size.y / 2.0,
                w,
                h,
            );
            for frame in 0..4 {
                host.set_now_ms(4_000 + frame * 300);
                let mut frame_backend = NativeFrameBackend::new(&mut backend, surface.canvas());
                host.paint(&mut frame_backend, w, h);
            }
            write_png(
                &surface.image_snapshot(),
                &out_dir,
                "home-390x844-professional",
            );
        }
    }
}

fn write_png(image: &skia_safe::Image, out_dir: &str, name: &str) {
    let data = image
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .expect("encode png");
    let target = format!("{out_dir}/{name}.png");
    std::fs::write(&target, data.as_bytes()).expect("write png");
    println!("home-shots: wrote {target}");
}

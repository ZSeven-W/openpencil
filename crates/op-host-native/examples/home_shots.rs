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

const VIEWPORTS: [(f32, f32, &str); 2] = [
    (1440.0, 900.0, "home-1440x900"),
    (1180.0, 820.0, "home-1180x820"),
];

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/home-shots".into());
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    for (w, h, name) in VIEWPORTS {
        let mut host = WidgetHostNative::new();
        // The approved prototype capture's state: light theme, a usable
        // chat model selected (the primary button reads 开始设计).
        host.editor_state_mut().editor_ui.theme_mode = op_editor_core::ThemeMode::Light;
        host.editor_state_mut().editor_ui.home.visible = true;
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
        let image = surface.image_snapshot();
        let data = image
            .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
            .expect("encode png");
        let target = format!("{out_dir}/{name}.png");
        std::fs::write(&target, data.as_bytes()).expect("write png");
        println!("home-shots: wrote {target}");
    }
}

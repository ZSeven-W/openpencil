//! Bake Home's single-screen previews from the real first template board.
use op_editor_ui::widgets::{canvas_viewport_paint, PaintCx};
use op_editor_ui::{Point2D, Rect};
use op_host_native::backend::{NativeBackend, NativeFrameBackend};

fn main() {
    let out = std::env::args().nth(1).expect("preview output directory");
    for (id, scale) in [
        ("coffee-order-app", 1.0_f32),
        ("coffee-counter-desktop", 0.5),
    ] {
        let source = op_editor_core::scene_template_catalog::scene_template_document(id).unwrap();
        let doc = op_pen_loader::load_canonical(source).unwrap().value;
        let state = op_editor_core::EditorState::from_document(doc);
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&state);
        let node = &scene.active_page().unwrap().children[0];
        let bounds = node.aggregate_bounds();
        let size = Point2D::new(bounds.size.x * scale, bounds.size.y * scale);
        let mut surface =
            skia_safe::surfaces::raster_n32_premul((size.x as i32, size.y as i32)).unwrap();
        let mut backend = NativeBackend::with_dpi(1.0);
        for _ in 0..6 {
            surface.canvas().clear(skia_safe::Color::WHITE);
            let mut frame = NativeFrameBackend::new(&mut backend, surface.canvas());
            canvas_viewport_paint::paint_node(
                &mut PaintCx {
                    backend: &mut frame,
                },
                node,
                Point2D::new(-bounds.origin.x * scale, -bounds.origin.y * scale),
                scale,
                Rect {
                    origin: Point2D::ZERO,
                    size,
                },
            );
            for pending in
                op_editor_ui::widgets::canvas_viewport_image::take_pending_decodes(usize::MAX)
            {
                if let Some((image, covers)) =
                    op_editor_ui::widgets::canvas_viewport_image::cached_bytes_for(pending.id)
                        .and_then(|bytes| {
                            op_host_native::decode_raster_capped(&bytes, pending.max_edge_px)
                        })
                {
                    backend.install_raster_image(pending.id, image, covers);
                    op_editor_ui::widgets::canvas_viewport_image::mark_decode_done(pending.id);
                }
            }
        }
        let bytes = surface
            .image_snapshot()
            .encode(None, skia_safe::EncodedImageFormat::JPEG, 92)
            .unwrap();
        let path = format!("{out}/{id}-single.jpg");
        std::fs::write(&path, bytes.as_bytes()).unwrap();
        println!("Baked {path}: {}x{}", size.x, size.y);
    }
}

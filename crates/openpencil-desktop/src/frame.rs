//! Per-frame paint pass for the desktop runner — extracted from
//! `main.rs` to keep that file under the 800-line cap.

use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend};
use openpencil_shell_native::{
    NativeBackend, NativeFrameBackend, SharedSkiaContext, WidgetHostNative,
};

/// Paint pass — clear, scale by DPI, dispatch to the widget host,
/// present the GL surface.
pub fn paint(
    ctx: &mut SharedSkiaContext,
    backend: &mut NativeBackend,
    host: &WidgetHostNative,
    viewport_width: f32,
    viewport_height: f32,
    dpi: f32,
) {
    ctx.begin_frame();
    ctx.with_frame(|canvas, _glow| {
        canvas.clear(skia_safe::Color::BLACK);
        canvas.reset_matrix();
        canvas.scale((dpi, dpi));
        let mut frame = NativeFrameBackend::new(backend, canvas);
        frame.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, viewport_height),
            },
            Color::BLACK,
        );
        host.paint(&mut frame, viewport_width, viewport_height);
    });
    ctx.present();
}

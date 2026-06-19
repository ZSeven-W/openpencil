//! The only module permitted to call `op_editor_ui::widgets::*` (widget-boundary rule).
//!
//! Provides the sole paint entry point for the read-only viewer. All widget
//! logic is contained here; `render.rs` calls through this module without
//! touching `op_editor_ui::widgets` directly.
//!
//! `paint_scene` is called only from the `canvaskit`-feature render path.
#![allow(dead_code)]

use op_editor_core::Viewport as DocViewport;
use op_editor_ui::layout_scene::LayoutScene;
use op_editor_ui::theme::Theme;
use op_editor_ui::widgets::canvas_viewport::CanvasViewport;
use op_editor_ui::widgets::{PaintCx, Widget};
use op_editor_ui::{Rect, RenderBackend};

// glue: read-only viewer paint pass.
pub fn paint_scene(
    backend: &mut dyn RenderBackend,
    scene: &LayoutScene,
    viewport: DocViewport,
    theme: Theme,
    w: f32,
    h: f32,
) {
    let view = CanvasViewport::from_scene(scene, viewport, theme);
    let mut cx = PaintCx { backend };
    view.paint(&mut cx, Rect::xywh(0.0, 0.0, w, h));
}

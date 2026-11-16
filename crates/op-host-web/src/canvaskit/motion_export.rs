//! Paint an isolated clip on a CanvasKit raster surface and encode it locally.

use op_editor_core::motion_export::{export_reduced_motion, MotionExportRequest};
use op_editor_core::{walkers::find_node, EditorState, NodeId};
use op_editor_ui::widgets::{canvas_viewport_paint, PaintCx};
use op_editor_ui::{Point2D, Rect, RenderBackend};
use op_motion_export::{FrameClock, MotionError, MotionScene};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use super::{CanvasKitBackend, OpCk};

pub(crate) struct LocalMotionExport {
    scene: MotionScene,
    clock: FrameClock,
    reduced: bool,
}

impl LocalMotionExport {
    pub(crate) fn prepare(
        state: &EditorState,
        request: &MotionExportRequest,
    ) -> Result<Self, MotionError> {
        FrameClock::validate_fps(request.fps)?;
        let root = NodeId::new_opt(request.root_id.clone())
            .and_then(|id| find_node(state.active_children(), &id))
            .ok_or_else(|| MotionError::RootMissing(request.root_id.clone()))?;
        let clip = jian_core::motion::Clip::build(root, &request.trigger.as_clip_trigger());
        if clip.is_empty() {
            return Err(MotionError::EmptyClip);
        }
        let clock = FrameClock::new(
            request.fps,
            request.duration_ms.or_else(|| clip.duration_ms()),
        )?;
        clock.validate_mp4()?;
        let layout = op_pen_loader::editor_state_to_active_page_export_layout_scene(state);
        Ok(Self {
            scene: MotionScene::new(&layout, &request.root_id, request.scale, clip)?,
            clock,
            reduced: export_reduced_motion(&state.doc, request.reduced_motion),
        })
    }
}

pub(crate) async fn encode_mp4(ck: OpCk, export: LocalMotionExport) -> Result<Vec<u8>, JsValue> {
    use op_motion_export::Mp4Encoder;

    let size = export.scene.size;
    let mut output = std::io::Cursor::new(Vec::new());
    let mut encoder = Mp4Encoder::new(&mut output, size, export.clock).map_err(encoder_error)?;
    let surface = ExportSurface::new(ck.clone(), size.0 as u32, size.1 as u32)?;
    // The same bridge carries the editor's font and raster caches. Its
    // backing surface is switched only during each synchronous paint.
    let mut backend = CanvasKitBackend {
        ck,
        dpr: export.scene.scale,
        quantize_rect_colors: true,
        logical_w: size.0 as u32,
        logical_h: size.1 as u32,
    };
    let mut rgba = vec![0; size.0 as usize * size.1 as usize * 4];

    let first = export.scene.sample(0, export.reduced);
    // Discover images before encoding so the first frame never ships the
    // editor's lazy placeholders. A bounded queue may take several passes.
    for _ in 0..64 {
        paint(&surface, &mut backend, &export.scene, &first, 0)?;
        if backend.drain_pending_decodes(usize::MAX) == 0 {
            break;
        }
    }
    for index in 0..export.clock.frame_count() {
        yield_to_browser().await?;
        let time = export.clock.time_at(index);
        let root = export.scene.sample(time, export.reduced);
        let shader_time = if export.reduced { 0 } else { time };
        paint(&surface, &mut backend, &export.scene, &root, shader_time)?;
        surface.read_rgba(&mut rgba)?;
        encoder.push_rgba(&rgba).map_err(encoder_error)?;
    }
    encoder.finish().map_err(encoder_error)?;
    Ok(output.into_inner())
}

fn encoder_error(error: MotionError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

struct ExportSurface {
    ck: OpCk,
    handle: JsValue,
    width: u32,
    height: u32,
}

impl ExportSurface {
    fn new(ck: OpCk, width: u32, height: u32) -> Result<Self, JsValue> {
        let handle = ck.create_export_surface(width, height)?;
        Ok(Self {
            ck,
            handle,
            width,
            height,
        })
    }

    fn read_rgba(&self, rgba: &mut [u8]) -> Result<(), JsValue> {
        let pixels = self
            .ck
            .read_export_surface(&self.handle, self.width, self.height)?;
        if pixels.length() as usize != rgba.len() {
            return Err(JsValue::from_str("wrong CanvasKit export pixel count"));
        }
        pixels.copy_to(rgba);
        Ok(())
    }
}

impl Drop for ExportSurface {
    fn drop(&mut self) {
        self.ck.delete_export_surface(&self.handle);
    }
}

fn paint(
    surface: &ExportSurface,
    backend: &mut CanvasKitBackend,
    scene: &MotionScene,
    root: &op_editor_ui::layout_scene::SceneNode,
    shader_time: u64,
) -> Result<(), JsValue> {
    let mut draw = || {
        backend.begin_frame();
        surface.ck.clear(1.0, 1.0, 1.0, 1.0);
        surface
            .ck
            .translate(-scene.bounds.origin.x, -scene.bounds.origin.y);
        op_editor_ui::shader_clock::with_sample_time(shader_time, || {
            canvas_viewport_paint::paint_node(
                &mut PaintCx { backend },
                root,
                Point2D::ZERO,
                1.0,
                Rect::xywh(-1.0e15, -1.0e15, 2.0e15, 2.0e15),
            );
        });
        backend.end_frame();
    };
    let callback = ScopedClosure::<dyn FnMut()>::borrow_mut_assert_unwind_safe(&mut draw);
    surface
        .ck
        .with_export_surface(&surface.handle, callback.as_ref().unchecked_ref())
}

async fn yield_to_browser() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let next_task = js_sys::Promise::new(&mut |resolve, reject| {
        if let Err(error) =
            window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
        {
            let _ = reject.call1(&JsValue::UNDEFINED, &error);
        }
    });
    JsFuture::from(next_task).await?;
    Ok(())
}

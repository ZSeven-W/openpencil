use jian_core::motion::Clip;
use op_editor_ui::layout_scene::LayoutScene;
use op_motion_export::MotionScene;
use skia_safe::{AlphaType, ColorType, ImageInfo, Surface};

use crate::{encode_surface, paint_node, raster_surface};
use crate::{ExportError, RasterFormat};

pub(super) struct FrameRenderer {
    scene: MotionScene,
    transparent: bool,
    reduced_motion: bool,
}

impl FrameRenderer {
    pub(super) fn new(
        scene: &LayoutScene,
        root_id: &str,
        scale: f32,
        clip: Clip,
    ) -> Result<Self, ExportError> {
        Ok(Self {
            scene: MotionScene::new(scene, root_id, scale, clip)?,
            transparent: true,
            reduced_motion: false,
        })
    }

    pub(super) fn with_options(mut self, transparent: bool, reduced_motion: bool) -> Self {
        self.transparent = transparent;
        self.reduced_motion = reduced_motion;
        self
    }

    pub(super) fn size(&self) -> (i32, i32) {
        self.scene.size
    }

    pub(super) fn render(&self, time_ms: u64) -> Result<Surface, ExportError> {
        let root = self.scene.sample(time_ms, self.reduced_motion);
        raster_surface(
            self.scene.size,
            self.scene.bounds,
            self.scene.scale,
            0.0,
            self.transparent,
            |canvas| {
                let shader_time = if self.reduced_motion { 0 } else { time_ms };
                op_editor_ui::shader_clock::with_sample_time(shader_time, || {
                    paint_node(canvas, &root)
                });
            },
        )
    }

    pub(super) fn render_png(&self, time_ms: u64) -> Result<Vec<u8>, ExportError> {
        encode_surface(&mut self.render(time_ms)?, RasterFormat::Png)
    }

    pub(super) fn read_rgba(&self, time_ms: u64, pixels: &mut [u8]) -> Result<(), ExportError> {
        let mut surface = self.render(time_ms)?;
        let info = ImageInfo::new(
            self.scene.size,
            ColorType::RGBA8888,
            AlphaType::Unpremul,
            None,
        );
        if !surface.read_pixels(&info, pixels, self.scene.size.0 as usize * 4, (0, 0)) {
            return Err(ExportError::MotionReadback);
        }
        Ok(())
    }
}

use jian_core::motion::Clip;
use jian_scene::layout_scene_motion::apply_motion_overrides;
use op_editor_ui::layout_scene::LayoutScene;
use op_editor_ui::Rect;
use skia_safe::{AlphaType, ColorType, ImageInfo, Surface};

use crate::{collect_bounds, encode_surface, paint_node, raster_size, raster_surface, BoundsAcc};
use crate::{ExportError, RasterFormat};

pub(super) struct FrameRenderer {
    scene: LayoutScene,
    clip: Clip,
    scale: f32,
    bounds: Rect,
    size: (i32, i32),
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
        let page = scene.active_page().ok_or(ExportError::NoActivePage)?;
        let root = page
            .find(root_id)
            .ok_or_else(|| ExportError::NodeNotFoundOnActivePage {
                node_id: root_id.into(),
            })?;
        if root.hidden {
            return Err(ExportError::NodeHidden {
                node_id: root_id.into(),
            });
        }
        let mut bounds = BoundsAcc::new();
        collect_bounds(root, glam::Affine2::IDENTITY, &mut bounds);
        let bounds = bounds
            .into_rect()
            .ok_or_else(|| ExportError::NodePaintsNothing {
                node_id: root_id.into(),
            })?;
        let size = raster_size(bounds, scale, 0.0)?;
        let scene = LayoutScene {
            pages: vec![op_editor_ui::layout_scene::ScenePage {
                id: page.id.clone(),
                name: page.name.clone(),
                children: vec![root.clone()],
            }],
            active_page_index: 0,
        };
        Ok(Self {
            scene,
            clip,
            scale,
            bounds,
            size,
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
        self.size
    }

    pub(super) fn render(&self, time_ms: u64) -> Result<Surface, ExportError> {
        let mut scene = self.scene.clone();
        let overrides = if self.reduced_motion {
            self.clip.sample_reduced(&self.scene)
        } else {
            self.clip.sample(time_ms, &self.scene)
        };
        apply_motion_overrides(&mut scene, &overrides);
        // The renderer's scene contains only the export root and its subtree.
        let root = &scene.pages[0].children[0];
        raster_surface(
            self.size,
            self.bounds,
            self.scale,
            0.0,
            self.transparent,
            |canvas| {
                let shader_time = if self.reduced_motion { 0 } else { time_ms };
                op_editor_ui::shader_clock::with_sample_time(shader_time, || {
                    paint_node(canvas, root)
                });
            },
        )
    }

    pub(super) fn render_png(&self, time_ms: u64) -> Result<Vec<u8>, ExportError> {
        encode_surface(&mut self.render(time_ms)?, RasterFormat::Png)
    }

    pub(super) fn read_rgba(&self, time_ms: u64, pixels: &mut [u8]) -> Result<(), ExportError> {
        let mut surface = self.render(time_ms)?;
        let info = ImageInfo::new(self.size, ColorType::RGBA8888, AlphaType::Unpremul, None);
        if !surface.read_pixels(&info, pixels, self.size.0 as usize * 4, (0, 0)) {
            return Err(ExportError::MotionReadback);
        }
        Ok(())
    }
}

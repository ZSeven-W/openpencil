use jian_core::motion::Clip;
use jian_scene::layout_scene_motion::apply_motion_overrides;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode, ScenePage};
use op_editor_ui::scene_bounds::{normalize_rect, BoundsAcc, PaintCornerRules};
use op_editor_ui::Rect;

use crate::{clamp_scale, raster_size, MotionError};

/// The isolated layout root, motion clip, and fixed crop shared by every frame.
pub struct MotionScene {
    scene: LayoutScene,
    clip: Clip,
    pub bounds: Rect,
    pub scale: f32,
    pub size: (i32, i32),
}

impl MotionScene {
    pub fn new(
        scene: &LayoutScene,
        root_id: &str,
        scale: f32,
        clip: Clip,
    ) -> Result<Self, MotionError> {
        let page = scene.active_page().ok_or(MotionError::NoActivePage)?;
        let root = page
            .find(root_id)
            .ok_or_else(|| MotionError::RootMissing(root_id.into()))?;
        if root.hidden {
            return Err(MotionError::RootHidden(root_id.into()));
        }
        let mut bounds = BoundsAcc::new();
        collect_raster_bounds(root, glam::Affine2::IDENTITY, &mut bounds);
        let bounds = bounds
            .into_rect()
            .ok_or_else(|| MotionError::RootPaintsNothing(root_id.into()))?;
        let scale = clamp_scale(scale);
        let size = raster_size(bounds, scale, 0.0)?;
        let scene = LayoutScene {
            pages: vec![ScenePage {
                id: page.id.clone(),
                name: page.name.clone(),
                children: vec![root.clone()],
            }],
            active_page_index: 0,
        };
        Ok(Self {
            scene,
            clip,
            bounds,
            scale,
            size,
        })
    }

    pub fn sample(&self, time_ms: u64, reduced: bool) -> SceneNode {
        let mut scene = self.scene.clone();
        let overrides = if reduced {
            self.clip.sample_reduced(&self.scene)
        } else {
            self.clip.sample(time_ms, &self.scene)
        };
        apply_motion_overrides(&mut scene, &overrides);
        scene.pages.pop().unwrap().children.pop().unwrap()
    }
}

/// Raster export counts solid fill / stroke as own paint only (the SVG
/// exporter also counts gradient + image fills).
const RASTER_PAINT_RULES: PaintCornerRules = PaintCornerRules {
    gradient_paints: false,
    image_paints: false,
};

/// Mirror of `paint_node`'s traversal — visits the SAME nodes paint
/// visits, in the SAME order, threading the cumulative parent
/// transform so a child painted under a rotated container ends up in
/// the same world coords paint will emit. `canvas_viewport_paint.rs`
/// pivots rotation around the node's `aggregate_bounds()` (own bounds
/// when bounded, child union otherwise); this mirrors that exactly so
/// the surface never clips a row a downstream draw would touch.
pub fn collect_raster_bounds(n: &SceneNode, parent_xform: glam::Affine2, acc: &mut BoundsAcc) {
    if n.hidden {
        return;
    }
    let pivot_rect = n.aggregate_bounds();
    let rotate_self =
        n.rotation.abs() > f32::EPSILON && (pivot_rect.size.x != 0.0 || pivot_rect.size.y != 0.0);
    let local_xform = if rotate_self {
        let pivot = glam::Vec2::new(
            pivot_rect.origin.x + pivot_rect.size.x * 0.5,
            pivot_rect.origin.y + pivot_rect.size.y * 0.5,
        );
        parent_xform
            * glam::Affine2::from_translation(pivot)
            * glam::Affine2::from_angle(n.rotation)
            * glam::Affine2::from_translation(-pivot)
    } else {
        parent_xform
    };
    if let Some(local_corners) = own_paint_corners(n) {
        for p in local_corners {
            let w = local_xform.transform_point2(p);
            acc.add(w.x, w.y, w.x, w.y);
        }
    }
    // A `clip_content` container clips its children to its own rect
    // (the painter pushes the same clip scope the live canvas does —
    // see `canvas_viewport_paint::push_clip_content`), so child
    // overflow can never paint outside it. Contribute the container
    // rect and skip the subtree instead of sizing the surface to the
    // un-clipped child union. Gate mirrors the painter: only bounded
    // containers with children actually clip.
    if n.clip_content && !n.children.is_empty() && n.bounds.size.x > 0.0 && n.bounds.size.y > 0.0 {
        let nr = normalize_rect(n.bounds);
        for (x, y) in [
            (nr.origin.x, nr.origin.y),
            (nr.origin.x + nr.size.x, nr.origin.y),
            (nr.origin.x + nr.size.x, nr.origin.y + nr.size.y),
            (nr.origin.x, nr.origin.y + nr.size.y),
        ] {
            let w = local_xform.transform_point2(glam::Vec2::new(x, y));
            acc.add(w.x, w.y, w.x, w.y);
        }
        return;
    }
    for child in &n.children {
        collect_raster_bounds(child, local_xform, acc);
    }
}

fn own_paint_corners(n: &SceneNode) -> Option<Vec<glam::Vec2>> {
    op_editor_ui::scene_bounds::own_paint_corners(n, RASTER_PAINT_RULES)
}

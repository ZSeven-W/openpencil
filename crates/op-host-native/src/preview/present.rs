//! Device-frame presentation for [`super::PreviewSession`]: framed-root
//! selection, bottom-nav detection, and the two-pass framed paint.
//!
//! Sibling of `input.rs` / `app_mode.rs` so `preview/mod.rs` stays under
//! the 800-line cap.

use super::PreviewSession;

use op_editor_ui::layout_scene::SceneNode;
use op_editor_ui::widgets::{
    paint_scene_page_with, paint_scene_subtree, PaintCx, PaintSceneOptions,
};
use op_editor_ui::{Point2D, Rect, RenderBackend};

/// Everything the pinned-nav pass needs, precomputed by the host's
/// `DeviceFrame` so paint and hit-testing share one set of numbers.
pub struct PinnedPaint {
    /// Schema id of the pinned nav node (excluded from pass 1).
    pub node_id: String,
    /// Screen-space clip rect of the strip (full inner frame width).
    pub strip_clip: Rect,
    /// Screen-space origin the nav subtree paints at.
    pub paint_origin: Point2D,
    /// The nav's scene origin (for the caret's page-origin math).
    pub nav_scene_origin: Point2D,
}

impl PreviewSession {
    /// The root the device frame presents: schema id + scene-space rect.
    pub fn framed_root(&self) -> Option<(String, Rect)> {
        let page = self.scene.active_page()?;
        page.children
            .first()
            .map(|node| (node.id.clone(), node.bounds))
    }

    /// Detect a pinnable bottom nav among the framed root's direct
    /// children. Semantic candidates take precedence over the positional
    /// heuristic, and document order breaks ties within each pass.
    pub fn pinned_nav_candidate(&self, phone: bool) -> Option<(String, Rect)> {
        if !phone {
            return None;
        }
        let page = self.scene.active_page()?;
        let root = page.children.first()?;
        let root_bottom = root.bounds.origin.y + root.bounds.size.y;
        let valid = |node: &SceneNode| {
            let height = node.bounds.size.y;
            !node.hidden && height.is_finite() && height > 0.0 && height <= 200.0
        };
        let bottom_gap =
            |node: &SceneNode| root_bottom - (node.bounds.origin.y + node.bounds.size.y);

        for child in &root.children {
            if !valid(child) {
                continue;
            }
            let gap = bottom_gap(child);
            if (-1.0..=40.0).contains(&gap) && self.node_is_semantic_nav(&child.id) {
                return Some((child.id.clone(), child.bounds));
            }
        }

        for child in &root.children {
            if !valid(child) {
                continue;
            }
            let gap = bottom_gap(child);
            if (-1.0..=2.0).contains(&gap)
                && child.bounds.size.x >= root.bounds.size.x * 0.9
                && child.bounds.size.y <= 120.0
            {
                return Some((child.id.clone(), child.bounds));
            }
        }
        None
    }

    /// Join a scene id to its runtime schema node to inspect semantics.
    fn node_is_semantic_nav(&self, id: &str) -> bool {
        let Some(document) = self.runtime.document.as_ref() else {
            return false;
        };
        let Some(key) = document.tree.by_id.get(id).copied() else {
            return false;
        };
        let Some(node) = document.tree.nodes.get(key) else {
            return false;
        };
        node_semantics_role_is_nav(&node.schema)
    }

    /// Paint the framed root in a scrolled pass, then paint an optional
    /// pinned nav in its own clipped pass. A focused caret is clipped in
    /// the pass that owns its node and is suppressed outside the root.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_framed(
        &self,
        backend: &mut dyn RenderBackend,
        only_root: &str,
        content_clip: Rect,
        content_origin: Point2D,
        fit: f32,
        pinned: Option<&PinnedPaint>,
        now_ms: u64,
    ) {
        let overlaid;
        let scene = if self.runtime.widget_states.iter().next().is_none()
            && self.binding_sites.is_empty()
        {
            &self.scene
        } else {
            overlaid = self.overlay_runtime_state(&self.scene);
            &overlaid
        };
        let Some(page) = scene.active_page() else {
            return;
        };

        const CULL_MARGIN: f32 = 64.0;
        let cull = Rect {
            origin: Point2D::new(
                content_clip.origin.x - CULL_MARGIN,
                content_clip.origin.y - CULL_MARGIN,
            ),
            size: Point2D::new(
                content_clip.size.x + CULL_MARGIN * 2.0,
                content_clip.size.y + CULL_MARGIN * 2.0,
            ),
        };

        backend.save();
        backend.clip_rect(content_clip);
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            paint_scene_page_with(
                &mut cx,
                page,
                content_origin,
                fit,
                cull,
                PaintSceneOptions {
                    only_root: Some(only_root),
                    skip_node: pinned.map(|paint| paint.node_id.as_str()),
                },
            );
        }
        backend.restore();

        if let Some(paint) = pinned {
            backend.save();
            backend.clip_rect(paint.strip_clip);
            {
                let mut cx = PaintCx {
                    backend: &mut *backend,
                };
                paint_scene_subtree(&mut cx, page, &paint.node_id, paint.paint_origin, fit);
            }
            backend.restore();
        }

        let focused = self.focused_schema_id();
        let in_framed_root = focused.as_ref().is_some_and(|focused_id| {
            page.find(only_root)
                .is_some_and(|root| subtree_contains(root, focused_id))
        });
        if !in_framed_root {
            return;
        }

        let pinned_member = match (pinned, focused) {
            (Some(paint), Some(ref focused_id)) => page
                .find(&paint.node_id)
                .is_some_and(|nav| subtree_contains(nav, focused_id)),
            _ => false,
        };
        backend.save();
        if pinned_member {
            let paint = pinned.expect("membership implies pinned paint");
            backend.clip_rect(paint.strip_clip);
            let origin = Point2D::new(
                paint.paint_origin.x - paint.nav_scene_origin.x * fit,
                paint.paint_origin.y - paint.nav_scene_origin.y * fit,
            );
            self.paint_focus_caret(backend, scene, origin, fit, now_ms);
        } else {
            backend.clip_rect(content_clip);
            self.paint_focus_caret(backend, scene, content_origin, fit, now_ms);
        }
        backend.restore();
    }
}

fn subtree_contains(node: &SceneNode, id: &str) -> bool {
    node.id == id
        || node
            .children
            .iter()
            .any(|child| subtree_contains(child, id))
}

fn node_semantics_role_is_nav(node: &jian_ops_schema::node::PenNode) -> bool {
    use jian_ops_schema::semantics::SemanticRole;

    node.gestures_and_semantics()
        .1
        .is_some_and(|semantics| semantics.role == Some(SemanticRole::Nav))
}

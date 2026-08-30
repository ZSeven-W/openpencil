//! Device-frame presentation for [`super::PreviewSession`]: framed-root
//! selection, bottom-nav detection, and the two-pass framed paint.
//!
//! Sibling of `input.rs` / `app_mode.rs` so `preview/mod.rs` stays under
//! the 800-line cap.

use super::PreviewSession;

use op_editor_core::PenNodeExt;
use op_editor_ui::layout_scene::SceneNode;
use op_editor_ui::widgets::{
    paint_scene_page_with, paint_scene_subtree, PaintCx, PaintSceneOptions,
};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend};

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

    /// Scene-space rect of one top-level root by schema id.
    ///
    /// The slideshow presents a board the user picked with the arrow keys
    /// rather than whichever root happens to be first, so it needs to ask
    /// for a specific one. Only top-level children are searched: a board is
    /// a page root by definition, and matching a nested node would frame
    /// something inside a slide.
    pub fn root_scene_rect(&self, node_id: &str) -> Option<Rect> {
        self.scene
            .active_page()?
            .children
            .iter()
            .find(|node| node.id == node_id)
            .map(|node| node.bounds)
    }

    /// The framed root's own resolved background fill, if any. The
    /// device frame's fixed silhouette (`frame_size` in
    /// `widget_host::preview_frame`) doesn't always match the screen's
    /// authored width (an iPhone-SE-width design against the shared
    /// 390 px chrome, say), so the bezel shows a thin strip on each
    /// side of the centred content. Lining that strip with the
    /// screen's OWN background (falling back to the host's theme when
    /// the root has no fill at all — transparent artboards) makes the
    /// letterbox blend with the design instead of reading as a stray
    /// white seam against a dark theme.
    pub fn framed_root_fill(&self) -> Option<Color> {
        let page = self.scene.active_page()?;
        page.children.first()?.fill
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

        // `role="bottom-tab-bar"` is OpenPencil's canonical authored
        // contract. Once a direct child carries it, App Mode owns its
        // placement: pin it even when a short/overflowing flex layout left
        // the authored y away from the root's bottom edge.
        for child in &root.children {
            if valid(child) && self.node_has_bottom_tab_role(&child.id) {
                return Some((child.id.clone(), child.bounds));
            }
        }

        for child in &root.children {
            if !valid(child) {
                continue;
            }
            let gap = bottom_gap(child);
            if (-1.0..=40.0).contains(&gap) && self.node_has_nav_semantics(&child.id) {
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

    /// Detect a pinnable top status-bar surface among the framed root's
    /// direct children — the symmetric counterpart of
    /// [`Self::pinned_nav_candidate`], mirroring its non-semantic (no
    /// established `SemanticRole` for a status bar) heuristic pass but
    /// anchored to the FIRST child + the top edge instead of the last
    /// candidate + the bottom edge: flush-to-top (small positive gap
    /// tolerated), full-width, and short (a status bar is 20-60 px on
    /// every real device profile — a materially tighter ceiling than
    /// the bottom nav's 120 px so an ordinary page header doesn't get
    /// mistaken for one).
    pub fn pinned_status_bar_candidate(&self, phone: bool) -> Option<(String, Rect)> {
        if !phone {
            return None;
        }
        let page = self.scene.active_page()?;
        let root = page.children.first()?;
        let candidate = root.children.first()?;
        let height = candidate.bounds.size.y;
        if candidate.hidden || !height.is_finite() || height <= 0.0 || height > 60.0 {
            return None;
        }
        let top_gap = candidate.bounds.origin.y - root.bounds.origin.y;
        if (-1.0..=2.0).contains(&top_gap) && candidate.bounds.size.x >= root.bounds.size.x * 0.9 {
            return Some((candidate.id.clone(), candidate.bounds));
        }
        None
    }

    fn schema_node(&self, id: &str) -> Option<&jian_ops_schema::node::PenNode> {
        let document = self.runtime.document.as_ref()?;
        let key = document.tree.by_id.get(id).copied()?;
        let node = document.tree.nodes.get(key)?;
        Some(&node.schema)
    }

    fn node_has_bottom_tab_role(&self, id: &str) -> bool {
        self.schema_node(id).is_some_and(node_role_is_bottom_nav)
    }

    fn node_has_nav_semantics(&self, id: &str) -> bool {
        self.schema_node(id).is_some_and(node_semantics_role_is_nav)
    }

    /// Paint the framed root in a scrolled pass, then paint an optional
    /// pinned bottom nav AND an optional pinned top status bar in their
    /// own clipped passes. A focused caret is clipped in the pass that
    /// owns its node and is suppressed outside the root.
    ///
    /// `content_clip` is assumed to already exclude both strips (the
    /// caller — `widget_host::preview_frame::paint_device_frame` —
    /// insets it top and bottom before calling in), so the scrolled
    /// pass 1 never needs to `skip_node` the status bar the way it
    /// does the nav: the clip alone keeps it out of view regardless of
    /// scroll offset, since the clip rect itself never moves.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_framed(
        &self,
        backend: &mut dyn RenderBackend,
        only_root: &str,
        content_clip: Rect,
        content_origin: Point2D,
        fit: f32,
        pinned: Option<&PinnedPaint>,
        pinned_top: Option<&PinnedPaint>,
        now_ms: u64,
    ) {
        let overlaid;
        let scene = if self.runtime.widget_states.iter().next().is_none()
            && self.binding_sites.is_empty()
            && !self.ui_actions.has_visual_state()
            && !self.binding_overlay.has_visual_state()
            && !self.animation.has_visual_state()
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

        if let Some(paint) = pinned_top {
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

        let member_of = |paint: &PinnedPaint| {
            focused.as_ref().is_some_and(|focused_id| {
                page.find(&paint.node_id)
                    .is_some_and(|strip| subtree_contains(strip, focused_id))
            })
        };
        let pinned_member = pinned
            .filter(|paint| member_of(paint))
            .or_else(|| pinned_top.filter(|paint| member_of(paint)));
        backend.save();
        if let Some(paint) = pinned_member {
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

fn node_role_is_bottom_nav(node: &jian_ops_schema::node::PenNode) -> bool {
    node.base().role.as_deref() == Some("bottom-tab-bar")
}

fn node_semantics_role_is_nav(node: &jian_ops_schema::node::PenNode) -> bool {
    use jian_ops_schema::semantics::SemanticRole;

    node.gestures_and_semantics()
        .1
        .is_some_and(|semantics| semantics.role == Some(SemanticRole::Nav))
}

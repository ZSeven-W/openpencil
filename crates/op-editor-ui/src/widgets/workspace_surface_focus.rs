//! Keyboard focus for the desktop generation workspace: the Tab order and
//! the visible focus ring.
//!
//! Same contract as Home's (`home_surface_focus.rs`): the order is the
//! reading order — header, the open quality report's rows, the toolbar,
//! the canvas banners, the deck strip — validated against the real
//! hit-test, so a disabled Play tile, a hidden pager or a scrim-covered
//! control drops out on its own and keyboard activation can be a press
//! at the target's centre.

use super::{StudioPalette, WorkspaceLayout, WorkspaceSurface};
use crate::widgets::studio_focus_ring::paint_focus_ring;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::{WorkspaceHit, WORKSPACE_TOOLBAR_H};

/// Corner radius of the workspace's focus ring.
const RING_RADIUS: f32 = 8.0;

fn centre(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

impl WorkspaceSurface<'_> {
    /// The toolbar's chat toggle (the one control `WorkspaceLayout` does not
    /// carry a rect for).
    fn toggle_rect(layout: &WorkspaceLayout) -> Rect {
        Rect::xywh(
            layout.toolbar.origin.x + 12.0,
            layout.toolbar.origin.y + (WORKSPACE_TOOLBAR_H - 28.0) / 2.0,
            28.0,
            28.0,
        )
    }

    /// Every candidate target with its rect, in Tab order.
    fn focus_candidates(&self, layout: &WorkspaceLayout) -> Vec<(WorkspaceHit, Rect)> {
        let mut order = vec![(WorkspaceHit::Back, layout.back)];
        if let Some(chip) = self.quality_chip(layout) {
            order.push((WorkspaceHit::QualityChip, chip));
        }
        if let Some(panel) = self.quality_panel(layout) {
            for row in &panel.rows {
                if let crate::widgets::QualityRowKind::Remaining { topic, item } = row.kind {
                    order.push((WorkspaceHit::QualityItem { topic, item }, row.rect));
                }
            }
        }
        order.push((WorkspaceHit::Export, layout.export));
        order.push((WorkspaceHit::Professional, layout.professional));
        order.push((WorkspaceHit::ToggleDock, Self::toggle_rect(layout)));
        let views = super::family_views(self.state.family);
        for (view, rect) in views.iter().zip(&layout.view_segments) {
            order.push((WorkspaceHit::View(*view), *rect));
        }
        order.extend(layout.prev.map(|rect| (WorkspaceHit::Prev, rect)));
        order.extend(layout.next.map(|rect| (WorkspaceHit::Next, rect)));
        order.push((WorkspaceHit::ZoomOut, layout.zoom_out));
        order.push((WorkspaceHit::ZoomFit, layout.zoom_fit));
        order.push((WorkspaceHit::ZoomIn, layout.zoom_in));
        if let Some(button) = self.draft_banner_button(layout) {
            order.push((WorkspaceHit::DraftAction, button));
        }
        if let Some((retry, return_edit)) = self.banner_buttons(layout) {
            order.push((WorkspaceHit::Retry, retry));
            order.push((WorkspaceHit::ReturnEdit, return_edit));
        }
        if layout.strip.is_some() {
            order.push((WorkspaceHit::Overview, layout.overview));
            for (slot, rect) in layout.thumbs.iter().enumerate() {
                order.push((WorkspaceHit::Thumb(layout.thumb_first + slot), *rect));
            }
            order.extend(layout.play.map(|rect| (WorkspaceHit::Play, rect)));
        }
        order
    }

    /// The Tab order: each target a press at its centre would reach.
    pub fn focus_order(&self, layout: &WorkspaceLayout) -> Vec<(WorkspaceHit, Rect)> {
        self.focus_candidates(layout)
            .into_iter()
            .filter(|(hit, rect)| {
                rect.size.x > 0.0
                    && rect.size.y > 0.0
                    && self.hit_test_layout(layout, centre(*rect)) == Some(*hit)
            })
            .collect()
    }

    /// Paint the ring around the workspace's keyboard-focused target. The
    /// host calls this last so the ring sits above the canvas, the drawer
    /// and the report panel.
    pub fn paint_focus_ring(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let Some(focus) = self.state.key_focus else {
            return;
        };
        let layout = self.layout(rect.size.x, rect.size.y);
        let Some((_, target)) = self
            .focus_order(&layout)
            .into_iter()
            .find(|(hit, _)| *hit == focus)
        else {
            return;
        };
        let palette = StudioPalette::for_mode(self.ui.effective_theme_mode());
        paint_focus_ring(cx.backend, target, RING_RADIUS, palette.blue);
    }
}

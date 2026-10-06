//! Paint-walk inputs (`PaintNodeOptions`) and outputs (`PaintNodeHits`)
//! for the canvas painter, split out of `canvas_viewport_paint.rs` to
//! keep that spine under the repository's 800-line cap.

use super::RevealSchedule;
use crate::layout_scene::SceneNode;
use crate::widgets::canvas_overlay_transform::OverlayTransform;
use crate::widgets::canvas_viewport::EditCaret;
use crate::{Color, Point2D, Rect};
use std::collections::HashSet;
pub(super) struct PaintNodeOptions<'a, 'generation> {
    pub(super) viewport_origin: Point2D,
    pub(super) zoom: f32,
    pub(super) edit_caret: Option<EditCaret>,
    pub(super) cull: Rect,
    pub(super) reveals: Option<RevealSchedule<'a>>,
    pub(super) hovered: Option<&'a str>,
    pub(super) selected: Option<&'a str>,
    pub(super) pen: Option<&'a str>,
    pub(super) hidden: Option<&'a str>,
    pub(super) now_ms: u64,
    pub(super) generating_descendant_ids: Option<&'generation HashSet<String>>,
    pub(super) generation_accent: Option<Color>,
    /// Queued empty shells: the skeleton shows, but as a quiet wireframe —
    /// only the on-deck shell gets the active radar (see
    /// `canvas_generation_scan::GenerationPaintSets::queued`).
    pub(super) queued_shell_ids: Option<&'generation HashSet<String>>,
    /// True while rendering a deferred mask source. Editor-only animation and
    /// image placeholder art must not contribute coverage to the mask.
    pub(super) mask_source: bool,
    /// Whether the editor-only video play badge should be painted. Preview
    /// hosts render the poster without chrome; the web host adds a real DOM
    /// video element over it instead.
    pub(super) show_video_badge: bool,
    /// The deferred mask root is already being used as a DstIn/luminance
    /// source, so its own node-level blend must not composite again. Exact id
    /// matching suppresses only that root; blended descendants still render.
    pub(super) suppress_node_composite_id: Option<&'a str>,
    /// True while a pan/zoom gesture is active: effect save-layers
    /// (shadow / blur / backdrop) and sub-pixel leaves skip so the
    /// interactive frame stays cheap; the gesture-end repaint restores
    /// full quality (Figma-style interactive degrade).
    pub(super) fast_interaction: bool,
}

#[derive(Default)]
pub struct PaintNodeHits<'a> {
    pub(crate) hover_rect: Option<Rect>,
    /// Root→node transform chain active where the hovered node paints;
    /// empty when `hover_rect` is `None` or the chain is identity.
    pub(crate) hover_transforms: Vec<OverlayTransform>,
    /// Direct visible children of the hovered focus node. Each child
    /// keeps the transform chain active at its own paint site so the
    /// dashed hierarchy hint follows rotated/flipped descendants.
    pub(crate) hover_child_rects: Vec<(Rect, Vec<OverlayTransform>)>,
    pub(crate) selected_node: Option<&'a SceneNode>,
    pub(crate) selected_transforms: Vec<OverlayTransform>,
    pub(crate) pen_node: Option<&'a SceneNode>,
}

impl<'a> PaintNodeHits<'a> {
    pub(super) fn for_node(
        node: &'a SceneNode,
        options: &PaintNodeOptions<'_, '_>,
        transforms: &[OverlayTransform],
        parent_hovered: bool,
    ) -> Self {
        let is_hovered = options.hovered == Some(node.id.as_str());
        let outline_rect = (is_hovered || parent_hovered)
            .then(|| node_outline_rect(node, options))
            .flatten();
        let hover_rect = is_hovered.then_some(outline_rect).flatten();
        let selected_node = (options.selected == Some(node.id.as_str())).then_some(node);
        Self {
            hover_transforms: if hover_rect.is_some() {
                transforms.to_vec()
            } else {
                Vec::new()
            },
            hover_rect,
            hover_child_rects: if parent_hovered {
                outline_rect
                    .map(|rect| vec![(rect, transforms.to_vec())])
                    .unwrap_or_default()
            } else {
                Vec::new()
            },
            selected_transforms: if selected_node.is_some() {
                transforms.to_vec()
            } else {
                Vec::new()
            },
            selected_node,
            pen_node: (options.pen == Some(node.id.as_str())).then_some(node),
        }
    }

    pub(crate) fn merge_missing(&mut self, child: Self) {
        if self.hover_rect.is_none() {
            self.hover_rect = child.hover_rect;
            self.hover_transforms = child.hover_transforms;
        }
        self.hover_child_rects.extend(child.hover_child_rects);
        if self.selected_node.is_none() {
            self.selected_node = child.selected_node;
            self.selected_transforms = child.selected_transforms;
        }
        if self.pen_node.is_none() {
            self.pen_node = child.pen_node;
        }
    }
}

fn node_outline_rect(node: &SceneNode, options: &PaintNodeOptions<'_, '_>) -> Option<Rect> {
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    Some(Rect {
        origin: Point2D::new(
            options.viewport_origin.x + bounds.origin.x * options.zoom,
            options.viewport_origin.y + bounds.origin.y * options.zoom,
        ),
        size: Point2D::new(bounds.size.x * options.zoom, bounds.size.y * options.zoom),
    })
}

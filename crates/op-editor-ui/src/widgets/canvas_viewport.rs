//! `CanvasViewport` — center-canvas widget that renders the editor's
//! resolved render scene as visual primitives.
//!
//! PAINT path: the widget reads editor state (`viewport` / selection /
//! `tool` / pen-draft) from `op_editor_core::EditorState` and the
//! resolved render-node tree from a [`LayoutScene`]. The per-kind node
//! painter lives in the sibling [`super::canvas_viewport_paint`]
//! module to keep this file under the 800-line ceiling.
//!
//! INPUT path: the host-input hit-test helpers
//! [`rotation_corner_at_point`] / [`selection_handle_at_point`] read
//! the layout-resolved [`LayoutScene`] + the editor's selection /
//! viewport state — they serve the hosts' input dispatch, not widget
//! paint.
//!
//! Per-kind paint:
//! - Frame: fill (if any) at `bounds`, optional stroke, then recurse.
//! - Group / Other(_): no own paint, just recurse into children.
//! - Rect / Ellipse / Polygon / Line / Path / Text: per-kind paint.
//! - `Other("icon_font")`: lucide glyph from `text`.
//!
//! Selection overlay (outlines + handles), grid, pen rubber-band and
//! per-anchor Path handles are layered on top of the resolved scene.
//!
//! Handle geometry + input hit-tests live in
//! `canvas_viewport/hit_test.rs`, the constructors and derived label
//! data in `canvas_viewport/builders.rs`, and the drop-indicator /
//! dashed-rect overlay helpers in `canvas_viewport/overlays.rs` — split
//! out so every file stays under the 800-line cap and re-exported here
//! so existing `canvas_viewport::…` paths keep resolving.

use crate::layout_scene::LayoutScene;
use crate::theme::Theme;
use crate::widgets::canvas_frame_labels::FrameLabel;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect};
use jian_core::text_input::TextInputState;
use op_editor_core::Viewport as DocViewport;

#[path = "canvas_viewport/builders.rs"]
mod builders;
#[path = "canvas_viewport/hit_test.rs"]
mod hit_test;
#[path = "canvas_viewport/overlays.rs"]
mod overlays;

#[cfg(test)]
use builders::generating_label_text;
pub use hit_test::{
    arc_handle_positions, path_handle_positions, rotate_point, rotation_corner_at_point,
    selection_handle_at_point, ArcHandle, SelectionHandle, PATH_HANDLE_GHOST_PX,
};
pub(super) use overlays::paint_dashed_rect;
use overlays::paint_drop_indicator;

/// Caret-blink descriptor for the text node currently being edited.
/// `pub` so the sibling `canvas_viewport_paint` module can name it in
/// the public `paint_node` signature.
#[derive(Clone)]
pub struct EditCaret {
    /// The node id (scene-space string) being edited.
    pub editing: String,
    pub input: TextInputState,
    pub now_ms: u64,
    /// Pre-resolved selection wash color (theme isn't reachable in the
    /// `paint_node` walker, so the viewport stashes it here).
    pub selection_color: Color,
}

pub struct CanvasViewport<'a> {
    pub id: WidgetId,
    /// Pan / zoom of the infinite canvas — read from `EditorState`.
    pub(super) viewport: DocViewport,
    /// The resolved render scene — the node tree the painter walks.
    pub(super) scene: &'a LayoutScene,
    /// Anchor-selected node id (scene-space string). Empty = none.
    pub(super) selected: String,
    /// Full selection set (scene-space string ids).
    pub(super) selected_set: Vec<String>,
    /// Pencil-style selected element/count label painted near the overlay.
    pub(super) selection_label: Option<String>,
    /// Active canvas tool — gates the per-anchor Path handles.
    pub(super) tool: op_editor_core::Tool,
    /// Pen-tool draft: the in-progress path id + last cursor doc
    /// coord, used to paint the rubber-band preview.
    pub(super) pen_in_progress: Option<String>,
    /// Ghost of the blank starter frame `(x, y, w, h)` doc-px — painted
    /// after a design prompt clears the real node, until the generated
    /// design's sized root arrives, so the canvas never flashes empty.
    /// User-selected pencil-cursor silhouette (Settings > System).
    pub(super) pencil_cursor_style: op_editor_core::PencilCursorStyle,
    pub(super) starter_ghost: Option<[f32; 4]>,
    pub(super) pen_cursor_doc: Option<Point2D>,
    /// True while the Pen press-drag is minting handles — hides the
    /// rubber band (TS `isDraggingHandle`).
    pub(super) pen_dragging_handle: bool,
    /// Smart-guide alignment lines to paint during a node drag —
    /// doc-space, computed by the host's `align_guides` pass.
    pub(super) active_guides: Vec<op_editor_core::align_guides::AlignmentGuide>,
    /// Drop-target preview during canvas node dragging.
    pub(super) drop_indicator: Option<op_editor_core::editor_ui_state::CanvasDropIndicator>,
    /// True while a selected canvas node is actively being dragged.
    /// Selection chrome is hidden in that state so the dragged
    /// element itself is the only moving visual affordance.
    pub(super) node_drag_active: bool,
    /// True while the selected image fill is in crop editing mode.
    /// The selection outline remains visible, but resize/rotate chrome is
    /// hidden so dragging inside the frame clearly pans the bitmap.
    pub(super) image_crop_edit_active: bool,
    /// Optional floating copy of the dragged node. The base scene can
    /// still be reflowed to preview sibling avoidance while this copy
    /// follows the cursor.
    pub(super) node_drag_overlay: Option<CanvasNodeDragOverlay>,
    /// Text node being edited (scene-space string id) and its shared
    /// draft/caret/selection state.
    pub(super) text_editing: Option<String>,
    pub(super) text_edit_input: TextInputState,
    /// Background fill outside any Frame.
    pub canvas_background: Color,
    /// Authored Figma page backgrounds replace the editor grid.
    pub(super) show_grid: bool,
    pub theme: Theme,
    /// Host ms clock — text-edit caret blink.
    pub now_ms: u64,
    /// Hierarchy focus under the cursor. An unselected focus paints a
    /// solid outline; its direct visible children paint dashed hints.
    pub(super) hovered: Option<String>,
    /// Top-level frame labels, including transient generation state.
    /// Collected from the canonical tree at build time (the scene
    /// carries no node names); painted screen-space above each root
    /// frame (TS `drawFrameLabelColored`).
    pub(super) frame_labels: Vec<FrameLabel>,
    /// Bounded collaboration cursor/selection projection. It is painted
    /// below the local selection overlay, so local edit affordances remain
    /// visually authoritative.
    pub(super) collab_presence: Vec<crate::widgets::canvas_collab_presence::CollabPresencePaint>,
    /// True while a pan/zoom gesture is live — the scene paints in
    /// interactive-degrade mode (effect layers + sub-pixel leaves
    /// skip); the host schedules a full-quality repaint on gesture end.
    pub fast_interaction: bool,
    /// Restrict node culling (and thus the painted content) to this
    /// rect instead of the widget rect. The host's pan cache uses it
    /// to repaint only the strip a scroll refresh exposed.
    pub cull_override: Option<Rect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasNodeDragOverlay {
    pub node_id: String,
    pub target_origin_doc: Point2D,
}

impl<'a> Widget for CanvasViewport<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Host sizes via the paint rect; we just report a default.
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, cx.available_width, cx.available_width.max(400.0)),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
            return;
        }
        cx.backend.save();
        cx.backend.clip_rect(rect);

        // 1. Paint the canvas background INSIDE the clip region.
        cx.backend.fill_rect(rect, self.canvas_background);

        // 2. Dotted grid — canvas-local, scales with pan/zoom.
        let viewport = &self.viewport;
        if self.show_grid {
            super::canvas_viewport_grid::paint_grid(cx, rect, viewport, &self.theme);
        }
        let indicators = op_editor_core::agent_indicators::snapshot_at_if_active(self.now_ms);
        let selection_chrome_visible = !self.node_drag_active;
        let show_handles = selection_chrome_visible
            && !self.image_crop_edit_active
            && self.selected_set.len() == 1;
        let single_selected_id = self.selected_set.first().map(String::as_str);
        let selected_lookup = if show_handles {
            single_selected_id
        } else {
            None
        };
        let hovered_lookup = if selection_chrome_visible {
            self.hovered.as_deref()
        } else {
            None
        };
        let hovered_focus_selected = hovered_lookup
            .is_some_and(|hovered| self.selected_set.iter().any(|selected| selected == hovered));
        // Root labels carry their durable generating/selection colour
        // from construction. Apply the transient hover tint here so
        // `node_drag_active` suppresses it together with every other
        // hierarchy-hover affordance.
        let hovered_frame_labels = hovered_lookup.map(|hovered| {
            self.frame_labels
                .iter()
                .map(|label| {
                    let mut label = label.clone();
                    if label.id == hovered {
                        label.color = self.theme.primary;
                    }
                    label
                })
                .collect::<Vec<_>>()
        });
        let frame_labels = hovered_frame_labels
            .as_deref()
            .unwrap_or(&self.frame_labels);
        let selected_root_frame_label = selection_chrome_visible
            && show_handles
            && self.scene.active_page().is_some_and(|page| {
                single_selected_id.is_some_and(|id| page.children.iter().any(|node| node.id == id))
            });
        let mut paint_hits = super::canvas_viewport_paint::PaintNodeHits::default();

        // 3. Walk the active page; clip enforces widget bounds.
        if let Some(page) = self.scene.active_page() {
            let viewport_origin = Point2D::new(
                rect.origin.x + viewport.pan_x,
                rect.origin.y + viewport.pan_y,
            );
            let edit_caret = self.text_editing.as_ref().map(|id| EditCaret {
                editing: id.clone(),
                input: self.text_edit_input.clone(),
                now_ms: self.now_ms,
                selection_color: crate::widgets::text_selection::selection_color(&self.theme),
            });
            const CULL_MARGIN: f32 = 64.0;
            let cull = self.cull_override.unwrap_or(Rect {
                origin: Point2D::new(rect.origin.x - CULL_MARGIN, rect.origin.y - CULL_MARGIN),
                size: Point2D::new(
                    rect.size.x + CULL_MARGIN * 2.0,
                    rect.size.y + CULL_MARGIN * 2.0,
                ),
            });
            let reveal_schedule = indicators
                .as_ref()
                .and_then(|indicators| reveal_schedule_for_paint(&indicators.reveals, self.now_ms));
            let hidden_drag_node = self
                .node_drag_overlay
                .as_ref()
                .map(|overlay| overlay.node_id.as_str());
            let generation_sets = super::canvas_generation_scan::generating_paint_sets(
                &page.children,
                indicators.as_ref(),
                self.now_ms,
            );
            // Starter ghost: after a design prompt clears the blank starter
            // frame, keep painting its silhouette (white artboard + name
            // label) until the generated design's root lands — the canvas
            // must never flash empty between prompt and first batch.
            if let Some([gx, gy, gw, gh]) = self.starter_ghost {
                let ghost = Rect {
                    origin: Point2D::new(
                        viewport_origin.x + gx * viewport.zoom,
                        viewport_origin.y + gy * viewport.zoom,
                    ),
                    size: Point2D::new(gw * viewport.zoom, gh * viewport.zoom),
                };
                cx.backend.fill_rect(ghost, Color::WHITE);
                cx.backend
                    .stroke_rect(ghost, self.theme.border.with_alpha(0.8), 1.0);
                let mf = self.theme.muted_foreground;
                let label = crate::TextLayout::single_run(
                    "Frame",
                    "system-ui",
                    11.0,
                    jian_core::scene::Color::rgba(
                        (mf.r * 255.0) as u8,
                        (mf.g * 255.0) as u8,
                        (mf.b * 255.0) as u8,
                        255,
                    ),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend
                    .draw_text(&label, Point2D::new(ghost.origin.x, ghost.origin.y - 6.0));
            }
            let child_hits = super::canvas_viewport_paint::paint_scene_nodes_with_options_hiding(
                cx,
                &page.children,
                viewport_origin,
                viewport.zoom,
                edit_caret.clone(),
                cull,
                reveal_schedule,
                hovered_lookup,
                selected_lookup,
                self.pen_in_progress.as_deref(),
                hidden_drag_node,
                self.now_ms,
                generation_sets.as_ref().map(|sets| &sets.scan),
                generation_sets
                    .as_ref()
                    .map(|_| super::canvas_generation_scan::SKELETON_BLUE),
                generation_sets.as_ref().map(|sets| &sets.queued),
                self.fast_interaction,
            );
            paint_hits.merge_missing(child_hits);
            if let Some(overlay) = self.node_drag_overlay.as_ref() {
                if let Some(node) = page.find(overlay.node_id.as_str()) {
                    let mut floating = node.clone();
                    let current = floating.aggregate_bounds();
                    super::canvas_layout_transition::translate_scene_subtree(
                        &mut floating,
                        overlay.target_origin_doc.x - current.origin.x,
                        overlay.target_origin_doc.y - current.origin.y,
                    );
                    let _ = super::canvas_viewport_paint::paint_node_with_options(
                        cx,
                        &floating,
                        viewport_origin,
                        viewport.zoom,
                        None,
                        cull,
                        reveal_schedule,
                        None,
                        None,
                        None,
                    );
                }
            }
            if let Some(indicators) = indicators.as_ref() {
                super::canvas_agent_cursor::paint_agent_cursors(
                    cx,
                    &page.children,
                    viewport_origin,
                    viewport.zoom,
                    self.now_ms,
                    indicators,
                    self.pencil_cursor_style,
                    Point2D::new(
                        rect.origin.x + rect.size.x / 2.0,
                        rect.origin.y + rect.size.y / 2.0,
                    ),
                );
            }
            const HOVER: Color = Color {
                r: 0.231,
                g: 0.51,
                b: 0.965,
                a: 1.0,
            };
            if !hovered_focus_selected {
                if let Some(screen) = paint_hits.hover_rect {
                    // Replay the focus node's root→node flip/rotation
                    // chain so its solid outline lands on the rendered
                    // geometry, not the unrotated doc-space bounds.
                    let hover_transformed = super::canvas_overlay_transform::replay_on_backend(
                        cx,
                        &paint_hits.hover_transforms,
                    );
                    cx.backend.stroke_rect(screen, HOVER, 1.5);
                    if hover_transformed {
                        cx.backend.restore();
                    }
                }
            }
            for (screen, transforms) in &paint_hits.hover_child_rects {
                // A direct child can add its own flip/rotation after
                // the focus node's ancestor chain, so replay each hint
                // independently instead of sharing the focus transform.
                let hover_transformed =
                    super::canvas_overlay_transform::replay_on_backend(cx, transforms);
                paint_dashed_rect(cx, *screen, HOVER, 1.5);
                if hover_transformed {
                    cx.backend.restore();
                }
            }
            super::canvas_frame_labels::paint_frame_labels(
                cx,
                &page.children,
                frame_labels,
                if selected_root_frame_label {
                    &[]
                } else {
                    &self.selected_set
                },
                viewport_origin,
                viewport,
                rect,
            );
            super::canvas_collab_presence::paint(
                cx,
                &page.children,
                &self.collab_presence,
                rect,
                viewport,
            );
        }

        // 3a. Smart-guide alignment lines (magenta) — painted over the
        // nodes during a node drag, cleared on release.
        if !self.active_guides.is_empty() {
            const GUIDE_COLOR: Color = Color {
                r: 0.93,
                g: 0.12,
                b: 0.55,
                a: 1.0,
            };
            for g in &self.active_guides {
                let (from, to) = if g.vertical {
                    let x = rect.origin.x + viewport.pan_x + (g.pos as f32) * viewport.zoom;
                    let y0 = rect.origin.y + viewport.pan_y + (g.start as f32) * viewport.zoom;
                    let y1 = rect.origin.y + viewport.pan_y + (g.end as f32) * viewport.zoom;
                    (Point2D::new(x, y0), Point2D::new(x, y1))
                } else {
                    let y = rect.origin.y + viewport.pan_y + (g.pos as f32) * viewport.zoom;
                    let x0 = rect.origin.x + viewport.pan_x + (g.start as f32) * viewport.zoom;
                    let x1 = rect.origin.x + viewport.pan_x + (g.end as f32) * viewport.zoom;
                    (Point2D::new(x0, y), Point2D::new(x1, y))
                };
                cx.backend.stroke_line(from, to, GUIDE_COLOR, 1.0);
            }
        }

        // 3b. Drag/drop preview — transient target highlight, ghost
        // bounds, and flex insertion line. It is painted over nodes
        // but below the final selection handles.
        if let Some(indicator) = self.drop_indicator.as_ref() {
            paint_drop_indicator(
                cx,
                rect,
                viewport,
                &self.theme,
                indicator,
                !self.node_drag_active,
            );
        }

        // 4. Selection overlay — outlines + handles (single-select only).
        let active_page = self.scene.active_page();
        let single_selected_node = if show_handles {
            paint_hits.selected_node
        } else {
            None
        };
        let anchor_selected_node = if single_selected_id == Some(self.selected.as_str()) {
            single_selected_node
        } else {
            None
        };
        if selection_chrome_visible {
            if let Some(page) = active_page {
                let selection_input = super::canvas_selection_overlay::SelectionPaintInput {
                    theme: &self.theme,
                    indicators: indicators.as_ref(),
                    now_ms: self.now_ms,
                    canvas_rect: rect,
                    viewport,
                    selection_label: self.selection_label.as_deref(),
                };
                if let Some(node) = single_selected_node {
                    super::canvas_selection_overlay::paint_selected_node(
                        cx,
                        node,
                        &selection_input,
                        show_handles,
                        &paint_hits.selected_transforms,
                    );
                } else if !self.selected_set.is_empty() {
                    super::canvas_selection_overlay::paint_multi_selection_overlays(
                        cx,
                        &page.children,
                        &self.selected_set,
                        &selection_input,
                    );
                }
            }
        }

        // 4b. Pen + path-editing overlays (`canvas_path_overlay.rs`):
        //     - TS `drawPenPreview` while a pen session is authoring
        //       (blue segments + dashed rubber band + anchor dots);
        //     - the ghost-handle edit overlay when the Pen tool has a
        //       single Path selected (pre-existing Rust superset);
        //     - TS `drawPathEditor` for a single selected Path under
        //       any other tool (Select-tool bezier editing, #5).
        super::canvas_path_overlay::paint_path_overlays(
            cx,
            &self.theme,
            self.tool,
            self.pen_in_progress.is_some(),
            paint_hits.pen_node,
            self.pen_cursor_doc,
            self.pen_dragging_handle,
            self.selected_set.len(),
            anchor_selected_node,
            &paint_hits.selected_transforms,
            rect,
            viewport,
        );

        // 4c. Arc-edit handles for a single-selected Ellipse with the
        //     Select tool — start / sweep / inner-radius grab dots.
        if matches!(self.tool, op_editor_core::Tool::Select) && self.selected_set.len() == 1 {
            if let Some(node) = anchor_selected_node {
                if let Some(handles) = arc_handle_positions(node) {
                    let zoom = viewport.zoom;
                    let to_screen = |p: Point2D| {
                        Point2D::new(
                            rect.origin.x + viewport.pan_x + p.x * zoom,
                            rect.origin.y + viewport.pan_y + p.y * zoom,
                        )
                    };
                    // Replay the selected ellipse's transform chain so
                    // arc handles follow rotated/flipped ancestors too.
                    let transformed = super::canvas_overlay_transform::replay_on_backend(
                        cx,
                        &paint_hits.selected_transforms,
                    ) || {
                        let rotated = node.rotation.abs() > f32::EPSILON;
                        if rotated {
                            let b = node.bounds;
                            let pivot = to_screen(Point2D::new(
                                b.origin.x + b.size.x / 2.0,
                                b.origin.y + b.size.y / 2.0,
                            ));
                            cx.backend.save();
                            cx.backend.rotate(node.rotation, pivot);
                        }
                        rotated
                    };
                    let r = 4.5; // screen-px radius
                    for (_, p) in handles {
                        let center = to_screen(p);
                        let bounds = Rect {
                            origin: Point2D::new(center.x - r, center.y - r),
                            size: Point2D::new(r * 2.0, r * 2.0),
                        };
                        // Filled primary dot — distinct from the white
                        // square resize handles.
                        cx.backend.fill_oval(bounds, self.theme.primary);
                        cx.backend.stroke_oval(bounds, self.theme.background, 1.5);
                    }
                    if transformed {
                        cx.backend.restore();
                    }
                }
            }
        }

        cx.backend.restore();
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Canvas);
        node.set_label("Canvas");
        node
    }
}

pub(crate) fn reveal_schedule_for_paint<'a>(
    reveals: &'a std::collections::HashMap<String, u64>,
    now_ms: u64,
) -> Option<super::canvas_viewport_paint::RevealSchedule<'a>> {
    (!reveals.is_empty()).then_some(super::canvas_viewport_paint::RevealSchedule {
        starts: reveals,
        now_ms,
    })
}

#[cfg(test)]
#[path = "canvas_viewport_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "canvas_viewport_from_scene_tests.rs"]
mod from_scene_tests;

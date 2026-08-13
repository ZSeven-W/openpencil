//! `LayerPanel` hit-test + drop-target resolution — split out of
//! `layer_panel.rs` to honor the 800-line file ceiling. These methods
//! mirror the paint geometry (same clipped viewports + row heights) so
//! clicks and drop indicators land on the rows the user sees.

use super::layer_panel::*;
use super::layer_panel_metrics::{
    add_page_target, collapse_target, delete_page_target, layer_action_targets,
};
use super::layer_panel_walkers::row_index_at;
use crate::{Point2D, Rect};

impl LayerPanel {
    /// Drop target for a drag-in-progress. Over a row: top-25 %
    /// Before, middle 50 % Into (container rows only), bottom 25 %
    /// After. Below-rows area: After-last. Outside / above rows:
    /// None.
    pub fn drop_target_at(&self, rect: Rect, point: Point2D) -> Option<DropTarget> {
        if !(rect).contains(point) {
            return None;
        }
        let r = self.regions(rect);
        // A drop is only valid when the cursor is inside the visible
        // (clipped) Layers viewport — a row scrolled out of view, or
        // the cursor over the Pages section / headers, is no drop
        // target. Mirrors the paint clip so the drop-indicator the
        // user sees and where the node actually lands always agree.
        if point.y < r.layers_rows_top || point.y > r.layers_rows_top + r.layers_view_h {
            return None;
        }
        let layers_top = r.layers_rows_top - r.layers.offset;
        if let Some((index, row_top)) = row_index_at(
            self.items.len(),
            r.layers_rows_top,
            r.layers.offset,
            r.layers_view_h,
            self.metrics.layer_row_height,
            point.y,
        ) {
            let item = &self.items[index];
            let row_bottom = row_top + self.metrics.layer_row_height;
            // Container rows use Before / Into / After bands; leaves
            // fall back to a two-way Before / After split.
            let local = point.y - row_top;
            let position = if item.is_container {
                if local < self.metrics.layer_row_height * 0.25 {
                    DropPosition::Before
                } else if local > self.metrics.layer_row_height * 0.75 {
                    DropPosition::After
                } else {
                    DropPosition::Into
                }
            } else if local < self.metrics.layer_row_height / 2.0 {
                DropPosition::Before
            } else {
                DropPosition::After
            };
            let indicator_y = match position {
                DropPosition::Before => row_top,
                DropPosition::After => row_bottom,
                DropPosition::Into => row_top,
            };
            return Some(DropTarget {
                anchor: item.node_id.clone(),
                position,
                indicator_y,
            });
        }
        // Cursor is below the last row but still inside the panel —
        // drop at end (anchor = last layer, position = After). `y`
        // already points at the bottom of the final row, which is
        // exactly where the indicator paints.
        if point.y > layers_top {
            if let Some(last) = self.items.last() {
                let y = layers_top + self.items.len() as f32 * self.metrics.layer_row_height;
                return Some(DropTarget {
                    anchor: last.node_id.clone(),
                    position: DropPosition::After,
                    indicator_y: y,
                });
            }
        }
        None
    }

    /// Hit test a (rect, point) — returns `Page(idx)` for a page
    /// row, `Layer(node_id)` for a layer row, eye/lock/chevron
    /// toggles for the trailing icons, or `AddPage` for the `+`
    /// on the Pages section header.
    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<LayerPanelHit> {
        if !(rect).contains(point) {
            return None;
        }
        // Pages section header — `+` add-page affordance at top-right.
        let r = self.regions(rect);
        if add_page_target(rect, r.pages_header_y, self.metrics).contains(point) {
            return Some(LayerPanelHit::AddPage);
        }
        // Bounded Pages / Layers viewports — a row only counts as a
        // hit when the cursor is inside its (clipped) viewport, so a
        // row scrolled out of view can't be clicked through.
        if let Some((index, y)) = row_index_at(
            self.pages.len(),
            r.pages_rows_top,
            r.pages.offset,
            r.pages_view_h,
            self.metrics.page_row_height,
            point.y,
        ) {
            let page = &self.pages[index];
            let row = Rect {
                origin: Point2D::new(rect.origin.x, y),
                size: Point2D::new(rect.size.x, self.metrics.page_row_height),
            };
            let actions_visible = self.metrics.touch || self.is_page_hovered(page.page_index);
            if row.contains(point)
                && actions_visible
                && delete_page_target(rect, y, self.metrics).contains(point)
            {
                return Some(LayerPanelHit::DeletePage(page.page_index));
            }
            if (row).contains(point) {
                return Some(LayerPanelHit::Page(page.page_index));
            }
        }
        if let Some((index, y)) = row_index_at(
            self.items.len(),
            r.layers_rows_top,
            r.layers.offset,
            r.layers_view_h,
            self.metrics.layer_row_height,
            point.y,
        ) {
            let item = &self.items[index];
            let row = Rect {
                origin: Point2D::new(rect.origin.x, y),
                size: Point2D::new(rect.size.x, self.metrics.layer_row_height),
            };
            if !(row).contains(point) {
                return None;
            }
            // Match the paint geometry — same 14 px icon boxes with
            // 4 px slop so small mouse offsets still register.
            let inner = Rect {
                origin: Point2D::new(row.origin.x + 6.0, y + 2.0),
                size: Point2D::new(row.size.x - 12.0, self.metrics.layer_row_height - 4.0),
            };
            let (eye_target, lock_target) = layer_action_targets(inner, self.metrics);
            let row_hovered = self.is_row_hovered(&item.node_id);
            let actions_visible = self.metrics.touch || row_hovered;
            if actions_visible && !item.renaming && lock_target.contains(point) {
                return Some(LayerPanelHit::ToggleLocked(item.node_id.clone()));
            }
            if actions_visible && !item.renaming && eye_target.contains(point) {
                return Some(LayerPanelHit::ToggleHidden(item.node_id.clone()));
            }
            if item.has_children {
                let indent = self.metrics.row_pad_x + item.depth as f32 * 12.0;
                if collapse_target(inner, indent, r.layers.horizontal_offset, self.metrics)
                    .contains(point)
                {
                    return Some(LayerPanelHit::ToggleCollapsed(item.node_id.clone()));
                }
            }
            return Some(LayerPanelHit::Layer(item.node_id.clone()));
        }
        None
    }
}

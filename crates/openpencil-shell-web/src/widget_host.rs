//! Step 2 widget glue — the only file in shell-web allowed to call
//! into `openpencil_shell_core::widgets` / `::document`. All widget
//! logic (state, paint, layout, accesskit) lives in shell-core; this
//! host is a thin paint-loop adapter that takes a `&mut WebBackend`
//! and dispatches to the editor-UI composition (Toolbar at top,
//! LayerPanel left, PropertyPanel right) driven by an owned
//! `Document`.
//!
//! Any function that pulls in `openpencil_shell_core::widgets::*`
//! MUST live in this file (per spec §1.4). The Phase B4 boundary
//! check script greps for the `// glue:` marker on the per-frame
//! dispatch signatures so the invariant survives drift.
//!
//! Step 1b legacy: previously held one of each B2 primitive
//! widget (`TreeWidget` / `PropertyRow` / `Dropdown` / `TextInput`)
//! as a hardcoded sample. Step 2 replaces that with a `Document` +
//! per-frame editor-UI composition. The old primitives still
//! compose internally (LayerPanel uses TreeWidget patterns,
//! PropertyPanel uses PropertyRow, etc.) — see
//! `openpencil_shell_core::widgets::{layer_panel, property_panel,
//! toolbar}` for the views, and `openpencil_shell_core::document`
//! for the model.

use crate::backend::WebBackend;
use openpencil_shell_core::document::Document;
use openpencil_shell_core::widgets::{
    CanvasViewport, LayerPanel, LayoutCx, PaintCx, PropertyPanel, Toolbar, Widget,
};
use openpencil_shell_core::{Point2D, Rect, RenderBackend};

/// Below this viewport width (in CSS px) we paint the Toolbar
/// only and skip both rails — there's not enough horizontal
/// space for a usable LayerPanel + PropertyPanel split. Codex
/// Step 2 R1 CONCERN-3: clamp `rail_w` to non-negative AND skip
/// when too small to be useful.
const MIN_RAIL_WIDTH: f32 = 80.0;

/// The Step 3 editor-UI host. Owns the document model + toolbar
/// tool selection; per-frame builds LayerPanel / PropertyPanel /
/// CanvasViewport from the document. Aux Dropdown / TextInput
/// (Step 1b holdovers) retired — keyboard / IME wiring stays
/// (apply_key / apply_ime are no-ops without a target until Step
/// 4+ wires per-widget focus).
pub struct WidgetHost {
    document: Document,
    /// Toolbar state (active tool index). Lives on the host until
    /// the document model gains a "current tool" field.
    toolbar: Toolbar,
}

impl WidgetHost {
    pub fn new() -> Self {
        Self {
            document: Document::sample(),
            toolbar: Toolbar::default_set(),
        }
    }

    /// Phase C2 IME forwarding stub. Step 3 retired the aux
    /// TextInput target; Step 4+ wires per-widget focus before
    /// IME composition can be routed back to the document.
    pub fn apply_ime(&mut self, _event: &openpencil_shell_core::ImeEvent) { // glue:
    }

    /// Phase C2 keyboard forwarding stub. Step 3 retired the aux
    /// Dropdown target; Step 4+ wires document selection so arrow
    /// keys can navigate the canvas / layer panel.
    pub fn apply_key(&mut self, _event: &openpencil_shell_core::KeyEvent) { // glue:
    }

    /// Dispatches paint to the editor-UI composition. Layout:
    ///   - Toolbar pinned top, full width
    ///   - LayerPanel left rail
    ///   - CanvasViewport center (the actual document render)
    ///   - PropertyPanel right rail
    pub fn paint(&self, backend: &mut WebBackend, viewport_width: f32) { // glue:
        let layout_cx_top = LayoutCx {
            available_width: viewport_width,
            dpi: backend.dpi_scale(),
        };

        // Toolbar at the top of the viewport.
        let toolbar_box = self.toolbar.layout(&layout_cx_top);
        let toolbar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, toolbar_box.rect.size.y),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            self.toolbar.paint(&mut cx, toolbar_rect);
        }

        // Build editor-UI panels from the current document.
        let layer_panel = LayerPanel::from_document(&self.document);
        let property_panel = PropertyPanel::for_selected(&self.document);
        let canvas = CanvasViewport::from_document(&self.document);

        // Rail widths clamped to non-negative + skipped below
        // the minimum usable width (codex Step 2 R1 CONCERN-3).
        let rail_w_raw = ((viewport_width / 4.0) - 8.0).min(240.0);
        let rail_w = rail_w_raw.max(0.0);
        if rail_w < MIN_RAIL_WIDTH {
            return;
        }
        let rail_top_y = toolbar_rect.size.y + 8.0;
        let rail_layout_cx = LayoutCx {
            available_width: rail_w,
            dpi: backend.dpi_scale(),
        };

        // LayerPanel pinned to the left.
        let lp_layout = layer_panel.layout(&rail_layout_cx);
        let lp_rect = Rect {
            origin: Point2D::new(8.0, rail_top_y),
            size: Point2D::new(rail_w, lp_layout.rect.size.y),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            layer_panel.paint(&mut cx, lp_rect);
        }

        // PropertyPanel pinned to the right.
        let pp_layout = property_panel.layout(&rail_layout_cx);
        let pp_rect = Rect {
            origin: Point2D::new(viewport_width - rail_w - 8.0, rail_top_y),
            size: Point2D::new(rail_w, pp_layout.rect.size.y),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            property_panel.paint(&mut cx, pp_rect);
        }

        // CanvasViewport in the middle band — between the two
        // rails, below the toolbar. The canvas takes whatever
        // height the host gives it; in practice the smoke HTML
        // gives 640 px so we paint a 640-px-tall band here too.
        let canvas_x = lp_rect.origin.x + lp_rect.size.x + 8.0;
        let canvas_w = (pp_rect.origin.x - canvas_x - 8.0).max(0.0);
        if canvas_w >= MIN_RAIL_WIDTH {
            let canvas_rect = Rect {
                origin: Point2D::new(canvas_x, rail_top_y),
                size: Point2D::new(canvas_w, 640.0 - rail_top_y),
            };
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            canvas.paint(&mut cx, canvas_rect);
        }
    }
}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}

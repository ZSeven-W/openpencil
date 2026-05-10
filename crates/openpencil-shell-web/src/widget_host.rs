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
    Dropdown, LayerPanel, LayoutCx, PaintCx, PropertyPanel, TextInput, Toolbar, Widget,
};
use openpencil_shell_core::{Point2D, Rect, RenderBackend};

/// Below this viewport width (in CSS px) we paint the Toolbar
/// only and skip both rails — there's not enough horizontal
/// space for a usable LayerPanel + PropertyPanel split. Codex
/// Step 2 R1 CONCERN-3: clamp `rail_w` to non-negative AND skip
/// when too small to be useful.
const MIN_RAIL_WIDTH: f32 = 80.0;

/// The Step 2 editor-UI host. Owns the document model + a sliver of
/// auxiliary widget state (dropdown / text-input) that's not yet
/// part of the document model itself; per-frame builds LayerPanel /
/// PropertyPanel / Toolbar from the document.
pub struct WidgetHost {
    document: Document,
    /// Toolbar state (active tool index). Lives on the host until
    /// the document model gains a "current tool" field.
    toolbar: Toolbar,
    /// Aux widget state for the right-rail dropdown + text input
    /// — these are still sample fixtures (Step 1b inspector
    /// holdovers) until the property panel is fully document-driven
    /// in Step 3+.
    aux_dropdown: Dropdown,
    aux_text_input: TextInput,
}

impl WidgetHost {
    pub fn new() -> Self {
        Self {
            document: Document::sample(),
            toolbar: Toolbar::default_set(),
            aux_dropdown: Dropdown::sample(),
            aux_text_input: TextInput::sample(),
        }
    }

    /// Phase C2 IME forwarding: route a composition event into the
    /// auxiliary text-input widget's state.
    pub fn apply_ime(&mut self, event: &openpencil_shell_core::ImeEvent) { // glue:
        self.aux_text_input.state.apply_ime(event);
    }

    /// Phase C2 keyboard forwarding: arrow / Enter / Escape land on
    /// the auxiliary dropdown.
    pub fn apply_key(&mut self, event: &openpencil_shell_core::KeyEvent) { // glue:
        self.aux_dropdown
            .state
            .apply_key(event, self.aux_dropdown.options.len());
    }

    /// Dispatches paint to the editor-UI composition. The
    /// `// glue:` marker keeps the boundary script happy. Layout:
    ///   - Toolbar pinned to top: `(0..viewport_w, 0..toolbar_h)`
    ///   - LayerPanel left: `(0..left_rail_w, toolbar_h..)`
    ///   - PropertyPanel right: `(viewport_w - right_rail_w..viewport_w, toolbar_h..)`
    ///   - Aux widgets stacked under the property panel for now
    ///     (Step 3 absorbs them into the document-driven property
    ///     section sets).
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

        // Clamp rail_w to a non-negative value (codex Step 2 R1
        // CONCERN-3: a < 32 px viewport made `viewport_width / 2.0
        // - 8.0` go negative, producing negative-size rects). At
        // tiny viewport widths we just skip the rails entirely —
        // there's no usable space to paint into.
        let rail_w_raw = (viewport_width / 2.0 - 8.0).min(240.0);
        let rail_w = rail_w_raw.max(0.0);
        if rail_w < MIN_RAIL_WIDTH {
            // Toolbar already painted; rails are skipped on
            // sub-minimum viewports. Aux widgets also skip — Step
            // 3 may scroll-collapse instead.
            return;
        }
        let rail_top_y = toolbar_rect.size.y + 8.0;
        let layer_panel_layout_cx = LayoutCx {
            available_width: rail_w,
            dpi: backend.dpi_scale(),
        };

        // LayerPanel pinned to the left.
        let lp_layout = layer_panel.layout(&layer_panel_layout_cx);
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
        let pp_layout = property_panel.layout(&layer_panel_layout_cx);
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

        // Aux dropdown + text input under the property panel.
        // Step 1b holdovers; Step 3 absorbs into PropertyPanel
        // sections.
        let aux_top = pp_rect.origin.y + pp_rect.size.y + 12.0;
        let aux_widgets: [&dyn Widget; 2] = [&self.aux_dropdown, &self.aux_text_input];
        let mut y = aux_top;
        for widget in aux_widgets {
            let widget_layout = widget.layout(&layer_panel_layout_cx);
            let rect = Rect {
                origin: Point2D::new(viewport_width - rail_w - 8.0, y),
                size: Point2D::new(rail_w, widget_layout.rect.size.y),
            };
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            widget.paint(&mut cx, rect);
            y += rect.size.y + 12.0;
        }
    }
}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}

//! Inline canvas text-edit pointer + caret-key glue — click-to-place
//! caret, drag-select, and the arrow-key visual-line mapping.
//!
//! Mirrors the chat-input precedent (`AIChatHit::SelectInputText` +
//! `chat_input_selection_drag`), but the canvas text node paints with
//! REAL font metrics, so hit-testing goes through the same
//! `canvas_text_edit::text_edit_layout` + a measure-only
//! `NativeBackend` (`measure_text_weighted` is canvas-free) instead
//! of a fixed-advance approximation. That keeps the placed caret on
//! the same wrapped line / glyph the painter draws.

use super::{TextEditSelectionDragState, WidgetHostNative};
use op_editor_ui::layout_scene::SceneNode;
use op_editor_ui::widgets::canvas_text_edit::{text_edit_layout, TextEditLayout};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

/// Measure-only [`RenderBackend`] facade over a [`crate::NativeBackend`].
/// Every paint primitive is a no-op (there is no canvas outside the
/// paint pass); only the text measurement forwards to the real
/// FontMgr-backed advance, which is what the layout helper needs.
struct MeasureOnly<'a> {
    inner: &'a mut crate::NativeBackend,
}

impl RenderBackend for MeasureOnly<'_> {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        self.inner.measure_text(text, font_size)
    }
    fn measure_text_weighted(&mut self, text: &str, font_size: f32, weight: u16) -> f32 {
        self.inner.measure_text_weighted(text, font_size, weight)
    }
}

/// Inverse-rotate a doc point about the node's bounds centre so hit
/// geometry tracks a rotated text node's painted glyphs. Mirroring
/// (`flip_x` / `flip_y`) is not compensated — flipped text is rare
/// and the caret stays within the node either way.
fn inverse_rotate_doc(p: Point2D, node: &SceneNode) -> Point2D {
    if node.rotation.abs() <= f32::EPSILON {
        return p;
    }
    let b = node.bounds;
    let cx = b.origin.x + b.size.x / 2.0;
    let cy = b.origin.y + b.size.y / 2.0;
    let (s, c) = (-node.rotation).sin_cos();
    Point2D::new(
        c * (p.x - cx) - s * (p.y - cy) + cx,
        s * (p.x - cx) + c * (p.y - cy) + cy,
    )
}

impl WidgetHostNative {
    /// The edited Text node's resolved scene node, cloned out of the
    /// layout scene so no scene borrow survives into the mutators.
    fn text_edit_scene_node(&mut self) -> Option<SceneNode> {
        let id = self
            .editor_state
            .ui
            .text_editing
            .as_ref()?
            .as_str()
            .to_string();
        self.refresh_layout_scene();
        self.layout_scene.active_page()?.find(&id).cloned()
    }

    /// Run `f` against the shared text-edit layout of `node`, lazily
    /// creating (and caching) the measure-only backend. Take/put-back
    /// keeps the borrow checker happy across `&mut self` use inside.
    fn with_text_edit_layout<R>(
        &mut self,
        node: &SceneNode,
        f: impl FnOnce(&TextEditLayout, &mut dyn RenderBackend) -> R,
    ) -> R {
        let mut measure = self
            .text_measure
            .take()
            .unwrap_or_else(|| crate::NativeBackend::with_dpi(1.0));
        let out = {
            let mut backend = MeasureOnly {
                inner: &mut measure,
            };
            let layout = text_edit_layout(&mut backend, node);
            f(&layout, &mut backend)
        };
        self.text_measure = Some(measure);
        out
    }

    /// Convert a screen point to the edited node's un-rotated doc
    /// space (canvas-region origin + viewport pan/zoom + inverse node
    /// rotation).
    fn text_edit_doc_point(&self, x: f32, y: f32, node: &SceneNode) -> Point2D {
        let (cx0, cy0) = self.canvas_origin();
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let doc = self.editor_state.viewport.to_document(canvas_local);
        inverse_rotate_doc(doc, node)
    }

    /// Byte offset a press at screen `(x, y)` places the caret at —
    /// `Some` ONLY when the press is a caret placement: a text-edit
    /// session is active, no modal / floating overlay owns the point,
    /// the point is over the canvas region, and it lands INSIDE the
    /// edited node's bounds. `apply_press` keeps its commit-on-blur
    /// for every other press (TS textarea parity: a click inside the
    /// textarea moves the caret; a click outside blurs + commits).
    pub(in crate::widget_host) fn text_edit_press_offset(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<usize> {
        self.editor_state.ui.text_editing.as_ref()?;
        let eui = &self.editor_state.editor_ui;
        // Modal layers + outside-click-dismiss dropdowns route presses
        // to themselves — never a caret hit while one is open.
        if eui.agent_settings_open
            || eui.export_dialog_open
            || eui.figma_import_open
            || eui.file_menu_open
            || eui.locale_picker_open
            || eui.shape_picker_open
            || eui.layer_context_menu.is_some()
            || self.editor_state.ui.color_picker.is_some()
        {
            return None;
        }
        // An open Git ready-popover is modal too (`apply_press` §0-git).
        let gp = &eui.git_panel;
        if gp.open && (gp.branch_picker_open || gp.overflow_open) {
            return None;
        }
        if !self.over_canvas(x, y, viewport_w, viewport_h)
            || self.over_floating_overlay(x, y, viewport_w, viewport_h)
        {
            return None;
        }
        let node = self.text_edit_scene_node()?;
        let p = self.text_edit_doc_point(x, y, &node);
        let b = node.bounds;
        let inside = p.x >= b.origin.x
            && p.x <= b.origin.x + b.size.x
            && p.y >= b.origin.y
            && p.y <= b.origin.y + b.size.y;
        if !inside {
            return None;
        }
        Some(
            self.with_text_edit_layout(&node, |layout, backend| layout.offset_at_point(backend, p)),
        )
    }

    /// Place the caret from a press (shift extends, mirroring a
    /// shift+click in a textarea) and start the selection drag.
    pub(in crate::widget_host) fn place_text_edit_caret(&mut self, offset: usize) {
        let extend = self.shift_held;
        let _ = self.editor_state.text_edit_set_caret(offset, extend);
        let anchor = self
            .editor_state
            .ui
            .text_edit_selection_anchor
            .unwrap_or(offset);
        self.text_edit_selection_drag = Some(TextEditSelectionDragState { anchor });
        self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
    }

    /// Live drag-select: map the cursor back to a byte offset (lines
    /// clamp vertically, x clamps to line ends — so dragging past the
    /// node keeps selecting to the boundary) and extend the selection
    /// from the press anchor.
    pub(in crate::widget_host) fn apply_text_edit_selection_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(drag) = self.text_edit_selection_drag else {
            return false;
        };
        let Some(node) = self.text_edit_scene_node() else {
            return true;
        };
        let p = self.text_edit_doc_point(x, y, &node);
        let focus =
            self.with_text_edit_layout(&node, |layout, backend| layout.offset_at_point(backend, p));
        let current = (
            self.editor_state.ui.text_edit_selection_anchor,
            self.editor_state.ui.text_edit_caret,
        );
        if self.editor_state.text_edit_select_range(drag.anchor, focus) {
            let next = (
                self.editor_state.ui.text_edit_selection_anchor,
                self.editor_state.ui.text_edit_caret,
            );
            if next != current {
                self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
            }
        }
        true
    }

    /// Byte `(start, end)` ranges of the edited node's painted lines
    /// — the visual-line contract `text_edit_caret_vertical` and
    /// `text_edit_line_edge` consume.
    fn text_edit_line_ranges(&mut self) -> Option<Vec<(usize, usize)>> {
        let node = self.text_edit_scene_node()?;
        Some(self.with_text_edit_layout(&node, |layout, _| layout.line_ranges()))
    }

    /// Left / Right arrow while a text-edit session is active —
    /// returns `false` when none is, so the caller falls through to
    /// the rename / property / nudge handlers.
    pub fn apply_text_edit_caret(&mut self, forward: bool) -> bool {
        if self.editor_state.ui.text_editing.is_none() {
            return false;
        }
        if self
            .editor_state
            .text_edit_caret_horizontal(forward, self.shift_held)
        {
            self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
        }
        // Consumed regardless — an arrow over the inline editor must
        // never fall through to nudging the node.
        true
    }

    /// Up / Down arrow — move the caret by VISUAL line through the
    /// painted wrap (char-column approximation; see
    /// `EditorState::text_edit_caret_vertical`).
    pub fn apply_text_edit_vertical(&mut self, down: bool) -> bool {
        if self.editor_state.ui.text_editing.is_none() {
            return false;
        }
        let ranges = self.text_edit_line_ranges().unwrap_or_default();
        if self
            .editor_state
            .text_edit_caret_vertical(down, self.shift_held, &ranges)
        {
            self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
        }
        true
    }

    /// Cmd+Left / Cmd+Right — jump to the visual line's start / end.
    pub fn apply_text_edit_line_edge(&mut self, forward: bool) -> bool {
        if self.editor_state.ui.text_editing.is_none() {
            return false;
        }
        let ranges = self.text_edit_line_ranges().unwrap_or_default();
        if self
            .editor_state
            .text_edit_line_edge(forward, self.shift_held, &ranges)
        {
            self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
        }
        true
    }
}

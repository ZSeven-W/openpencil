//! Cursor-affordance resolution for `WidgetHostNative`.
//!
//! Split out of `geometry.rs` to honour the 800-line cap. `cursor_hint`
//! is the single place that turns a screen point into the pointer shape
//! the desktop runner installs: it walks the overlay stack top-down
//! (modals swallow first), then the panel resize gutters, then the
//! canvas affordances for the active tool / selection handles. Every
//! canvas-relative branch derives from `canvas_region` per the
//! coordinate invariant.

use super::{cursor_for_handle, CursorHint, WidgetHostNative};
use op_editor_ui::widgets::{
    rotation_corner_at_point, selection_handle_at_point, AIChatHit, AIChatPlaceholder,
    ChatResizeEdge, PromptCenterPanel,
};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// The resize cursor for a chat-panel edge / corner handle.
    fn chat_resize_cursor(edge: ChatResizeEdge) -> CursorHint {
        match edge {
            ChatResizeEdge::E | ChatResizeEdge::W => CursorHint::ResizeEw,
            ChatResizeEdge::N | ChatResizeEdge::S => CursorHint::ResizeNs,
            ChatResizeEdge::Nw | ChatResizeEdge::Se => CursorHint::ResizeNwse,
            ChatResizeEdge::Ne | ChatResizeEdge::Sw => CursorHint::ResizeNesw,
        }
    }

    pub fn cursor_hint(&self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> CursorHint {
        use op_editor_core::Tool;
        // Modal overlays — keep the pointer the OS default.
        if self.editor_state.editor_ui.agent_settings_open
            || self.editor_state.ui.color_picker.is_some()
        {
            return CursorHint::Default;
        }
        // The disabled empty-state Init card shows a not-allowed cursor
        // (TS `cursor-not-allowed`) — checked before the overlay block
        // so it wins over the Git popover's neutral Default.
        if self.over_disabled_init_card(x, y, viewport_w, viewport_h) {
            return CursorHint::NotAllowed;
        }
        if let Some(resize) = self.chat_resize {
            return Self::chat_resize_cursor(resize.edge);
        }
        // The workspace dock's drag handle resizes the dock — an
        // in-flight drag or a hover on the handle both read as EW.
        if self.workspace_dock_drag.is_some() {
            return CursorHint::ResizeEw;
        }
        if self.workspace_dock_resize_hover(x, y, viewport_w, viewport_h) {
            return CursorHint::ResizeEw;
        }
        // Keep an in-flight Variables resize gesture authoritative even if a
        // different overlay opens before the next pointer event.
        if let Some(edge) = self.variables_resize {
            use op_editor_ui::widgets::variables_panel::VariablesResizeEdge;
            return match edge {
                VariablesResizeEdge::Right => CursorHint::ResizeEw,
                VariablesResizeEdge::Bottom => CursorHint::ResizeNs,
                VariablesResizeEdge::Corner => CursorHint::ResizeNwse,
            };
        }
        let point = Point2D::new(x, y);
        // Design-MD and Icon Picker paint above Prompt Center. Their chrome
        // must keep the neutral cursor even when it overlaps a prompt input.
        if self
            .design_md_panel_rect(viewport_w, viewport_h)
            .is_some_and(|rect| rect.contains(point))
            || self
                .icon_picker_panel_rect(viewport_w, viewport_h)
                .is_some_and(|rect| rect.contains(point))
        {
            return CursorHint::Default;
        }
        if let (Some(panel), Some(rect)) = (
            PromptCenterPanel::for_editor(&self.editor_state),
            self.prompt_center_panel_rect(viewport_w, viewport_h),
        ) {
            if panel.text_input_at(rect, point) {
                return CursorHint::Text;
            }
        }
        // Image popovers paint above Chat. Their editor gets an I-beam; the
        // rest of the popup stays neutral before the model picker applies its
        // modal cursor gate.
        let image_panel = &self.editor_state.editor_ui.image_panel;
        if image_panel.search_open || image_panel.generate_open {
            if let Some(panel) =
                op_editor_ui::widgets::PropertyPanel::for_selection(&self.editor_state)
            {
                let property_rect = self.property_rect(viewport_w, viewport_h);
                if panel.image_popover_input_at(property_rect, point).is_some() {
                    return CursorHint::Text;
                }
                if panel.image_popovers_contain(property_rect, point) {
                    return CursorHint::Default;
                }
            }
        }
        // The model picker is modal below the overlays handled above. Stop
        // before probing Chat/Variables resize and transcript geometry: those
        // controls are visually covered or intentionally inactive, and each
        // probe used to rebuild AIChatPlaceholder on every raw mouse move.
        if self.editor_state.editor_ui.chat_model_picker.open {
            return CursorHint::Default;
        }

        // Build Chat once for both resize and transcript cursor probes.
        let chat_panel = self.ai_chat_rect(viewport_w, viewport_h).map(|rect| {
            (
                rect,
                AIChatPlaceholder::from_editor(&self.editor_state).owned_by(self.chat_panel_owner),
            )
        });
        if let Some((rect, panel)) = chat_panel.as_ref() {
            if let Some(edge) = panel.resize_edge_at(*rect, Point2D::new(x, y)) {
                return Self::chat_resize_cursor(edge);
            }
        }
        // Floating VariablesPanel resize affordances (TS ew/ns/nwse cursor
        // strips). The panel is below Chat/model-picker in paint order.
        if let Some(edge) = self
            .variables_panel_rect(viewport_w, viewport_h)
            .and_then(|rect| {
                use op_editor_ui::widgets::variables_panel::VariablesPanel;
                VariablesPanel::for_editor(&self.editor_state)
                    .resize_edge_at(rect, Point2D::new(x, y))
            })
        {
            use op_editor_ui::widgets::variables_panel::VariablesResizeEdge;
            return match edge {
                VariablesResizeEdge::Right => CursorHint::ResizeEw,
                VariablesResizeEdge::Bottom => CursorHint::ResizeNs,
                VariablesResizeEdge::Corner => CursorHint::ResizeNwse,
            };
        }
        // Cursor-shape probe against the LAST BUILT (= last painted) transcript
        // layout: the user points at what is on screen, so a pure geometric hit
        // over the displayed build is the correct question — and it hashes
        // nothing. `hit_test` would re-resolve + re-fingerprint the live
        // transcript here (a second hash for the same physical move);
        // `hit_test_current_build` reuses the stored build instead. The
        // redraw-time `cursor_probe` (the single hash per cursor move)
        // re-resolves the build and self-corrects any staleness on the next
        // painted frame. Before the first paint no build exists and this yields
        // the default arrow, which is acceptable.
        if let Some((chat_rect, panel)) = chat_panel.as_ref() {
            if let Some(AIChatHit::SelectInputText(_) | AIChatHit::SelectTranscriptText(_, _)) =
                panel.hit_test_current_build(*chat_rect, Point2D::new(x, y))
            {
                return CursorHint::Text;
            }
        }
        // Any floating overlay (panels, Git popover, Toolbar /
        // StatusBar / chat, open dropdowns) — a neutral cursor over
        // them, never a canvas action cursor (Move / Crosshair)
        // bleeding through from a node underneath.
        if self.over_floating_overlay(x, y, viewport_w, viewport_h) {
            return CursorHint::Default;
        }
        // The floating Git panel paints on top of the right-rail
        // resize gutter (and in diff mode is wide enough to cover
        // it), so don't show the resize cursor over the panel.
        let over_git_panel = self
            .git_panel_outer_rect(viewport_w, viewport_h)
            .is_some_and(|r| (r).contains(Point2D::new(x, y)));
        if self.is_resizing_panel()
            || (!over_git_panel && self.panel_resize_hover(x, y, viewport_w).is_some())
        {
            return CursorHint::ResizeEw;
        }
        if self.image_crop_drag.is_some() {
            return CursorHint::Grabbing;
        }
        if self.is_dragging_node() {
            return CursorHint::Default;
        }
        if self.rotate_drag.is_some() {
            return CursorHint::Rotate;
        }
        if let Some(handle) = self.handle_drag.map(|d| d.handle) {
            return cursor_for_handle(handle);
        }
        if !self.over_canvas(x, y, viewport_w, viewport_h) {
            return CursorHint::Default;
        }
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let doc_point = self.editor_state.viewport.to_document(canvas_local);
        let zoom = self.editor_state.viewport.zoom;
        let over_canvas_node = self
            .layout_scene
            .node_at_doc_point(doc_point, zoom)
            .is_some();
        match self.editor_state.tool {
            Tool::Hand => CursorHint::Grab,
            // Shapes / Frame / Form-widget tools all place a node on
            // click — a placement crosshair reads the same for each.
            Tool::Rect
            | Tool::Ellipse
            | Tool::Polygon
            | Tool::Line
            | Tool::Pen
            | Tool::Frame
            | Tool::TextInput
            | Tool::TextArea
            | Tool::NumberInput
            | Tool::Select_
            | Tool::RadioGroup
            | Tool::Switch
            | Tool::Checkbox
            | Tool::Slider
            | Tool::Progress
            | Tool::Tabs => {
                if over_canvas_node {
                    CursorHint::Default
                } else {
                    CursorHint::Crosshair
                }
            }
            Tool::Text => CursorHint::Text,
            Tool::Select => {
                if let Some(editing) = self.editor_state.editor_ui.image_crop_editing.as_ref() {
                    let over_editing_node = self
                        .layout_scene
                        .node_path_at_doc_point(doc_point, zoom)
                        .is_some_and(|path| path.iter().any(|id| id == editing.as_str()));
                    if over_editing_node {
                        return CursorHint::Grab;
                    }
                }
                let canvas_rect = Rect {
                    origin: Point2D::new(cx0, cy0),
                    size: Point2D::new(cw, ch),
                };
                let point = Point2D::new(x, y);
                if let Some(handle) = selection_handle_at_point(
                    canvas_rect,
                    &self.layout_scene,
                    &self.editor_state,
                    point,
                ) {
                    return cursor_for_handle(handle);
                }
                if rotation_corner_at_point(
                    canvas_rect,
                    &self.layout_scene,
                    &self.editor_state,
                    point,
                )
                .is_some()
                {
                    return CursorHint::Rotate;
                }
                if over_canvas_node {
                    return CursorHint::Default;
                }
                CursorHint::Default
            }
        }
    }
}

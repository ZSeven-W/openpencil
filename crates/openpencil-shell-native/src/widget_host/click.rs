//! Primary-button click routing + marquee / layer-drag commit on
//! `WidgetHostNative`. Split out of `keyboard.rs` to honor the
//! 800-line cap.
//!
//! Widget hit-tests run against the derived paint `Document`; their
//! shell-core results (`NodeId` / hit enums) are translated into
//! op-editor-core types before feeding `EditorState` mutators.

use super::helpers::{TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::WidgetHostNative;
use openpencil_shell_core::widgets::{
    AIChatHit, AIChatPlaceholder, LayerPanel, LayoutCx, Toolbar, Widget, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect};

impl WidgetHostNative {
    /// Layer drag release → reorder_before/after/into.
    pub(in crate::widget_host) fn commit_layer_drag(
        &mut self,
        d: super::LayerDragState,
        viewport_h: f32,
    ) -> bool {
        if !d.active {
            // Never moved past threshold — selection on press is the
            // only effect, nothing more to do.
            return false;
        }
        self.refresh_paint_doc();
        if self
            .paint_doc
            .active_page()
            .map(|p| p.find(&d.source).is_none())
            .unwrap_or(true)
        {
            return false;
        }
        use openpencil_shell_core::widgets::{DropPosition, LayerPanel};
        let layer_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.editor_state.editor_ui.layer_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        // Build with source excluded so indicator y matches post-commit.
        let panel =
            LayerPanel::from_document_with_drag_source(&self.paint_doc, d.source.clone());
        let cursor = Point2D::new(d.current_x, d.current_y);
        let Some(drop) = panel.drop_target_at(layer_rect, cursor) else {
            return true;
        };
        if drop.anchor == d.source {
            return true; // self-drop no-op
        }
        let source = op_pen_loader::rev::node_id(&d.source);
        let anchor = op_pen_loader::rev::node_id(&drop.anchor);
        match drop.position {
            DropPosition::Before => {
                self.editor_state.reorder_before(source, anchor);
            }
            DropPosition::After => {
                self.editor_state.reorder_after(source, anchor);
            }
            DropPosition::Into => {
                self.editor_state.reorder_into(source, anchor);
            }
        }
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn commit_marquee_selection(
        &mut self,
        m: super::MarqueeDragState,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        // 2 screen-px marquee threshold (TS `useMarqueeStart`).
        let screen_dx = (m.current_screen_x - m.start_screen_x).abs();
        let screen_dy = (m.current_screen_y - m.start_screen_y).abs();
        if screen_dx < 2.0 && screen_dy < 2.0 {
            return;
        }
        self.refresh_paint_doc();
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let to_doc = |sx: f32, sy: f32| -> Point2D {
            let local = Point2D::new(sx - cx0, sy - cy0);
            self.editor_state.viewport.to_document(local)
        };
        let p0 = to_doc(m.start_screen_x, m.start_screen_y);
        let p1 = to_doc(m.current_screen_x, m.current_screen_y);
        let x = p0.x.min(p1.x);
        let y = p0.y.min(p1.y);
        let w = (p1.x - p0.x).abs();
        let h = (p1.y - p0.y).abs();
        let rect = Rect::xywh(x, y, w, h);
        // `nodes_intersecting_doc_rect` is a `&Document`-bound helper —
        // it returns shell-core `NodeId`s.
        let ids = self.paint_doc.nodes_intersecting_doc_rect(rect);
        if m.additive {
            // ADD-only: every hit joins the set; already-selected
            // hits stay selected (TS shift-marquee parity).
            for id in ids {
                let ec_id = op_pen_loader::rev::node_id(&id);
                if !self.editor_state.is_selected(&ec_id) {
                    self.editor_state.toggle_selection(ec_id);
                }
            }
            self.mark_dirty();
        } else if !ids.is_empty() {
            // Replace with the hit set; anchor = last hit.
            let ec_ids: Vec<op_editor_core::NodeId> =
                ids.iter().map(op_pen_loader::rev::node_id).collect();
            let anchor = ec_ids.last().unwrap().clone();
            self.editor_state.selection.set = ec_ids;
            self.editor_state.selection.anchor = anchor;
            self.mark_dirty();
        }
        // Empty marquee on plain press already cleared at start.
    }

    /// Primary-button click — routes to AI chat / Toolbar / Layer.
    pub fn apply_click(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.refresh_paint_doc();
        // AI chat panel sits above canvas — check first.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel = AIChatPlaceholder::from_document(&self.paint_doc);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                match hit {
                    AIChatHit::FocusInput => {
                        self.editor_state.chat.focused = true;
                        self.editor_state.chat.caret_anchor_ms = self.now_ms;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::Send => {
                        self.editor_state.chat.begin_send();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::Example(text) => {
                        self.editor_state.chat.input = text;
                        self.editor_state.chat.focused = true;
                        self.editor_state.chat.caret_anchor_ms = self.now_ms;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::DragHandle => {
                        // Drag handle handled in apply_press ahead of
                        // this; reaching here is a path bypass.
                        return false;
                    }
                    AIChatHit::ToggleCollapse => {
                        self.editor_state.chat.collapsed =
                            !self.editor_state.chat.collapsed;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ToggleModelPicker => {
                        self.editor_state.editor_ui.chat_model_picker_open =
                            !self.editor_state.editor_ui.chat_model_picker_open;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::SelectModel(idx) => {
                        self.editor_state.select_chat_model(idx);
                        self.mark_dirty();
                        return true;
                    }
                }
            }
        }
        // Click outside chat panel — defocus the input + close the
        // model picker if it was open.
        let picker_was_open = self.editor_state.editor_ui.chat_model_picker_open;
        self.editor_state.editor_ui.chat_model_picker_open = false;
        let was_focused = self.editor_state.chat.focused || picker_was_open;
        self.editor_state.chat.focused = false;
        self.mark_dirty();
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let toolbar = Toolbar::for_document(&self.paint_doc);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
        if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                    self.editor_state.tool = op_pen_loader::rev::tool(tool);
                    self.mark_dirty();
                    return true;
                }
                openpencil_shell_core::widgets::ToolbarHit::Action(_) => return false,
                openpencil_shell_core::widgets::ToolbarHit::ToggleShapePicker => {
                    self.editor_state.editor_ui.shape_picker_open =
                        !self.editor_state.editor_ui.shape_picker_open;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        // Panel hits only when sidebar is open.
        if !self.editor_state.editor_ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.editor_state.editor_ui.layer_panel_width,
                (viewport_height - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        let panel = LayerPanel::from_document(&self.paint_doc);
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            use op_editor_core::ui_draft::LayerContextTarget;
            use openpencil_shell_core::widgets::LayerPanelHit as H;
            // Build the op-editor-core context target for the
            // double-click rename detection.
            let target_for_dbl = match &hit {
                H::Layer(id) => Some(LayerContextTarget::Layer(
                    op_pen_loader::rev::node_id(id),
                )),
                H::Page(idx) => Some(LayerContextTarget::Page(*idx)),
                _ => None,
            };
            if let Some(target) = target_for_dbl {
                if let Some((prev, prev_ms)) =
                    self.editor_state.editor_ui.last_layer_click.clone()
                {
                    if prev == target && self.now_ms.saturating_sub(prev_ms) < 400 {
                        let started = match &target {
                            LayerContextTarget::Layer(id) => {
                                self.editor_state.start_rename_layer(id.clone())
                            }
                            LayerContextTarget::Page(idx) => {
                                self.editor_state.start_rename_page(*idx)
                            }
                        };
                        if started {
                            self.editor_state.editor_ui.rename_caret_anchor_ms =
                                self.now_ms;
                        }
                        self.editor_state.editor_ui.last_layer_click = None;
                        self.mark_dirty();
                        return true;
                    }
                }
                self.editor_state.editor_ui.last_layer_click =
                    Some((target, self.now_ms));
            }
            match hit {
                H::Page(idx) => {
                    let _ = self.editor_state.set_active_page(idx);
                    self.editor_state.clear_selection();
                    self.mark_dirty();
                    return true;
                }
                H::Layer(node_id) => {
                    let ec_id = op_pen_loader::rev::node_id(&node_id);
                    if self.shift_held {
                        self.editor_state.toggle_selection(ec_id);
                    } else {
                        self.editor_state.set_single_selection(ec_id);
                    }
                    self.mark_dirty();
                    return true;
                }
                H::ToggleHidden(node_id) => {
                    self.editor_state
                        .toggle_node_hidden(&op_pen_loader::rev::node_id(&node_id));
                    self.mark_dirty();
                    return true;
                }
                H::ToggleLocked(node_id) => {
                    self.editor_state
                        .toggle_node_locked(&op_pen_loader::rev::node_id(&node_id));
                    self.mark_dirty();
                    return true;
                }
                H::ToggleCollapsed(node_id) => {
                    self.editor_state
                        .toggle_node_collapsed(&op_pen_loader::rev::node_id(&node_id));
                    self.mark_dirty();
                    return true;
                }
                H::AddPage => {
                    let _ = self.editor_state.add_page();
                    self.mark_dirty();
                    return true;
                }
                H::DeletePage(idx) => {
                    let _ = self.editor_state.remove_page(idx);
                    self.mark_dirty();
                    return true;
                }
            }
        }
        // Click hit no chrome — repaint if focus changed.
        was_focused
    }
}

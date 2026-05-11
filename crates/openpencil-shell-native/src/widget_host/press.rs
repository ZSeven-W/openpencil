//! Mouse-press dispatcher on `WidgetHostNative` — the largest
//! single method in the host. Pulled into its own file so the
//! spine `widget_host.rs` stays under the 800-line ceiling.
//!
//! Hit-test order (top-most overlay first):
//!   0aa. Commit-on-blur for property-panel inputs
//!   0z.  Panel-resize gutters (LayerPanel right / PropertyPanel left)
//!   0ab. Shape picker overlay
//!   0a.  Locale picker overlay
//!   0b.  TopBar (sidebar / theme / locale toggle)
//!   0c0. Fill-type picker overlay
//!   0c.  PropertyPanel input row + action button
//!   1.   AI chat panel (DragHandle starts chat drag; other hits
//!        defer to apply_click)
//!   2.   Toolbar (gaps + buttons consume clicks)
//!   3.   apply_click — LayerPanel + chat-defocus
//!   4.   Canvas — Select / shape / Hand branch

use super::helpers::color_to_hex;
use super::helpers::{rect_contains, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::{
    ChatDragState, CreateDragState, DragState, HandleDragState, NodeDragState, PanelResize,
    PanelResizeKind, RotateDragState, WidgetHostNative,
};
use openpencil_shell_core::widgets::{
    rotation_corner_at_point, selection_handle_at_point, AIChatHit, AIChatPlaceholder, LayoutCx,
    LocalePicker, PropertyPanel, ShapeChoice, ShapePicker, Toolbar, TopBar, TopBarHit, Widget,
    TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect};

impl WidgetHostNative {
    /// Mouse-press handler. Returns whether anything visible changed.
    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // 0aa. Commit-on-blur for property-panel inputs — if a
        //      click lands outside the property panel while an
        //      input is focused, commit the draft (parse + write to
        //      node) before processing the new click. The
        //      property-panel hit-test below replaces it with the
        //      new focus when the click was inside the panel.
        if self.document.ui.property_focus.is_some() {
            let property_left = if self.document.property_panel_visible() {
                viewport_width - self.document.ui.property_panel_width
            } else {
                viewport_width
            };
            if x < property_left {
                self.commit_property_focus_if_any();
            }
        }

        // 0z. Panel-resize gutter — clicks within ±4 px of the
        //     LayerPanel right edge or PropertyPanel left edge
        //     start a resize drag. Below TopBar so the gutter
        //     doesn't eat title-bar clicks.
        if y >= TOP_BAR_HEIGHT {
            if let Some(kind) = self.panel_resize_hover(x, y, viewport_width) {
                let start_width = match kind {
                    PanelResizeKind::LayerRight => self.document.ui.layer_panel_width,
                    PanelResizeKind::PropertyLeft => self.document.ui.property_panel_width,
                };
                self.panel_resize = Some(PanelResize {
                    kind,
                    start_x: x,
                    start_width,
                });
                return true;
            }
        }

        // 0ab. Shape picker overlay — same dismissal rules as the
        //      locale picker. Row hit sets the shape tool + closes;
        //      click anywhere else closes silently and swallows
        //      the press so the same click can't re-toggle the
        //      picker via the toolbar shape slot below.
        if self.document.ui.shape_picker_open {
            let panel_rect = self.shape_picker_rect(viewport_width, viewport_height);
            let picker = ShapePicker::for_document(&self.document);
            if let Some(choice) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
                match choice {
                    ShapeChoice::Tool(tool) => {
                        self.document.ui.shape_tool = tool;
                        self.document.tool = tool;
                    }
                    ShapeChoice::OpenIconPicker | ShapeChoice::ImportImageOrSvg => {
                        // Host-side dispatch lands when the icon
                        // picker / file dialog widgets ship.
                    }
                }
                self.document.ui.shape_picker_open = false;
                return true;
            }
            self.document.ui.shape_picker_open = false;
            return true;
        }

        // 0a. Locale picker overlay — when open, it sits on top of
        //     everything. Row click sets locale + closes; ANY
        //     other click (including the Globe button itself, the
        //     canvas, the toolbar, the chip) just closes the
        //     picker. The click is swallowed so the same press
        //     doesn't simultaneously re-toggle the picker open
        //     via the Globe button hit-test below.
        if self.document.ui.locale_picker_open {
            let panel_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_document(&self.document);
            if let Some(locale) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
                self.document.ui.locale = locale;
                self.document.ui.locale_picker_open = false;
                return true;
            }
            self.document.ui.locale_picker_open = false;
            return true;
        }

        // 0b. TopBar — sidebar toggle button + theme + locale picker.
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        let top_bar = TopBar::for_document(&self.document);
        if let Some(hit) = top_bar.hit_test(top_bar_rect, Point2D::new(x, y)) {
            match hit {
                TopBarHit::ToggleSidebar => {
                    self.document.ui.sidebar_open = !self.document.ui.sidebar_open;
                    return true;
                }
                TopBarHit::ToggleTheme => {
                    self.document.ui.theme_mode = self.document.ui.theme_mode.flipped();
                    return true;
                }
                TopBarHit::ToggleLocale => {
                    self.document.ui.locale_picker_open = !self.document.ui.locale_picker_open;
                    return true;
                }
            }
        }
        if rect_contains(top_bar_rect, Point2D::new(x, y)) {
            // Other top-bar gaps eat clicks but don't act.
            return false;
        }

        // 0c0. Fill-type picker: any click that isn't a row /
        //      dropdown toggle dismisses the picker. Same pattern
        //      as locale / shape pickers — overlay swallows ANY
        //      click and closes; row hits also dispatch SetFillType
        //      via the action branch below.
        if self.document.ui.fill_type_picker_open {
            if let Some(panel) = PropertyPanel::for_selection(&self.document) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        viewport_width - self.document.ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.document.ui.property_panel_width,
                        (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                    if matches!(
                        action,
                        openpencil_shell_core::widgets::PropertyPanelAction::SetFillType(_)
                            | openpencil_shell_core::widgets::PropertyPanelAction::ToggleFillTypePicker
                    ) {
                        self.apply_property_action(action);
                        return true;
                    }
                }
            }
            // Anywhere else — close + swallow.
            self.document.ui.fill_type_picker_open = false;
            return true;
        }

        // 0c. PropertyPanel input row — focus the row + seed the
        //     edit draft from the snapshot value. Any other click
        //     (canvas, chat, toolbar, layer panel) commits + clears
        //     the focused input via the catch-all branches below.
        if let Some(panel) = PropertyPanel::for_selection(&self.document) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.document.ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.document.ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            // Button / checkbox click first (flex modes + size flags).
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                self.commit_property_focus_if_any();
                self.apply_property_action(action);
                return true;
            }
            if let Some(focus) = panel.hit_test(property_rect, Point2D::new(x, y)) {
                self.commit_property_focus_if_any();
                let initial = match focus {
                    openpencil_shell_core::document::PropertyFocus::PositionX => {
                        panel.snapshot.x.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::PositionY => {
                        panel.snapshot.y.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::SizeW => {
                        panel.snapshot.width.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::SizeH => {
                        panel.snapshot.height.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::Rotation => {
                        (panel.snapshot.rotation_deg.round() as i32).to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::Opacity => "100".to_string(),
                    openpencil_shell_core::document::PropertyFocus::FillHex => panel
                        .snapshot
                        .fill
                        .map(color_to_hex)
                        .unwrap_or_else(|| "#FFFFFF".to_string()),
                    openpencil_shell_core::document::PropertyFocus::StrokeHex => panel
                        .snapshot
                        .stroke
                        .map(|s| color_to_hex(s.color))
                        .unwrap_or_else(|| "#000000".to_string()),
                    openpencil_shell_core::document::PropertyFocus::StrokeWidth => panel
                        .snapshot
                        .stroke
                        .map(|s| format!("{}", s.width.round() as i32))
                        .unwrap_or_else(|| "1".to_string()),
                    _ => String::new(),
                };
                self.document.ui.property_focus = Some(focus);
                self.document.ui.property_input_draft = initial;
                self.document.ui.property_caret_anchor_ms = self.now_ms;
                // No select-all-on-focus: the user can edit the
                // seeded value character-by-character via
                // backspace + typing. Replacing the whole field
                // means backspacing through it.
                self.document.ui.property_draft_select_all = false;
                self.document.chat.focused = false;
                return true;
            }
        }

        // 1. AI chat panel — sits on top of the toolbar in paint
        //    order, so any click inside its rect is consumed
        //    here. DragHandle starts a chat drag; other AI hits
        //    defer to apply_click for focus/send/example/toggle.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel = AIChatPlaceholder::from_document(&self.document);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                if matches!(hit, AIChatHit::DragHandle) {
                    self.chat_drag = Some(ChatDragState {
                        grab_dx: x - chat_rect.origin.x,
                        grab_dy: y - chat_rect.origin.y,
                        pos_x: chat_rect.origin.x,
                        pos_y: chat_rect.origin.y,
                    });
                    self.document.chat.focused = false;
                    return true;
                }
                // Non-drag chat hit: route through apply_click +
                // short-circuit so we never fall through to the
                // toolbar (which is below the chat panel).
                let _ = self.apply_click(x, y, viewport_width, viewport_height);
                return true;
            }
        }

        // 2. Toolbar — second-highest overlay. A click anywhere
        //    inside its bounding rect is consumed (gaps + padding
        //    too) so it never falls through to the canvas. The
        //    toolbar's x anchor follows `canvas_region`, so when
        //    the sidebar is collapsed it shifts left along with
        //    the canvas (codex stop-hook fix: "collapsed-sidebar
        //    interactions break").
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let toolbar = Toolbar::for_document(&self.document);
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
        if rect_contains(toolbar_rect, Point2D::new(x, y)) {
            if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
                match hit {
                    openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                        self.document.tool = tool;
                        self.document.ui.shape_picker_open = false;
                        return true;
                    }
                    openpencil_shell_core::widgets::ToolbarHit::Action(_) => {
                        self.document.ui.shape_picker_open = false;
                        return false;
                    }
                    openpencil_shell_core::widgets::ToolbarHit::ToggleShapePicker => {
                        self.document.ui.shape_picker_open = !self.document.ui.shape_picker_open;
                        return true;
                    }
                }
            }
            return false;
        }

        // 3. apply_click — LayerPanel + chat-defocus.
        let consumed = self.apply_click(x, y, viewport_width, viewport_height);
        if consumed {
            return true;
        }

        // 4. Canvas click — branch on the active tool.
        //    - Hand: pan-drag.
        //    - Select: handle-drag → node-drag → pan/deselect.
        //    - Shape tools: spawn a new node + drag-to-size.
        if self.over_canvas(x, y, viewport_width, viewport_height) {
            use openpencil_shell_core::document::Tool;
            if matches!(self.document.tool, Tool::Hand) {
                self.drag = Some(DragState {
                    last_x: x,
                    last_y: y,
                });
                return false;
            }
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_width, viewport_height);
            let canvas_rect = Rect {
                origin: Point2D::new(cx0, cy0),
                size: Point2D::new(cw, ch),
            };
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc_point = self.document.viewport.to_document(canvas_local);

            if matches!(self.document.tool, Tool::Select) {
                if let Some(handle) =
                    selection_handle_at_point(canvas_rect, &self.document, Point2D::new(x, y))
                {
                    if let Some(node) = self.document.selected_node() {
                        // Handles are painted from the node's
                        // aggregate bounds (so container handles
                        // surround the child union). For nodes
                        // with real `bounds` (leaves + bounded
                        // Frames) we resize against their own
                        // rect — `set_selected_bounds` writes
                        // straight to `node.bounds`. Unbounded
                        // containers (`Rect::ZERO`) would have
                        // their semantics flipped from "use
                        // children's bounds" to a concrete rect
                        // on the very first resize, and we don't
                        // yet recursively scale descendants — so
                        // skip handle-drag on those for now and
                        // fall through to node-drag / pan.
                        let raw = node.bounds;
                        if raw.size.x > 0.0 || raw.size.y > 0.0 {
                            self.handle_drag = Some(HandleDragState {
                                handle,
                                start_screen_x: x,
                                start_screen_y: y,
                                start_bounds: raw,
                            });
                            return true;
                        }
                    }
                }
                if rotation_corner_at_point(canvas_rect, &self.document, Point2D::new(x, y))
                    .is_some()
                {
                    if let Some(node) = self.document.selected_node() {
                        let bounds = node.aggregate_bounds();
                        let cx_doc = bounds.origin.x + bounds.size.x / 2.0;
                        let cy_doc = bounds.origin.y + bounds.size.y / 2.0;
                        let center_screen_x = canvas_rect.origin.x
                            + self.document.viewport.pan_x
                            + cx_doc * self.document.viewport.zoom;
                        let center_screen_y = canvas_rect.origin.y
                            + self.document.viewport.pan_y
                            + cy_doc * self.document.viewport.zoom;
                        let start_cursor_angle = (y - center_screen_y).atan2(x - center_screen_x);
                        self.rotate_drag = Some(RotateDragState {
                            center_screen_x,
                            center_screen_y,
                            start_cursor_angle,
                            start_rotation: node.rotation,
                        });
                        return true;
                    }
                }
                if let Some(node_id) = self.document.node_at_doc_point(doc_point) {
                    if self.shift_held {
                        // Shift+click toggles set membership.
                        // Only start a node-drag if the click
                        // ADDED the node (so the user can
                        // immediately drag the new selection
                        // together with the existing set).
                        let was_in_set = self.document.is_selected(node_id);
                        self.document.toggle_selection(node_id);
                        if !was_in_set {
                            self.node_drag = Some(NodeDragState {
                                last_screen_x: x,
                                last_screen_y: y,
                            });
                        }
                        return true;
                    }
                    // Plain click: when the click lands on an
                    // already-selected node within a multi-set,
                    // KEEP the set (TS parity — clicking inside
                    // a selection starts a multi-node drag).
                    // Otherwise collapse to single-select.
                    let already_in_set = self.document.is_selected(node_id);
                    if !already_in_set || self.document.selection_count() == 1 {
                        self.document.set_single_selection(node_id);
                    }
                    self.node_drag = Some(NodeDragState {
                        last_screen_x: x,
                        last_screen_y: y,
                    });
                    return true;
                }
                // Empty canvas press. Shift+click on empty does
                // NOT clear the set (TS parity — only plain
                // empty-click deselects).
                // Empty canvas with Select tool — start a marquee.
                // Plain-press clears the existing selection up front
                // so the marquee starts from empty; shift-press
                // preserves the existing set and toggles on release.
                // A zero-area marquee (no drag, just click) cleanly
                // collapses to "deselect-on-click".
                let cleared_now = if !self.shift_held {
                    let was_set = !self.document.selected_set.is_empty();
                    if was_set {
                        self.document.clear_selection();
                    }
                    was_set
                } else {
                    false
                };
                self.marquee_drag = Some(super::MarqueeDragState {
                    start_screen_x: x,
                    start_screen_y: y,
                    current_screen_x: x,
                    current_screen_y: y,
                    additive: self.shift_held,
                });
                return cleared_now;
            }

            // Shape / Frame / Text tool: spawn a new node at the
            // press point and drag-resize via cursor-move.
            if let Some(node_id) = self.create_node_for_active_tool(doc_point) {
                self.document.set_single_selection(node_id);
                self.create_drag = Some(CreateDragState {
                    start_doc_x: doc_point.x,
                    start_doc_y: doc_point.y,
                });
                return true;
            }

            // Tool didn't accept this point — fall back to pan.
            self.drag = Some(DragState {
                last_x: x,
                last_y: y,
            });
            return false;
        }
        false
    }

    /// Spawn a fresh document node for the active shape/frame/text
    /// tool at `doc_point`. Returns the new node's id when the
    /// tool maps to a creatable kind; `None` for Select / Hand.
    pub(in crate::widget_host) fn create_node_for_active_tool(
        &mut self,
        doc_point: Point2D,
    ) -> Option<openpencil_shell_core::document::NodeId> {
        use openpencil_shell_core::document::{Node, NodeId, NodeKind, Tool};
        use openpencil_shell_core::Color;
        let body_fill = Color {
            r: 0.74,
            g: 0.78,
            b: 0.85,
            a: 1.0,
        };
        let (kind, name, fill, stroke): (NodeKind, &str, Option<Color>, Option<(Color, f32)>) =
            match self.document.tool {
                Tool::Rect => (NodeKind::Rect, "Rectangle", Some(body_fill), None),
                Tool::Ellipse => (NodeKind::Ellipse, "Ellipse", Some(body_fill), None),
                Tool::Polygon => (NodeKind::Polygon, "Polygon", Some(body_fill), None),
                Tool::Line => (NodeKind::Line, "Line", None, Some((Color::BLACK, 2.0))),
                Tool::Pen => (NodeKind::Line, "Path", None, Some((Color::BLACK, 2.0))),
                Tool::Frame => (NodeKind::Frame, "Frame", Some(Color::WHITE), None),
                Tool::Text => (NodeKind::Text, "Text", None, None),
                _ => return None,
            };
        // Allocator-collision guard — mirror the duplicate path so
        // a document loaded with ids ≥ next_node_id (or near
        // u64::MAX) can't silently mint a colliding id.
        let safe = self.document.max_node_id().checked_add(1)?;
        self.next_node_id = self.next_node_id.max(safe);
        let id = NodeId::new(self.next_node_id);
        self.next_node_id = self.next_node_id.checked_add(1)?;
        let mut node = Node::leaf(id.raw(), kind, name).with_bounds(Rect::xywh(
            doc_point.x,
            doc_point.y,
            1.0,
            1.0,
        ));
        if let Some(c) = fill {
            node = node.with_fill(c);
        }
        if let Some((sc, sw)) = stroke {
            node = node.with_stroke(sc, sw);
        }
        if matches!(self.document.tool, Tool::Text) {
            node = node.with_text("Text");
        }
        let page = self
            .document
            .pages
            .get_mut(self.document.active_page_index)?;
        page.children.push(node);
        Some(id)
    }
}

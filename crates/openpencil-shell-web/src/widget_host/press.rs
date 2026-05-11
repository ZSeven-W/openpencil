//! Web `apply_press` + `apply_click` — extracted from
//! `widget_host.rs` so the spine stays under the 800-line cap.
//! Mirrors the native `widget_host/press.rs` split.

use openpencil_shell_core::document::Tool;
use openpencil_shell_core::widgets::{
    AIChatHit, AIChatPlaceholder, LayerPanel, LayerPanelHit, LocalePicker, PropertyPanel, Toolbar,
    TopBar, TopBarHit, AI_CHAT_COLLAPSED_HEIGHT, AI_CHAT_HEIGHT, LOCALE_PICKER_WIDTH,
    TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect};

use super::{
    rect_contains, ChatDragState, DragState, LayerDragState, MarqueeDragState, WidgetHost,
};

impl WidgetHost {
    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // 0a. Locale picker overlay — top-most when open. Row hit
        //     sets locale + closes; ANY other hit (including the
        //     Globe button itself) closes the picker AND swallows
        //     the click so the same press doesn't re-toggle open.
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

        // 0b. TopBar — sidebar toggle button. Mirrors the native
        //     host so web + native behave identically.
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
            return false;
        }

        // 0c. PropertyPanel button / checkbox — flex modes + size
        //     flags. Runs AFTER locale picker + TopBar so the
        //     dropdown overlays still win (codex stop-hook fix:
        //     "web property-panel action hit-test intercepts the
        //     locale picker").
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
                self.apply_property_action(action);
                return true;
            }
        }

        // 1. AI chat panel — painted on top of toolbar so a
        //    click inside its rect is consumed here, even when
        //    that point lies inside the toolbar rect underneath.
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
                let _ = self.apply_click(x, y, viewport_width, viewport_height);
                return true;
            }
        }

        // 2. Toolbar — second-highest overlay. Bounding rect
        //    consumes all clicks (gaps + padding too) so it
        //    never falls through to the canvas for tool gaps
        //    that lie outside the chat panel.
        let toolbar_rect = self.toolbar_rect(viewport_width);
        let toolbar = Toolbar::for_document(&self.document);
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
        //    Pre-seed a `layer_drag` candidate when the press lands
        //    on a Layer row so a subsequent move past the threshold
        //    promotes the gesture to a drag-to-reorder (mirrors
        //    native; see `widget_host/press.rs`).
        if self.document.ui.sidebar_open {
            let layer_rect = self.layer_panel_rect(viewport_height);
            let panel = LayerPanel::from_document(&self.document);
            if let Some(LayerPanelHit::Layer(node_id)) =
                panel.hit_test(layer_rect, Point2D::new(x, y))
            {
                self.layer_drag = Some(LayerDragState {
                    source: node_id,
                    start_y: y,
                    current_x: x,
                    current_y: y,
                    active: false,
                });
            }
        }
        if self.apply_click(x, y, viewport_width, viewport_height) {
            return true;
        }

        // 4. Canvas click — branch on tool.
        //    - Hand: pan-drag.
        //    - Select + node hit: set/toggle selection.
        //    - Select + empty: marquee.
        if self.over_canvas(x, y, viewport_width, viewport_height) {
            if matches!(self.document.tool, Tool::Hand) {
                self.drag = Some(DragState {
                    last_x: x,
                    last_y: y,
                });
                return false;
            }
            if matches!(self.document.tool, Tool::Select) {
                // Convert screen → doc to ask which node (if any)
                // is under the cursor.
                let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
                let canvas_local = Point2D::new(x - cx0, y - cy0);
                let doc_point = self.document.viewport.to_document(canvas_local);
                if let Some(node_id) = self.document.node_at_doc_point(doc_point) {
                    if self.shift_held {
                        self.document.toggle_selection(node_id);
                    } else {
                        let already_in_set = self.document.is_selected(node_id);
                        if !already_in_set || self.document.selection_count() == 1 {
                            self.document.set_single_selection(node_id);
                        }
                    }
                    return true;
                }
                // Empty canvas with Select → marquee.
                let cleared_now = if !self.shift_held {
                    let was_set = !self.document.selected_set.is_empty();
                    if was_set {
                        self.document.clear_selection();
                    }
                    was_set
                } else {
                    false
                };
                self.marquee_drag = Some(MarqueeDragState {
                    start_screen_x: x,
                    start_screen_y: y,
                    current_screen_x: x,
                    current_screen_y: y,
                    additive: self.shift_held,
                });
                return cleared_now;
            }
            // Any other tool on empty canvas — fall back to pan
            // (web doesn't ship shape-creation drag yet).
            self.drag = Some(DragState {
                last_x: x,
                last_y: y,
            });
            return false;
        }
        false
    }

    pub fn apply_click(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        // glue:
        // Floating chat panel sits on top — check first so its
        // clicks don't fall through to the canvas.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_w, viewport_h) {
            let panel = AIChatPlaceholder::from_document(&self.document);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                match hit {
                    AIChatHit::FocusInput => {
                        self.document.chat.focused = true;
                        return true;
                    }
                    AIChatHit::Send => {
                        self.document.chat.send();
                        return true;
                    }
                    AIChatHit::Example(text) => {
                        self.document.chat.input = text;
                        self.document.chat.focused = true;
                        return true;
                    }
                    AIChatHit::DragHandle => {
                        return false;
                    }
                    AIChatHit::ToggleCollapse => {
                        self.document.chat.collapsed = !self.document.chat.collapsed;
                        return true;
                    }
                }
            }
        }
        let was_focused = self.document.chat.focused;
        self.document.chat.focused = false;

        let toolbar_rect = self.toolbar_rect(viewport_w);
        let toolbar = Toolbar::for_document(&self.document);
        if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                    self.document.tool = tool;
                    return true;
                }
                openpencil_shell_core::widgets::ToolbarHit::Action(_) => {
                    return false;
                }
                openpencil_shell_core::widgets::ToolbarHit::ToggleShapePicker => {
                    self.document.ui.shape_picker_open = !self.document.ui.shape_picker_open;
                    return true;
                }
            }
        }
        if !self.document.ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = LayerPanel::from_document(&self.document);
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::LayerPanelHit::Page(idx) => {
                    self.document.active_page_index = idx;
                    self.document.clear_selection();
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::Layer(node_id) => {
                    if self.shift_held {
                        self.document.toggle_selection(node_id);
                    } else {
                        self.document.set_single_selection(node_id);
                    }
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::ToggleHidden(node_id) => {
                    self.document.toggle_node_hidden(node_id);
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::ToggleLocked(node_id) => {
                    self.document.toggle_node_locked(node_id);
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::ToggleCollapsed(node_id) => {
                    self.document.toggle_node_collapsed(node_id);
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::AddPage => {
                    let _ = self.document.add_page();
                    return true;
                }
            }
        }
        // Defocusing the chat input itself is a visible change —
        // the caller should still repaint to drop the caret.
        was_focused
    }
}

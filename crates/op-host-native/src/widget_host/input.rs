//! Non-press input handlers on `WidgetHostNative`. press -> press.rs.

use super::helpers::{resize_bounds, PANEL_MAX_WIDTH, PANEL_MIN_WIDTH};
use super::{DragState, PanelResizeKind, WidgetHostNative};
use op_editor_core::codegen::CodeSelection;
use op_editor_ui::widgets::{
    AIChatHit, AIChatPlaceholder, ChatResizeEdge, AI_CHAT_MAX_RATIO, AI_CHAT_MIN_HEIGHT,
    AI_CHAT_MIN_WIDTH,
};
use op_editor_ui::{Point2D, Rect};

/// Minimum cursor travel (logical px) from the node-drag press point
/// before a move is committed. A pure click with sub-pixel jitter then
/// never mutates the document — kills "first click breaks the layout".
const NODE_DRAG_THRESHOLD_PX: f32 = 4.0;
const MAX_SMART_GUIDE_NODES: usize = 1_000;

impl WidgetHostNative {
    /// True iff a text-input surface owns the keyboard.
    pub(in crate::widget_host) fn input_active(&self) -> bool {
        // Preview (Play) mode disables every editor edit shortcut — the
        // canvas belongs to the live runtime, so duplicate / nudge /
        // boolean-op / etc. must all bail.
        if self.preview.is_some() {
            return true;
        }
        let ui = &self.editor_state.ui;
        ui.layer_rename.is_some()
            || ui.text_editing.is_some()
            || ui.property_focus.is_some()
            || self.editor_state.editor_ui.variable_row_focus.is_some()
            || self
                .editor_state
                .editor_ui
                .variables_theme_rename_axis
                .is_some()
            || self
                .editor_state
                .editor_ui
                .variables_variant_rename_value
                .is_some()
            // #20: preset dropdown's save-as-name input.
            || self.editor_state.editor_ui.preset_name_input_active()
            || self.variables_search_active()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
            || self.editor_state.editor_ui.agent_settings.focus.is_some()
            || self.editor_state.editor_ui.icon_picker.open
            || self.editor_state.editor_ui.chat_model_picker.open
            || self.editor_state.editor_ui.component_browser_open
            || self.editor_state.chat.focused
            || self.git_commit_focus_active()
            || self.git_remote_focus_active()
            || self.git_https_focus_active()
            || self.git_branch_create_focus_active()
            || self.git_author_focus_active()
            || self.git_clone_input_active()
    }

    pub fn settings_focus_active(&self) -> bool {
        self.editor_state.editor_ui.agent_settings.focus.is_some()
    }

    /// Whether the variables-panel search input owns the keyboard.
    /// Gated on the panel being open so a stale focus flag can't eat
    /// keystrokes after the panel closes.
    pub fn variables_search_active(&self) -> bool {
        self.editor_state.editor_ui.variables_panel_open
            && self.editor_state.editor_ui.variables_search_focus
    }

    /// Whether the visible Git commit-message input owns the keyboard.
    pub fn git_commit_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        // A stale `commit_focused` must not route keys to a HIDDEN commit box —
        // the box is gone while the branch-picker dropdown OR the signature
        // form (`author_prompt`) has replaced it.
        panel.open
            && panel.commit_focused
            && !panel.loading
            && !panel.branch_picker_open
            && !panel.author_prompt
    }

    /// Whether the visible Git remote-URL input owns the keyboard.
    pub fn git_remote_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.remote_focused && !panel.loading && !panel.branch_picker_open
    }

    /// Whether the visible Git HTTPS-credential input owns the keyboard.
    pub fn git_https_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.https_focused && !panel.loading && !panel.branch_picker_open
    }

    /// Whether the inline create-branch name input owns the keyboard.
    pub fn git_branch_create_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.branch_create_focused && !panel.loading
    }

    /// Whether a commit-signature form input (name / email) owns the keyboard.
    pub fn git_author_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open
            && panel.author_prompt
            && (panel.author_name_focused || panel.author_email_focused)
            && !panel.loading
    }

    /// Whether a ready-state Git popover (branch picker / overflow menu) is
    /// actually visible — the panel is open, in the ready view, and a popover
    /// flag is set. Scopes the Enter swallow so a stale flag while the panel
    /// is closed / loading / merging / showing a diff can't eat global Enter.
    pub fn git_ready_popover_open(&self) -> bool {
        let p = &self.editor_state.editor_ui.git_panel;
        p.open
            && p.in_repo
            && !p.loading
            && !p.merging
            && p.diff.is_none()
            && p.merge_resolve.is_none()
            && (p.branch_picker_open || p.overflow_open)
    }

    /// Whether the inline Git clone wizard is up. While it is, the
    /// wizard owns the keyboard: a focused URL / destination field takes
    /// text, and every other key is swallowed so no canvas shortcut
    /// (tool letters, Delete, arrow nudges, …) leaks to the document
    /// while the user types a URL. View-level (not field-level) because
    /// the wizard covers the panel even between field focuses.
    pub fn git_clone_input_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.clone_form.is_some()
    }

    /// Snap node drags to nearby top-level edge/centre guides.
    fn apply_smart_guides(&mut self) -> (f64, f64) {
        use op_editor_core::{
            aggregate_bounds, align_guides::compute_alignment_guides, PenNodeExt,
        };
        /// Snap range in doc-px.
        const GUIDE_THRESHOLD: f64 = 6.0;

        if self.editor_state.active_children().len() > MAX_SMART_GUIDE_NODES {
            self.editor_state.editor_ui.active_guides.clear();
            return (0.0, 0.0);
        }

        let selected = self.editor_state.selection.anchor.as_str().to_string();
        // Smart guides are a drag-time affordance, so keep them off the
        // layout-scene hot path. The previous version refreshed the whole
        // layout scene on every cursor move; with 100+ nodes that made node
        // movement feel stuck. This mirrors the prior top-level-only behavior
        // but reads the current canonical tree directly after translation.
        let mut moving = None;
        let mut others = Vec::new();
        for node in self.editor_state.active_children() {
            let b = aggregate_bounds(node);
            let aabb = [b.x, b.y, b.w, b.h];
            if node.id_str() == selected {
                moving = Some(aabb);
            } else {
                others.push(aabb);
            }
        }
        let Some(m) = moving else {
            self.editor_state.editor_ui.active_guides.clear();
            return (0.0, 0.0);
        };
        let others: Vec<(f64, f64, f64, f64)> =
            others.iter().map(|a| (a[0], a[1], a[2], a[3])).collect();
        let result = compute_alignment_guides((m[0], m[1], m[2], m[3]), &others, GUIDE_THRESHOLD);
        let snap = (result.snap_dx, result.snap_dy);
        if result.snap_dx != 0.0 || result.snap_dy != 0.0 {
            self.editor_state
                .translate_selected(result.snap_dx, result.snap_dy);
        }
        self.editor_state.editor_ui.active_guides = result.guides;
        snap
    }

    fn apply_node_drag_cursor_move(&mut self, x: f32, y: f32) -> Option<bool> {
        let drag = self.node_drag?;
        if !drag.moved
            && (x - drag.press_screen_x).abs() <= NODE_DRAG_THRESHOLD_PX
            && (y - drag.press_screen_y).abs() <= NODE_DRAG_THRESHOLD_PX
        {
            return Some(false);
        }
        if !drag.moved {
            if let Some(d) = self.node_drag.as_mut() {
                d.moved = true;
            }
        }
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        // Net doc-space travel since the press — the release commit
        // uses it to locate dropped flex children (which never
        // doc-translate during the drag). Recomputed from the press
        // anchor so smart-guide rewinds of `last_screen_*` can't
        // double-count.
        if let Some(d) = self.node_drag.as_mut() {
            d.total_dx = ((x - d.press_screen_x) / zoom) as f64;
            d.total_dy = ((y - d.press_screen_y) / zoom) as f64;
        }
        let prev_screen_x = drag.last_screen_x;
        let prev_screen_y = drag.last_screen_y;
        let dx = (x - prev_screen_x) / zoom;
        let dy = (y - prev_screen_y) / zoom;
        if dx != 0.0 || dy != 0.0 {
            if let Some(drag) = self.node_drag.as_mut() {
                drag.last_screen_x = x;
                drag.last_screen_y = y;
            }
            let translated = self.editor_state.translate_selected(dx as f64, dy as f64);
            let (snap_dx, snap_dy) = if translated {
                self.apply_smart_guides()
            } else {
                self.editor_state.editor_ui.active_guides.clear();
                (0.0, 0.0)
            };
            let scene_dx = dx as f64 + snap_dx;
            let scene_dy = dy as f64 + snap_dy;
            if translated && !self.editor_state_dirty {
                let children = self.editor_state.active_children();
                let ids: Vec<String> = self
                    .editor_state
                    .selection
                    .set
                    .iter()
                    .filter(|id| !op_editor_core::walkers::is_flow_child_of_flex(children, id))
                    .map(|id| id.as_str().to_string())
                    .collect();
                let _ = self
                    .layout_scene
                    .translate_nodes(&ids, scene_dx as f32, scene_dy as f32);
            } else if translated {
                self.mark_dirty();
            }
            if let Some(drag) = self.node_drag.as_mut() {
                if snap_dx != 0.0 {
                    drag.last_screen_x = prev_screen_x;
                }
                if snap_dy != 0.0 {
                    drag.last_screen_y = prev_screen_y;
                }
            }
            return Some(true);
        }
        Some(false)
    }

    pub(in crate::widget_host) fn code_text_offset_at_screen(
        &self,
        x: f32,
        y: f32,
    ) -> Option<usize> {
        if !self.editor_state.property_panel_visible()
            || !matches!(
                self.editor_state.editor_ui.property_tab,
                op_editor_core::PropertyTab::Code
            )
        {
            return None;
        }
        let pw = self.editor_state.editor_ui.property_panel_width;
        let panel_x = self.last_viewport_w - pw;
        if x < panel_x || x > self.last_viewport_w {
            return None;
        }
        let panel_rect = Rect {
            origin: Point2D::new(panel_x, op_editor_ui::widgets::TOP_BAR_HEIGHT),
            size: Point2D::new(
                pw,
                (self.last_viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        op_editor_ui::widgets::property_panel_code::code_text_offset_at(
            panel_rect,
            &self.editor_state.codegen,
            Point2D::new(x, y),
        )
    }

    fn apply_code_selection_drag_cursor_move(&mut self, x: f32, y: f32) -> bool {
        let Some(anchor) = self.code_selection_drag.map(|drag| drag.anchor) else {
            return false;
        };
        if let Some(focus) = self.code_text_offset_at_screen(x, y) {
            let next = Some(CodeSelection { anchor, focus });
            if self.editor_state.codegen.code_selection != next {
                self.editor_state.codegen.code_selection = next;
                self.mark_dirty();
            }
        }
        true
    }

    fn chat_transcript_text_offset_at_screen(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let chat_rect = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h)?;
        match AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
            .hit_test(chat_rect, Point2D::new(x, y))
        {
            Some(AIChatHit::SelectTranscriptText(message_index, offset)) => {
                Some((message_index, offset))
            }
            _ => None,
        }
    }

    fn chat_input_text_offset_at_screen(&self, x: f32, y: f32) -> Option<usize> {
        let chat_rect = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h)?;
        match AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
            .hit_test(chat_rect, Point2D::new(x, y))
        {
            Some(AIChatHit::SelectInputText(offset)) => Some(offset),
            _ => None,
        }
    }

    fn apply_chat_input_selection_drag_cursor_move(&mut self, x: f32, y: f32) -> bool {
        let Some(drag) = self.chat_input_selection_drag else {
            return false;
        };
        if let Some(focus) = self.chat_input_text_offset_at_screen(x, y) {
            if self
                .editor_state
                .chat
                .drag_input_selection(drag.anchor, focus, self.now_ms)
            {
                self.editor_state.chat.focused = true;
                self.mark_dirty();
            }
        }
        true
    }

    fn apply_chat_text_selection_drag_cursor_move(&mut self, x: f32, y: f32) -> bool {
        let Some(drag) = self.chat_text_selection_drag else {
            return false;
        };
        if let Some((message_index, focus)) = self.chat_transcript_text_offset_at_screen(x, y) {
            if message_index == drag.message_index {
                let next = Some(op_editor_core::chat::ChatTranscriptSelection {
                    message_index,
                    anchor: drag.anchor,
                    focus,
                });
                if self.editor_state.chat.transcript_selection != next {
                    self.editor_state.chat.transcript_selection = next;
                    self.mark_dirty();
                }
            }
        }
        true
    }

    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        // In-flight VariablesPanel edge resize — owns the cursor.
        if self.variables_resize.is_some()
            && self.apply_variables_panel_resize(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return true;
        }
        if self.editor_state.editor_ui.agent_settings_open && self.update_agent_settings_hover(x, y)
        {
            return true;
        }
        // Modal export dialog — owns the cursor while open. Update its
        // per-button hover wash (format / scale / cancel / export) and
        // swallow the move so lower-layer hovers don't fire beneath the
        // scrim.
        if self.editor_state.editor_ui.export_dialog_open {
            use op_editor_ui::widgets::ExportDialog;
            let dlg = ExportDialog::centered(self.last_viewport_w, self.last_viewport_h);
            let new_hover = dlg
                .hit_test(Point2D::new(x, y))
                .map(op_editor_ui::widgets::editor_state_ext::export_dialog_button);
            let changed = new_hover != self.editor_state.editor_ui.export_dialog_hover;
            if changed {
                self.editor_state.editor_ui.export_dialog_hover = new_hover;
                self.mark_dirty();
            }
            return changed;
        }
        // Modal Figma-import dialog — owns the cursor while open. Hover
        // the close `✕` + the browse drop-zone.
        if self.editor_state.editor_ui.figma_import_open {
            use op_editor_ui::widgets::figma_import::FigmaImportModal;
            let modal = FigmaImportModal::for_editor(&self.editor_state);
            let panel = modal.rect(self.last_viewport_w, self.last_viewport_h);
            let new_hover = op_editor_ui::widgets::editor_state_ext::figma_import_button(
                modal.hit_test(panel, Point2D::new(x, y)),
            );
            let changed = new_hover != self.editor_state.editor_ui.figma_import_hover;
            if changed {
                self.editor_state.editor_ui.figma_import_hover = new_hover;
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(state) = self.editor_state.ui.color_picker.clone() {
            if let Some(kind) = state.drag {
                use op_editor_core::ui_draft::ColorPickerDrag;
                use op_editor_ui::widgets::color_picker::ColorPicker;
                let picker = ColorPicker::for_state(&self.editor_state, state.clone());
                let panel = picker.rect(self.last_viewport_w, self.last_viewport_h);
                let point = Point2D::new(x, y);
                // Instance-write redirect (GAP #10) — picker drags on
                // a Ref anchor route the live colour into descendants.
                let instance_scope = self.editor_state.begin_instance_write_for_anchor();
                match kind {
                    ColorPickerDrag::SvBox => {
                        let (s, v) = picker.sv_at(panel, point);
                        let _ = self.editor_state.color_picker_set_hsv(state.hue, s, v);
                    }
                    ColorPickerDrag::HueSlider => {
                        let h = picker.hue_at(panel, point);
                        let _ = self
                            .editor_state
                            .color_picker_set_hsv(h, state.sat, state.val);
                    }
                }
                if let Some(scope) = instance_scope {
                    self.editor_state.finish_instance_write(scope);
                }
                self.mark_dirty();
                return true;
            }
        }
        // Top-most floating panel drags own cursor movement.
        if let Some(d) = self.design_md_drag {
            self.editor_state.editor_ui.design_md_panel_pos = Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return true;
        }
        // Design-MD panel hover (close / import / export / remove /
        // section headers).
        if self.editor_state.editor_ui.design_md_panel_open {
            use op_editor_ui::widgets::design_md_panel::DesignMdPanel;
            if let Some(panel_rect) =
                self.design_md_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let new_hover = DesignMdPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, Point2D::new(x, y)));
                if new_hover != self.editor_state.editor_ui.design_md_hover {
                    self.editor_state.editor_ui.design_md_hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if let Some(d) = self.component_browser_drag {
            self.editor_state.editor_ui.component_browser_pos =
                Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return true;
        }
        // Component-browser panel hover (close / category pills / cards).
        if self.editor_state.editor_ui.component_browser_open {
            use op_editor_ui::widgets::component_browser_panel::ComponentBrowserPanel;
            if let Some(panel_rect) =
                self.component_browser_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let new_hover = ComponentBrowserPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, Point2D::new(x, y)));
                if new_hover != self.editor_state.editor_ui.component_browser_hover {
                    self.editor_state.editor_ui.component_browser_hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if let Some(d) = self.icon_picker_drag {
            self.editor_state.editor_ui.icon_picker_panel_pos =
                Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return true;
        }
        // Icon-picker panel hover (close / icon rows / load-more).
        if self.editor_state.editor_ui.icon_picker.open {
            use op_editor_ui::widgets::icon_picker_panel::IconPickerPanel;
            if let Some(panel_rect) =
                self.icon_picker_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let new_hover = IconPickerPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, Point2D::new(x, y)));
                if new_hover != self.editor_state.editor_ui.icon_picker.hover {
                    self.editor_state.editor_ui.icon_picker.hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if self.editor_state.editor_ui.variables_panel_open {
            let point = Point2D::new(x, y);
            if let Some(panel_rect) =
                self.variables_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                if (panel_rect).contains(point) {
                    use op_editor_ui::widgets::variables_panel::VariablesPanel;
                    let new_hover = VariablesPanel::for_editor_at(&self.editor_state, self.now_ms)
                        .hover_at(panel_rect, point);
                    let changed = new_hover != self.editor_state.editor_ui.variables_panel_hover;
                    if changed {
                        self.editor_state.editor_ui.variables_panel_hover = new_hover;
                        self.mark_dirty();
                    }
                    return changed;
                }
            }
            if self
                .editor_state
                .editor_ui
                .variables_panel_hover
                .take()
                .is_some()
            {
                self.mark_dirty();
                return true;
            }
        }
        if let Some(field) = self.image_adjustment_drag {
            if let Some(panel) =
                op_editor_ui::widgets::PropertyPanel::for_selection(&self.editor_state)
            {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        op_editor_ui::widgets::TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                if let Some(action) = panel.image_adjustment_drag_action(property_rect, field, x) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        if self.apply_chat_text_selection_drag_cursor_move(x, y) {
            return true;
        }
        if self.apply_chat_input_selection_drag_cursor_move(x, y) {
            return true;
        }
        if self.apply_text_edit_selection_drag_cursor_move(x, y) {
            return true;
        }
        if self.apply_code_selection_drag_cursor_move(x, y) {
            return true;
        }
        if let Some(consumed) = self.apply_node_drag_cursor_move(x, y) {
            return consumed;
        }
        // Pen handle-drag minting + rubber-band (`pen_press.rs`).
        if let Some(consumed) = self.apply_pen_cursor_move(x, y) {
            return consumed;
        }
        // Git empty-state Init-card hover — toggles the disabled hint
        // pill (a no-op repaint=false when the panel is closed / the
        // cursor isn't over that card).
        if self.update_git_panel_empty_hover(x, y) {
            return true;
        }
        // Git ready-view branch-button hover — `hover:bg-accent` wash on
        // the `⎇ <branch> ▾` trigger.
        if self.update_git_panel_ready_hover(x, y) {
            return true;
        }
        // Path-anchor context-menu row hover (`pen_press.rs`).
        if self.update_path_anchor_menu_hover(x, y) {
            return true;
        }
        // Suppress lower-overlay hover while a floating panel is on top.
        let over_topmost =
            self.over_topmost_panel(x, y, self.last_viewport_w, self.last_viewport_h);
        // Fold stale-hover clearing into the final repaint signal.
        let cleared = over_topmost && self.clear_lower_overlay_hover();
        if let Some(state) = self
            .editor_state
            .editor_ui
            .layer_context_menu
            .clone()
            .filter(|_| !over_topmost)
        {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            let new_hover = menu.hovered_row_at(Point2D::new(x, y));
            if new_hover != state.menu.hover {
                let mut next = state;
                next.menu.hover = new_hover;
                self.editor_state.editor_ui.layer_context_menu = Some(next);
                self.mark_dirty();
                return true;
            }
        }
        // File-menu / locale / shape dropdown hover (`geometry.rs`).
        if self.update_dropdown_hover(x, y, over_topmost) {
            return true;
        }
        // Export-section select-popup row hover (no-op when closed).
        if !over_topmost
            && self.update_export_picker_hover(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return true;
        }
        // Padding-mode gear popover row hover (no-op when closed).
        if self.update_padding_mode_popover_hover(x, y) {
            return true;
        }
        // Font-weight dropdown row hover (no-op when closed).
        if self.update_font_weight_picker_hover(x, y) {
            return true;
        }
        // Font-family picker entry hover (no-op when closed).
        if self.update_font_picker_hover(x, y) {
            return true;
        }
        // TopBar window-control cluster — hovering it reveals the
        // close / minimise / maximise glyphs on the 3 dots.
        {
            use op_editor_ui::widgets::{TopBar, TOP_BAR_HEIGHT};
            let tb_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, TOP_BAR_HEIGHT),
            };
            let over = (TopBar::traffic_cluster_rect(tb_rect)).contains(Point2D::new(x, y));
            if over != self.editor_state.editor_ui.topbar_traffic_hover {
                self.editor_state.editor_ui.topbar_traffic_hover = over;
                self.mark_dirty();
                return true;
            }
        }
        // TopBar chrome-button hover wash (sidebar / file-menu / figma /
        // theme / locale / fullscreen / git / agent chip). Reuses the
        // click hit-test so paint + hover can never drift.
        {
            use op_editor_ui::widgets::{TopBar, TOP_BAR_HEIGHT};
            let tb_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, TOP_BAR_HEIGHT),
            };
            let new_hover = TopBar::for_editor_ui(&self.editor_state.editor_ui)
                .hit_test(tb_rect, Point2D::new(x, y))
                .map(op_editor_ui::widgets::editor_state_ext::topbar_button_hover);
            if new_hover != self.editor_state.editor_ui.topbar_button_hover {
                self.editor_state.editor_ui.topbar_button_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        // Open chat model-picker — track the model row under the
        // cursor so the dropdown paints a hover wash. Non-row chrome
        // clears any stale highlight.
        if self.editor_state.editor_ui.chat_model_picker.open && !over_topmost {
            use op_editor_ui::widgets::ai_chat_model_picker::{model_picker_hit, SelectHit};
            use op_editor_ui::widgets::AIChatPlaceholder;
            let picker = self
                .ai_chat_rect(self.last_viewport_w, self.last_viewport_h)
                .and_then(|chat_rect| {
                    AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                        .model_picker_bounds(chat_rect)
                });
            if let Some(picker) = picker {
                let new_hover = match model_picker_hit(
                    &self.editor_state.editor_ui.chat_model_picker,
                    picker,
                    Point2D::new(x, y),
                    &self.editor_state.chat.available_models,
                    self.editor_state.editor_ui.chat_model_picker_input.text(),
                ) {
                    SelectHit::Row(index) => Some(index),
                    SelectHit::Inside | SelectHit::Outside => None,
                };
                if new_hover != self.editor_state.editor_ui.chat_model_picker.hover {
                    self.editor_state.editor_ui.chat_model_picker.hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        // AI chat header buttons (chevron / maximize / new chat) hover.
        // The chat panel is itself the topmost surface, so this is NOT
        // gated on `over_topmost`; hit_test returns None off the panel,
        // which clears any stale header hover.
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .hit_test(chat_rect, Point2D::new(x, y))
                .as_ref()
                .and_then(op_editor_ui::widgets::editor_state_ext::chat_header_hover);
            if new_hover != self.editor_state.editor_ui.chat_header_hover {
                self.editor_state.editor_ui.chat_header_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .footer_hover_at(chat_rect, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.chat_footer_hover {
                self.editor_state.editor_ui.chat_footer_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        } else if self
            .editor_state
            .editor_ui
            .chat_footer_hover
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .example_hover_at(chat_rect, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.chat_example_hover {
                self.editor_state.editor_ui.chat_example_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        } else if self
            .editor_state
            .editor_ui
            .chat_example_hover
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        if self.update_chat_design_hover(x, y, over_topmost) {
            return true;
        }
        if let Some(drag) = self.rotate_drag {
            let cursor_angle = (y - drag.center_screen_y).atan2(x - drag.center_screen_x);
            let new_rotation = drag.start_rotation + (cursor_angle - drag.start_cursor_angle);
            self.editor_state.set_selected_rotation(new_rotation);
            self.mark_dirty();
            return true;
        }
        if let Some(drag) = self.handle_drag {
            let zoom = self.editor_state.viewport.zoom.max(0.0001);
            let dx = (x - drag.start_screen_x) / zoom;
            let dy = (y - drag.start_screen_y) / zoom;
            let new_bounds = resize_bounds(drag.start_bounds, drag.handle, dx, dy);
            self.editor_state
                .set_selected_bounds(rect_to_doc_rect(new_bounds));
            self.mark_dirty();
            return true;
        }
        if let Some(drag) = self.create_drag {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let cur = self.editor_state.viewport.to_document(canvas_local);
            let min_x = drag.start_doc_x.min(cur.x);
            let min_y = drag.start_doc_y.min(cur.y);
            // Text needs room for placeholder glyphs.
            let (min_w, min_h) = match self.editor_state.tool {
                op_editor_core::Tool::Text => (
                    op_editor_core::DEFAULT_TEXT_NODE_WIDTH as f32,
                    op_editor_core::DEFAULT_TEXT_NODE_HEIGHT as f32,
                ),
                _ => (1.0_f32, 1.0_f32),
            };
            let w = (drag.start_doc_x - cur.x).abs().max(min_w);
            let h = (drag.start_doc_y - cur.y).abs().max(min_h);
            let new_bounds = Rect::xywh(min_x, min_y, w, h);
            self.editor_state
                .set_selected_bounds(rect_to_doc_rect(new_bounds));
            self.mark_dirty();
            return true;
        }
        // Path-anchor / handle drag — TS `movePathControl` semantics
        // (`pen_press.rs::apply_path_anchor_drag_move`).
        if self.apply_path_anchor_drag_move(x, y) {
            return true;
        }
        // Ellipse arc-handle drag: recompute arc geometry from the cursor.
        if self.arc_handle_drag.is_some() {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc = self.editor_state.viewport.to_document(canvas_local);
            let (id, handle, start, already_moved) = {
                let d = self.arc_handle_drag.as_ref().unwrap();
                (d.node_id.clone(), d.handle, d.start_doc, d.moved)
            };
            // Do not mutate until the cursor first travels.
            let is_move = (doc.x - start.x).abs() > 0.001 || (doc.y - start.y).abs() > 0.001;
            if is_move || already_moved {
                self.refresh_layout_scene();
                if let Some(cmd) = self.arc_drag_command(&id, handle, doc) {
                    if self.editor_state.apply(cmd) {
                        self.mark_dirty();
                        if let Some(d) = self.arc_handle_drag.as_mut() {
                            d.moved = true;
                        }
                    }
                }
            }
            return true;
        }
        if let Some(m) = self.marquee_drag.as_mut() {
            m.current_screen_x = x;
            m.current_screen_y = y;
            return true;
        }
        if self.layer_drag.is_some() {
            self.refresh_layout_scene();
            let source_id = self.layer_drag.as_ref().unwrap().source.clone();
            let still_present = self
                .layout_scene
                .active_page()
                .map(|p| p.find(source_id.as_str()).is_some())
                .unwrap_or(false);
            if !still_present {
                self.layer_drag = None;
                return true;
            }
            let d = self.layer_drag.as_mut().unwrap();
            d.current_x = x;
            d.current_y = y;
            // Vertical-only activation — horizontal wiggle preserved
            // for selection / eye / lock click-feel.
            if !d.active && (y - d.start_y).abs() > 4.0 {
                d.active = true;
            }
            return true;
        }
        if let Some(resize) = self.panel_resize {
            let dx = x - resize.start_x;
            match resize.kind {
                PanelResizeKind::LayerRight => {
                    let new_w = (resize.start_width + dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
                    self.editor_state.editor_ui.layer_panel_width = new_w;
                }
                PanelResizeKind::PropertyLeft => {
                    let new_w = (resize.start_width - dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
                    self.editor_state.editor_ui.property_panel_width = new_w;
                }
            }
            self.mark_dirty();
            return true;
        }
        if let Some(resize) = self.chat_resize {
            let (cx0, cy0, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let max_w = (cw * AI_CHAT_MAX_RATIO).max(AI_CHAT_MIN_WIDTH);
            let max_h = (ch * AI_CHAT_MAX_RATIO).max(AI_CHAT_MIN_HEIGHT);
            let dx = x - resize.start_x;
            let dy = y - resize.start_y;
            let mut new_w = resize.start_rect.size.x;
            let mut new_h = resize.start_rect.size.y;
            let mut new_left = resize.start_rect.origin.x;
            let mut new_top = resize.start_rect.origin.y;

            if matches!(
                resize.edge,
                ChatResizeEdge::E | ChatResizeEdge::Ne | ChatResizeEdge::Se
            ) {
                new_w = resize.start_rect.size.x + dx;
            }
            if matches!(
                resize.edge,
                ChatResizeEdge::W | ChatResizeEdge::Nw | ChatResizeEdge::Sw
            ) {
                new_w = resize.start_rect.size.x - dx;
                new_left = resize.start_rect.origin.x + dx;
            }
            if matches!(
                resize.edge,
                ChatResizeEdge::S | ChatResizeEdge::Se | ChatResizeEdge::Sw
            ) {
                new_h = resize.start_rect.size.y + dy;
            }
            if matches!(
                resize.edge,
                ChatResizeEdge::N | ChatResizeEdge::Ne | ChatResizeEdge::Nw
            ) {
                new_h = resize.start_rect.size.y - dy;
                new_top = resize.start_rect.origin.y + dy;
            }

            if new_w < AI_CHAT_MIN_WIDTH {
                let diff = AI_CHAT_MIN_WIDTH - new_w;
                new_w = AI_CHAT_MIN_WIDTH;
                if matches!(
                    resize.edge,
                    ChatResizeEdge::W | ChatResizeEdge::Nw | ChatResizeEdge::Sw
                ) {
                    new_left -= diff;
                }
            }
            if new_w > max_w {
                let diff = new_w - max_w;
                new_w = max_w;
                if matches!(
                    resize.edge,
                    ChatResizeEdge::W | ChatResizeEdge::Nw | ChatResizeEdge::Sw
                ) {
                    new_left += diff;
                }
            }
            if new_h < AI_CHAT_MIN_HEIGHT {
                let diff = AI_CHAT_MIN_HEIGHT - new_h;
                new_h = AI_CHAT_MIN_HEIGHT;
                if matches!(
                    resize.edge,
                    ChatResizeEdge::N | ChatResizeEdge::Ne | ChatResizeEdge::Nw
                ) {
                    new_top -= diff;
                }
            }
            if new_h > max_h {
                let diff = new_h - max_h;
                new_h = max_h;
                if matches!(
                    resize.edge,
                    ChatResizeEdge::N | ChatResizeEdge::Ne | ChatResizeEdge::Nw
                ) {
                    new_top += diff;
                }
            }

            let max_left = cx0 + cw - new_w;
            let max_top = cy0 + ch - new_h;
            new_left = new_left.clamp(cx0, max_left.max(cx0));
            new_top = new_top.clamp(cy0, max_top.max(cy0));
            self.editor_state.chat.panel_width = new_w.round();
            self.editor_state.chat.panel_height = new_h.round();
            self.editor_state.chat.panel_position = Some((new_left.round(), new_top.round()));
            self.mark_dirty();
            return true;
        }
        if let Some(d) = self.chat_drag.as_mut() {
            d.pos_x = x - d.grab_dx;
            d.pos_y = y - d.grab_dy;
            return true;
        }
        if let Some(drag) = self.drag.as_mut() {
            let dx = x - drag.last_x;
            let dy = y - drag.last_y;
            drag.last_x = x;
            drag.last_y = y;
            self.editor_state.viewport.pan(dx, dy);
            // Canvas pan only translates the viewport; keep layout cache intact.
            return true;
        }
        // Toolbar hover after drag detection.
        if self.update_toolbar_hover(x, y, over_topmost) {
            return true;
        }
        // StatusBar control hover wash (search / zoom-out / zoom-in).
        // Suppressed under a floating panel so it doesn't light up
        // beneath the chat / a dropdown.
        {
            let new_hover = if over_topmost {
                None
            } else {
                self.status_bar_rect(self.last_viewport_w, self.last_viewport_h)
                    .and_then(|r| {
                        op_editor_ui::widgets::StatusBar::for_editor(&self.editor_state)
                            .control_at(r, Point2D::new(x, y))
                    })
            };
            if new_hover != self.editor_state.editor_ui.statusbar_hover {
                self.editor_state.editor_ui.statusbar_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        // PropertyPanel tab/action hover wash. Shown with a selection.
        let mut property_hover_changed = false;
        if !over_topmost && self.editor_state.property_panel_visible() {
            use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
            if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                let point = Point2D::new(x, y);
                let new_tab_hover = panel.tab_hover_at(property_rect, point);
                if new_tab_hover != self.editor_state.editor_ui.property_tab_hover {
                    self.editor_state.editor_ui.property_tab_hover = new_tab_hover;
                    property_hover_changed = true;
                }
                let new_fill_type_hover = panel.fill_type_picker_row_at(property_rect, point);
                if new_fill_type_hover != self.editor_state.editor_ui.fill_type_picker.hover {
                    self.editor_state.editor_ui.fill_type_picker.hover = new_fill_type_hover;
                    property_hover_changed = true;
                }
                let new_action_hover = panel.action_hover_index(property_rect, point);
                if new_action_hover != self.editor_state.editor_ui.property_action_hover {
                    self.editor_state.editor_ui.property_action_hover = new_action_hover;
                    property_hover_changed = true;
                }
            }
        } else {
            let ui = &mut self.editor_state.editor_ui;
            property_hover_changed |= ui.property_tab_hover.take().is_some();
            property_hover_changed |= ui.fill_type_picker.hover.take().is_some();
        }
        // Align toolbar hover after drag detection.
        let new_hover = if self.editor_state.selection_count() >= 2 && !over_topmost {
            self.align_toolbar_hit(x, y, self.last_viewport_w, self.last_viewport_h)
        } else {
            None
        };
        let new_hover_ec = new_hover;
        if new_hover_ec != self.editor_state.editor_ui.align_toolbar_hover {
            self.editor_state.editor_ui.align_toolbar_hover = new_hover_ec;
            self.mark_dirty();
            return true;
        }
        // Code-panel hover wash. Reuses Code-panel action geometry so
        // framework chips, scroll chevrons, and body buttons share click and
        // hover hit-testing.
        let (new_fw_hover, new_action_hover) = if !over_topmost
            && self.editor_state.property_panel_visible()
            && matches!(
                self.editor_state.editor_ui.property_tab,
                op_editor_core::PropertyTab::Code
            ) {
            use op_editor_ui::widgets::{property_panel_code, TOP_BAR_HEIGHT};
            let pw = self.editor_state.editor_ui.property_panel_width;
            let panel_x = self.last_viewport_w - pw;
            let panel_rect = Rect {
                origin: Point2D::new(panel_x, TOP_BAR_HEIGHT),
                size: Point2D::new(pw, (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0)),
            };
            if x >= panel_x && x <= self.last_viewport_w {
                property_panel_code::code_hover_at_with_locale(
                    panel_rect,
                    &self.editor_state.codegen,
                    Point2D::new(x, y),
                    self.editor_state.editor_ui.locale,
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        if new_fw_hover != self.editor_state.codegen.framework_hover
            || new_action_hover != self.editor_state.codegen.action_hover
        {
            self.editor_state.codegen.framework_hover = new_fw_hover;
            self.editor_state.codegen.action_hover = new_action_hover;
            self.mark_dirty();
            return true;
        }
        if property_hover_changed {
            self.mark_dirty();
            return true;
        }
        // Canvas hover outline (TS `hoveredNodeId`): track the node
        // under the cursor while the Select tool idles over the
        // canvas. Reads the CURRENT layout scene without refreshing
        // (same discipline as layer-row hover — hover must not
        // rebuild a stale scene).
        let hover_eligible = !over_topmost
            && matches!(self.editor_state.tool, op_editor_core::Tool::Select)
            && self.over_canvas(x, y, self.last_viewport_w, self.last_viewport_h);
        let new_canvas_hover = if hover_eligible {
            // Skip the (full-tree) hover hit-test for sub-3px jitter —
            // the outline can't visibly change inside that radius and
            // path-heavy documents pay real cost per walk. The skip
            // only ever bypasses the WALK; leaving the canvas (the
            // else branch) always clears, threshold or not.
            if let Some((hx, hy)) = self.last_hover_probe {
                if (x - hx).abs() < 3.0 && (y - hy).abs() < 3.0 {
                    return cleared;
                }
            }
            self.last_hover_probe = Some((x, y));
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc = self.editor_state.viewport.to_document(canvas_local);
            self.layout_scene
                .node_at_doc_point(doc, self.editor_state.viewport.zoom)
                .map(|id| op_editor_core::NodeId::new(&id))
        } else {
            self.last_hover_probe = None;
            None
        };
        if new_canvas_hover != self.editor_state.editor_ui.canvas_hover_node {
            self.editor_state.editor_ui.canvas_hover_node = new_canvas_hover;
            self.mark_dirty();
            return true;
        }
        // Fold stale-hover clearing into the repaint signal.
        cleared
    }

    /// Mouse-release — ends active drag; chat-panel snaps corner.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        let pressed_released = self.release_pressed_feedback();
        // Pen owns the release while authoring (TS onMouseUp).
        if self.apply_pen_release() {
            return true;
        }
        // Drop color-picker drag.
        if self.editor_state.ui.color_picker.is_some() {
            self.editor_state.color_picker_set_drag(None);
            self.mark_dirty();
        }
        if self.editor_state.editor_ui.agent_settings_drag.is_some() {
            self.editor_state.editor_ui.agent_settings_drag = None;
            self.mark_dirty();
        }
        if self.panel_resize.take().is_some() {
            return true;
        }
        if self.variables_resize.take().is_some() {
            return true;
        }
        if self.chat_resize.take().is_some() {
            return true;
        }
        if self.rotate_drag.take().is_some() {
            return true;
        }
        if self.handle_drag.take().is_some() {
            return true;
        }
        if self.create_drag.take().is_some() {
            // Switch back to Select for immediate shape refinement.
            self.editor_state.tool = op_editor_core::Tool::Select;
            self.mark_dirty();
            return true;
        }
        if let Some(drag) = self.node_drag.take() {
            // Drag ended — drop the transient smart-guide lines, then
            // run the drop policy (auto-layout reorder / reparent).
            self.editor_state.editor_ui.active_guides.clear();
            let _ = self.commit_node_drag(&drag);
            self.mark_dirty();
            return true;
        }
        if self.code_selection_drag.take().is_some() {
            return true;
        }
        if self.chat_input_selection_drag.take().is_some() {
            return true;
        }
        if self.chat_text_selection_drag.take().is_some() {
            return true;
        }
        if self.text_edit_selection_drag.take().is_some() {
            return true;
        }
        if let Some(drag) = self.path_anchor_drag.take() {
            // Push history only when the anchor actually moved.
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
                return true;
            }
            return false;
        }
        if let Some(drag) = self.arc_handle_drag.take() {
            // Commit history only when the arc actually changed.
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
                return true;
            }
            return false;
        }
        if self.design_md_drag.take().is_some() {
            // Position was updated live; release only ends the drag.
            return true;
        }
        if self.component_browser_drag.take().is_some() {
            return true;
        }
        if self.icon_picker_drag.take().is_some() {
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
            return true;
        }
        if let Some(m) = self.marquee_drag.take() {
            self.commit_marquee_selection(m, viewport_w, viewport_h);
            return true;
        }
        if let Some(d) = self.layer_drag.take() {
            return self.commit_layer_drag(d, viewport_h);
        }
        if let Some(d) = self.chat_drag.take() {
            // Snap using the live expanded/collapsed panel size.
            let (panel_w, panel_h) = self.ai_chat_size();
            let center = Point2D::new(d.pos_x + panel_w / 2.0, d.pos_y + panel_h / 2.0);
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
            self.editor_state.chat.anchor =
                op_editor_core::ChatAnchor::nearest(center, cx0, cy0, cw, ch);
            self.editor_state.chat.panel_position = None;
            self.mark_dirty();
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging || pressed_released
    }

    /// Viewport-less release variant — drops viewport-bound drags.
    /// Begin a canvas pan directly (middle-mouse press) — bypasses
    /// the tool branch; the shared cursor-move / release paths drive
    /// and end it like any pan drag.
    pub fn apply_pan_press(&mut self, x: f32, y: f32) -> bool {
        self.drag = Some(DragState {
            last_x: x,
            last_y: y,
        });
        true
    }

    pub fn apply_release(&mut self) -> bool {
        let pressed_released = self.release_pressed_feedback();
        // Pen owns the release while authoring (TS onMouseUp).
        if self.apply_pen_release() {
            return true;
        }
        if self.panel_resize.take().is_some() {
            return true;
        }
        if self.variables_resize.take().is_some() {
            return true;
        }
        if self.chat_resize.take().is_some() {
            return true;
        }
        if self.rotate_drag.take().is_some() {
            return true;
        }
        if self.handle_drag.take().is_some() {
            return true;
        }
        if self.create_drag.take().is_some() {
            self.editor_state.tool = op_editor_core::Tool::Select;
            self.mark_dirty();
            return true;
        }
        if let Some(drag) = self.node_drag.take() {
            // Drag ended — drop the transient smart-guide lines, then
            // run the drop policy (auto-layout reorder / reparent).
            self.editor_state.editor_ui.active_guides.clear();
            let _ = self.commit_node_drag(&drag);
            self.mark_dirty();
            return true;
        }
        if self.code_selection_drag.take().is_some() {
            return true;
        }
        if self.chat_input_selection_drag.take().is_some() {
            return true;
        }
        if self.chat_text_selection_drag.take().is_some() {
            return true;
        }
        if self.text_edit_selection_drag.take().is_some() {
            return true;
        }
        if self.marquee_drag.take().is_some() {
            // No viewport: drop without committing.
            return true;
        }
        if self.layer_drag.take().is_some() {
            // No viewport: drop the candidate.
            return true;
        }
        // Commit path / arc history when the drag actually moved.
        if let Some(drag) = self.path_anchor_drag.take() {
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
            }
            return true;
        }
        if let Some(drag) = self.arc_handle_drag.take() {
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
            }
            return true;
        }
        // Chat drag without viewport — drop it (best effort).
        if self.chat_drag.take().is_some() {
            return true;
        }
        if self.design_md_drag.take().is_some() {
            return true;
        }
        if self.component_browser_drag.take().is_some() {
            return true;
        }
        if self.icon_picker_drag.take().is_some() {
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging || pressed_released
    }

    // `arc_drag_command` (the `SetEllipseArc` builder) lives in the
    // `arc_drag.rs` sibling — relocated when the pen hooks landed
    // here, to keep this over-cap file from growing.
}

/// Convert a shell-core `Rect` (screen / doc px) into op-editor-core's
/// `DocRect`. Both crates carry `f32` rects; `DocRect` is `f64`.
pub(in crate::widget_host) fn rect_to_doc_rect(r: Rect) -> op_editor_core::DocRect {
    op_editor_core::DocRect {
        x: r.origin.x as f64,
        y: r.origin.y as f64,
        w: r.size.x as f64,
        h: r.size.y as f64,
    }
}

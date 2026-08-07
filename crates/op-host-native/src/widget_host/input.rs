//! Non-press input handlers on `WidgetHostNative`. press -> press.rs.

use super::helpers::{resize_bounds, PANEL_MAX_WIDTH, PANEL_MIN_WIDTH};
use super::{DragState, PanelResizeKind, WidgetHostNative};
use op_editor_core::codegen::CodeSelection;
use op_editor_ui::widgets::{
    AIChatHit, AIChatPlaceholder, CanvasViewport, ChatResizeEdge, PropertyPanel, AI_CHAT_MAX_RATIO,
    AI_CHAT_MIN_HEIGHT, AI_CHAT_MIN_WIDTH,
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
            || self.editor_state.color_picker_hex_focused()
            || self.editor_state.color_picker_rgb_focused()
            || self.editor_state.editor_ui.font_picker.open
            || self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
            || (self.editor_state.editor_ui.agent_settings_open
                && self.editor_state.editor_ui.agent_settings.focus.is_some())
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
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        let total_dx = ((x - drag.press_screen_x) / zoom) as f64;
        let total_dy = ((y - drag.press_screen_y) / zoom) as f64;
        if !drag.moved {
            // A transition from the previous drop/reflow paints geometry away
            // from the resolved scene. Once direct manipulation starts, the
            // cursor must own the node's position exactly. Later same-gesture
            // flex reorders may install fresh transitions that exclude the
            // dragged node and continue animating sibling avoidance.
            self.layout_transition = None;
            // Once the gesture becomes a drag it cannot be the first
            // half of a later double-click drill.
            self.editor_state.editor_ui.last_canvas_click = None;
            let option_source_ids: Vec<op_editor_core::NodeId> =
                self.editor_state.selection.set.to_vec();
            if self.alt_held
                && !option_source_ids.is_empty()
                && self
                    .editor_state
                    .duplicate_selected(&mut self.next_node_id, 0.0)
                    .is_some()
            {
                self.option_drag_source_ids = option_source_ids;
                let _ = self
                    .editor_state
                    .move_selected_in_layout_direction(total_dx, total_dy);
                // The drag snapshot already advanced the revision before the
                // clone and any flex reorder were authored.
                self.scene_cache.invalidate();
                self.mark_dirty();
            }
            if let Some(d) = self.node_drag.as_mut() {
                d.moved = true;
            }
        }
        // Net doc-space travel since the press — the release commit
        // uses it to locate dropped flex children (which never
        // doc-translate during the drag). Recomputed from the press
        // anchor so smart-guide rewinds of `last_screen_*` can't
        // double-count.
        if let Some(d) = self.node_drag.as_mut() {
            d.total_dx = total_dx;
            d.total_dy = total_dy;
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
                    .filter(|id| {
                        // Move exactly what `translate_selected` moved in the
                        // document: editable nodes only (locked / hidden are
                        // skipped there) and not flex-flow children. Otherwise
                        // the scene drifts nodes the doc never moved, then snaps
                        // back on the release-time reconversion.
                        self.editor_state.is_editable(id)
                            && !op_editor_core::walkers::is_flow_child_of_flex(children, id)
                    })
                    .map(|id| id.as_str().to_string())
                    .collect();
                let _ = self
                    .layout_scene
                    .translate_nodes(&ids, scene_dx as f32, scene_dy as f32);
                // The scene is now patched away from the last cached build, but
                // `scene_cache.last` still reflects the pre-drag inputs. Invalidate
                // it so a later refresh always rebuilds — otherwise, if the doc
                // returns to the cached value (e.g. undo, or dirty flipping
                // mid-drag), the cache would skip and leave this stale patch on
                // screen. Cheap: it only clears a field; no rebuild happens until
                // the next dirty refresh (release).
                self.scene_cache.invalidate();
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
            if let Some(drag) = self.node_drag {
                self.apply_live_node_drag_preview(&drag);
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
        // Selection probe resolves the transcript cache; owner-stamp it so the
        // slot stays tagged with this host's panel instead of clobbering it to
        // UNOWNED (which would flip the cursor-shape hint's read to None).
        match AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
            .owned_by(self.chat_panel_owner)
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
            .owned_by(self.chat_panel_owner)
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

    /// Patch the live scene's resolved fill / stroke for the active
    /// colour-picker drag without a layout rebuild. Returns `true` when the
    /// patch applied (caller skips `mark_dirty`); `false` when the edit is
    /// not in the patchable set — variable-mode, an instance redirect, a
    /// gradient-stop / effect target, or a node not solid-patchable — and
    /// the caller must rebuild for correctness.
    fn try_patch_color_drag(&mut self, is_instance: bool) -> bool {
        use op_editor_core::ui_draft::ColorTarget;
        use op_editor_ui::widgets::color_picker::hsv_to_rgb;
        if is_instance || self.editor_state_dirty {
            return false;
        }
        // Snapshot the picker inputs, then drop the borrow before touching
        // the doc / scene below.
        let (hue, sat, val, is_fill) = {
            let Some(state) = self.editor_state.ui.color_picker.as_ref() else {
                return false;
            };
            // Variable-mode writes fan out to every node referencing the
            // variable — far beyond the anchor — so let the rebuild repaint.
            if state.variable.is_some() {
                return false;
            }
            let is_fill = match state.target {
                ColorTarget::Fill => true,
                ColorTarget::Stroke => false,
                // GradientStop / EffectColor touch gradient bodies / effects
                // the scene patch does not model — rebuild instead.
                _ => return false,
            };
            (state.hue, state.sat, state.val, is_fill)
        };
        let anchor = self.editor_state.selection.anchor.clone();
        if !anchor.is_real() || !self.editor_state.is_editable(&anchor) {
            return false;
        }
        let Some(node) = self.editor_state.selected_node() else {
            return false;
        };
        // The picker write only lands when the anchor carries a writable
        // solid paint slot. A Ref / instance anchor has none, so
        // `set_selected_color` is a no-op there — patching the scene would
        // then show a colour the document never received (and snap back on
        // release). Require the freshly-written hex to be present, which
        // also screens out gradient-only / slotless nodes. (Belt to the
        // `is_instance` guard above for any Ref the redirect missed.)
        //
        // The loader also bakes the paint body's own opacity into the
        // resolved scene fill / stroke alpha (`apply_alpha` in
        // style_payload), and `set_node_*` bakes node cumulative opacity on
        // top. The picker writes an opaque hex but preserves the body
        // opacity, so fold it in here — otherwise a fill / stroke authored
        // below 100 % paints too opaque on every drag frame.
        let (wrote_paint, body_opacity) = if is_fill {
            (
                op_editor_core::fills::first_solid_fill_hex(node).is_some(),
                op_editor_core::fills::first_solid_fill_opacity(node),
            )
        } else {
            (
                op_editor_core::fills::first_solid_stroke_hex(node).is_some(),
                op_editor_core::fills::first_solid_stroke_opacity(node),
            )
        };
        if !wrote_paint {
            return false;
        }
        let mut color = hsv_to_rgb(hue, sat, val);
        color.a *= body_opacity.clamp(0.0, 1.0);
        let ids = [anchor.as_str().to_string()];
        let patched = if is_fill {
            self.layout_scene.set_node_fill(&ids, color)
        } else {
            self.layout_scene.set_node_stroke_color(&ids, color)
        };
        if patched {
            // The scene now drifts from the cached build inputs; invalidate
            // so a later refresh always rebuilds from the canonical doc
            // (mirrors the node-drag patch path — otherwise an undo back to
            // the cached colour would skip the rebuild and leave the patch).
            self.scene_cache.invalidate();
        }
        patched
    }

    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        // Session-switch owner rotation before the cursor_probe resolve below
        // stores the canonical build (mirrors the paint entry).
        self.rotate_chat_owner_if_session_changed();
        // In-flight VariablesPanel edge resize — owns the cursor.
        if self.variables_resize.is_some()
            && self.apply_variables_panel_resize(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return true;
        }
        // Missing-fonts modal — owns the cursor while open. Hover the
        // per-row choose-file buttons + the dismiss action.
        if self.editor_state.editor_ui.missing_fonts_modal_open {
            use op_editor_ui::widgets::missing_fonts_panel::MissingFontsPanel;
            let viewport = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, self.last_viewport_h),
            };
            let (new_hover, picker_hover, import_hover) =
                MissingFontsPanel::for_editor(&self.editor_state)
                    .map(|panel| {
                        let rect = panel.rect(self.last_viewport_w, self.last_viewport_h);
                        if panel.picker_layout(rect, viewport).is_some() {
                            let (entry, import) =
                                panel.picker_hover(rect, viewport, Point2D::new(x, y));
                            (None, entry, import)
                        } else {
                            (
                                op_editor_ui::widgets::editor_state_ext::missing_fonts_button(
                                    panel.hit_test(rect, viewport, Point2D::new(x, y)),
                                ),
                                None,
                                false,
                            )
                        }
                    })
                    .unwrap_or((None, None, false));
            let changed = new_hover != self.editor_state.editor_ui.missing_fonts_hover
                || picker_hover != self.editor_state.editor_ui.font_picker.hover
                || import_hover != self.editor_state.editor_ui.font_picker_import_hover;
            if changed {
                let ui = &mut self.editor_state.editor_ui;
                ui.missing_fonts_hover = new_hover;
                ui.font_picker.hover = picker_hover;
                ui.font_picker_import_hover = import_hover;
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.agent_settings_open {
            return self.update_agent_settings_hover(x, y);
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
        // Sign-in modal — owns the cursor while open. Hover the close
        // `✕` + the primary sign-in button.
        if op_editor_ui::widgets::ACCOUNT_UI_AVAILABLE
            && self.editor_state.editor_ui.login_modal_open
        {
            use op_editor_ui::widgets::login_modal::LoginModal;
            let modal = LoginModal::for_editor(&self.editor_state);
            let panel = modal.rect(self.last_viewport_w, self.last_viewport_h);
            let new_hover = op_editor_ui::widgets::editor_state_ext::login_modal_button(
                modal.hit_test(panel, Point2D::new(x, y)),
            );
            let changed = new_hover != self.editor_state.editor_ui.login_modal_hover;
            if changed {
                self.editor_state.editor_ui.login_modal_hover = new_hover;
                self.mark_dirty();
            }
            return changed;
        }
        // Signed-in account dropdown — owns the cursor while open.
        if op_editor_ui::widgets::ACCOUNT_UI_AVAILABLE
            && self.editor_state.editor_ui.account_menu_open
        {
            use op_editor_ui::widgets::account_menu::AccountMenu;
            use op_editor_ui::widgets::top_bar::TopBar;
            use op_editor_ui::widgets::TOP_BAR_HEIGHT;
            let top_bar_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, TOP_BAR_HEIGHT),
            };
            let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
            let anchor = top_bar.account_button_rect(top_bar_rect);
            let new_hover =
                AccountMenu::for_editor_ui(&self.editor_state.editor_ui).and_then(|menu| {
                    let panel = menu.rect_at(anchor);
                    menu.row_at(panel, Point2D::new(x, y))
                });
            let changed = new_hover != self.editor_state.editor_ui.account_menu_hover;
            if changed {
                self.editor_state.editor_ui.account_menu_hover = new_hover;
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
                let is_instance = instance_scope.is_some();
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
                // A solid Fill/Stroke change on a concrete anchor touches
                // no layout, so patch the resolved scene paint in place
                // rather than re-running taffy + reshape on every drag
                // frame (mirrors the node-drag `translate_nodes` fast
                // path). Variable-mode, instance redirects, gradient-stop /
                // effect targets, and new strokes fall back to a rebuild.
                if !self.try_patch_color_drag(is_instance) {
                    self.mark_dirty();
                }
                return true;
            }
            // Picker open but not dragging — it is a top-most overlay, so a move
            // over its panel must be swallowed (canvas hover must not bleed
            // through underneath it). Matches the design-md / modal behaviour.
            let picker = op_editor_ui::widgets::color_picker::ColorPicker::for_state(
                &self.editor_state,
                state.clone(),
            );
            let panel = picker.rect(self.last_viewport_w, self.last_viewport_h);
            if panel.contains(Point2D::new(x, y)) {
                self.clear_lower_overlay_hover();
                return true;
            }
        }
        // Live preview owns canvas cursor moves (hover + drag into the
        // runtime). Runs below the modal guards (they own the cursor
        // while open); floating overlays are excluded inside
        // `preview_dispatch_move` via `over_topmost_panel`, and
        // off-canvas moves fall through so top-bar hover still works
        // while previewing.
        if self.preview.is_some() {
            self.screen_switcher_hover(x, y, self.last_viewport_w, self.last_viewport_h);
            self.preview_switcher_hover(x, y, self.last_viewport_w, self.last_viewport_h);
            if self.preview_dispatch_move(x, y) {
                return true;
            }
        }
        // Top-most floating panel drags own cursor movement.
        let model_picker_open = self.editor_state.editor_ui.chat_model_picker.open;
        // Chat paints above Variables/Toolbar/Canvas. Resolve its cheap surface
        // ownership before those lower layers get a chance to consume the move;
        // the full transcript-aware `cursor_probe` remains single-shot below.
        let chat_surface_owns_point = !model_picker_open
            && self.chat_panel_surface_contains(x, y, self.last_viewport_w, self.last_viewport_h);
        let chat_or_picker_owns_point = model_picker_open || chat_surface_owns_point;
        let mut higher_overlay_hover_changed = false;
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
                let point = Point2D::new(x, y);
                let new_hover = DesignMdPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, point));
                let changed = new_hover != self.editor_state.editor_ui.design_md_hover;
                if changed {
                    self.editor_state.editor_ui.design_md_hover = new_hover;
                    self.mark_dirty();
                }
                if panel_rect.contains(point) {
                    self.clear_lower_overlay_hover();
                    return true;
                }
                if changed && !chat_or_picker_owns_point {
                    return true;
                }
                higher_overlay_hover_changed |= changed;
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
                let point = Point2D::new(x, y);
                let new_hover = ComponentBrowserPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, point));
                let changed = new_hover != self.editor_state.editor_ui.component_browser_hover;
                if changed {
                    self.editor_state.editor_ui.component_browser_hover = new_hover;
                    self.mark_dirty();
                }
                if panel_rect.contains(point) {
                    self.clear_lower_overlay_hover();
                    return true;
                }
                if changed && !chat_or_picker_owns_point {
                    return true;
                }
                higher_overlay_hover_changed |= changed;
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
                let point = Point2D::new(x, y);
                let new_hover = IconPickerPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, point));
                let changed = new_hover != self.editor_state.editor_ui.icon_picker.hover;
                if changed {
                    self.editor_state.editor_ui.icon_picker.hover = new_hover;
                    self.mark_dirty();
                }
                if panel_rect.contains(point) {
                    self.clear_lower_overlay_hover();
                    return true;
                }
                if changed && !chat_or_picker_owns_point {
                    return true;
                }
                higher_overlay_hover_changed |= changed;
            }
        }
        if self.over_dropdown_overlay(x, y, self.last_viewport_w, self.last_viewport_h) {
            self.update_dropdown_hover(x, y, false);
            self.clear_chat_and_lower_hover();
            return true;
        }
        // Preset dropdown is a top-most overlay over the variables panel — track
        // its per-row hover and swallow moves over it first.
        if !chat_or_picker_owns_point
            && self.editor_state.editor_ui.variables_preset_menu_open
            && self.update_variables_preset_menu_hover(
                x,
                self.last_viewport_w,
                self.last_viewport_h,
                y,
            )
        {
            self.clear_lower_overlay_hover();
            return true;
        }
        if !chat_or_picker_owns_point && self.editor_state.editor_ui.variables_panel_open {
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
                    self.clear_lower_overlay_hover();
                    return true;
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
        // Keep one owned PropertyPanel snapshot for this cursor event. The
        // nested Option records that construction was attempted even when the
        // current selection cannot produce a panel, so a later hover layer
        // does not repeat the same expensive snapshot/i18n work.
        let property_rect = self.property_rect(self.last_viewport_w, self.last_viewport_h);
        let point = Point2D::new(x, y);
        let property_drag_active =
            self.image_adjustment_drag.is_some() || self.effect_radius_drag.is_some();
        let mut property_panel_probe =
            property_drag_active.then(|| PropertyPanel::for_selection(&self.editor_state));
        if let Some(field) = self.image_adjustment_drag {
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
                if let Some(action) = panel.image_adjustment_drag_action(property_rect, field, x) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        if let Some(effect) = self.effect_radius_drag {
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
                if let Some(action) = panel.effect_radius_drag_action(property_rect, effect, x) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        if self.apply_image_input_selection_drag_cursor_move(x, y) {
            return true;
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
        if let Some(consumed) = self.apply_image_crop_drag_cursor_move(x, y) {
            return consumed;
        }
        if let Some(consumed) = self.apply_node_drag_cursor_move(x, y) {
            return consumed;
        }
        // Pen handle-drag minting + rubber-band (`pen_press.rs`).
        if let Some(consumed) = self.apply_pen_cursor_move(x, y) {
            return consumed;
        }
        // Suppress lower-overlay hover while a floating panel is on top.
        // VariablesPanel is below Chat in paint order; when the model picker
        // is visible, the earlier true-top-panel branches have already
        // returned and Variables must not make this gate look top-most.
        let over_topmost = !chat_or_picker_owns_point
            && self.over_topmost_panel(x, y, self.last_viewport_w, self.last_viewport_h);
        let mut upper_hover_changed = higher_overlay_hover_changed;
        // Fold stale-hover clearing into the final repaint signal.
        let cleared = over_topmost && self.clear_lower_overlay_hover();
        // Path-anchor context menu is painted above Git and Chat. An unchanged
        // row still owns the point, so return without falling into the model
        // picker behind it.
        let over_path_menu = self
            .editor_state
            .ui
            .path_anchor_menu
            .clone()
            .map(|state| {
                op_editor_ui::widgets::path_anchor_context_menu::PathAnchorContextMenu::for_state(
                    &self.editor_state,
                    state,
                )
                .rect()
                .contains(Point2D::new(x, y))
            })
            .unwrap_or(false);
        let path_menu_changed = self.update_path_anchor_menu_hover(x, y);
        if over_path_menu {
            let below_changed = self.clear_chat_and_lower_hover();
            return path_menu_changed || below_changed;
        }
        if path_menu_changed {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        if let Some(state) = self
            .editor_state
            .editor_ui
            .layer_context_menu
            .clone()
            .filter(|_| !over_topmost)
        {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            let over_menu = menu.rect().contains(Point2D::new(x, y));
            let new_hover = menu.hovered_row_at(Point2D::new(x, y));
            let menu_hover_changed = new_hover != state.menu.hover;
            if menu_hover_changed {
                let mut next = state;
                next.menu.hover = new_hover;
                self.editor_state.editor_ui.layer_context_menu = Some(next);
                self.mark_dirty();
            }
            if over_menu {
                let below_changed = self.clear_chat_and_lower_hover();
                return menu_hover_changed || below_changed;
            }
            if menu_hover_changed {
                if chat_or_picker_owns_point {
                    upper_hover_changed = true;
                } else {
                    return true;
                }
            }
        }
        // Git is painted and pressed above Chat. Update both Git views before
        // the picker and keep an unchanged point inside the Git footprint from
        // leaking into the picker behind it.
        let over_git = self
            .git_panel_outer_rect(self.last_viewport_w, self.last_viewport_h)
            .is_some_and(|rect| rect.contains(Point2D::new(x, y)));
        let git_hover_changed =
            self.update_git_panel_empty_hover(x, y) | self.update_git_panel_ready_hover(x, y);
        if over_git {
            let below_changed = self.clear_chat_and_lower_hover();
            return git_hover_changed || below_changed;
        }
        if git_hover_changed {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        // File-menu / locale / shape dropdown hover (`geometry.rs`).
        if self.update_dropdown_hover(x, y, over_topmost) {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        let property_popover_open = self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
            || self.editor_state.editor_ui.compositing_picker.open
            || self.editor_state.editor_ui.effect_add_picker_open
            || self.editor_state.editor_ui.interaction_menu_open
            || self.editor_state.editor_ui.padding_mode_popover_open
            || self.editor_state.editor_ui.stroke_mode_popover_open
            || self.editor_state.editor_ui.font_weight_picker_open
            || self.editor_state.editor_ui.font_picker.open
            || self.editor_state.editor_ui.image_fill_popover_open
            || self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open;
        if property_popover_open && property_panel_probe.is_none() {
            // The old per-popover helpers each refreshed and rebuilt the same
            // panel. Refresh once, then share one immutable snapshot across
            // every overlay and the eventual base-property hover pass.
            self.refresh_layout_scene();
            property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
        }
        let property_panel = property_panel_probe.as_ref().and_then(Option::as_ref);
        // Image Search / Generate and image-fill adjustment popovers are
        // painted and pressed above Chat. Their popup body receives first
        // refusal even while the Chat model picker is open; reuse the same
        // PropertyPanel snapshot as every other property overlay.
        let over_image_popup = property_panel.is_some_and(|panel| {
            panel.image_popovers_contain(property_rect, point)
                || panel.image_fill_popover_contains(property_rect, point)
        });
        if over_image_popup {
            return self.clear_chat_and_lower_hover();
        }
        // Export-section select-popup row hover (no-op when closed).
        let over_export_popup = !over_topmost
            && property_panel
                .is_some_and(|panel| panel.export_picker_contains(property_rect, point));
        let export_hover_changed = !over_topmost
            && self.update_export_picker_hover(
                x,
                y,
                self.last_viewport_w,
                self.last_viewport_h,
                property_panel,
            );
        if over_export_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return export_hover_changed || below_changed;
        }
        if export_hover_changed {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        // Effects "+" add-menu row hover (no-op when closed).
        let over_effect_popup = !over_topmost
            && property_panel
                .is_some_and(|panel| panel.effect_add_menu_contains(property_rect, point));
        let effect_hover_changed = !over_topmost
            && self.update_effect_add_menu_hover(
                x,
                y,
                self.last_viewport_w,
                self.last_viewport_h,
                property_panel,
            );
        if over_effect_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return effect_hover_changed || below_changed;
        }
        if effect_hover_changed {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        // Layer / mask / per-fill blend dropdown hover. Blend choices use a
        // compact two-column popup, but the SelectState still stores the
        // flattened row index so paint and both hosts remain identical.
        let over_compositing_popup = !over_topmost
            && self.editor_state.editor_ui.compositing_picker.open
            && property_panel
                .is_some_and(|panel| panel.compositing_picker_contains(property_rect, point));
        let new_compositing_hover = (!over_topmost
            && self.editor_state.editor_ui.compositing_picker.open)
            .then(|| {
                property_panel
                    .and_then(|panel| panel.compositing_picker_row_at(property_rect, point))
            })
            .flatten();
        let compositing_hover_changed =
            new_compositing_hover != self.editor_state.editor_ui.compositing_picker.hover;
        if compositing_hover_changed {
            self.editor_state.editor_ui.compositing_picker.hover = new_compositing_hover;
            self.mark_dirty();
        }
        if over_compositing_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return compositing_hover_changed || below_changed;
        }
        if compositing_hover_changed {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        // Interactions Navigate/Back/Remove popover row hover (no-op
        // when closed).
        let over_interaction_popup = !over_topmost
            && property_panel.is_some_and(|panel| {
                !matches!(
                    panel.interaction_menu_hit(property_rect, point),
                    op_editor_ui::widgets::InteractionMenuHit::Outside
                )
            });
        let interaction_hover_changed = !over_topmost
            && self.update_interaction_menu_hover(
                x,
                y,
                self.last_viewport_w,
                self.last_viewport_h,
                property_panel,
            );
        if over_interaction_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return interaction_hover_changed || below_changed;
        }
        if interaction_hover_changed {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        // Padding-mode gear popover row hover (no-op when closed).
        let over_padding_or_stroke_popup = property_panel.is_some_and(|panel| {
            (self.editor_state.editor_ui.padding_mode_popover_open
                && panel.padding_mode_popover_contains(property_rect, point))
                || (self.editor_state.editor_ui.stroke_mode_popover_open
                    && panel.stroke_mode_popover_contains(property_rect, point))
        });
        let padding_hover_changed = self.update_padding_mode_popover_hover(x, y, property_panel);
        if over_padding_or_stroke_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return padding_hover_changed || below_changed;
        }
        if padding_hover_changed {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        // Font-weight dropdown row hover (no-op when closed).
        let over_font_weight_popup = property_panel
            .is_some_and(|panel| panel.font_weight_picker_contains(property_rect, point));
        let font_weight_hover_changed = self.update_font_weight_picker_hover(x, y, property_panel);
        if over_font_weight_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return font_weight_hover_changed || below_changed;
        }
        if font_weight_hover_changed {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        // Font-family picker entry hover (no-op when closed).
        let (over_font_picker, font_picker_hover_changed) =
            self.update_font_picker_hover(x, y, property_panel);
        if over_font_picker {
            let below_changed = self.clear_chat_and_lower_hover();
            return font_picker_hover_changed || below_changed;
        }
        if font_picker_hover_changed {
            if chat_or_picker_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        // StatusBar paints and presses above Chat. Preserve its hover/cursor
        // ownership before the model picker globally truncates lower layers.
        if let Some(status_rect) = self.status_bar_rect(self.last_viewport_w, self.last_viewport_h)
        {
            let point = Point2D::new(x, y);
            let over_status = !over_topmost && status_rect.contains(point);
            let new_hover = if over_topmost {
                None
            } else {
                op_editor_ui::widgets::StatusBar::for_editor(&self.editor_state)
                    .control_at(status_rect, point)
            };
            let status_hover_changed = new_hover != self.editor_state.editor_ui.statusbar_hover;
            if status_hover_changed {
                self.editor_state.editor_ui.statusbar_hover = new_hover;
                self.mark_dirty();
            }
            if over_status {
                let below_changed = self.clear_chat_and_lower_hover();
                return status_hover_changed || below_changed;
            }
            if status_hover_changed {
                if chat_or_picker_owns_point {
                    upper_hover_changed = true;
                } else {
                    return true;
                }
            }
        }
        // AlignToolbar is painted after Chat, so its entire card (including
        // gaps between buttons) receives first refusal before either the model
        // picker or the ordinary Chat surface truncates lower hover dispatch.
        let late_pointer_capture_active = self.rotate_drag.is_some()
            || self.handle_drag.is_some()
            || self.create_drag.is_some()
            || self.path_anchor_drag.is_some()
            || self.arc_handle_drag.is_some()
            || self.marquee_drag.is_some()
            || self.layer_drag.is_some()
            || self.panel_resize.is_some()
            || self.chat_resize.is_some()
            || self.chat_drag.is_some()
            || self.drag.is_some();
        if !late_pointer_capture_active {
            let over_align = !over_topmost
                && self
                    .align_toolbar_rect(self.last_viewport_w, self.last_viewport_h)
                    .is_some_and(|rect| rect.contains(point));
            let new_align_hover = if !over_topmost {
                self.align_toolbar_hit(x, y, self.last_viewport_w, self.last_viewport_h)
            } else {
                None
            };
            let align_hover_changed =
                new_align_hover != self.editor_state.editor_ui.align_toolbar_hover;
            if align_hover_changed {
                self.editor_state.editor_ui.align_toolbar_hover = new_align_hover;
                self.mark_dirty();
            }
            if over_align {
                let below_changed = self.clear_chat_and_lower_hover();
                return align_hover_changed || below_changed;
            }
            if align_hover_changed {
                if chat_or_picker_owns_point {
                    upper_hover_changed = true;
                } else {
                    return true;
                }
            }
        }
        // The chat model picker is modal only below overlays painted and
        // pressed above the chat (Git, context menus, and property popovers).
        // Once those surfaces have had first refusal, update the picker row,
        // clear stale lower highlights, and stop before constructing the base
        // chat/property/canvas probes.
        if self.editor_state.editor_ui.chat_model_picker.open {
            use op_editor_ui::widgets::ai_chat_model_picker::{model_picker_hit, SelectHit};
            let Some(picker) =
                self.chat_model_picker_rect(self.last_viewport_w, self.last_viewport_h)
            else {
                // A collapsed/hidden chat or a viewport too small to lay it
                // out must not leave an invisible modal hover lock behind.
                self.editor_state.editor_ui.close_chat_model_picker();
                self.mark_dirty();
                return true;
            };
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
            let hover_changed = new_hover != self.editor_state.editor_ui.chat_model_picker.hover;
            if hover_changed {
                self.editor_state.editor_ui.chat_model_picker.hover = new_hover;
                self.mark_dirty();
            }
            let lower_changed = self.clear_hover_below_chat_model_picker();
            return upper_hover_changed || hover_changed || lower_changed;
        }
        // TopBar window-control cluster — hovering it reveals the
        // close / minimise / maximise glyphs on the 3 dots.
        {
            use op_editor_ui::widgets::{TopBar, TOP_BAR_HEIGHT};
            let tb_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, TOP_BAR_HEIGHT),
            };
            let over = !chat_surface_owns_point
                && (TopBar::traffic_cluster_rect(tb_rect)).contains(Point2D::new(x, y));
            if over != self.editor_state.editor_ui.topbar_traffic_hover {
                self.editor_state.editor_ui.topbar_traffic_hover = over;
                self.mark_dirty();
                if chat_surface_owns_point {
                    upper_hover_changed = true;
                } else {
                    return true;
                }
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
            let mut top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
            top_bar.chip_text_w = Some(self.topbar_chip_text_w(&top_bar));
            let new_hover = (!chat_surface_owns_point)
                .then(|| top_bar.hit_test(tb_rect, Point2D::new(x, y)))
                .flatten()
                .map(op_editor_ui::widgets::editor_state_ext::topbar_button_hover);
            if new_hover != self.editor_state.editor_ui.topbar_button_hover {
                self.editor_state.editor_ui.topbar_button_hover = new_hover;
                self.mark_dirty();
                if chat_surface_owns_point {
                    upper_hover_changed = true;
                } else {
                    return true;
                }
            }
        }
        // Construct the chat panel once for this cursor event and resolve all
        // of its hover results in one immutable scope. Besides keeping the
        // transcript fingerprint to one pass, this avoids cloning translated
        // labels and tab titles again for every chat sub-control.
        let (chat_probe, chat_tab_hover, chat_footer_hover, parallel_hover, example_hover) =
            if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
                let point = Point2D::new(x, y);
                let panel = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                    .owned_by(self.chat_panel_owner);
                (
                    Some(panel.cursor_probe(chat_rect, point)),
                    panel.tab_hover_at(chat_rect, point),
                    panel.footer_hover_at(chat_rect, point),
                    panel.parallel_agents_picker_hover_at(chat_rect, point),
                    panel.example_hover_at(chat_rect, point),
                )
            } else {
                (None, None, None, None, None)
            };
        // `cursor_probe.hit` is the authoritative Chat ownership result: it is
        // Some for every painted body point and for the invisible resize
        // gutter. Aggregate all Chat-owned hover writes before returning so a
        // changed control can never leave a stale canvas highlight behind.
        let chat_owns_point = chat_probe.as_ref().is_some_and(|probe| probe.hit.is_some());
        let mut chat_hover_changed = false;
        let new_header_hover = chat_probe
            .as_ref()
            .and_then(|probe| probe.hit.as_ref())
            .and_then(op_editor_ui::widgets::editor_state_ext::chat_header_hover);
        if new_header_hover != self.editor_state.editor_ui.chat_header_hover {
            self.editor_state.editor_ui.chat_header_hover = new_header_hover;
            chat_hover_changed = true;
        }
        // AI chat tab row hover — drives the close-× visibility on each tab.
        if chat_tab_hover != self.editor_state.editor_ui.chat_tab_hover {
            self.editor_state.editor_ui.chat_tab_hover = chat_tab_hover;
            chat_hover_changed = true;
        }
        if chat_footer_hover != self.editor_state.editor_ui.chat_footer_hover {
            self.editor_state.editor_ui.chat_footer_hover = chat_footer_hover;
            chat_hover_changed = true;
        }
        // Parallel-agents picker row hover — drives the highlight wash inside the overlay.
        if parallel_hover != self.editor_state.editor_ui.parallel_agents_picker_hover {
            self.editor_state.editor_ui.parallel_agents_picker_hover = parallel_hover;
            chat_hover_changed = true;
        }
        if example_hover != self.editor_state.editor_ui.chat_example_hover {
            self.editor_state.editor_ui.chat_example_hover = example_hover;
            chat_hover_changed = true;
        }
        // Design-block hover — reuse the combined probe resolved above (gated on
        // `over_topmost` exactly as the old dedicated pass was).
        let design_hover = if over_topmost {
            None
        } else {
            chat_probe
                .as_ref()
                .and_then(|probe| probe.design_block_hover)
        };
        chat_hover_changed |= self.apply_chat_design_hover(design_hover);
        if chat_hover_changed {
            self.mark_dirty();
            upper_hover_changed = true;
        }
        if let Some(drag) = self.rotate_drag {
            self.refresh_layout_scene();
            let cursor_angle = (y - drag.center_screen_y).atan2(x - drag.center_screen_x);
            let new_rotation = drag.start_rotation + (cursor_angle - drag.start_cursor_angle);
            self.editor_state.set_selected_rotation(new_rotation);
            self.finish_live_rotation_update(new_rotation);
            return true;
        }
        if let Some(drag) = self.handle_drag {
            let patch_scene = self.prepare_live_bounds_update();
            let zoom = self.editor_state.viewport.zoom.max(0.0001);
            let dx = (x - drag.start_screen_x) / zoom;
            let dy = (y - drag.start_screen_y) / zoom;
            let new_bounds = resize_bounds(drag.start_bounds, drag.handle, dx, dy);
            let new_x = drag.handle.moves_left_edge().then(|| {
                drag.start_authored_x.unwrap_or(0.0)
                    + f64::from(new_bounds.origin.x - drag.start_bounds.origin.x)
            });
            let new_y = drag.handle.moves_top_edge().then(|| {
                drag.start_authored_y.unwrap_or(0.0)
                    + f64::from(new_bounds.origin.y - drag.start_bounds.origin.y)
            });
            self.editor_state.resize_selected_bounds(
                rect_to_doc_rect(new_bounds),
                drag.handle.resize_axes(),
                new_x,
                new_y,
            );
            self.finish_live_bounds_update(new_bounds, patch_scene);
            return true;
        }
        if let Some(drag) = self.create_drag {
            let patch_scene = self.prepare_live_bounds_update();
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
            self.finish_live_bounds_update(new_bounds, patch_scene);
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
            self.note_viewport_gesture();
            // Canvas pan only translates the viewport; keep layout cache intact.
            return true;
        }
        if chat_owns_point {
            let lower_changed = self.clear_hover_below_chat_panel();
            return upper_hover_changed || lower_changed;
        }
        // Toolbar hover after drag detection.
        if self.update_toolbar_hover(x, y, over_topmost) {
            return true;
        }
        // PropertyPanel tab/action hover wash. Shown with a selection.
        let mut property_hover_changed = false;
        let needs_property_probe = !over_topmost
            && self.editor_state.property_panel_visible()
            && (property_rect.contains(point)
                || self.editor_state.editor_ui.fill_type_picker.open
                || self.editor_state.editor_ui.compositing_picker.open);
        if needs_property_probe && property_panel_probe.is_none() {
            property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
        }
        let property_panel = if needs_property_probe {
            property_panel_probe.as_ref().and_then(Option::as_ref)
        } else {
            None
        };
        if let Some(panel) = property_panel {
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
            let new_compositing_hover = panel.compositing_picker_row_at(property_rect, point);
            if new_compositing_hover != self.editor_state.editor_ui.compositing_picker.hover {
                self.editor_state.editor_ui.compositing_picker.hover = new_compositing_hover;
                property_hover_changed = true;
            }
            let new_action_hover = panel.action_hover_index(property_rect, point);
            if new_action_hover != self.editor_state.editor_ui.property_action_hover {
                self.editor_state.editor_ui.property_action_hover = new_action_hover;
                property_hover_changed = true;
            }
        } else {
            let ui = &mut self.editor_state.editor_ui;
            property_hover_changed |= ui.property_tab_hover.take().is_some();
            property_hover_changed |= ui.fill_type_picker.hover.take().is_some();
            property_hover_changed |= ui.compositing_picker.hover.take().is_some();
            property_hover_changed |= ui.property_action_hover.take().is_some();
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
        // Canvas hierarchy hover: resolve the current level's focus
        // from the root-to-deepest scene path. Shared paint outlines
        // the focus solid and all direct children dashed. Reads the
        // CURRENT layout scene without refreshing (same discipline as
        // layer-row hover — hover must not rebuild a stale scene).
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
                    return cleared || upper_hover_changed;
                }
            }
            self.last_hover_probe = Some((x, y));
            let (cx0, cy0, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let canvas_rect = Rect {
                origin: Point2D::new(cx0, cy0),
                size: Point2D::new(cw, ch),
            };
            let canvas = CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
            if let Some(root) = canvas.frame_label_at_point(canvas_rect, Point2D::new(x, y)) {
                Some(op_editor_core::NodeId::new(root))
            } else {
                let canvas_local = Point2D::new(x - cx0, y - cy0);
                let doc = self.editor_state.viewport.to_document(canvas_local);
                self.layout_scene
                    .node_path_at_doc_point(doc, self.editor_state.viewport.zoom)
                    .and_then(|path| {
                        let path = path
                            .into_iter()
                            .map(op_editor_core::NodeId::new)
                            .collect::<Vec<_>>();
                        op_editor_core::selection_resolve::resolve_canvas_depth_targets(
                            &path,
                            self.editor_state.editor_ui.entered_container.as_ref(),
                        )
                        .map(|targets| targets.primary)
                    })
            }
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
        cleared || upper_hover_changed
    }

    /// Mouse-release — ends active drag; chat-panel snaps corner.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        let pressed_released = self.release_pressed_feedback();
        if self.screen_switcher_release() {
            return true;
        }
        if self.preview_switcher_release() {
            return true;
        }
        // Live preview drag → pointer Up into the runtime.
        if self.preview_dispatch_release() {
            return true;
        }
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
            self.invalidate_live_scene_for_rebuild();
            return true;
        }
        if self.handle_drag.take().is_some() {
            self.invalidate_live_scene_for_rebuild();
            return true;
        }
        if self.create_drag.take().is_some() {
            // Switch back to Select for immediate shape refinement.
            self.editor_state.tool = op_editor_core::Tool::Select;
            self.invalidate_live_scene_for_rebuild();
            return true;
        }
        if self.finish_image_crop_drag() {
            return true;
        }
        if let Some(drag) = self.node_drag.take() {
            self.canvas_drop_index = None;
            // Drag ended — drop the transient smart-guide lines, then
            // run the drop policy (auto-layout reorder / reparent).
            self.refresh_layout_scene();
            let before_scene = self.layout_scene.clone();
            let release_overlay = (self.editor_state.selection_count() == 1)
                .then(|| {
                    drag.overlay_bounds
                        .map(|bounds| (self.editor_state.selection.anchor.clone(), bounds))
                })
                .flatten();
            let should_commit_drop = self
                .editor_state
                .editor_ui
                .canvas_drop_indicator
                .as_ref()
                .map(|indicator| indicator.target.is_some())
                .unwrap_or(false)
                || self.editor_state.selection_count() != 1;
            self.editor_state.editor_ui.active_guides.clear();
            self.editor_state.editor_ui.canvas_drop_indicator = None;
            if should_commit_drop {
                let _ = self.commit_node_drag(&drag);
            }
            self.option_drag_source_ids.clear();
            self.mark_dirty();
            if should_commit_drop {
                self.start_layout_transition_from_scene(before_scene);
            } else {
                self.refresh_layout_scene();
                if let Some((node_id, bounds)) = release_overlay {
                    self.start_layout_transition_from_bounds(&node_id, bounds);
                }
            }
            self.mark_dirty();
            return true;
        }
        if self.code_selection_drag.take().is_some() {
            return true;
        }
        if self.image_input_selection_drag.take().is_some() {
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
        if self.effect_radius_drag.take().is_some() {
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
            self.invalidate_live_scene_for_rebuild();
            return true;
        }
        if self.handle_drag.take().is_some() {
            self.invalidate_live_scene_for_rebuild();
            return true;
        }
        if self.create_drag.take().is_some() {
            self.editor_state.tool = op_editor_core::Tool::Select;
            self.invalidate_live_scene_for_rebuild();
            return true;
        }
        if self.finish_image_crop_drag() {
            return true;
        }
        if let Some(drag) = self.node_drag.take() {
            self.canvas_drop_index = None;
            // Drag ended — drop the transient smart-guide lines, then
            // run the drop policy (auto-layout reorder / reparent).
            self.refresh_layout_scene();
            let before_scene = self.layout_scene.clone();
            let release_overlay = (self.editor_state.selection_count() == 1)
                .then(|| {
                    drag.overlay_bounds
                        .map(|bounds| (self.editor_state.selection.anchor.clone(), bounds))
                })
                .flatten();
            let should_commit_drop = self
                .editor_state
                .editor_ui
                .canvas_drop_indicator
                .as_ref()
                .map(|indicator| indicator.target.is_some())
                .unwrap_or(false)
                || self.editor_state.selection_count() != 1;
            self.editor_state.editor_ui.active_guides.clear();
            self.editor_state.editor_ui.canvas_drop_indicator = None;
            if should_commit_drop {
                let _ = self.commit_node_drag(&drag);
            }
            self.option_drag_source_ids.clear();
            self.mark_dirty();
            if should_commit_drop {
                self.start_layout_transition_from_scene(before_scene);
            } else {
                self.refresh_layout_scene();
                if let Some((node_id, bounds)) = release_overlay {
                    self.start_layout_transition_from_bounds(&node_id, bounds);
                }
            }
            self.mark_dirty();
            return true;
        }
        if self.code_selection_drag.take().is_some() {
            return true;
        }
        if self.image_input_selection_drag.take().is_some() {
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
        if self.effect_radius_drag.take().is_some() {
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

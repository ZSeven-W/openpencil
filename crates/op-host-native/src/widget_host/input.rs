//! Non-press input handlers on `WidgetHostNative`. press -> press.rs.
//!
//! `apply_cursor_move` is the spine of the cursor-move tier ladder; the
//! tier bodies live in the `cursor_move_*.rs` siblings and the shared
//! per-event scratch state in `cursor_move_ctx.rs`. Mouse-release
//! handlers live in `release.rs`.

use super::cursor_move_ctx::CursorMoveCtx;
use super::WidgetHostNative;
use op_editor_core::codegen::CodeSelection;
use op_editor_ui::widgets::{AIChatHit, AIChatPlaceholder, PropertyPanel};
use op_editor_ui::{Point2D, Rect};

/// Minimum cursor travel (logical px) from the node-drag press point
/// before a move is committed. A pure click with sub-pixel jitter then
/// never mutates the document — kills "first click breaks the layout".
pub(in crate::widget_host) const NODE_DRAG_THRESHOLD_PX: f32 = 4.0;
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
        self.editor_state.editor_ui.collab_join_input_active()
            || ui.layer_rename.is_some()
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
            || self.editor_state.editor_ui.prompt_center.open
            // The Asset Center owns the keyboard while a visible field has
            // explicit focus. Desktop opens search-focused; touch waits for a
            // field tap so opening the gallery cannot raise the software
            // keyboard over its cards. Its
            // absence here is what left the gallery unable to take IME input:
            // `text_input_focus_active` reads this list, and a `false` there
            // makes the desktop shell call `set_ime_allowed(false)`, so the
            // platform never opens a composition session and pinyin produced
            // nothing at all while ASCII still went through `apply_text`.
            || self.editor_state.editor_ui.scene_template_center.input_active()
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
            || self.editor_state.editor_ui.collab_join_input_active()
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

    pub(in crate::widget_host) fn apply_node_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
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
            // A previous drop/reflow transition paints interpolated geometry.
            // Once direct manipulation starts, the cursor owns the dragged
            // node's position exactly.
            self.layout_transition = None;
            let mutation = if self.alt_held {
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::Duplicate,
                )
            } else {
                op_editor_core::CollabDocumentMutation::NodeMove
            };
            if !self.collab_allows_document_mutation(mutation) {
                self.node_drag = None;
                return Some(true);
            }
            let result = if let Some(allocator) = self.collab_id_allocator.as_mut() {
                op_editor_core::host_drag_transitions::activate_node_drag_with_allocator(
                    &mut self.editor_state,
                    allocator,
                    self.alt_held,
                    total_dx,
                    total_dy,
                )
            } else {
                Ok(op_editor_core::host_drag_transitions::activate_node_drag(
                    &mut self.editor_state,
                    &mut self.next_node_id,
                    self.alt_held,
                    total_dx,
                    total_dy,
                ))
            };
            let activation = match result {
                Ok(activation) => activation,
                Err(error) => {
                    self.node_drag = None;
                    self.show_collab_id_error(error);
                    return Some(true);
                }
            };
            if activation.duplicated {
                self.option_drag_source_ids = activation.option_drag_source_ids;
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
            // Re-check at the mutation sink. A role downgrade, disconnect, or
            // session end can arrive after the drag crossed its threshold.
            if !self
                .collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeMove)
            {
                self.node_drag = None;
                self.canvas_drop_index = None;
                self.editor_state.editor_ui.active_guides.clear();
                self.editor_state.editor_ui.canvas_drop_indicator = None;
                return Some(true);
            }
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
                let ids =
                    op_editor_ui::widgets::drag_flow::drag_scene_translate_ids(&self.editor_state);
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
        if self.editor_state.editor_ui.touch_chrome()
            && !self.editor_state.editor_ui.expanded_touch_layout()
            && self.editor_state.editor_ui.mobile_sheet
                != Some(op_editor_core::size_class::MobileSheetKind::Properties)
        {
            return None;
        }
        let panel_rect = self.property_rect(self.last_viewport_w, self.last_viewport_h);
        if !panel_rect.contains(Point2D::new(x, y)) {
            return None;
        }
        op_editor_ui::widgets::PropertyPanel::for_selection(&self.editor_state)?
            .code_text_offset_at(panel_rect, Point2D::new(x, y))
    }

    pub(in crate::widget_host) fn apply_code_selection_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
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

    pub(in crate::widget_host) fn apply_chat_input_selection_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
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

    pub(in crate::widget_host) fn apply_chat_text_selection_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
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
    pub(in crate::widget_host) fn try_patch_color_drag(&mut self, is_instance: bool) -> bool {
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

    /// Cursor-move dispatcher.
    ///
    /// A strictly ordered ladder. Each tier gets first refusal in paint
    /// Z-order: modals -> floating panels -> in-flight drags -> menus /
    /// popovers -> chrome -> late pointer drags -> base rails / canvas.
    /// Every tier helper returns `Option<bool>`: `None` means "not
    /// consumed, fall through to the next tier", `Some(dirty)` means
    /// "consumed, and this is the repaint signal".
    ///
    /// THE CALL ORDER BELOW *IS* THE BEHAVIOUR. The tier bodies live in
    /// the `cursor_move_*.rs` siblings purely to respect the per-file
    /// line cap; reordering these calls changes which surface owns a
    /// cursor move. See `crates/CLAUDE.md` for the canonical hit-test
    /// order before touching anything here.
    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        if let Some(changed) = self.update_agent_settings_touch_gesture(x, y) {
            return changed;
        }
        if let Some(changed) = self.update_touch_panel_gesture(x, y) {
            return changed;
        }
        // Session-switch owner rotation before the cursor_probe resolve below
        // stores the canonical build (mirrors the paint entry).
        self.rotate_chat_owner_if_session_changed();
        // Tier 1 — modals / top-most overlays own the cursor outright.
        if let Some(consumed) = self.cursor_move_modal_tiers(x, y) {
            return consumed;
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
        // Tier 2 — floating panels: design-md, component browser, icon
        // picker, dropdown overlays, variables preset menu + panel.
        if let Some(consumed) = self.cursor_move_floating_panel_tiers(
            x,
            y,
            chat_or_picker_owns_point,
            &mut higher_overlay_hover_changed,
        ) {
            return consumed;
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
        // Tier 3a — a live rail-resize drag. First of the pointer-capture
        // drags because it is the one whose pointer travels back over the
        // rail it is resizing, where the hover tiers below would otherwise
        // claim the move (see `cursor_move_panel_resize_tier`).
        if let Some(consumed) = self.cursor_move_panel_resize_tier(x) {
            return consumed;
        }
        // Tier 3 — in-flight property / text-selection / crop / node / pen
        // drags. Pointer capture: these own the cursor until release.
        if let Some(consumed) =
            self.cursor_move_active_drag_tiers(x, y, property_rect, &mut property_panel_probe)
        {
            return consumed;
        }
        // Suppress lower-overlay hover while a floating panel is on top.
        // VariablesPanel is below Chat in paint order; when the model picker
        // is visible, the earlier true-top-panel branches have already
        // returned and Variables must not make this gate look top-most.
        let over_topmost = !chat_or_picker_owns_point
            && self.over_topmost_panel(x, y, self.last_viewport_w, self.last_viewport_h);
        // Fold stale-hover clearing into the final repaint signal.
        let cleared = over_topmost && self.clear_lower_overlay_hover();
        let mut ctx = CursorMoveCtx {
            x,
            y,
            point,
            property_rect,
            chat_surface_owns_point,
            chat_or_picker_owns_point,
            over_topmost,
            upper_hover_changed: higher_overlay_hover_changed,
            cleared,
            property_panel_probe,
        };
        // Tier 4 — path-anchor menu, layer context menu, Git panel, and the
        // file / locale / shape dropdown hover.
        if let Some(consumed) = self.cursor_move_menu_tiers(&mut ctx) {
            return consumed;
        }
        // Tier 5 — property-panel popovers (image, export, effects,
        // compositing, interactions, padding / stroke, fonts).
        if let Some(consumed) = self.cursor_move_property_overlay_tiers(&mut ctx) {
            return consumed;
        }
        // Tier 6 — StatusBar, align toolbar, chat model picker.
        if let Some(consumed) = self.cursor_move_status_align_picker_tiers(&mut ctx) {
            return consumed;
        }
        // Tier 7 — TopBar traffic cluster + chrome-button hover wash.
        if let Some(consumed) = self.cursor_move_topbar_tiers(&mut ctx) {
            return consumed;
        }
        // Tier 7b — the left rail's slides tab. Above the base tier's
        // layer-row hover because when the slides tab is showing there
        // are no layer rows under the cursor to hover, and its tab row
        // sits over the tree in the other tab. A live row drag keeps
        // ownership wherever the pointer went, so a reorder does not
        // cancel the moment the cursor leaves the rail.
        let slides_hover =
            self.slides_panel_hover(x, y, self.last_viewport_w, self.last_viewport_h);
        if slides_hover.0 {
            return slides_hover.1;
        }
        // Tier 8 — single-shot chat probe. Deliberately NOT a consuming
        // tier: the late drags below must still run, so chat ownership is
        // returned and acted on by the base tier.
        let chat_owns_point = self.cursor_move_chat_hover(&mut ctx);
        // Tier 9 — late pointer-capture drags (rotate / handle / create /
        // path / arc / marquee / layer / panel + chat resize / pan).
        if let Some(consumed) = self.cursor_move_late_drag_tiers(x, y) {
            return consumed;
        }
        // Tier 10 — chat ownership, then toolbar / property / code /
        // canvas-hierarchy hover. Produces the event's repaint signal.
        self.cursor_move_base_tiers(&mut ctx, chat_owns_point)
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

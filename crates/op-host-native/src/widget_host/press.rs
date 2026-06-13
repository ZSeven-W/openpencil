//! Mouse-press dispatcher.
//!
//! `EditorState` is the host's source of truth. Canvas hit-tests run
//! against the layout-resolved `LayoutScene` (refreshed at the top
//! of `apply_press`); the resolved-scene node ids are wrapped into
//! op-editor-core `NodeId`s before feeding mutators. Press-helper
//! methods (`create_node_for_active_tool`,
//! `dispatch_agent_settings_press`) live in `press_helpers.rs`.

use super::helpers::{TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::{
    ChatDragState, ChatInputSelectionDragState, ChatResizeState, ChatTextSelectionDragState,
    CodeSelectionDragState, CreateDragState, DragState, HandleDragState, PanelResize,
    PanelResizeKind, RotateDragState, WidgetHostNative,
};
use op_editor_core::codegen::CodeSelection;
use op_editor_ui::widgets::{
    rotation_corner_at_point, selection_handle_at_point, AIChatHit, AIChatPlaceholder, LayoutCx,
    LocalePicker, PropertyPanel, Toolbar, TopBar, TopBarHit, Widget, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    // `pub(in crate::widget_host)` so the instance-panel tests can
    // drive the context-menu rows without simulating menu geometry.
    pub(in crate::widget_host) fn dispatch_layer_context_action(
        &mut self,
        action: op_editor_ui::widgets::layer_context_menu::LayerContextAction,
        target: op_editor_core::ui_draft::LayerContextTarget,
    ) {
        use op_editor_core::ui_draft::LayerContextTarget as T;
        use op_editor_ui::widgets::layer_context_menu::LayerContextAction as A;
        match (action, target) {
            (A::Duplicate, T::Layer(id)) => {
                // Act on the whole multi-selection when the right-clicked
                // row is part of it; otherwise retarget to just this row.
                if !self.editor_state.is_selected(&id) {
                    self.editor_state.set_single_selection(id);
                }
                self.editor_state.commit_history();
                let _ = self
                    .editor_state
                    .duplicate_selected(&mut self.next_node_id, 10.0);
            }
            (A::Delete, T::Layer(id)) => {
                // Keep the multi-selection so Delete removes every selected
                // layer, not just the right-clicked one.
                if !self.editor_state.is_selected(&id) {
                    self.editor_state.set_single_selection(id);
                }
                self.editor_state.commit_history();
                let _ = self.editor_state.delete_selected();
            }
            (A::GroupSelection, T::Layer(_)) => {
                let _ = self.apply_group();
            }
            // TS boolean rows act on the current selection and push
            // history explicitly (`layer-panel.tsx:389-407`); the
            // host's `apply_boolean_op` does both.
            (
                A::BooleanUnion | A::BooleanSubtract | A::BooleanIntersect | A::BooleanExclude,
                T::Layer(_),
            ) => {
                use op_editor_core::BooleanOp;
                let op = match action {
                    A::BooleanSubtract => BooleanOp::Subtract,
                    A::BooleanIntersect => BooleanOp::Intersect,
                    A::BooleanExclude => BooleanOp::Exclude,
                    _ => BooleanOp::Union,
                };
                let _ = self.apply_boolean_op(op);
            }
            (A::ToggleLock, T::Layer(id)) => {
                // TS toggleLock runs through mutateWithHistory
                // (document-store-node-actions.ts:176-188).
                self.with_doc_history(|s| s.toggle_node_locked(&id));
            }
            (A::ToggleVisibility, T::Layer(id)) => {
                // TS toggleVisibility runs through mutateWithHistory
                // (document-store-node-actions.ts:162-174).
                self.with_doc_history(|s| s.toggle_node_hidden(&id));
            }
            (A::CreateComponent, T::Layer(id)) => {
                let _ = self.editor_state.create_component_from_node_name(&id);
            }
            (A::DetachComponent | A::DetachInstance, T::Layer(id)) => {
                // Reusable component sheds its flag; a Ref instance
                // materializes into an independent subtree (#22).
                let _ = self.editor_state.detach_component(&id);
            }
            // Page CRUD pushes history in TS
            // (document-store-pages.ts:19-121) — snapshot-before-
            // mutate, skipped when the guard rejects the op.
            (A::DuplicatePage, T::Page(idx)) => {
                self.with_doc_history(|s| s.duplicate_page(idx).is_some());
            }
            (A::MovePageUp, T::Page(idx)) => {
                self.with_doc_history(|s| s.move_page_up(idx));
            }
            (A::MovePageDown, T::Page(idx)) => {
                self.with_doc_history(|s| s.move_page_down(idx));
            }
            (A::DeletePage, T::Page(idx)) => {
                self.with_doc_history(|s| s.remove_page(idx));
            }
            (A::RenamePage, T::Page(idx)) => {
                if self.editor_state.start_rename_page(idx) {
                    if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
                        rename.input.touch(self.now_ms);
                    }
                }
            }
            (A::RenameLayer, T::Layer(id)) => {
                if self.editor_state.start_rename_layer(id) {
                    if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
                        rename.input.touch(self.now_ms);
                    }
                }
            }
            _ => {} // mismatched action/target — no-op
        }
        self.mark_dirty();
    }

    /// Right-click handler — opens the LayerPanel context menu on
    /// a layer row OR page row.
    pub fn apply_right_press(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        // Codex stop-gate: right-click outside the variables panel
        // must commit any pending row focus first.
        self.commit_variable_row_focus_if_any();
        // Any top-most floating panel swallows a right-click on its
        // rect — no context menu opens under them.
        if self.over_topmost_panel(x, y, viewport_w, viewport_h) {
            return true;
        }
        // Select-tool right-click on a path anchor / handle opens the
        // point-type menu (`pen_press.rs`).
        if self.try_open_path_anchor_menu(x, y, viewport_w, viewport_h) {
            return true;
        }
        if !self.editor_state.editor_ui.sidebar_open {
            // No layer panel to hit — the right press is blank chrome;
            // blur inputs like the sidebar-open fall-through below.
            return self.blur_text_inputs_on_blank_press();
        }
        use op_editor_core::editor_ui_state::LayerContextMenuState;
        use op_editor_core::ui_draft::LayerContextTarget;
        use op_editor_ui::widgets::{LayerPanel, LayerPanelHit};
        let layer_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.editor_state.editor_ui.layer_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        let panel = LayerPanel::from_editor(&self.editor_state);
        match panel.hit_test(layer_rect, Point2D::new(x, y)) {
            Some(LayerPanelHit::Layer(id)) => {
                let ec_id = id.clone();
                if !(self.editor_state.is_selected(&ec_id)
                    && self.editor_state.selection_count() > 1)
                {
                    self.editor_state.set_single_selection(ec_id.clone());
                }
                self.editor_state.editor_ui.layer_context_menu = Some(LayerContextMenuState {
                    target: LayerContextTarget::Layer(ec_id),
                    anchor_x: x,
                    anchor_y: y,
                    menu: Default::default(),
                });
                return true;
            }
            Some(LayerPanelHit::Page(idx)) => {
                self.editor_state.editor_ui.layer_context_menu = Some(LayerContextMenuState {
                    target: LayerContextTarget::Page(idx),
                    anchor_x: x,
                    anchor_y: y,
                    menu: Default::default(),
                });
                return true;
            }
            _ => {}
        }
        if self
            .editor_state
            .editor_ui
            .layer_context_menu
            .take()
            .is_some()
        {
            return true;
        }
        // Right-press on blank chrome — blur inputs like a left press.
        self.blur_text_inputs_on_blank_press()
    }

    /// Mouse-press handler. Returns whether anything visible changed.
    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        // Blur-commit rename + text-edit; track to repaint on click.
        let rename_committed =
            self.editor_state.ui.layer_rename.is_some() && self.editor_state.rename_commit();
        let text_edit_was_active = self.editor_state.ui.text_editing.is_some();
        // EXCEPTION to commit-on-press: a press INSIDE the edited
        // text node places the caret instead of committing (TS
        // textarea parity — a click inside the overlay moves the
        // caret; only an outside click blurs + commits). The probe
        // returns `None` whenever any overlay / modal owns the point,
        // so those presses still commit + route normally.
        let text_edit_caret_press =
            self.text_edit_press_offset(x, y, viewport_width, viewport_height);
        let text_edit_committed =
            text_edit_caret_press.is_none() && self.editor_state.text_edit_commit();
        if rename_committed || text_edit_committed {
            self.mark_dirty();
        }
        if let Some(offset) = text_edit_caret_press {
            self.place_text_edit_caret(offset);
            return true;
        }
        // Floating Design-MD panel — painted top-most (`paint.rs`
        // §12), so it hit-tests first: a click on its rect is the
        // panel's before any lower layer can claim it (dispatch in
        // `design_md_press.rs`).
        if self.dispatch_design_md_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        if self.dispatch_icon_picker_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        // Floating Component-Browser panel — painted just under the
        // Design-MD panel; hit-tests right after it.
        if self.dispatch_component_browser_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        if self.editor_state.editor_ui.agent_settings_open
            && self.dispatch_agent_settings_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }
        // 0-color. Color picker overlay — top-most when open
        //          (dispatch in `color_picker_press.rs`).
        if self.dispatch_color_picker_press(x, y, viewport_width, viewport_height) {
            return true;
        }

        // 0-git-modal. While a Git ready-state header popover (branch
        // picker / overflow / its subview) is open it is MODAL: route
        // every press to the Git panel first so a click anywhere
        // outside the popover dismisses it. The popover extends past the
        // panel rect, so the rail / top-bar / canvas blocks below cannot
        // be relied on to forward an outside click — `hit_test` returns
        // `DismissPopover` for any outside-popover point.
        //
        // IMPORTANT: fall through on `false`. If the popover flags are
        // stale (set while the panel was ready, but it has since left
        // the ready state — gone dirty / merging / loading, so
        // `hit_test`'s ready-gated popover capture no longer fires),
        // `dispatch_git_panel_press` returns `false` for an outside
        // click. Returning that directly would dead-end EVERY press; so
        // only consume when it actually handled the click.
        {
            let gp = &self.editor_state.editor_ui.git_panel;
            if gp.open
                && (gp.branch_picker_open || gp.overflow_open)
                && self.dispatch_git_panel_press(x, y, viewport_width, viewport_height)
            {
                return true;
            }
        }

        // StatusBar controls — Search frames content, `[-]` / `[+]`
        // step the zoom. Floating bottom-right, so it hit-tests above
        // the canvas / toolbar.
        if let Some(r) = self.status_bar_rect(viewport_width, viewport_height) {
            use op_editor_core::StatusBarButton;
            let bar = op_editor_ui::widgets::StatusBar::for_editor(&self.editor_state);
            if let Some(btn) = bar.control_at(r, Point2D::new(x, y)) {
                self.editor_state.editor_ui.pressed_button =
                    Some(op_editor_core::ButtonPressTarget::StatusBar(btn));
                match btn {
                    StatusBarButton::Search => self.zoom_to_fit(viewport_width, viewport_height),
                    StatusBarButton::ZoomOut => {
                        self.status_bar_zoom(false, viewport_width, viewport_height)
                    }
                    StatusBarButton::ZoomIn => {
                        self.status_bar_zoom(true, viewport_width, viewport_height)
                    }
                }
                self.mark_dirty();
                return true;
            }
        }

        // Path-anchor context menu — a row hit dispatches + consumes;
        // an outside press closes it and FALLS THROUGH (TS parity:
        // the canvas mousedown still routes). `pen_press.rs`.
        if self.dispatch_path_anchor_menu_press(x, y) {
            return true;
        }

        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::{LayerContextMenu, MenuHit};
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            match menu.hit(Point2D::new(x, y)) {
                MenuHit::Row(_) => {
                    if let Some(action) = menu.hit_test(Point2D::new(x, y)) {
                        self.dispatch_layer_context_action(action, state.target.clone());
                        self.editor_state.editor_ui.layer_context_menu = None;
                        self.mark_dirty();
                    }
                    return true;
                }
                MenuHit::Inside => {
                    return true;
                }
                MenuHit::Outside => {}
            }
            // Dismissing the menu on a miss is a blank press — blur
            // every text input along with it.
            self.blur_text_inputs_on_blank_press();
            self.editor_state.editor_ui.layer_context_menu = None;
            self.mark_dirty();
            return true;
        }
        // 0aa. Commit-on-blur for property-panel inputs +
        // variable-row inline editor.
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.property_focus.is_some() {
            let property_left = if self.editor_state.property_panel_visible() {
                viewport_width - self.editor_state.editor_ui.property_panel_width
            } else {
                viewport_width
            };
            if x < property_left {
                self.commit_property_focus_if_any();
            }
        }

        // The floating Git panel paints on top of the right-rail
        // panels (see `paint.rs` §8.2) — and in diff mode it widens
        // to 620 px, which overlaps the property / variables rail
        // (and its resize gutter) on a narrow window. Hit-test order
        // must mirror paint Z-order: a click inside the Git-panel
        // rect belongs to the Git panel (block 0.9 below), so every
        // rail block in between skips a click it would otherwise own.
        let in_git_panel = self
            .git_panel_outer_rect(viewport_width, viewport_height)
            .is_some_and(|r| (r).contains(Point2D::new(x, y)));

        // 0z. Panel-resize gutter — ±4 px from rail edges.
        if y >= TOP_BAR_HEIGHT && !in_git_panel {
            if let Some(kind) = self.panel_resize_hover(x, y, viewport_width) {
                let start_width = match kind {
                    PanelResizeKind::LayerRight => self.editor_state.editor_ui.layer_panel_width,
                    PanelResizeKind::PropertyLeft => {
                        self.editor_state.editor_ui.property_panel_width
                    }
                };
                self.panel_resize = Some(PanelResize {
                    kind,
                    start_x: x,
                    start_width,
                });
                return true;
            }
        }

        // 0ab. Shape picker overlay.
        if self.dispatch_shape_picker_press(x, y, viewport_width, viewport_height) {
            return true;
        }

        if self.editor_state.editor_ui.file_menu_open {
            self.dispatch_file_menu_press(x, y, viewport_width);
            return true;
        }
        if self.editor_state.editor_ui.export_dialog_open {
            self.dispatch_export_dialog_press(x, y, viewport_width, viewport_height);
            return true;
        }
        if self.editor_state.editor_ui.figma_import_open {
            self.dispatch_figma_import_press(x, y, viewport_width, viewport_height);
            return true;
        }

        // 0a. Locale picker overlay — top-most when open.
        if self.editor_state.editor_ui.locale_picker.open {
            self.refresh_layout_scene();
            let panel_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            match picker.hit_popup(panel_rect, Point2D::new(x, y)) {
                op_editor_ui::widgets::locale_picker::SelectHit::Row(idx) => {
                    if let Some(locale) = LocalePicker::locale_at(idx) {
                        self.editor_state.editor_ui.locale = locale;
                    }
                    self.editor_state.editor_ui.locale_picker.open = false;
                    self.editor_state.editor_ui.locale_picker.hover = None;
                    self.mark_dirty();
                    return true;
                }
                op_editor_ui::widgets::locale_picker::SelectHit::Inside => return true,
                op_editor_ui::widgets::locale_picker::SelectHit::Outside => {}
            }
            // Silent outside-close is a blank press — blur inputs too.
            self.blur_text_inputs_on_blank_press();
            self.editor_state.editor_ui.locale_picker.open = false;
            self.editor_state.editor_ui.locale_picker.hover = None;
            self.mark_dirty();
            return true;
        }

        // 0b. TopBar — sidebar toggle button + theme + locale picker.
        self.refresh_layout_scene();
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
        if let Some(hit) = top_bar.hit_test(top_bar_rect, Point2D::new(x, y)) {
            let pressed = op_editor_ui::widgets::editor_state_ext::topbar_button_hover(hit);
            self.editor_state.editor_ui.pressed_button =
                Some(op_editor_core::ButtonPressTarget::TopBar(pressed));
            match hit {
                TopBarHit::ToggleSidebar => {
                    let v = &mut self.editor_state.editor_ui.sidebar_open;
                    *v = !*v;
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::ToggleTheme => {
                    self.editor_state.editor_ui.theme_mode =
                        self.editor_state.editor_ui.theme_mode.flipped();
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::ToggleLocale => {
                    let picker = &mut self.editor_state.editor_ui.locale_picker;
                    picker.open = !picker.open;
                    picker.hover = None;
                    picker.pressed = None;
                    if picker.open {
                        picker.scroll.offset = 0.0;
                    }
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::OpenAgentSettings => {
                    self.editor_state.editor_ui.agent_settings_open = true;
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::ToggleFileMenu => {
                    self.editor_state.editor_ui.file_menu_open ^= true;
                    self.editor_state.editor_ui.file_menu.hover = None;
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::ToggleFullscreen => {
                    // The widget host doesn't own the winit window — raise
                    // an intent the desktop runner consumes next frame.
                    self.editor_state.editor_ui.pending_fullscreen_toggle = true;
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::OpenFigmaImport => {
                    self.editor_state.editor_ui.figma_import_open = true;
                    self.mark_dirty();
                    return true;
                }
                TopBarHit::ToggleGitPanel => {
                    // Mirror `main.rs` A::ToggleGitPanel bookkeeping; the
                    // binary's per-frame `if git_panel.open { refresh }`
                    // performs the actual repo scan.
                    let panel = &mut self.editor_state.editor_ui.git_panel;
                    let opening = !panel.open;
                    panel.open = opening;
                    if opening {
                        panel.loading = true;
                    } else {
                        panel.defocus_commit_input(self.now_ms);
                        panel.remote_focused = false;
                        panel.https_focused = false;
                        panel.diff = None;
                        panel.merge_resolve = None;
                        // Close the clone wizard synchronously so a rapid
                        // close→reopen can't resurface a stale form before
                        // the host's `poll_git_clone_job` runs (it then
                        // abandons any in-flight job — `cloning` form gone).
                        panel.clone_form = None;
                    }
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if (top_bar_rect).contains(Point2D::new(x, y)) {
            // Other top-bar gaps eat clicks but don't act — still a
            // blank press, so every text input blurs.
            let blurred = self.blur_text_inputs_on_blank_press();
            return blurred || rename_committed || text_edit_committed;
        }

        // 0c0a. Image-fill popover — outside-click dismiss.
        if !in_git_panel
            && self.dismiss_image_fill_popover_on_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }

        // 0c0. Fill-type picker — outside-click dismiss.
        if self.editor_state.editor_ui.fill_type_picker.open && !in_git_panel {
            self.refresh_layout_scene();
            if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        viewport_width - self.editor_state.editor_ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                match panel.fill_type_picker_hit(property_rect, Point2D::new(x, y)) {
                    op_editor_ui::widgets::property_panel_fill::SelectHit::Row(idx) => {
                        if let Some(fill_type) =
                            op_editor_ui::widgets::property_panel_fill::fill_type_at(idx)
                        {
                            self.apply_property_action(
                                op_editor_ui::widgets::PropertyPanelAction::SetFillType(fill_type),
                            );
                            return true;
                        }
                    }
                    op_editor_ui::widgets::property_panel_fill::SelectHit::Inside => return true,
                    op_editor_ui::widgets::property_panel_fill::SelectHit::Outside => {}
                }
            }
            self.editor_state.editor_ui.close_fill_type_picker();
            self.mark_dirty();
            return true;
        }

        // 0c0a0. Fill/stroke colour-variable picker — outside-click dismiss.
        if self
            .editor_state
            .editor_ui
            .property_color_variable_picker_open
            .is_some()
            && !in_git_panel
        {
            self.refresh_layout_scene();
            if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        viewport_width - self.editor_state.editor_ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                    if matches!(
                        action,
                        op_editor_ui::widgets::PropertyPanelAction::ToggleColorVariablePicker(_)
                            | op_editor_ui::widgets::PropertyPanelAction::BindColorVariable { .. }
                            | op_editor_ui::widgets::PropertyPanelAction::UnbindColorVariable(_)
                    ) {
                        self.apply_property_action(action);
                        return true;
                    }
                }
            }
            self.editor_state
                .editor_ui
                .property_color_variable_picker_open = None;
            self.mark_dirty();
            return true;
        }

        // 0c0a0a. Image-node Search / Generate popovers — overlay
        // controls win; outside clicks dismiss.
        if !in_git_panel
            && self.dismiss_image_popovers_on_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }

        // 0c0a1. Text font-family picker — outside-click dismiss.
        if !in_git_panel && self.dismiss_font_picker_on_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }

        // 0c0a2. Text font-weight picker — outside-click dismiss.
        if !in_git_panel
            && self.dismiss_font_weight_picker_on_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }

        // 0c0a3. Padding mode-selector popover — outside-click dismiss.
        if !in_git_panel
            && self.dismiss_padding_mode_popover_on_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }

        // 0c0b. Export scale / format inline select popup —
        //       outside-click dismiss (`property_dispatch.rs`).
        if !in_git_panel
            && self.dismiss_export_picker_on_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }

        // 0b0. Theme-preset dropdown (#20) — floats above the
        //       variables panel, so it must win before the panel's
        //       stub menu mapping (variables_preset_press.rs).
        if !in_git_panel
            && self.dispatch_variables_preset_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }

        // 0b1. VariablesPanel — tested before PropertyPanel.
        if !in_git_panel
            && self.dispatch_variables_panel_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }

        // 0c. PropertyPanel input row.
        self.refresh_layout_scene();
        if let Some(panel) =
            PropertyPanel::for_selection(&self.editor_state).filter(|_| !in_git_panel)
        {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            if let Some(anchor) = self.code_text_offset_at_screen(x, y) {
                self.commit_property_focus_if_any();
                self.editor_state.codegen.code_selection = Some(CodeSelection {
                    anchor,
                    focus: anchor,
                });
                self.code_selection_drag = Some(CodeSelectionDragState { anchor });
                self.editor_state.chat.transcript_selection = None;
                self.editor_state.codegen.framework_hover = None;
                self.editor_state.codegen.action_hover = None;
                self.editor_state.chat.focused = false;
                self.mark_dirty();
                return true;
            }
            // Button / checkbox click first (flex modes + size flags).
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                self.commit_property_focus_if_any();
                if let op_editor_ui::widgets::PropertyPanelAction::OpenColorPicker(target) = action
                {
                    let _ = self
                        .editor_state
                        .open_color_picker(super::press_helpers::color_target(target), y);
                    self.mark_dirty();
                } else if let op_editor_ui::widgets::PropertyPanelAction::OpenEffectColorPicker(
                    index,
                ) = action
                {
                    // Anchor the picker at the clicked swatch so it
                    // pops next to the row, not at the panel top.
                    let _ = self.editor_state.open_color_picker(
                        op_editor_core::ui_draft::ColorTarget::EffectColor(index),
                        y,
                    );
                    self.mark_dirty();
                } else {
                    self.apply_property_action(action);
                }
                return true;
            }
            if let Some(focus) = panel.hit_test(property_rect, Point2D::new(x, y)) {
                self.commit_property_focus_if_any();
                let initial = super::press_helpers::property_focus_initial(focus, &panel);
                // shell-core `PropertyFocus` → op-editor-core.
                self.editor_state.ui.property_focus = Some(focus);
                self.editor_state
                    .ui
                    .property_input
                    .set_text(initial.clone());
                self.editor_state.ui.property_input.touch(self.now_ms);
                self.editor_state.ui.property_input_draft = initial;
                // Caret starts at the end of the seeded draft.
                self.editor_state.ui.property_caret_pos =
                    self.editor_state.ui.property_input_draft.len();
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.chat.focused = false;
                self.mark_dirty();
                return true;
            }
        }

        // 0.9. Floating Git panel — status + interactive actions
        //      (dispatch in `git_press.rs`).
        if self.dispatch_git_panel_press(x, y, viewport_width, viewport_height) {
            return true;
        }

        // 1. AI chat panel — sits on top of the toolbar in paint
        //    order. DragHandle starts a chat drag; other AI hits
        //    defer to apply_click.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel = AIChatPlaceholder::from_editor(&self.editor_state);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                if let AIChatHit::Resize(edge) = hit {
                    self.chat_resize = Some(ChatResizeState {
                        edge,
                        start_x: x,
                        start_y: y,
                        start_rect: chat_rect,
                    });
                    self.editor_state.chat.focused = false;
                    self.mark_dirty();
                    return true;
                }
                if let AIChatHit::SelectInputText(anchor) = hit {
                    self.chat_input_selection_drag = Some(ChatInputSelectionDragState { anchor });
                    self.editor_state.chat.focused = true;
                    self.editor_state.chat.set_input_caret(anchor, self.now_ms);
                    self.editor_state.chat.transcript_selection = None;
                    self.editor_state.codegen.code_selection = None;
                    self.mark_dirty();
                    return true;
                }
                if let AIChatHit::SelectTranscriptText(message_index, anchor) = hit {
                    self.chat_text_selection_drag = Some(ChatTextSelectionDragState {
                        message_index,
                        anchor,
                    });
                    self.editor_state.chat.transcript_selection =
                        Some(op_editor_core::chat::ChatTranscriptSelection {
                            message_index,
                            anchor,
                            focus: anchor,
                        });
                    self.editor_state.codegen.code_selection = None;
                    self.editor_state.chat.focused = false;
                    self.mark_dirty();
                    return true;
                }
                if matches!(hit, AIChatHit::DragHandle) {
                    self.chat_drag = Some(ChatDragState {
                        grab_dx: x - chat_rect.origin.x,
                        grab_dy: y - chat_rect.origin.y,
                        pos_x: chat_rect.origin.x,
                        pos_y: chat_rect.origin.y,
                    });
                    self.editor_state.chat.focused = false;
                    self.mark_dirty();
                    return true;
                }
                let _ = self.apply_click(x, y, viewport_width, viewport_height);
                return true;
            }
        }

        // 2. Toolbar — second-highest overlay.
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let toolbar = Toolbar::for_editor(&self.editor_state);
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
        if (toolbar_rect).contains(Point2D::new(x, y)) {
            if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
                match hit {
                    op_editor_ui::widgets::ToolbarHit::Tool(tool) => {
                        // TS onToolChange DISCARDS an in-progress pen
                        // path on tool switch (`skia-pen-tool.ts:38-50`).
                        self.cancel_pen_on_tool_switch(tool);
                        self.editor_state.tool = tool;
                        self.editor_state.editor_ui.shape_picker.open = false;
                        self.editor_state.editor_ui.shape_picker.hover = None;
                        self.editor_state.editor_ui.shape_picker.pressed = None;
                        self.mark_dirty();
                        return true;
                    }
                    op_editor_ui::widgets::ToolbarHit::Action(action) => {
                        self.editor_state.editor_ui.shape_picker.open = false;
                        self.editor_state.editor_ui.shape_picker.hover = None;
                        self.editor_state.editor_ui.shape_picker.pressed = None;
                        let acted = self.dispatch_toolbar_action(action);
                        return acted || rename_committed || text_edit_committed;
                    }
                    op_editor_ui::widgets::ToolbarHit::ToggleShapePicker => {
                        let picker = &mut self.editor_state.editor_ui.shape_picker;
                        picker.open = !picker.open;
                        picker.hover = None;
                        picker.pressed = None;
                        if picker.open {
                            picker.scroll.offset = 0.0;
                        }
                        self.mark_dirty();
                        return true;
                    }
                }
            }
            // Toolbar padding / gaps eat the click — blank press.
            let blurred = self.blur_text_inputs_on_blank_press();
            return blurred || rename_committed || text_edit_committed;
        }

        if let Some(hit) = self.selection_toolbar_hit(x, y, viewport_width, viewport_height) {
            match hit {
                op_editor_ui::widgets::AlignToolbarHit::Align(action) => {
                    self.editor_state.align_selected(action);
                    self.mark_dirty();
                }
                op_editor_ui::widgets::AlignToolbarHit::Boolean(op) => {
                    let _ = self.apply_boolean_op(op);
                }
            }
            return true;
        }
        // 3. apply_click — LayerPanel + chat-defocus. Peek the
        //    LayerPanel hit-test for a drag-to-reorder candidate.
        if self.editor_state.editor_ui.sidebar_open {
            use op_editor_ui::widgets::{LayerPanel, LayerPanelHit};
            let layer_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    self.editor_state.editor_ui.layer_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let panel = LayerPanel::from_editor(&self.editor_state);
            if let Some(LayerPanelHit::Layer(node_id)) =
                panel.hit_test(layer_rect, Point2D::new(x, y))
            {
                self.layer_drag = Some(crate::widget_host::LayerDragState {
                    source: node_id,
                    start_y: y,
                    current_x: x,
                    current_y: y,
                    active: false,
                });
            }
        }
        let consumed = self.apply_click(x, y, viewport_width, viewport_height);
        if consumed {
            return true;
        }

        // 4. Canvas click — branch on the active tool.
        if self.over_canvas(x, y, viewport_width, viewport_height) {
            use op_editor_core::Tool;
            // Any canvas press blurs the chrome text inputs first —
            // node hit or not, the pointer left every input (DOM
            // parity: mousedown anywhere outside an input blurs it).
            let blurred = self.blur_text_inputs_on_blank_press();
            // A press on the canvas while the Git panel is open dismisses
            // it — the panel is a popover (TS parity: clicking off a
            // popover closes it), and a click inside it was already
            // consumed at §0.9, so anything reaching here is genuinely
            // outside. Swallow the press so the dismiss doesn't also
            // select / pan / start drawing.
            if self.editor_state.editor_ui.git_panel.open {
                self.close_git_panel();
                self.mark_dirty();
                return true;
            }
            self.refresh_layout_scene();
            if matches!(self.editor_state.tool, Tool::Hand) || self.space_pan {
                self.drag = Some(DragState {
                    last_x: x,
                    last_y: y,
                });
                return blurred || rename_committed || text_edit_committed;
            }
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_width, viewport_height);
            let canvas_rect = Rect {
                origin: Point2D::new(cx0, cy0),
                size: Point2D::new(cw, ch),
            };
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc_point = self.editor_state.viewport.to_document(canvas_local);

            if matches!(self.editor_state.tool, Tool::Select) {
                // Path controls hit-test FIRST (TS handleSelectMouseDown,
                // skia-interaction.ts:277-306) — before arc / resize /
                // rotation, so anchor dots near a corner stay grabbable.
                if self.try_path_anchor_press(x, y, viewport_width, viewport_height) {
                    return true;
                }
                // The selected anchor's resolved scene node — shared
                // by the handle-drag + rotation-drag branches below.
                let selected_anchor = self.editor_state.selection.anchor.as_str().to_string();
                // Arc handles take priority over the 8 resize handles —
                // the sweep handle can overlap the right-mid resize grip.
                if let Some((node_id, handle)) =
                    self.arc_handle_hit(x, y, viewport_width, viewport_height)
                {
                    let pre = self.editor_state.snapshot_for_history();
                    self.arc_handle_drag = Some(super::ArcHandleDragState {
                        node_id: op_editor_core::NodeId::new(&node_id),
                        handle,
                        start_doc: doc_point,
                        moved: false,
                        pre_drag_snapshot: pre,
                    });
                    return true;
                }
                if let Some(handle) = selection_handle_at_point(
                    canvas_rect,
                    &self.layout_scene,
                    &self.editor_state,
                    Point2D::new(x, y),
                ) {
                    if let Some(node) = self
                        .layout_scene
                        .active_page()
                        .and_then(|p| p.find(&selected_anchor))
                    {
                        // Only handle-drag on nodes with real bounds.
                        let raw = node.bounds;
                        if raw.size.x > 0.0 || raw.size.y > 0.0 {
                            self.editor_state.commit_history();
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
                if rotation_corner_at_point(
                    canvas_rect,
                    &self.layout_scene,
                    &self.editor_state,
                    Point2D::new(x, y),
                )
                .is_some()
                {
                    if let Some(node) = self
                        .layout_scene
                        .active_page()
                        .and_then(|p| p.find(&selected_anchor))
                    {
                        let bounds = node.aggregate_bounds();
                        let cx_doc = bounds.origin.x + bounds.size.x / 2.0;
                        let cy_doc = bounds.origin.y + bounds.size.y / 2.0;
                        let center_screen_x = canvas_rect.origin.x
                            + self.editor_state.viewport.pan_x
                            + cx_doc * self.editor_state.viewport.zoom;
                        let center_screen_y = canvas_rect.origin.y
                            + self.editor_state.viewport.pan_y
                            + cy_doc * self.editor_state.viewport.zoom;
                        let start_cursor_angle = (y - center_screen_y).atan2(x - center_screen_x);
                        let start_rotation = node.rotation;
                        self.editor_state.commit_history();
                        self.rotate_drag = Some(RotateDragState {
                            center_screen_x,
                            center_screen_y,
                            start_cursor_angle,
                            start_rotation,
                        });
                        return true;
                    }
                }
                if let Some(node_id) = self
                    .layout_scene
                    .node_at_doc_point(doc_point, self.editor_state.viewport.zoom)
                {
                    // Selection promotion / enter-group / drag start —
                    // see `canvas_select_drag.rs`.
                    let ec_id = op_editor_core::NodeId::new(&node_id);
                    return self.apply_canvas_node_press(ec_id, x, y, text_edit_was_active);
                }
                // Empty canvas press — start a marquee.
                let cleared_now = if !self.shift_held {
                    let was_set = !self.editor_state.selection.set.is_empty();
                    if was_set {
                        self.editor_state.clear_selection();
                    }
                    // Clicking blank canvas steps out of the entered
                    // container (clearing-exits rule).
                    self.editor_state.sync_entered_container_with_selection();
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
                return cleared_now || blurred;
            }

            // Pen tool: anchor edit / author (`pen_press.rs`).
            if matches!(self.editor_state.tool, Tool::Pen) {
                return self.apply_pen_tool_press(x, y, viewport_width, viewport_height);
            }

            // Shape / Frame / Text tool: spawn a new node + drag.
            let pre_create = self.editor_state.snapshot_for_history();
            if let Some(node_id) = self.create_node_for_active_tool(doc_point) {
                self.editor_state.history_push_past(pre_create);
                self.editor_state.set_single_selection(node_id);
                self.create_drag = Some(CreateDragState {
                    start_doc_x: doc_point.x,
                    start_doc_y: doc_point.y,
                });
                self.mark_dirty();
                return true;
            }

            // Tool didn't accept this point — fall back to pan.
            self.drag = Some(DragState {
                last_x: x,
                last_y: y,
            });
            return blurred || rename_committed || text_edit_committed;
        }
        // Final fall-through — the press hit no interactive chrome
        // (panel-rail gaps, property-panel padding, …): blank press.
        let blurred = self.blur_text_inputs_on_blank_press();
        blurred || rename_committed || text_edit_committed
    }
}

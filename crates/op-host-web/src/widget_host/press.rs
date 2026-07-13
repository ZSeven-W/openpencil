//! Web press handlers split from `widget_host.rs`; mirrors the
//! native press/click split and keeps `EditorState` as source of truth.
//! `apply_click` lives in `click.rs`, the StatusBar / overlay rect
//! helpers in `overlay_rects.rs`, and the per-overlay press
//! dispatchers in their own sibling modules (mirroring the native
//! host's layout) so this file stays under the 800-line cap.
use op_editor_ui::widgets::{
    AIChatHit, AIChatPlaceholder, CanvasViewport, LayerPanel, LayerPanelHit, LocalePicker,
    PropertyPanel, Toolbar, TopBarHit, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

use super::{
    ChatDragState, ChatInputSelectionDragState, ChatTextSelectionDragState, CodeSelectionDragState,
    DragState, LayerDragState, MarqueeDragState, WidgetHost,
};
use op_editor_core::codegen::CodeSelection;

impl WidgetHost {
    /// Right-click handler — opens the LayerPanel context menu on
    /// a layer or page row.
    pub fn apply_right_press(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        self.commit_variable_row_focus_if_any();
        if self.over_topmost_panel(x, y, viewport_w, viewport_h) {
            return true;
        }
        if self.try_open_path_anchor_menu(x, y, viewport_w, viewport_h) {
            return true;
        }
        if !self.editor_state.editor_ui.sidebar_open {
            return self.blur_text_inputs_on_blank_press();
        }
        use op_editor_core::editor_ui_state::LayerContextMenuState;
        use op_editor_core::ui_draft::LayerContextTarget;
        self.refresh_layout_scene();
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = LayerPanel::from_editor(&self.editor_state);
        match panel.hit_test(layer_rect, Point2D::new(x, y)) {
            Some(LayerPanelHit::Layer(id)) => {
                let ec_id = id.clone();
                // Right-clicking a row that's part of a multi-selection
                // keeps the whole selection (so context-menu Delete /
                // Duplicate act on every selected layer); right-clicking
                // outside the selection retargets to just that row.
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
                self.mark_dirty();
                return true;
            }
            Some(LayerPanelHit::Page(idx)) => {
                self.editor_state.editor_ui.layer_context_menu = Some(LayerContextMenuState {
                    target: LayerContextTarget::Page(idx),
                    anchor_x: x,
                    anchor_y: y,
                    menu: Default::default(),
                });
                self.mark_dirty();
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
            self.mark_dirty();
            return true;
        }
        self.blur_text_inputs_on_blank_press()
    }

    fn dispatch_layer_context_action(
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
                self.with_doc_history(|s| s.toggle_node_locked(&id));
            }
            (A::ToggleVisibility, T::Layer(id)) => {
                self.with_doc_history(|s| s.toggle_node_hidden(&id));
            }
            (A::CreateComponent, T::Layer(id)) => {
                let _ = self.editor_state.create_component_from_node_name(&id);
            }
            (A::DetachComponent | A::DetachInstance, T::Layer(id)) => {
                let _ = self.editor_state.detach_component(&id);
            }
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
            _ => {}
        }
        self.mark_dirty();
    }

    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // Cache the viewport dims so `apply_cursor_move(x, y)` (no
        // viewport params in signature) can rebuild the canvas region
        // for the floating align toolbar's hover sync. Mirrors the
        // native host's `last_viewport_w` / `_h` cache.
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        // Refresh the derived paint doc once up front — every hit-test
        // below reads `&self.layout_scene`, so it must be current.
        self.refresh_layout_scene();
        // 0-pre. Commit any in-flight rename + canvas text-edit on
        // first press anywhere. Tracked so the final return reports
        // the visible change.
        let rename_committed =
            self.editor_state.ui.layer_rename.is_some() && self.editor_state.rename_commit();
        let text_edit_was_active = self.editor_state.ui.text_editing.is_some();
        let text_edit_committed = self.editor_state.text_edit_commit();
        if rename_committed || text_edit_committed {
            self.mark_dirty();
        }
        // Floating Design-MD panel — painted top-most, so it
        // hit-tests first: a click on its rect is the panel's before
        // any lower layer can claim it (mirrors native press order).
        if self.dispatch_design_md_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        if self.dispatch_icon_picker_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        // Floating Component-Browser panel — painted just under the
        // Design-MD panel; hit-tests right after it. A consumed press
        // may queue an insert — drain it against this viewport (web
        // has no per-frame runner drain like the desktop loop).
        if self.dispatch_component_browser_press(x, y, viewport_width, viewport_height) {
            let _ = self.drain_component_browser_insert(viewport_width, viewport_height);
            return true;
        }
        if self.editor_state.editor_ui.agent_settings_open
            && self.dispatch_agent_settings_press(x, y, viewport_width, viewport_height)
        {
            return true;
        }
        // Colour-picker overlay — top-most when open. Falls through
        // on an outside click (the picker closes as a side effect).
        if self.dispatch_color_picker_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        // StatusBar controls — Search frames content, `[-]` / `[+]`
        // step the zoom (floating bottom-right; hit-tests above the
        // canvas).
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
        if self.dispatch_path_anchor_menu_press(x, y) {
            return true;
        }
        // 0. Layer context menu — top-most overlay when open.
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::{LayerContextMenu, MenuHit};
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            match menu.hit(Point2D::new(x, y)) {
                MenuHit::Row(_) => {
                    if let Some(action) = menu.hit_test(Point2D::new(x, y)) {
                        self.dispatch_layer_context_action(action, state.target);
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
        // 0a. Locale picker overlay — top-most when open. Row hit
        //     sets locale + closes; ANY other hit (including the
        //     Globe button itself) closes the picker AND swallows
        //     the click so the same press doesn't re-toggle open.
        if self.editor_state.editor_ui.locale_picker.open {
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

        // 0aa. Shape picker overlay (native press order: before the
        //      file-menu / export / figma modal blocks).
        if self.dispatch_shape_picker_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        // 0aa. Theme-preset dropdown (#20) — runs BEFORE the panel dispatch so
        //      the functional menu rows win over the panel's stub
        //      TogglePresetMenu mapping (native parity).
        if self.dispatch_variables_preset_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        // 0ab. Floating VariablesPanel — full interactive grid
        //      mirroring the native host (#21). A press inside the
        //      panel rect dispatches; outside presses fall through to
        //      the normal layers (the panel floats, it isn't modal).
        if self.dispatch_variables_panel_press(x, y, viewport_width, viewport_height) {
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

        // 0b. TopBar — sidebar toggle + chrome buttons. Mirrors the
        //     native host so web + native behave identically.
        let top_bar_rect = self.top_bar_rect(viewport_width);
        let top_bar = self.top_bar();
        if let Some(hit) = top_bar.hit_test(top_bar_rect, Point2D::new(x, y)) {
            self.commit_property_family_focus_if_any();
            let pressed = op_editor_ui::widgets::editor_state_ext::topbar_button_hover(hit);
            self.editor_state.editor_ui.pressed_button =
                Some(op_editor_core::ButtonPressTarget::TopBar(pressed));
            match hit {
                TopBarHit::ToggleSidebar => {
                    let v = &mut self.editor_state.editor_ui.sidebar_open;
                    *v = !*v;
                }
                TopBarHit::ToggleTheme => {
                    self.editor_state.editor_ui.theme_mode =
                        self.editor_state.editor_ui.theme_mode.flipped();
                }
                TopBarHit::ToggleLocale => {
                    let picker = &mut self.editor_state.editor_ui.locale_picker;
                    picker.open = !picker.open;
                    picker.hover = None;
                    picker.pressed = None;
                    if picker.open {
                        picker.scroll.offset = 0.0;
                    }
                }
                TopBarHit::OpenAgentSettings => {
                    self.editor_state.editor_ui.agent_settings_open = true;
                    self.editor_state.chat.blur_input(self.now_ms);
                }
                TopBarHit::ToggleFileMenu => {
                    self.editor_state.editor_ui.file_menu_open ^= true;
                    self.editor_state.editor_ui.file_menu.hover = None;
                    self.clear_layer_panel_hover();
                }
                TopBarHit::OpenFigmaImport => {
                    self.editor_state.editor_ui.figma_import_open = true;
                }
                TopBarHit::ToggleGitPanel => {
                    self.editor_state.editor_ui.git_panel.open ^= true;
                }
                TopBarHit::ToggleFullscreen => {
                    // The web host runs in WASM, so it toggles the browser
                    // Fullscreen API directly — no runner round-trip and no
                    // unconsumed intent flag (unlike native, where the host
                    // can't reach the winit window).
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        if doc.fullscreen_element().is_some() {
                            doc.exit_fullscreen();
                        } else if let Some(el) = doc.document_element() {
                            let _ = el.request_fullscreen();
                        }
                    }
                }
                TopBarHit::TogglePreview => {
                    self.editor_state.editor_ui.toggle_preview();
                }
            }
            self.mark_dirty();
            return true;
        }
        if (top_bar_rect).contains(Point2D::new(x, y)) {
            // Top-bar gaps eat clicks but don't act — still a blank
            // press, so every text input blurs.
            let blurred = self.blur_text_inputs_on_blank_press();
            return blurred || rename_committed || text_edit_committed;
        }

        // 0c0a. Image-fill popover — outside-click dismiss.
        if self.editor_state.editor_ui.image_fill_popover_open {
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
                    self.apply_property_action(action);
                    return true;
                }
            }
            self.editor_state.editor_ui.image_fill_popover_open = false;
            self.mark_dirty();
            return true;
        }

        // 0c0. Fill-type picker — outside-click dismiss. A row
        // click applies the fill type; a click inside the popup body
        // is swallowed; any outside click closes the picker.
        if self.editor_state.editor_ui.fill_type_picker.open {
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
                            let index = self.editor_state.editor_ui.fill_type_picker_index;
                            self.apply_property_action(
                                op_editor_ui::widgets::PropertyPanelAction::SetFillType {
                                    index,
                                    fill_type,
                                },
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

        // 0c0z. Effects "+" add-menu — outside-click dismiss.
        if self.editor_state.editor_ui.effect_add_picker_open {
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
                match panel.effect_add_menu_hit(property_rect, Point2D::new(x, y)) {
                    op_editor_ui::widgets::EffectAddMenuHit::Row(action) => {
                        self.apply_property_action(action);
                        return true;
                    }
                    op_editor_ui::widgets::EffectAddMenuHit::Inside => return true,
                    op_editor_ui::widgets::EffectAddMenuHit::Outside => {}
                }
            }
            self.editor_state.editor_ui.close_effect_add_picker();
            self.mark_dirty();
            return true;
        }

        // 0c0a0. Fill/stroke colour-variable picker — outside-click dismiss.
        if self
            .editor_state
            .editor_ui
            .property_color_variable_picker_open
            .is_some()
        {
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
                    use op_editor_ui::widgets::PropertyPanelAction as A;
                    if matches!(
                        action,
                        A::ToggleColorVariablePicker(_)
                            | A::BindColorVariable { .. }
                            | A::UnbindColorVariable(_)
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

        // 0c0b. Export scale / format inline select popup —
        //       outside-click dismiss. A click on a popup row or a
        //       dropdown toggle is applied; any other click closes
        //       both pickers and is swallowed. Mirrors the native
        //       host's `0c0b` block.
        if self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
        {
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
                        op_editor_ui::widgets::PropertyPanelAction::SetExportScale(_)
                            | op_editor_ui::widgets::PropertyPanelAction::SetExportFormat(_)
                            | op_editor_ui::widgets::PropertyPanelAction::ToggleExportScalePicker
                            | op_editor_ui::widgets::PropertyPanelAction::ToggleExportFormatPicker
                    ) {
                        self.apply_property_action(action);
                        return true;
                    }
                }
            }
            self.editor_state.editor_ui.export_scale_picker_open = false;
            self.editor_state.editor_ui.export_format_picker_open = false;
            self.mark_dirty();
            return true;
        }

        // 0c0b1. Image-node Search / Generate popovers — overlay
        // controls win; outside clicks dismiss.
        if self.dismiss_image_popovers_on_press(x, y, viewport_width, viewport_height) {
            return true;
        }

        // 0c0b2. Font-family picker — outside-click dismiss. A click
        //        on an entry / the trigger is applied; one inside the
        //        popup body (search box / headers) is swallowed.
        if self.editor_state.editor_ui.font_picker.open {
            use op_editor_ui::widgets::PropertyPanelAction as A;
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
                let point = Point2D::new(x, y);
                if let Some(action) = panel.hit_test_action(property_rect, point) {
                    if matches!(
                        action,
                        A::SetFontFamilyIndex(_)
                            | A::ToggleFontFamilyPicker
                            | A::ImportFont
                            | A::RemoveImportedFont(_)
                    ) {
                        self.apply_property_action(action);
                        return true;
                    }
                }
                if panel.font_picker_contains(property_rect, point) {
                    return true;
                }
            }
            let ui = &mut self.editor_state.editor_ui;
            ui.close_font_picker();
            self.mark_dirty();
            return true;
        }

        // 0c0c. Font-weight dropdown + padding mode-selector popover —
        //       outside-click dismiss. A click on a picker row / toggle
        //       is applied; any other click closes the popover and is
        //       swallowed (mirrors the native host's dismiss handlers).
        if self.editor_state.editor_ui.font_weight_picker_open
            || self.editor_state.editor_ui.padding_mode_popover_open
            || self.editor_state.editor_ui.stroke_mode_popover_open
        {
            use op_editor_ui::widgets::PropertyPanelAction as A;
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
                        A::SetFontWeight(_)
                            | A::ToggleFontWeightPicker
                            | A::SetPaddingMode(_)
                            | A::TogglePaddingModePopover
                            | A::SetStrokeMode(_)
                            | A::ToggleStrokeModePopover
                    ) {
                        if let A::SetFontWeight(choice) = action {
                            self.editor_state.editor_ui.pressed_button =
                                op_editor_ui::widgets::FontWeightChoice::ALL
                                    .iter()
                                    .position(|c| *c == choice)
                                    .map(op_editor_core::ButtonPressTarget::FontWeightPicker);
                            self.mark_dirty();
                            return true;
                        }
                        self.apply_property_action(action);
                        return true;
                    }
                }
            }
            self.editor_state.editor_ui.font_weight_picker_open = false;
            self.editor_state.editor_ui.font_weight_picker_hover = None;
            self.editor_state.editor_ui.padding_mode_popover_open = false;
            self.editor_state.editor_ui.padding_mode_popover_hover = None;
            self.editor_state.editor_ui.stroke_mode_popover_open = false;
            self.editor_state.editor_ui.stroke_mode_popover_hover = None;
            self.mark_dirty();
            return true;
        }

        // 0c. PropertyPanel button / checkbox — flex modes + size
        //     flags. Runs AFTER locale picker + TopBar so the
        //     dropdown overlays still win.
        if let Some(panel) =
            PropertyPanel::for_selection_with_scene(&self.editor_state, &self.layout_scene)
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
                self.commit_property_family_focus_if_any();
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
            let point = Point2D::new(x, y);
            if let Some(action) = panel.hit_test_action(property_rect, point) {
                self.editor_state.editor_ui.pressed_button =
                    if let op_editor_ui::widgets::PropertyPanelAction::Codegen(codegen_action) =
                        action
                    {
                        op_editor_ui::widgets::property_panel_code::codegen_hover_for_action(
                            codegen_action,
                        )
                        .map(op_editor_core::ButtonPressTarget::Codegen)
                    } else {
                        panel
                            .action_hover_index(property_rect, point)
                            .map(op_editor_core::ButtonPressTarget::PropertyPanel)
                    };
                self.commit_property_focus_if_any();
                // Anchor the colour picker at the clicked y so it
                // pops next to the swatch row, not at the panel top.
                if let op_editor_ui::widgets::PropertyPanelAction::OpenColorPicker(target) = action
                {
                    let _ = self.editor_state.open_color_picker(
                        super::property_dispatch::color_target_public(target),
                        y,
                    );
                    self.mark_dirty();
                } else if let op_editor_ui::widgets::PropertyPanelAction::OpenFillColorPicker(
                    index,
                ) = action
                {
                    // Non-primary fill swatch — bind the picker to this
                    // fill so HSV writes back to `fills[index]`.
                    self.editor_state
                        .editor_ui
                        .property_color_variable_picker_open = None;
                    let _ = self.editor_state.open_color_picker_for_fill(
                        op_editor_core::ui_draft::ColorTarget::Fill,
                        index,
                        y,
                    );
                    self.mark_dirty();
                } else if let op_editor_ui::widgets::PropertyPanelAction::OpenEffectColorPicker(
                    index,
                ) = action
                {
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
            if let Some(focus) = panel.hit_test(property_rect, point) {
                return self.focus_property_input_from_press(focus, property_rect, point);
            }
            if (property_rect).contains(point) {
                self.blur_text_inputs_on_blank_press();
                return true;
            }
        }
        let property_focus_committed = self.commit_property_family_focus_if_any();

        // 1. AI chat panel — painted on top of toolbar so a
        //    click inside its rect is consumed here, even when
        //    that point lies inside the toolbar rect underneath.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .owned_by(self.chat_panel_owner);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                if matches!(hit, AIChatHit::Resize(_)) {
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

        // 2. Toolbar — second-highest overlay. Bounding rect
        //    consumes all clicks (gaps + padding too) so it
        //    never falls through to the canvas for tool gaps
        //    that lie outside the chat panel.
        let toolbar_rect = self.toolbar_rect(viewport_width);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        if (toolbar_rect).contains(Point2D::new(x, y)) {
            if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
                match hit {
                    op_editor_ui::widgets::ToolbarHit::Tool(tool) => {
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
                        return acted || rename_committed || property_focus_committed;
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
            return blurred || rename_committed || text_edit_committed || property_focus_committed;
        }

        // 3. apply_click — LayerPanel + chat-defocus.
        //    Pre-seed a `layer_drag` candidate when the press lands
        //    on a Layer row so a subsequent move past the threshold
        //    promotes the gesture to a drag-to-reorder.
        if self.editor_state.editor_ui.sidebar_open {
            let layer_rect = self.layer_panel_rect(viewport_height);
            let panel = LayerPanel::from_editor(&self.editor_state);
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
        // 2.5. Floating align/distribute toolbar — visible when
        //      2+ nodes are selected. Hit-tested before apply_click
        //      so the visible button always wins over a layer row
        //      that happens to share screen y (matches native order).
        {
            use op_editor_ui::widgets::{AlignToolbar, AlignToolbarHit};
            let (acx, _, acw, ach) = self.canvas_region(viewport_width, viewport_height);
            let canvas_region = op_editor_ui::Rect {
                origin: Point2D::new(acx, TOP_BAR_HEIGHT),
                size: Point2D::new(acw, ach),
            };
            if let Some(hit) = AlignToolbar::for_canvas_region(canvas_region, &self.editor_state)
                .and_then(|tb| tb.hit_test_action(Point2D::new(x, y)))
            {
                match hit {
                    AlignToolbarHit::Align(action) => {
                        self.editor_state.align_selected(action);
                        self.mark_dirty();
                    }
                    AlignToolbarHit::Boolean(op) => {
                        let _ = self.apply_boolean_op(op);
                    }
                }
                return true;
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
            if matches!(self.editor_state.tool, op_editor_core::Tool::Hand) || self.space_pan {
                self.drag = Some(DragState {
                    last_x: x,
                    last_y: y,
                });
                return rename_committed || text_edit_committed || property_focus_committed;
            }
            if matches!(self.editor_state.tool, op_editor_core::Tool::Select) {
                if self.try_path_anchor_press(x, y, viewport_width, viewport_height) {
                    return true;
                }
                if self.try_selection_handle_press(x, y, viewport_width, viewport_height) {
                    return true;
                }
                // Convert screen → doc to ask which node (if any)
                // is under the cursor — `node_at_doc_point` queries
                // the layout-resolved render scene.
                let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
                let canvas_rect = Rect {
                    origin: Point2D::new(cx0, cy0),
                    size: Point2D::new(_cw, _ch),
                };
                let canvas_local = Point2D::new(x - cx0, y - cy0);
                let doc_point = self.editor_state.viewport.to_document(canvas_local);
                let canvas = CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
                if let Some(sc_node_id) =
                    canvas.frame_label_at_point(canvas_rect, Point2D::new(x, y))
                {
                    let node_id = op_editor_core::NodeId::new(&sc_node_id);
                    return self.apply_canvas_node_press(
                        vec![node_id],
                        x,
                        y,
                        text_edit_was_active,
                        viewport_height,
                    );
                }
                let hit_path = self
                    .layout_scene
                    .node_path_at_doc_point(doc_point, self.editor_state.viewport.zoom);
                if let Some(hit_path) = hit_path {
                    let hit_path = hit_path
                        .into_iter()
                        .map(op_editor_core::NodeId::new)
                        .collect();
                    return self.apply_canvas_node_press(
                        hit_path,
                        x,
                        y,
                        text_edit_was_active,
                        viewport_height,
                    );
                }
                // Empty canvas with Select → marquee.
                self.editor_state.editor_ui.last_canvas_click = None;
                let cleared_now = if !self.shift_held {
                    let was_set = !self.editor_state.selection.set.is_empty();
                    let exited_scope = self
                        .editor_state
                        .editor_ui
                        .entered_container
                        .take()
                        .is_some();
                    if was_set {
                        self.editor_state.clear_selection();
                    }
                    if was_set || exited_scope {
                        self.mark_dirty();
                    }
                    was_set || exited_scope
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
                return cleared_now
                    || rename_committed
                    || text_edit_committed
                    || property_focus_committed;
            }
            let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc_point = self.editor_state.viewport.to_document(canvas_local);
            if self.start_create_drag_at(doc_point) {
                return true;
            }

            // Tool didn't accept this point — fall back to pan.
            self.drag = Some(DragState {
                last_x: x,
                last_y: y,
            });
            return rename_committed || text_edit_committed || property_focus_committed;
        }
        // Final fall-through — the press hit no interactive chrome
        // (panel-rail gaps, property-panel padding, …): blank press.
        let blurred = self.blur_text_inputs_on_blank_press();
        blurred || rename_committed || text_edit_committed || property_focus_committed
    }
}

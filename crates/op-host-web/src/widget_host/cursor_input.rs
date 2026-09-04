//! Cursor-move dispatch for the web `WidgetHost` — canvas pan-drag,
//! marquee / layer / chat drags, and every hover wash (agent
//! settings, toolbar, top bar, status bar, chat, property panel,
//! code panel, align toolbar). Split out of `widget_host.rs` to keep
//! the spine under the repo's 800-line cap (mirrors the native
//! host's `widget_host/input.rs` split).

use op_editor_ui::widgets::cursor_hover_flow as hover_flow;
use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

use super::WidgetHost;

impl WidgetHost {
    /// Cursor-move handler — drives canvas pan-drag, marquee /
    /// layer / chat / overlay drags, and the chrome hover washes.
    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        self.last_cursor_x = x;
        self.last_cursor_y = y;
        // Session-switch owner rotation before the cursor_probe resolve stores
        // the canonical build (mirrors native).
        self.rotate_chat_owner_if_session_changed();
        // Refresh the derived paint doc once up front so every hit-test
        // below (layer context menu, layer drag, align toolbar) reads
        // current geometry, never a stale snapshot.
        self.refresh_layout_scene();
        // Route canvas moves to preview when preview mode is active
        #[cfg(feature = "canvaskit")]
        if self.editor_state.editor_ui.preview.mode && self.preview.is_some() {
            // Chrome hover first: both switchers maintain their own hover so
            // release can confirm the pointer never left the armed segment.
            let (vw, vh) = (self.last_viewport_w, self.last_viewport_h);
            self.preview_switcher_hover(x, y, vw, vh);
            self.screen_switcher_hover(x, y, vw, vh);
            return self.preview_dispatch_move(x, y, vw, vh);
        }
        // Pointer-capture drags + the modal surfaces, in `cursor_input_modals`.
        // They run FIRST — a modal that does not claim the cursor lets the
        // hover washes underneath it light up through its own scrim.
        if let Some(consumed) = self.cursor_move_modal_tiers(x, y) {
            return consumed;
        }
        let picker_open = self.editor_state.editor_ui.chat_model_picker.open;
        let point = Point2D::new(x, y);
        // Cheap transcript-free ownership probe used only to keep lower
        // floating panels and stale higher-hover exits from returning before
        // the exact combined Chat probe below. The exact ownership decision
        // still comes from `cursor_probe.hit` and is resolved once per move.
        let chat_surface_owns_point = !picker_open
            && self
                .ai_chat_rect(self.last_viewport_w, self.last_viewport_h)
                .is_some_and(|rect| {
                    rect.contains(point)
                        || op_editor_ui::widgets::AIChatPlaceholder::from_editor(&self.editor_state)
                            .resize_edge_at(rect, point)
                            .is_some()
                });
        let chat_or_picker_surface_owns_point = picker_open || chat_surface_owns_point;
        let mut upper_hover_changed = false;
        // Property dropdowns are handled by the overlay dispatcher before the
        // base right-rail hover pass. Cache the construction attempt (including
        // a `None` result) so this cursor event never rebuilds the same panel.
        let property_dropdown_open = self.editor_state.editor_ui.fill_type_picker.open
            || self.editor_state.editor_ui.effect_add_picker_open
            || self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
            || self.editor_state.editor_ui.interaction_menu_open
            || self.editor_state.editor_ui.padding_mode_popover_open
            || self.editor_state.editor_ui.stroke_mode_popover_open
            || self.editor_state.editor_ui.font_weight_picker_open
            || self.editor_state.editor_ui.font_picker.open
            || self
                .editor_state
                .editor_ui
                .property_color_variable_picker_open
                .is_some()
            || self.editor_state.editor_ui.image_fill_popover_open
            || self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open;
        let mut property_panel_probe =
            property_dropdown_open.then(|| PropertyPanel::for_selection(&self.editor_state));
        // Floating-overlay drags + hovers (colour picker, Design-MD /
        // Icon-picker / Component-Browser panels, open dropdowns) own
        // the cursor before lower context menus. This matches native:
        // a topmost panel covering a path-anchor / layer menu must
        // block that lower menu's hover wash.
        let overlay_property_panel = property_panel_probe.as_ref().and_then(Option::as_ref);
        if self.apply_overlay_cursor_move(
            x,
            y,
            overlay_property_panel,
            chat_or_picker_surface_owns_point,
            &mut upper_hover_changed,
        ) {
            return true;
        }
        let over_path_menu =
            hover_flow::path_anchor_menu_contains(&self.editor_state, Point2D::new(x, y));
        let path_menu_changed = self.update_path_anchor_menu_hover(x, y);
        if over_path_menu {
            let lower_cleared = self.clear_chat_and_lower_hover();
            return path_menu_changed || lower_cleared;
        }
        if path_menu_changed {
            if chat_or_picker_surface_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            let over_menu = menu.rect().contains(Point2D::new(x, y));
            let new_hover = menu.hovered_row_at(Point2D::new(x, y));
            if new_hover != state.menu.hover {
                let mut next = state;
                next.menu.hover = new_hover;
                self.editor_state.editor_ui.layer_context_menu = Some(next);
                self.mark_dirty();
                if over_menu {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if chat_or_picker_surface_owns_point {
                    upper_hover_changed = true;
                } else {
                    return true;
                }
            }
            if over_menu {
                let lower_cleared = self.clear_chat_and_lower_hover();
                return lower_cleared;
            }
        }
        if self.apply_collab_panel_cursor_move(
            x,
            y,
            chat_or_picker_surface_owns_point,
            &mut upper_hover_changed,
        ) {
            return true;
        }
        // Export-section select-popup row hover highlight.
        if self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
        {
            if property_panel_probe.is_none() {
                property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
            }
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
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
                let point = Point2D::new(x, y);
                let over_popup = panel.export_picker_contains(property_rect, point);
                let new_hover = panel.export_picker_row_at(property_rect, point);
                let changed = new_hover != self.editor_state.editor_ui.export_picker_hover;
                if changed {
                    self.editor_state.editor_ui.export_picker_hover = new_hover;
                    self.mark_dirty();
                }
                if over_popup {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
                }
            }
        }
        if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
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
            if self.editor_state.editor_ui.interaction_menu_open {
                let new_hover = panel.interaction_menu_row_at(property_rect, point);
                let changed = new_hover != self.editor_state.editor_ui.interaction_menu_hover;
                if changed {
                    self.editor_state.editor_ui.interaction_menu_hover = new_hover;
                    self.mark_dirty();
                }
                let over_popup = !matches!(
                    panel.interaction_menu_hit(property_rect, point),
                    op_editor_ui::widgets::InteractionMenuHit::Outside
                );
                if over_popup {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
                }
            }
            if let Some(consumed) = self.color_variable_picker_hover_tier(
                panel,
                property_rect,
                point,
                chat_or_picker_surface_owns_point,
                &mut upper_hover_changed,
            ) {
                return consumed;
            }
            if self.editor_state.editor_ui.padding_mode_popover_open
                || self.editor_state.editor_ui.stroke_mode_popover_open
            {
                use op_editor_ui::widgets::PropertyPanelAction as A;
                let new_hover = match panel.hit_test_action(property_rect, point) {
                    Some(A::SetPaddingMode(mode)) | Some(A::SetStrokeMode(mode)) => {
                        op_editor_core::PaddingEditMode::ALL
                            .iter()
                            .position(|candidate| *candidate == mode)
                    }
                    _ => None,
                };
                let mut changed = false;
                if self.editor_state.editor_ui.padding_mode_popover_open
                    && new_hover != self.editor_state.editor_ui.padding_mode_popover_hover
                {
                    self.editor_state.editor_ui.padding_mode_popover_hover = new_hover;
                    changed = true;
                }
                if self.editor_state.editor_ui.stroke_mode_popover_open
                    && new_hover != self.editor_state.editor_ui.stroke_mode_popover_hover
                {
                    self.editor_state.editor_ui.stroke_mode_popover_hover = new_hover;
                    changed = true;
                }
                if changed {
                    self.mark_dirty();
                }
                let over_popup = (self.editor_state.editor_ui.padding_mode_popover_open
                    && panel.padding_mode_popover_contains(property_rect, point))
                    || (self.editor_state.editor_ui.stroke_mode_popover_open
                        && panel.stroke_mode_popover_contains(property_rect, point));
                if over_popup {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
                }
            }
            if self.editor_state.editor_ui.font_weight_picker_open {
                use op_editor_ui::widgets::PropertyPanelAction as A;
                let new_hover = match panel.hit_test_action(property_rect, point) {
                    Some(A::SetFontWeight(choice)) => op_editor_ui::widgets::FontWeightChoice::ALL
                        .iter()
                        .position(|candidate| *candidate == choice),
                    _ => None,
                };
                let changed = new_hover != self.editor_state.editor_ui.font_weight_picker_hover;
                if changed {
                    self.editor_state.editor_ui.font_weight_picker_hover = new_hover;
                    self.mark_dirty();
                }
                if panel.font_weight_picker_contains(property_rect, point) {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
                }
            }
            if self.editor_state.editor_ui.font_picker.open {
                let over_popup = panel.font_picker_contains(property_rect, point);
                let entry_hover = if over_popup {
                    panel.font_picker_entry_index_at(property_rect, point)
                } else {
                    None
                };
                let import_hover =
                    over_popup && panel.font_picker_import_action_at(property_rect, point);
                let changed = entry_hover != self.editor_state.editor_ui.font_picker.hover
                    || import_hover != self.editor_state.editor_ui.font_picker_import_hover;
                if changed {
                    self.editor_state.editor_ui.font_picker.hover = entry_hover;
                    self.editor_state.editor_ui.font_picker_import_hover = import_hover;
                    self.mark_dirty();
                }
                if over_popup {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
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
        if self.apply_code_selection_drag_cursor_move(x, y) {
            return true;
        }
        if let Some(consumed) = self.apply_image_crop_drag_cursor_move(x, y) {
            return consumed;
        }
        if let Some(consumed) = self.apply_node_drag_cursor_move(x, y) {
            return consumed;
        }
        if self.apply_selection_handle_drag_move(x, y) {
            return true;
        }
        if self.update_create_drag(x, y) {
            return true;
        }
        if let Some(m) = self.marquee_drag.as_mut() {
            m.current_screen_x = x;
            m.current_screen_y = y;
            return true;
        }
        if self.layer_drag.is_some() {
            // Drop the gesture if the source disappeared mid-drag —
            // see the native host for the rationale.
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
            // VERTICAL-ONLY activation (4 px). See the native host
            // for the rationale: pure horizontal wiggle must not
            // steal click-feel from row-level gestures.
            if !d.active && (y - d.start_y).abs() > 4.0 {
                d.active = true;
            }
            return true;
        }
        if let Some(d) = self.chat_drag.as_mut() {
            d.pos_x = x - d.grab_dx;
            d.pos_y = y - d.grab_dy;
            return true;
        }
        if let Some(field) = self.image_adjustment_drag {
            if property_panel_probe.is_none() {
                property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
            }
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
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
        if let Some(effect) = self.effect_radius_drag {
            if property_panel_probe.is_none() {
                property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
            }
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
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
                if let Some(action) = panel.effect_radius_drag_action(property_rect, effect, x) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        if let Some(drag) = self.drag.as_mut() {
            let dx = x - drag.last_x;
            let dy = y - drag.last_y;
            drag.last_x = x;
            drag.last_y = y;
            self.editor_state.viewport.pan(dx, dy);
            // Canvas pan only translates the viewport; the layout-resolved
            // scene is document-space (camera applied at paint time), so keep
            // the layout cache intact — `mark_dirty()` here forced a full
            // serde reconversion of the document every move (matches native
            // `op-host-native/.../input.rs`). The listener still repaints off
            // this `true` return.
            return true;
        }
        // Variables is painted below Chat even though the shared historical
        // `over_topmost_panel` helper groups it with floating panels. Only the
        // three panels painted after Chat may suppress ordinary Chat ownership.
        let over_topmost = !picker_open
            && self.over_true_topmost_panel(point, self.last_viewport_w, self.last_viewport_h);
        // The left rail's slides tab (body in `slides_panel.rs`), which
        // yields to the open model picker painted above it.
        if let Some(dirty) = self.slides_panel_cursor_tier(point, over_topmost || picker_open) {
            return dirty;
        }
        // StatusBar paints and presses above Chat. Preserve its ownership
        // before the model picker truncates LayerPanel / toolbar / base UI.
        if let Some(status_rect) = self.status_bar_rect(self.last_viewport_w, self.last_viewport_h)
        {
            let over_status = !over_topmost && status_rect.contains(point);
            let new_hover = if over_topmost {
                None
            } else {
                op_editor_ui::widgets::StatusBar::for_editor(&self.editor_state)
                    .control_at(status_rect, point)
            };
            if new_hover != self.editor_state.editor_ui.statusbar_hover {
                self.editor_state.editor_ui.statusbar_hover = new_hover;
                self.mark_dirty();
                if over_status {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if chat_or_picker_surface_owns_point {
                    upper_hover_changed = true;
                } else {
                    return true;
                }
            }
            if over_status {
                let lower_cleared = self.clear_chat_and_lower_hover();
                return lower_cleared;
            }
        }
        // AlignToolbar paints above both the regular Chat card and its model
        // picker. Its whole opaque pill (including padding and group gutters)
        // receives first refusal, not only the action buttons.
        let (new_align_hover, align_owns_point) = if self.editor_state.selection_count() >= 2 {
            use op_editor_ui::widgets::{AlignToolbar, TOP_BAR_HEIGHT};
            let (cx, _, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let canvas_region = op_editor_ui::Rect {
                origin: Point2D::new(cx, TOP_BAR_HEIGHT),
                size: Point2D::new(cw, ch),
            };
            AlignToolbar::for_canvas_region(canvas_region, &self.editor_state)
                .map(|toolbar| (toolbar.hit_test(point), toolbar.rect().contains(point)))
                .unwrap_or((None, false))
        } else {
            (None, false)
        };
        let mut retained_hover_changed = upper_hover_changed;
        if new_align_hover != self.editor_state.editor_ui.align_toolbar_hover {
            self.editor_state.editor_ui.align_toolbar_hover = new_align_hover;
            self.mark_dirty();
            retained_hover_changed = true;
        }
        if picker_open && align_owns_point {
            let lower_changed = self.clear_chat_and_lower_hover();
            return retained_hover_changed || lower_changed;
        }
        // The visible chat model picker owns every lower hover surface. Keep
        // higher overlays and active drags above it, then stop before the
        // LayerPanel / toolbar / base chat and property probes.
        if self.editor_state.editor_ui.chat_model_picker.open {
            let Some(picker) =
                self.chat_model_picker_rect(self.last_viewport_w, self.last_viewport_h)
            else {
                // A hidden chat (embed/collapse) or a viewport too small to
                // lay it out must not leave an invisible modal lock behind.
                self.editor_state.editor_ui.close_chat_model_picker();
                self.mark_dirty();
                return true;
            };
            let hover_changed = self.update_chat_model_picker_hover(x, y, picker);
            let lower_changed = self.clear_hover_below_chat_model_picker();
            return upper_hover_changed || hover_changed || lower_changed;
        }
        // TopBar chrome-button hover wash (sidebar / file-menu / figma /
        // theme / locale / fullscreen / agent chip). Git and Preview are
        // compiled out on wasm32; every visible button lights up the same
        // as native. Reuses the click hit-test so paint can't drift.
        {
            let tb_rect = self.top_bar_rect(self.last_viewport_w);
            let new_hover = (!chat_surface_owns_point)
                .then(|| self.top_bar().hit_test(tb_rect, point))
                .flatten()
                .map(op_editor_ui::widgets::editor_state_ext::topbar_button_hover);
            let ui = &mut self.editor_state.editor_ui;
            if ui.set_topbar_button_hover(new_hover, self.now_ms) {
                self.mark_dirty();
                if chat_surface_owns_point {
                    retained_hover_changed = true;
                } else {
                    return true;
                }
            }
        }
        // Construct the chat panel once for this cursor event and resolve all
        // hover results from that instance. Besides fingerprinting the
        // transcript once, this avoids rebuilding translated labels and tabs
        // for each sub-control.
        let mut style_chip_hover = false;
        let (chat_probe, chat_tab_hover, chat_footer_hover, parallel_hover, example_hover) =
            if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
                use op_editor_ui::widgets::AIChatPlaceholder;
                let panel = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                    .owned_by(self.chat_panel_owner);
                style_chip_hover = panel.style_chip_hover_at(chat_rect, point);
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
        let chat_owns_point = !over_topmost
            && !align_owns_point
            && chat_probe.as_ref().is_some_and(|probe| probe.hit.is_some());
        let chat_header_hover = if !over_topmost && !align_owns_point {
            chat_probe
                .as_ref()
                .and_then(|probe| probe.hit.as_ref())
                .and_then(op_editor_ui::widgets::editor_state_ext::chat_header_hover)
        } else {
            None
        };
        let (chat_tab_hover, chat_footer_hover, parallel_hover, example_hover) =
            if !over_topmost && !align_owns_point {
                (
                    chat_tab_hover,
                    chat_footer_hover,
                    parallel_hover,
                    example_hover,
                )
            } else {
                (None, None, None, None)
            };
        // Same gate: a clock started under a higher surface would time out into
        // a card floating over it.
        let style_chip_hover = style_chip_hover && !over_topmost && !align_owns_point;
        let design_hover = if !over_topmost && !align_owns_point {
            chat_probe
                .as_ref()
                .and_then(|probe| probe.design_block_hover)
        } else {
            None
        };
        // Update every Chat hover from the same probe before deciding whether
        // the panel owns the move. A control transition must not prevent the
        // panel's blank body from blocking lower layers.
        let mut chat_hover_changed = false;
        let now_ms = self.now_ms;
        {
            let ui = &mut self.editor_state.editor_ui;
            if chat_header_hover != ui.chat_header_hover {
                ui.chat_header_hover = chat_header_hover;
                chat_hover_changed = true;
            }
            if chat_tab_hover != ui.chat_tab_hover {
                ui.chat_tab_hover = chat_tab_hover;
                chat_hover_changed = true;
            }
            if chat_footer_hover != ui.chat_footer_hover {
                ui.chat_footer_hover = chat_footer_hover;
                chat_hover_changed = true;
            }
            if parallel_hover != ui.parallel_agents_picker_hover {
                ui.parallel_agents_picker_hover = parallel_hover;
                chat_hover_changed = true;
            }
            if example_hover != ui.chat_example_hover {
                ui.chat_example_hover = example_hover;
                chat_hover_changed = true;
            }
            // Pinned-style chip — a dwell clock rather than a wash, so it goes
            // through the setter that owns the clock's start / stop rule.
            chat_hover_changed |= ui.set_chat_style_chip_hover(style_chip_hover, now_ms);
        }
        if chat_hover_changed {
            self.mark_dirty();
        }
        chat_hover_changed |= self.apply_chat_design_hover(design_hover);
        if chat_owns_point || align_owns_point {
            let lower_changed = self.clear_hover_below_chat_panel();
            return retained_hover_changed || chat_hover_changed || lower_changed;
        }

        // Chat sits above these base surfaces. They are dispatched only after
        // the combined Chat ownership probe has missed.
        if self.update_layer_hover(x, y, self.last_viewport_h) {
            return true;
        }
        if self.update_toolbar_hover(x, y) {
            return true;
        }
        if self.editor_state.editor_ui.variables_panel_open {
            use op_editor_ui::widgets::variables_panel::VariablesPanel;
            if let Some(vars_rect) =
                self.variables_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                if (vars_rect).contains(point) {
                    let new_hover = VariablesPanel::for_editor_at(&self.editor_state, self.now_ms)
                        .hover_at(vars_rect, point);
                    let changed = new_hover != self.editor_state.editor_ui.variables_panel_hover;
                    if changed {
                        self.editor_state.editor_ui.variables_panel_hover = new_hover;
                        self.mark_dirty();
                    }
                    return retained_hover_changed || chat_hover_changed || changed;
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
        } else if self
            .editor_state
            .editor_ui
            .variables_panel_hover
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        // PropertyPanel tab/action hover wash. Shown with a selection.
        let mut property_hover_changed = false;
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
        let over_property_panel = self.editor_state.property_panel_visible()
            && !over_topmost
            && property_rect.contains(point);
        let needs_property_probe = self.editor_state.property_panel_visible()
            && !over_topmost
            && (over_property_panel
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
        let inside_property_panel = over_property_panel && property_panel.is_some();
        property_hover_changed |= hover_flow::property_base_hover(
            &mut self.editor_state,
            property_panel,
            property_rect,
            point,
        );
        // Code-panel hover wash. Reuses the panel's click geometry for
        // framework chips, scroll chevrons, and body actions. Web has no
        // `over_topmost` gate here (native does) — the floating panels that
        // would suppress it are already handled by `apply_overlay_cursor_move`.
        if hover_flow::code_panel_hover(&mut self.editor_state, property_rect, point, true) {
            self.mark_dirty();
            return true;
        }
        if inside_property_panel {
            let lower_hover_changed = self.clear_hover_below_property_panel();
            if property_hover_changed && !lower_hover_changed {
                self.mark_dirty();
            }
            return true;
        }
        if property_hover_changed {
            self.mark_dirty();
            return true;
        }
        // Canvas hierarchy hover. The scene hit path is resolved to
        // the current level's primary target; shared canvas paint then
        // draws that node solid plus all of its direct children dashed.
        // Frame labels participate as explicit root targets.
        let hover_eligible = !over_topmost
            && matches!(self.editor_state.tool, op_editor_core::Tool::Select)
            && self.over_canvas(x, y, self.last_viewport_w, self.last_viewport_h);
        let new_canvas_hover = if hover_eligible {
            let (cx0, cy0, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let canvas_rect = Rect {
                origin: Point2D::new(cx0, cy0),
                size: Point2D::new(cw, ch),
            };
            hover_flow::canvas_hover_target(
                &self.editor_state,
                &self.layout_scene,
                canvas_rect,
                Point2D::new(x, y),
            )
        } else {
            None
        };
        if new_canvas_hover != self.editor_state.editor_ui.canvas_hover_node {
            self.editor_state.editor_ui.canvas_hover_node = new_canvas_hover;
            self.mark_dirty();
            return true;
        }
        retained_hover_changed || chat_hover_changed
    }
}

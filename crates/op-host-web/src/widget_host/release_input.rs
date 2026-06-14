//! Mouse-release handling for the web `WidgetHost` — marquee commit,
//! layer drag-to-reorder commit, chat-panel corner snap, and drag
//! teardown. Split out of `widget_host.rs` to keep the spine under
//! the repo's 800-line cap.

use op_editor_core::{agent_settings::ImageGenField, AgentSettingsButton, ButtonPressTarget};
use op_editor_ui::widgets::{FontWeightChoice, PropertyPanelAction};
use op_editor_ui::{Point2D, Rect};

use super::{LayerDragState, MarqueeDragState, WidgetHost};

impl WidgetHost {
    /// Convert a marquee drag (screen-space) into a doc-space
    /// rect, ask the document which top-level nodes overlap it,
    /// and either replace or extend the selection. Mirrors
    /// native `WidgetHostNative::commit_marquee_selection`.
    pub(in crate::widget_host) fn commit_marquee_selection(
        &mut self,
        m: MarqueeDragState,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        // Near-zero marquee = a click without drag. Threshold is
        // measured in SCREEN pixels (2 px) so it stays consistent
        // regardless of canvas zoom — matches native.
        let screen_dx = (m.current_screen_x - m.start_screen_x).abs();
        let screen_dy = (m.current_screen_y - m.start_screen_y).abs();
        if screen_dx < 2.0 && screen_dy < 2.0 {
            return;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let p0 = {
            let local = Point2D::new(m.start_screen_x - cx0, m.start_screen_y - cy0);
            self.editor_state.viewport.to_document(local)
        };
        let p1 = {
            let local = Point2D::new(m.current_screen_x - cx0, m.current_screen_y - cy0);
            self.editor_state.viewport.to_document(local)
        };
        let x = p0.x.min(p1.x);
        let y = p0.y.min(p1.y);
        let w = (p1.x - p0.x).abs();
        let h = (p1.y - p0.y).abs();
        let rect = Rect::xywh(x, y, w, h);
        // `nodes_intersecting_doc_rect` queries the `LayoutScene` —
        // it returns the resolved-scene node id strings.
        self.refresh_layout_scene();
        let ids = self.layout_scene.nodes_intersecting_doc_rect(rect);
        if m.additive {
            // ADD-only: every hit joins the set; already-selected
            // hits stay selected. Shift-marquee never removes.
            for id in ids {
                let ec_id = op_editor_core::NodeId::new(&id);
                if !self.editor_state.is_selected(&ec_id) {
                    self.editor_state.toggle_selection(ec_id);
                }
            }
            self.mark_dirty();
        } else if !ids.is_empty() {
            let ec_ids: Vec<op_editor_core::NodeId> =
                ids.iter().map(op_editor_core::NodeId::new).collect();
            let anchor = ec_ids.last().unwrap().clone();
            self.editor_state.selection.set = ec_ids;
            self.editor_state.selection.anchor = anchor;
            self.mark_dirty();
        }
    }

    /// Mouse-release handler — commits any marquee drag, snaps a
    /// chat-panel drag to the nearest corner, or ends the canvas
    /// pan-drag.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        self.last_viewport_w = viewport_w;
        self.last_viewport_h = viewport_h;
        let pressed_released = self.release_pressed_feedback();
        // Colour-picker drag end (non-consuming) + floating-panel
        // header drags — see `widget_host/overlay_cursor.rs`.
        if self.release_overlay_drags() {
            return true;
        }
        if self.variables_resize.take().is_some() {
            return true;
        }
        if let Some(m) = self.marquee_drag.take() {
            self.commit_marquee_selection(m, viewport_w, viewport_h);
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
        if let Some(d) = self.layer_drag.take() {
            return self.commit_layer_drag(d, viewport_h);
        }
        if let Some(d) = self.chat_drag.take() {
            // Use the live panel size (expanded vs collapsed) so a
            // dragged collapsed pill snaps to the corner closest to
            // its actual center, matching native.
            let (panel_w, panel_h) = self.ai_chat_size();
            let center = Point2D::new(d.pos_x + panel_w / 2.0, d.pos_y + panel_h / 2.0);
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
            self.editor_state.chat.anchor =
                op_editor_core::ChatAnchor::nearest(center, cx0, cy0, cw, ch);
            self.mark_dirty();
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging || pressed_released
    }

    /// Mouse-release handler — viewport-less variant. Public host
    /// API parity with the native shell; the browser runner wires
    /// the viewport-aware `apply_release_with_viewport` instead.
    #[allow(dead_code)]
    pub fn apply_release(&mut self) -> bool {
        let pressed_released = self.release_pressed_feedback();
        if self.release_overlay_drags() {
            return true;
        }
        if self.variables_resize.take().is_some() {
            return true;
        }
        if self.marquee_drag.take().is_some() {
            // Can't compute the doc-space marquee rect without a
            // viewport; drop without committing. The viewport-
            // aware variant is the one runners should call.
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
        if self.layer_drag.take().is_some() {
            // Same — drop_target_at needs the panel rect (and thus
            // viewport_h). Drop the candidate without committing.
            return true;
        }
        if self.chat_drag.take().is_some() {
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging || pressed_released
    }

    fn release_pressed_feedback(&mut self) -> bool {
        let pressed_button = self.editor_state.editor_ui.pressed_button.take();
        let button_released = pressed_button.is_some();
        let chat_model_pressed = self.editor_state.editor_ui.chat_model_picker.pressed.take();
        let chat_model_released = chat_model_pressed.is_some();
        let icon_picker_released = self
            .editor_state
            .editor_ui
            .icon_picker
            .pressed
            .take()
            .is_some();

        self.commit_deferred_pressed_button(pressed_button);
        self.commit_deferred_chat_model(chat_model_pressed);

        let released = button_released || chat_model_released || icon_picker_released;
        if released {
            self.mark_dirty();
        }
        released
    }

    fn commit_deferred_pressed_button(&mut self, pressed: Option<ButtonPressTarget>) {
        match pressed {
            Some(ButtonPressTarget::FontWeightPicker(index)) => {
                if let Some(choice) = FontWeightChoice::ALL.get(index).copied() {
                    self.apply_property_action(PropertyPanelAction::SetFontWeight(choice));
                }
            }
            Some(ButtonPressTarget::AgentSettings(AgentSettingsButton::ImageProviderOption {
                index,
                provider,
            })) => {
                {
                    let settings = &mut self.editor_state.editor_ui.agent_settings;
                    if let Some(profile) = settings.image_gen_profiles.get_mut(index) {
                        if profile.provider != provider {
                            profile.provider = provider;
                            profile.model.clear();
                        }
                    }
                    settings.image_gen_provider_menu_open = None;
                }
                self.focus_image_gen_profile(index, ImageGenField::Name);
            }
            _ => {}
        }
    }

    fn commit_deferred_chat_model(&mut self, pressed: Option<usize>) {
        if let Some(index) = pressed {
            self.editor_state.select_chat_model(index);
        }
    }

    /// Resolve a layer drag-to-reorder gesture on release. Mirrors
    /// native `WidgetHostNative::commit_layer_drag`.
    pub(in crate::widget_host) fn commit_layer_drag(
        &mut self,
        d: LayerDragState,
        viewport_h: f32,
    ) -> bool {
        if !d.active {
            return false;
        }
        self.refresh_layout_scene();
        // Defensive source-validity check (mirrors native) — bail
        // if the dragged node disappeared between move and release.
        if self
            .layout_scene
            .active_page()
            .map(|p| p.find(d.source.as_str()).is_none())
            .unwrap_or(true)
        {
            return false;
        }
        use op_editor_ui::widgets::{DropPosition, LayerPanel};
        let layer_rect = self.layer_panel_rect(viewport_h);
        // Source-excluded panel so the indicator y the user saw and
        // the row landed on match the post-commit layout — see the
        // native `commit_layer_drag` for the rationale.
        let panel = LayerPanel::from_editor_with_drag_source(&self.editor_state, &d.source);
        let cursor = Point2D::new(d.current_x, d.current_y);
        let Some(drop) = panel.drop_target_at(layer_rect, cursor) else {
            return true;
        };
        if drop.anchor == d.source {
            return true;
        }
        let source = d.source.clone();
        let anchor = drop.anchor.clone();
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
}

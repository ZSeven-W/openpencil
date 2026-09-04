//! Mouse-release handling for the web `WidgetHost` — marquee commit,
//! layer drag-to-reorder commit, chat-panel corner snap, and drag
//! teardown. Split out of `widget_host.rs` to keep the spine under
//! the repo's 800-line cap.

use op_editor_core::{agent_settings::ImageGenField, AgentSettingsButton, ButtonPressTarget};
use op_editor_ui::widgets::{FontWeightChoice, PropertyPanelAction};
use op_editor_ui::Point2D;

use super::{LayerDragState, MarqueeDragState, WidgetHost};

impl WidgetHost {
    /// Convert a marquee drag (screen-space) into a doc-space
    /// rect, ask the document which top-level nodes overlap it,
    /// and either replace or extend the selection. Mirrors
    /// native `WidgetHostNative::commit_marquee_selection`.
    pub(in crate::widget_host) fn commit_marquee_selection(
        &mut self,
        m: MarqueeDragState,
        _viewport_w: f32,
        _viewport_h: f32,
    ) {
        use op_editor_ui::widgets::marquee_flow;
        if !marquee_flow::marquee_dragged(&m) {
            return;
        }
        self.refresh_layout_scene();
        if marquee_flow::commit_marquee_selection(&mut self.editor_state, &self.layout_scene, &m) {
            self.mark_dirty();
        }
    }

    /// Mouse-release handler — commits any marquee drag, snaps a
    /// chat-panel drag to the nearest corner, or ends the canvas
    /// pan-drag.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        self.last_viewport_w = viewport_w;
        self.last_viewport_h = viewport_h;
        // Dispatch preview pointer up if preview is active
        #[cfg(feature = "canvaskit")]
        if self.editor_state.editor_ui.preview.mode && self.preview.is_some() {
            if self.preview_switcher_release(viewport_w, viewport_h)
                || self.screen_switcher_release(viewport_w, viewport_h)
            {
                self.preview_surface_capture = None;
                return true;
            }
            let consumed = self.preview_dispatch_release();
            self.preview_surface_capture = None;
            return consumed;
        }
        let pressed_released = self.release_pressed_feedback();
        // The rail's slides tab — a row click frames its board, a row
        // drag reorders the deck. Both resolve on release so a press that
        // turned out to be a drag is not also a navigation.
        if self.slides_panel_release(viewport_w, viewport_h) {
            return true;
        }
        // Colour-picker drag end (non-consuming) + floating-panel
        // header drags — see `widget_host/overlay_cursor.rs`.
        if self.release_overlay_drags() {
            return true;
        }
        if self.variables_resize.take().is_some() {
            return true;
        }
        if self.release_selection_handle_drag() {
            return true;
        }
        if self.release_create_drag() {
            return true;
        }
        if self.finish_image_crop_drag() {
            return true;
        }
        if self.release_node_drag() {
            return true;
        }
        if let Some(drag) = self.path_anchor_drag.take() {
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
                return true;
            }
            return false;
        }
        if let Some(m) = self.marquee_drag.take() {
            self.commit_marquee_selection(m, viewport_w, viewport_h);
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
        if self.effect_radius_drag.take().is_some() {
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
        // The cached viewport is what every other viewport-less path
        // here uses; leaving the gesture open would strand a row
        // mid-drag.
        if self.slides_panel_release(self.last_viewport_w, self.last_viewport_h) {
            return true;
        }
        if self.release_overlay_drags() {
            return true;
        }
        if self.variables_resize.take().is_some() {
            return true;
        }
        if self.release_selection_handle_drag() {
            return true;
        }
        if self.release_create_drag() {
            return true;
        }
        if self.finish_image_crop_drag() {
            return true;
        }
        if self.release_node_drag() {
            return true;
        }
        if self.marquee_drag.take().is_some() {
            // Can't compute the doc-space marquee rect without a
            // viewport; drop without committing. The viewport-
            // aware variant is the one runners should call.
            return true;
        }
        if let Some(drag) = self.path_anchor_drag.take() {
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
            }
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
        if self.effect_radius_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging || pressed_released
    }

    fn release_pressed_feedback(&mut self) -> bool {
        let pressed_button = self.editor_state.editor_ui.pressed_button.take();
        let button_released = pressed_button.is_some();
        let icon_picker_released = self
            .editor_state
            .editor_ui
            .icon_picker
            .pressed
            .take()
            .is_some();

        self.commit_deferred_pressed_button(pressed_button);

        let released = button_released || icon_picker_released;
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
        // #13 web-undo parity: wrap the reorder/reparent in history like the
        // native host (op-host-native/click.rs::commit_layer_drag) so Cmd+Z
        // reverses a layer-panel drag.
        self.with_doc_history(|s| match drop.position {
            DropPosition::Before => s.reorder_before(source, anchor),
            DropPosition::After => s.reorder_after(source, anchor),
            DropPosition::Into => s.reorder_into(source, anchor),
        });
        self.mark_dirty();
        true
    }
}

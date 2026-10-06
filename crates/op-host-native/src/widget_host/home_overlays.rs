//! The overlays Home itself opens, painted and pressed ABOVE the Home
//! takeover.
//!
//! Home is a full-surface takeover (see `paint.rs` / `press.rs`), but the
//! things it opens — the agent-settings modal, the sign-in modal, the
//! Home-anchored chat model picker — must sit above it. This module is
//! their seam: it runs ahead of `press_home` in the press ladder and
//! after the Home paint, mirroring the z-order those overlays hold over
//! the rest of the editor (modals above dropdowns, exactly like
//! `press_overlay_tiers` / `paint_topmost_overlays` order them).

use super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_ui::widgets::ai_chat_model_picker::{
    max_picker_scroll, model_picker_hit, paint_model_picker, search_clear_hit, SelectHit,
};
use op_editor_ui::widgets::home_surface::home_model_picker_rects;
use op_editor_ui::widgets::{HomeSurface, PaintCx, Widget};
use op_editor_ui::{Point2D, Rect, RenderBackend};

impl WidgetHostNative {
    /// The overlay tier that runs BEFORE the Home takeover in the press
    /// ladder. `None` when Home (or nothing it opens) claims the press.
    pub(in crate::widget_host) fn press_home_overlays(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.home_visible() {
            return None;
        }
        let (settings_open, login_open, account_menu_open, account_ui) = {
            let ui = &self.editor_state.editor_ui;
            (
                ui.agent_settings_open,
                ui.login_modal_open && (ui.account_ui_available || ui.touch_chrome()),
                ui.account_menu_open && ui.account_ui_available,
                ui.chat_model_picker.open,
            )
        };
        // Same z-order as the normal path: the settings modal (tier 1),
        // then the sign-in modal (tier 2), then the model picker.
        if settings_open {
            return self
                .dispatch_agent_settings_press(x, y, viewport_width, viewport_height)
                .then_some(true);
        }
        if login_open {
            self.dispatch_login_modal_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        // The signed-in half of what Home's avatar opens. Without this
        // the menu painted and then swallowed nothing: every press fell
        // through to the surface underneath and the menu never closed.
        if account_menu_open {
            self.dispatch_account_menu_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        if account_ui {
            return self.press_home_model_picker(x, y, viewport_width, viewport_height);
        }
        None
    }

    /// Paint the Home-opened overlays above the takeover: the agent-
    /// settings modal, the sign-in modal, the signed-in account menu,
    /// then the Home-anchored model picker with its trailing
    /// connect-more row.
    pub(in crate::widget_host) fn paint_home_overlays(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        if !self.home_visible() {
            return;
        }
        self.paint_agent_settings_modal_overlay(frame, viewport_width, viewport_height);
        self.paint_login_modal_overlay(frame, viewport_width, viewport_height);
        self.paint_account_menu_overlay(frame, viewport_width, viewport_height);
        self.paint_home_model_picker(frame, viewport_width, viewport_height);
    }

    /// The signed-in account dropdown. Home paints it for the same
    /// reason it paints the sign-in modal: its avatar opens one of the
    /// two, and a takeover that draws only the signed-OUT half leaves a
    /// signed-in user clicking an avatar that does nothing.
    pub(in crate::widget_host) fn paint_account_menu_overlay(
        &self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        _viewport_height: f32,
    ) {
        let ui = &self.editor_state.editor_ui;
        if !ui.account_ui_available || !ui.account_menu_open {
            return;
        }
        use op_editor_ui::widgets::account_menu::AccountMenu;
        let Some(menu) = AccountMenu::for_editor_ui(ui) else {
            return;
        };
        let menu_rect = op_editor_ui::widgets::touch_overlay_geometry::account_menu_rect(
            &self.editor_state,
            &menu,
            viewport_width,
        );
        let mut cx = op_editor_ui::widgets::PaintCx {
            backend: &mut *frame,
        };
        menu.paint(&mut cx, menu_rect);
    }

    /// The sign-in modal: full-viewport scrim + centred card. Extracted
    /// from the normal-path paint so Home can share the exact card.
    pub(in crate::widget_host) fn paint_login_modal_overlay(
        &self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        let ui = &self.editor_state.editor_ui;
        if !((ui.account_ui_available || ui.touch_chrome()) && ui.login_modal_open) {
            return;
        }
        use op_editor_ui::widgets::login_modal::LoginModal;
        frame.fill_rect(
            Rect::xywh(0.0, 0.0, viewport_width, viewport_height),
            op_editor_ui::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.45,
            },
        );
        let modal = LoginModal::for_editor(&self.editor_state);
        let modal_rect = modal.rect(viewport_width, viewport_height);
        let mut cx = PaintCx {
            backend: &mut *frame,
        };
        modal.paint(&mut cx, modal_rect);
    }

    /// The agent-settings modal: dim scrim across the viewport plus the
    /// panel. Extracted from the normal-path paint so Home can share it.
    pub(in crate::widget_host) fn paint_agent_settings_modal_overlay(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        if !self.editor_state.editor_ui.agent_settings_open {
            return;
        }
        frame.fill_rect(
            Rect::xywh(0.0, 0.0, viewport_width, viewport_height),
            op_editor_ui::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.45,
            },
        );
        let (panel, panel_rect) = self.agent_settings_geometry(viewport_width, viewport_height);
        let mut cx = PaintCx {
            backend: &mut *frame,
        };
        panel.paint(&mut cx, panel_rect);
    }

    /// The Home-anchored picker card + its "接入更多模型…" row.
    fn paint_home_model_picker(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        if !self.editor_state.editor_ui.chat_model_picker.open {
            return;
        }
        let Some(card) = self.home_model_picker_geometry(viewport_width, viewport_height) else {
            return;
        };
        let ui = &self.editor_state.editor_ui;
        let models = &self.editor_state.chat.available_models;
        let selected = self.editor_state.chat.selected_model;
        let locale = ui.locale;
        let mut cx = PaintCx {
            backend: &mut *frame,
        };
        paint_model_picker(
            &mut cx,
            &self.theme,
            card,
            models,
            selected,
            &ui.chat_model_picker,
            &ui.chat_model_picker_input,
            self.now_ms,
            locale,
        );
    }

    /// Resolve the Home-anchored picker rects from the live layout.
    pub(in crate::widget_host) fn home_model_picker_geometry(
        &self,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<Rect> {
        let surface = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let layout = surface.layout(viewport_width, viewport_height);
        let search = self
            .editor_state
            .editor_ui
            .chat_model_picker_input
            .text()
            .to_string();
        home_model_picker_rects(
            &layout,
            viewport_width,
            &self.editor_state.chat.available_models,
            &search,
        )
    }

    /// Press routing for the Home-anchored picker: rows select, the
    /// connect row opens the Agents settings tab, anything else closes.
    fn press_home_model_picker(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        let point = Point2D::new(x, y);
        let Some(card) = self.home_model_picker_geometry(viewport_width, viewport_height) else {
            self.editor_state.editor_ui.close_chat_model_picker();
            self.mark_dirty();
            return Some(true);
        };
        let search = self
            .editor_state
            .editor_ui
            .chat_model_picker_input
            .text()
            .to_string();
        if search_clear_hit(card, point, &search) {
            self.editor_state
                .editor_ui
                .chat_model_picker_input
                .set_text("");
            self.mark_dirty();
            return Some(true);
        }
        let models = self.editor_state.chat.available_models.clone();
        let hit = model_picker_hit(
            &self.editor_state.editor_ui.chat_model_picker,
            card,
            point,
            &models,
            &search,
        );
        match hit {
            SelectHit::Row(index) => {
                // Same mutator the chat panel's picker uses — it updates
                // the selection and closes the picker.
                self.editor_state.select_chat_model(index);
            }
            SelectHit::Inside => {
                self.editor_state
                    .editor_ui
                    .chat_model_picker_input
                    .touch(self.now_ms);
            }
            SelectHit::Outside => {
                self.editor_state.editor_ui.close_chat_model_picker();
            }
        }
        self.mark_dirty();
        Some(true)
    }

    /// Hover bookkeeping for the Home-anchored picker: rows highlight,
    /// leaving the popover closes it (the chat picker's behavior).
    pub(in crate::widget_host) fn cursor_move_home_overlays(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.home_visible() || !self.editor_state.editor_ui.chat_model_picker.open {
            return None;
        }
        let point = Point2D::new(x, y);
        let Some(card) = self.home_model_picker_geometry(viewport_width, viewport_height) else {
            return Some(true);
        };
        let search = self
            .editor_state
            .editor_ui
            .chat_model_picker_input
            .text()
            .to_string();
        let models = self.editor_state.chat.available_models.clone();
        let hit = model_picker_hit(
            &self.editor_state.editor_ui.chat_model_picker,
            card,
            point,
            &models,
            &search,
        );
        let picker = &mut self.editor_state.editor_ui.chat_model_picker;
        let hover = match hit {
            SelectHit::Row(index) => Some(index),
            _ => None,
        };
        picker.hover = hover;
        // Hover must NOT dismiss. This popover is opened by a click and
        // closes on a click outside (the press path below); closing it
        // on mouse-out meant the cursor could not reach its own footer
        // row without the card vanishing on the way.
        self.mark_dirty();
        Some(true)
    }

    /// Wheel routing for the Home-anchored picker; `None` lets the Home
    /// stack (or anything below) keep the scroll.
    pub(in crate::widget_host) fn wheel_home_model_picker(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.home_visible() || !self.editor_state.editor_ui.chat_model_picker.open {
            return None;
        }
        let point = Point2D::new(x, y);
        let card = self.home_model_picker_geometry(viewport_width, viewport_height)?;
        if !card.contains(point) {
            return None;
        }
        let search = self
            .editor_state
            .editor_ui
            .chat_model_picker_input
            .text()
            .to_string();
        let models = self.editor_state.chat.available_models.clone();
        let max = max_picker_scroll(&models, &search);
        let picker = &mut self.editor_state.editor_ui.chat_model_picker;
        let next = (picker.scroll.offset - delta_y).clamp(0.0, max);
        if next != picker.scroll.offset {
            picker.scroll.offset = next;
            self.mark_dirty();
        }
        Some(true)
    }
}

#[cfg(test)]
#[path = "home_overlays_tests.rs"]
mod tests;

//! Studio Home keyboard, wheel and overlay routing for the web host.
//!
//! Web twin of the native `home.rs` text arms, `home_scroll.rs` and
//! `home_overlays.rs`. Home is a takeover, but the things it opens — the
//! agent-settings modal, the sign-in modal, the signed-in account menu and
//! the Home-anchored model picker — sit above it, so their presses, keys and
//! wheel run first.
//!
//! Browser text reaches Home through the ordinary web paths: plain keys via
//! `apply_text`, IME commits and `beforeinput` payloads via
//! `apply_paste_text` (which folds char by char into `apply_text`).

use super::WidgetHost;
use op_editor_ui::widgets::ai_chat_model_picker::{
    max_picker_scroll, model_picker_hit, paint_model_picker, search_clear_hit, SelectHit,
};
use op_editor_ui::widgets::home_surface::{
    home_model_picker_rects, max_scroll_for_mode, model_chip_label, model_chip_width,
};
use op_editor_ui::widgets::{HomeSurface, PaintCx, HOME_TOPBAR_H};
use op_editor_ui::{Point2D, Rect, RenderBackend};

impl WidgetHost {
    /// Whether a modal Home opened owns the keyboard / pointer above it.
    fn home_modal_open(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        ui.agent_settings_open || (ui.account_ui_available && ui.login_modal_open)
    }

    /// Whether the Home composer itself owns the keyboard: Home is up and
    /// none of the overlays it opens (settings, sign-in, model picker) is
    /// taking keys. Copy / cut follow the same owner typing does, so the
    /// chord never reaches a chat input or canvas selection hidden under
    /// Home.
    pub(in crate::widget_host) fn home_composer_owns_keyboard(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        self.home_visible()
            && !ui.agent_settings_open
            && !ui.login_modal_open
            && !ui.chat_model_picker.open
    }

    /// Typed character while Home is up. `None` lets the ordinary ladder run
    /// (the settings modal over Home types into its own fields).
    pub(in crate::widget_host) fn home_text(&mut self, c: char) -> Option<bool> {
        if !self.home_visible() || self.editor_state.editor_ui.agent_settings_open {
            return None;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return Some(self.apply_chat_model_picker_text(c));
        }
        if self.editor_state.editor_ui.login_modal_open || c.is_control() {
            return Some(true);
        }
        if self
            .editor_state
            .editor_ui
            .home
            .insert_text(&c.to_string(), self.now_ms)
        {
            self.mark_dirty();
        }
        Some(true)
    }

    pub(in crate::widget_host) fn home_backspace(&mut self) -> Option<bool> {
        if !self.home_visible() || self.editor_state.editor_ui.agent_settings_open {
            return None;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return Some(self.apply_chat_model_picker_backspace());
        }
        if !self.editor_state.editor_ui.login_modal_open {
            self.editor_state.editor_ui.home.backspace(self.now_ms);
            self.mark_dirty();
        }
        Some(true)
    }

    pub(in crate::widget_host) fn home_delete(&mut self) -> Option<bool> {
        if !self.home_visible() || self.editor_state.editor_ui.agent_settings_open {
            return None;
        }
        if !self.editor_state.editor_ui.login_modal_open
            && !self.editor_state.editor_ui.chat_model_picker.open
        {
            self.editor_state.editor_ui.home.delete_forward(self.now_ms);
            self.mark_dirty();
        }
        Some(true)
    }

    /// ← / → in the composer. `false` when Home does not own the caret.
    pub fn apply_home_caret(&mut self, forward: bool, extend: bool) -> bool {
        if !self.home_visible() || self.home_modal_open() {
            return false;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return true;
        }
        self.editor_state
            .editor_ui
            .home
            .move_caret(forward, extend, self.now_ms);
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn home_select_all(&mut self) -> bool {
        if !self.home_visible() || self.home_modal_open() {
            return false;
        }
        self.editor_state.editor_ui.home.select_all(self.now_ms);
        self.mark_dirty();
        true
    }

    /// Enter while Home is up. `None` lets the settings modal's own Enter
    /// handling run.
    pub(in crate::widget_host) fn home_enter(&mut self) -> Option<bool> {
        if !self.home_visible() || self.editor_state.editor_ui.agent_settings_open {
            return None;
        }
        let ui = &self.editor_state.editor_ui;
        if ui.chat_model_picker.open || ui.login_modal_open {
            return Some(true);
        }
        if ui.home.generation_prompt().is_none() {
            // An EMPTY box stays a no-op on Enter: the one-click start is
            // the button's labelled action, not a stray key.
            return Some(true);
        }
        let sent = if self.home_draft_uses_example() {
            self.home_send()
        } else {
            self.queue_home_send()
        };
        self.mark_dirty();
        Some(sent)
    }

    /// Escape peels Home's own overlays one per press: the connect card, the
    /// 更多 popover, the inline replace strip, then the model picker.
    pub(in crate::widget_host) fn home_escape(&mut self) -> bool {
        // A modal Home opened (settings, sign-in) is closed by its own rung
        // of the ordinary ladder first.
        if !self.home_visible() || self.home_modal_open() {
            return false;
        }
        let home = &mut self.editor_state.editor_ui.home;
        if home.connect_card_open {
            home.connect_card_open = false;
        } else if home.more_open {
            home.more_open = false;
        } else if home.replace_pending {
            home.keep_draft();
        } else if !self.editor_state.editor_ui.escape_chat_model_picker() {
            return false;
        }
        self.mark_dirty();
        true
    }

    /// Caret anchor for the browser IME candidate window.
    pub(in crate::widget_host) fn home_ime_anchor_rect(&self) -> Option<Rect> {
        let home = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        Some(home.focused_input_caret_rect(self.last_viewport_w, self.last_viewport_h))
    }

    /// Wheel while Home is up: the settings modal and the model picker
    /// scroll first, then the page between the pinned top bar and the
    /// bottom. `None` when Home is hidden; Home swallows every other wheel.
    pub(in crate::widget_host) fn wheel_home(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.home_visible() {
            return None;
        }
        if self.editor_state.editor_ui.agent_settings_open {
            return Some(self.try_scroll_agent_settings(
                x,
                y,
                delta_y,
                viewport_width,
                viewport_height,
            ));
        }
        if let Some(scrolled) = self.wheel_home_model_picker(x, y, delta_y) {
            return Some(scrolled);
        }
        if y < HOME_TOPBAR_H {
            return Some(false);
        }
        let chip_w = model_chip_width(&model_chip_label(&self.editor_state));
        let locale = self.editor_state.editor_ui.locale;
        let home = &self.editor_state.editor_ui.home;
        let max_scroll = max_scroll_for_mode(
            viewport_width,
            viewport_height,
            home.task,
            chip_w,
            false,
            locale,
        );
        let next = (home.scroll_y - delta_y).clamp(0.0, max_scroll);
        if (next - home.scroll_y).abs() <= f32::EPSILON {
            return Some(false);
        }
        self.editor_state.editor_ui.home.scroll_y = next;
        self.mark_dirty();
        Some(true)
    }

    fn wheel_home_model_picker(&mut self, x: f32, y: f32, delta_y: f32) -> Option<bool> {
        if !self.editor_state.editor_ui.chat_model_picker.open {
            return None;
        }
        let card = self.home_model_picker_geometry(self.last_viewport_w, self.last_viewport_h)?;
        if !card.contains(Point2D::new(x, y)) {
            return None;
        }
        let search = self.editor_state.editor_ui.chat_model_picker_input.text();
        let max = max_picker_scroll(&self.editor_state.chat.available_models, search);
        let picker = &mut self.editor_state.editor_ui.chat_model_picker;
        let next = (picker.scroll.offset - delta_y).clamp(0.0, max);
        if next == picker.scroll.offset {
            return Some(false);
        }
        picker.scroll.offset = next;
        self.mark_dirty();
        Some(true)
    }

    /// The overlay tier that runs BEFORE the Home takeover in the press
    /// ladder: the settings modal, the sign-in modal, the account menu and
    /// the Home-anchored model picker, in that z-order.
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
        let ui = &self.editor_state.editor_ui;
        if ui.agent_settings_open {
            self.dispatch_agent_settings_press(x, y, viewport_width, viewport_height);
            self.mark_dirty();
            return Some(true);
        }
        if ui.account_ui_available && ui.login_modal_open {
            self.dispatch_login_modal_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        if ui.account_ui_available && ui.account_menu_open {
            self.dispatch_account_menu_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        if ui.chat_model_picker.open {
            self.press_home_model_picker(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        None
    }

    /// Hover above Home: an open modal keeps its own hover wash, the picker
    /// highlights its rows. `None` lets Home's own hover run.
    pub(in crate::widget_host) fn cursor_move_home_overlays(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        if !self.home_visible() {
            return None;
        }
        if self.home_modal_open() || self.editor_state.editor_ui.account_menu_open {
            return Some(self.cursor_move_modal_tiers(x, y).unwrap_or(false));
        }
        if !self.editor_state.editor_ui.chat_model_picker.open {
            return None;
        }
        let card = self.home_model_picker_geometry(self.last_viewport_w, self.last_viewport_h)?;
        let search = self
            .editor_state
            .editor_ui
            .chat_model_picker_input
            .text()
            .to_string();
        let hit = model_picker_hit(
            &self.editor_state.editor_ui.chat_model_picker,
            card,
            Point2D::new(x, y),
            &self.editor_state.chat.available_models,
            &search,
        );
        let hover = match hit {
            SelectHit::Row(index) => Some(index),
            _ => None,
        };
        let picker = &mut self.editor_state.editor_ui.chat_model_picker;
        if picker.hover == hover {
            return Some(false);
        }
        // Hover must NOT dismiss: the popover closes on a click outside.
        picker.hover = hover;
        self.mark_dirty();
        Some(true)
    }

    /// Resolve the Home-anchored picker card from the live layout.
    fn home_model_picker_geometry(
        &self,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<Rect> {
        let surface = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let layout = surface.layout(viewport_width, viewport_height);
        home_model_picker_rects(
            &layout,
            viewport_width,
            &self.editor_state.chat.available_models,
            self.editor_state.editor_ui.chat_model_picker_input.text(),
        )
    }

    /// Rows select, the connect row opens the Agents settings tab, anything
    /// else closes.
    fn press_home_model_picker(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        let point = Point2D::new(x, y);
        let Some(card) = self.home_model_picker_geometry(viewport_width, viewport_height) else {
            self.editor_state.editor_ui.close_chat_model_picker();
            self.mark_dirty();
            return;
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
        } else if op_editor_ui::widgets::ai_chat_model_picker::footer_action_hit(card, point) {
            let ui = &mut self.editor_state.editor_ui;
            ui.close_chat_model_picker();
            ui.agent_settings_open = true;
            ui.agent_settings.tab = op_editor_core::AgentSettingsTab::Agents;
        } else {
            let hit = model_picker_hit(
                &self.editor_state.editor_ui.chat_model_picker,
                card,
                point,
                &self.editor_state.chat.available_models,
                &search,
            );
            match hit {
                SelectHit::Row(index) => {
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
        }
        self.mark_dirty();
    }

    /// Paint the Home takeover plus the overlays it opens, above it.
    pub(in crate::widget_host) fn paint_home(
        &mut self,
        backend: &mut dyn RenderBackend,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.home_visible() {
            return false;
        }
        if self.editor_state.editor_ui.home.shown_at_ms == 0 {
            // First frame since the show: stamp the entrance clock.
            self.editor_state.editor_ui.home.shown_at_ms = self.now_ms.max(1);
        }
        self.prefetch_home_example_template();
        let viewport = Rect::xywh(0.0, 0.0, viewport_width, viewport_height);
        if let Some(home) = HomeSurface::for_editor_at(&self.editor_state, self.now_ms) {
            use op_editor_ui::widgets::Widget;
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            home.paint(&mut cx, viewport);
        }
        self.paint_agent_settings_overlay(backend, viewport_width, viewport_height);
        self.paint_login_modal_overlay(backend, viewport_width, viewport_height);
        self.paint_home_account_menu(backend, viewport_width);
        if self.editor_state.editor_ui.chat_model_picker.open {
            if let Some(card) = self.home_model_picker_geometry(viewport_width, viewport_height) {
                let ui = &self.editor_state.editor_ui;
                let mut cx = PaintCx {
                    backend: &mut *backend,
                };
                paint_model_picker(
                    &mut cx,
                    &self.theme,
                    card,
                    &self.editor_state.chat.available_models,
                    self.editor_state.chat.selected_model,
                    &ui.chat_model_picker,
                    &ui.chat_model_picker_input,
                    self.now_ms,
                    ui.locale,
                );
            }
        }
        // The entrance choreography, the card hover lift and the example art
        // crossfade run on the clock with no input behind them. The caret
        // blink is deliberately not folded in: the web host has no blink
        // scheduler, and it would turn Home into a permanent 60 fps loop.
        let home = &self.editor_state.editor_ui.home;
        let now = self.now_ms;
        if home.entrance_deadline_ms(now).is_some()
            || home.hover_lift_deadline_ms(now).is_some()
            || home.art_deadline_ms(now).is_some()
        {
            crate::repaint_coalescer::request();
        }
        true
    }

    /// The signed-in account menu, hung off Home's avatar.
    fn paint_home_account_menu(&self, backend: &mut dyn RenderBackend, viewport_width: f32) {
        let ui = &self.editor_state.editor_ui;
        if !ui.account_ui_available || !ui.account_menu_open {
            return;
        }
        use op_editor_ui::widgets::account_menu::AccountMenu;
        use op_editor_ui::widgets::Widget;
        let Some(menu) = AccountMenu::for_editor_ui(ui) else {
            return;
        };
        let menu_rect = op_editor_ui::widgets::touch_overlay_geometry::account_menu_rect(
            &self.editor_state,
            &menu,
            viewport_width,
        );
        let mut cx = PaintCx { backend };
        menu.paint(&mut cx, menu_rect);
    }
}

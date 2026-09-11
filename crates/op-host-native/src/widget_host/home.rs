//! Home surface input routing for the native widget host.

use super::WidgetHostNative;
use op_editor_core::{EntrySurface, HomeFamily, HomeHit};
use op_editor_ui::{widgets::HomeSurface, Point2D};

impl WidgetHostNative {
    pub fn home_visible(&self) -> bool {
        self.editor_state.editor_ui.home.visible
    }

    pub(in crate::widget_host) fn press_home(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.home_visible() {
            return None;
        }
        let point = Point2D::new(x, y);
        let home = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let Some(hit) = home.hit_test(viewport_width, viewport_height, point) else {
            return Some(true);
        };
        self.editor_state.editor_ui.home.pressed = Some(hit);
        match hit {
            HomeHit::Sheet => {
                let caret = self.editor_state.editor_ui.home.input.text().len();
                self.editor_state
                    .editor_ui
                    .home
                    .set_caret(caret, self.now_ms);
            }
            HomeHit::Chip(family) => {
                self.editor_state.editor_ui.home.toggle_family(family);
            }
            HomeHit::Card(family) => {
                self.editor_state.editor_ui.home.bind(family);
            }
            HomeHit::Device(device) => {
                self.editor_state.editor_ui.home.device = device;
            }
            HomeHit::Attachment => {
                // The existing chat attachment picker is the one M0-supported
                // image-input path. It will open from the desktop event drain.
                self.editor_state.chat.pending_attachment_pick = true;
            }
            HomeHit::TryExample => {
                let draft = match self.editor_state.editor_ui.home.bound {
                    Some(HomeFamily::KnowledgeCards) => "把这段内容做成一套知识卡片",
                    Some(HomeFamily::ScreenshotTutorial) => "把这几张截图串成一篇步骤教程",
                    Some(HomeFamily::EventPoster) => "做一张周末音乐节活动海报",
                    _ => "做一个三页的取餐预约 mobile app",
                };
                self.editor_state.editor_ui.home.set_draft(draft);
            }
            HomeHit::Professional => {
                self.editor_state.editor_ui.home.visible = false;
                self.editor_state.editor_ui.entry_surface = EntrySurface::Canvas;
            }
            HomeHit::Send => {
                self.queue_home_send();
            }
            HomeHit::NewCanvas => {
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::FileAction::New);
            }
            HomeHit::OpenFile => {
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::FileAction::Open);
            }
            HomeHit::Recent => {
                // Recent projects are listed in a later Home pass; the label
                // remains a safe, consuming target in M0.
            }
            HomeHit::ReferenceLink | HomeHit::Figma | HomeHit::Footer => {
                // Visible but disabled in M0; the eventual tooltip is also
                // deliberately deferred with the link/Figma implementation.
            }
        }
        self.mark_dirty();
        Some(true)
    }

    pub(in crate::widget_host) fn cursor_move_home(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.home_visible() {
            return None;
        }
        let home = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let next = home.hit_test(viewport_width, viewport_height, Point2D::new(x, y));
        if self.editor_state.editor_ui.home.hover == next {
            return Some(true);
        }
        self.editor_state.editor_ui.home.hover = next;
        self.mark_dirty();
        Some(true)
    }

    fn queue_home_send(&mut self) -> bool {
        let Some(prompt) = self.editor_state.editor_ui.home.generation_prompt() else {
            return false;
        };
        self.editor_state.editor_ui.home.visible = false;
        self.editor_state.chat.focus_input_at_end(self.now_ms);
        self.editor_state.chat.set_input_text(prompt);
        let sent = self.editor_state.chat.begin_send();
        self.editor_state.chat.focused = false;
        sent
    }

    pub(in crate::widget_host) fn home_text(&mut self, c: char) -> bool {
        if !self.home_visible() || c.is_control() {
            return false;
        }
        let changed = self
            .editor_state
            .editor_ui
            .home
            .insert_text(&c.to_string(), self.now_ms);
        if changed {
            self.mark_dirty();
        }
        true
    }

    pub(in crate::widget_host) fn home_backspace(&mut self) -> Option<bool> {
        if !self.home_visible() {
            return None;
        }
        self.editor_state.editor_ui.home.backspace(self.now_ms);
        self.mark_dirty();
        Some(true)
    }

    pub(in crate::widget_host) fn home_delete(&mut self) -> Option<bool> {
        if !self.home_visible() {
            return None;
        }
        self.editor_state.editor_ui.home.delete_forward(self.now_ms);
        self.mark_dirty();
        Some(true)
    }

    pub(in crate::widget_host) fn home_caret(&mut self, forward: bool, extend: bool) -> bool {
        if !self.home_visible() {
            return false;
        }
        self.editor_state
            .editor_ui
            .home
            .move_caret(forward, extend, self.now_ms);
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn home_select_all(&mut self) -> bool {
        if !self.home_visible() {
            return false;
        }
        self.editor_state.editor_ui.home.select_all(self.now_ms);
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn home_ime_preedit(
        &mut self,
        text: &str,
        cursor: Option<(usize, usize)>,
    ) -> bool {
        if !self.home_visible() {
            return false;
        }
        let input = &mut self.editor_state.editor_ui.home.input;
        if text.is_empty() {
            input.clear_composition();
        } else {
            let (start, end) = cursor.unwrap_or((text.len(), text.len()));
            input.set_composing_text(text, start, end, self.now_ms);
        }
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn home_ime_commit(&mut self, text: &str) -> bool {
        if !self.home_visible() {
            return false;
        }
        if !text.is_empty() {
            self.editor_state
                .editor_ui
                .home
                .input
                .commit_text(text, self.now_ms);
            self.editor_state.editor_ui.home.draft =
                self.editor_state.editor_ui.home.input.text().to_string();
        }
        self.mark_dirty();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 1440.0;
    const H: f32 = 900.0;

    #[test]
    fn home_typing_and_enter_queue_the_wrapped_chat_turn() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().editor_ui.home.visible = true;
        host.editor_state_mut()
            .editor_ui
            .home
            .bind(HomeFamily::AppUi);
        for character in "取餐预约".chars() {
            assert!(host.apply_text(character));
        }
        assert_eq!(host.editor_state().editor_ui.home.draft, "取餐预约");
        let expected = host
            .editor_state()
            .editor_ui
            .home
            .generation_prompt()
            .unwrap();
        assert!(host.apply_send());
        assert!(!host.home_visible());
        assert_eq!(
            host.editor_state().chat.pending_send.as_deref(),
            Some(expected.as_str())
        );
        assert_eq!(host.editor_state().chat.messages[0].content, expected);
    }

    #[test]
    fn professional_mode_hides_home_and_sets_canvas_preference() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().editor_ui.home.visible = true;
        let home = HomeSurface::for_editor(host.editor_state()).unwrap();
        let rect = home.layout(W, H).professional;
        assert!(host.apply_press(rect.origin.x + 4.0, rect.origin.y + 4.0, W, H));
        assert!(!host.home_visible());
        assert_eq!(
            host.editor_state().editor_ui.entry_surface,
            EntrySurface::Canvas
        );
    }
}

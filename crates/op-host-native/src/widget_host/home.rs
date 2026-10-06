//! Home surface input routing for the native widget host.

use super::WidgetHostNative;
use op_editor_core::{EntrySurface, HomeDevice, HomeFamily, HomeHit, InfoKind, SlideRatio};
use op_editor_ui::widgets::HomeSurface;
use op_editor_ui::Point2D;

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
        let hit = home.hit_test(viewport_width, viewport_height, point);
        drop(home);
        let Some(hit) = hit else {
            // A press outside the 更多 popover closes it, like the
            // prototype's document-level click handler.
            if self.editor_state.editor_ui.home.more_open {
                self.editor_state.editor_ui.home.more_open = false;
                self.mark_dirty();
            }
            return Some(true);
        };
        self.editor_state.editor_ui.home.pressed = Some(hit);
        if self.editor_state.editor_ui.home.more_open
            && !matches!(hit, HomeHit::More | HomeHit::MoreItem(_))
        {
            self.editor_state.editor_ui.home.more_open = false;
        }
        match hit {
            HomeHit::Sheet => {
                let caret = self.editor_state.editor_ui.home.input.text().len();
                self.editor_state
                    .editor_ui
                    .home
                    .set_caret(caret, self.now_ms);
            }
            HomeHit::Tab(family) | HomeHit::MoreItem(family) => {
                self.editor_state
                    .editor_ui
                    .home
                    .set_task(family, self.now_ms);
            }
            HomeHit::More => {
                let open = !self.editor_state.editor_ui.home.more_open;
                self.editor_state.editor_ui.home.more_open = open;
            }
            HomeHit::Segment(index) => {
                let home = &mut self.editor_state.editor_ui.home;
                match home.task {
                    HomeFamily::AppUi => home.set_device(match index {
                        1 => HomeDevice::Desktop,
                        _ => HomeDevice::Mobile,
                    }),
                    HomeFamily::Presentation => home.set_ratio(match index {
                        1 => SlideRatio::Classic43,
                        _ => SlideRatio::Wide169,
                    }),
                    HomeFamily::Infographic => home.set_info_kind(match index {
                        2 => InfoKind::Comparison,
                        1 => InfoKind::Flow,
                        _ => InfoKind::Data,
                    }),
                    _ => {}
                }
                home.art_switched_at_ms = self.now_ms.max(1);
            }
            HomeHit::Attachment => {
                // The existing chat attachment picker is the one M1-supported
                // image-input path. It opens from the desktop event drain;
                // the staged list it produces is the composer's source of truth.
                self.editor_state.chat.pending_attachment_pick = true;
            }
            HomeHit::UseExample => {
                let home = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)?;
                let example = home.example_prompt().to_string();
                drop(home);
                self.editor_state.editor_ui.home.use_example(&example);
            }
            HomeHit::BackToWorkspace => {
                // The same footer rect, the workspace-active reading:
                // drop the Home takeover and take the workspace back
                // (state, view, and phase were all kept).
                self.editor_state.editor_ui.home.hide();
                self.editor_state.editor_ui.workspace.reenter(self.now_ms);
            }
            HomeHit::ReplaceKeep => {
                self.editor_state.editor_ui.home.keep_draft();
            }
            HomeHit::ReplaceConfirm => {
                let home = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)?;
                let example = home.example_prompt().to_string();
                drop(home);
                self.editor_state
                    .editor_ui
                    .home
                    .confirm_replace_example(&example);
            }
            HomeHit::ExploreCard(family) => {
                let home = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)?;
                let example = home.example_prompt_for(family).to_string();
                drop(home);
                let home = &mut self.editor_state.editor_ui.home;
                home.set_task(family, self.now_ms);
                home.use_example(&example);
            }
            HomeHit::Professional => {
                self.editor_state.editor_ui.home.hide();
                self.editor_state.editor_ui.entry_surface = EntrySurface::Canvas;
            }
            HomeHit::ModelChip => {
                if !self.editor_state.has_usable_chat_agent() {
                    // Nothing can answer yet — the chip becomes the
                    // connect card instead of a picker over an empty
                    // catalog.
                    self.editor_state.editor_ui.home.connect_card_open = true;
                } else {
                    // Same open path the chat panel's model pill takes.
                    let opening = self.editor_state.editor_ui.toggle_chat_model_picker();
                    if opening {
                        self.editor_state.rebuild_chat_models();
                        self.editor_state.editor_ui.close_parallel_agents_picker();
                        self.editor_state
                            .editor_ui
                            .chat_model_picker_input
                            .touch(self.now_ms);
                    }
                }
            }
            HomeHit::ConnectFreeTier => {
                self.editor_state.editor_ui.home.connect_card_open = false;
                // TODO(hosted-quota): the free tier becomes its own
                // hosted-quota sign-up later; for now it opens the plain
                // sign-in modal.
                if self.editor_state.editor_ui.account_ui_available {
                    self.editor_state.editor_ui.login_modal_open = true;
                    self.editor_state.editor_ui.login_modal_hover = None;
                }
            }
            HomeHit::ConnectApiKey | HomeHit::ConnectCli => {
                self.editor_state.editor_ui.home.connect_card_open = false;
                self.editor_state.editor_ui.agent_settings_open = true;
                self.editor_state.editor_ui.agent_settings.tab =
                    op_editor_core::AgentSettingsTab::Agents;
            }
            HomeHit::ConnectClose => {
                self.editor_state.editor_ui.home.connect_card_open = false;
            }
            HomeHit::Send => {
                if !self.editor_state.has_usable_chat_agent() {
                    // The send would fail silently — open the connect
                    // card instead of queueing a dead turn.
                    self.editor_state.editor_ui.home.connect_card_open = true;
                } else {
                    self.queue_home_send();
                }
            }
            HomeHit::NewCanvas => {
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::FileAction::New);
            }
            HomeHit::Account => {
                // Same two destinations the professional TopBar's avatar
                // opens — signed in goes to the account menu, signed out
                // to the login modal. Routed through one place so the two
                // entry points cannot answer differently.
                self.open_account_entry();
            }
            HomeHit::OpenFile => {
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::FileAction::Open);
            }
            HomeHit::Recent(index) => {
                if index < self.editor_state.editor_ui.recent_files.len() {
                    self.editor_state.editor_ui.pending_file_action =
                        Some(op_editor_core::FileAction::OpenRecent(index));
                }
            }
            HomeHit::ReferenceLink | HomeHit::Figma => {
                // Visible but disabled in M1; the tooltip explains why.
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
        let previous = self.editor_state.editor_ui.home.hover;
        let card_of = |hit: Option<HomeHit>| match hit {
            Some(HomeHit::ExploreCard(family)) => Some(family),
            _ => None,
        };
        let (from, to) = (card_of(previous), card_of(next));
        if from.is_some() || to.is_some() {
            self.editor_state
                .editor_ui
                .home
                .stamp_card_hover(from, self.now_ms);
        }
        self.editor_state.editor_ui.home.hover = next;
        self.mark_dirty();
        Some(true)
    }

    /// Open the account entry: the menu when signed in, the login modal
    /// when not. ONE answer for the two surfaces that offer it (the
    /// professional TopBar's avatar and Home's), so a first-run user
    /// cannot find a way in on one screen and a different one on the
    /// other. Returns whether the host has an account gate at all.
    pub(in crate::widget_host) fn open_account_entry(&mut self) -> bool {
        if !self.editor_state.editor_ui.account_ui_available {
            return false;
        }
        if self.editor_state.editor_ui.account.is_signed_in() {
            self.editor_state.editor_ui.account_menu_open = true;
            self.editor_state.editor_ui.account_menu_hover = None;
        } else {
            self.editor_state.editor_ui.login_modal_open = true;
            self.editor_state.editor_ui.login_modal_hover = None;
        }
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn queue_home_send(&mut self) -> bool {
        let Some(prompt) = self.editor_state.editor_ui.home.generation_prompt() else {
            return false;
        };
        // A brief started from Home is a NEW deliverable, so it needs a
        // page of its own. Without this the run appends to whatever the
        // last one drew and the two designs share a canvas, a deck strip
        // and a transcript (measured 2026-09-13: a 演示文稿 brief landed
        // its 5 slides beside the previous coffee app's 3 screens, and
        // the strip listed all 8 as one deck). A page that is still the
        // untouched starter has nothing to protect, so the very first
        // run keeps the document it was launched on.
        if !op_editor_core::blank_starter::active_page_is_blank_starter(&self.editor_state) {
            self.start_fresh_document_for_home();
        }
        // Open the generation workspace on the SAME document: the chat
        // pins into the dock and the canvas renders the boards the run
        // produces. Family, brief, and the task's options are captured
        // now because Home is about to hide. `run_epoch = 0` until the
        // desktop launch stamps the live agent epoch.
        let family = self.editor_state.editor_ui.home.task;
        let brief = self.editor_state.editor_ui.home.draft.trim().to_string();
        let options = self.editor_state.editor_ui.home.task_draft().clone();
        let previous_tool = Some(self.editor_state.tool);
        {
            // The workspace opener also opens the LEFT PANEL on the
            // Chat tab (the one-time width bump included) — the dock is
            // the rail, so a run always lands with its conversation
            // visible and Professional Editing keeps it that way.
            self.editor_state.editor_ui.open_workspace_for_generation(
                family,
                brief,
                options,
                0,
                self.now_ms,
                previous_tool,
            );
        }
        // Pan-only viewing while the workspace owns the canvas; the
        // previous tool is restored on 专业编辑.
        self.editor_state.tool = op_editor_core::Tool::Hand;
        self.editor_state.editor_ui.home.hide();
        self.editor_state.chat.focus_input_at_end(self.now_ms);
        self.editor_state.chat.set_input_text(prompt);
        // Home briefs are whole-design requests — pin the turn to the
        // orchestrator pipeline (reasoning-budget models finish there;
        // the design-agent loop burns their budget thinking). The
        // desktop launcher consumes the route on the next drain.
        self.editor_state.chat.launch_route = op_editor_core::LaunchRoute::Orchestrator;
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
            let home = &mut self.editor_state.editor_ui.home;
            home.input.commit_text(text, self.now_ms);
            home.sync_committed_input();
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

    /// Paint one Home frame into a fresh raster surface (the same harness
    /// shape the pan-cache tests use) so the stamping in `paint` runs.
    fn paint_home_once(host: &mut WidgetHostNative) {
        let mut backend = crate::backend::NativeBackend::with_dpi(1.0);
        let mut surface =
            skia_safe::surfaces::raster_n32_premul((320, 240)).expect("raster surface allocated");
        surface.canvas().clear(skia_safe::Color::WHITE);
        let mut frame = crate::backend::NativeFrameBackend::new(&mut backend, surface.canvas());
        host.paint(&mut frame, 320.0, 240.0);
    }

    #[test]
    fn painting_home_stamps_the_entrance_clock_and_hiding_resets_it() {
        let mut host = WidgetHostNative::new();
        host.editor_state.editor_ui.home.visible = true;
        host.set_now_ms(5_000);
        paint_home_once(&mut host);
        assert_eq!(host.editor_state().editor_ui.home.shown_at_ms, 5_000);

        host.editor_state_mut().editor_ui.home.hide();
        assert_eq!(
            host.editor_state().editor_ui.home.shown_at_ms,
            0,
            "hiding resets the stamp so the next show animates again"
        );

        host.editor_state_mut().editor_ui.home.visible = true;
        host.set_now_ms(20_000);
        paint_home_once(&mut host);
        assert_eq!(host.editor_state().editor_ui.home.shown_at_ms, 20_000);
    }

    #[test]
    fn home_typing_and_enter_queue_the_wrapped_chat_turn() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().editor_ui.home.visible = true;
        for character in "取餐预约".chars() {
            assert!(host.apply_text(character));
        }
        assert_eq!(host.editor_state().editor_ui.home.draft, "取餐预约");
        assert_eq!(
            host.editor_state().editor_ui.home.task_draft().text,
            "取餐预约",
            "typing lands in the active task's draft"
        );
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
    fn switching_tabs_and_back_restores_each_task_draft() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().editor_ui.home.visible = true;
        for character in "取餐预约".chars() {
            assert!(host.apply_text(character));
        }
        let home = HomeSurface::for_editor(host.editor_state()).expect("home");
        let layout = home.layout(W, H);
        let poster_tab = center(layout.tabs[6]);
        assert!(host.apply_press(poster_tab.x, poster_tab.y, W, H));
        assert_eq!(
            host.editor_state().editor_ui.home.task,
            HomeFamily::EventPoster
        );
        assert!(host.editor_state().editor_ui.home.draft.is_empty());
        let home = HomeSurface::for_editor(host.editor_state()).expect("home");
        let layout = home.layout(W, H);
        let app_tab = center(layout.tabs[0]);
        assert!(host.apply_press(app_tab.x, app_tab.y, W, H));
        assert_eq!(host.editor_state().editor_ui.home.draft, "取餐预约");
    }

    #[test]
    fn send_with_an_empty_draft_is_a_noop() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().editor_ui.home.visible = true;
        let home = HomeSurface::for_editor(host.editor_state()).expect("home");
        let send = center(home.layout(W, H).send);
        assert!(host.apply_press(send.x, send.y, W, H));
        assert!(
            host.editor_state().chat.pending_send.is_none(),
            "an empty draft queues nothing"
        );
        assert!(
            host.editor_state().editor_ui.home.connect_card_open,
            "with no usable agent the connect card opens instead"
        );
    }

    #[test]
    fn using_the_example_fills_the_draft() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().editor_ui.home.visible = true;
        let expected = {
            let home = HomeSurface::for_editor(host.editor_state()).expect("home");
            home.example_prompt().to_string()
        };
        let use_example = {
            let home = HomeSurface::for_editor(host.editor_state()).expect("home");
            center(home.layout(W, H).use_example)
        };
        assert!(host.apply_press(use_example.x, use_example.y, W, H));
        assert_eq!(host.editor_state().editor_ui.home.draft, expected);
    }

    #[test]
    fn a_busy_draft_arms_the_inline_replace_strip() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().editor_ui.home.visible = true;
        host.editor_state_mut()
            .editor_ui
            .home
            .set_draft("我自己的需求");
        let home = HomeSurface::for_editor(host.editor_state()).expect("home");
        let expected = home.example_prompt().to_string();
        let layout = home.layout(W, H);
        let use_example = center(layout.use_example);
        assert!(host.apply_press(use_example.x, use_example.y, W, H));
        assert!(host.editor_state().editor_ui.home.replace_pending);
        assert_eq!(host.editor_state().editor_ui.home.draft, "我自己的需求");
        // 使用示例 replaces the draft via the strip.
        let home = HomeSurface::for_editor(host.editor_state()).expect("home");
        let layout = home.layout(W, H);
        let confirm = center(layout.replace_use);
        assert!(host.apply_press(confirm.x, confirm.y, W, H));
        assert!(!host.editor_state().editor_ui.home.replace_pending);
        assert_eq!(host.editor_state().editor_ui.home.draft, expected);
    }

    #[test]
    fn professional_mode_hides_home_and_sets_canvas_preference() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().editor_ui.home.visible = true;
        let home = HomeSurface::for_editor(host.editor_state()).expect("home");
        let rect = home.layout(W, H).professional;
        assert!(host.apply_press(rect.origin.x + 4.0, rect.origin.y + 4.0, W, H));
        assert!(!host.home_visible());
        assert_eq!(
            host.editor_state().editor_ui.entry_surface,
            EntrySurface::Canvas
        );
    }

    fn center(rect: op_editor_ui::Rect) -> Point2D {
        Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        )
    }
}

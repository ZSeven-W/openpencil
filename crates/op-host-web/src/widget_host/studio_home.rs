//! Studio Home press / hover routing for the web widget host.
//!
//! The web twin of `op-host-native/src/widget_host/home.rs`. Home is a
//! full-surface takeover painted from the shared `HomeSurface` widget; this
//! module maps its hit-test onto the shared `HomeState` transitions. The
//! browser only ever runs the desktop layout (`touch_chrome()` is never set
//! here), so the phone-only arms — the works reader and the software-keyboard
//! reveal — fall back to their desktop readings.

use super::WidgetHost;
use op_editor_core::{EntrySurface, HomeDevice, HomeFamily, HomeHit, InfoKind, SlideRatio};
use op_editor_ui::widgets::HomeSurface;
use op_editor_ui::Point2D;

impl WidgetHost {
    pub(crate) fn home_visible(&self) -> bool {
        self.editor_state.editor_ui.home.visible
    }

    /// The Home takeover tier. `None` while Home is hidden so the ordinary
    /// ladder runs; otherwise Home owns the press outright.
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
        let home = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let hit = home.hit_test(viewport_width, viewport_height, Point2D::new(x, y));
        drop(home);
        let Some(hit) = hit else {
            // A press outside the 更多 popover closes it, like the
            // prototype's document-level click handler.
            self.editor_state.editor_ui.home.composer_focused = false;
            if self.editor_state.editor_ui.home.more_open {
                self.editor_state.editor_ui.home.more_open = false;
                self.mark_dirty();
            }
            return Some(true);
        };
        let home = &mut self.editor_state.editor_ui.home;
        home.pressed = Some(hit);
        home.composer_focused = matches!(hit, HomeHit::Sheet);
        if home.more_open && !matches!(hit, HomeHit::More | HomeHit::MoreItem(_)) {
            home.more_open = false;
        }
        self.run_home_hit(hit);
        self.mark_dirty();
        Some(true)
    }

    fn run_home_hit(&mut self, hit: HomeHit) {
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
                // The browser file picker behind the chat attachment button;
                // `dom_io::drain_pending_attachment_pick` opens it after the
                // press returns.
                self.editor_state.chat.pending_attachment_pick = true;
            }
            HomeHit::UseExample => {
                let example = self.home_example_prompt(None);
                self.editor_state.editor_ui.home.use_example(&example);
            }
            HomeHit::BackToWorkspace | HomeHit::WorksCurrent => {
                // Drop the Home takeover and take the workspace back (state,
                // view and phase were all kept). Without a live workspace the
                // current work is simply the canvas.
                self.editor_state.editor_ui.home.hide();
                if self.editor_state.editor_ui.workspace.active {
                    self.editor_state.editor_ui.workspace.reenter(self.now_ms);
                } else {
                    self.editor_state.editor_ui.entry_surface = EntrySurface::Canvas;
                }
            }
            HomeHit::ReplaceKeep => self.editor_state.editor_ui.home.keep_draft(),
            HomeHit::ReplaceConfirm => {
                let example = self.home_example_prompt(None);
                self.editor_state
                    .editor_ui
                    .home
                    .confirm_replace_example(&example);
            }
            HomeHit::ExploreCard(family) => {
                let example = self.home_example_prompt(Some(family));
                let home = &mut self.editor_state.editor_ui.home;
                home.set_task(family, self.now_ms);
                home.use_example(&example);
            }
            HomeHit::Professional => {
                self.editor_state.editor_ui.home.hide();
                self.editor_state.editor_ui.entry_surface = EntrySurface::Canvas;
            }
            HomeHit::ModeNormal => {
                // Home IS the normal mode: the 普通 half of the switch only
                // confirms the segment already selected.
            }
            HomeHit::NavCreate => {
                let home = &mut self.editor_state.editor_ui.home;
                home.works_open = false;
                home.scroll_y = 0.0;
            }
            HomeHit::NavProjects => {
                let home = &mut self.editor_state.editor_ui.home;
                home.works_open = true;
                home.composer_focused = false;
                home.more_open = false;
            }
            HomeHit::WorksRecent(index) | HomeHit::Recent(index) => {
                // The daemon performs the load (`/api/file/open-recent`);
                // Home hides once the reply installs the document.
                if index < self.editor_state.editor_ui.recent_files.len() {
                    self.editor_state.editor_ui.pending_file_action =
                        Some(op_editor_core::FileAction::OpenRecent(index));
                }
            }
            HomeHit::NavSettings | HomeHit::ConnectApiKey | HomeHit::ConnectCli => {
                // The web settings modal is the connect path for both rows:
                // a browser has no local CLI, and the Agents tab carries the
                // API-key form.
                self.editor_state.editor_ui.home.connect_card_open = false;
                self.editor_state.editor_ui.agent_settings_open = true;
                self.editor_state.editor_ui.agent_settings.tab =
                    op_editor_core::AgentSettingsTab::Agents;
            }
            HomeHit::ModelChip => self.press_home_model_chip(),
            HomeHit::ConnectFreeTier => {
                self.editor_state.editor_ui.home.connect_card_open = false;
                if self.editor_state.editor_ui.account_ui_available {
                    self.editor_state.editor_ui.login_modal_open = true;
                    self.editor_state.editor_ui.login_modal_hover = None;
                }
            }
            HomeHit::ConnectClose => {
                self.editor_state.editor_ui.home.connect_card_open = false;
            }
            HomeHit::Send => {
                self.home_send();
            }
            HomeHit::NewCanvas => {
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::FileAction::New);
            }
            HomeHit::Account => {
                self.open_account_entry();
            }
            HomeHit::OpenFile => {
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::FileAction::Open);
            }
            HomeHit::ReferenceLink | HomeHit::Figma => {
                // Visible but disabled in M1; the tooltip explains why.
            }
            // Never laid out on web (`variants_unavailable`).
            HomeHit::Variants => {}
        }
    }

    /// The active (or named) task's example prompt — what 使用这个示例
    /// fills.
    pub(in crate::widget_host) fn home_example_prompt(&self, family: Option<HomeFamily>) -> String {
        HomeSurface::for_editor_at(&self.editor_state, self.now_ms)
            .map(|home| match family {
                Some(family) => home.example_prompt_for(family).to_string(),
                None => home.example_prompt().to_string(),
            })
            .unwrap_or_default()
    }

    fn press_home_model_chip(&mut self) {
        if !self.editor_state.has_usable_chat_agent() {
            // Nothing can answer yet — the chip becomes the connect card
            // instead of a picker over an empty catalog.
            self.editor_state.editor_ui.home.connect_card_open = true;
            return;
        }
        // The web catalog is reconciled from the daemon every frame
        // (`web_chat::reconcile_models`), so opening only toggles the card.
        if self.editor_state.editor_ui.toggle_chat_model_picker() {
            self.editor_state.editor_ui.close_parallel_agents_picker();
            self.editor_state
                .editor_ui
                .chat_model_picker_input
                .touch(self.now_ms);
        }
    }

    /// Hover bookkeeping; Home owns the cursor while it is up.
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
        drop(home);
        if self.editor_state.editor_ui.home.hover == next {
            return Some(false);
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
    /// when not — one answer for the TopBar avatar and Home's.
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
}

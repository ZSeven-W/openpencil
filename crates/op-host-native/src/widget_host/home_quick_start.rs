//! Home's one-click start: 开始设计 on an empty box (or on the unchanged
//! example) opens the task's example as an instant template draft in the
//! generation workspace, then — when a model can answer — refines those
//! same boards toward the example brief.
//!
//! The decisions (which template, what Send offers, the refine wording)
//! are platform-free in `op_editor_core::editor_ui_state::home_example_draft`;
//! this module is the host arm every native shell shares — desktop and
//! the touch hosts both drive `WidgetHostNative`. It loads the template
//! through the same shared adopt path the native template picker uses
//! (`EditorState::adopt_template_boards` + the scene-template install seam),
//! on the SAME document the workspace then shows, so the draft is on
//! screen in the frame that follows the press — no model call, no file IO.

use super::WidgetHostNative;
use op_editor_core::scene_template_append::template_boards;
use op_editor_core::scene_template_catalog::{scene_template_by_id, scene_template_document};
use op_editor_core::{ChatMessage, HomeSendMode, LaunchRoute, NodeId, SelectionState, Tool};
use op_editor_ui::widgets::HomeSurface;

impl WidgetHostNative {
    /// The Send press (button or Enter): one entry that reads the same
    /// [`HomeSendMode`] the button paints. Returns whether anything was
    /// queued or opened.
    pub(in crate::widget_host) fn home_send(&mut self) -> bool {
        let (mode, example) = {
            let Some(home) = HomeSurface::for_editor_at(&self.editor_state, self.now_ms) else {
                return false;
            };
            (home.send_mode(), home.example_prompt().to_string())
        };
        match mode {
            HomeSendMode::Connect => {
                // Nothing can run the brief — open the connect card instead
                // of queueing a dead turn.
                self.editor_state.editor_ui.home.connect_card_open = true;
                true
            }
            HomeSendMode::Start => self.queue_home_send(),
            HomeSendMode::UseExample => self.start_home_from_example(&example),
        }
    }

    /// Whether the Home box holds the unchanged example (or nothing).
    pub(in crate::widget_host) fn home_draft_uses_example(&self) -> bool {
        HomeSurface::for_editor_at(&self.editor_state, self.now_ms)
            .is_some_and(|home| home.state.draft_uses_example(home.example_prompt()))
    }

    /// Start from the active task's example. The example is written into
    /// the box first, so what runs is visibly what the user was shown.
    fn start_home_from_example(&mut self, example: &str) -> bool {
        self.editor_state.editor_ui.home.set_draft(example);
        if let Some(template) = self.editor_state.editor_ui.home.example_draft_template() {
            if self.open_home_template_draft(template) {
                return true;
            }
            // A template that fails to load must not strand the press:
            // fall through to an ordinary generation of the example.
        }
        if !self.editor_state.has_usable_chat_agent() {
            self.editor_state.editor_ui.home.connect_card_open = true;
            return true;
        }
        self.queue_home_send()
    }

    /// Load `template_id` as the instant draft into a fresh generation
    /// workspace, then queue its refine turn when a model can answer (or
    /// leave the draft + connect banner when none can).
    pub(in crate::widget_host) fn open_home_template_draft(
        &mut self,
        template_id: &'static str,
    ) -> bool {
        let Some(boards) = scene_template_document(template_id)
            .and_then(|source| template_boards(source, template_id))
        else {
            return false;
        };
        // A brief started from Home is a NEW deliverable: a page that holds
        // real work is parked for the shell (never silently dropped) and a
        // fresh starter takes its place — the same swap a typed brief makes.
        if !op_editor_core::blank_starter::active_page_is_blank_starter(&self.editor_state)
            && !self.start_fresh_document_for_home()
        {
            return false;
        }
        let mut next = self.editor_state.clone();
        if !next.adopt_template_boards(boards) {
            return false;
        }
        next.editor_ui.scenario = scene_template_by_id(template_id).map(|template| template.scene);
        next.clear_selection();
        if self.install_home_template_draft_state(next, true).is_err() {
            return false;
        }
        // A staged brand kit re-skins the template draft through its
        // variables before the refine turn reads the document.
        self.apply_staged_home_brand();

        let home = &self.editor_state.editor_ui.home;
        let family = home.task;
        let brief = home.draft.trim().to_string();
        let options = home.task_draft().clone();
        let previous_tool = Some(self.editor_state.tool);
        self.editor_state.editor_ui.open_workspace_for_generation(
            family,
            brief.clone(),
            options,
            0,
            self.now_ms,
            previous_tool,
        );
        self.editor_state.tool = Tool::Hand;
        self.editor_state.editor_ui.home.hide();

        let refine_now = self.editor_state.has_usable_chat_agent();
        self.editor_state
            .editor_ui
            .workspace
            .adopt_template_draft(template_id, refine_now);
        if refine_now {
            self.queue_draft_refine();
        } else {
            self.note_draft_awaiting_model(template_id, &brief);
        }
        // The draft is on the page now: frame it for the family's view
        // instead of leaving the starter's camera behind.
        let (viewport_w, viewport_h) = (self.last_viewport_w, self.last_viewport_h);
        if viewport_w > 0.0 && viewport_h > 0.0 {
            self.apply_workspace_fit(viewport_w, viewport_h);
        }
        self.mark_dirty();
        true
    }

    /// Queue the in-place refine of the workspace's template draft: select
    /// its boards (the modify route edits exactly the frames selected when
    /// the turn launches) and send the refine brief pinned to
    /// [`LaunchRoute::Refine`]. Returns whether a turn was queued.
    pub(in crate::widget_host) fn queue_draft_refine(&mut self) -> bool {
        let workspace = &self.editor_state.editor_ui.workspace;
        if !workspace.active || workspace.draft_template.is_none() {
            return false;
        }
        let family = workspace.family;
        let brief = workspace.brief.trim().to_string();
        let boards = op_editor_core::preview_slideshow::active_page_boards(&self.editor_state);
        let Some(anchor) = boards.last().cloned() else {
            return false;
        };
        self.editor_state.selection = SelectionState {
            anchor: NodeId::new(anchor),
            set: boards.into_iter().map(NodeId::new).collect(),
        };
        self.editor_state.editor_ui.workspace.begin_draft_refine();
        let chat = &mut self.editor_state.chat;
        chat.title_from_prompt_if_untitled(&brief);
        chat.focus_input_at_end(self.now_ms);
        chat.set_input_text(op_editor_core::refine_prompt(family, &brief));
        chat.launch_route = LaunchRoute::Refine;
        let sent = chat.begin_send();
        chat.focused = false;
        self.mark_dirty();
        sent
    }

    /// No model: the draft IS the result for now. The conversation still
    /// records what was asked and says how to take it further, so the
    /// dock is not a blank column beside the draft.
    fn note_draft_awaiting_model(&mut self, template_id: &str, brief: &str) {
        let locale = self.editor_state.editor_ui.locale;
        let name = scene_template_by_id(template_id)
            .map(|template| template.title_for_locale(locale))
            .unwrap_or(template_id);
        let note = op_i18n::translate(locale, "workspace.draft.chatNote").replace("{{name}}", name);
        let chat = &mut self.editor_state.chat;
        chat.title_from_prompt_if_untitled(brief);
        chat.messages.push(ChatMessage::user(brief));
        chat.messages.push(ChatMessage::assistant(note));
        chat.expand();
    }

    /// The draft banner's action: refine once a model can answer, else
    /// the connect path (the same Agents settings the connect card's key
    /// and CLI rows open).
    pub(in crate::widget_host) fn run_workspace_draft_action(&mut self) {
        if self.editor_state.has_usable_chat_agent() {
            self.queue_draft_refine();
        } else {
            self.editor_state.editor_ui.agent_settings_open = true;
            self.editor_state.editor_ui.agent_settings.tab =
                op_editor_core::AgentSettingsTab::Agents;
        }
    }
}

#[cfg(test)]
#[path = "home_quick_start_tests.rs"]
mod tests;

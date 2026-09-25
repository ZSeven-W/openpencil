//! Studio Home's send path on the web host: the typed brief, the one-click
//! example draft, and the document swap a new deliverable needs.
//!
//! Web twin of the native `home.rs::queue_home_send` +
//! `home_quick_start.rs` + `home_document_swap.rs`. The decisions (what Send
//! offers, which template, the refine wording) stay in `op_editor_core`; this
//! module is the browser arm.
//!
//! ## The replaced document
//!
//! A brief sent from Home is a NEW deliverable, so a page that already holds
//! work is swapped for a fresh starter before the run starts. Desktop parks
//! the replaced document for its shell, which saves unsaved work as a rescue
//! copy. The browser has no file system to write that copy to and no
//! autosave store, so the web host asks instead — and only when there is
//! something to lose:
//!
//! * the page is still the untouched starter → no swap at all;
//! * the document has no unsaved changes → swap silently (it is saved);
//! * the document has unsaved changes → the send is parked as a
//!   [`HomeReplaceIntent`] and the DOM layer asks with `window.confirm`
//!   (`studio_web::drain_home_replace_confirm`). Confirming runs the same
//!   send with the swap allowed; cancelling leaves Home, the brief and the
//!   document exactly as they were.
//!
//! A Retry from a stopped run replaces only that run's partial boards, which
//! is what the user asked for, so it swaps without asking.
//!
//! Every swap also unbinds the daemon's file (`--file` / a recent open):
//! the fresh page is a new, untitled deliverable, so File > Save must
//! download it instead of overwriting the file the daemon was bound to. A
//! send on the untouched starter swaps nothing and unbinds nothing.

use super::WidgetHost;
use op_editor_core::scene_template_append::template_boards;
use op_editor_core::scene_template_catalog::{
    scene_template_by_id, scene_template_document, scene_template_document_route,
};
use op_editor_core::{ChatMessage, HomeSendMode, LaunchRoute, NodeId, SelectionState, Tool};
use op_editor_ui::widgets::HomeSurface;

/// A Home send waiting for the user to agree to discard unsaved work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeReplaceIntent {
    /// The typed brief (or the example, generated rather than drafted).
    Brief,
    /// The one-click example draft built from this scene template.
    TemplateDraft(&'static str),
    /// "Import this website": the imported page replaces the document.
    ImportSite,
}

impl WidgetHost {
    /// The Send press (button or Enter on the example): one entry that
    /// reads the same [`HomeSendMode`] the button paints.
    pub(in crate::widget_host) fn home_send(&mut self) -> bool {
        let (mode, example) = {
            let Some(home) = HomeSurface::for_editor_at(&self.editor_state, self.now_ms) else {
                return false;
            };
            (home.send_mode(), home.example_prompt().to_string())
        };
        match mode {
            HomeSendMode::Connect => {
                self.editor_state.editor_ui.home.connect_card_open = true;
                true
            }
            HomeSendMode::Start => self.queue_home_send(),
            HomeSendMode::UseExample => self.start_home_from_example(&example),
            // Offered only when the daemon reports `siteImport` (see
            // `studio_web::probe_bound_file`); the fetch runs there.
            HomeSendMode::ImportSite => self.home_site_import_press(false),
        }
    }

    /// Whether the Home box holds the unchanged example (or nothing).
    pub(in crate::widget_host) fn home_draft_uses_example(&self) -> bool {
        HomeSurface::for_editor_at(&self.editor_state, self.now_ms)
            .is_some_and(|home| home.state.draft_uses_example(home.example_prompt()))
    }

    /// Queue the typed brief as a whole-design run in a fresh workspace.
    pub(in crate::widget_host) fn queue_home_send(&mut self) -> bool {
        self.queue_home_send_with(false)
    }

    fn queue_home_send_with(&mut self, discard_confirmed: bool) -> bool {
        let Some(prompt) = self.editor_state.editor_ui.home.generation_prompt() else {
            return false;
        };
        if !self.make_room_for_home_run(HomeReplaceIntent::Brief, discard_confirmed) {
            return false;
        }
        let family = self.editor_state.editor_ui.home.task;
        let brief = self.editor_state.editor_ui.home.draft.trim().to_string();
        let options = self.editor_state.editor_ui.home.task_draft().clone();
        let previous_tool = Some(self.editor_state.tool);
        // The workspace opener also pins the chat into the left panel, so
        // the run lands with its conversation visible. `run_epoch = 0` until
        // the web chat launch stamps the turn's generation.
        self.editor_state.editor_ui.open_workspace_for_generation(
            family,
            brief,
            options,
            0,
            self.now_ms,
            previous_tool,
        );
        self.editor_state.tool = Tool::Hand;
        self.editor_state.editor_ui.home.hide();
        self.editor_state.chat.focus_input_at_end(self.now_ms);
        self.editor_state.chat.set_input_text(prompt);
        // Home briefs are whole-design requests: the daemon's standard route
        // honours the pinned route instead of re-classifying the wording.
        self.editor_state.chat.launch_route = LaunchRoute::Orchestrator;
        let sent = self.begin_chat_send();
        self.editor_state.chat.focused = false;
        self.mark_dirty();
        sent
    }

    /// Swap in a fresh starter when the page already holds work. Returns
    /// `false` when the swap needs the user's consent first (the intent is
    /// parked for the DOM layer's confirm).
    fn make_room_for_home_run(
        &mut self,
        intent: HomeReplaceIntent,
        discard_confirmed: bool,
    ) -> bool {
        if op_editor_core::blank_starter::active_page_is_blank_starter(&self.editor_state) {
            return true;
        }
        if !discard_confirmed && self.editor_state.is_dirty() {
            self.home_replace_confirm = Some(intent);
            return false;
        }
        self.start_fresh_document_for_home();
        // The swap dropped every document-scoped pin; a Make-one-like-this
        // send gets its recipe's style guide back.
        self.editor_state.editor_ui.restore_make_same_pin();
        true
    }

    /// Take the send waiting on a discard confirm (DOM drain).
    pub fn take_home_replace_confirm(&mut self) -> Option<HomeReplaceIntent> {
        self.home_replace_confirm.take()
    }

    /// The user agreed to discard the unsaved document: run the parked send.
    pub fn confirm_home_replace(&mut self, intent: HomeReplaceIntent) -> bool {
        if !self.home_visible() {
            return false;
        }
        match intent {
            HomeReplaceIntent::Brief => self.queue_home_send_with(true),
            HomeReplaceIntent::TemplateDraft(template) => {
                self.open_home_template_draft(template, true)
            }
            HomeReplaceIntent::ImportSite => self.home_site_import_press(true),
        }
    }

    /// Replace the document with a blank starter for a new Home deliverable,
    /// keeping chrome, app preferences and the chat model selection. The
    /// transcript restarts so the new run never reads the last design's
    /// conversation as its own history; staged attachments belong to the
    /// brief being sent, so they cross the swap.
    pub(in crate::widget_host) fn start_fresh_document_for_home(&mut self) {
        let starter = op_editor_core::EditorState::starter();
        self.install_home_document(starter.doc, true);
    }

    /// Install `doc` as a new Home deliverable (the swap behind
    /// [`Self::start_fresh_document_for_home`] and a website import).
    /// `saved` marks it clean — a blank starter has nothing to lose, while an
    /// imported site is unsaved work.
    pub(in crate::widget_host) fn install_home_document(
        &mut self,
        doc: op_editor_core::PenDocument,
        saved: bool,
    ) {
        // A live preview was built from the document being replaced.
        self.finish_exit_teardown();
        self.editor_state.replace_document(doc);
        self.editor_state
            .editor_ui
            .workspace
            .reset_for_new_document();
        let ui = &mut self.editor_state.editor_ui;
        ui.file_name_display = None;
        ui.pending_file_action = None;
        ui.scenario = None;
        ui.pinned_style_guide = None;
        // A new deliverable is not the replaced one's import; an import
        // stamps its own origin right after this swap.
        ui.home.imported_from = None;
        if saved {
            self.editor_state.mark_saved_revision();
        } else {
            self.editor_state.mark_document_changed();
        }
        self.document_epoch = self.document_epoch.wrapping_add(1).max(1);
        self.force_rotate_layer_panel_owner();
        self.layout_transition = None;
        self.scene_cache.invalidate();
        let attachments = std::mem::take(&mut self.editor_state.chat.pending_attachments);
        self.editor_state.chat.new_chat();
        self.editor_state.chat.pending_attachments = attachments;
        // The fresh page is not the file the daemon may be bound to: Save
        // must download it rather than overwrite that file.
        self.daemon_file_unbind_pending = true;
        self.mark_dirty();
    }

    /// Take the pending daemon-file unbind a Home swap raised (DOM drain).
    pub fn take_daemon_file_unbind(&mut self) -> bool {
        std::mem::take(&mut self.daemon_file_unbind_pending)
    }

    /// Start from the active task's example. The example is written into the
    /// box first, so what runs is visibly what the user was shown.
    fn start_home_from_example(&mut self, example: &str) -> bool {
        self.editor_state.editor_ui.home.set_draft(example);
        if let Some(template) = self.editor_state.editor_ui.home.example_draft_template() {
            if self.open_home_template_draft(template, false) {
                return true;
            }
            if self.home_replace_confirm.is_some() {
                return false;
            }
            // A template not fetched yet (or unloadable) must not strand the
            // press: fall through to an ordinary generation of the example.
        }
        if !self.editor_state.has_usable_chat_agent() {
            self.editor_state.editor_ui.home.connect_card_open = true;
            return true;
        }
        self.queue_home_send()
    }

    /// Ask the daemon for the active task's example template ahead of the
    /// press, so the one-click draft is instant. Idempotent and
    /// single-flighted by the asset registry; called from the Home paint.
    pub(in crate::widget_host) fn prefetch_home_example_template(&self) {
        let Some(template) = self.editor_state.editor_ui.home.example_draft_template() else {
            return;
        };
        if scene_template_document(template).is_none() {
            if let Some(route) = scene_template_document_route(template) {
                op_editor_core::web_assets::request(route);
            }
        }
    }

    /// Load `template_id` as the instant draft into a fresh workspace, then
    /// queue its refine turn when a model can answer.
    fn open_home_template_draft(
        &mut self,
        template_id: &'static str,
        discard_confirmed: bool,
    ) -> bool {
        let Some(boards) = scene_template_document(template_id)
            .and_then(|source| template_boards(source, template_id))
        else {
            return false;
        };
        if !self.make_room_for_home_run(
            HomeReplaceIntent::TemplateDraft(template_id),
            discard_confirmed,
        ) {
            return false;
        }
        if !self.editor_state.adopt_template_boards(boards) {
            return false;
        }
        self.editor_state.editor_ui.scenario =
            scene_template_by_id(template_id).map(|template| template.scene);
        self.editor_state.clear_selection();
        self.force_rotate_layer_panel_owner();
        self.scene_cache.invalidate();

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
        let (viewport_w, viewport_h) = (self.last_viewport_w, self.last_viewport_h);
        if viewport_w > 0.0 && viewport_h > 0.0 {
            self.apply_workspace_fit(viewport_w, viewport_h);
        }
        self.mark_dirty();
        true
    }

    /// Queue the in-place refine of the workspace's template draft: select
    /// its boards and send the refine brief pinned to [`LaunchRoute::Refine`].
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
        let sent = self.begin_chat_send();
        // The provider gets the engineered refine prompt; the bubble shows
        // the brief the user asked for, in their own words.
        if sent {
            self.editor_state.chat.show_last_user_message_as(&brief);
        }
        self.editor_state.chat.focused = false;
        self.mark_dirty();
        sent
    }

    /// No model: the draft IS the result for now. The conversation still
    /// records what was asked and how to take it further.
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

    /// The draft banner's action: refine once a model can answer, else the
    /// connect path (the Agents settings tab).
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

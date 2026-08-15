//! Shared collaboration presentation models.
//!
//! Hosts feed the same models into native/web paint and hit-test surfaces.
//! This module deliberately contains no socket or file-dialog work. It also
//! reads authenticated session/profile data only through
//! `CollabUiState::authenticated_session`, preserving the pre-auth privacy
//! boundary in one reusable flow.

use op_editor_core::{
    CollabAdmissionRequestKey, CollabAvailability, CollabConnectionPathUi, CollabConnectionPhase,
    CollabGateReason, CollabInviteCode, CollabParticipantUi, CollabPendingEditUi,
    CollabShareEndpoint, CollabUiAction, CollabUiRole, DiscoveredCollabEndpoint, EditorUiState,
    MAX_COLLAB_INVITE_CODE_CHARS,
};

const TOP_BAR_AVATAR_LIMIT: usize = 3;
const MAX_JOIN_TARGET_CHARS: usize = MAX_COLLAB_INVITE_CODE_CHARS;

#[path = "collab_ui_action.rs"]
mod action;
#[path = "collab_ui_debug.rs"]
mod debug;
#[path = "collab_ui_owner_confirm.rs"]
mod owner_confirm;
use action::action_model;
pub use owner_confirm::{CollabOwnerConfirmModel, CollabOwnerIdentityRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabTopBarTone {
    Neutral,
    Progress,
    Connected,
    Warning,
    ReadOnly,
    Ended,
}

#[derive(Clone, PartialEq, Eq)]
pub struct CollabAvatarModel {
    pub participant_key: String,
    pub display_name: String,
    pub initials: String,
    pub color_rgba: u32,
    pub role: CollabUiRole,
    pub is_self: bool,
}

impl From<&CollabParticipantUi> for CollabAvatarModel {
    fn from(participant: &CollabParticipantUi) -> Self {
        Self {
            participant_key: participant.participant_key.clone(),
            display_name: participant.display_name.clone(),
            initials: participant.initials.clone(),
            color_rgba: participant.color_rgba,
            role: participant.role,
            is_self: participant.is_self,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollabTopBarModel {
    pub visible: bool,
    pub enabled: bool,
    pub label: String,
    pub tone: CollabTopBarTone,
    pub avatars: Vec<CollabAvatarModel>,
    pub participant_overflow: usize,
}

impl Default for CollabTopBarModel {
    fn default() -> Self {
        Self {
            visible: false,
            enabled: false,
            label: String::new(),
            tone: CollabTopBarTone::Neutral,
            avatars: Vec::new(),
            participant_overflow: 0,
        }
    }
}

impl CollabTopBarModel {
    pub fn for_editor_ui(ui: &EditorUiState) -> Self {
        let collab = &ui.collab;
        let visible = collab.availability != CollabAvailability::Unavailable
            || collab.phase != CollabConnectionPhase::Idle;
        let enabled = collab.availability != CollabAvailability::Unavailable;
        let (label_key, tone) = match collab.phase {
            CollabConnectionPhase::Idle | CollabConnectionPhase::Discovering => {
                ("collab.topbar.collaborate", CollabTopBarTone::Neutral)
            }
            CollabConnectionPhase::Starting => {
                ("collab.topbar.starting", CollabTopBarTone::Progress)
            }
            CollabConnectionPhase::Joining => ("collab.topbar.joining", CollabTopBarTone::Progress),
            CollabConnectionPhase::Authenticating => {
                ("collab.topbar.authenticating", CollabTopBarTone::Progress)
            }
            CollabConnectionPhase::Active => {
                if ui
                    .collab
                    .authenticated_session()
                    .is_some_and(|session| session.role == CollabUiRole::Viewer)
                {
                    ("collab.topbar.readOnly", CollabTopBarTone::ReadOnly)
                } else {
                    ("collab.topbar.connected", CollabTopBarTone::Connected)
                }
            }
            CollabConnectionPhase::Reconnecting => {
                ("collab.topbar.reconnecting", CollabTopBarTone::Warning)
            }
            CollabConnectionPhase::ReadOnly => {
                ("collab.topbar.readOnly", CollabTopBarTone::ReadOnly)
            }
            CollabConnectionPhase::Ended => ("collab.topbar.ended", CollabTopBarTone::Ended),
        };
        let participants = collab.participants();
        let avatars = participants
            .iter()
            .take(TOP_BAR_AVATAR_LIMIT)
            .map(CollabAvatarModel::from)
            .collect();
        Self {
            visible,
            enabled,
            label: op_i18n::translate(ui.effective_locale(), label_key).to_string(),
            tone,
            avatars,
            participant_overflow: participants.len().saturating_sub(TOP_BAR_AVATAR_LIMIT),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum CollabPanelScreen {
    Unavailable,
    SignInRequired,
    Home,
    Create,
    Join {
        address: String,
        discovered: Vec<DiscoveredCollabEndpoint>,
    },
    Progress {
        message: String,
    },
    /// A guest has verified the owner's ticket and must now confirm the
    /// identity behind it. No session data exists on this screen by
    /// construction: it is reached only before admission completes.
    ConfirmOwner(Box<CollabOwnerConfirmModel>),
    Session {
        session_name: String,
        role_label: String,
        invite: Option<CollabInviteCode>,
        connection: Option<CollabConnectionPathUi>,
        share_endpoint: Option<CollabShareEndpoint>,
        participants: Vec<CollabAvatarModel>,
        pending: bool,
        admission_request: Option<CollabAdmissionRequestModel>,
    },
}

/// The owner sees one generic request at a time. The routing key is opaque
/// and redacted by its core type; no pre-approval profile fields enter this
/// paint model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollabAdmissionRequestModel {
    pub request_key: CollabAdmissionRequestKey,
    pub label: String,
    pub actions: Vec<CollabPanelActionModel>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct CollabPanelActionModel {
    pub action: CollabUiAction,
    pub label: String,
    pub primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollabPanelModel {
    pub title: String,
    pub screen: CollabPanelScreen,
    pub notice: Option<String>,
    pub actions: Vec<CollabPanelActionModel>,
}

impl CollabPanelModel {
    pub fn for_editor_ui(ui: &EditorUiState) -> Self {
        let collab = &ui.collab;
        let title = op_i18n::translate(ui.effective_locale(), "collab.session.title").to_string();
        let notice = collab.notice.map(|notice| notice_text(ui, notice.kind));

        let (screen, actions) = match collab.availability {
            CollabAvailability::Unavailable if collab.phase == CollabConnectionPhase::Idle => {
                (CollabPanelScreen::Unavailable, Vec::new())
            }
            CollabAvailability::SignInRequired if collab.phase == CollabConnectionPhase::Idle => {
                (CollabPanelScreen::SignInRequired, Vec::new())
            }
            _ => panel_session_or_pre_auth(ui),
        };
        Self {
            title,
            screen,
            notice,
            actions,
        }
    }
}

fn panel_session_or_pre_auth(
    ui: &EditorUiState,
) -> (CollabPanelScreen, Vec<CollabPanelActionModel>) {
    let collab = &ui.collab;
    match collab.phase {
        CollabConnectionPhase::Starting
        | CollabConnectionPhase::Joining
        | CollabConnectionPhase::Authenticating => {
            // The guest's own admission gate. It preempts the progress screen
            // because the join must not look like it is proceeding while a
            // human decision is outstanding.
            if let Some(confirm) = owner_confirm::owner_confirm_model(ui) {
                let actions = confirm.actions.clone();
                return (CollabPanelScreen::ConfirmOwner(Box::new(confirm)), actions);
            }
            let key = match collab.phase {
                CollabConnectionPhase::Starting => "collab.topbar.starting",
                CollabConnectionPhase::Joining => "collab.topbar.joining",
                CollabConnectionPhase::Authenticating => "collab.join.authenticating",
                _ => unreachable!("outer match restricts transition phases"),
            };
            (
                CollabPanelScreen::Progress {
                    message: op_i18n::translate(ui.effective_locale(), key).to_string(),
                },
                vec![action_model(ui, CollabUiAction::Cancel, false)],
            )
        }
        CollabConnectionPhase::Active
        | CollabConnectionPhase::Reconnecting
        | CollabConnectionPhase::ReadOnly
        | CollabConnectionPhase::Ended => {
            let Some(session) = collab.authenticated_session() else {
                // Fail closed if a host publishes an authenticated phase before
                // its verified display projection.
                return (
                    CollabPanelScreen::Progress {
                        message: op_i18n::translate(
                            ui.effective_locale(),
                            "collab.join.authenticating",
                        )
                        .to_string(),
                    },
                    vec![action_model(ui, CollabUiAction::Leave, false)],
                );
            };
            let participants = collab
                .participants()
                .iter()
                .map(CollabAvatarModel::from)
                .collect();
            let public_session = collab.public_session();
            let admission_request = collab.pending_admissions().first().map(|request| {
                let request_key = request.request_key().clone();
                let mut actions = Vec::new();
                if request
                    .resume_role()
                    .is_none_or(|role| role == CollabUiRole::Editor)
                {
                    actions.push(action_model(
                        ui,
                        CollabUiAction::ApproveAdmissionEditor {
                            request_key: request_key.clone(),
                        },
                        true,
                    ));
                }
                if request
                    .resume_role()
                    .is_none_or(|role| role == CollabUiRole::Viewer)
                {
                    actions.push(action_model(
                        ui,
                        CollabUiAction::ApproveAdmissionViewer {
                            request_key: request_key.clone(),
                        },
                        request.resume_role() == Some(CollabUiRole::Viewer),
                    ));
                }
                actions.push(action_model(
                    ui,
                    CollabUiAction::RejectAdmission {
                        request_key: request_key.clone(),
                    },
                    false,
                ));
                CollabAdmissionRequestModel {
                    request_key,
                    label: op_i18n::translate(ui.effective_locale(), "collab.admission.request")
                        .to_string(),
                    actions,
                }
            });
            let mut actions = Vec::new();
            if collab.phase == CollabConnectionPhase::Ended {
                if collab.pending_edit != CollabPendingEditUi::None {
                    actions.push(action_model(ui, CollabUiAction::DiscardPending, false));
                }
                actions.push(action_model(ui, CollabUiAction::SaveAsFork, true));
            } else {
                if collab.phase == CollabConnectionPhase::Reconnecting {
                    actions.push(action_model(ui, CollabUiAction::Retry, true));
                }
                if collab.phase == CollabConnectionPhase::Active
                    && collab.discarded_edit.is_some()
                    && collab.pending_edit == CollabPendingEditUi::None
                {
                    actions.push(action_model(ui, CollabUiAction::ReapplyDiscarded, true));
                }
                actions.push(action_model(ui, CollabUiAction::Leave, false));
            }
            (
                CollabPanelScreen::Session {
                    session_name: session.session_name.clone(),
                    role_label: role_label(ui, session.role).to_string(),
                    invite: if session.role == CollabUiRole::Owner {
                        public_session.and_then(|public| public.invite()).cloned()
                    } else {
                        None
                    },
                    connection: public_session.and_then(|public| public.connection()),
                    share_endpoint: if session.role == CollabUiRole::Owner {
                        session.share_endpoint.clone()
                    } else {
                        None
                    },
                    participants,
                    pending: collab.pending_edit != CollabPendingEditUi::None,
                    admission_request,
                },
                actions,
            )
        }
        CollabConnectionPhase::Idle | CollabConnectionPhase::Discovering => {
            if collab.panel.view == op_editor_core::CollabPanelView::Join
                || collab.phase == CollabConnectionPhase::Discovering
            {
                let mut actions = Vec::new();
                let endpoint = collab.panel.join_input.text().trim();
                if !endpoint.is_empty() {
                    actions.push(CollabPanelActionModel {
                        action: CollabUiAction::JoinAddress {
                            endpoint: endpoint.to_string(),
                        },
                        label: op_i18n::translate(ui.effective_locale(), "collab.action.connect")
                            .to_string(),
                        primary: true,
                    });
                }
                if collab.phase == CollabConnectionPhase::Idle
                    && collab.transport_capabilities.nearby_discovery
                {
                    actions.push(action_model(
                        ui,
                        CollabUiAction::BeginDiscovery,
                        endpoint.is_empty(),
                    ));
                }
                actions.push(action_model(ui, CollabUiAction::Cancel, false));
                (
                    CollabPanelScreen::Join {
                        address: collab.panel.join_input.text().to_owned(),
                        discovered: collab.panel.discovered.as_ref().clone(),
                    },
                    actions,
                )
            } else if collab.panel.view == op_editor_core::CollabPanelView::Create {
                let mut actions = vec![action_model(ui, CollabUiAction::Start, true)];
                if collab.transport_capabilities.lan_hosting {
                    actions.push(action_model(ui, CollabUiAction::StartLan, false));
                }
                actions.push(action_model(ui, CollabUiAction::Cancel, false));
                (CollabPanelScreen::Create, actions)
            } else {
                (
                    CollabPanelScreen::Home,
                    vec![
                        action_model(ui, CollabUiAction::OpenCreate, true),
                        action_model(ui, CollabUiAction::OpenJoin, false),
                    ],
                )
            }
        }
    }
}

pub fn role_label(ui: &EditorUiState, role: CollabUiRole) -> &'static str {
    let key = match role {
        CollabUiRole::Owner => "collab.session.role.owner",
        CollabUiRole::Editor => "collab.session.role.editor",
        CollabUiRole::Viewer => "collab.session.role.viewer",
    };
    op_i18n::translate(ui.effective_locale(), key)
}

pub fn connection_path_label(ui: &EditorUiState, connection: CollabConnectionPathUi) -> String {
    let path = op_i18n::translate(ui.effective_locale(), connection.i18n_key());
    match connection.home_region() {
        Some(region) => format!(
            "{path} · {}",
            op_i18n::translate(ui.effective_locale(), region.i18n_key())
        ),
        None => path.to_string(),
    }
}

pub fn gate_reason_text(ui: &EditorUiState, reason: CollabGateReason) -> &'static str {
    op_i18n::translate(ui.effective_locale(), reason.i18n_key())
}

pub fn notice_text(ui: &EditorUiState, kind: op_editor_core::CollabNoticeKind) -> String {
    let message = op_i18n::translate(ui.effective_locale(), kind.i18n_key());
    match kind {
        op_editor_core::CollabNoticeKind::UnsupportedEdit(feature) => {
            format!(
                "{message} {}",
                op_i18n::translate(ui.effective_locale(), feature.i18n_key())
            )
        }
        // Only the cancellation that produced the stash names it; a plain
        // `Reject(Conflict)` (for example the pending-edit gate) must not
        // borrow an older discarded edit's detail.
        op_editor_core::CollabNoticeKind::EditConflictDiscarded => {
            let Some(discarded) = ui.collab.discarded_edit.as_ref() else {
                return message.to_string();
            };
            let detail = op_i18n::translate(ui.effective_locale(), "collab.reject.conflictDetail")
                .replace("{{fields}}", &discarded.fields.join(", "))
                .replace("{{node}}", &discarded.node_label);
            format!("{message} {detail}")
        }
        _ => message.to_string(),
    }
}

/// Queue one host-owned collaboration action. A pending action is not
/// overwritten, avoiding duplicate start/join/leave side effects when a
/// pointer is pressed twice before the runtime drains the first request.
pub fn request_action(ui: &mut EditorUiState, action: CollabUiAction) -> bool {
    if ui.collab.pending_action.is_some() {
        return false;
    }
    ui.collab.pending_action = Some(action);
    true
}

/// Shared panel dispatch used by both native and web hosts. It performs only
/// local chrome transitions and queues runtime-owned side effects.
pub fn apply_panel_hit(
    ui: &mut EditorUiState,
    hit: crate::widgets::collab_panel::CollabPanelHit,
) -> bool {
    use crate::widgets::collab_panel::CollabPanelHit;
    match hit {
        CollabPanelHit::Close => {
            ui.collab.panel.open = false;
            ui.collab.panel.join_address_focused = false;
            ui.collab.panel.hover = None;
            true
        }
        CollabPanelHit::FocusJoinAddress => {
            // A plain click focuses with a collapsed caret at the end; it
            // never keeps a stale whole-field selection alive.
            ui.collab.panel.join_address_focused = true;
            let input = &mut ui.collab.panel.join_input;
            let end = input.text().len();
            input.set_caret(end, 0);
            true
        }
        CollabPanelHit::ClearJoinAddress => {
            ui.collab.panel.join_input.set_text("");
            ui.collab.panel.join_address_focused = true;
            ui.collab.panel.hover = None;
            true
        }
        CollabPanelHit::OpenSignIn => {
            if !ui.account_ui_available {
                return false;
            }
            ui.login_modal_open = true;
            ui.login_modal_hover = None;
            ui.collab.panel.open = false;
            ui.collab.panel.join_address_focused = false;
            ui.collab.panel.hover = None;
            true
        }
        // Platform hosts perform this inside the originating pointer event so
        // native/browser clipboard gesture requirements remain satisfied.
        CollabPanelHit::CopyShareEndpoint(_) => false,
        CollabPanelHit::CopyInvite(_) => false,
        CollabPanelHit::Inside => {
            ui.collab.panel.join_address_focused = false;
            true
        }
        CollabPanelHit::Action(CollabUiAction::OpenCreate) => {
            ui.collab.panel.view = op_editor_core::CollabPanelView::Create;
            ui.collab.panel.join_address_focused = false;
            ui.collab.panel.hover = None;
            true
        }
        CollabPanelHit::Action(CollabUiAction::OpenJoin) => {
            ui.collab.panel.view = op_editor_core::CollabPanelView::Join;
            ui.collab.panel.join_address_focused = true;
            ui.collab.panel.hover = None;
            true
        }
        CollabPanelHit::Action(CollabUiAction::BeginDiscovery) => {
            ui.collab.panel.view = op_editor_core::CollabPanelView::Join;
            request_action(ui, CollabUiAction::BeginDiscovery)
        }
        CollabPanelHit::Action(CollabUiAction::Cancel)
            if ui.collab.phase == CollabConnectionPhase::Idle =>
        {
            ui.collab.panel.view = op_editor_core::CollabPanelView::Home;
            ui.collab.panel.join_address_focused = false;
            ui.collab.panel.hover = None;
            true
        }
        CollabPanelHit::Action(action) => {
            ui.collab.panel.join_address_focused = false;
            request_action(ui, action)
        }
    }
}

/// Typed-character routing for the invite-or-`host:port` field. `None` means
/// the collaboration input is not focused; `Some` means it owns the key.
pub fn join_address_text(ui: &mut EditorUiState, character: char, now_ms: u64) -> Option<bool> {
    if !ui.collab.panel.join_address_focused {
        return None;
    }
    if character.is_control() || !join_address_char_allowed(character) {
        return Some(false);
    }
    let input = &mut ui.collab.panel.join_input;
    // The cap applies to the post-edit length: typing over a selection must
    // still be able to replace a full field.
    let selected = input
        .highlight_range()
        .map(|(start, end)| end - start)
        .unwrap_or(0);
    if input.text().chars().count() - selected >= MAX_JOIN_TARGET_CHARS {
        return Some(false);
    }
    let mut buffer = [0_u8; 4];
    input.insert_str(character.encode_utf8(&mut buffer), now_ms);
    ui.collab.panel.hover = None;
    Some(true)
}

/// Presentation filter for the invite-or-`host:port` field. The runtime
/// performs authoritative validation; this merely keeps whitespace and
/// shell-like punctuation out of the one-line endpoint field.
fn join_address_char_allowed(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | ':' | '-' | '[' | ']' | '_')
}

pub fn join_address_backspace(ui: &mut EditorUiState, now_ms: u64) -> Option<bool> {
    if !ui.collab.panel.join_address_focused {
        return None;
    }
    let input = &mut ui.collab.panel.join_input;
    let before = input.text().to_owned();
    input.backspace(now_ms);
    let changed = input.text() != before;
    if changed {
        ui.collab.panel.hover = None;
    }
    Some(changed)
}

/// Forward deletion (the Delete key) on the focused join field.
pub fn join_address_delete_forward(ui: &mut EditorUiState, now_ms: u64) -> Option<bool> {
    if !ui.collab.panel.join_address_focused {
        return None;
    }
    let input = &mut ui.collab.panel.join_input;
    let before = input.text().to_owned();
    input.delete_forward(now_ms);
    let changed = input.text() != before;
    if changed {
        ui.collab.panel.hover = None;
    }
    Some(changed)
}

/// Cmd/Ctrl+A on the focused join field — whole-field selection. `None`
/// means the field is not focused and the chord belongs to someone else.
pub fn join_address_select_all(ui: &mut EditorUiState, now_ms: u64) -> Option<bool> {
    if !ui.collab.panel.join_address_focused {
        return None;
    }
    let input = &mut ui.collab.panel.join_input;
    let selectable = !input.text().is_empty();
    if selectable {
        input.select_all();
        input.touch(now_ms);
    }
    Some(selectable)
}

/// Clipboard paste into the focused join field. Replaces the whole field —
/// an invite code is pasted as a unit, and append semantics silently
/// produced corrupt old+new concatenations. `None` means not focused.
pub fn join_address_paste(ui: &mut EditorUiState, text: &str, now_ms: u64) -> Option<bool> {
    if !ui.collab.panel.join_address_focused {
        return None;
    }
    let sanitized: String = text
        .chars()
        .filter(|character| !character.is_control() && join_address_char_allowed(*character))
        .take(MAX_JOIN_TARGET_CHARS)
        .collect();
    if sanitized.is_empty() {
        return Some(false);
    }
    let input = &mut ui.collab.panel.join_input;
    input.set_text(sanitized);
    input.touch(now_ms);
    ui.collab.panel.hover = None;
    Some(true)
}

pub fn join_address_submit(ui: &mut EditorUiState) -> Option<bool> {
    if !ui.collab.panel.join_address_focused {
        return None;
    }
    let endpoint = ui.collab.panel.join_input.text().trim();
    if endpoint.is_empty() {
        return Some(false);
    }
    let action = CollabUiAction::JoinAddress {
        endpoint: endpoint.to_string(),
    };
    ui.collab.panel.join_address_focused = false;
    ui.collab.panel.hover = None;
    Some(request_action(ui, action))
}

#[cfg(test)]
#[path = "collab_ui_model_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "collab_ui_tests.rs"]
mod public_flow_tests;

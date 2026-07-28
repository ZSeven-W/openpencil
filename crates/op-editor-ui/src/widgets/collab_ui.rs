//! Shared collaboration presentation models.
//!
//! Hosts feed the same models into native/web paint and hit-test surfaces.
//! This module deliberately contains no socket or file-dialog work. It also
//! reads authenticated session/profile data only through
//! `CollabUiState::authenticated_session`, preserving the pre-auth privacy
//! boundary in one reusable flow.

use op_editor_core::{
    CollabAdmissionRequestKey, CollabAvailability, CollabConnectionPhase, CollabGateReason,
    CollabParticipantUi, CollabPendingEditUi, CollabShareEndpoint, CollabUiAction, CollabUiRole,
    DiscoveredCollabEndpoint, EditorUiState,
};

const TOP_BAR_AVATAR_LIMIT: usize = 3;
const MAX_JOIN_ADDRESS_CHARS: usize = 255;

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
            label: op_i18n::translate(ui.locale, label_key).to_string(),
            tone,
            avatars,
            participant_overflow: participants.len().saturating_sub(TOP_BAR_AVATAR_LIMIT),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollabPanelScreen {
    Unavailable,
    SignInRequired,
    Home,
    Join {
        address: String,
        discovered: Vec<DiscoveredCollabEndpoint>,
    },
    Progress {
        message: String,
    },
    Session {
        session_name: String,
        role_label: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
        let title = op_i18n::translate(ui.locale, "collab.session.title").to_string();
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
            let key = match collab.phase {
                CollabConnectionPhase::Starting => "collab.topbar.starting",
                CollabConnectionPhase::Joining => "collab.topbar.joining",
                CollabConnectionPhase::Authenticating => "collab.join.authenticating",
                _ => unreachable!("outer match restricts transition phases"),
            };
            (
                CollabPanelScreen::Progress {
                    message: op_i18n::translate(ui.locale, key).to_string(),
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
                        message: op_i18n::translate(ui.locale, "collab.join.authenticating")
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
                    label: op_i18n::translate(ui.locale, "collab.admission.request").to_string(),
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
                actions.push(action_model(ui, CollabUiAction::Leave, false));
            }
            (
                CollabPanelScreen::Session {
                    session_name: session.session_name.clone(),
                    role_label: role_label(ui, session.role).to_string(),
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
                let endpoint = collab.panel.join_address.trim();
                if !endpoint.is_empty() {
                    actions.push(CollabPanelActionModel {
                        action: CollabUiAction::JoinAddress {
                            endpoint: endpoint.to_string(),
                        },
                        label: op_i18n::translate(ui.locale, "collab.action.connect").to_string(),
                        primary: true,
                    });
                }
                actions.push(action_model(ui, CollabUiAction::Cancel, false));
                (
                    CollabPanelScreen::Join {
                        address: collab.panel.join_address.clone(),
                        discovered: collab.panel.discovered.as_ref().clone(),
                    },
                    actions,
                )
            } else {
                (
                    CollabPanelScreen::Home,
                    vec![
                        action_model(ui, CollabUiAction::Start, true),
                        action_model(ui, CollabUiAction::BeginDiscovery, false),
                    ],
                )
            }
        }
    }
}

fn action_model(
    ui: &EditorUiState,
    action: CollabUiAction,
    primary: bool,
) -> CollabPanelActionModel {
    let key = match action {
        CollabUiAction::Start => "collab.action.start",
        CollabUiAction::BeginDiscovery => "collab.action.join",
        CollabUiAction::JoinDiscovered { .. } | CollabUiAction::JoinAddress { .. } => {
            "collab.action.connect"
        }
        CollabUiAction::Cancel => "collab.action.cancel",
        CollabUiAction::Retry => "collab.action.retry",
        CollabUiAction::Leave => "collab.action.leave",
        CollabUiAction::DiscardPending => "collab.action.discardPending",
        CollabUiAction::SaveAsFork => "collab.action.saveAsFork",
        CollabUiAction::ApproveAdmissionEditor { .. } => "collab.action.approveEditor",
        CollabUiAction::ApproveAdmissionViewer { .. } => "collab.action.approveViewer",
        CollabUiAction::RejectAdmission { .. } => "collab.action.rejectAdmission",
    };
    CollabPanelActionModel {
        action,
        label: op_i18n::translate(ui.locale, key).to_string(),
        primary,
    }
}

pub fn role_label(ui: &EditorUiState, role: CollabUiRole) -> &'static str {
    let key = match role {
        CollabUiRole::Owner => "collab.session.role.owner",
        CollabUiRole::Editor => "collab.session.role.editor",
        CollabUiRole::Viewer => "collab.session.role.viewer",
    };
    op_i18n::translate(ui.locale, key)
}

pub fn gate_reason_text(ui: &EditorUiState, reason: CollabGateReason) -> &'static str {
    op_i18n::translate(ui.locale, reason.i18n_key())
}

pub fn notice_text(ui: &EditorUiState, kind: op_editor_core::CollabNoticeKind) -> String {
    let message = op_i18n::translate(ui.locale, kind.i18n_key());
    if let op_editor_core::CollabNoticeKind::UnsupportedEdit(feature) = kind {
        format!(
            "{message} {}",
            op_i18n::translate(ui.locale, feature.i18n_key())
        )
    } else {
        message.to_string()
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
            true
        }
        CollabPanelHit::FocusJoinAddress => {
            ui.collab.panel.join_address_focused = true;
            true
        }
        CollabPanelHit::OpenSignIn => {
            if !ui.account_ui_available {
                return false;
            }
            ui.login_modal_open = true;
            ui.login_modal_hover = None;
            ui.collab.panel.join_address_focused = false;
            true
        }
        // Platform hosts perform this inside the originating pointer event so
        // native/browser clipboard gesture requirements remain satisfied.
        CollabPanelHit::CopyShareEndpoint(_) => false,
        CollabPanelHit::Inside => {
            ui.collab.panel.join_address_focused = false;
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
            true
        }
        CollabPanelHit::Action(action) => {
            ui.collab.panel.join_address_focused = false;
            request_action(ui, action)
        }
    }
}

/// Typed-character routing for the manual `host:port` field. `None` means
/// the collaboration input is not focused; `Some` means it owns the key.
pub fn join_address_text(ui: &mut EditorUiState, character: char) -> Option<bool> {
    if !ui.collab.panel.join_address_focused {
        return None;
    }
    if character.is_control()
        || ui.collab.panel.join_address.chars().count() >= MAX_JOIN_ADDRESS_CHARS
    {
        return Some(false);
    }
    // The runtime performs authoritative SocketAddr/hostname validation.
    // This presentation filter merely keeps whitespace and shell-like
    // punctuation out of the one-line endpoint field.
    if !(character.is_ascii_alphanumeric()
        || matches!(character, '.' | ':' | '-' | '[' | ']' | '_'))
    {
        return Some(false);
    }
    ui.collab.panel.join_address.push(character);
    Some(true)
}

pub fn join_address_backspace(ui: &mut EditorUiState) -> Option<bool> {
    if !ui.collab.panel.join_address_focused {
        return None;
    }
    Some(ui.collab.panel.join_address.pop().is_some())
}

pub fn join_address_submit(ui: &mut EditorUiState) -> Option<bool> {
    if !ui.collab.panel.join_address_focused {
        return None;
    }
    let endpoint = ui.collab.panel.join_address.trim();
    if endpoint.is_empty() {
        return Some(false);
    }
    let action = CollabUiAction::JoinAddress {
        endpoint: endpoint.to_string(),
    };
    ui.collab.panel.join_address_focused = false;
    Some(request_action(ui, action))
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::{
        AuthenticatedCollabSession, CollabAdmissionRequestKey, CollabPanelView,
        CollabParticipantUi, CollabShareEndpoint, Locale,
    };

    fn participant(key: &str, name: &str) -> CollabParticipantUi {
        CollabParticipantUi::new(key, name, 0x3366ffff, CollabUiRole::Editor, false)
    }

    #[test]
    fn pre_auth_panel_never_contains_session_or_participant_profiles() {
        let mut ui = EditorUiState::default();
        ui.collab.availability = CollabAvailability::Ready;
        ui.collab.phase = CollabConnectionPhase::Authenticating;
        let model = CollabPanelModel::for_editor_ui(&ui);
        assert!(matches!(model.screen, CollabPanelScreen::Progress { .. }));
        assert!(!format!("{model:?}").contains("participant"));
    }

    #[test]
    fn authenticated_topbar_caps_avatar_stack_and_reports_overflow() {
        let mut ui = EditorUiState::default();
        ui.collab.availability = CollabAvailability::Ready;
        ui.collab.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Landing page".to_string(),
                role: CollabUiRole::Editor,
                share_endpoint: None,
            },
            vec![
                participant("p1", "Ada"),
                participant("p2", "Grace"),
                participant("p3", "Linus"),
                participant("p4", "Margaret"),
            ],
        );
        let model = CollabTopBarModel::for_editor_ui(&ui);
        assert_eq!(model.avatars.len(), 3);
        assert_eq!(model.participant_overflow, 1);
        assert_eq!(model.tone, CollabTopBarTone::Connected);
    }

    #[test]
    fn participant_models_project_to_both_surfaces_without_profile_urls() {
        let mut ui = EditorUiState::default();
        ui.collab.availability = CollabAvailability::Ready;
        ui.collab.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Design".to_string(),
                role: CollabUiRole::Editor,
                share_endpoint: None,
            },
            vec![
                CollabParticipantUi::new("owner", "Owner", 0x3366ffff, CollabUiRole::Owner, false),
                CollabParticipantUi::new("guest", "Guest", 0x6633ffff, CollabUiRole::Editor, true),
            ],
        );

        let topbar = CollabTopBarModel::for_editor_ui(&ui);
        assert_eq!(
            topbar
                .avatars
                .iter()
                .map(|avatar| avatar.participant_key.as_str())
                .collect::<Vec<_>>(),
            vec!["owner", "guest"]
        );
        let panel = CollabPanelModel::for_editor_ui(&ui);
        let CollabPanelScreen::Session { participants, .. } = panel.screen else {
            panic!("expected session model");
        };
        assert_eq!(participants, topbar.avatars);
    }

    #[test]
    fn join_model_exposes_only_discovery_endpoint_data() {
        let mut ui = EditorUiState::default();
        ui.collab.availability = CollabAvailability::Ready;
        ui.collab.panel.view = CollabPanelView::Join;
        ui.collab.panel.join_address = "10.0.0.2:43120".to_string();
        ui.collab.panel.discovered = std::sync::Arc::new(vec![DiscoveredCollabEndpoint {
            discovery_id: "opaque-1".to_string(),
            endpoint: "10.0.0.3:43120".to_string(),
            compatible: true,
        }]);
        let model = CollabPanelModel::for_editor_ui(&ui);
        let CollabPanelScreen::Join { discovered, .. } = model.screen else {
            panic!("expected join model");
        };
        assert_eq!(discovered[0].endpoint, "10.0.0.3:43120");
    }

    #[test]
    fn gate_reason_uses_the_active_locale() {
        let ui = EditorUiState {
            locale: Locale::ZhCn,
            ..Default::default()
        };
        assert_eq!(
            gate_reason_text(&ui, CollabGateReason::OwnerOnlySave),
            "只有所有者可以保存共享源文件。"
        );
    }

    #[test]
    fn action_queue_is_single_flight() {
        let mut ui = EditorUiState::default();
        assert!(request_action(&mut ui, CollabUiAction::Start));
        assert!(!request_action(&mut ui, CollabUiAction::Leave));
        assert_eq!(ui.collab.take_pending_action(), Some(CollabUiAction::Start));
    }

    #[test]
    fn owner_admission_model_has_three_decisions_without_identity_data() {
        let mut ui = EditorUiState::default();
        ui.collab.availability = CollabAvailability::Ready;
        ui.collab.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Design".to_string(),
                role: CollabUiRole::Owner,
                share_endpoint: None,
            },
            Vec::new(),
        );
        let request_key = CollabAdmissionRequestKey::new("opaque-request-7").unwrap();
        assert!(ui
            .collab
            .publish_pending_admission(request_key.clone(), None));

        let model = CollabPanelModel::for_editor_ui(&ui);
        let CollabPanelScreen::Session {
            admission_request: Some(request),
            ..
        } = &model.screen
        else {
            panic!("owner must see the oldest pending admission");
        };
        assert_eq!(request.actions.len(), 3);
        assert!(request.actions.iter().any(|action| {
            action.action
                == CollabUiAction::ApproveAdmissionEditor {
                    request_key: request_key.clone(),
                }
        }));
        assert!(request.actions.iter().any(|action| {
            action.action
                == CollabUiAction::ApproveAdmissionViewer {
                    request_key: request_key.clone(),
                }
        }));
        assert!(request.actions.iter().any(|action| {
            action.action
                == CollabUiAction::RejectAdmission {
                    request_key: request_key.clone(),
                }
        }));
        let debug = format!("{model:?}");
        assert!(!debug.contains(request_key.as_str()));
        assert!(!debug.contains("subject"));
        assert!(!debug.contains("device"));
    }

    #[test]
    fn viewer_panel_cannot_project_owner_admission_controls() {
        let mut ui = EditorUiState::default();
        ui.collab.availability = CollabAvailability::Ready;
        ui.collab.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Design".to_string(),
                role: CollabUiRole::Viewer,
                share_endpoint: None,
            },
            Vec::new(),
        );
        let model = CollabPanelModel::for_editor_ui(&ui);
        let CollabPanelScreen::Session {
            admission_request, ..
        } = model.screen
        else {
            panic!("expected session model");
        };
        assert!(admission_request.is_none());
    }

    #[test]
    fn manual_share_endpoint_is_projected_only_for_the_owner() {
        let raw_endpoint = "192.168.1.8:43120";
        let mut owner_ui = EditorUiState::default();
        owner_ui.collab.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Design".to_string(),
                role: CollabUiRole::Owner,
                share_endpoint: CollabShareEndpoint::new(raw_endpoint),
            },
            Vec::new(),
        );
        let owner_model = CollabPanelModel::for_editor_ui(&owner_ui);
        let CollabPanelScreen::Session { share_endpoint, .. } = &owner_model.screen else {
            panic!("expected owner session model");
        };
        assert_eq!(
            share_endpoint.as_ref().map(CollabShareEndpoint::as_str),
            Some(raw_endpoint)
        );
        assert!(!format!("{owner_model:?}").contains(raw_endpoint));

        for role in [CollabUiRole::Editor, CollabUiRole::Viewer] {
            let mut guest_ui = EditorUiState::default();
            guest_ui.collab.set_authenticated_session(
                CollabConnectionPhase::Active,
                AuthenticatedCollabSession {
                    session_name: "Design".to_string(),
                    role,
                    share_endpoint: CollabShareEndpoint::new(raw_endpoint),
                },
                Vec::new(),
            );
            let guest_model = CollabPanelModel::for_editor_ui(&guest_ui);
            let CollabPanelScreen::Session { share_endpoint, .. } = guest_model.screen else {
                panic!("expected guest session model");
            };
            assert!(share_endpoint.is_none());
        }
    }

    #[test]
    fn panel_hit_dispatches_navigation_without_fake_runtime_state() {
        let mut ui = EditorUiState::default();
        ui.collab.availability = CollabAvailability::Ready;
        ui.collab.panel.open = true;
        assert!(apply_panel_hit(
            &mut ui,
            crate::widgets::collab_panel::CollabPanelHit::Action(CollabUiAction::BeginDiscovery)
        ));
        assert_eq!(ui.collab.panel.view, CollabPanelView::Join);
        assert_eq!(
            ui.collab.take_pending_action(),
            Some(CollabUiAction::BeginDiscovery)
        );
        assert_eq!(ui.collab.phase, CollabConnectionPhase::Idle);
    }

    #[test]
    fn manual_address_input_is_bounded_and_queues_one_join() {
        let mut ui = EditorUiState::default();
        ui.collab.panel.join_address_focused = true;
        for character in "192.168.1.8:43120".chars() {
            assert_eq!(join_address_text(&mut ui, character), Some(true));
        }
        assert_eq!(join_address_text(&mut ui, ' '), Some(false));
        assert_eq!(join_address_submit(&mut ui), Some(true));
        assert_eq!(
            ui.collab.take_pending_action(),
            Some(CollabUiAction::JoinAddress {
                endpoint: "192.168.1.8:43120".into()
            })
        );
    }
}

//! Platform-neutral collaboration display state.
//!
//! The transport/session actor owns sockets, tickets, stable subjects, device
//! identifiers, and cryptographic material. This module contains only the
//! bounded, authenticated display projection consumed by shared widgets.
//! Keeping that split explicit prevents paint-state clones and debug output
//! from becoming a credential or persistent-identity side channel.

use std::sync::Arc;

use crate::collab_admission_ui::{CollabAdmissionRequestKey, PendingCollabAdmissionUi};
use crate::{CollabPanelHover, CollabPublicSessionUi};

/// Presence is lossy and is never part of the ordered commit stream.
pub const COLLAB_PRESENCE_FRAME_INTERVAL_MS: u64 = 33;
/// Defensive paint bound. The wire layer has its own byte bound; this smaller
/// UI bound prevents one participant selection from dominating a frame.
pub const MAX_COLLAB_UI_SELECTION_IDS: usize = 256;
/// Display-side bound below the protocol participant limit.
pub const MAX_COLLAB_UI_PARTICIPANTS: usize = 64;
/// Bounded again at the presentation boundary even though admission claims
/// are already validated by the authentication bridge.
pub const MAX_COLLAB_DISPLAY_NAME_CHARS: usize = 80;
/// Manual owner share endpoints are presentation data, not discovery state.
/// Keep the projection bounded before it can enter widget or debug snapshots.
pub const MAX_COLLAB_SHARE_ENDPOINT_CHARS: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollabAvailability {
    /// This host has no collaboration runtime (for example an M1 web build).
    #[default]
    Unavailable,
    /// The runtime exists, but an authenticated account is required.
    SignInRequired,
    /// Starting or joining a session is available.
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollabConnectionPhase {
    /// No session is bound to the document.
    #[default]
    Idle,
    /// The owner runtime is being started.
    Starting,
    /// mDNS discovery is active, but the current document is not yet bound.
    Discovering,
    /// A selected endpoint is being connected.
    Joining,
    /// The encrypted channel exists and admission is in progress.
    Authenticating,
    /// The authenticated session is live.
    Active,
    /// A guest lost its link and is immediately read-only.
    Reconnecting,
    /// The session remains visible but document edits are frozen.
    ReadOnly,
    /// The epoch changed or the owner left. Only fork/discard flows remain.
    Ended,
}

impl CollabConnectionPhase {
    pub fn is_authenticated(self) -> bool {
        matches!(
            self,
            Self::Active | Self::Reconnecting | Self::ReadOnly | Self::Ended
        )
    }

    pub fn freezes_document(self) -> bool {
        matches!(
            self,
            Self::Starting
                | Self::Joining
                | Self::Authenticating
                | Self::Reconnecting
                | Self::ReadOnly
                | Self::Ended
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabUiRole {
    Owner,
    Editor,
    Viewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollabPanelView {
    #[default]
    Home,
    Create,
    Join,
    Session,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredCollabEndpoint {
    /// Opaque discovery-row identity. It is not a session or account id.
    pub discovery_id: String,
    /// User-enterable LAN endpoint such as `192.168.1.8:43120`.
    pub endpoint: String,
    /// Protocol/schema compatibility derived without exposing session data.
    pub compatible: bool,
}

#[derive(Clone, PartialEq, Default)]
pub struct CollabPanelState {
    pub open: bool,
    pub view: CollabPanelView,
    /// The invite-or-`host:port` field, on the unified single-line input
    /// component (caret, blink, selection, click-to-position all shared).
    pub join_input: jian_core::text_input::TextInputState,
    pub join_address_focused: bool,
    pub hover: Option<CollabPanelHover>,
    pub discovered: Arc<Vec<DiscoveredCollabEndpoint>>,
}

/// Authenticated session display data. A runtime must not construct this from
/// an unauthenticated mDNS record or pre-admission peer frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedCollabSession {
    pub session_name: String,
    pub role: CollabUiRole,
    /// Owner-only address that can be shared out of band for manual joining.
    pub share_endpoint: Option<CollabShareEndpoint>,
}

/// Bounded, display-only manual join address published by the owner runtime.
///
/// The value is intentionally redacted from `Debug`: it is visible in the
/// owner's panel, but should not become ambient log metadata.
#[derive(Clone, PartialEq, Eq)]
pub struct CollabShareEndpoint(String);

impl CollabShareEndpoint {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.chars().count() <= MAX_COLLAB_SHARE_ENDPOINT_CHARS
            && value.is_ascii()
            && value
                .chars()
                .all(|character| !character.is_control() && !character.is_whitespace());
        valid.then_some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for CollabShareEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CollabShareEndpoint([REDACTED])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CollabParticipantUi {
    /// Epoch-local opaque roster identity. Never a subject or device id.
    pub participant_key: String,
    /// Display name retained only from verified admission metadata.
    pub display_name: String,
    /// Precomputed fallback shown when the avatar cannot be decoded/fetched.
    pub initials: String,
    /// Stable colour for this epoch, encoded as `0xRRGGBBAA`.
    pub color_rgba: u32,
    pub role: CollabUiRole,
    pub is_self: bool,
}

impl std::fmt::Debug for CollabParticipantUi {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CollabParticipantUi")
            .field("participant_key", &"[REDACTED]")
            .field("display_name", &"[REDACTED]")
            .field("initials", &"[REDACTED]")
            .field("color_rgba", &self.color_rgba)
            .field("role", &self.role)
            .field("is_self", &self.is_self)
            .finish()
    }
}

impl CollabParticipantUi {
    pub fn new(
        participant_key: impl Into<String>,
        display_name: impl Into<String>,
        color_rgba: u32,
        role: CollabUiRole,
        is_self: bool,
    ) -> Self {
        let mut participant = Self {
            participant_key: participant_key.into(),
            initials: String::new(),
            display_name: display_name.into(),
            color_rgba,
            role,
            is_self,
        };
        participant.normalize_for_ui();
        participant
    }

    fn normalize_for_ui(&mut self) {
        self.display_name = self
            .display_name
            .trim()
            .chars()
            .filter(|character| !character.is_control())
            .take(MAX_COLLAB_DISPLAY_NAME_CHARS)
            .collect();
        self.initials = participant_initials(&self.display_name);
    }
}

fn participant_initials(display_name: &str) -> String {
    let mut words = display_name
        .split_whitespace()
        .filter_map(|word| word.chars().find(|character| character.is_alphanumeric()));
    let first = words.next();
    let second = words.next();
    match (first, second) {
        (Some(a), Some(b)) => [a, b].into_iter().flat_map(char::to_uppercase).collect(),
        (Some(a), None) => a.to_uppercase().collect(),
        (None, _) => "?".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollabCanvasPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemotePresenceUi {
    pub participant_key: String,
    /// Document-space cursor position.
    pub cursor: Option<CollabCanvasPoint>,
    /// Bounded, de-duplicated node ids. Unknown/deleted ids are ignored by
    /// the painter rather than retained as document state.
    pub selection: Arc<Vec<String>>,
    pub editing_node: Option<String>,
    pub updated_at_ms: u64,
}

impl RemotePresenceUi {
    pub fn bounded(
        participant_key: impl Into<String>,
        cursor: Option<CollabCanvasPoint>,
        selection: impl IntoIterator<Item = String>,
        editing_node: Option<String>,
        updated_at_ms: u64,
    ) -> Self {
        let mut bounded = Vec::new();
        for id in selection {
            if id.is_empty() || bounded.iter().any(|existing| existing == &id) {
                continue;
            }
            bounded.push(id);
            if bounded.len() == MAX_COLLAB_UI_SELECTION_IDS {
                break;
            }
        }
        let cursor = cursor.filter(|point| point.x.is_finite() && point.y.is_finite());
        let editing_node = editing_node.filter(|id| !id.is_empty());
        Self {
            participant_key: participant_key.into(),
            cursor,
            selection: Arc::new(bounded),
            editing_node,
            updated_at_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollabPendingEditUi {
    #[default]
    None,
    Submitting,
    Replaying,
    Conflict,
}

impl CollabPendingEditUi {
    pub fn blocks_another_edit(self) -> bool {
        !matches!(self, Self::None)
    }
}

// Notice kinds and the discarded-edit projection live in `collab_notice_ui`
// (split at the 800-line cap); re-export them so existing
// `collab_ui_state::*` import paths stay stable.
pub use crate::collab_notice_ui::{
    CollabDiscardedEditUi, CollabNotice, CollabNoticeKind, CollabRejectUiCode,
    MAX_COLLAB_DISCARDED_FIELDS, MAX_COLLAB_DISCARDED_NODE_LABEL_CHARS,
};

#[derive(Clone, PartialEq, Eq)]
pub enum CollabUiAction {
    /// Open the local create-session choice screen.
    OpenCreate,
    /// Start an internet-reachable session through the configured public relay.
    Start,
    /// Start a direct session restricted to the local network.
    StartLan,
    OpenJoin,
    BeginDiscovery,
    JoinDiscovered {
        discovery_id: String,
    },
    JoinAddress {
        endpoint: String,
    },
    Cancel,
    Retry,
    Leave,
    DiscardPending,
    /// Resubmit the stashed conflict-discarded local edit as a new edit.
    ReapplyDiscarded,
    SaveAsFork,
    ApproveAdmissionEditor {
        request_key: CollabAdmissionRequestKey,
    },
    ApproveAdmissionViewer {
        request_key: CollabAdmissionRequestKey,
    },
    RejectAdmission {
        request_key: CollabAdmissionRequestKey,
    },
    /// Guest accepts the verified owner identity it was shown and lets the
    /// join proceed. Nothing from the session is applied before this.
    ConfirmOwnerIdentity {
        request_key: CollabAdmissionRequestKey,
    },
    /// Guest declines the verified owner identity; the connection closes.
    RejectOwnerIdentity {
        request_key: CollabAdmissionRequestKey,
    },
}

/// Complete shared display state. Authentication/session actors publish only
/// sanitized projections through the methods below.
#[derive(Debug, Clone)]
pub struct CollabUiState {
    pub availability: CollabAvailability,
    pub phase: CollabConnectionPhase,
    pub panel: CollabPanelState,
    pub pending_edit: CollabPendingEditUi,
    pub notice: Option<CollabNotice>,
    /// Set while the host runtime holds a replayable conflict-discarded edit.
    pub discarded_edit: Option<CollabDiscardedEditUi>,
    pub pending_action: Option<CollabUiAction>,
    pub(crate) authenticated: Option<AuthenticatedCollabSession>,
    pub(crate) public_session: CollabPublicSessionUi,
    pub(crate) pending_admissions: Arc<Vec<PendingCollabAdmissionUi>>,
    /// Guest-side owner identity awaiting an explicit human confirmation.
    pub(crate) pending_owner_confirmation: Option<crate::PendingOwnerConfirmationUi>,
    participants: Arc<Vec<CollabParticipantUi>>,
    presence: Arc<Vec<RemotePresenceUi>>,
    queued_presence: Option<Vec<RemotePresenceUi>>,
    last_presence_flush_ms: Option<u64>,
}

impl Default for CollabUiState {
    fn default() -> Self {
        Self {
            availability: CollabAvailability::Unavailable,
            phase: CollabConnectionPhase::Idle,
            panel: CollabPanelState::default(),
            pending_edit: CollabPendingEditUi::None,
            notice: None,
            discarded_edit: None,
            pending_action: None,
            authenticated: None,
            public_session: CollabPublicSessionUi::default(),
            pending_admissions: Arc::new(Vec::new()),
            pending_owner_confirmation: None,
            participants: Arc::new(Vec::new()),
            presence: Arc::new(Vec::new()),
            queued_presence: None,
            last_presence_flush_ms: None,
        }
    }
}

impl CollabUiState {
    pub fn authenticated_session(&self) -> Option<&AuthenticatedCollabSession> {
        self.phase
            .is_authenticated()
            .then_some(self.authenticated.as_ref())
            .flatten()
    }

    pub fn participants(&self) -> &[CollabParticipantUi] {
        if self.phase.is_authenticated() {
            self.participants.as_slice()
        } else {
            &[]
        }
    }

    pub fn presence(&self) -> &[RemotePresenceUi] {
        if self.phase.is_authenticated() {
            self.presence.as_slice()
        } else {
            &[]
        }
    }

    /// Publish authenticated session metadata. Pre-auth phases intentionally
    /// clear it, so discovery/join UI cannot accidentally reveal it.
    pub fn set_authenticated_session(
        &mut self,
        phase: CollabConnectionPhase,
        mut session: AuthenticatedCollabSession,
        participants: Vec<CollabParticipantUi>,
    ) -> bool {
        if !phase.is_authenticated() {
            self.clear_authenticated();
            self.set_phase(phase);
            return false;
        }
        if session.role != CollabUiRole::Owner {
            session.share_endpoint = None;
        }
        self.set_phase(phase);
        if phase == CollabConnectionPhase::Active {
            // A connect notice describes the attempt that just completed. It
            // must not survive into the authenticated session, while
            // session-scoped notices (rejects, conflicts, owner departure,
            // and so on) remain actionable until their own lifecycle retires
            // them.
            self.clear_connect_notice();
        }
        // The join decision has been made; retiring the projection keeps a
        // stale prompt from reappearing over a live session.
        self.clear_owner_confirmation();
        if self.authenticated.as_ref() != Some(&session) {
            self.panel.hover = None;
        }
        self.authenticated = Some(session);
        if self
            .authenticated
            .as_ref()
            .is_some_and(|session| session.role != CollabUiRole::Owner)
        {
            self.clear_pending_admissions();
        }
        self.set_participants(participants);
        true
    }

    pub fn clear_authenticated(&mut self) {
        self.authenticated = None;
        self.public_session = CollabPublicSessionUi::default();
        self.clear_pending_admissions();
        self.clear_owner_confirmation();
        self.discarded_edit = None;
        self.participants = Arc::new(Vec::new());
        self.clear_presence();
    }

    pub fn set_participants(&mut self, mut participants: Vec<CollabParticipantUi>) {
        if !self.phase.is_authenticated() {
            self.participants = Arc::new(Vec::new());
            self.clear_presence();
            return;
        }
        participants
            .iter_mut()
            .for_each(CollabParticipantUi::normalize_for_ui);
        let mut seen = std::collections::HashSet::new();
        participants.retain(|participant| {
            !participant.participant_key.is_empty()
                && seen.insert(participant.participant_key.clone())
        });
        participants.truncate(MAX_COLLAB_UI_PARTICIPANTS);
        if self.participants.as_slice() != participants.as_slice() {
            self.panel.hover = None;
        }
        self.participants = Arc::new(participants);
        self.retain_rostered_presence();
    }

    /// Queue the newest lossy presence projection. Repeated calls coalesce.
    pub fn queue_presence_snapshot(&mut self, mut presence: Vec<RemotePresenceUi>) {
        if !self.phase.is_authenticated() {
            self.queued_presence = None;
            return;
        }
        let participants = &self.participants;
        let mut seen = std::collections::HashSet::new();
        presence.retain(|item| {
            participants
                .iter()
                .any(|participant| participant.participant_key == item.participant_key)
                && seen.insert(item.participant_key.clone())
        });
        presence.truncate(MAX_COLLAB_UI_PARTICIPANTS);
        self.queued_presence = Some(presence);
    }

    /// Queue one participant's newest presence without replacing updates that
    /// are already waiting in the same throttle window.
    pub fn queue_presence_update(&mut self, item: RemotePresenceUi) {
        if !self.phase.is_authenticated() {
            self.queued_presence = None;
            return;
        }
        if !self
            .participants
            .iter()
            .any(|participant| participant.participant_key == item.participant_key)
        {
            return;
        }
        let presence = self
            .queued_presence
            .get_or_insert_with(|| self.presence.as_ref().clone());
        if let Some(existing) = presence
            .iter_mut()
            .find(|existing| existing.participant_key == item.participant_key)
        {
            *existing = item;
        } else if presence.len() < MAX_COLLAB_UI_PARTICIPANTS {
            presence.push(item);
        }
    }

    /// Publish at most once per 33 ms. Returns whether paint-visible state
    /// changed. Presence never carries or advances a collaboration sequence.
    pub fn flush_presence(&mut self, now_ms: u64) -> bool {
        let Some(pending) = self.queued_presence.as_ref() else {
            return false;
        };
        if self
            .last_presence_flush_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < COLLAB_PRESENCE_FRAME_INTERVAL_MS)
        {
            return false;
        }
        let next = Arc::new(pending.clone());
        self.presence = next;
        self.queued_presence = None;
        self.last_presence_flush_ms = Some(now_ms);
        true
    }

    /// Participant departure is not throttled: stale cursors disappear in the
    /// same UI turn even when a presence frame was just painted.
    pub fn remove_participant(&mut self, participant_key: &str) {
        let mut participants = self.participants.as_ref().clone();
        let before = participants.len();
        participants.retain(|participant| participant.participant_key != participant_key);
        if participants.len() != before {
            self.panel.hover = None;
        }
        self.participants = Arc::new(participants);

        let mut presence = self.presence.as_ref().clone();
        presence.retain(|item| item.participant_key != participant_key);
        self.presence = Arc::new(presence);
        if let Some(queued) = &mut self.queued_presence {
            queued.retain(|item| item.participant_key != participant_key);
        }
    }

    pub fn clear_presence(&mut self) {
        self.presence = Arc::new(Vec::new());
        self.queued_presence = None;
        self.last_presence_flush_ms = None;
    }

    pub fn take_pending_action(&mut self) -> Option<CollabUiAction> {
        self.pending_action.take()
    }

    pub fn set_notice(&mut self, kind: CollabNoticeKind, now_ms: u64) {
        self.panel.hover = None;
        self.notice = Some(CollabNotice {
            kind,
            created_at_ms: now_ms,
        });
    }

    pub(crate) fn clear_connect_notice(&mut self) {
        if self
            .notice
            .is_some_and(|notice| matches!(notice.kind, CollabNoticeKind::Connect(_)))
        {
            self.panel.hover = None;
            self.notice = None;
        }
    }

    fn retain_rostered_presence(&mut self) {
        let participants = &self.participants;
        let mut presence = self.presence.as_ref().clone();
        presence.retain(|item| {
            participants
                .iter()
                .any(|participant| participant.participant_key == item.participant_key)
        });
        self.presence = Arc::new(presence);
        if let Some(queued) = &mut self.queued_presence {
            queued.retain(|item| {
                participants
                    .iter()
                    .any(|participant| participant.participant_key == item.participant_key)
            });
        }
    }
}

#[cfg(test)]
mod presence_merge_tests;

#[cfg(test)]
mod notice_lifecycle_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn participant(key: &str) -> CollabParticipantUi {
        CollabParticipantUi::new(key, "Ada Lovelace", 0x112233ff, CollabUiRole::Editor, false)
    }

    fn presence(key: &str, updated_at_ms: u64) -> RemotePresenceUi {
        RemotePresenceUi::bounded(
            key,
            Some(CollabCanvasPoint { x: 1.0, y: 2.0 }),
            ["n1".to_string(), "n1".to_string(), "n2".to_string()],
            Some("n2".to_string()),
            updated_at_ms,
        )
    }

    #[test]
    fn pre_auth_phase_cannot_retain_session_or_profile_data() {
        let mut state = CollabUiState::default();
        assert!(!state.set_authenticated_session(
            CollabConnectionPhase::Authenticating,
            AuthenticatedCollabSession {
                session_name: "Secret design".to_string(),
                role: CollabUiRole::Editor,
                share_endpoint: None,
            },
            vec![participant("p1")],
        ));
        assert!(state.authenticated_session().is_none());
        assert!(state.participants().is_empty());

        state.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Visible only after admission".to_string(),
                role: CollabUiRole::Editor,
                share_endpoint: None,
            },
            vec![participant("p1")],
        );
        state.phase = CollabConnectionPhase::Authenticating;
        assert!(state.authenticated_session().is_none());
        assert!(state.participants().is_empty());
        assert!(state.presence().is_empty());
    }

    #[test]
    fn manual_share_endpoint_is_bounded_redacted_and_owner_only() {
        let raw_endpoint = "192.168.1.8:43120";
        let endpoint = CollabShareEndpoint::new(raw_endpoint).unwrap();
        assert_eq!(endpoint.as_str(), raw_endpoint);
        assert!(!format!("{endpoint:?}").contains(raw_endpoint));
        assert!(CollabShareEndpoint::new("").is_none());
        assert!(CollabShareEndpoint::new("192.168.1.8:43 120").is_none());
        assert!(CollabShareEndpoint::new("192.168.1.8:\u{0007}").is_none());
        assert!(CollabShareEndpoint::new("主机:43120").is_none());
        assert!(CollabShareEndpoint::new("a".repeat(MAX_COLLAB_SHARE_ENDPOINT_CHARS)).is_some());
        assert!(
            CollabShareEndpoint::new("a".repeat(MAX_COLLAB_SHARE_ENDPOINT_CHARS + 1)).is_none()
        );

        let mut owner = CollabUiState::default();
        assert!(owner.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Design".into(),
                role: CollabUiRole::Owner,
                share_endpoint: Some(endpoint),
            },
            Vec::new(),
        ));
        assert_eq!(
            owner
                .authenticated_session()
                .and_then(|session| session.share_endpoint.as_ref())
                .map(CollabShareEndpoint::as_str),
            Some(raw_endpoint)
        );

        for role in [CollabUiRole::Editor, CollabUiRole::Viewer] {
            let mut guest = CollabUiState::default();
            assert!(guest.set_authenticated_session(
                CollabConnectionPhase::Active,
                AuthenticatedCollabSession {
                    session_name: "Design".into(),
                    role,
                    share_endpoint: CollabShareEndpoint::new(raw_endpoint),
                },
                Vec::new(),
            ));
            assert!(guest
                .authenticated_session()
                .unwrap()
                .share_endpoint
                .is_none());
        }
    }

    #[test]
    fn presence_coalesces_at_thirty_hz_and_leave_cleans_immediately() {
        let mut state = CollabUiState::default();
        state.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Design".to_string(),
                role: CollabUiRole::Editor,
                share_endpoint: None,
            },
            vec![participant("p1")],
        );
        state.queue_presence_snapshot(vec![presence("p1", 1)]);
        assert!(state.flush_presence(100));
        state.queue_presence_snapshot(vec![presence("p1", 2)]);
        assert!(!state.flush_presence(120));
        assert_eq!(state.presence()[0].updated_at_ms, 1);
        assert!(state.flush_presence(133));
        assert_eq!(state.presence()[0].updated_at_ms, 2);

        state.remove_participant("p1");
        assert!(state.participants().is_empty());
        assert!(state.presence().is_empty());
    }

    #[test]
    fn initials_and_selection_are_bounded_for_paint() {
        assert_eq!(participant("p1").initials, "AL");
        let selection = (0..(MAX_COLLAB_UI_SELECTION_IDS + 10)).map(|i| format!("n{i}"));
        let item = RemotePresenceUi::bounded("p1", None, selection, None, 0);
        assert_eq!(item.selection.len(), MAX_COLLAB_UI_SELECTION_IDS);
    }

    #[test]
    fn display_projection_bounds_profile_and_non_finite_cursor() {
        let participant = CollabParticipantUi::new(
            "p1",
            "  Ada\u{0000} Lovelace  ",
            0x112233ff,
            CollabUiRole::Editor,
            false,
        );
        assert_eq!(participant.display_name, "Ada Lovelace");
        assert_eq!(participant.initials, "AL");

        let presence = RemotePresenceUi::bounded(
            "p1",
            Some(CollabCanvasPoint {
                x: f64::NAN,
                y: 1.0,
            }),
            Vec::new(),
            Some(String::new()),
            0,
        );
        assert!(presence.cursor.is_none());
        assert!(presence.editing_node.is_none());

        let participant = CollabParticipantUi::new(
            "p2",
            "Grace Hopper",
            0x112233ff,
            CollabUiRole::Editor,
            false,
        );
        let debug = format!("{participant:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("Grace Hopper"));
        assert!(!debug.contains("p2"));
    }
}

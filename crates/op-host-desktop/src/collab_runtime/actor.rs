use std::collections::HashSet;
use std::net::SocketAddr;

use op_collab::{
    AdmissionGrant, CommitSeq, ConnectionKey, ConnectionPrincipal, Epoch, Participant,
    ParticipantId, PeerId, PeerNamespace, Role, SessionId, VerifiedAuthMetadata,
};
use op_editor_core::{
    AuthenticatedCollabSession, CollabConnectionPhase, CollabParticipantUi, CollabShareEndpoint,
    CollabUiRole,
};
use op_editor_host_core::collab::{
    GuestEditorLimits, GuestEditorSession, OwnerEditorLimits, OwnerEditorSession,
};
use op_host_native::WidgetHostNative;

use super::types::{CollabRuntimeError, CollabRuntimeFailure};

const OWNER_CONNECTION_RAW: u64 = 1;

pub(super) enum EditorActor {
    Owner(Box<OwnerActor>),
    Guest(Box<GuestActor>),
}

pub(super) struct OwnerActor {
    pub(super) session: OwnerEditorSession,
    pub(super) connections: HashSet<ConnectionKey>,
    local_connection: ConnectionKey,
    share_endpoint: Option<CollabShareEndpoint>,
}

pub(super) struct GuestActor {
    pub(super) session: GuestEditorSession,
    pub(super) connection: Option<ConnectionKey>,
}

pub(super) struct PendingGuestAdmission {
    pub(super) connection: ConnectionKey,
    pub(super) session_id: SessionId,
    pub(super) epoch: Epoch,
}

impl OwnerActor {
    pub(super) fn new(
        session_id: SessionId,
        epoch: Epoch,
        auth: VerifiedAuthMetadata,
        host: &mut WidgetHostNative,
    ) -> Result<Self, CollabRuntimeError> {
        let participant_id = ParticipantId::from(random_identifier("participant")?);
        let peer_id = PeerId::from(random_identifier("peer")?);
        let namespace = random_namespace()?;
        let principal =
            ConnectionPrincipal::from_verified(auth, participant_id, peer_id, Role::Owner);
        let owner_connection =
            ConnectionKey::new(OWNER_CONNECTION_RAW).expect("constant owner key is non-zero");
        let grant = AdmissionGrant::new(principal, namespace.clone());
        let session = OwnerEditorSession::new(
            session_id,
            epoch,
            CommitSeq(0),
            owner_connection,
            grant,
            host,
            Default::default(),
            OwnerEditorLimits::default(),
        )
        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::InvalidSession))?;
        host.enable_collaboration_ids(namespace)
            .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::InvalidSession))?;
        Ok(Self {
            session,
            connections: HashSet::new(),
            local_connection: owner_connection,
            share_endpoint: None,
        })
    }

    pub(super) fn set_share_endpoint(&mut self, endpoint: Option<SocketAddr>) {
        self.share_endpoint =
            endpoint.and_then(|endpoint| CollabShareEndpoint::new(endpoint.to_string()));
    }

    pub(super) fn is_local_connection(&self, connection: ConnectionKey) -> bool {
        connection == self.local_connection
    }

    pub(super) fn grant_new_peer(
        &self,
        auth: VerifiedAuthMetadata,
        role: Role,
    ) -> Result<AdmissionGrant, CollabRuntimeError> {
        if role == Role::Owner {
            return Err(CollabRuntimeError::invalid_session());
        }
        let principal = ConnectionPrincipal::from_verified(
            auth,
            ParticipantId::from(random_identifier("participant")?),
            PeerId::from(random_identifier("peer")?),
            role,
        );
        Ok(AdmissionGrant::new(principal, random_namespace()?))
    }
}

impl GuestActor {
    pub(super) fn new(
        session_id: SessionId,
        epoch: Epoch,
        welcome: op_collab::Welcome,
        connection: ConnectionKey,
        host: &mut WidgetHostNative,
    ) -> Result<Self, CollabRuntimeError> {
        let session = GuestEditorSession::new(
            session_id,
            epoch,
            welcome,
            Default::default(),
            GuestEditorLimits::default(),
        )
        .map_err(|_| CollabRuntimeError::invalid_session())?;
        host.enable_collaboration_ids(session.core().peer_namespace().clone())
            .map_err(|_| CollabRuntimeError::invalid_session())?;
        Ok(Self {
            session,
            connection: Some(connection),
        })
    }
}

pub(super) fn owner_ui(
    actor: &OwnerActor,
) -> (AuthenticatedCollabSession, Vec<CollabParticipantUi>) {
    let self_peer = actor
        .session
        .core()
        .active_participants()
        .into_iter()
        .find(|participant| participant.role == Role::Owner);
    let self_id = self_peer
        .as_ref()
        .map(|participant| &participant.participant_id);
    (
        AuthenticatedCollabSession {
            session_name: "Shared document".to_string(),
            role: CollabUiRole::Owner,
            share_endpoint: actor.share_endpoint.clone(),
        },
        project_participants(actor.session.core().active_participants(), self_id),
    )
}

pub(super) fn guest_ui(
    actor: &GuestActor,
) -> (AuthenticatedCollabSession, Vec<CollabParticipantUi>) {
    let core = actor.session.core();
    (
        AuthenticatedCollabSession {
            session_name: "Shared document".to_string(),
            role: ui_role(core.role()),
            share_endpoint: None,
        },
        project_participants(core.participants(), Some(core.participant_id())),
    )
}

pub(super) fn set_owner_ui(host: &mut WidgetHostNative, actor: &OwnerActor) {
    let (session, participants) = owner_ui(actor);
    host.editor_state_mut()
        .editor_ui
        .collab
        .set_authenticated_session(CollabConnectionPhase::Active, session, participants);
    host.mark_editor_state_dirty();
}

pub(super) fn set_guest_ui(
    host: &mut WidgetHostNative,
    actor: &GuestActor,
    phase: CollabConnectionPhase,
) {
    let (session, participants) = guest_ui(actor);
    host.editor_state_mut()
        .editor_ui
        .collab
        .set_authenticated_session(phase, session, participants);
    host.mark_editor_state_dirty();
}

pub(super) fn project_participants(
    participants: Vec<Participant>,
    self_id: Option<&ParticipantId>,
) -> Vec<CollabParticipantUi> {
    let mut ordinal_by_role = [0_usize; 3];
    participants
        .into_iter()
        .map(|participant| {
            let role_index = match participant.role {
                Role::Owner => 0,
                Role::Editor => 1,
                Role::Viewer => 2,
            };
            ordinal_by_role[role_index] += 1;
            let ordinal = ordinal_by_role[role_index];
            let is_self = self_id == Some(&participant.participant_id);
            let fallback_name = || {
                if is_self {
                    "You".to_string()
                } else {
                    match participant.role {
                        Role::Owner => "Owner".to_string(),
                        Role::Editor => format!("Collaborator {ordinal}"),
                        Role::Viewer => format!("Viewer {ordinal}"),
                    }
                }
            };
            let key = participant.participant_id.as_ref().to_owned();
            let _ = op_editor_ui::collab_avatar_runtime::register_collab_avatar_url(
                &key,
                participant.avatar_url.as_deref(),
            );
            CollabParticipantUi::new(
                key.clone(),
                participant.display_name.unwrap_or_else(fallback_name),
                participant_color(&key),
                ui_role(participant.role),
                is_self,
            )
        })
        .collect()
}

pub(super) const fn ui_role(role: Role) -> CollabUiRole {
    match role {
        Role::Owner => CollabUiRole::Owner,
        Role::Editor => CollabUiRole::Editor,
        Role::Viewer => CollabUiRole::Viewer,
    }
}

fn participant_color(key: &str) -> u32 {
    const COLORS: [u32; 8] = [
        0x4f46e5ff, 0x0891b2ff, 0x059669ff, 0xca8a04ff, 0xea580cff, 0xdc2626ff, 0xdb2777ff,
        0x7c3aedff,
    ];
    let hash = key.bytes().fold(0_usize, |hash, byte| {
        hash.wrapping_mul(16777619) ^ usize::from(byte)
    });
    COLORS[hash % COLORS.len()]
}

pub(super) fn random_identifier(prefix: &str) -> Result<String, CollabRuntimeError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::SecureKeyUnavailable))?;
    let mut value = String::with_capacity(prefix.len() + 1 + bytes.len() * 2);
    value.push_str(prefix);
    value.push('-');
    append_hex(&mut value, &bytes);
    Ok(value)
}

pub(super) fn random_namespace() -> Result<PeerNamespace, CollabRuntimeError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::SecureKeyUnavailable))?;
    let mut value = String::with_capacity(2 + bytes.len() * 2);
    value.push_str("p-");
    append_hex(&mut value, &bytes);
    PeerNamespace::parse(value).map_err(|_| CollabRuntimeError::invalid_session())
}

fn append_hex(output: &mut String, bytes: &[u8]) {
    use std::fmt::Write as _;
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_local_connection_is_never_a_remote_socket_key() {
        let local =
            ConnectionKey::new(OWNER_CONNECTION_RAW).expect("constant owner key is non-zero");
        let remote = ConnectionKey::new(OWNER_CONNECTION_RAW + 1).expect("remote key");
        assert_eq!(local.get(), 1);
        assert_ne!(local, remote);
    }

    #[test]
    fn participant_projection_uses_only_verified_roster_profile() {
        let _guard = crate::collab_avatar_host::lock_avatar_test_registry();
        op_editor_ui::collab_avatar_runtime::begin_collab_avatar_generation(31);
        let self_id = ParticipantId::from("participant-self");
        let projected = project_participants(
            vec![
                Participant {
                    participant_id: self_id.clone(),
                    peer_id: PeerId::from("peer-self"),
                    role: Role::Editor,
                    display_name: Some("Signed Self".to_string()),
                    avatar_url: Some("https://cdn.example/self.png".to_string()),
                },
                Participant {
                    participant_id: ParticipantId::from("participant-peer"),
                    peer_id: PeerId::from("peer-peer"),
                    role: Role::Owner,
                    display_name: None,
                    avatar_url: None,
                },
            ],
            Some(&self_id),
        );

        assert_eq!(projected[0].display_name, "Signed Self");
        assert!(projected[0].is_self);
        assert_eq!(projected[1].display_name, "Owner");
        let request = op_editor_ui::collab_avatar_runtime::take_collab_avatar_requests(1)
            .pop()
            .expect("verified roster URL is registered outside EditorState");
        assert_eq!(request.url(), "https://cdn.example/self.png");
    }
}

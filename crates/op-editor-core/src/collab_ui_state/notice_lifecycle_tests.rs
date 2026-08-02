use super::*;
use crate::{CollabConnectErrorUi, CollabRejectUiCode};

fn authenticated_session() -> AuthenticatedCollabSession {
    AuthenticatedCollabSession {
        session_name: "Shared design".to_string(),
        role: CollabUiRole::Editor,
        share_endpoint: None,
    }
}

#[test]
fn authenticated_active_retires_stale_connect_notice() {
    let mut state = CollabUiState::default();
    state.set_notice(
        CollabNoticeKind::Connect(CollabConnectErrorUi::RelayUnavailable),
        7,
    );

    assert!(state.set_authenticated_session(
        CollabConnectionPhase::Active,
        authenticated_session(),
        Vec::new(),
    ));

    assert_eq!(state.notice, None);
}

#[test]
fn authenticated_active_preserves_session_notices() {
    let notices = [
        CollabNoticeKind::Reject(CollabRejectUiCode::Conflict),
        CollabNoticeKind::EditConflictDiscarded,
        CollabNoticeKind::OwnerLeft,
        CollabNoticeKind::DisconnectedReadOnly,
    ];

    for (index, kind) in notices.into_iter().enumerate() {
        let mut state = CollabUiState::default();
        state.set_notice(kind, index as u64);

        assert!(state.set_authenticated_session(
            CollabConnectionPhase::Active,
            authenticated_session(),
            Vec::new(),
        ));

        assert_eq!(
            state.notice,
            Some(CollabNotice {
                kind,
                created_at_ms: index as u64,
            })
        );
    }
}

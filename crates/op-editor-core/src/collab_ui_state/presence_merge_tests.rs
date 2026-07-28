use super::*;

fn participant(key: &str) -> CollabParticipantUi {
    CollabParticipantUi::new(key, "Ada Lovelace", 0x112233ff, CollabUiRole::Editor, false)
}

fn presence(key: &str, updated_at_ms: u64) -> RemotePresenceUi {
    RemotePresenceUi::bounded(
        key,
        Some(CollabCanvasPoint { x: 1.0, y: 2.0 }),
        Vec::new(),
        None,
        updated_at_ms,
    )
}

#[test]
fn participant_updates_merge_with_the_queued_throttle_snapshot() {
    let mut state = CollabUiState::default();
    state.set_authenticated_session(
        CollabConnectionPhase::Active,
        AuthenticatedCollabSession {
            session_name: "Design".to_string(),
            role: CollabUiRole::Editor,
            share_endpoint: None,
        },
        vec![participant("p1"), participant("p2")],
    );
    state.queue_presence_update(presence("p1", 1));
    assert!(state.flush_presence(100));
    state.queue_presence_update(presence("p1", 2));
    assert!(!state.flush_presence(120));
    state.queue_presence_update(presence("p2", 3));
    assert!(state.flush_presence(133));

    assert_eq!(state.presence().len(), 2);
    assert_eq!(state.presence()[0].updated_at_ms, 2);
    assert_eq!(state.presence()[1].updated_at_ms, 3);
}

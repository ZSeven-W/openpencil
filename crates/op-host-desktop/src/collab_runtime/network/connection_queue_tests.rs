use std::time::Instant;

use op_collab::{
    ByeReason, CollabMessage, Epoch, FrameEnvelope, ParticipantPresence, Presence, SessionId,
};
use op_collab_transport::{
    EncodedFrameTransfer, QueueError, RuntimeError, SharedQueueBudget, TransportConfig,
};

use super::super::super::types::{is_lossy_presence_frame, BudgetedFrame};
use super::queue_command_frame;
use super::tests::{admitted_pair, bye_frame};

const EXPIRES_AT_UNIX_MS: u64 = 11_000;

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from("session"), Epoch(1), message)
}

fn presence() -> Presence {
    Presence {
        cursor: None,
        selection: Vec::new(),
        viewport: None,
        editing_node: None,
    }
}

fn budgeted(frame: FrameEnvelope, budget: &SharedQueueBudget) -> BudgetedFrame {
    let lossy = is_lossy_presence_frame(&frame);
    let encoded = EncodedFrameTransfer::encode(&frame, op_collab_transport::m1_wire_limits())
        .expect("encode frame");
    let encoded_len = encoded.encoded_len();
    BudgetedFrame::new(
        encoded,
        lossy,
        budget.reserve(encoded_len).expect("bridge reservation"),
    )
}

fn queue_budgeted(
    driver: &mut op_collab_transport::ConnectionDriver,
    frame: BudgetedFrame,
    coalesce_key: Option<u64>,
) -> Result<(), RuntimeError> {
    let lossy_presence = frame.is_lossy_presence();
    let (encoded, bridge_reservation) = frame.into_parts();
    let result = queue_command_frame(
        driver,
        encoded,
        coalesce_key,
        lossy_presence,
        Instant::now(),
    );
    drop(bridge_reservation);
    result
}

fn fill_reliable_backlog(driver: &mut op_collab_transport::ConnectionDriver) {
    for _ in 0..TransportConfig::default().connections.outbound_queue_items {
        driver
            .queue_frame(&bye_frame(ByeReason::Normal), Instant::now())
            .expect("fill reliable driver backlog");
    }
}

#[test]
fn full_reliable_backlog_drops_guest_presence_update_without_failing_driver() {
    let (_owner, mut guest, _, _) = admitted_pair(EXPIRES_AT_UNIX_MS);
    fill_reliable_backlog(&mut guest);
    let queued_items = guest.queued_items();
    let bridge_budget = SharedQueueBudget::new(1024).expect("bridge budget");
    let presence = budgeted(
        frame(CollabMessage::PresenceUpdate(presence())),
        &bridge_budget,
    );

    queue_budgeted(&mut guest, presence, Some(1)).expect("lossy guest presence is dropped");
    assert_eq!(guest.queued_items(), queued_items);
    assert_eq!(bridge_budget.used().unwrap(), 0);

    let reliable = budgeted(bye_frame(ByeReason::Normal), &bridge_budget);
    assert!(matches!(
        queue_budgeted(&mut guest, reliable, None),
        Err(RuntimeError::Queue(QueueError::Full))
    ));
    assert_eq!(bridge_budget.used().unwrap(), 0);
}

#[test]
fn full_reliable_backlog_drops_owner_presence_changed_without_failing_driver() {
    let (mut owner, _guest, _, _) = admitted_pair(EXPIRES_AT_UNIX_MS);
    fill_reliable_backlog(&mut owner);
    let queued_items = owner.queued_items();
    let bridge_budget = SharedQueueBudget::new(2048).expect("bridge budget");
    let presence = budgeted(
        frame(CollabMessage::PresenceChanged(ParticipantPresence {
            participant_id: "participant-a".into(),
            peer_id: "peer-a".into(),
            presence: presence(),
        })),
        &bridge_budget,
    );

    queue_budgeted(&mut owner, presence, None).expect("lossy owner presence is dropped");
    assert_eq!(owner.queued_items(), queued_items);
    assert_eq!(bridge_budget.used().unwrap(), 0);

    let reliable = budgeted(bye_frame(ByeReason::Normal), &bridge_budget);
    assert!(matches!(
        queue_budgeted(&mut owner, reliable, None),
        Err(RuntimeError::Queue(QueueError::Full))
    ));
    assert_eq!(bridge_budget.used().unwrap(), 0);
}

use std::time::Instant;

use op_collab::{
    ByeReason, CollabMessage, Epoch, FrameEnvelope, ParticipantPresence, Presence, SessionId,
};
use op_collab_transport::{
    EncodedFrameTransfer, QueueError, RuntimeError, SharedQueueBudget, TransportConfig,
};

use super::super::super::types::{is_lossy_presence_frame, BudgetedFrame};
use super::super::connection::tests::{admitted_pair, bye_frame};
use super::queue_command_frame;

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

#[test]
fn presence_leaves_outbound_slots_for_the_commit_that_ends_a_drag() {
    // A drag republishes presence at roughly 30 Hz. Before the reserve,
    // presence alone could occupy all eight outbound slots the moment the
    // socket stalled, and the commit that closed the gesture was then refused
    // with `QueueError::Full` — fatal to the connection.
    let (mut owner, _guest, _, _) = admitted_pair(EXPIRES_AT_UNIX_MS);
    let bridge_budget = SharedQueueBudget::new(64 * 1024).expect("bridge budget");
    let capacity = TransportConfig::default().connections.outbound_queue_items;

    for _ in 0..(capacity * 4) {
        let presence = budgeted(
            frame(CollabMessage::PresenceChanged(ParticipantPresence {
                participant_id: "participant-a".into(),
                peer_id: "peer-a".into(),
                presence: presence(),
            })),
            &bridge_budget,
        );
        queue_budgeted(&mut owner, presence, None).expect("presence is droppable, never fatal");
    }

    assert!(
        owner.queued_items() <= capacity - 2,
        "presence stopped short of the reserve, leaving {} of {capacity} slots used",
        owner.queued_items()
    );

    let commit = budgeted(bye_frame(ByeReason::Normal), &bridge_budget);
    queue_budgeted(&mut owner, commit, None)
        .expect("an undroppable frame still has room after a presence flood");
}

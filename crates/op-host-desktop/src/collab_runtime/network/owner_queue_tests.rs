use op_collab::{Bye, ByeReason, CollabMessage, Epoch, FrameEnvelope, Presence, SessionId};
use op_collab_transport::{encode_frame_transfer, m1_wire_limits, EncodedFrameTransfer};

use super::*;
use crate::collab_runtime::types::BudgetedFrame;

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from("bridge-routing"), Epoch(1), message)
}

fn budgeted(frame: FrameEnvelope, budget: &SharedQueueBudget) -> Box<BudgetedFrame> {
    let lossy = crate::collab_runtime::types::is_lossy_presence_frame(&frame);
    let encoded = EncodedFrameTransfer::encode(&frame, m1_wire_limits()).unwrap();
    let encoded_len = encoded.encoded_len();
    Box::new(BudgetedFrame::new(
        encoded,
        lossy,
        budget.reserve(encoded_len).unwrap(),
    ))
}

fn active_peer_registry(
    connection: ConnectionKey,
    commands: std::sync::mpsc::SyncSender<PeerNetworkCommand>,
) -> PeerRegistry {
    let (shutdown, _shutdown_receiver) = mpsc::sync_channel(1);
    let mut peers = PeerRegistry::new();
    peers.insert(
        connection,
        PeerControl {
            commands,
            shutdown,
            cancel: None,
            phase: Arc::new(AtomicU8::new(PeerPhase::Active as u8)),
            thread: None,
        },
    );
    peers
}

#[test]
fn owner_outer_and_peer_handoffs_retain_one_shared_reservation() {
    let reliable = frame(CollabMessage::Bye(Bye {
        reason: ByeReason::Normal,
    }));
    let encoded_len = encode_frame_transfer(&reliable, m1_wire_limits())
        .unwrap()
        .1
        .len();
    let budget = SharedQueueBudget::new(encoded_len).unwrap();
    let connection = ConnectionKey::new(2).unwrap();
    let (outer_sender, outer_receiver) = mpsc::sync_channel(1);
    let (peer_sender, peer_receiver) = mpsc::sync_channel(1);
    let mut peers = active_peer_registry(connection, peer_sender);

    outer_sender
        .send(OwnerNetworkCommand::Send {
            connection,
            frame: budgeted(reliable, &budget),
            coalesce_key: None,
        })
        .unwrap();
    assert_eq!(budget.used().unwrap(), encoded_len);
    assert!(budget.reserve(1).is_err());

    let command = outer_receiver.recv().unwrap();
    assert!(route_command(command, &mut peers).unwrap());
    assert_eq!(budget.used().unwrap(), encoded_len);
    assert!(budget.reserve(1).is_err());

    let peer_command = peer_receiver.recv().unwrap();
    assert_eq!(budget.used().unwrap(), encoded_len);
    drop(peer_command);
    assert_eq!(budget.used().unwrap(), 0);
}

#[test]
fn full_peer_lane_fails_reliable_but_drops_presence() {
    let reliable = frame(CollabMessage::Bye(Bye {
        reason: ByeReason::Normal,
    }));
    let presence = frame(CollabMessage::PresenceUpdate(Presence {
        cursor: None,
        selection: Vec::new(),
        viewport: None,
        editing_node: None,
    }));
    let reliable_len = encode_frame_transfer(&reliable, m1_wire_limits())
        .unwrap()
        .1
        .len();
    let presence_len = encode_frame_transfer(&presence, m1_wire_limits())
        .unwrap()
        .1
        .len();
    let budget = SharedQueueBudget::new(reliable_len.max(presence_len)).unwrap();
    let connection = ConnectionKey::new(3).unwrap();
    let (peer_sender, _peer_receiver) = mpsc::sync_channel(1);
    peer_sender.try_send(PeerNetworkCommand::Stop).unwrap();
    let mut peers = active_peer_registry(connection, peer_sender);

    let reliable_result = route_command(
        OwnerNetworkCommand::Send {
            connection,
            frame: budgeted(reliable, &budget),
            coalesce_key: None,
        },
        &mut peers,
    );
    assert_eq!(reliable_result, Err(CollabRuntimeFailure::ResourceLimit));
    assert_eq!(budget.used().unwrap(), 0);

    assert!(route_command(
        OwnerNetworkCommand::Send {
            connection,
            frame: budgeted(presence, &budget),
            coalesce_key: Some(1),
        },
        &mut peers,
    )
    .unwrap());
    assert_eq!(budget.used().unwrap(), 0);
}

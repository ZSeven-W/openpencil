//! Network-thread ownership and bounded GUI bridge.

mod connection;
mod discovery;
mod guest;
mod lifecycle;
mod owner;
mod owner_lifecycle;
mod share_endpoint;
mod shutdown;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread::JoinHandle;

use op_collab::{ByeReason, Epoch, SessionId};
use op_collab_transport::{JoinIntent, SharedQueueBudget, StaticKeyStore};

use super::types::{
    GuestNetworkCommand, NetworkEvent, OwnerNetworkCommand, TaggedNetworkEvent,
    TerminalNetworkEvent,
};

pub(super) const GUI_COMMAND_CAPACITY: usize = 256;
pub(super) const GUI_EVENT_CAPACITY: usize = 256;
const GUI_TERMINAL_EVENT_CAPACITY: usize = 256;
const DISCOVERY_COMMAND_CAPACITY: usize = 4;
const TERMINAL_COMMAND_CAPACITY: usize = 1;
pub(super) const TERMINAL_FLUSH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

pub(super) use lifecycle::{PendingNetworkLaunch, Retirement};

#[derive(Clone)]
pub(super) struct EventSink {
    sender: SyncSender<TaggedNetworkEvent>,
    terminal: TerminalEventLane,
    bridge_budget: SharedQueueBudget,
    wake: Arc<dyn Fn() + Send + Sync>,
    generation: u64,
}

impl EventSink {
    pub(super) fn new(
        sender: SyncSender<TaggedNetworkEvent>,
        terminal: TerminalEventLane,
        bridge_budget: SharedQueueBudget,
        wake: Arc<dyn Fn() + Send + Sync>,
        generation: u64,
    ) -> Self {
        Self {
            sender,
            terminal,
            bridge_budget,
            wake,
            generation,
        }
    }

    pub(super) fn try_send(&self, event: NetworkEvent) -> Result<(), EventSendError> {
        self.try_send_reserved(event, None)
    }

    /// Reserves decoded frame bytes until the GUI finishes handling the event.
    pub(super) fn try_send_sized(
        &self,
        event: NetworkEvent,
        encoded_len: usize,
    ) -> Result<(), EventSendError> {
        let reservation = self
            .bridge_budget
            .reserve(encoded_len)
            .map_err(|_| EventSendError::Full)?;
        self.try_send_reserved(event, Some(reservation))
    }

    fn try_send_reserved(
        &self,
        event: NetworkEvent,
        bridge_reservation: Option<op_collab_transport::SharedQueueReservation>,
    ) -> Result<(), EventSendError> {
        match self.sender.try_send(TaggedNetworkEvent {
            generation: self.generation,
            event,
            bridge_reservation,
        }) {
            Ok(()) => {
                (self.wake)();
                Ok(())
            }
            Err(TrySendError::Full(_)) => Err(EventSendError::Full),
            Err(TrySendError::Disconnected(_)) => Err(EventSendError::Disconnected),
        }
    }

    /// Compatibility wrapper for lifecycle producers outside socket drivers.
    ///
    /// A full bridge is treated like a disconnected GUI: callers stop their
    /// bounded worker instead of blocking socket ownership indefinitely.
    pub(super) fn send(&self, event: NetworkEvent) -> bool {
        self.try_send(event).is_ok()
    }

    /// Retains terminal state on an independent bounded lane without waiting
    /// for the GUI. Overflow becomes a generation-tagged resource failure,
    /// which still forces the authoritative UI out of Active.
    pub(super) fn send_terminal(&self, event: TerminalNetworkEvent) -> bool {
        let delivered = self.terminal.try_send(TaggedNetworkEvent {
            generation: self.generation,
            event: event.into(),
            bridge_reservation: None,
        });
        (self.wake)();
        delivered
    }
}

#[derive(Clone)]
pub(super) struct TerminalEventLane {
    sender: SyncSender<TaggedNetworkEvent>,
    overflow_generation: Arc<AtomicU64>,
}

impl TerminalEventLane {
    fn try_send(&self, event: TaggedNetworkEvent) -> bool {
        match self.sender.try_send(event) {
            Ok(()) => true,
            Err(TrySendError::Full(event)) => {
                self.overflow_generation
                    .fetch_max(event.generation, Ordering::Release);
                true
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }
}

pub(super) struct TerminalEventReceiver {
    receiver: Receiver<TaggedNetworkEvent>,
    overflow_generation: Arc<AtomicU64>,
}

impl TerminalEventReceiver {
    pub(super) fn try_recv(&self) -> Result<TaggedNetworkEvent, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    pub(super) fn take_overflow(&self, generation: u64) -> bool {
        self.overflow_generation.swap(0, Ordering::AcqRel) == generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EventSendError {
    Full,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NetworkCommandSendError {
    Full,
    Disconnected,
}

fn send_command<T>(sender: &SyncSender<T>, command: T) -> Result<(), NetworkCommandSendError> {
    match sender.try_send(command) {
        Ok(()) => Ok(()),
        Err(TrySendError::Full(_)) => Err(NetworkCommandSendError::Full),
        Err(TrySendError::Disconnected(_)) => Err(NetworkCommandSendError::Disconnected),
    }
}

pub(super) enum SessionNetwork {
    Owner {
        commands: SyncSender<OwnerNetworkCommand>,
        shutdown: SyncSender<ByeReason>,
        thread: Option<JoinHandle<()>>,
    },
    Guest {
        commands: SyncSender<GuestNetworkCommand>,
        shutdown: SyncSender<ByeReason>,
        thread: Option<JoinHandle<()>>,
    },
}

impl SessionNetwork {
    pub(super) fn send_owner(
        &self,
        command: OwnerNetworkCommand,
    ) -> Result<(), NetworkCommandSendError> {
        match self {
            Self::Owner { commands, .. } => send_command(commands, command),
            Self::Guest { .. } => Err(NetworkCommandSendError::Disconnected),
        }
    }

    pub(super) fn send_guest(
        &self,
        command: GuestNetworkCommand,
    ) -> Result<(), NetworkCommandSendError> {
        match self {
            Self::Guest { commands, .. } => send_command(commands, command),
            Self::Owner { .. } => Err(NetworkCommandSendError::Disconnected),
        }
    }

    pub(super) fn request_shutdown(&self) {
        match self {
            Self::Owner { shutdown, .. } => {
                let _ = shutdown.try_send(ByeReason::OwnerLeft);
            }
            Self::Guest { shutdown, .. } => {
                let _ = shutdown.try_send(ByeReason::Normal);
            }
        }
    }

    fn take_thread(&mut self) -> Option<JoinHandle<()>> {
        match self {
            Self::Owner { thread, .. } | Self::Guest { thread, .. } => thread.take(),
        }
    }

    pub(super) fn into_thread(mut self) -> JoinHandle<()> {
        self.request_shutdown();
        self.take_thread()
            .expect("live collaboration network owns one worker")
    }
}

impl Drop for SessionNetwork {
    fn drop(&mut self) {
        self.request_shutdown();
        if let Some(thread) = self.take_thread() {
            let _ = Retirement::start(vec![thread], Arc::new(|| {}));
        }
    }
}

pub(super) struct DiscoveryNetwork {
    stop: SyncSender<()>,
    thread: Option<JoinHandle<()>>,
}

impl DiscoveryNetwork {
    pub(super) fn stop(&self) {
        let _ = self.stop.try_send(());
    }

    pub(super) fn into_thread(mut self) -> JoinHandle<()> {
        self.stop();
        self.thread
            .take()
            .expect("live discovery network owns one worker")
    }
}

impl Drop for DiscoveryNetwork {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = Retirement::start(vec![thread], Arc::new(|| {}));
        }
    }
}

pub(super) fn event_channel() -> (
    SyncSender<TaggedNetworkEvent>,
    Receiver<TaggedNetworkEvent>,
    TerminalEventLane,
    TerminalEventReceiver,
) {
    event_channel_with_capacity(GUI_EVENT_CAPACITY, GUI_TERMINAL_EVENT_CAPACITY)
}

fn event_channel_with_capacity(
    data_capacity: usize,
    terminal_capacity: usize,
) -> (
    SyncSender<TaggedNetworkEvent>,
    Receiver<TaggedNetworkEvent>,
    TerminalEventLane,
    TerminalEventReceiver,
) {
    let (sender, receiver) = mpsc::sync_channel(data_capacity);
    let (terminal_sender, terminal_receiver) = mpsc::sync_channel(terminal_capacity);
    let overflow_generation = Arc::new(AtomicU64::new(0));
    (
        sender,
        receiver,
        TerminalEventLane {
            sender: terminal_sender,
            overflow_generation: Arc::clone(&overflow_generation),
        },
        TerminalEventReceiver {
            receiver: terminal_receiver,
            overflow_generation,
        },
    )
}

#[cfg(test)]
pub(super) fn owner_command_channel_for_test() -> (SessionNetwork, Receiver<OwnerNetworkCommand>) {
    owner_command_channel_with_capacity_for_test(GUI_COMMAND_CAPACITY)
}

#[cfg(test)]
pub(super) fn owner_command_channel_with_capacity_for_test(
    capacity: usize,
) -> (SessionNetwork, Receiver<OwnerNetworkCommand>) {
    let (network, receiver, _shutdown) = owner_channels_with_capacity_for_test(capacity);
    (network, receiver)
}

#[cfg(test)]
pub(super) fn owner_channels_with_capacity_for_test(
    capacity: usize,
) -> (
    SessionNetwork,
    Receiver<OwnerNetworkCommand>,
    Receiver<ByeReason>,
) {
    let (commands, receiver) = mpsc::sync_channel(capacity);
    let (shutdown, shutdown_receiver) = mpsc::sync_channel(TERMINAL_COMMAND_CAPACITY);
    let thread = std::thread::spawn(|| {});
    (
        SessionNetwork::Owner {
            commands,
            shutdown,
            thread: Some(thread),
        },
        receiver,
        shutdown_receiver,
    )
}

#[cfg(test)]
pub(super) fn guest_command_channel_with_capacity_for_test(
    capacity: usize,
) -> (SessionNetwork, Receiver<GuestNetworkCommand>) {
    let (network, receiver, _shutdown) = guest_channels_with_capacity_for_test(capacity);
    (network, receiver)
}

#[cfg(test)]
pub(super) fn guest_channels_with_capacity_for_test(
    capacity: usize,
) -> (
    SessionNetwork,
    Receiver<GuestNetworkCommand>,
    Receiver<ByeReason>,
) {
    let (commands, receiver) = mpsc::sync_channel(capacity);
    let (shutdown, shutdown_receiver) = mpsc::sync_channel(TERMINAL_COMMAND_CAPACITY);
    let thread = std::thread::spawn(|| {});
    (
        SessionNetwork::Guest {
            commands,
            shutdown,
            thread: Some(thread),
        },
        receiver,
        shutdown_receiver,
    )
}

pub(super) fn spawn_owner(
    sink: EventSink,
    key_store: Arc<dyn StaticKeyStore>,
    bind_address: SocketAddr,
    session_id: SessionId,
    epoch: Epoch,
) -> SessionNetwork {
    let (commands, receiver) = mpsc::sync_channel(GUI_COMMAND_CAPACITY);
    let (shutdown, shutdown_receiver) = mpsc::sync_channel(TERMINAL_COMMAND_CAPACITY);
    let thread = std::thread::Builder::new()
        .name("op-collab-owner-network".to_string())
        .spawn(move || {
            owner::run(
                sink,
                key_store,
                bind_address,
                session_id,
                epoch,
                receiver,
                shutdown_receiver,
            );
        })
        .expect("spawn collaboration owner network thread");
    SessionNetwork::Owner {
        commands,
        shutdown,
        thread: Some(thread),
    }
}

pub(super) fn spawn_guest(
    sink: EventSink,
    key_store: Arc<dyn StaticKeyStore>,
    addresses: Vec<SocketAddr>,
    expected_discovery_id: Option<String>,
    expected_remote_static: Option<[u8; 32]>,
    intent: JoinIntent,
) -> SessionNetwork {
    let (commands, receiver) = mpsc::sync_channel(GUI_COMMAND_CAPACITY);
    let (shutdown, shutdown_receiver) = mpsc::sync_channel(TERMINAL_COMMAND_CAPACITY);
    let thread = std::thread::Builder::new()
        .name("op-collab-guest-network".to_string())
        .spawn(move || {
            guest::run(
                sink,
                key_store,
                guest::GuestTarget {
                    addresses,
                    expected_discovery_id,
                    expected_remote_static,
                    intent,
                },
                receiver,
                shutdown_receiver,
            );
        })
        .expect("spawn collaboration guest network thread");
    SessionNetwork::Guest {
        commands,
        shutdown,
        thread: Some(thread),
    }
}

pub(super) fn spawn_discovery(sink: EventSink) -> DiscoveryNetwork {
    let (stop, receiver) = mpsc::sync_channel(DISCOVERY_COMMAND_CAPACITY);
    let thread = std::thread::Builder::new()
        .name("op-collab-discovery".to_string())
        .spawn(move || discovery::run(sink, receiver))
        .expect("spawn collaboration discovery thread");
    DiscoveryNetwork {
        stop,
        thread: Some(thread),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use op_collab::{
        Bye, ByeReason, CollabMessage, ConnectionKey, Epoch, FrameEnvelope, SessionId,
    };
    use op_collab_transport::{encode_frame_transfer, m1_wire_limits};

    use super::*;

    fn sink_with_capacity(
        data_capacity: usize,
        terminal_capacity: usize,
    ) -> (
        EventSink,
        Receiver<TaggedNetworkEvent>,
        TerminalEventReceiver,
        Arc<AtomicUsize>,
    ) {
        let (sender, receiver, terminal, terminal_receiver) =
            event_channel_with_capacity(data_capacity, terminal_capacity);
        let wakes = Arc::new(AtomicUsize::new(0));
        let wake_counter = Arc::clone(&wakes);
        let sink = EventSink::new(
            sender,
            terminal,
            SharedQueueBudget::new(
                op_collab_transport::TransportConfig::default()
                    .connections
                    .global_queued_bytes,
            )
            .expect("default bridge budget"),
            Arc::new(move || {
                wake_counter.fetch_add(1, Ordering::Relaxed);
            }),
            7,
        );
        (sink, receiver, terminal_receiver, wakes)
    }

    #[test]
    fn data_delivery_is_nonblocking_when_gui_queue_is_full() {
        let (sink, receiver, _terminal, wakes) = sink_with_capacity(1, 1);
        assert_eq!(sink.try_send(NetworkEvent::Stopped), Ok(()));
        assert_eq!(
            sink.try_send(NetworkEvent::Stopped),
            Err(EventSendError::Full)
        );
        assert_eq!(wakes.load(Ordering::Relaxed), 1);
        assert_eq!(receiver.recv().unwrap().generation, 7);
    }

    #[test]
    fn terminal_delivery_uses_an_independent_bounded_lane() {
        let (sink, receiver, terminal, wakes) = sink_with_capacity(1, 1);
        sink.try_send(NetworkEvent::Stopped).unwrap();
        assert!(sink.send_terminal(TerminalNetworkEvent::Stopped));
        assert!(matches!(
            terminal.try_recv(),
            Ok(TaggedNetworkEvent {
                event: NetworkEvent::Stopped,
                ..
            })
        ));
        assert_eq!(receiver.recv().unwrap().generation, 7);
        assert_eq!(wakes.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn terminal_lane_overflow_is_retained_as_a_resource_failure() {
        let (sink, _receiver, terminal, wakes) = sink_with_capacity(1, 1);
        assert!(sink.send_terminal(TerminalNetworkEvent::Stopped));
        assert!(sink.send_terminal(TerminalNetworkEvent::Stopped));
        assert!(terminal.take_overflow(7));
        assert!(!terminal.take_overflow(7));
        assert_eq!(wakes.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn sized_data_event_holds_shared_bytes_until_tagged_event_is_dropped() {
        let frame = FrameEnvelope::new(
            SessionId::from("bridge-budget"),
            Epoch(1),
            CollabMessage::Bye(Bye {
                reason: ByeReason::Normal,
            }),
        );
        let encoded_len = encode_frame_transfer(&frame, m1_wire_limits())
            .unwrap()
            .1
            .len();
        let budget = SharedQueueBudget::new(encoded_len).unwrap();
        let (sender, receiver, terminal, _terminal_receiver) = event_channel_with_capacity(2, 1);
        let sink = EventSink::new(sender, terminal, budget.clone(), Arc::new(|| {}), 9);
        let connection = ConnectionKey::new(1).unwrap();

        sink.try_send_sized(NetworkEvent::Frame { connection, frame }, encoded_len)
            .unwrap();
        assert_eq!(budget.used().unwrap(), encoded_len);
        assert_eq!(
            sink.try_send_sized(NetworkEvent::Stopped, 1),
            Err(EventSendError::Full)
        );

        let tagged = receiver.recv().unwrap();
        assert_eq!(budget.used().unwrap(), encoded_len);
        assert_eq!(
            tagged
                .bridge_reservation
                .as_ref()
                .map(op_collab_transport::SharedQueueReservation::amount),
            Some(encoded_len)
        );
        drop(tagged);
        assert_eq!(budget.used().unwrap(), 0);
    }

    #[test]
    fn full_normal_lane_cannot_hide_owner_left_shutdown() {
        let (network, _commands, shutdown) = owner_channels_with_capacity_for_test(1);
        let connection = ConnectionKey::new(9).unwrap();
        network
            .send_owner(OwnerNetworkCommand::Close { connection })
            .unwrap();
        assert_eq!(
            network.send_owner(OwnerNetworkCommand::Close { connection }),
            Err(NetworkCommandSendError::Full)
        );

        network.request_shutdown();
        assert_eq!(shutdown.recv().unwrap(), ByeReason::OwnerLeft);
    }

    #[test]
    fn full_normal_lane_cannot_hide_guest_normal_shutdown() {
        let (network, _commands, shutdown) = guest_channels_with_capacity_for_test(1);
        network
            .send_guest(GuestNetworkCommand::VerifyRenewal(
                op_collab::OpaqueTicket::new("renewal-one".to_owned()).unwrap(),
            ))
            .unwrap();
        assert_eq!(
            network.send_guest(GuestNetworkCommand::VerifyRenewal(
                op_collab::OpaqueTicket::new("renewal-two".to_owned()).unwrap(),
            )),
            Err(NetworkCommandSendError::Full)
        );

        network.request_shutdown();
        assert_eq!(shutdown.recv().unwrap(), ByeReason::Normal);
    }
}

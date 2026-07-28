use std::collections::HashMap;
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::JoinHandle;

use op_collab::{ByeReason, ConnectionKey};

use super::super::types::PeerNetworkCommand;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum PeerPhase {
    Joining = 0,
    Approval = 1,
    Active = 2,
    Done = 3,
}

impl PeerPhase {
    fn load(value: &AtomicU8) -> Self {
        match value.load(Ordering::Acquire) {
            0 => Self::Joining,
            1 => Self::Approval,
            2 => Self::Active,
            _ => Self::Done,
        }
    }
}

pub(super) struct PeerControl {
    pub(super) commands: SyncSender<PeerNetworkCommand>,
    pub(super) shutdown: SyncSender<ByeReason>,
    pub(super) cancel: Option<TcpStream>,
    pub(super) phase: Arc<AtomicU8>,
    pub(super) thread: Option<JoinHandle<()>>,
}

impl PeerControl {
    fn signal_shutdown(&self, reason: ByeReason) {
        let _ = self.shutdown.try_send(reason);
        if PeerPhase::load(&self.phase) != PeerPhase::Active {
            if let Some(cancel) = self.cancel.as_ref() {
                let _ = cancel.shutdown(Shutdown::Both);
            }
        }
    }

    fn join(mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(super) struct PeerRegistry {
    pub(super) peers: HashMap<ConnectionKey, PeerControl>,
    exit_reason: ByeReason,
    shutdown_sent: bool,
}

impl PeerRegistry {
    pub(super) fn new() -> Self {
        Self {
            peers: HashMap::new(),
            exit_reason: ByeReason::OwnerLeft,
            shutdown_sent: false,
        }
    }

    pub(super) fn set_exit_reason(&mut self, reason: ByeReason) {
        self.exit_reason = reason;
    }

    pub(super) fn insert(&mut self, connection: ConnectionKey, control: PeerControl) {
        self.peers.insert(connection, control);
    }

    pub(super) fn reap(&mut self, connection: ConnectionKey) {
        if let Some(control) = self.peers.remove(&connection) {
            control.join();
        }
    }

    pub(super) fn signal_all(&mut self) {
        if self.shutdown_sent {
            return;
        }
        for control in self.peers.values() {
            control.signal_shutdown(self.exit_reason);
        }
        self.shutdown_sent = true;
    }
}

impl Drop for PeerRegistry {
    fn drop(&mut self) {
        self.signal_all();
        for (_, control) in self.peers.drain() {
            control.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{ErrorKind, Read};
    use std::net::{Ipv4Addr, TcpListener};
    use std::sync::mpsc::{self, Receiver};

    use super::*;

    fn registry_with_phase(
        connection: ConnectionKey,
        phase: PeerPhase,
        cancel: Option<TcpStream>,
    ) -> (PeerRegistry, Receiver<ByeReason>) {
        let (commands, _commands_receiver) = mpsc::sync_channel(1);
        let (shutdown, shutdown_receiver) = mpsc::sync_channel(1);
        let mut peers = PeerRegistry::new();
        peers.insert(
            connection,
            PeerControl {
                commands,
                shutdown,
                cancel,
                phase: Arc::new(AtomicU8::new(phase as u8)),
                thread: None,
            },
        );
        (peers, shutdown_receiver)
    }

    fn tcp_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        (server, client)
    }

    #[test]
    fn joining_and_approval_shutdown_cancel_the_blocking_socket() {
        for phase in [PeerPhase::Joining, PeerPhase::Approval] {
            let (server, mut client) = tcp_pair();
            let connection = ConnectionKey::new(u64::from(phase as u8) + 20).unwrap();
            let (mut peers, _terminal) = registry_with_phase(connection, phase, Some(server));
            peers.signal_all();
            let mut byte = [0_u8; 1];
            match client.read(&mut byte) {
                Ok(0) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::ConnectionReset | ErrorKind::BrokenPipe
                    ) => {}
                result => panic!("shutdown must cancel blocked peer socket: {result:?}"),
            }
        }
    }

    #[test]
    fn active_shutdown_preserves_socket_until_terminal_drain() {
        let (server, client) = tcp_pair();
        client.set_nonblocking(true).unwrap();
        let connection = ConnectionKey::new(30).unwrap();
        let (mut peers, terminal) =
            registry_with_phase(connection, PeerPhase::Active, Some(server));
        peers.set_exit_reason(ByeReason::OwnerLeft);
        peers.signal_all();
        assert_eq!(terminal.recv().unwrap(), ByeReason::OwnerLeft);
        let mut byte = [0_u8; 1];
        assert!(matches!(
            client.peek(&mut byte),
            Err(error) if error.kind() == ErrorKind::WouldBlock
        ));
    }

    #[test]
    fn auth_expiry_reason_reaches_only_the_active_terminal_lane() {
        let connection = ConnectionKey::new(31).unwrap();
        let (mut peers, terminal) = registry_with_phase(connection, PeerPhase::Active, None);
        peers.set_exit_reason(ByeReason::AuthenticationExpired);
        peers.signal_all();
        assert_eq!(terminal.recv().unwrap(), ByeReason::AuthenticationExpired);
    }
}

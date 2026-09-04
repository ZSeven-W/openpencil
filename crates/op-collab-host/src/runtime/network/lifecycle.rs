use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use op_collab::{Epoch, SessionId};
use op_collab_transport::JoinIntent;

use super::{DiscoveryNetwork, SessionNetwork};
use crate::runtime::relay::{GuestConnectionRoute, RelayOwnerRequest};
#[cfg(test)]
use crate::runtime::types::{CollabRuntimeError, CollabRuntimeFailure};
use crate::runtime::CollabRuntime;

pub(in crate::runtime) struct PendingNetworkLaunch {
    generation: u64,
    kind: PendingNetworkLaunchKind,
}

enum PendingNetworkLaunchKind {
    Owner {
        session_id: SessionId,
        epoch: Epoch,
        relay: Option<RelayOwnerRequest>,
    },
    Guest {
        route: GuestConnectionRoute,
        intent: JoinIntent,
    },
    Discovery,
}

/// Completion acknowledgement for one retired collaboration generation.
///
/// The GUI owns at most one of these at a time. Joining happens on the
/// short-lived reaper thread, so checking the acknowledgement never blocks the
/// event loop.
pub(in crate::runtime) enum Retirement {
    Background { complete: Arc<AtomicBool> },
    Local { handles: Vec<JoinHandle<()>> },
}

struct RetirementCompletion(Arc<AtomicBool>);

impl Drop for RetirementCompletion {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl Retirement {
    pub(in crate::runtime) fn start(
        handles: Vec<JoinHandle<()>>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        debug_assert!(!handles.is_empty());
        let complete = Arc::new(AtomicBool::new(false));
        let worker_complete = Arc::clone(&complete);
        let pending = Arc::new(Mutex::new(Some(handles)));
        let worker_pending = Arc::clone(&pending);
        let spawn = std::thread::Builder::new()
            .name("op-collab-network-reaper".to_string())
            .spawn(move || {
                let _completion = RetirementCompletion(worker_complete);
                let handles = worker_pending
                    .lock()
                    .expect("lifecycle handle lock")
                    .take()
                    .expect("reaper owns one retired generation");
                for handle in handles {
                    let _ = handle.join();
                }
                // Platform callbacks are foreign-code boundaries: one must not
                // kill the reaper and strand `shutdown` forever. The completion
                // guard publishes only after this callback returns or unwinds.
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| wake()));
            });
        match spawn {
            Ok(_reaper) => Self::Background { complete },
            Err(_) => Self::Local {
                handles: pending
                    .lock()
                    .expect("lifecycle handle lock")
                    .take()
                    .expect("failed spawn retains the retired generation"),
            },
        }
    }

    pub(super) fn poll_complete(&mut self) -> bool {
        match self {
            Self::Background { complete } => complete.load(Ordering::Acquire),
            Self::Local { handles } if handles.iter().all(JoinHandle::is_finished) => {
                for handle in handles.drain(..) {
                    let _ = handle.join();
                }
                true
            }
            Self::Local { .. } => false,
        }
    }
}

impl CollabRuntime {
    /// Whether a host without a worker-thread event-loop wake primitive should
    /// keep scheduling short polling ticks.
    pub fn needs_poll(&self) -> bool {
        self.network.is_some()
            || self.discovery.is_some()
            || self.retirement.is_some()
            || self.pending_network_launch.is_some()
            || self.next_reconnect_deadline().is_some()
    }

    /// Leave the session and synchronously join every network worker.
    ///
    /// Embedders must call this before releasing platform callback state.
    pub fn shutdown(&mut self, host: &mut impl crate::host::CollabHost) {
        self.leave(host);
        while self.retirement.is_some() {
            self.reap_retirement();
            if self.retirement.is_some() {
                std::thread::yield_now();
            }
        }
    }

    pub(in crate::runtime) fn reap_retirement(&mut self) -> bool {
        let complete = self
            .retirement
            .as_mut()
            .is_some_and(Retirement::poll_complete);
        if complete {
            self.retirement = None;
        }
        complete
    }

    #[cfg(test)]
    pub(in crate::runtime) fn require_worker_slot(&mut self) -> Result<(), CollabRuntimeError> {
        self.reap_retirement();
        if self.retirement.is_none() {
            Ok(())
        } else {
            Err(CollabRuntimeError::new(CollabRuntimeFailure::ResourceLimit))
        }
    }

    #[cfg(test)]
    pub(in crate::runtime) fn wait_for_worker_slot_for_test(&mut self) {
        // Tests that inject a replacement transport must cross the same
        // retirement acknowledgement gate as launch_pending_network.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while self.require_worker_slot().is_err() {
            assert!(
                std::time::Instant::now() < deadline,
                "retired collaboration worker must release its slot"
            );
            std::thread::yield_now();
        }
    }

    pub(in crate::runtime) fn retire_workers(&mut self) {
        // A generation identifies one worker incarnation, not the retained
        // guest session actor. Fence it before requesting shutdown so neither
        // already-buffered nor subsequently-arriving events can revive a
        // retired transport.
        self.advance_generation();
        self.pending_network_launch = None;
        // The worker that was blocked on the guest's confirmation is going
        // away; its routing key must not outlive it and answer for whatever
        // connects next.
        self.pending_owner_confirmation = None;
        self.reap_retirement();
        if self.retirement.is_some() {
            debug_assert!(self.network.is_none() && self.discovery.is_none());
            return;
        }
        let mut handles = Vec::with_capacity(2);
        if let Some(network) = self.network.take() {
            handles.push(SessionNetwork::into_thread(network));
        }
        if let Some(discovery) = self.discovery.take() {
            handles.push(DiscoveryNetwork::into_thread(discovery));
        }
        if handles.is_empty() {
            return;
        }
        let wake_notifier = Arc::clone(&self.wake_notifier);
        self.retirement = Some(Retirement::start(
            handles,
            Arc::new(move || {
                if let Ok(slot) = wake_notifier.lock() {
                    if let Some(notify) = slot.as_ref() {
                        notify();
                    }
                }
            }),
        ));
    }

    pub(in crate::runtime) fn defer_owner_launch(
        &mut self,
        session_id: SessionId,
        epoch: Epoch,
        relay: Option<RelayOwnerRequest>,
    ) {
        self.pending_network_launch = Some(PendingNetworkLaunch {
            generation: self.generation,
            kind: PendingNetworkLaunchKind::Owner {
                session_id,
                epoch,
                relay,
            },
        });
    }

    pub(in crate::runtime) fn defer_guest_launch(
        &mut self,
        route: GuestConnectionRoute,
        intent: JoinIntent,
    ) {
        self.pending_network_launch = Some(PendingNetworkLaunch {
            generation: self.generation,
            kind: PendingNetworkLaunchKind::Guest { route, intent },
        });
    }

    pub(in crate::runtime) fn defer_discovery_launch(&mut self) {
        self.pending_network_launch = Some(PendingNetworkLaunch {
            generation: self.generation,
            kind: PendingNetworkLaunchKind::Discovery,
        });
    }

    pub(in crate::runtime) fn launch_pending_network(&mut self) -> bool {
        let Some(launch) = self.take_ready_network_launch() else {
            return false;
        };
        match launch.kind {
            PendingNetworkLaunchKind::Owner {
                session_id,
                epoch,
                relay,
            } => {
                self.network = Some(super::spawn_owner(
                    self.event_sink(),
                    Arc::clone(&self.key_store),
                    "[::]:0".parse().expect("constant owner bind address"),
                    session_id,
                    epoch,
                    relay,
                    self.transport_capabilities.lan_hosting,
                ));
            }
            PendingNetworkLaunchKind::Guest { route, intent } => {
                self.network = Some(super::spawn_guest(
                    self.event_sink(),
                    Arc::clone(&self.key_store),
                    route,
                    intent,
                ));
            }
            PendingNetworkLaunchKind::Discovery => {
                self.discovery = Some(super::spawn_discovery(self.event_sink()));
            }
        }
        true
    }

    fn take_ready_network_launch(&mut self) -> Option<PendingNetworkLaunch> {
        self.reap_retirement();
        if self.retirement.is_some() || self.network.is_some() || self.discovery.is_some() {
            return None;
        }
        let launch = self.pending_network_launch.take()?;
        (launch.generation == self.generation).then_some(launch)
    }

    #[cfg(test)]
    pub(in crate::runtime) fn take_ready_network_launch_for_test(&mut self) -> bool {
        self.take_ready_network_launch().is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, TcpListener};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::*;

    fn wait_until_complete(retirement: &mut Retirement) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !retirement.poll_complete() {
            assert!(
                Instant::now() < deadline,
                "retired generation must be reaped within its bounded worker shutdown"
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn repeated_retirement_reaps_workers_and_releases_listener_ports() {
        for _ in 0..16 {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind listener");
            let endpoint = listener.local_addr().expect("listener address");
            let (stop, stopped) = mpsc::sync_channel(1);
            let (ready, started) = mpsc::sync_channel(1);
            let live = Arc::new(AtomicUsize::new(0));
            let worker_live = Arc::clone(&live);
            let worker = std::thread::spawn(move || {
                worker_live.fetch_add(1, Ordering::SeqCst);
                ready.send(()).unwrap();
                let _ = stopped.recv();
                drop(listener);
                worker_live.fetch_sub(1, Ordering::SeqCst);
            });

            started.recv().unwrap();
            assert_eq!(live.load(Ordering::SeqCst), 1);
            stop.send(()).expect("request worker stop");
            let mut retirement = Retirement::start(vec![worker], Arc::new(|| {}));
            wait_until_complete(&mut retirement);
            assert_eq!(live.load(Ordering::SeqCst), 0);

            let rebound = TcpListener::bind(endpoint).expect("retirement releases listener port");
            drop(rebound);
        }
    }

    #[test]
    fn acknowledgement_waits_for_every_worker_in_the_generation() {
        let (first_stop, first_stopped) = mpsc::sync_channel(1);
        let (second_stop, second_stopped) = mpsc::sync_channel(1);
        let first = std::thread::spawn(move || {
            let _ = first_stopped.recv();
        });
        let second = std::thread::spawn(move || {
            let _ = second_stopped.recv();
        });
        let mut retirement = Retirement::start(vec![first, second], Arc::new(|| {}));

        first_stop.send(()).unwrap();
        std::thread::yield_now();
        assert!(!retirement.poll_complete());
        second_stop.send(()).unwrap();
        wait_until_complete(&mut retirement);
    }

    #[test]
    fn discovery_to_join_waits_for_retirement_ack_without_losing_the_launch() {
        let mut runtime = CollabRuntime::new();
        let generation = runtime.generation;
        let (release, released) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let _ = released.recv();
        });
        runtime.retirement = Some(Retirement::start(vec![worker], Arc::new(|| {})));
        runtime.pending_network_launch = Some(PendingNetworkLaunch {
            generation,
            kind: PendingNetworkLaunchKind::Guest {
                route: GuestConnectionRoute::lan(
                    vec!["127.0.0.1:43120".parse().unwrap()],
                    Some("fast-discovery-to-join".to_owned()),
                    None,
                ),
                intent: JoinIntent::New,
            },
        });

        assert!(runtime.take_ready_network_launch().is_none());
        assert!(runtime.pending_network_launch.is_some());
        release.send(()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !runtime.reap_retirement() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(matches!(
            runtime.take_ready_network_launch(),
            Some(PendingNetworkLaunch {
                generation: ready_generation,
                kind: PendingNetworkLaunchKind::Guest { .. },
            }) if ready_generation == generation
        ));
    }

    #[test]
    fn repeated_start_then_leave_cancels_the_stale_pending_generation() {
        let mut runtime = CollabRuntime::new();
        runtime.pending_network_launch = Some(PendingNetworkLaunch {
            generation: runtime.generation,
            kind: PendingNetworkLaunchKind::Discovery,
        });
        let retired_generation = runtime.generation;
        runtime.retire_workers();

        assert_ne!(runtime.generation, retired_generation);
        assert!(runtime.pending_network_launch.is_none());
        assert!(runtime.take_ready_network_launch().is_none());
    }

    #[test]
    fn shutdown_reaps_workers_before_returning() {
        let worker = std::thread::spawn(|| {});
        let mut runtime = CollabRuntime::new();
        runtime.retirement = Some(Retirement::start(vec![worker], Arc::new(|| {})));
        assert!(runtime.needs_poll());

        let mut host = crate::HeadlessCollabHost::new();
        runtime.shutdown(&mut host);

        assert!(!runtime.needs_poll());
        assert_eq!(
            host.editor_state().editor_ui.collab.phase,
            op_editor_core::CollabConnectionPhase::Idle
        );
    }

    #[test]
    fn shutdown_cannot_finish_before_the_reapers_final_wake_returns() {
        let worker = std::thread::spawn(|| {});
        let (wake_started, started) = mpsc::sync_channel(1);
        let (release_wake, released) = mpsc::sync_channel(1);
        let released = Arc::new(Mutex::new(released));
        let wake = {
            let released = Arc::clone(&released);
            Arc::new(move || {
                wake_started.send(()).unwrap();
                released.lock().unwrap().recv().unwrap();
            })
        };
        let mut runtime = CollabRuntime::new();
        runtime.retirement = Some(Retirement::start(vec![worker], wake));
        let mut host = crate::HeadlessCollabHost::new();

        std::thread::scope(|scope| {
            let shutdown = scope.spawn(|| runtime.shutdown(&mut host));
            started.recv_timeout(Duration::from_secs(1)).unwrap();
            assert!(
                !shutdown.is_finished(),
                "Acquire completion must remain false while wake is in flight"
            );
            release_wake.send(()).unwrap();
            shutdown.join().unwrap();
        });
        assert!(!runtime.needs_poll());
    }

    #[test]
    fn panicking_final_wake_cannot_strand_shutdown() {
        let worker = std::thread::spawn(|| {});
        let mut runtime = CollabRuntime::new();
        runtime.retirement = Some(Retirement::start(
            vec![worker],
            Arc::new(|| panic!("simulated platform notifier panic")),
        ));
        let (finished, completion) = mpsc::sync_channel(1);

        let shutdown = std::thread::spawn(move || {
            let mut host = crate::HeadlessCollabHost::new();
            runtime.shutdown(&mut host);
            finished.send(runtime.needs_poll()).unwrap();
        });

        assert!(!completion.recv_timeout(Duration::from_secs(2)).unwrap());
        shutdown.join().unwrap();
    }
}

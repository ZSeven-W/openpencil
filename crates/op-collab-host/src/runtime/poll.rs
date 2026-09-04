use std::sync::mpsc::TryRecvError;

use super::types::{CollabRuntimeFailure, NetworkEvent, TaggedNetworkEvent};
use super::CollabRuntime;
use crate::host::CollabHost;

const MAX_EVENTS_PER_POLL: usize = 256;

impl CollabRuntime {
    pub fn poll(&mut self, host: &mut impl CollabHost) -> bool {
        self.poll_with_hooks(host, || {}, CollabRuntime::launch_pending_network)
    }

    fn poll_with_hooks(
        &mut self,
        host: &mut impl CollabHost,
        after_initial_data_drain: impl FnOnce(),
        launch_pending_network: impl FnOnce(&mut CollabRuntime) -> bool,
    ) -> bool {
        let mut changed = false;
        let mut data_events = Vec::new();
        drain_events(
            || self.events.try_recv(),
            self.generation,
            &mut data_events,
            MAX_EVENTS_PER_POLL,
        );
        after_initial_data_drain();

        let mut terminal_events = Vec::new();
        drain_events(
            || self.terminal_events.try_recv(),
            self.generation,
            &mut terminal_events,
            MAX_EVENTS_PER_POLL,
        );
        if self.terminal_events.take_overflow(self.generation) {
            terminal_events.push(TaggedNetworkEvent {
                generation: self.generation,
                event: NetworkEvent::Failed(CollabRuntimeFailure::ResourceLimit),
                bridge_reservation: None,
                _lane_seat: None,
            });
        }

        // Data and lifecycle events use independent bounded lanes. Once a
        // terminal event is visible, every data event sent before it by the
        // same worker is also visible, but it may have arrived just after the
        // first data scan observed Empty. Drain that causal prefix before
        // applying EOF/failure so an authoritative frame cannot land after its
        // worker has been retired.
        if !terminal_events.is_empty() {
            drain_events(
                || self.events.try_recv(),
                self.generation,
                &mut data_events,
                MAX_EVENTS_PER_POLL,
            );
        }

        for tagged in data_events.into_iter().chain(terminal_events) {
            // Handling one event may retire the current worker generation.
            // Events already cached in these vectors must cross the same fence
            // as events still waiting in either channel.
            if tagged.generation != self.generation {
                continue;
            }
            let TaggedNetworkEvent {
                event,
                bridge_reservation,
                ..
            } = tagged;
            changed |= self.handle_event(event, host);
            // Keep decoded frame bytes charged while `handle_event` applies
            // them, including while other ready events wait in `events`.
            drop(bridge_reservation);
        }
        let now = self.now_ms();
        if host.editor_state_mut().editor_ui.collab.flush_presence(now) {
            host.mark_editor_state_dirty();
            changed = true;
        }
        // Retirement acknowledgement means old producers are joined, but both
        // bounded GUI lanes may still contain their final generation-tagged
        // backlog. Launch only after this poll has purged and processed both
        // lanes so a stale full queue cannot reject a new worker's first event.
        changed |= self.launch_due_guest_reconnect(host);
        changed |= launch_pending_network(self);
        changed
    }

    #[cfg(test)]
    pub(in crate::runtime) fn poll_with_after_initial_data_drain(
        &mut self,
        host: &mut impl CollabHost,
        after_initial_data_drain: impl FnOnce(),
    ) -> bool {
        self.poll_with_hooks(
            host,
            after_initial_data_drain,
            CollabRuntime::launch_pending_network,
        )
    }

    #[cfg(test)]
    pub(in crate::runtime) fn poll_with_launch_probe(
        &mut self,
        host: &mut impl CollabHost,
        launch_probe: impl FnOnce(&mut CollabRuntime) -> bool,
    ) -> bool {
        self.poll_with_hooks(host, || {}, launch_probe)
    }
}

fn drain_events(
    mut try_recv: impl FnMut() -> Result<TaggedNetworkEvent, TryRecvError>,
    generation: u64,
    output: &mut Vec<TaggedNetworkEvent>,
    limit: usize,
) {
    for _ in 0..limit {
        match try_recv() {
            Ok(event) if event.generation == generation => output.push(event),
            Ok(_) => {}
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
        }
    }
}

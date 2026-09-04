use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use op_collab_relay_protocol::{RelayRole, RouteMapKey};
use tokio::sync::{mpsc, oneshot, Mutex, OwnedSemaphorePermit, Semaphore};

use crate::observe::QueueCensus;

pub(crate) struct QueuedPayload {
    bytes: Vec<u8>,
    _global_budget: OwnedSemaphorePermit,
    _route_budget: OwnedSemaphorePermit,
}

impl QueuedPayload {
    pub(crate) fn new(
        bytes: Vec<u8>,
        global_budget: OwnedSemaphorePermit,
        route_budget: OwnedSemaphorePermit,
    ) -> Self {
        Self {
            bytes,
            _global_budget: global_budget,
            _route_budget: route_budget,
        }
    }

    pub(crate) fn into_parts(self) -> (Vec<u8>, OwnedSemaphorePermit, OwnedSemaphorePermit) {
        (self.bytes, self._global_budget, self._route_budget)
    }
}

#[derive(Clone)]
pub(crate) struct Registry {
    inner: Arc<Mutex<RegistryState>>,
    max_active_pairs: usize,
    max_waiting_per_route: usize,
    max_queued_bytes_per_route: usize,
}

impl Registry {
    pub(crate) fn new(
        max_active_pairs: usize,
        max_waiting_per_route: usize,
        max_queued_bytes_per_route: usize,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RegistryState::default())),
            max_active_pairs,
            max_waiting_per_route,
            max_queued_bytes_per_route,
        }
    }

    pub(crate) async fn register(
        &self,
        route: RouteMapKey,
        role: RelayRole,
        relay_tx: mpsc::Sender<QueuedPayload>,
    ) -> Result<Registration, RegistryError> {
        let mut state = self.inner.lock().await;
        let counterpart_role = opposite(role);

        loop {
            let counterpart = state
                .routes
                .get_mut(&route)
                .and_then(|queue| queue.for_role_mut(counterpart_role).pop_front());
            let Some(counterpart) = counterpart else {
                break;
            };
            state.waiting = state.waiting.saturating_sub(1);

            if state.active_pairs.len() >= self.max_active_pairs {
                state
                    .routes
                    .entry(route)
                    .or_default()
                    .for_role_mut(counterpart_role)
                    .push_front(counterpart);
                state.waiting += 1;
                return Err(RegistryError::Capacity);
            }

            let pair_id = state.next_pair_id;
            state.next_pair_id = state.next_pair_id.wrapping_add(1);
            let route_budget = Arc::clone(
                state
                    .route_budgets
                    .entry(route)
                    .or_insert_with(|| Arc::new(Semaphore::new(self.max_queued_bytes_per_route))),
            );
            let (counterpart_ready_tx, counterpart_ready_rx) = oneshot::channel();
            let (newcomer_ready_tx, newcomer_ready_rx) = oneshot::channel();
            let counterpart_notice = PairNotice {
                pair_id,
                peer_tx: relay_tx.clone(),
                route_budget: Arc::clone(&route_budget),
                ready_tx: Some(counterpart_ready_tx),
                peer_ready_rx: newcomer_ready_rx,
            };
            if counterpart.pair_tx.send(counterpart_notice).is_err() {
                continue;
            }

            state.active_pairs.insert(pair_id, route);
            state.remove_route_if_unused(route);
            return Ok(Registration::Paired(PairNotice {
                pair_id,
                peer_tx: counterpart.relay_tx,
                route_budget,
                ready_tx: Some(newcomer_ready_tx),
                peer_ready_rx: counterpart_ready_rx,
            }));
        }

        let waiting_on_route = state.routes.get(&route).map_or(0, RouteQueue::waiting);
        if waiting_on_route >= self.max_waiting_per_route {
            return Err(RegistryError::Capacity);
        }

        let id = state.next_waiter_id;
        state.next_waiter_id = state.next_waiter_id.wrapping_add(1);
        let (pair_tx, pair_rx) = oneshot::channel();
        state
            .routes
            .entry(route)
            .or_default()
            .for_role_mut(role)
            .push_back(WaitingPeer {
                id,
                relay_tx,
                pair_tx,
            });
        state.waiting += 1;
        let waiting_on_route = state.routes.get(&route).map_or(0, RouteQueue::waiting);

        Ok(Registration::Waiting(WaitingRegistration {
            id,
            route,
            role,
            waiting_on_route,
            pair_rx,
        }))
    }

    /// Sample the live queue depths for the operational census.
    ///
    /// Depths only — never a route key — so the sample is safe to log. Walks
    /// the route table once per census interval, which is cheap next to the
    /// per-connection work already taken under this lock.
    pub(crate) async fn queue_census(&self) -> QueueCensus {
        let state = self.inner.lock().await;
        let mut census = QueueCensus {
            waiting_routes: state.routes.len(),
            active_pairs: state.active_pairs.len(),
            ..QueueCensus::default()
        };
        for queue in state.routes.values() {
            census.waiting_owners += queue.owners.len();
            census.waiting_guests += queue.guests.len();
            let depth = queue.waiting();
            census.max_route_queue_depth = census.max_route_queue_depth.max(depth);
            if depth >= self.max_waiting_per_route {
                census.routes_at_waiting_capacity += 1;
            }
        }
        census
    }

    pub(crate) async fn unregister_waiter(&self, waiting: &WaitingRegistration) {
        let mut state = self.inner.lock().await;
        let removed = state.routes.get_mut(&waiting.route).is_some_and(|queue| {
            let peers = queue.for_role_mut(waiting.role);
            let before = peers.len();
            peers.retain(|peer| peer.id != waiting.id);
            before != peers.len()
        });
        if removed {
            state.waiting = state.waiting.saturating_sub(1);
        }
        state.remove_route_if_unused(waiting.route);
    }

    pub(crate) async fn close_pair(&self, pair_id: u64) {
        let mut state = self.inner.lock().await;
        if let Some(route) = state.active_pairs.remove(&pair_id) {
            state.remove_route_if_unused(route);
        }
    }
}

pub(crate) enum Registration {
    Waiting(WaitingRegistration),
    Paired(PairNotice),
}

pub(crate) struct WaitingRegistration {
    pub(crate) id: u64,
    pub(crate) route: RouteMapKey,
    pub(crate) role: RelayRole,
    /// Peers queued on this route, including this one, at registration time.
    pub(crate) waiting_on_route: usize,
    pub(crate) pair_rx: oneshot::Receiver<PairNotice>,
}

pub(crate) struct PairNotice {
    pub(crate) pair_id: u64,
    pub(crate) peer_tx: mpsc::Sender<QueuedPayload>,
    pub(crate) route_budget: Arc<Semaphore>,
    ready_tx: Option<oneshot::Sender<()>>,
    peer_ready_rx: oneshot::Receiver<()>,
}

impl PairNotice {
    pub(crate) async fn synchronize(&mut self, timeout: std::time::Duration) -> bool {
        let Some(ready_tx) = self.ready_tx.take() else {
            return false;
        };
        if ready_tx.send(()).is_err() {
            return false;
        }
        matches!(
            tokio::time::timeout(timeout, &mut self.peer_ready_rx).await,
            Ok(Ok(()))
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistryError {
    Capacity,
}

#[derive(Default)]
struct RegistryState {
    routes: HashMap<RouteMapKey, RouteQueue>,
    waiting: usize,
    active_pairs: HashMap<u64, RouteMapKey>,
    route_budgets: HashMap<RouteMapKey, Arc<Semaphore>>,
    next_waiter_id: u64,
    next_pair_id: u64,
}

impl RegistryState {
    fn remove_route_if_unused(&mut self, route: RouteMapKey) {
        if self.routes.get(&route).is_some_and(RouteQueue::is_empty) {
            self.routes.remove(&route);
        }
        if !self.routes.contains_key(&route)
            && !self
                .active_pairs
                .values()
                .any(|active_route| *active_route == route)
        {
            self.route_budgets.remove(&route);
        }
    }
}

#[derive(Default)]
struct RouteQueue {
    owners: VecDeque<WaitingPeer>,
    guests: VecDeque<WaitingPeer>,
}

impl RouteQueue {
    fn for_role_mut(&mut self, role: RelayRole) -> &mut VecDeque<WaitingPeer> {
        match role {
            RelayRole::Owner => &mut self.owners,
            RelayRole::Guest => &mut self.guests,
        }
    }

    fn waiting(&self) -> usize {
        self.owners.len() + self.guests.len()
    }

    fn is_empty(&self) -> bool {
        self.owners.is_empty() && self.guests.is_empty()
    }
}

struct WaitingPeer {
    id: u64,
    relay_tx: mpsc::Sender<QueuedPayload>,
    pair_tx: oneshot::Sender<PairNotice>,
}

fn opposite(role: RelayRole) -> RelayRole {
    match role {
        RelayRole::Owner => RelayRole::Guest,
        RelayRole::Guest => RelayRole::Owner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU64;

    use op_collab_relay_protocol::{RouteCapability, RouteId};

    fn route(byte: u8) -> RouteMapKey {
        RouteMapKey::derive(
            &RouteId::new([byte; 16]).expect("non-zero route"),
            NonZeroU64::MIN,
            &RouteCapability::new([byte; 32]).expect("non-zero capability"),
        )
    }

    async fn pair(registry: &Registry, route: RouteMapKey) -> (PairNotice, PairNotice) {
        let (owner_tx, _owner_rx) = mpsc::channel(1);
        let owner = match registry
            .register(route, RelayRole::Owner, owner_tx)
            .await
            .expect("owner registration")
        {
            Registration::Waiting(waiting) => waiting,
            Registration::Paired(_) => panic!("first role must wait"),
        };
        let (guest_tx, _guest_rx) = mpsc::channel(1);
        let guest = match registry
            .register(route, RelayRole::Guest, guest_tx)
            .await
            .expect("guest registration")
        {
            Registration::Paired(pair) => pair,
            Registration::Waiting(_) => panic!("counterpart must pair"),
        };
        let owner = owner.pair_rx.await.expect("owner pair notice");
        (owner, guest)
    }

    #[tokio::test]
    async fn concurrent_pairs_on_one_route_share_the_route_byte_budget() {
        let registry = Registry::new(4, 4, 1_024);
        let (first_owner, first_guest) = pair(&registry, route(1)).await;
        let (second_owner, second_guest) = pair(&registry, route(1)).await;

        assert!(Arc::ptr_eq(
            &first_owner.route_budget,
            &first_guest.route_budget
        ));
        assert!(Arc::ptr_eq(
            &first_owner.route_budget,
            &second_owner.route_budget
        ));
        assert!(Arc::ptr_eq(
            &second_owner.route_budget,
            &second_guest.route_budget
        ));
    }

    #[tokio::test]
    async fn independent_routes_do_not_share_the_route_byte_budget() {
        let registry = Registry::new(4, 4, 1_024);
        let (first, _) = pair(&registry, route(1)).await;
        let (second, _) = pair(&registry, route(2)).await;

        assert!(!Arc::ptr_eq(&first.route_budget, &second.route_budget));
    }
}

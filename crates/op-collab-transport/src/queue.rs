use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use op_collab::SensitiveFrameJson;
use zeroize::Zeroizing;

use crate::{FrameTransportError, QueueError, TransferClass};

pub(crate) struct QueueItem {
    class: TransferClass,
    bytes: QueueBytes,
    coalesce_key: Option<u64>,
}

enum QueueBytes {
    Shared(Arc<[u8]>),
    Sensitive(SensitiveQueueBytes),
}

enum SensitiveQueueBytes {
    Frame(SensitiveFrameJson),
    Admission(Zeroizing<Vec<u8>>),
}

impl QueueBytes {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Shared(bytes) => bytes,
            Self::Sensitive(SensitiveQueueBytes::Frame(bytes)) => bytes.as_bytes(),
            Self::Sensitive(SensitiveQueueBytes::Admission(bytes)) => bytes,
        }
    }
}

impl QueueItem {
    pub(crate) fn reliable(
        class: TransferClass,
        bytes: impl Into<Arc<[u8]>>,
    ) -> Result<Self, FrameTransportError> {
        if class == TransferClass::Ticket {
            return Err(FrameTransportError::SensitiveTransferNotShareable);
        }
        Ok(Self {
            class,
            bytes: QueueBytes::Shared(bytes.into()),
            coalesce_key: None,
        })
    }

    /// Creates a lossy latest-value item, intended for presence.
    pub(crate) fn coalescing(
        class: TransferClass,
        key: u64,
        bytes: impl Into<Arc<[u8]>>,
    ) -> Result<Self, FrameTransportError> {
        if class == TransferClass::Ticket {
            return Err(FrameTransportError::SensitiveTransferNotShareable);
        }
        Ok(Self {
            class,
            bytes: QueueBytes::Shared(bytes.into()),
            coalesce_key: Some(key),
        })
    }

    pub(crate) fn sensitive_ticket_frame(bytes: SensitiveFrameJson) -> Self {
        Self {
            class: TransferClass::Ticket,
            bytes: QueueBytes::Sensitive(SensitiveQueueBytes::Frame(bytes)),
            coalesce_key: None,
        }
    }

    pub(crate) fn sensitive_admission(bytes: Zeroizing<Vec<u8>>) -> Self {
        Self {
            class: TransferClass::Ticket,
            bytes: QueueBytes::Sensitive(SensitiveQueueBytes::Admission(bytes)),
            coalesce_key: None,
        }
    }

    pub(crate) const fn class(&self) -> TransferClass {
        self.class
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub(crate) fn encoded_len(&self) -> usize {
        self.bytes().len()
    }
}

impl fmt::Debug for QueueItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueueItem")
            .field("class", &self.class)
            .field("encoded_len", &self.encoded_len())
            .field("coalesce_key", &self.coalesce_key)
            .finish()
    }
}

#[derive(Debug)]
pub(crate) struct BoundedTransferQueue {
    items: VecDeque<QueueItem>,
    max_items: usize,
    max_bytes: usize,
    queued_bytes: usize,
    shared_budget: Option<SharedQueueBudget>,
}

impl BoundedTransferQueue {
    #[cfg(test)]
    pub(crate) fn new(max_items: usize, max_bytes: usize) -> Result<Self, QueueError> {
        Self::build(max_items, max_bytes, None)
    }

    pub(crate) fn with_shared_budget(
        max_items: usize,
        max_bytes: usize,
        shared_budget: SharedQueueBudget,
    ) -> Result<Self, QueueError> {
        Self::build(max_items, max_bytes, Some(shared_budget))
    }

    fn build(
        max_items: usize,
        max_bytes: usize,
        shared_budget: Option<SharedQueueBudget>,
    ) -> Result<Self, QueueError> {
        if max_items == 0 || max_bytes == 0 {
            return Err(QueueError::Full);
        }
        Ok(Self {
            items: VecDeque::new(),
            max_items,
            max_bytes,
            queued_bytes: 0,
            shared_budget,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn queued_bytes(&self) -> usize {
        self.queued_bytes
    }

    pub(crate) fn replaces_queued_item(&self, item: &QueueItem) -> bool {
        item.coalesce_key.is_some_and(|key| {
            self.items
                .iter()
                .any(|queued| queued.coalesce_key == Some(key))
        })
    }

    pub(crate) fn push(&mut self, item: QueueItem) -> Result<(), QueueError> {
        let item_bytes = item.encoded_len();
        if item_bytes > self.max_bytes {
            return Err(QueueError::ItemTooLarge);
        }
        if let Some(key) = item.coalesce_key {
            if let Some(index) = self
                .items
                .iter()
                .position(|queued| queued.coalesce_key == Some(key))
            {
                let previous = self.items[index].encoded_len();
                let next_total = self
                    .queued_bytes
                    .checked_sub(previous)
                    .and_then(|bytes| bytes.checked_add(item_bytes))
                    .ok_or(QueueError::ByteBudget)?;
                if next_total > self.max_bytes {
                    return Err(QueueError::ByteBudget);
                }
                if item_bytes > previous {
                    self.reserve_shared(item_bytes - previous)?;
                }
                self.items[index] = item;
                self.queued_bytes = next_total;
                if previous > item_bytes {
                    self.release_shared(previous - item_bytes);
                }
                return Ok(());
            }
        }
        if self.items.len() >= self.max_items {
            return Err(QueueError::Full);
        }
        let next_total = self
            .queued_bytes
            .checked_add(item_bytes)
            .ok_or(QueueError::ByteBudget)?;
        if next_total > self.max_bytes {
            return Err(QueueError::ByteBudget);
        }
        self.reserve_shared(item_bytes)?;
        self.items.push_back(item);
        self.queued_bytes = next_total;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn pop(&mut self) -> Option<QueueItem> {
        let item = self.items.pop_front()?;
        self.queued_bytes -= item.encoded_len();
        self.release_shared(item.encoded_len());
        Some(item)
    }

    pub(crate) fn take_reserved(&mut self) -> Option<ReservedQueueItem> {
        let item = self.items.pop_front()?;
        self.queued_bytes -= item.encoded_len();
        Some(ReservedQueueItem {
            item,
            shared_budget: self.shared_budget.clone(),
        })
    }

    fn reserve_shared(&self, amount: usize) -> Result<(), QueueError> {
        match &self.shared_budget {
            Some(budget) => budget.reserve_untracked(amount),
            None => Ok(()),
        }
    }

    fn release_shared(&self, amount: usize) {
        if let Some(budget) = &self.shared_budget {
            budget.release(amount);
        }
    }
}

#[derive(Debug)]
pub(crate) struct ReservedQueueItem {
    item: QueueItem,
    shared_budget: Option<SharedQueueBudget>,
}

impl ReservedQueueItem {
    pub(crate) const fn class(&self) -> TransferClass {
        self.item.class()
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        self.item.bytes()
    }

    pub(crate) fn encoded_len(&self) -> usize {
        self.item.encoded_len()
    }
}

impl Drop for ReservedQueueItem {
    fn drop(&mut self) {
        if let Some(budget) = &self.shared_budget {
            budget.release(self.item.encoded_len());
        }
    }
}

impl Drop for BoundedTransferQueue {
    fn drop(&mut self) {
        self.release_shared(self.queued_bytes);
        self.queued_bytes = 0;
    }
}

#[derive(Debug)]
struct SharedQueueBudgetState {
    maximum: usize,
    used: usize,
}

/// A process-wide byte budget that can be shared by every peer queue.
#[derive(Debug, Clone)]
pub struct SharedQueueBudget {
    state: Arc<Mutex<SharedQueueBudgetState>>,
}

/// An RAII reservation against a [`SharedQueueBudget`].
///
/// This is useful for bounded cross-thread handoff queues that sit in front of
/// the transport queue. Keeping the reservation with the handed-off value
/// makes the byte bound survive channel routing and early-return paths.
#[derive(Debug)]
#[must_use = "dropping the reservation immediately releases its byte budget"]
pub struct SharedQueueReservation {
    budget: SharedQueueBudget,
    amount: usize,
}

impl SharedQueueReservation {
    pub const fn amount(&self) -> usize {
        self.amount
    }
}

impl Drop for SharedQueueReservation {
    fn drop(&mut self) {
        self.budget.release(self.amount);
    }
}

impl SharedQueueBudget {
    pub fn new(maximum: usize) -> Result<Self, QueueError> {
        if maximum == 0 {
            return Err(QueueError::ByteBudget);
        }
        Ok(Self {
            state: Arc::new(Mutex::new(SharedQueueBudgetState { maximum, used: 0 })),
        })
    }

    pub fn used(&self) -> Result<usize, QueueError> {
        self.state
            .lock()
            .map(|state| state.used)
            .map_err(|_| QueueError::Unavailable)
    }

    pub fn maximum(&self) -> Result<usize, QueueError> {
        self.state
            .lock()
            .map(|state| state.maximum)
            .map_err(|_| QueueError::Unavailable)
    }

    pub fn reserve(&self, amount: usize) -> Result<SharedQueueReservation, QueueError> {
        self.reserve_untracked(amount)?;
        Ok(SharedQueueReservation {
            budget: self.clone(),
            amount,
        })
    }

    fn reserve_untracked(&self, amount: usize) -> Result<(), QueueError> {
        let mut state = self.state.lock().map_err(|_| QueueError::Unavailable)?;
        let next = state
            .used
            .checked_add(amount)
            .ok_or(QueueError::ByteBudget)?;
        if next > state.maximum {
            return Err(QueueError::ByteBudget);
        }
        state.used = next;
        Ok(())
    }

    fn release(&self, amount: usize) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.used = state.used.saturating_sub(amount);
    }
}

#[derive(Debug, Clone)]
pub struct TokenBucket {
    rate_per_second: u64,
    capacity: u64,
    available: u64,
    fractional: u128,
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(rate_per_second: u64, capacity: u64, now: Instant) -> Result<Self, QueueError> {
        if rate_per_second == 0 || capacity == 0 {
            return Err(QueueError::Full);
        }
        Ok(Self {
            rate_per_second,
            capacity,
            available: capacity,
            fractional: 0,
            last_refill: now,
        })
    }

    pub fn available(&mut self, now: Instant) -> u64 {
        self.refill(now);
        self.available
    }

    pub fn try_take(&mut self, amount: u64, now: Instant) -> bool {
        self.refill(now);
        if amount > self.available {
            return false;
        }
        self.available -= amount;
        true
    }

    /// Returns the deterministic refill delay, or `None` when `amount`
    /// exceeds this bucket's capacity and can never become available.
    pub fn time_until_available(&mut self, amount: u64, now: Instant) -> Option<Duration> {
        self.refill(now);
        if amount <= self.available {
            return Some(Duration::ZERO);
        }
        if amount > self.capacity {
            return None;
        }
        let deficit = u128::from(amount - self.available);
        let scaled = deficit
            .saturating_mul(1_000_000_000)
            .saturating_sub(self.fractional);
        let nanos = scaled.div_ceil(u128::from(self.rate_per_second));
        Some(Duration::from_nanos(
            u64::try_from(nanos).unwrap_or(u64::MAX),
        ))
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_refill);
        self.last_refill = now;
        let numerator = elapsed
            .as_nanos()
            .saturating_mul(u128::from(self.rate_per_second))
            .saturating_add(self.fractional);
        let whole = numerator / 1_000_000_000;
        self.fractional = numerator % 1_000_000_000;
        let added = u64::try_from(whole).unwrap_or(u64::MAX);
        self.available = self.available.saturating_add(added).min(self.capacity);
        if self.available == self.capacity {
            self.fractional = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_not_impl_any;
    use std::time::Duration;

    assert_not_impl_any!(QueueItem: Clone);

    fn reliable(class: TransferClass, bytes: Vec<u8>) -> QueueItem {
        QueueItem::reliable(class, bytes).unwrap()
    }

    fn coalescing(class: TransferClass, key: u64, bytes: Vec<u8>) -> QueueItem {
        QueueItem::coalescing(class, key, bytes).unwrap()
    }

    #[test]
    fn queue_bounds_count_and_bytes() {
        let mut queue = BoundedTransferQueue::new(2, 10).unwrap();
        queue
            .push(reliable(TransferClass::Control, vec![1; 6]))
            .unwrap();
        assert_eq!(
            queue.push(reliable(TransferClass::Control, vec![2; 5])),
            Err(QueueError::ByteBudget)
        );
        queue
            .push(reliable(TransferClass::Control, vec![3; 4]))
            .unwrap();
        assert_eq!(
            queue.push(reliable(TransferClass::Control, vec![4; 1])),
            Err(QueueError::Full)
        );
        assert_eq!(queue.queued_bytes(), 10);
        assert_eq!(queue.pop().unwrap().encoded_len(), 6);
        assert_eq!(queue.queued_bytes(), 4);
    }

    #[test]
    fn coalescing_item_replaces_without_growing_count() {
        let mut queue = BoundedTransferQueue::new(1, 8).unwrap();
        queue
            .push(coalescing(TransferClass::Control, 7, vec![1; 8]))
            .unwrap();
        queue
            .push(coalescing(TransferClass::Control, 7, vec![2; 3]))
            .unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.queued_bytes(), 3);
        assert_eq!(queue.pop().unwrap().bytes(), &[2; 3]);
    }

    #[test]
    fn token_bucket_refills_deterministically() {
        let start = Instant::now();
        let mut bucket = TokenBucket::new(100, 200, start).unwrap();
        assert!(bucket.try_take(200, start));
        assert!(!bucket.try_take(1, start));
        assert_eq!(bucket.available(start + Duration::from_millis(250)), 25);
        assert!(bucket.try_take(25, start + Duration::from_millis(250)));
        assert_eq!(bucket.available(start + Duration::from_secs(10)), 200);

        assert!(bucket.try_take(200, start + Duration::from_secs(10)));
        assert_eq!(
            bucket.time_until_available(25, start + Duration::from_secs(10)),
            Some(Duration::from_millis(250))
        );
        assert_eq!(
            bucket.time_until_available(201, start + Duration::from_secs(10)),
            None
        );
    }

    #[test]
    fn shared_budget_bounds_aggregate_queues_and_releases_on_drop() {
        let budget = SharedQueueBudget::new(10).unwrap();
        let mut first = BoundedTransferQueue::with_shared_budget(2, 10, budget.clone()).unwrap();
        let mut second = BoundedTransferQueue::with_shared_budget(2, 10, budget.clone()).unwrap();
        first
            .push(reliable(TransferClass::Control, vec![1; 6]))
            .unwrap();
        assert_eq!(
            second.push(reliable(TransferClass::Control, vec![2; 5])),
            Err(QueueError::ByteBudget)
        );
        second
            .push(reliable(TransferClass::Control, vec![3; 4]))
            .unwrap();
        assert_eq!(budget.used().unwrap(), 10);
        first.pop();
        assert_eq!(budget.used().unwrap(), 4);
        drop(second);
        assert_eq!(budget.used().unwrap(), 0);
    }

    #[test]
    fn coalescing_updates_the_shared_budget_exactly() {
        let budget = SharedQueueBudget::new(8).unwrap();
        let mut queue = BoundedTransferQueue::with_shared_budget(1, 8, budget.clone()).unwrap();
        queue
            .push(coalescing(TransferClass::Control, 1, vec![1; 6]))
            .unwrap();
        queue
            .push(coalescing(TransferClass::Control, 1, vec![2; 8]))
            .unwrap();
        assert_eq!(budget.used().unwrap(), 8);
        queue
            .push(coalescing(TransferClass::Control, 1, vec![3; 3]))
            .unwrap();
        assert_eq!(budget.used().unwrap(), 3);
    }

    #[test]
    fn ticket_items_require_the_unique_sensitive_constructor() {
        const SECRET: &[u8] = b"opaque-ticket-secret";
        assert!(matches!(
            QueueItem::reliable(TransferClass::Ticket, SECRET.to_vec()),
            Err(FrameTransportError::SensitiveTransferNotShareable)
        ));
        assert!(matches!(
            QueueItem::coalescing(TransferClass::Ticket, 9, SECRET.to_vec()),
            Err(FrameTransportError::SensitiveTransferNotShareable)
        ));

        let ticket = QueueItem::sensitive_admission(Zeroizing::new(SECRET.to_vec()));
        let debug = format!("{ticket:?}");
        assert_eq!(ticket.class(), TransferClass::Ticket);
        assert!(debug.contains("class: Ticket"));
        assert!(debug.contains(&format!("encoded_len: {}", SECRET.len())));
        assert!(!debug.contains("opaque-ticket-secret"));
        assert!(!debug.contains(&format!("{SECRET:?}")));
    }

    #[test]
    fn explicit_reservation_releases_on_drop_and_rejects_overflow() {
        let budget = SharedQueueBudget::new(10).unwrap();
        let reservation = budget.reserve(7).unwrap();
        assert_eq!(reservation.amount(), 7);
        assert_eq!(budget.used().unwrap(), 7);
        assert!(matches!(budget.reserve(4), Err(QueueError::ByteBudget)));
        drop(reservation);
        assert_eq!(budget.used().unwrap(), 0);
        assert_eq!(budget.reserve(10).unwrap().amount(), 10);
    }
}

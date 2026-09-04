use std::collections::{HashSet, VecDeque};
use std::sync::{Mutex, OnceLock};

const PENDING_DECODE_CAP: usize = 64;
const FAILED_DECODE_CAP: usize = 1024;

/// A queued decode: the image plus the longest edge (device px) the
/// current view actually needs. Decoding a 2048 px source for a node
/// drawn 40 px wide costs 16 MB of raster for 6 KB of visible pixels —
/// on an image-dense page zoomed out that alone thrashed the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingDecode {
    pub id: u64,
    pub max_edge_px: u32,
}

#[derive(Default)]
struct DecodeRegistry {
    pending: VecDeque<PendingDecode>,
    in_flight: HashSet<u64>,
    failed: HashSet<u64>,
    failed_order: VecDeque<u64>,
}

static DECODES: OnceLock<Mutex<DecodeRegistry>> = OnceLock::new();

fn decodes() -> &'static Mutex<DecodeRegistry> {
    DECODES.get_or_init(|| Mutex::new(DecodeRegistry::default()))
}

pub fn note_pending_decode(id: u64, max_edge_px: u32) {
    let Ok(mut registry) = decodes().lock() else {
        return;
    };
    if registry.failed.contains(&id) || registry.in_flight.contains(&id) {
        return;
    }
    // Already queued: keep the largest requested edge so one pass
    // satisfies every node sharing the image (a page can draw the same
    // asset as a thumbnail and as a hero).
    if let Some(queued) = registry.pending.iter_mut().find(|queued| queued.id == id) {
        queued.max_edge_px = queued.max_edge_px.max(max_edge_px);
        return;
    }
    if registry.pending.len() >= PENDING_DECODE_CAP {
        return;
    }
    registry
        .pending
        .push_back(PendingDecode { id, max_edge_px });
}

pub fn take_pending_decodes(max: usize) -> Vec<PendingDecode> {
    let Ok(mut registry) = decodes().lock() else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(max.min(registry.pending.len()));
    while out.len() < max {
        let Some(entry) = registry.pending.pop_front() else {
            break;
        };
        registry.in_flight.insert(entry.id);
        out.push(entry);
    }
    out
}

pub fn mark_decode_done(id: u64) {
    if let Ok(mut registry) = decodes().lock() {
        registry.in_flight.remove(&id);
    }
}

pub fn mark_decode_failed(id: u64) {
    let Ok(mut registry) = decodes().lock() else {
        return;
    };
    registry.pending.retain(|queued| queued.id != id);
    registry.in_flight.remove(&id);
    if registry.failed.insert(id) {
        registry.failed_order.push_back(id);
        while registry.failed.len() > FAILED_DECODE_CAP {
            let Some(oldest) = registry.failed_order.pop_front() else {
                break;
            };
            registry.failed.remove(&oldest);
        }
    }
}

/// Whether another consumer is currently decoding taken entries. While an id
/// sits in flight, paint will NOT re-record a miss for it, so a headless
/// export that sees an empty queue cannot conclude its scene is fully
/// decoded — the entry may have been taken by a concurrent consumer that
/// installs into ITS raster cache, not the exporter's. See
/// `op-render-export/src/scene_painter.rs::ensure_images_decoded`.
pub fn has_in_flight_decodes() -> bool {
    decodes()
        .lock()
        .map(|registry| !registry.in_flight.is_empty())
        .unwrap_or(false)
}

pub fn has_pending_decodes() -> bool {
    decodes()
        .lock()
        .map(|registry| !registry.pending.is_empty())
        .unwrap_or(false)
}

/// Number of paint-recorded decode requests that have not yet been submitted.
pub fn pending_decode_count() -> usize {
    decodes()
        .lock()
        .map(|registry| registry.pending.len())
        .unwrap_or(0)
}

#[cfg(test)]
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub(crate) fn lock_decode_registry_for_tests() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    if let Ok(mut registry) = decodes().lock() {
        *registry = DecodeRegistry::default();
    }
    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taken_ids(taken: Vec<PendingDecode>) -> Vec<u64> {
        taken.into_iter().map(|entry| entry.id).collect()
    }

    #[test]
    fn pending_decode_queue_dedupes_and_tracks_in_flight_lifecycle() {
        let _guard = lock_decode_registry_for_tests();
        note_pending_decode(41, 256);
        note_pending_decode(41, 256);

        assert!(has_pending_decodes());
        assert_eq!(taken_ids(take_pending_decodes(8)), vec![41]);
        assert!(!has_pending_decodes());
        note_pending_decode(41, 256);
        assert!(
            take_pending_decodes(8).is_empty(),
            "in-flight ids stay deduped"
        );

        mark_decode_done(41);
        note_pending_decode(41, 256);
        assert_eq!(taken_ids(take_pending_decodes(8)), vec![41]);
    }

    #[test]
    fn failed_decode_is_removed_and_never_requeued() {
        let _guard = lock_decode_registry_for_tests();
        note_pending_decode(42, 256);
        assert_eq!(taken_ids(take_pending_decodes(1)), vec![42]);

        mark_decode_failed(42);
        note_pending_decode(42, 256);

        assert!(!has_pending_decodes());
        assert!(take_pending_decodes(1).is_empty());

        // Failure is terminal even when reported before a queued id is taken.
        note_pending_decode(43, 256);
        mark_decode_failed(43);
        assert!(take_pending_decodes(1).is_empty());
    }

    #[test]
    fn failed_decode_cache_is_bounded_and_evicts_oldest() {
        let _guard = lock_decode_registry_for_tests();
        for id in 0..=(FAILED_DECODE_CAP as u64) {
            mark_decode_failed(id);
        }

        note_pending_decode(0, 256);
        note_pending_decode(FAILED_DECODE_CAP as u64, 256);

        assert_eq!(taken_ids(take_pending_decodes(2)), vec![0]);
    }

    #[test]
    fn pending_decode_queue_is_bounded() {
        let _guard = lock_decode_registry_for_tests();
        for id in 0..(PENDING_DECODE_CAP as u64 + 10) {
            note_pending_decode(id, 256);
        }
        assert_eq!(take_pending_decodes(usize::MAX).len(), PENDING_DECODE_CAP);
    }

    #[test]
    fn pending_decode_count_tracks_only_queued_work() {
        let _guard = lock_decode_registry_for_tests();
        note_pending_decode(51, 256);
        note_pending_decode(52, 256);
        assert_eq!(pending_decode_count(), 2);

        assert_eq!(taken_ids(take_pending_decodes(1)), vec![51]);
        assert_eq!(pending_decode_count(), 1, "in-flight work is not pending");
        mark_decode_failed(52);
        assert_eq!(pending_decode_count(), 0);
    }
}

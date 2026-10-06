//! Runtime-loaded product assets for the browser bundle.
//!
//! The desktop binary embeds its product assets with `include_bytes!` /
//! `include_str!` — it is already a local file, so a few megabytes of preview
//! JPEGs and template documents cost nothing at run time. The browser bundle
//! cannot afford the same trade: every embedded byte is a byte the user
//! downloads before the editor paints its first frame.
//!
//! So on `wasm32` those assets are left out of the binary and fetched from the
//! daemon on demand. This module is the platform-free half of that: a
//! process-global registry of "asset route → bytes", plus the single-flight
//! bookkeeping that stops N cards on screen from firing N requests for the
//! same file. The host supplies the transport; nothing here knows about XHR,
//! and it is therefore testable without a DOM.
//!
//! ## Why the registry hands out `&'static` references
//!
//! Every consumer of these assets ultimately passes the bytes to a renderer
//! that caches them by id for the lifetime of the process (`store_remote_
//! image_bytes`, the icon catalog's `OnceLock`). Handing out `&'static` keeps
//! those call sites byte-identical between native and web — the native side
//! genuinely has a `&'static [u8]` from `include_bytes!`, and the web side
//! leaks each fetched asset once. The leak is bounded by the shipped asset
//! count (a fixed catalogue, not user input) and each asset installs at most
//! once, so this is a one-time cost, not a growth path.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// URL prefix the daemon serves runtime assets from.
///
/// `/pkg/` and not `/assets/`: the daemon already routes `/pkg/*` into the
/// resolved web-bundle directory and the production gateway already forwards
/// it, while `/assets/` belongs to the hub's own frontend. Assets are copied
/// into `pkg/assets/` by the bundle build (see `tools/check-wasm-bundle.sh`).
pub const WEB_ASSET_ROUTE_PREFIX: &str = "/pkg/assets/";

/// Where an asset is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebAssetState {
    /// Never asked for. The next [`begin_fetch`] owns the request.
    Absent,
    /// A fetch is in flight. Callers paint their placeholder and wait.
    Pending,
    /// Installed and available from [`installed_bytes`] / [`installed_str`].
    Ready,
    /// The fetch failed. The asset stays unavailable and its feature degrades;
    /// [`begin_fetch`] will hand out the request again so a later user action
    /// can retry rather than being stuck forever on one bad response.
    Failed,
}

#[derive(Default)]
struct Registry {
    bytes: HashMap<String, &'static [u8]>,
    state: HashMap<String, WebAssetState>,
    /// Routes claimed by [`request`] and not yet handed to the host.
    ///
    /// The widget layer is platform-free and cannot fetch anything, so it
    /// notes what it needs here and the host drains it — the same
    /// widget-records / host-drains channel the image decode queue already
    /// uses (`image_runtime::note_pending_decode`).
    pending: Vec<String>,
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

fn lock() -> std::sync::MutexGuard<'static, Registry> {
    // A poisoned registry is still readable: the data is append-only and a
    // panicking installer cannot leave a half-written entry (the insert is one
    // statement). Refusing to serve assets after an unrelated panic would turn
    // a cosmetic failure into a blank editor.
    registry()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

/// Where `route` is in its lifecycle.
pub fn state(route: &str) -> WebAssetState {
    lock()
        .state
        .get(route)
        .copied()
        .unwrap_or(WebAssetState::Absent)
}

/// Claim the right to fetch `route`.
///
/// `true` means this caller owns the request; `false` means it is already in
/// flight or already answered. This is the single-flight gate: a panel with
/// forty preview cards calls it on every card of every frame and exactly one
/// request goes out.
pub fn begin_fetch(route: &str) -> bool {
    let mut registry = lock();
    match registry.state.get(route) {
        Some(WebAssetState::Pending | WebAssetState::Ready) => false,
        // `Absent` and `Failed` both hand out the request. Retrying a failure
        // is deliberate: the daemon may simply not have been up yet, and the
        // alternative is a permanently blank card with no way back short of a
        // page reload.
        _ => {
            registry
                .state
                .insert(route.to_string(), WebAssetState::Pending);
            true
        }
    }
}

/// Install fetched bytes. Returns `true` when this call is what made the asset
/// available (a second install for the same route is ignored, so a duplicate
/// response cannot swap the bytes under a renderer that already cached them).
pub fn install(route: &str, bytes: Vec<u8>) -> bool {
    let mut registry = lock();
    if registry.bytes.contains_key(route) {
        registry
            .state
            .insert(route.to_string(), WebAssetState::Ready);
        return false;
    }
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    registry.bytes.insert(route.to_string(), leaked);
    registry
        .state
        .insert(route.to_string(), WebAssetState::Ready);
    true
}

/// Record that the fetch for `route` failed.
///
/// The asset stays unavailable; its feature degrades to a placeholder, an
/// empty state, or a notice — never a panic and never a hang.
pub fn mark_failed(route: &str) {
    let mut registry = lock();
    if registry.bytes.contains_key(route) {
        // A late failure for an asset that already landed changes nothing.
        return;
    }
    registry
        .state
        .insert(route.to_string(), WebAssetState::Failed);
}

/// Ask the host to fetch `route`, if nobody already is.
///
/// Safe to call from paint: it is the single-flight gate plus an enqueue, so a
/// grid of forty cards repainting every frame produces one request per asset.
pub fn request(route: &str) {
    if !begin_fetch(route) {
        return;
    }
    lock().pending.push(route.to_string());
}

/// Take up to `max` routes the widget layer asked for.
///
/// The host owns them from here: it must answer every one with [`install`] or
/// [`mark_failed`], or the asset stays `Pending` forever and its card never
/// stops showing a placeholder.
pub fn take_pending_requests(max: usize) -> Vec<String> {
    let mut registry = lock();
    let take = max.min(registry.pending.len());
    registry.pending.drain(..take).collect()
}

/// Take one exact queued route without disturbing the FIFO order of others.
///
/// Mount-time locale prefetch uses this when a remount finds that a picker from
/// the previous shell already queued the same route but no frame has drained it
/// yet. The asset remains in [`WebAssetState::Pending`]; the caller becomes the
/// host responsible for answering it with [`install`] or [`mark_failed`].
pub fn take_pending_request(route: &str) -> bool {
    let mut registry = lock();
    let Some(index) = registry.pending.iter().position(|pending| pending == route) else {
        return false;
    };
    registry.pending.remove(index);
    true
}

/// Whether any route is waiting for the host to pick it up.
pub fn has_pending_requests() -> bool {
    !lock().pending.is_empty()
}

/// The installed bytes for `route`, or `None` while it is absent, pending or
/// failed.
pub fn installed_bytes(route: &str) -> Option<&'static [u8]> {
    lock().bytes.get(route).copied()
}

/// [`installed_bytes`] decoded as UTF-8, for the text assets (template
/// documents, the icon catalog).
///
/// A non-UTF-8 body reads as "not available" rather than panicking: these are
/// files served over HTTP, so a truncated or misrouted response is a runtime
/// possibility, not an invariant violation.
pub fn installed_str(route: &str) -> Option<&'static str> {
    std::str::from_utf8(installed_bytes(route)?).ok()
}

/// Drop everything, for tests that need a clean registry.
#[cfg(test)]
pub(crate) fn reset_for_test() {
    let mut registry = lock();
    registry.bytes.clear();
    registry.state.clear();
    registry.pending.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialises the tests, which share the process-global registry.
    fn lock_registry() -> std::sync::MutexGuard<'static, ()> {
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        TEST_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    #[test]
    fn an_unknown_route_is_absent_and_serves_nothing() {
        let _guard = lock_registry();
        reset_for_test();
        assert_eq!(state("/pkg/assets/none.jpg"), WebAssetState::Absent);
        assert!(installed_bytes("/pkg/assets/none.jpg").is_none());
        assert!(installed_str("/pkg/assets/none.jpg").is_none());
    }

    #[test]
    fn only_the_first_caller_owns_the_fetch() {
        // The single-flight property: a panel repaints its whole card grid
        // every frame, so without this each frame would fire a fresh request
        // for every card still waiting.
        let _guard = lock_registry();
        reset_for_test();
        let route = "/pkg/assets/single-flight.jpg";

        assert!(begin_fetch(route), "the first caller owns it");
        assert_eq!(state(route), WebAssetState::Pending);
        for _ in 0..10 {
            assert!(!begin_fetch(route), "no second request while in flight");
        }
    }

    #[test]
    fn a_successful_fetch_installs_once_and_stays_readable() {
        let _guard = lock_registry();
        reset_for_test();
        let route = "/pkg/assets/ok.jpg";

        assert!(begin_fetch(route));
        assert!(install(route, vec![1, 2, 3]));
        assert_eq!(state(route), WebAssetState::Ready);
        assert_eq!(installed_bytes(route), Some(&[1u8, 2, 3][..]));

        // A duplicate response must not swap bytes a renderer already cached.
        assert!(!install(route, vec![9, 9, 9]));
        assert_eq!(installed_bytes(route), Some(&[1u8, 2, 3][..]));
        // And nothing re-fetches something already in hand.
        assert!(!begin_fetch(route));
    }

    #[test]
    fn a_failed_fetch_degrades_and_stays_retryable() {
        let _guard = lock_registry();
        reset_for_test();
        let route = "/pkg/assets/bad.jpg";

        assert!(begin_fetch(route));
        mark_failed(route);
        assert_eq!(state(route), WebAssetState::Failed);
        assert!(
            installed_bytes(route).is_none(),
            "a failed asset must serve nothing, not stale or empty bytes"
        );
        assert!(
            begin_fetch(route),
            "a failure must be retryable — the daemon may just not have been up"
        );
        assert_eq!(state(route), WebAssetState::Pending);
    }

    #[test]
    fn a_late_failure_never_unloads_an_asset_that_landed() {
        // Two requests can be outstanding across a retry; the loser must not
        // pull the rug out from under the winner.
        let _guard = lock_registry();
        reset_for_test();
        let route = "/pkg/assets/race.jpg";

        assert!(begin_fetch(route));
        assert!(install(route, vec![7]));
        mark_failed(route);

        assert_eq!(state(route), WebAssetState::Ready);
        assert_eq!(installed_bytes(route), Some(&[7u8][..]));
    }

    #[test]
    fn a_request_is_enqueued_once_and_drained_by_the_host() {
        let _guard = lock_registry();
        reset_for_test();
        let route = "/pkg/assets/queued.jpg";

        for _ in 0..5 {
            request(route);
        }
        assert!(has_pending_requests());
        assert_eq!(take_pending_requests(10), vec![route.to_string()]);
        assert!(
            !has_pending_requests(),
            "a drained request must not be handed out twice"
        );

        // Still pending as far as the state machine is concerned: the host owes
        // an install or a failure.
        assert_eq!(state(route), WebAssetState::Pending);
        request(route);
        assert!(
            !has_pending_requests(),
            "an in-flight asset must not be re-enqueued behind the host's back"
        );
    }

    #[test]
    fn an_exact_take_preserves_the_other_pending_routes_order() {
        let _guard = lock_registry();
        reset_for_test();
        for route in ["/pkg/assets/a", "/pkg/assets/locale", "/pkg/assets/b"] {
            request(route);
        }

        assert!(take_pending_request("/pkg/assets/locale"));
        assert!(!take_pending_request("/pkg/assets/missing"));
        assert_eq!(
            take_pending_requests(usize::MAX),
            ["/pkg/assets/a", "/pkg/assets/b"]
        );
        for route in ["/pkg/assets/a", "/pkg/assets/locale", "/pkg/assets/b"] {
            mark_failed(route);
        }
    }

    #[test]
    fn the_host_drain_is_bounded_so_one_frame_cannot_fire_every_request() {
        let _guard = lock_registry();
        reset_for_test();
        for index in 0..10 {
            request(&format!("/pkg/assets/bounded-{index}.jpg"));
        }
        assert_eq!(take_pending_requests(3).len(), 3);
        assert_eq!(take_pending_requests(usize::MAX).len(), 7);
        assert!(!has_pending_requests());
    }

    #[test]
    fn a_failed_request_can_be_enqueued_again() {
        let _guard = lock_registry();
        reset_for_test();
        let route = "/pkg/assets/retry.jpg";

        request(route);
        let _ = take_pending_requests(usize::MAX);
        mark_failed(route);

        request(route);
        assert_eq!(take_pending_requests(usize::MAX), vec![route.to_string()]);
    }

    #[test]
    fn a_text_asset_round_trips_and_rejects_invalid_utf8() {
        let _guard = lock_registry();
        reset_for_test();

        assert!(install(
            "/pkg/assets/doc.op",
            b"{\"version\":\"1.0.0\"}".to_vec()
        ));
        assert_eq!(
            installed_str("/pkg/assets/doc.op"),
            Some("{\"version\":\"1.0.0\"}")
        );

        // A truncated or misrouted response is a runtime possibility, so it
        // reads as unavailable rather than panicking.
        assert!(install("/pkg/assets/broken.op", vec![0xff, 0xfe]));
        assert!(installed_str("/pkg/assets/broken.op").is_none());
    }
}

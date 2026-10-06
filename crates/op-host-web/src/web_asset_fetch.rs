//! Browser transport for the runtime-fetched product assets.
//!
//! The widget layer is platform-free: when a card needs a preview that is not
//! in the bundle it calls `op_editor_core::web_assets::request(route)` and
//! paints its placeholder. This module is the other half — it drains those
//! requests once per frame, fetches each over XHR, and installs the bytes back
//! into the registry so the next paint finds them.
//!
//! Three properties matter, and all three are the registry's, not this file's:
//! single-flight (a forty-card grid produces forty requests, not forty per
//! frame), exactly-one-answer (every drained route gets `install` or
//! `mark_failed`, so nothing stays `Pending` forever), and graceful failure
//! (an unavailable asset degrades to a placeholder — never a panic, never a
//! spinner that outlives the session).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use js_sys::Promise;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::JsValue;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// How many assets may be in flight from one drain.
///
/// The Prompt Center opens with dozens of cards visible; firing every request
/// at once buries the daemon's connection pool behind a burst nobody is
/// looking at yet. A small batch per frame keeps the visible rows filling in
/// first, and the queue is drained again next frame.
const MAX_IN_FLIGHT_PER_DRAIN: usize = 6;

/// Abandon a request after this long. A hung asset must not hold a route in
/// `Pending` forever — that would leave its card on a placeholder with no
/// retry.
const FETCH_TIMEOUT_MS: u32 = 20_000;

/// First-frame locale loading has a tighter contract than background product
/// assets: it may delay mount for at most three seconds before English fallback.
const INITIAL_LOCALE_FETCH_TIMEOUT_MS: u32 = 3_000;

type AssetWaiter = Box<dyn FnOnce(bool)>;

thread_local! {
    /// Active asset XHRs by registry route. Used only to join/abort an initial
    /// locale request across a supported shell remount.
    static IN_FLIGHT: RefCell<HashMap<String, web_sys::XmlHttpRequest>> =
        RefCell::new(HashMap::new());
    /// Mount futures waiting for a request the previous shell already started.
    static WAITERS: RefCell<HashMap<String, Vec<AssetWaiter>>> =
        RefCell::new(HashMap::new());
}

/// Why an asset fetch did not produce bytes.
///
/// Typed rather than a string because each variant is a different operational
/// story: no XHR at all is a hostile embedding, a non-2xx is a bundle that was
/// deployed without its assets, and a timeout is a slow link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebAssetFetchError {
    /// `XMLHttpRequest` could not be constructed.
    XhrUnavailable,
    /// `open()` was rejected — a malformed route.
    RequestOpenFailed,
    /// `send()` was rejected.
    RequestSendFailed,
    /// The response arrived with a non-2xx status (0 = network / timeout).
    Http(u16),
    /// A 2xx with no readable body.
    EmptyBody,
}

impl std::fmt::Display for WebAssetFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::XhrUnavailable => write!(f, "XMLHttpRequest is unavailable"),
            Self::RequestOpenFailed => write!(f, "could not open the asset request"),
            Self::RequestSendFailed => write!(f, "could not send the asset request"),
            Self::Http(status) => write!(f, "asset request failed with status {status}"),
            Self::EmptyBody => write!(f, "asset response carried no body"),
        }
    }
}

/// Drain whatever the widget layer asked for and fetch it.
///
/// Called once per paint. Cheap when idle: one lock and a length test. It
/// takes no host handle on purpose — the install path wakes the editor through
/// `repaint_coalescer`, the same free-function seam the agent-indicator relay
/// uses, so this module never has to borrow a host that a DOM event may
/// already hold.
pub(crate) fn drain_pending() {
    for route in op_editor_core::web_assets::take_pending_requests(MAX_IN_FLIGHT_PER_DRAIN) {
        fetch_asset(route);
    }
    if op_editor_core::web_assets::has_pending_requests() {
        // A locale can sit behind preview/icon requests in the shared FIFO. Arm
        // another frame now, before those requests settle, so six failures can
        // never strand the seventh route in Pending forever.
        crate::repaint_coalescer::request();
    }
}

/// Start the synchronously-known mount locale before CanvasKit initialization.
///
/// This claims the same registry state as the per-frame queue but starts its XHR
/// immediately: merely enqueueing here would deadlock because the first queue
/// drain happens inside the repaint that is waiting for this catalog. The
/// returned future settles on success, validation failure, HTTP failure, or the
/// dedicated three-second XHR timeout.
pub(crate) fn prefetch_initial_catalog(locale: op_i18n::Locale) -> Option<JsFuture> {
    if op_i18n::catalog_ready(locale) {
        return None;
    }
    let route = op_i18n::catalog_route(locale);
    if op_editor_core::web_assets::begin_fetch(&route)
        || op_editor_core::web_assets::take_pending_request(&route)
    {
        return Some(start_initial_catalog_fetch(route));
    }
    (op_editor_core::web_assets::state(&route)
        == op_editor_core::web_assets::WebAssetState::Pending)
        .then(|| join_initial_catalog_fetch(route))
}

fn start_initial_catalog_fetch(route: String) -> JsFuture {
    let promise = Promise::new(&mut move |resolve, _reject| {
        let resolve = resolve.clone();
        fetch_asset_with_timeout(
            route.clone(),
            INITIAL_LOCALE_FETCH_TIMEOUT_MS,
            move |ready| {
                let _ = resolve.call1(&JsValue::NULL, &JsValue::from_bool(ready));
            },
        );
    });
    JsFuture::from(promise)
}

fn join_initial_catalog_fetch(route: String) -> JsFuture {
    let promise = Promise::new(&mut move |resolve, _reject| {
        let settled = Rc::new(Cell::new(false));
        let settled_by_response = settled.clone();
        let resolve_response = resolve.clone();
        register_waiter(
            &route,
            Box::new(move |ready| {
                if !settled_by_response.replace(true) {
                    let _ = resolve_response.call1(&JsValue::NULL, &JsValue::from_bool(ready));
                }
            }),
        );

        let route_timeout = route.clone();
        let settled_by_timeout = settled.clone();
        let resolve_timeout = resolve.clone();
        let timeout = Rc::new(move || {
            if settled_by_timeout.replace(true) {
                return;
            }
            op_editor_core::web_assets::mark_failed(&route_timeout);
            abort_in_flight(&route_timeout);
            resolve_waiters(&route_timeout, false);
            let _ = resolve_timeout.call1(&JsValue::NULL, &JsValue::from_bool(false));
        });
        let Some(window) = web_sys::window() else {
            timeout();
            return;
        };
        let timeout_callback = timeout.clone();
        let callback = Closure::<dyn FnMut()>::once_into_js(move || timeout_callback());
        if window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.unchecked_ref(),
                INITIAL_LOCALE_FETCH_TIMEOUT_MS as i32,
            )
            .is_err()
        {
            timeout();
        }
    });
    JsFuture::from(promise)
}

fn register_waiter(route: &str, waiter: AssetWaiter) {
    WAITERS.with(|waiters| {
        waiters
            .borrow_mut()
            .entry(route.to_string())
            .or_default()
            .push(waiter);
    });
}

fn resolve_waiters(route: &str, ready: bool) {
    let waiters = WAITERS.with(|waiters| waiters.borrow_mut().remove(route));
    if let Some(waiters) = waiters {
        for waiter in waiters {
            waiter(ready);
        }
    }
}

fn remember_in_flight(route: String, xhr: web_sys::XmlHttpRequest) {
    IN_FLIGHT.with(|requests| {
        requests.borrow_mut().insert(route, xhr);
    });
}

fn forget_in_flight(route: &str) {
    IN_FLIGHT.with(|requests| {
        requests.borrow_mut().remove(route);
    });
}

fn abort_in_flight(route: &str) {
    let xhr = IN_FLIGHT.with(|requests| requests.borrow_mut().remove(route));
    if let Some(xhr) = xhr {
        let _ = xhr.abort();
    }
}

/// Promote or clear the current runtime locale request before a frame paints.
///
/// Fetch callbacks intentionally never borrow the host. They install global
/// catalog/asset state and request a frame; that frame reconciles only the
/// *current* pending target, so a stale response can never win after the user
/// selects a different locale.
pub(crate) fn reconcile_pending_locale(
    ui: &mut op_editor_core::editor_ui_state::EditorUiState,
) -> bool {
    let persistence_ready = ui
        .locale_persistence_override
        .is_some_and(op_i18n::catalog_ready);
    if ui.pending_locale.is_none() {
        return promote_persistence_override(ui, persistence_ready);
    }
    let Some(locale) = ui.pending_locale else {
        return false;
    };
    let ready = op_i18n::catalog_ready(locale);
    let failed = op_editor_core::web_assets::state(&op_i18n::catalog_route(locale))
        == op_editor_core::web_assets::WebAssetState::Failed;
    settle_pending_locale(ui, ready, failed, persistence_ready)
}

fn promote_persistence_override(
    ui: &mut op_editor_core::editor_ui_state::EditorUiState,
    ready: bool,
) -> bool {
    let Some(locale) = ui.locale_persistence_override else {
        return false;
    };
    if !ready {
        return false;
    }
    ui.set_locale_when_catalog_ready(locale, true);
    true
}

fn settle_pending_locale(
    ui: &mut op_editor_core::editor_ui_state::EditorUiState,
    ready: bool,
    failed: bool,
    persistence_ready: bool,
) -> bool {
    let Some(locale) = ui.pending_locale else {
        return false;
    };
    if ready {
        ui.set_locale_when_catalog_ready(locale, true);
        return true;
    }
    if failed {
        ui.pending_locale = None;
        if promote_persistence_override(ui, persistence_ready) {
            return true;
        }
    }
    false
}

fn fetch_asset(route: String) {
    fetch_asset_with_timeout(route, FETCH_TIMEOUT_MS, |_| {});
}

fn fetch_asset_with_timeout(
    route: String,
    timeout_ms: u32,
    on_settled: impl FnOnce(bool) + 'static,
) {
    let url = crate::daemon_base::daemon_url(&route);
    let route_for_result = route.clone();
    let xhr = fetch_bytes_with_timeout(&url, timeout_ms, move |result| {
        forget_in_flight(&route_for_result);
        let is_locale = locale_for_catalog_route(&route_for_result).is_some();
        let ready = match result {
            Ok(bytes) => install_fetched_asset(&route_for_result, bytes),
            Err(_error) => {
                // Degrade, do not retry in place: `mark_failed` leaves the route
                // retryable, so a later picker press or reload can ask again.
                op_editor_core::web_assets::mark_failed(&route_for_result);
                false
            }
        };
        // Locale success promotes pending state on the next paint; locale
        // failure clears it without changing the painted language. Both edges
        // therefore need a repaint. Other assets keep the existing success-only
        // wakeup because their failed placeholder is already on screen.
        if ready || is_locale {
            crate::repaint_coalescer::request();
        }
        resolve_waiters(&route_for_result, ready);
        on_settled(ready);
    });
    if let Some(xhr) = xhr {
        remember_in_flight(route, xhr);
    }
}

fn install_fetched_asset(route: &str, bytes: Vec<u8>) -> bool {
    if let Some(locale) = locale_for_catalog_route(route) {
        let Ok(entries) = serde_json::from_slice::<HashMap<String, String>>(&bytes) else {
            op_editor_core::web_assets::mark_failed(route);
            return false;
        };
        // Validate and install the parsed catalog BEFORE the raw registry bytes
        // become Ready. Once raw bytes are installed, `mark_failed` deliberately
        // refuses to unload them, which would make malformed JSON unretryable.
        if !op_i18n::install_catalog(locale, entries) {
            op_editor_core::web_assets::mark_failed(route);
            return false;
        }
    }

    if !op_editor_core::web_assets::install(route, bytes) {
        return false;
    }
    // The icon catalog is not consumed as raw bytes: it has to be parsed into
    // the shared catalog before any lookup can see it. Done here, once, on the
    // install edge.
    if route == op_editor_ui::ICONIFY_CORE_ROUTE {
        if let Some(json) = op_editor_core::web_assets::installed_str(route) {
            op_editor_ui::set_core_catalog(json);
        }
    }
    true
}

fn locale_for_catalog_route(route: &str) -> Option<op_i18n::Locale> {
    let code = route
        .strip_prefix("/pkg/assets/i18n/")?
        .strip_suffix(".json")?;
    let locale = op_i18n::Locale::from_tag(code)?;
    (locale.code() == code).then_some(locale)
}

type DoneFn = Box<dyn FnOnce(Result<Vec<u8>, WebAssetFetchError>)>;

/// Fire a GET for binary content and hand the body (or an error) to `on_done`
/// exactly once.
///
/// The callback is slot-wrapped so the synchronous failure paths still resolve
/// it — a dropped callback would strand its route in `Pending`, which is the
/// one state the registry cannot recover from on its own.
pub(crate) fn fetch_bytes(
    url: &str,
    on_done: impl FnOnce(Result<Vec<u8>, WebAssetFetchError>) + 'static,
) {
    let _ = fetch_bytes_with_timeout(url, FETCH_TIMEOUT_MS, on_done);
}

fn fetch_bytes_with_timeout(
    url: &str,
    timeout_ms: u32,
    on_done: impl FnOnce(Result<Vec<u8>, WebAssetFetchError>) + 'static,
) -> Option<web_sys::XmlHttpRequest> {
    let slot: Rc<RefCell<Option<DoneFn>>> = Rc::new(RefCell::new(Some(Box::new(on_done))));
    let resolve = |slot: &Rc<RefCell<Option<DoneFn>>>, result| {
        if let Some(done) = slot.borrow_mut().take() {
            done(result);
        }
    };
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
        resolve(&slot, Err(WebAssetFetchError::XhrUnavailable));
        return None;
    };
    if xhr.open_with_async("GET", url, true).is_err() {
        resolve(&slot, Err(WebAssetFetchError::RequestOpenFailed));
        return None;
    }
    // These are JPEGs, `.op` documents, and UTF-8 JSON catalogs. ArrayBuffer is
    // the one response mode that preserves every binary/text body byte-exactly.
    xhr.set_response_type(web_sys::XmlHttpRequestResponseType::Arraybuffer);
    // The assets live under the daemon's `/pkg/` route, so in managed mode the
    // bridge token has to ride along exactly as it does for every other daemon
    // call. `attach_daemon_headers` decides that from the URL.
    crate::live_sync::attach_daemon_headers(&xhr, url);
    xhr.set_timeout(timeout_ms);
    let xhr_cb = xhr.clone();
    let slot_cb = slot.clone();
    let onloadend = Closure::<dyn FnMut()>::once_into_js(move || {
        let status = xhr_cb.status().unwrap_or(0);
        let result = if (200..300).contains(&status) {
            match xhr_cb.response() {
                Ok(value) if !value.is_null() && !value.is_undefined() => {
                    let buffer = js_sys::Uint8Array::new(&value);
                    let mut bytes = vec![0u8; buffer.length() as usize];
                    buffer.copy_to(&mut bytes);
                    if bytes.is_empty() {
                        Err(WebAssetFetchError::EmptyBody)
                    } else {
                        Ok(bytes)
                    }
                }
                _ => Err(WebAssetFetchError::EmptyBody),
            }
        } else {
            Err(WebAssetFetchError::Http(status))
        };
        if let Some(done) = slot_cb.borrow_mut().take() {
            done(result);
        }
    });
    xhr.set_onloadend(Some(onloadend.unchecked_ref()));
    if xhr.send().is_err() {
        resolve(&slot, Err(WebAssetFetchError::RequestSendFailed));
        return None;
    }
    Some(xhr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::web_assets::{self, WebAssetState};

    /// Serialises against the process-global asset registry.
    fn lock_registry() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    #[test]
    fn every_failure_variant_reads_as_its_own_operational_story() {
        // Each is a different thing to go and fix, which is the whole reason
        // this is an enum rather than a bool.
        let messages = [
            WebAssetFetchError::XhrUnavailable.to_string(),
            WebAssetFetchError::RequestOpenFailed.to_string(),
            WebAssetFetchError::RequestSendFailed.to_string(),
            WebAssetFetchError::Http(404).to_string(),
            WebAssetFetchError::EmptyBody.to_string(),
        ];
        let unique: std::collections::HashSet<_> = messages.iter().collect();
        assert_eq!(unique.len(), messages.len());
        assert!(messages[3].contains("404"));
    }

    #[test]
    fn a_drain_is_bounded_so_one_frame_cannot_open_every_socket() {
        // The bound is what keeps a freshly opened Prompt Center from firing
        // 57 sockets in a single frame.
        let _guard = lock_registry();
        for index in 0..(MAX_IN_FLIGHT_PER_DRAIN + 4) {
            web_assets::request(&format!("/pkg/assets/drain-bound-{index}.jpg"));
        }
        assert_eq!(
            web_assets::take_pending_requests(MAX_IN_FLIGHT_PER_DRAIN).len(),
            MAX_IN_FLIGHT_PER_DRAIN
        );
        assert!(web_assets::has_pending_requests(), "the rest wait a frame");
        // Leave the shared registry clean for other tests.
        for route in web_assets::take_pending_requests(usize::MAX) {
            web_assets::mark_failed(&route);
        }
        for index in 0..MAX_IN_FLIGHT_PER_DRAIN {
            web_assets::mark_failed(&format!("/pkg/assets/drain-bound-{index}.jpg"));
        }
    }

    #[test]
    fn a_failed_asset_degrades_and_can_be_asked_for_again() {
        // This is the contract the paint sites depend on: a failure must leave
        // the card on its placeholder AND leave the door open, never wedge the
        // route in `Pending`.
        let _guard = lock_registry();
        let route = "/pkg/assets/web-asset-fetch-failure.jpg";

        web_assets::request(route);
        let drained = web_assets::take_pending_requests(usize::MAX);
        assert!(drained.iter().any(|r| r == route));

        web_assets::mark_failed(route);
        assert_eq!(web_assets::state(route), WebAssetState::Failed);
        assert!(web_assets::installed_bytes(route).is_none());

        web_assets::request(route);
        assert_eq!(web_assets::state(route), WebAssetState::Pending);
        for r in web_assets::take_pending_requests(usize::MAX) {
            web_assets::mark_failed(&r);
        }
    }

    #[test]
    fn locale_routes_accept_only_exact_staged_bcp47_names() {
        assert_eq!(
            locale_for_catalog_route("/pkg/assets/i18n/zh-TW.json"),
            Some(op_i18n::Locale::ZhTw)
        );
        assert_eq!(
            locale_for_catalog_route("/pkg/assets/i18n/ja.json"),
            Some(op_i18n::Locale::Ja)
        );
        assert_eq!(
            locale_for_catalog_route("/pkg/assets/i18n/zh_tw.json"),
            None
        );
        assert_eq!(locale_for_catalog_route("/assets/i18n/ja.json"), None);
    }

    #[test]
    fn an_existing_asset_flight_resolves_every_joiner_once() {
        let first = Rc::new(Cell::new(None));
        let second = Rc::new(Cell::new(None));
        let first_result = first.clone();
        let second_result = second.clone();
        let route = "/pkg/assets/i18n/remount-test.json";
        register_waiter(route, Box::new(move |ready| first_result.set(Some(ready))));
        register_waiter(route, Box::new(move |ready| second_result.set(Some(ready))));

        resolve_waiters(route, true);
        resolve_waiters(route, false);

        assert_eq!(first.get(), Some(true));
        assert_eq!(second.get(), Some(true));
    }

    #[test]
    fn invalid_locale_json_fails_before_raw_bytes_become_ready_and_can_retry() {
        let _guard = lock_registry();
        let route = "/pkg/assets/i18n/de.json";

        assert!(web_assets::begin_fetch(route));
        assert!(!install_fetched_asset(
            route,
            br#"["not-an-object"]"#.to_vec()
        ));
        assert_eq!(web_assets::state(route), WebAssetState::Failed);
        assert!(web_assets::installed_bytes(route).is_none());

        assert!(
            web_assets::begin_fetch(route),
            "invalid JSON stays retryable"
        );
        assert!(!install_fetched_asset(
            route,
            br#"{"not.an.i18n.key":"Nein"}"#.to_vec()
        ));
        assert_eq!(web_assets::state(route), WebAssetState::Failed);
        assert!(web_assets::installed_bytes(route).is_none());

        assert!(web_assets::begin_fetch(route));
        let valid = br#"{"common.cancel":"Abbrechen runtime"}"#.to_vec();
        assert!(install_fetched_asset(route, valid.clone()));
        assert_eq!(web_assets::state(route), WebAssetState::Ready);
        assert_eq!(web_assets::installed_bytes(route), Some(valid.as_slice()));
    }

    #[test]
    fn pending_locale_promotes_only_when_ready_and_failure_keeps_old_language() {
        let mut ui = op_editor_core::editor_ui_state::EditorUiState {
            pending_locale: Some(op_i18n::Locale::Ja),
            ..Default::default()
        };

        assert!(!settle_pending_locale(&mut ui, false, false, false));
        assert_eq!(ui.locale, op_i18n::Locale::ZhCn);
        assert_eq!(ui.pending_locale, Some(op_i18n::Locale::Ja));

        assert!(!settle_pending_locale(&mut ui, false, true, false));
        assert_eq!(ui.locale, op_i18n::Locale::ZhCn);
        assert_eq!(ui.pending_locale, None);

        ui.pending_locale = Some(op_i18n::Locale::Ja);
        ui.locale_persistence_override = Some(op_i18n::Locale::Ja);
        assert!(!settle_pending_locale(&mut ui, false, true, false));
        assert_eq!(ui.locale, op_i18n::Locale::ZhCn);
        assert_eq!(ui.pending_locale, None);
        assert_eq!(ui.locale_persistence_override, Some(op_i18n::Locale::Ja));

        ui.pending_locale = Some(op_i18n::Locale::De);
        assert!(settle_pending_locale(&mut ui, false, true, true));
        assert_eq!(ui.locale, op_i18n::Locale::Ja);
        assert_eq!(ui.pending_locale, None);
        assert_eq!(ui.locale_persistence_override, None);

        ui.pending_locale = Some(op_i18n::Locale::De);
        assert!(settle_pending_locale(&mut ui, true, false, false));
        assert_eq!(ui.locale, op_i18n::Locale::De);
        assert_eq!(ui.pending_locale, None);
    }
}

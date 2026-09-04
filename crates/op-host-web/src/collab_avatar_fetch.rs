//! Browser transport for verified collaboration-peer avatars.
//!
//! The shared widget layer registers each roster participant's avatar URL and,
//! when it paints a peer whose image is not yet cached, enqueues a fetch
//! request (`op_editor_ui::collab_avatar_runtime`). Desktop drains that queue
//! with `op-host-desktop`'s `CollabAvatarHost`, which fetches each URL on a
//! worker thread. The browser had no counterpart, so on web the queue filled
//! and was never drained — every peer avatar fell back to its initials while
//! the document, cursors and presence all synced fine.
//!
//! This is that counterpart, with one difference forced by the sandbox: a wasm
//! page under CSP cannot fetch `qlogo.cn` (or any avatar CDN) directly, so it
//! does NOT use the request's URL. It posts the participant key to the daemon's
//! `POST /api/collab/avatar` proxy, which performs the SSRF-guarded fetch and
//! returns base64 JPEG — the same proxy the desktop host could use but does
//! not need to.
//!
//! ## Self-account avatars are somebody else's job
//!
//! The locally authenticated account's own avatar is handled entirely by
//! `web_auth_sync`, which posts `/api/auth/avatar` and installs the bytes with
//! `install_account_avatar_bytes` — it never registers a URL, so it never
//! enqueues a fetch request here. This driver therefore only ever sees peer
//! requests. The `is_current_account()` guard below is belt-and-braces: were
//! such a request ever to appear, the driver declines it (completes with
//! `None`) rather than racing the auth path for the account slot.
//!
//! ## Failure is a fallback, not a retry storm
//!
//! A non-2xx response or a decode failure completes the request with `None`.
//! The runtime marks that peer's slot failed and the widget keeps painting the
//! initials fallback; it does not re-enqueue on the next paint, so one bad
//! response is one request, not a loop.

use std::cell::Cell;
use std::rc::Rc;

use op_editor_ui::collab_avatar_runtime::{
    complete_collab_avatar_request, take_collab_avatar_requests, CollabAvatarFetchRequest,
};

// Wake later frames through the coalescer rather than borrowing a host handle:
// `CkInner::repaint` is `&mut self` with no `Rc<RefCell<Self>>` to hand down,
// and this is the same seam `web_asset_fetch` uses for exactly that reason.

/// Peer avatar fetches allowed in flight at once, mirroring the desktop host's
/// `MAX_CONCURRENT_FETCHES`. A busy roster must not open a socket per peer per
/// frame; the queue is drained again next frame as slots free.
const MAX_CONCURRENT_FETCHES: usize = 3;

thread_local! {
    /// Requests dispatched and not yet resolved. The per-frame drain fills
    /// only `MAX_CONCURRENT_FETCHES - IN_FLIGHT` slots, so this is the bound.
    static IN_FLIGHT: Cell<usize> = const { Cell::new(0) };
}

/// Drain the peer-avatar queue and fetch what fits in the free worker slots.
///
/// Called once per paint from the CanvasKit frame loop. Cheap when idle: a
/// counter read and a bounded queue take.
pub(crate) fn drain_pending() {
    let free = MAX_CONCURRENT_FETCHES.saturating_sub(IN_FLIGHT.with(Cell::get));
    if free == 0 {
        return;
    }
    for request in take_collab_avatar_requests(free) {
        if request.is_current_account() {
            // Cannot happen on web (the auth path registers no URL), but if it
            // did, declining keeps this driver strictly peer-only.
            let _ = complete_collab_avatar_request(&request, None);
            continue;
        }
        dispatch(request);
    }
}

/// POST one participant key to the daemon proxy and resolve the request when
/// the response lands.
fn dispatch(request: CollabAvatarFetchRequest) {
    let url = format!(
        "{}{}",
        crate::daemon_base::daemon_base(),
        op_editor_core::collab_routes::AVATAR
    );
    let Ok(body) = serde_json::to_string(&AvatarProxyRequest {
        participant_key: request.participant_key(),
    }) else {
        // Serialising a single string field cannot realistically fail, but a
        // request that is never sent must still be completed or its slot stays
        // in flight forever.
        let _ = complete_collab_avatar_request(&request, None);
        return;
    };

    IN_FLIGHT.with(|count| count.set(count.get() + 1));
    // `post_json_with_status` reports only whether the request STARTED; the
    // request itself has to live in the callback to be resolved when the
    // response lands. So it moves into the closure, and the synchronous
    // "never started" recovery works from a clone made before the move.
    let recovery = request.clone();
    let started = crate::live_sync::post_json_with_status(
        &url,
        &body,
        Rc::new(move |status, response| {
            IN_FLIGHT.with(|count| count.set(count.get().saturating_sub(1)));
            if apply_response(&request, status, &response) {
                // The bytes are cached now, but nothing else will repaint the
                // roster — the response is not an input event.
                crate::repaint_coalescer::request();
            }
        }),
    );
    if !started {
        // The XHR never left the ground; its callback will not run, so undo the
        // in-flight bump and let the peer fall back to initials.
        IN_FLIGHT.with(|count| count.set(count.get().saturating_sub(1)));
        let _ = complete_collab_avatar_request(&recovery, None);
    }
}

/// Apply one proxy response to its request. Pure of the DOM, so the whole
/// success / failure state machine is testable without a browser.
///
/// Returns whether the request delivered candidate avatar bytes — i.e. a 200
/// whose base64 decoded — which is the caller's cue to repaint the roster.
/// `complete_collab_avatar_request` cannot be that cue: it returns `true` for
/// any *valid* completion, failures included, so keying the repaint on it would
/// wake the frame loop on every dropped avatar. A 200 with bytes the runtime
/// then rejects (wrong signature, oversized) still returns `true` here; the
/// repaint is a harmless no-op that repaints the initials, matching how the
/// desktop host wakes on every completed job.
pub(crate) fn apply_response(request: &CollabAvatarFetchRequest, status: u16, body: &str) -> bool {
    let bytes = (status == 200)
        .then(|| decode_encoded_avatar(body))
        .flatten();
    // Always complete the request — with `None` on any failure path (non-200,
    // malformed JSON, bad base64). The runtime marks the slot failed and the
    // widget keeps the initials; it does not re-enqueue, so this is one
    // request, not a storm.
    let delivered = bytes.is_some();
    let _ = complete_collab_avatar_request(request, bytes);
    delivered
}

/// Pull the base64 `encoded` field out of a proxy 200 body and decode it.
fn decode_encoded_avatar(body: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let encoded = parsed.get("encoded")?.as_str()?;
    base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .ok()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AvatarProxyRequest<'a> {
    participant_key: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use op_editor_ui::collab_avatar_runtime::{
        collab_avatar_image, register_collab_avatar_url, take_collab_avatar_requests,
    };
    use std::sync::Mutex;

    /// Serialises against the process-global avatar registry.
    static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

    fn drain_stale() {
        for request in take_collab_avatar_requests(usize::MAX) {
            let _ = complete_collab_avatar_request(&request, None);
        }
    }

    /// A minimal image the runtime's dimension validation accepts. The
    /// registry only reads the header, so a well-formed 16×16 PNG signature +
    /// IHDR is enough — the daemon proxy really returns JPEG, but the byte
    /// content is opaque to this driver, which only base64-decodes and forwards.
    fn image_bytes() -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes[8..12].copy_from_slice(&13_u32.to_be_bytes());
        bytes[12..16].copy_from_slice(b"IHDR");
        bytes[16..20].copy_from_slice(&16_u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&16_u32.to_be_bytes());
        bytes
    }

    fn queued_request(key: &str) -> CollabAvatarFetchRequest {
        // Registering a roster URL queues the fetch request the same way the
        // desktop host observes it. The runtime namespaces the key under
        // `collab:`, so the request's key is the prefixed form.
        assert!(register_collab_avatar_url(
            key,
            Some("https://cdn.example/peer.png")
        ));
        take_collab_avatar_requests(usize::MAX)
            .into_iter()
            .find(|request| request.participant_key().ends_with(key))
            .expect("the roster registration enqueues a fetch request")
    }

    fn proxy_body(key: &str, bytes: &[u8]) -> String {
        serde_json::json!({
            "participantKey": key,
            "revision": "rev-1",
            "encoded": base64::engine::general_purpose::STANDARD.encode(bytes),
        })
        .to_string()
    }

    #[test]
    fn the_participant_key_is_exposed_for_the_proxy_body() {
        // The getter this feature added: the exact string the daemon proxy
        // wants, even though `Debug` still redacts identity.
        let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drain_stale();
        let request = queued_request("collab-avatar-key-getter");
        assert!(request
            .participant_key()
            .ends_with("collab-avatar-key-getter"));
        let debug = format!("{request:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("collab-avatar-key-getter"));
        let _ = complete_collab_avatar_request(&request, None);
    }

    #[test]
    fn a_scripted_200_installs_the_peer_avatar() {
        let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drain_stale();
        let key = "collab-avatar-success";
        let request = queued_request(key);
        assert!(collab_avatar_image(key).is_none());

        assert!(
            apply_response(&request, 200, &proxy_body(key, &image_bytes())),
            "delivering decodable bytes must signal a repaint"
        );

        assert!(
            collab_avatar_image(key).is_some(),
            "a 200 with decodable bytes must cache the peer avatar"
        );
    }

    #[test]
    fn a_non_200_falls_back_without_caching() {
        let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drain_stale();
        let key = "collab-avatar-http-error";
        let request = queued_request(key);

        assert!(
            !apply_response(&request, 502, "{}"),
            "a proxy error delivers no bytes, so no repaint is owed"
        );

        assert!(
            collab_avatar_image(key).is_none(),
            "a proxy error must leave the peer on its initials fallback"
        );
        assert!(
            take_collab_avatar_requests(1).is_empty(),
            "a failure must not re-enqueue — one bad response is one request"
        );
    }

    #[test]
    fn a_200_whose_bytes_cannot_be_decoded_delivers_nothing() {
        // Malformed JSON and non-base64 both fail before any bytes exist, so
        // there is nothing to hand the runtime and nothing to repaint.
        let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drain_stale();

        let malformed = queued_request("collab-avatar-bad-json");
        assert!(!apply_response(&malformed, 200, "not json"));
        assert!(collab_avatar_image("collab-avatar-bad-json").is_none());

        let bad_b64 = queued_request("collab-avatar-bad-b64");
        assert!(!apply_response(
            &bad_b64,
            200,
            &serde_json::json!({ "encoded": "!!!not-base64!!!" }).to_string()
        ));
        assert!(collab_avatar_image("collab-avatar-bad-b64").is_none());
    }

    #[test]
    fn bytes_the_runtime_rejects_never_reach_the_cache() {
        // A 200 whose base64 decodes but is not a real image: the bytes are
        // delivered (so a repaint is signalled), but the runtime's own
        // signature/size validation refuses them, so the peer stays on its
        // initials. The repaint is a harmless no-op — the guarantee that
        // matters is that no junk lands in the avatar cache.
        let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drain_stale();

        let request = queued_request("collab-avatar-not-image");
        let _delivered = apply_response(
            &request,
            200,
            &proxy_body("collab-avatar-not-image", b"this is not a jpeg"),
        );
        assert!(
            collab_avatar_image("collab-avatar-not-image").is_none(),
            "bytes that fail the runtime's validation must not be cached"
        );
    }

    #[test]
    fn the_proxy_body_carries_the_participant_key_camel_cased() {
        // The daemon deserialises `#[serde(rename_all = "camelCase")]`, so the
        // field must be `participantKey`, matching `collab_avatar_proxy`.
        let json = serde_json::to_string(&AvatarProxyRequest {
            participant_key: "collab-peer-7",
        })
        .expect("serialises");
        assert_eq!(json, r#"{"participantKey":"collab-peer-7"}"#);
    }
}

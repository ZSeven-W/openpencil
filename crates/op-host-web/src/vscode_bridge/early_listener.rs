//! The bridge's early receive-and-buffer phase (dsh-openpencil #2).
//!
//! The host starts sending `op-bridge/init` the moment the iframe is created
//! and retries only ~20x over ~10 s, while `mount_ck` spends those first
//! seconds downloading + instantiating ~24 MB of wasm (the `op_host_web`
//! bundle + CanvasKit) before the full [`super::install`] can run (it needs
//! the built `inner`/`sync`). `postMessage` to a window without a listener
//! is silently DROPPED — never queued — so without this module a slow mount
//! can exhaust the host's retry burst and lose the token forever.
//!
//! The fix has two parts, both here:
//! * [`install_early`] registers a minimal window `message` listener at the
//!   very start of the mount and immediately announces `op-bridge/listening`,
//!   which tells the host "I can receive now, please (re)send init".
//! * Inbound bridge messages arriving before the full install are BUFFERED
//!   (with their receive-time origin) in [`EARLY_INBOX`]; [`take_over`] hands
//!   them to `install`, which replays them through the normal pipeline.
//!
//! The early listener applies the same source lock as the full pipeline (only
//! `window.parent` may drive the bridge) and buffers only valid bridge
//! messages, so the replay performs byte-for-byte the same handling the live
//! path does.

use std::cell::RefCell;

use js_sys::Object;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::MessageEvent;

use super::helpers::post_to_parent;
use op_editor_core::bridge_protocol::{event_listening, BridgeInbound};

thread_local! {
    /// The early-phase buffer, shared between the early listener (push) and
    /// `install`'s take-over (drain + replay).
    static EARLY_INBOX: RefCell<EarlyInbox> = const { RefCell::new(EarlyInbox::new()) };
}

/// Pure state machine for the early-phase handoff: buffer inbound bridge
/// messages (with their receive-time origin) until the full pipeline exists,
/// then hand them over exactly once.
struct EarlyInbox {
    /// `true` once the full bridge has taken over. Later pushes are ignored:
    /// they cannot happen (the full listener owns every message by then), but
    /// ignoring beats queueing into limbo.
    drained: bool,
    pending: Vec<(String, BridgeInbound)>,
}

impl EarlyInbox {
    const fn new() -> Self {
        Self {
            drained: false,
            pending: Vec::new(),
        }
    }

    fn is_drained(&self) -> bool {
        self.drained
    }

    fn push(&mut self, origin: String, msg: BridgeInbound) {
        if self.drained {
            return;
        }
        self.pending.push((origin, msg));
    }

    fn take_over(&mut self) -> Vec<(String, BridgeInbound)> {
        self.drained = true;
        std::mem::take(&mut self.pending)
    }
}

/// Register the bridge's window `message` listener as early as possible —
/// BEFORE the heavy backend download + first frame — and announce
/// `op-bridge/listening` so the host (re)sends `init`.
///
/// The listener only BUFFERS valid bridge messages (see module docs); the
/// full [`super::install`] later drains + replays them once the shell state
/// exists. The `listening` announcement is sent unconditionally: standalone
/// tabs have no host to hear it, and old hosts ignore the unknown type.
/// `pub(crate)` because the spine re-exports it for the mount call site.
pub(crate) fn install_early() {
    let Some(window) = web_sys::window() else {
        return;
    };
    {
        let closure = Closure::<dyn FnMut(MessageEvent)>::new(move |evt: MessageEvent| {
            handle_message_early(&evt);
        });
        let _ =
            window.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref());
        closure.forget(); // page-lifetime listener, deliberately leaked
    }
    // Listener is live: tell the host it may (re)send init NOW.
    post_to_parent(&event_listening());
}

/// Hand the full bridge control: the early listener no-ops from here on and
/// everything buffered is returned for `install`'s replay, receive-time
/// origins included. Runs synchronously inside `install`, so no message event
/// can interleave between the flag flip and the drain.
pub(super) fn take_over() -> Vec<(String, BridgeInbound)> {
    EARLY_INBOX.with(|inbox| inbox.borrow_mut().take_over())
}

/// The early listener's body: same source lock as the full pipeline, parse,
/// buffer. No editor state is touched — there is none yet.
fn handle_message_early(evt: &MessageEvent) {
    if EARLY_INBOX.with(|inbox| inbox.borrow().is_drained()) {
        return; // the full bridge is live; its listener handles everything
    }
    // Only the parent frame (the VS Code webview host) may drive the bridge.
    let parent = web_sys::window().and_then(|w| w.parent().ok().flatten());
    let source_ok = match (evt.source(), parent) {
        (Some(src), Some(par)) => Object::is(src.as_ref(), par.as_ref()),
        _ => false,
    };
    if !source_ok {
        return;
    }
    // Bridge messages are JSON strings; anything else (react-devtools
    // objects, etc.) is foreign traffic — silently ignored, never an error.
    let Some(raw) = evt.data().as_string() else {
        return;
    };
    let Some(msg) = BridgeInbound::parse(&raw) else {
        return; // non-bridge / malformed
    };
    // Buffer with the origin captured NOW: the replay applies the same
    // origin-lock rules against the receive-time origin later.
    EARLY_INBOX.with(|inbox| inbox.borrow_mut().push(evt.origin(), msg));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_inbox_buffers_in_order_until_take_over() {
        let mut inbox = EarlyInbox::new();
        assert!(!inbox.is_drained());
        inbox.push(
            "https://host.example".into(),
            BridgeInbound::Init {
                token: "first".into(),
                mcp_url: None,
            },
        );
        inbox.push(
            "https://host.example".into(),
            BridgeInbound::Init {
                token: "second".into(),
                mcp_url: Some("http://127.0.0.1:9/mcp".into()),
            },
        );

        let buffered = inbox.take_over();
        assert!(inbox.is_drained());
        assert_eq!(
            buffered,
            vec![
                (
                    "https://host.example".to_string(),
                    BridgeInbound::Init {
                        token: "first".into(),
                        mcp_url: None
                    }
                ),
                (
                    "https://host.example".to_string(),
                    BridgeInbound::Init {
                        token: "second".into(),
                        mcp_url: Some("http://127.0.0.1:9/mcp".into())
                    }
                ),
            ]
        );
        // Repeated `init` is handed over intact rather than deduplicated:
        // the replay is idempotent at the token layer (`handle_init` writes
        // unconditionally), and a host may legitimately resend init.
    }

    #[test]
    fn take_over_drains_once_and_late_pushes_never_linger() {
        let mut inbox = EarlyInbox::new();
        assert!(inbox.take_over().is_empty());
        assert!(inbox.is_drained());
        // A push racing after take-over is ignored instead of queueing into
        // limbo.
        inbox.push(
            "https://host.example".into(),
            BridgeInbound::Theme {
                color_scheme: op_editor_core::ThemeMode::Dark,
            },
        );
        assert!(inbox.take_over().is_empty());
    }

    #[test]
    fn take_over_captures_the_receive_time_origin_per_message() {
        let mut inbox = EarlyInbox::new();
        inbox.push(
            "https://a.example".into(),
            BridgeInbound::Locale {
                locale: op_editor_core::Locale::ZhCn,
            },
        );
        let buffered = inbox.take_over();
        assert_eq!(buffered.len(), 1);
        assert_eq!(buffered[0].0, "https://a.example");
        assert_eq!(
            buffered[0].1,
            BridgeInbound::Locale {
                locale: op_editor_core::Locale::ZhCn
            }
        );
    }
}

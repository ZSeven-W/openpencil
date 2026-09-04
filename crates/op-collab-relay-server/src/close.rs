//! Relay-side connection teardown.
//!
//! Closing a relayed tunnel is not just "write a close frame and drop the
//! socket". Dropping a `TcpStream` whose receive queue still holds unread
//! bytes makes the kernel send RST instead of FIN, and an RST that overtakes
//! the frames already in flight discards them from the peer's receive buffer.
//! The peer's tungstenite then reports
//! `ProtocolError::ResetWithoutClosingHandshake` — a generic protocol fault —
//! and the reason the relay carefully encoded is gone.
//!
//! That is exactly how a `Rejected(PairingTimeout)` used to vanish: the client
//! saw an unexplained transport error where the relay had told it precisely
//! why its lane was retired. Two defences are applied here together:
//!
//! 1. the reject reason is repeated in the WebSocket close frame's reason
//!    payload, so it survives anything that forwards the closing handshake but
//!    not a trailing data frame;
//! 2. the close is followed by a bounded linger that drains the peer's side of
//!    the closing handshake before the socket is dropped, so the relay never
//!    resets a connection it has just written an explanation to.

use std::{borrow::Cow, time::Duration};

use futures_util::{SinkExt, StreamExt};
use op_collab_relay_protocol::{RelayRejectCode, RelayServerStatus};
use tokio_tungstenite::tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame},
    Message,
};

use crate::connection::{WebSocketSink, WebSocketSource};

/// How long the relay drains a peer's closing handshake before dropping the
/// socket.
///
/// Ends early on the peer's close echo or on EOF, so a well-behaved peer costs
/// a round trip; the ceiling only bounds a peer that stops reading.
pub(crate) const CLOSE_LINGER: Duration = Duration::from_secs(1);

/// Why the relay is closing a connection.
///
/// Typed rather than a bare string so every close site is enumerable, carries a
/// stable log label, and cannot invent an ad-hoc reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RelayCloseReason {
    Shutdown,
    IdleTimeout,
    TunnelLifetime,
    ReauthenticationFailed,
    PeerDisconnected,
    Backpressure,
    MessageTooLarge,
    TextUnsupported,
    UnexpectedFrame,
    /// The peer sent its own close frame.
    PeerClosed,
    /// The peer's stream ended without a closing handshake.
    ///
    /// Never written to the wire — there is nothing left to write to. It
    /// exists so a connection cannot disappear from the logs without a reason.
    PeerEof,
    /// The peer's transport failed: a reset, a framing fault, or an I/O error.
    ///
    /// Also never written to the wire. Counted separately from
    /// [`RelayCloseReason::PeerEof`] because a rising reset count means an
    /// intermediary is severing tunnels, which is a different outage from
    /// clients simply going away.
    PeerReset,
    Rejected(RelayRejectCode),
}

impl RelayCloseReason {
    pub(crate) const fn code(self) -> CloseCode {
        match self {
            Self::Shutdown => CloseCode::Restart,
            Self::IdleTimeout
            | Self::TunnelLifetime
            | Self::PeerDisconnected
            | Self::PeerClosed
            | Self::PeerEof => CloseCode::Away,
            Self::PeerReset => CloseCode::Protocol,
            Self::ReauthenticationFailed | Self::Rejected(_) => CloseCode::Policy,
            Self::Backpressure => CloseCode::Again,
            Self::MessageTooLarge => CloseCode::Size,
            Self::TextUnsupported => CloseCode::Unsupported,
            Self::UnexpectedFrame => CloseCode::Protocol,
        }
    }

    /// The close frame's reason payload.
    ///
    /// For a rejection this is the machine-readable
    /// [`RelayRejectCode::close_reason`] token the client decodes back into the
    /// original reject code.
    pub(crate) const fn wire_reason(self) -> &'static str {
        match self {
            Self::Shutdown => "relay shutdown",
            Self::IdleTimeout => "idle timeout",
            Self::TunnelLifetime => "tunnel lifetime reached",
            Self::ReauthenticationFailed => "relay reauthentication failed",
            Self::PeerDisconnected => "peer disconnected",
            Self::Backpressure => "relay backpressure",
            Self::MessageTooLarge => "message too large",
            Self::TextUnsupported => "binary messages only",
            Self::UnexpectedFrame => "unexpected frame",
            Self::PeerClosed => "peer closed",
            Self::PeerEof => "peer stream ended",
            Self::PeerReset => "peer transport failed",
            Self::Rejected(code) => code.close_reason(),
        }
    }

    /// Stable label for operational logging.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Shutdown => "shutdown",
            Self::IdleTimeout => "idle-timeout",
            Self::TunnelLifetime => "tunnel-lifetime",
            Self::ReauthenticationFailed => "reauthentication-failed",
            Self::PeerDisconnected => "peer-disconnected",
            Self::Backpressure => "backpressure",
            Self::MessageTooLarge => "message-too-large",
            Self::TextUnsupported => "text-unsupported",
            Self::UnexpectedFrame => "unexpected-frame",
            Self::PeerClosed => "peer-closed",
            Self::PeerEof => "peer-eof",
            Self::PeerReset => "peer-reset",
            Self::Rejected(code) => code.label(),
        }
    }
}

pub(crate) async fn send_status(sink: &mut WebSocketSink, status: RelayServerStatus) -> bool {
    sink.send(Message::Binary(status.encode().to_vec()))
        .await
        .is_ok()
}

/// Send a rejection status, then close carrying the same reason.
///
/// The status frame stays the primary channel — clients that read it decode the
/// exact [`RelayRejectCode`] — and the close frame repeats it as a fallback.
pub(crate) async fn reject_and_close(
    sink: &mut WebSocketSink,
    source: &mut WebSocketSource,
    code: RelayRejectCode,
) {
    let _ = send_status(sink, RelayServerStatus::Rejected(code)).await;
    close_and_linger(sink, source, RelayCloseReason::Rejected(code)).await;
}

/// Close the tunnel and complete the closing handshake.
pub(crate) async fn close_and_linger(
    sink: &mut WebSocketSink,
    source: &mut WebSocketSource,
    reason: RelayCloseReason,
) {
    close(sink, reason).await;
    linger(source, CLOSE_LINGER).await;
}

/// Write the close frame. `SinkExt::send` flushes, so the frame reaches the
/// socket before this returns.
///
/// Used bare only on the paired forwarding path, where the peer is actively
/// reading the tunnel and no reject reason is being conveyed. Every teardown
/// that carries a reason, and every teardown of a peer that may be sitting
/// idle, goes through [`close_and_linger`] instead.
pub(crate) async fn close(sink: &mut WebSocketSink, reason: RelayCloseReason) {
    let _ = sink
        .send(Message::Close(Some(CloseFrame {
            code: reason.code(),
            reason: Cow::Borrowed(reason.wire_reason()),
        })))
        .await;
}

/// Drain the peer until it acknowledges the close, disconnects, or the budget
/// runs out.
///
/// Draining is what actually prevents the reset: a socket dropped with unread
/// inbound bytes is closed with RST, which discards whatever the relay just
/// wrote from the peer's receive buffer.
async fn linger(source: &mut WebSocketSource, budget: Duration) {
    let _ = tokio::time::timeout(budget, async {
        while let Some(message) = source.next().await {
            match message {
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }
    })
    .await;
}

#[cfg(test)]
mod tests {
    use op_collab_relay_protocol::RELAY_REJECT_CODES;

    use super::*;

    #[test]
    fn a_rejection_close_frame_carries_a_decodable_reject_code() {
        for code in RELAY_REJECT_CODES {
            let reason = RelayCloseReason::Rejected(code);
            assert_eq!(reason.code(), CloseCode::Policy);
            assert_eq!(
                RelayRejectCode::from_close_reason(reason.wire_reason()),
                Some(code)
            );
            // Close frame payloads are capped at 125 bytes including the code.
            assert!(reason.wire_reason().len() <= 123);
        }
    }

    #[test]
    fn an_ordinary_close_reason_is_never_read_back_as_a_rejection() {
        for reason in [
            RelayCloseReason::Shutdown,
            RelayCloseReason::IdleTimeout,
            RelayCloseReason::TunnelLifetime,
            RelayCloseReason::ReauthenticationFailed,
            RelayCloseReason::PeerDisconnected,
            RelayCloseReason::Backpressure,
            RelayCloseReason::MessageTooLarge,
            RelayCloseReason::TextUnsupported,
            RelayCloseReason::UnexpectedFrame,
            RelayCloseReason::PeerClosed,
            RelayCloseReason::PeerEof,
            RelayCloseReason::PeerReset,
        ] {
            assert_eq!(
                RelayRejectCode::from_close_reason(reason.wire_reason()),
                None
            );
            assert!(!reason.label().is_empty());
        }
    }
}

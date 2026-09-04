use op_collab_relay_client::{RelayBridgePhase, RelayFailureKind};
use op_collab_transport::RuntimeError;

use super::super::relay::OwnerRelayBridgeReport;
use super::super::types::CollabRuntimeFailure;

struct RelaySecureTransportFailure<'a> {
    failure: CollabRuntimeFailure,
    relay_phase: RelayBridgePhase,
    relay_failure: Option<RelayFailureKind>,
    transport_error: &'a RuntimeError,
}

impl std::fmt::Display for RelaySecureTransportFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let diagnostic = self.transport_error.safe_diagnostic();
        write!(
            formatter,
            "RelayGuestStageFailed {{ stage: SecureTransport, failure: {:?}, relay_phase: {:?}, \
             relay_failure: {:?}, transport_phase: {:?}, io_kind: {:?} }}",
            self.failure,
            self.relay_phase,
            self.relay_failure,
            diagnostic.phase,
            diagnostic.io_kind
        )
    }
}

pub(super) fn report_relay_secure_transport_failure(
    failure: CollabRuntimeFailure,
    relay_phase: RelayBridgePhase,
    relay_failure: Option<RelayFailureKind>,
    transport_error: &RuntimeError,
) {
    let diagnostic = RelaySecureTransportFailure {
        failure,
        relay_phase,
        relay_failure,
        transport_error,
    };
    eprintln!("[collab] {diagnostic}");
}

struct OwnerSecureTransportFailure<'a> {
    transport_error: &'a RuntimeError,
}

impl std::fmt::Display for OwnerSecureTransportFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let diagnostic = self.transport_error.safe_diagnostic();
        write!(
            formatter,
            "RelayOwnerStageFailed {{ stage: SecureTransport, transport_phase: {:?}, io_kind: \
             {:?} }}",
            diagnostic.phase, diagnostic.io_kind
        )
    }
}

pub(super) fn report_owner_secure_transport_failure(transport_error: &RuntimeError) {
    let diagnostic = OwnerSecureTransportFailure { transport_error };
    eprintln!("[collab] {diagnostic}");
}

/// Report an owner relay-pool transition.
///
/// An owner whose lane pool is empty is invisible in the relay's waiting
/// queue: guests that try to join simply fail to pair, and until now nothing
/// on the owner side could observe that. Reporting every transition — not just
/// failures — makes a reconnect loop readable in a desktop terminal, Xcode, or
/// logcat.
pub(super) fn report_owner_relay_bridge(report: OwnerRelayBridgeReport) {
    eprintln!(
        "[collab] RelayOwnerBridge {{ phase: {:?}, waiting_lanes: {}, active_tunnels: {}, \
         last_error: {:?}, relay_pairing_timeouts: {} }}",
        report.phase,
        report.waiting_lanes,
        report.active_tunnels,
        report.last_error,
        report.relay_pairing_timeouts
    );
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use op_collab_transport::{NoiseTransportError, RuntimeError, RuntimeErrorPhase};

    use super::*;

    fn sensitive_noise_error() -> RuntimeError {
        RuntimeError::Noise(NoiseTransportError::Io(std::io::Error::new(
            ErrorKind::UnexpectedEof,
            "invite=1SECRET endpoint=wss://secret key=private",
        )))
    }

    #[test]
    fn relay_diagnostic_format_is_credential_free() {
        let error = sensitive_noise_error();
        let diagnostic = RelaySecureTransportFailure {
            failure: CollabRuntimeFailure::RelayUnavailable,
            relay_phase: RelayBridgePhase::Active,
            relay_failure: None,
            transport_error: &error,
        };

        assert_eq!(
            diagnostic.to_string(),
            "RelayGuestStageFailed { stage: SecureTransport, failure: RelayUnavailable, \
             relay_phase: Active, relay_failure: None, transport_phase: Noise, io_kind: \
             Some(UnexpectedEof) }"
        );
    }

    #[test]
    fn owner_diagnostic_format_is_credential_free() {
        let error = sensitive_noise_error();
        let diagnostic = OwnerSecureTransportFailure {
            transport_error: &error,
        };

        let rendered = diagnostic.to_string();
        assert_eq!(
            rendered,
            "RelayOwnerStageFailed { stage: SecureTransport, transport_phase: Noise, io_kind: \
             Some(UnexpectedEof) }"
        );
        assert!(!rendered.contains("1SECRET"));
        assert!(!rendered.contains("wss://secret"));
        assert!(!rendered.contains("private"));
        assert_eq!(error.safe_diagnostic().phase, RuntimeErrorPhase::Noise);
    }
}

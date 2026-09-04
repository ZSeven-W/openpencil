//! Shim over `op_chat_agent::provider_dial` (the connect-time endpoint
//! guard moved there, pure code motion, so mobile hosts share it). Only the
//! browser-credential allowlist policy stays here — it reads
//! `web_credentials`, which remains daemon-side.

pub(crate) use op_chat_agent::provider_dial::{
    client_for, client_for_tunnel_compatible_public_asset, EndpointDialPolicy, ProviderDialError,
};

/// Dial policy for a browser-supplied endpoint: explicit allowlist entries
/// (the operator's intranet opt-in) dial as configured; everything else is
/// screened and pinned at connect time.
pub(crate) fn web_dial_policy_for(base_url: &str, allowlist: Option<&str>) -> EndpointDialPolicy {
    if crate::web_credentials::base_url_is_explicitly_allowlisted(base_url, allowlist) {
        EndpointDialPolicy::Trusted
    } else {
        EndpointDialPolicy::PublicOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_endpoints_dial_public_only_unless_allowlisted() {
        assert_eq!(
            web_dial_policy_for("https://api.deepseek.com/v1", None),
            EndpointDialPolicy::PublicOnly
        );
        assert_eq!(
            web_dial_policy_for(
                "http://127.0.0.1:11434/v1",
                Some("https://inference.example.com,http://127.0.0.1:11434"),
            ),
            EndpointDialPolicy::Trusted
        );
        assert_eq!(
            web_dial_policy_for("https://api.deepseek.com/v1", Some("https://other.example")),
            EndpointDialPolicy::PublicOnly
        );
    }
}

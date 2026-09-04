//! Connect-time endpoint guarding for browser-originated AI providers.
//!
//! URL-shape validation (`web_credentials::validate_web_provider_base_url`)
//! screens the hostname, but a hostname that resolves to a public address at
//! validation time can resolve to a private one at connect time (DNS
//! rebinding). Providers built from browser-supplied credentials therefore
//! resolve the endpoint host themselves, reject any reserved resolution, and
//! pin the HTTP client to the screened addresses. Operator-owned daemon
//! settings and explicitly allowlisted endpoints stay unguarded so intranet
//! deployments (local ollama/vLLM) keep working.

use std::net::SocketAddr;

#[path = "provider_dial_error.rs"]
mod error;
pub use error::ProviderDialError;

/// How a provider endpoint may be dialed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointDialPolicy {
    /// Operator-owned or explicitly allowlisted endpoint: dial as configured.
    Trusted,
    /// Browser-supplied endpoint: resolve, screen every address against the
    /// reserved ranges, and pin the connection to the screened set.
    PublicOnly,
}

// NOTE: `web_dial_policy_for` (the browser-credential allowlist policy) did
// NOT move — it needs `web_credentials` and stays in op-host-services'
// provider_dial shim.

/// Build the HTTP client for one provider request. `Trusted` dials as
/// configured; `PublicOnly` resolves the endpoint host here, screens every
/// address, and pins the client to the screened set so the connection can
/// only reach what was screened (no rebinding window between checks).
pub async fn client_for(
    policy: EndpointDialPolicy,
    url: &str,
) -> Result<reqwest::Client, ProviderDialError> {
    match policy {
        EndpointDialPolicy::Trusted => crate::chat_builtin_http::builtin_http_client(),
        EndpointDialPolicy::PublicOnly => pinned_public_client(url, false).await,
    }
}

/// Build the stricter asset-only client used by bounded profile images.
///
/// Clash-style fake-IP TUN resolvers use RFC 2544 `198.18.0.0/15` addresses as
/// opaque public-host tokens. This seam accepts only a hostname whose entire
/// resolution is in that range; literals and mixed sets remain rejected.
pub async fn client_for_tunnel_compatible_public_asset(
    url: &str,
) -> Result<reqwest::Client, ProviderDialError> {
    pinned_public_client(url, true).await
}

async fn pinned_public_client(
    url: &str,
    allow_tunnel_fake_ip: bool,
) -> Result<reqwest::Client, ProviderDialError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| ProviderDialError::NotAUrl)?;
    let host = parsed
        .host_str()
        .ok_or(ProviderDialError::MissingHost)?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    let port = parsed
        .port_or_known_default()
        .ok_or(ProviderDialError::MissingPort)?;
    let host_is_literal = host.parse::<std::net::IpAddr>().is_ok();
    let addrs = if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        // Literal address: nothing to resolve, screening alone suffices.
        vec![SocketAddr::new(ip, port)]
    } else {
        tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|error| ProviderDialError::ResolveFailed {
                host: host.clone(),
                message: error.to_string(),
            })?
            .collect()
    };
    let addrs =
        screen_resolved_addrs_for_policy(&host, host_is_literal, addrs, allow_tunnel_fake_ip)?;
    // `.no_proxy()` is load-bearing, not a tidy-up: with an env/system HTTP
    // proxy configured, reqwest tunnels the request to the proxy and the
    // proxy re-resolves the target host — so `resolve_to_addrs` would be
    // bypassed and the DNS screen defeated. Pinning only holds when the
    // connection is made directly to the screened addresses.
    crate::chat_builtin_http::builtin_http_client_builder()
        .no_proxy()
        .resolve_to_addrs(&host, &addrs)
        .build()
        .map_err(|error| ProviderDialError::ClientBuild {
            message: error.to_string(),
        })
}

/// Screen a resolved address set for a `PublicOnly` dial. Empty resolutions
/// and sets containing ANY reserved address are rejected — a mixed
/// public/private answer is exactly the rebinding shape this guards against.
#[cfg(test)]
fn screen_resolved_addrs(
    host: &str,
    addrs: Vec<SocketAddr>,
) -> Result<Vec<SocketAddr>, ProviderDialError> {
    screen_resolved_addrs_for_policy(host, false, addrs, false)
}

fn screen_resolved_addrs_for_policy(
    host: &str,
    host_is_literal: bool,
    addrs: Vec<SocketAddr>,
    allow_tunnel_fake_ip: bool,
) -> Result<Vec<SocketAddr>, ProviderDialError> {
    if addrs.is_empty() {
        return Err(ProviderDialError::Unresolved {
            host: host.to_string(),
        });
    }
    let tunnel_fake_ip_set = allow_tunnel_fake_ip
        && !host_is_literal
        && addrs
            .iter()
            .all(|address| is_rfc2544_benchmark_address(address.ip()));
    if tunnel_fake_ip_set {
        return Ok(addrs);
    }
    if addrs
        .iter()
        .any(|addr| crate::ip_screen::is_restricted_ip(addr.ip()))
    {
        return Err(ProviderDialError::Reserved {
            host: host.to_string(),
        });
    }
    Ok(addrs)
}

fn is_rfc2544_benchmark_address(address: std::net::IpAddr) -> bool {
    matches!(
        address,
        std::net::IpAddr::V4(address)
            if address.octets()[0] == 198 && matches!(address.octets()[1], 18 | 19)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(ip: &str) -> SocketAddr {
        SocketAddr::new(ip.parse().expect("test ip parses"), 443)
    }

    #[test]
    fn public_only_client_builds_for_a_literal_public_ip() {
        let client = crate::chat_runtime::block_on_anywhere(client_for(
            EndpointDialPolicy::PublicOnly,
            "https://93.184.216.34/v1",
        ));
        assert!(
            client.is_ok(),
            "screened public literal IP must build a client"
        );
    }

    #[test]
    fn public_only_client_rejects_a_literal_reserved_ip() {
        let client = crate::chat_runtime::block_on_anywhere(client_for(
            EndpointDialPolicy::PublicOnly,
            "http://169.254.169.254/v1",
        ));
        assert!(
            client.is_err(),
            "reserved literal IP must never build a client"
        );
    }

    #[test]
    fn screen_rejects_empty_resolution() {
        assert!(screen_resolved_addrs("api.example.com", Vec::new()).is_err());
    }

    #[test]
    fn screen_accepts_public_addresses() {
        let addrs = vec![addr("93.184.216.34"), addr("2606:2800:220:1::1")];
        let screened =
            screen_resolved_addrs("api.example.com", addrs.clone()).expect("public set passes");
        assert_eq!(screened, addrs);
    }

    // `web_dial_policy_for`'s test moved with it to op-host-services'
    // provider_dial shim.

    #[test]
    fn screen_rejects_any_reserved_address_in_the_set() {
        for reserved in [
            "127.0.0.1",
            "10.0.0.7",
            "172.16.3.4",
            "192.168.1.1",
            "169.254.169.254",
            "168.63.129.16",
            "198.18.0.42",
            "::1",
            "fd00:ec2::254",
        ] {
            let mixed = vec![addr("93.184.216.34"), addr(reserved)];
            assert!(
                screen_resolved_addrs("api.example.com", mixed).is_err(),
                "reserved address {reserved} must poison the whole resolution"
            );
        }
    }

    #[test]
    fn bounded_asset_policy_accepts_only_hostname_fake_ip_sets() {
        let fake_ip = vec![addr("198.18.0.42")];
        assert!(
            screen_resolved_addrs_for_policy("avatar.example", false, fake_ip.clone(), true,)
                .is_ok()
        );
        assert!(screen_resolved_addrs_for_policy("198.18.0.42", true, fake_ip, true,).is_err());
        assert!(screen_resolved_addrs_for_policy(
            "avatar.example",
            false,
            vec![addr("198.18.0.42"), addr("93.184.216.34")],
            true,
        )
        .is_err());
    }
}

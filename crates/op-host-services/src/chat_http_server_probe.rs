//! OpenCode localhost-server identity checks and listen-line validation.

use std::time::Duration;

use tokio::sync::mpsc;

const PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_PROBE_BODY_BYTES: usize = 64 * 1024;

/// Verify that a localhost listener is actually OpenCode. The documented
/// health endpoint carries both a boolean marker and the running version;
/// merely accepting any 2xx on port 4096 is not an identity check.
pub(super) async fn probe_server(client: &reqwest::Client, base: &str) -> bool {
    tokio::time::timeout(PROBE_TIMEOUT, probe_server_inner(client, base))
        .await
        .unwrap_or(false)
}

/// Run the bounded identity probe only while the turn still has a receiver.
/// `None` distinguishes cancellation from a healthy/failed probe so callers
/// never fall through into spawning after Stop/New Chat.
pub(super) async fn probe_server_while_open<T>(
    tx: &mpsc::Sender<T>,
    client: &reqwest::Client,
    base: &str,
) -> Option<bool> {
    tokio::select! {
        biased;
        _ = tx.closed() => None,
        healthy = probe_server(client, base) => Some(healthy),
    }
}

async fn probe_server_inner(client: &reqwest::Client, base: &str) -> bool {
    let response = match client.get(format!("{base}/global/health")).send().await {
        Ok(response) if response.status().is_success() => response,
        _ => return false,
    };
    if response
        .content_length()
        .is_some_and(|size| size > MAX_PROBE_BODY_BYTES as u64)
    {
        return false;
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk else { return false };
        if body.len().saturating_add(chunk.len()) > MAX_PROBE_BODY_BYTES {
            return false;
        }
        body.extend_from_slice(&chunk);
    }
    let Ok(health) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return false;
    };
    health.get("healthy").and_then(|value| value.as_bool()) == Some(true)
        && health
            .get("version")
            .and_then(|value| value.as_str())
            .is_some_and(|version| !version.trim().is_empty())
}

/// Parse the listen announcement, accepting only unauthenticated HTTP URLs
/// on the loopback interface OpenPencil requested in the spawn argv.
pub fn parse_server_url(line: &str) -> Option<String> {
    if !line.starts_with("opencode server listening") {
        return None;
    }
    line.split_whitespace()
        .find(|token| token.starts_with("http://") || token.starts_with("https://"))
        .filter(|url| is_loopback_http_url(url))
        .map(|url| url.trim_end_matches('/').to_string())
}

fn is_loopback_http_url(raw: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

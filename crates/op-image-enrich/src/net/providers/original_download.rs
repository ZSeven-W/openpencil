//! Bounded upgrade from a selected catalogue thumbnail to its original image.

use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use reqwest::header::{CONTENT_TYPE, LOCATION};
use reqwest::Url;

use super::download::{normalize_image_mime_header, read_capped, sniff_image_mime};
use super::MAX_EMBEDDED_IMAGE_BYTES;

const ORIGINAL_FETCH_BUDGET: Duration = Duration::from_secs(8);
const MAX_REDIRECTS: usize = 3;
const MAX_ORIGINAL_PIXELS: u64 = 64_000_000;
const MAX_ORIGINAL_EDGE: u32 = 32_768;
const MAX_DELIVERED_EDGE: u32 = 2048;

/// Fetch only the selected hit's original. Failure leaves the caller's already
/// resolved thumbnail usable. DNS, redirects, body reads, and conversion share
/// one opportunity window; no redirect receives a fresh eight seconds.
pub(crate) async fn fetch_original_image_data_url(
    url: &str,
    deadline: Option<Instant>,
) -> Option<String> {
    let deadline = opportunity_deadline(Instant::now(), deadline)?;
    let url = safe_original_url(url)?;
    tokio::time::timeout_at(deadline.into(), fetch_until(url, deadline))
        .await
        .ok()
        .flatten()
}

fn opportunity_deadline(now: Instant, deadline: Option<Instant>) -> Option<Instant> {
    let deadline = deadline
        .unwrap_or(now + ORIGINAL_FETCH_BUDGET)
        .min(now + ORIGINAL_FETCH_BUDGET);
    (deadline > now).then_some(deadline)
}

async fn fetch_until(mut url: Url, deadline: Instant) -> Option<String> {
    for redirects in 0..=MAX_REDIRECTS {
        let host = url.host_str()?.trim_matches(['[', ']']).to_string();
        let addresses = public_addresses(&host, url.port_or_known_default()?).await?;
        let remaining = deadline.checked_duration_since(Instant::now())?;
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(remaining)
            .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
            // Pin validated DNS results for direct connections. System proxies
            // remain supported and own their remote DNS/connection policy.
            .resolve_to_addrs(&host, &addresses)
            .build()
            .ok()?;
        let response = client.get(url.clone()).send().await.ok()?;
        if response.status().is_redirection() {
            let location = response.headers().get(LOCATION)?.to_str().ok()?;
            url = checked_redirect(&url, location, redirects)?;
            continue;
        }
        if !response.status().is_success() {
            return None;
        }
        let mime = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(normalize_image_mime_header);
        let bytes = read_capped(response, MAX_EMBEDDED_IMAGE_BYTES).await?;
        let mime = mime.or_else(|| sniff_image_mime(&bytes).map(str::to_owned))?;
        // Decoding cannot block the async deadline or monopolize its worker.
        return tokio::task::spawn_blocking(move || original_data_url(&mime, &bytes))
            .await
            .ok()
            .flatten();
    }
    None
}

fn original_data_url(mime: &str, bytes: &[u8]) -> Option<String> {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;

    if bytes.len() > MAX_EMBEDDED_IMAGE_BYTES {
        return None;
    }
    let (width, height) = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;
    if !safe_dimensions(width, height) {
        return None;
    }
    if width.max(height) > MAX_DELIVERED_EDGE {
        // Highly compressed originals may be smaller in bytes than their
        // resized version. Enforce geometry even when compression grows.
        let (mime, encoded) = crate::net::downscale::reencode_for_renderer(bytes)?;
        if encoded.len() > MAX_EMBEDDED_IMAGE_BYTES {
            return None;
        }
        return Some(format!("data:{mime};base64,{}", B64.encode(encoded)));
    }
    let url = crate::net::fetch::renderable_image_data_url(mime, bytes)?;
    let encoded = url.split_once(';')?.1.strip_prefix("base64,")?;
    // The shared converter can transcode WebP to PNG/JPEG. Bound the resulting
    // raw payload, rather than treating base64's 4/3 inflation as image bytes.
    let padding = encoded
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'=')
        .count();
    let raw_len = encoded.len() / 4 * 3 - padding;
    (raw_len <= MAX_EMBEDDED_IMAGE_BYTES).then_some(url)
}

fn safe_dimensions(width: u32, height: u32) -> bool {
    width > 0
        && height > 0
        && width.max(height) <= MAX_ORIGINAL_EDGE
        && u64::from(width) * u64::from(height) <= MAX_ORIGINAL_PIXELS
}

fn checked_redirect(url: &Url, location: &str, followed: usize) -> Option<Url> {
    if followed >= MAX_REDIRECTS {
        return None;
    }
    safe_original_url(url.join(location).ok()?.as_str())
}

fn safe_original_url(value: &str) -> Option<Url> {
    let mut url = Url::parse(value).ok()?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    let host = url.host_str()?.trim_matches(['[', ']']);
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !public_ip(ip) {
            return None;
        }
    } else {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        if !host.contains('.')
            || [
                "localhost",
                "local",
                "internal",
                "home",
                "lan",
                "test",
                "invalid",
                "example",
                "onion",
                "arpa",
            ]
            .iter()
            .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
            || ["example.com", "example.net", "example.org"]
                .iter()
                .any(|name| host == *name || host.ends_with(&format!(".{name}")))
        {
            return None;
        }
    }
    url.set_fragment(None);
    Some(url)
}

async fn public_addresses(host: &str, port: u16) -> Option<Vec<SocketAddr>> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return public_ip(ip).then(|| vec![SocketAddr::new(ip, port)]);
    }
    let host = host.to_string();
    let addresses = tokio::task::spawn_blocking(move || {
        (host.as_str(), port)
            .to_socket_addrs()
            .ok()
            .map(|addresses| addresses.collect::<Vec<_>>())
    })
    .await
    .ok()??;
    validate_addresses(addresses)
}

fn validate_addresses(addresses: Vec<SocketAddr>) -> Option<Vec<SocketAddr>> {
    (!addresses.is_empty() && addresses.iter().all(|address| public_ip(address.ip())))
        .then_some(addresses)
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(matches!(a, 0 | 10 | 127 | 224..=255)
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && (b == 168 || (b == 0 && matches!(c, 0 | 2))))
                || (a == 192 && b == 88 && c == 99)
                || (a == 198 && matches!(b, 18 | 19))
                || (a == 198 && b == 51 && c == 100)
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            // Only global unicast; exclude protocol assignments, documentation,
            // and transition ranges that could tunnel to non-public IPv4.
            (segments[0] & 0xe000) == 0x2000
                && !(segments[0] == 0x2001 && (segments[1] <= 0x01ff || segments[1] == 0x0db8))
                && segments[0] != 0x2002
                && !(segments[0] == 0x3fff && segments[1] <= 0x0fff)
        }
    }
}

#[cfg(test)]
#[path = "original_download_tests.rs"]
mod tests;

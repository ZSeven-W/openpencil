//! Image download helpers: the capped streaming body read, mime
//! normalization and sniffing, and the data-URL embed. Carved out of the
//! `providers.rs` spine to keep it under the 800-line cap; pure code motion.

use reqwest::header::CONTENT_TYPE;

use super::MAX_EMBEDDED_IMAGE_BYTES;

/// Download `url` and embed it as a `data:` URL, subject to the 4 MiB cap.
/// Embeds only payloads the exact renderer can decode: PNG/JPEG go in as-is
/// (down-scaled when oversized), everything else must transcode or the
/// candidate is rejected — see `fetch::renderable_image_data_url`.
pub async fn fetch_image_data_url(client: &reqwest::Client, url: &str) -> Option<String> {
    let (mime, bytes) = fetch_image_bytes(client, url, MAX_EMBEDDED_IMAGE_BYTES).await?;
    crate::net::fetch::renderable_image_data_url(&mime, &bytes)
}

/// Download `url` and return its normalized image mime + raw bytes, subject
/// to `cap` (streaming abort). `None` for failures, empty bodies, and
/// non-embeddable mimes. Shared with the desktop, which layers its skia
/// down-scale pass on the bytes before embedding.
pub async fn fetch_image_bytes(
    client: &reqwest::Client,
    url: &str,
    cap: usize,
) -> Option<(String, Vec<u8>)> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let header_mime = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(normalize_image_mime_header);
    let bytes = read_capped(resp, cap).await?;
    if bytes.is_empty() {
        return None;
    }
    let mime = header_mime.or_else(|| sniff_image_mime(&bytes).map(str::to_string))?;
    Some((mime, bytes))
}

/// Read a response body, aborting as soon as it exceeds `cap` — the cap must
/// hold with or without a Content-Length header, and an over-cap body must
/// never be fully buffered first (a chunked response could otherwise stream
/// gigabytes into memory before a post-hoc length check).
pub async fn read_capped(mut resp: reqwest::Response, cap: usize) -> Option<Vec<u8>> {
    if resp.content_length().is_some_and(|len| len > cap as u64) {
        return None;
    }
    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await.ok()? {
        if bytes.len() + chunk.len() > cap {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    Some(bytes)
}

/// Normalize a Content-Type header into an embeddable `image/*` mime
/// (`image/jpg` alias folded, SVG rejected).
pub fn normalize_image_mime_header(value: &str) -> Option<String> {
    let mime = value.split(';').next()?.trim().to_ascii_lowercase();
    if mime == "image/jpg" {
        return Some("image/jpeg".to_string());
    }
    if mime.starts_with("image/") && mime != "image/svg+xml" {
        Some(mime)
    } else {
        None
    }
}

/// Magic-byte sniff for the embeddable raster formats.
pub fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

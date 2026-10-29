//! OpenAI / OpenAI-compatible `images/generations` (OpenAI itself plus the
//! Custom provider — Volcengine Ark Seedream, gateways, local servers).
//!
//! Two compatibility rules live here because "OpenAI-compatible" endpoints
//! disagree on details the plain OpenAI request hard-codes:
//!
//! - **Endpoint path.** A base URL that already ends in an API version
//!   segment (`https://api.openai.com/v1`, Ark's
//!   `https://ark.cn-beijing.volces.com/api/v3`) is the root that
//!   `images/generations` hangs off; only a bare host gets `/v1` added.
//!   Unconditionally appending `/v1/images/generations` turned Ark's base
//!   into `/api/v3/v1/images/generations`, so every generation failed
//!   before reaching the model (and silently degraded to a stock search).
//! - **Seedream request shape.** Seedream 4.x+ rejects the DALL·E 1024-class
//!   sizes (its pixel floor is far above 1024²) and watermarks results by
//!   default, so Ark gets an aspect-matched ~2K size and `watermark: false`.

use op_editor_core::agent_settings::{ImageGenProfile, ImageTestStatus};

use super::{provider_error, read_provider_body, ImageGenerateError};

const DEFAULT_BASE: &str = "https://api.openai.com";
const IMAGES_PATH: &str = "images/generations";

/// Target pixel area for Seedream 4.x+ requests: 2048², comfortably inside
/// the model family's accepted range and under the 4 MiB embed cap once
/// the JPEG result is downloaded.
const SEEDREAM_TARGET_AREA: f64 = 2048.0 * 2048.0;
/// Per-side ceiling Seedream accepts.
const SEEDREAM_MAX_SIDE: f64 = 4096.0;
/// Aspect clamp: beyond 4:1 the long side would hit the per-side ceiling
/// and drag the area under the model's pixel floor.
const SEEDREAM_MAX_RATIO: f64 = 4.0;

/// Resolve the `images/generations` URL for an OpenAI-compatible base.
pub(crate) fn images_endpoint(base_url: Option<&str>) -> String {
    let base = base_url
        .map(str::trim)
        .filter(|base| !base.is_empty())
        .unwrap_or(DEFAULT_BASE)
        .trim_end_matches('/');
    if base.ends_with(IMAGES_PATH) {
        return base.to_string();
    }
    let last_segment = base.rsplit('/').next().unwrap_or_default();
    // `https://host` has the host as its last segment, never a version.
    let has_path = base
        .split("://")
        .nth(1)
        .is_some_and(|rest| rest.contains('/'));
    if has_path && is_version_segment(last_segment) {
        format!("{base}/{IMAGES_PATH}")
    } else {
        format!("{base}/v1/{IMAGES_PATH}")
    }
}

/// `v1`, `v3`, `v1beta`, … — a `v` followed by at least one digit.
fn is_version_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    chars.next() == Some('v')
        && chars.next().is_some_and(|c| c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_alphanumeric())
}

/// Seedream (Volcengine Ark / BytePlus ModelArk). Ark callers often name an
/// inference endpoint id (`ep-…`) instead of the model, so the host counts.
fn is_seedream(profile: &ImageGenProfile) -> bool {
    let model = profile.model.to_ascii_lowercase();
    let host = profile
        .base_url
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    model.contains("seedream") || host.contains("volces.com") || host.contains("bytepluses.com")
}

/// Seedream 3.0 keeps per-side limits of [512, 2048], which the DALL·E
/// sizes already satisfy; only the 4.x+ family needs the ~2K sizing.
fn is_seedream_v3(profile: &ImageGenProfile) -> bool {
    profile.model.to_ascii_lowercase().contains("seedream-3")
}

/// TS `mapToOpenAISize`.
pub(crate) fn openai_size(width: Option<f64>, height: Option<f64>) -> &'static str {
    let (Some(w), Some(h)) = (width, height) else {
        return "1024x1024";
    };
    let ratio = w / h;
    if ratio > 1.3 {
        "1792x1024"
    } else if ratio < 0.77 {
        "1024x1792"
    } else {
        "1024x1024"
    }
}

/// Aspect-matched `WxH` at [`SEEDREAM_TARGET_AREA`], sides rounded to 16.
fn seedream_size(width: Option<f64>, height: Option<f64>) -> String {
    let ratio = match (width, height) {
        (Some(w), Some(h)) if w > 0.0 && h > 0.0 => {
            (w / h).clamp(1.0 / SEEDREAM_MAX_RATIO, SEEDREAM_MAX_RATIO)
        }
        _ => 1.0,
    };
    let side = |value: f64| ((value / 16.0).round() * 16.0).min(SEEDREAM_MAX_SIDE) as u32;
    let w = side((SEEDREAM_TARGET_AREA * ratio).sqrt());
    let h = side((SEEDREAM_TARGET_AREA / ratio).sqrt());
    format!("{w}x{h}")
}

/// The JSON body for one generation.
pub(crate) fn request_body(
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": profile.model,
        "prompt": prompt,
        "n": 1,
        "size": openai_size(width, height),
        "response_format": "url",
    });
    if is_seedream(profile) {
        if !is_seedream_v3(profile) {
            body["size"] = serde_json::Value::String(seedream_size(width, height));
        }
        body["watermark"] = serde_json::Value::Bool(false);
    }
    body
}

/// OpenAI / OpenAI-compatible `images/generations` request. Shared verbatim
/// by the desktop Generate popover (its own client/UA) and the web daemon
/// route.
pub async fn generate_openai(
    client: &reqwest::Client,
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, ImageGenerateError> {
    let resp = client
        .post(images_endpoint(profile.base_url.as_deref()))
        .bearer_auth(profile.api_key.trim())
        .json(&request_body(prompt, profile, width, height))
        .send()
        .await
        .map_err(|e| ImageGenerateError::Request {
            provider: "OpenAI",
            message: e.to_string(),
        })?;
    let (status, body) = read_provider_body("OpenAI", resp).await?;
    if !status.is_success() {
        return Err(ImageGenerateError::Provider(provider_error(
            "OpenAI", status, &body,
        )));
    }
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| ImageGenerateError::ResponseParse {
            provider: "OpenAI",
            message: e.to_string(),
        })?;
    json.get("data")
        .and_then(|d| d.as_array())
        .and_then(|d| d.first())
        .and_then(|d| {
            d.get("url")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    // Some OpenAI-compatible providers return b64_json.
                    d.get("b64_json")
                        .and_then(serde_json::Value::as_str)
                        .map(|b64| format!("data:image/png;base64,{b64}"))
                })
        })
        .ok_or(ImageGenerateError::MissingImageUrl)
}

/// Settings "Test" probe for OpenAI-compatible profiles, at zero cost: an
/// empty-prompt request to the real generation endpoint. A 400/422 that
/// complains about the prompt proves the URL routes and the key
/// authenticates without rendering (or billing) anything; 401/403 (key),
/// 404 (path or model) and transport failures are `Invalid`.
pub async fn test_openai_compatible(
    client: &reqwest::Client,
    profile: &ImageGenProfile,
) -> ImageTestStatus {
    let resp = match client
        .post(images_endpoint(profile.base_url.as_deref()))
        .bearer_auth(profile.api_key.trim())
        .json(&serde_json::json!({ "model": profile.model, "prompt": "" }))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return ImageTestStatus::Invalid,
    };
    let Ok((status, body)) = read_provider_body("OpenAI", resp).await else {
        return ImageTestStatus::Invalid;
    };
    if status.is_success() {
        return ImageTestStatus::Valid;
    }
    if matches!(status.as_u16(), 400 | 422) {
        // A rejected MODEL also answers 400 on some providers; only an
        // objection that leaves the model alone proves the profile works.
        let message = provider_error("OpenAI", status, &body).to_ascii_lowercase();
        if !message.contains("model") || message.contains("prompt") {
            return ImageTestStatus::Valid;
        }
    }
    ImageTestStatus::Invalid
}

#[cfg(test)]
#[path = "web_image_generate_openai_tests.rs"]
mod tests;

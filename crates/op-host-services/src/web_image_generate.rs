//! Browser image-generation route for the web daemon.
//!
//! `POST /api/ai/image/generate` mirrors the desktop Generate popover
//! backend (`op-host-desktop/src/image_generate_host.rs`: OpenAI /
//! OpenAI-compatible, Gemini inline-image, Replicate / Atlas prediction
//! polling).
//! The generation profile comes from the request body (browser-held keys)
//! or falls back to the daemon's persisted agent settings.
//!
//! Endpoint trust follows the chat route's rules: provider DEFAULT hosts
//! are product constants and dial with a plain client, but any custom
//! `base_url` reaching this browser-facing route is screened through
//! `web_credentials` + `provider_dial` (reserved-address rejection +
//! connect-time pinning) unless explicitly allowlisted. The provider's
//! RESULT url is screened the same way before download — a custom
//! provider's response is browser-controlled data and must not become an
//! SSRF hop through the daemon.

use std::time::Duration;

use op_editor_core::agent_settings::{ImageGenProfile, ImageGenProvider, ImageTestStatus};

use crate::provider_dial::EndpointDialPolicy;
use crate::web_image_search::fetch_image_data_url;

#[path = "web_image_generate_error.rs"]
mod error;
pub use error::ImageGenerateError;

#[path = "web_image_generate_atlas.rs"]
mod atlas;
pub use atlas::generate_atlas;

/// Truncation cap for surfaced provider errors (TS parity).
const ERROR_MESSAGE_CAP: usize = 200;

/// Cap for a provider RESPONSE body. Gemini legitimately inlines the image
/// as base64 (a 4 MiB image is ~5.5 MiB of JSON), so this sits well above
/// that — but it still bounds what a (browser-configured) custom endpoint
/// can stream into daemon memory.
const MAX_PROVIDER_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Read a provider response's status + body under
/// [`MAX_PROVIDER_BODY_BYTES`] (streaming abort, not read-then-check).
async fn read_provider_body(
    provider: &'static str,
    resp: reqwest::Response,
) -> Result<(reqwest::StatusCode, String), ImageGenerateError> {
    let status = resp.status();
    let bytes = crate::web_image_search::read_capped(resp, MAX_PROVIDER_BODY_BYTES)
        .await
        .ok_or(ImageGenerateError::BodyTooLarge { provider })?;
    Ok((status, String::from_utf8_lossy(&bytes).into_owned()))
}

pub(crate) struct WebImageGenerateRequest {
    pub(crate) prompt: String,
    pub(crate) width: Option<f64>,
    pub(crate) height: Option<f64>,
    pub(crate) profile: ImageGenProfile,
    /// Whether `profile` carries a custom endpoint that must be screened.
    pub(crate) custom_endpoint: bool,
}

/// Parse the request body, taking the browser-supplied profile when present
/// and falling back to the daemon's persisted active profile.
pub(crate) fn parse_generate_request(
    body: &str,
    state: &op_editor_core::EditorState,
) -> Result<WebImageGenerateRequest, ImageGenerateError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| ImageGenerateError::InvalidRequestBody)?;
    let obj = value
        .as_object()
        .ok_or(ImageGenerateError::InvalidRequestBody)?;
    let prompt = obj
        .get("prompt")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or(ImageGenerateError::MissingPrompt)?;
    let width = obj.get("width").and_then(serde_json::Value::as_f64);
    let height = obj.get("height").and_then(serde_json::Value::as_f64);
    let profile = match obj.get("profile").and_then(serde_json::Value::as_object) {
        Some(profile) => parse_profile(profile)?,
        None => daemon_active_profile(state).ok_or(ImageGenerateError::NotConfigured)?,
    };
    let custom_endpoint = profile
        .base_url
        .as_deref()
        .is_some_and(|base| !base.trim().is_empty());
    Ok(WebImageGenerateRequest {
        prompt: prompt.to_string(),
        width,
        height,
        profile,
        custom_endpoint,
    })
}

fn parse_profile(
    profile: &serde_json::Map<String, serde_json::Value>,
) -> Result<ImageGenProfile, ImageGenerateError> {
    let provider = match profile
        .get("provider")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
    {
        "openai" => ImageGenProvider::OpenAi,
        "gemini" => ImageGenProvider::Gemini,
        "replicate" => ImageGenProvider::Replicate,
        "atlas" => ImageGenProvider::Atlas,
        "custom" => ImageGenProvider::Custom,
        _ => return Err(ImageGenerateError::UnknownProvider),
    };
    let api_key = profile
        .get("api_key")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .ok_or(ImageGenerateError::MissingApiKey)?;
    let model = profile
        .get("model")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .ok_or(ImageGenerateError::MissingModel)?;
    let base_url = profile
        .get("base_url")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .map(str::to_string);
    Ok(ImageGenProfile {
        id: String::new(),
        name: String::new(),
        provider,
        api_key: api_key.to_string(),
        model: model.to_string(),
        base_url,
        test_status: ImageTestStatus::Idle,
    })
}

/// The daemon's persisted active profile (desktop `active_image_gen_profile`).
fn daemon_active_profile(state: &op_editor_core::EditorState) -> Option<ImageGenProfile> {
    let settings = &state.editor_ui.agent_settings;
    settings
        .image_gen_profiles
        .iter()
        .find(|p| Some(&p.id) == settings.active_image_gen_profile_id.as_ref())
        .or_else(|| settings.image_gen_profiles.first())
        .filter(|p| !p.api_key.trim().is_empty())
        .cloned()
}

/// Run one generation on the calling thread (the connection's own thread —
/// the caller must NOT hold the state lock). Returns the final `data:` URL.
pub(crate) fn run_generate_blocking(
    request: &WebImageGenerateRequest,
) -> Result<String, ImageGenerateError> {
    // Same reasoning as `web_image_search::run_search_blocking`: a private
    // current-thread runtime panics when this sync helper is called from a
    // tokio worker, so the generation future rides the shared runtime.
    crate::chat_runtime::block_on_anywhere(run_generate(request))
}

async fn run_generate(request: &WebImageGenerateRequest) -> Result<String, ImageGenerateError> {
    let profile = &request.profile;
    // Screen browser-supplied custom endpoints before any dial; default
    // provider hosts are product constants and stay trusted.
    let endpoint_policy = if request.custom_endpoint {
        let base = profile.base_url.as_deref().unwrap_or_default();
        // The screening REASON is intentionally discarded (`|_|`) — a
        // browser-facing route must not become a probe oracle.
        crate::web_credentials::validate_web_provider_base_url(base)
            .map_err(|_| ImageGenerateError::EndpointNotAllowed)?;
        let allowlist = std::env::var(crate::web_credentials::WEB_AI_ENDPOINT_ALLOWLIST_ENV).ok();
        crate::provider_dial::web_dial_policy_for(base, allowlist.as_deref())
    } else {
        EndpointDialPolicy::Trusted
    };
    let client = match endpoint_policy {
        EndpointDialPolicy::Trusted => generate_client()?,
        EndpointDialPolicy::PublicOnly => {
            let base = profile.base_url.as_deref().unwrap_or_default();
            crate::provider_dial::client_for(endpoint_policy, base).await?
        }
    };
    let url = match profile.provider {
        ImageGenProvider::OpenAi | ImageGenProvider::Custom => {
            generate_openai(
                &client,
                &request.prompt,
                profile,
                request.width,
                request.height,
            )
            .await?
        }
        ImageGenProvider::Gemini => {
            generate_gemini(
                &client,
                &request.prompt,
                profile,
                request.width,
                request.height,
            )
            .await?
        }
        ImageGenProvider::Replicate => {
            generate_replicate(
                &client,
                &request.prompt,
                profile,
                request.width,
                request.height,
            )
            .await?
        }
        ImageGenProvider::Atlas => {
            generate_atlas(
                &client,
                &request.prompt,
                profile,
                request.width,
                request.height,
            )
            .await?
        }
    };
    if url.starts_with("data:") {
        // Inline base64 (Gemini / OpenAI b64_json). Unlike the desktop, no
        // down-scale pass here — image_downscale needs skia and this crate
        // stays GL-free.
        return Ok(url);
    }
    // Remote URL → embed as a data URL so the preview paints and the applied
    // src stays renderable offline. Any CUSTOM provider's result URL is
    // browser/endpoint-controlled data and is screened independently — an
    // allowlist entry trusts the configured endpoint, not every URL its
    // responses point at (the download host can be allowlisted separately).
    // Default provider hosts keep the trusted client for their own CDNs.
    let download_client = if request.custom_endpoint {
        let allowlist = std::env::var(crate::web_credentials::WEB_AI_ENDPOINT_ALLOWLIST_ENV).ok();
        let policy = crate::provider_dial::web_dial_policy_for(&url, allowlist.as_deref());
        crate::provider_dial::client_for(policy, &url)
            .await
            .map_err(|_| ImageGenerateError::ResultUrlNotAllowed)?
    } else {
        client.clone()
    };
    fetch_image_data_url(&download_client, &url)
        .await
        .ok_or(ImageGenerateError::DownloadFailed)
}

fn generate_client() -> Result<reqwest::Client, ImageGenerateError> {
    reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("openpencil-web-daemon/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| ImageGenerateError::ClientBuild {
            message: e.to_string(),
        })
}

/// Cap + JSON-wrap an error for the reply body.
///
/// Takes anything `Display` rather than `&str` so the route can hand it an
/// [`ImageGenerateError`] directly — the rendered sentence is identical
/// either way, and the truncation still applies to the finished text.
pub(crate) fn generate_error_json(message: impl std::fmt::Display) -> String {
    let message: String = message
        .to_string()
        .chars()
        .take(ERROR_MESSAGE_CAP)
        .collect();
    serde_json::json!({ "ok": false, "error": message }).to_string()
}

pub(crate) fn generate_ok_json(url: &str) -> String {
    serde_json::json!({ "ok": true, "url": url }).to_string()
}

/// TS `mapToOpenAISize`.
fn openai_size(width: Option<f64>, height: Option<f64>) -> &'static str {
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

/// TS `mapToGeminiAspectRatio`.
fn gemini_aspect_ratio(width: Option<f64>, height: Option<f64>) -> Option<&'static str> {
    let (Some(w), Some(h)) = (width, height) else {
        return None;
    };
    let ratio = w / h;
    Some(if ratio > 1.6 {
        "16:9"
    } else if ratio > 1.3 {
        "4:3"
    } else if ratio < 0.625 {
        "9:16"
    } else if ratio < 0.77 {
        "3:4"
    } else {
        "1:1"
    })
}

fn provider_error(provider: &str, status: reqwest::StatusCode, body: &str) -> String {
    // TS: prefer the provider's error.message, else status + slice.
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(message) = json
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(serde_json::Value::as_str)
            .or_else(|| json.get("detail").and_then(serde_json::Value::as_str))
            .or_else(|| json.get("message").and_then(serde_json::Value::as_str))
        {
            return message.chars().take(ERROR_MESSAGE_CAP).collect();
        }
    }
    let mut msg = format!("{provider} returned {}", status.as_u16());
    if !body.is_empty() {
        msg.push_str(": ");
        msg.push_str(&body.chars().take(150).collect::<String>());
    }
    msg
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
    let base = profile
        .base_url
        .as_deref()
        .filter(|b| !b.trim().is_empty())
        .unwrap_or("https://api.openai.com");
    let endpoint = format!("{}/v1/images/generations", base.trim_end_matches('/'));
    let resp = client
        .post(endpoint)
        .bearer_auth(profile.api_key.trim())
        .json(&serde_json::json!({
            "model": profile.model,
            "prompt": prompt,
            "n": 1,
            "size": openai_size(width, height),
            "response_format": "url",
        }))
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

/// Gemini `generateContent` inline-image request (shared desktop + web).
pub async fn generate_gemini(
    client: &reqwest::Client,
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, ImageGenerateError> {
    let base = profile
        .base_url
        .as_deref()
        .filter(|b| !b.trim().is_empty())
        .unwrap_or("https://generativelanguage.googleapis.com");
    let endpoint = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        base.trim_end_matches('/'),
        profile.model,
        profile.api_key.trim()
    );
    let mut generation_config = serde_json::json!({
        "responseModalities": ["TEXT", "IMAGE"],
    });
    if let Some(aspect) = gemini_aspect_ratio(width, height) {
        generation_config["imageConfig"] = serde_json::json!({ "aspectRatio": aspect });
    }
    let resp = client
        .post(endpoint)
        .json(&serde_json::json!({
            "contents": [{ "parts": [{ "text": prompt }] }],
            "generationConfig": generation_config,
        }))
        .send()
        .await
        // `without_url`: the Gemini endpoint carries `?key=…`, and a plain
        // reqwest error Display would echo that URL — API key included —
        // into the surfaced error string (shown in the popover / returned
        // to the browser).
        .map_err(|e| ImageGenerateError::Request {
            provider: "Gemini",
            message: e.without_url().to_string(),
        })?;
    let (status, body) = read_provider_body("Gemini", resp).await?;
    if !status.is_success() {
        return Err(ImageGenerateError::Provider(provider_error(
            "Gemini", status, &body,
        )));
    }
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| ImageGenerateError::ResponseParse {
            provider: "Gemini",
            message: e.to_string(),
        })?;
    let parts = json
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array());
    let Some(parts) = parts else {
        return Err(ImageGenerateError::MissingInlineImage);
    };
    for part in parts {
        let Some(inline) = part.get("inlineData") else {
            continue;
        };
        let mime = inline
            .get("mimeType")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if !mime.starts_with("image/") {
            continue;
        }
        if let Some(data) = inline.get("data").and_then(serde_json::Value::as_str) {
            return Ok(format!("data:{mime};base64,{data}"));
        }
    }
    Err(ImageGenerateError::MissingInlineImage)
}

/// Replicate prediction create + poll (shared desktop + web).
pub async fn generate_replicate(
    client: &reqwest::Client,
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, ImageGenerateError> {
    let base = profile
        .base_url
        .as_deref()
        .filter(|b| !b.trim().is_empty())
        .unwrap_or("https://api.replicate.com");
    let base = base.trim_end_matches('/');
    let mut input = serde_json::json!({ "prompt": prompt });
    if let Some(w) = width {
        input["width"] = serde_json::json!(w as i64);
    }
    if let Some(h) = height {
        input["height"] = serde_json::json!(h as i64);
    }
    let resp = client
        .post(format!("{base}/v1/predictions"))
        .bearer_auth(profile.api_key.trim())
        .json(&serde_json::json!({ "model": profile.model, "input": input }))
        .send()
        .await
        .map_err(|e| ImageGenerateError::Request {
            provider: "Replicate",
            message: e.to_string(),
        })?;
    let (status, body) = read_provider_body("Replicate", resp).await?;
    if !status.is_success() {
        return Err(ImageGenerateError::Provider(provider_error(
            "Replicate",
            status,
            &body,
        )));
    }
    let prediction: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| ImageGenerateError::ResponseParse {
            provider: "Replicate",
            message: e.to_string(),
        })?;
    let Some(id) = prediction.get("id").and_then(serde_json::Value::as_str) else {
        return Err(ImageGenerateError::MissingPredictionId {
            provider: "Replicate",
        });
    };
    // Poll until terminal (TS: max 120 s, 2 s interval). The deadline is
    // wall-clock, not iteration-count: each poll also carries its own short
    // timeout so a black-holing endpoint can't stretch "60 polls" into
    // hours of a held connection thread (the shared client's 120 s
    // per-request timeout multiplies per iteration otherwise).
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        // A poll never runs past the deadline: its own timeout is capped to
        // the time remaining, so the loop's total stays ~120 s instead of
        // "deadline + one full poll".
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        let poll_timeout = Duration::from_secs(15).min(deadline - now);
        let resp = client
            .get(format!("{base}/v1/predictions/{id}"))
            .bearer_auth(profile.api_key.trim())
            .timeout(poll_timeout)
            .send()
            .await
            .map_err(|e| ImageGenerateError::PollRequest {
                provider: "Replicate",
                message: e.to_string(),
            })?;
        let (status, body) = read_provider_body("Replicate", resp).await?;
        if !status.is_success() {
            return Err(ImageGenerateError::PollStatus {
                provider: "Replicate",
                status: status.as_u16(),
                body: body.chars().take(ERROR_MESSAGE_CAP).collect::<String>(),
            });
        }
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| ImageGenerateError::PollParse {
                provider: "Replicate",
                message: e.to_string(),
            })?;
        match json.get("status").and_then(serde_json::Value::as_str) {
            Some("succeeded") => {
                let output = json.get("output");
                let url = output
                    .and_then(|o| o.as_array())
                    .and_then(|a| a.first())
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| output.and_then(serde_json::Value::as_str));
                return url
                    .map(str::to_string)
                    .ok_or(ImageGenerateError::OutputMissing {
                        provider: "Replicate",
                    });
            }
            Some(s @ ("failed" | "canceled")) => {
                let detail = json
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown error");
                return Err(ImageGenerateError::PredictionFailed {
                    provider: "Replicate",
                    state: s.to_string(),
                    detail: detail.to_string(),
                });
            }
            _ => {}
        }
    }
    Err(ImageGenerateError::PredictionTimeout {
        provider: "Replicate",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_profile(id: &str, api_key: &str) -> op_editor_core::EditorState {
        let mut state = op_editor_core::EditorState::default();
        state.editor_ui.agent_settings.image_gen_profiles = vec![ImageGenProfile {
            id: id.to_string(),
            name: "p".into(),
            provider: ImageGenProvider::OpenAi,
            api_key: api_key.to_string(),
            model: "gpt-image-1".into(),
            base_url: None,
            test_status: ImageTestStatus::Idle,
        }];
        state
    }

    #[test]
    fn parse_generate_request_prefers_the_request_profile() {
        let state = state_with_profile("persisted", "sk-persisted");
        let req = parse_generate_request(
            r#"{"prompt":"a cat","width":1600,"height":900,
                "profile":{"provider":"gemini","model":"gemini-img","api_key":"sk-req"}}"#,
            &state,
        )
        .expect("parses");
        assert_eq!(req.prompt, "a cat");
        assert_eq!(req.width, Some(1600.0));
        assert_eq!(req.profile.provider, ImageGenProvider::Gemini);
        assert_eq!(req.profile.api_key, "sk-req");
        assert!(!req.custom_endpoint);
    }

    #[test]
    fn parse_generate_request_accepts_atlas_profiles() {
        let state = op_editor_core::EditorState::default();
        let req = parse_generate_request(
            r#"{"prompt":"a cat","profile":{"provider":"atlas","model":"google/nano-banana-2-lite/text-to-image","api_key":"test-key"}}"#,
            &state,
        )
        .expect("parses");
        assert_eq!(req.profile.provider, ImageGenProvider::Atlas);
        assert!(!req.custom_endpoint);
    }

    #[test]
    fn parse_generate_request_falls_back_to_the_daemon_profile() {
        let state = state_with_profile("persisted", "sk-persisted");
        let req = parse_generate_request(r#"{"prompt":"a cat"}"#, &state).expect("parses");
        assert_eq!(req.profile.api_key, "sk-persisted");
        // No configured profile anywhere → explicit error.
        let empty = op_editor_core::EditorState::default();
        let err = parse_generate_request(r#"{"prompt":"a cat"}"#, &empty)
            .err()
            .expect("unconfigured daemon must error")
            .to_string();
        assert_eq!(err, "image generation not configured");
    }

    #[test]
    fn parse_generate_request_flags_custom_endpoints_and_rejects_bad_input() {
        let state = op_editor_core::EditorState::default();
        let req = parse_generate_request(
            r#"{"prompt":"x","profile":{"provider":"custom","model":"m","api_key":"k",
                "base_url":"https://images.example.com"}}"#,
            &state,
        )
        .expect("parses");
        assert!(req.custom_endpoint);
        assert!(parse_generate_request(r#"{"prompt":""}"#, &state).is_err());
        assert!(parse_generate_request(
            r#"{"prompt":"x","profile":{"provider":"nope","model":"m","api_key":"k"}}"#,
            &state
        )
        .is_err());
        assert!(parse_generate_request(
            r#"{"prompt":"x","profile":{"provider":"openai","model":"m","api_key":""}}"#,
            &state
        )
        .is_err());
    }

    #[test]
    fn size_mappers_mirror_ts() {
        assert_eq!(openai_size(None, None), "1024x1024");
        assert_eq!(openai_size(Some(1600.0), Some(900.0)), "1792x1024");
        assert_eq!(openai_size(Some(900.0), Some(1600.0)), "1024x1792");
        assert_eq!(gemini_aspect_ratio(None, None), None);
        assert_eq!(
            gemini_aspect_ratio(Some(1920.0), Some(1080.0)),
            Some("16:9")
        );
        assert_eq!(gemini_aspect_ratio(Some(500.0), Some(500.0)), Some("1:1"));
    }

    #[test]
    fn provider_error_prefers_the_message_field() {
        let status = reqwest::StatusCode::BAD_GATEWAY;
        assert_eq!(
            provider_error(
                "OpenAI",
                status,
                r#"{"error":{"message":"quota exceeded"}}"#
            ),
            "quota exceeded"
        );
        assert_eq!(
            provider_error("Replicate", status, r#"{"detail":"invalid token"}"#),
            "invalid token"
        );
        assert!(provider_error("Gemini", status, "<html>boom</html>")
            .starts_with("Gemini returned 502"));
    }

    #[test]
    fn error_json_truncates_to_the_ts_cap() {
        let long = "x".repeat(300);
        let json: serde_json::Value =
            serde_json::from_str(&generate_error_json(&long)).expect("valid json");
        assert_eq!(json["error"].as_str().unwrap().len(), ERROR_MESSAGE_CAP);
        let ok: serde_json::Value =
            serde_json::from_str(&generate_ok_json("data:image/png;base64,AA==")).unwrap();
        assert_eq!(ok["ok"], true);
    }
}

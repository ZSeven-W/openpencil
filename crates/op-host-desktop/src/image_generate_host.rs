//! Provider backends for the image Generate popover — a port of the
//! TS `server/api/ai/image-generate.ts` (OpenAI / custom
//! OpenAI-compatible, Gemini inline-image, Replicate prediction
//! polling). Split out of `image_panel_host.rs` for the 800-line cap.

use std::time::Duration;

use op_editor_core::agent_settings::{ImageGenProfile, ImageGenProvider};

use crate::image_search_session::fetch_image_data_url;

// --- Generate backend (TS image-generate.ts) -------------------------

pub(crate) fn run_generate_blocking(
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;
    runtime.block_on(run_generate(prompt, profile, width, height))
}

async fn run_generate(
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let url = match profile.provider {
        ImageGenProvider::OpenAi | ImageGenProvider::Custom => {
            generate_openai(&client, prompt, profile, width, height).await?
        }
        ImageGenProvider::Gemini => {
            generate_gemini(&client, prompt, profile, width, height).await?
        }
        ImageGenProvider::Replicate => {
            generate_replicate(&client, prompt, profile, width, height).await?
        }
    };
    if url.starts_with("data:") {
        // Inline base64 (Gemini / OpenAI b64_json) → shrink an oversized
        // render before it enters the document, same as the file-pick path.
        return Ok(crate::image_downscale::maybe_downscale_data_url(&url).unwrap_or(url));
    }
    // Remote URL → embed as a data URL so the preview paints and the
    // applied src stays renderable offline (matches the search path).
    fetch_image_data_url(&client, &url)
        .await
        .ok_or_else(|| "generated image could not be downloaded".to_string())
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
        {
            return message.chars().take(200).collect();
        }
    }
    let mut msg = format!("{provider} returned {}", status.as_u16());
    if !body.is_empty() {
        msg.push_str(": ");
        msg.push_str(&body.chars().take(150).collect::<String>());
    }
    msg
}

async fn generate_openai(
    client: &reqwest::Client,
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, String> {
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
        .map_err(|e| format!("OpenAI request failed: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(provider_error("OpenAI", status, &body));
    }
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("OpenAI response parse: {e}"))?;
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
        .ok_or_else(|| "OpenAI response missing image URL".to_string())
}

async fn generate_gemini(
    client: &reqwest::Client,
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, String> {
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
        .map_err(|e| format!("Gemini request failed: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(provider_error("Gemini", status, &body));
    }
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Gemini response parse: {e}"))?;
    let parts = json
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array());
    let Some(parts) = parts else {
        return Err("Gemini response missing inline image data".to_string());
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
    Err("Gemini response missing inline image data".to_string())
}

async fn generate_replicate(
    client: &reqwest::Client,
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, String> {
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
        .map_err(|e| format!("Replicate request failed: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(provider_error("Replicate", status, &body));
    }
    let prediction: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Replicate response parse: {e}"))?;
    let Some(id) = prediction.get("id").and_then(serde_json::Value::as_str) else {
        return Err("Replicate response missing prediction ID".to_string());
    };
    // Poll until terminal (TS: max 120 s, 2 s interval).
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let resp = client
            .get(format!("{base}/v1/predictions/{id}"))
            .bearer_auth(profile.api_key.trim())
            .send()
            .await
            .map_err(|e| format!("Replicate poll request failed: {e}"))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!(
                "Replicate poll returned {}: {}",
                status.as_u16(),
                body.chars().take(200).collect::<String>()
            ));
        }
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Replicate poll parse: {e}"))?;
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
                    .ok_or_else(|| "Replicate succeeded but output is missing".to_string());
            }
            Some(s @ ("failed" | "canceled")) => {
                let detail = json
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown error");
                return Err(format!("Replicate prediction {s}: {detail}"));
            }
            _ => {}
        }
    }
    Err("Replicate prediction timed out after 120 seconds".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_mappers_mirror_ts() {
        assert_eq!(openai_size(None, None), "1024x1024");
        assert_eq!(openai_size(Some(1600.0), Some(900.0)), "1792x1024");
        assert_eq!(openai_size(Some(900.0), Some(1600.0)), "1024x1792");
        assert_eq!(openai_size(Some(800.0), Some(800.0)), "1024x1024");
        assert_eq!(gemini_aspect_ratio(None, None), None);
        assert_eq!(
            gemini_aspect_ratio(Some(1920.0), Some(1080.0)),
            Some("16:9")
        );
        assert_eq!(gemini_aspect_ratio(Some(800.0), Some(600.0)), Some("4:3"));
        assert_eq!(
            gemini_aspect_ratio(Some(1080.0), Some(1920.0)),
            Some("9:16")
        );
        assert_eq!(gemini_aspect_ratio(Some(600.0), Some(800.0)), Some("3:4"));
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
}

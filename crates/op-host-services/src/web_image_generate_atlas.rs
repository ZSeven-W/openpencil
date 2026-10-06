//! Atlas Cloud image submission and bounded prediction polling.

use std::time::Duration;

use op_editor_core::agent_settings::ImageGenProfile;

use super::{provider_error, read_provider_body, ImageGenerateError, ERROR_MESSAGE_CAP};

const PROVIDER: &str = "Atlas Cloud";
const DEFAULT_BASE_URL: &str = "https://api.atlascloud.ai/api/v1";
const POLL_INTERVAL: Duration = Duration::from_secs(3);
const PREDICTION_TIMEOUT: Duration = Duration::from_secs(120);

enum AtlasPrediction {
    Pending { id: String },
    Completed { url: String },
    Failed { state: String, detail: String },
}

/// Submit one Atlas Cloud image job, then poll its prediction with GET only.
/// The billable POST is intentionally issued exactly once.
pub async fn generate_atlas(
    client: &reqwest::Client,
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, ImageGenerateError> {
    let base = profile
        .base_url
        .as_deref()
        .filter(|base| !base.trim().is_empty())
        .unwrap_or(DEFAULT_BASE_URL)
        .trim_end_matches('/');
    let body = atlas_request_body(prompt, profile, width, height);

    let response = client
        .post(format!("{base}/model/generateImage"))
        .bearer_auth(profile.api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|error| ImageGenerateError::Request {
            provider: PROVIDER,
            message: error.to_string(),
        })?;
    let (status, response_body) = read_provider_body(PROVIDER, response).await?;
    if !status.is_success() {
        return Err(ImageGenerateError::Provider(provider_error(
            PROVIDER,
            status,
            &response_body,
        )));
    }
    let prediction = parse_atlas_response(&response_body, false)?;
    let id = match prediction {
        AtlasPrediction::Completed { url } => return Ok(url),
        AtlasPrediction::Failed { state, detail } => {
            return Err(ImageGenerateError::PredictionFailed {
                provider: PROVIDER,
                state,
                detail,
            });
        }
        AtlasPrediction::Pending { id } => id,
    };

    let deadline = tokio::time::Instant::now() + PREDICTION_TIMEOUT;
    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        let poll_timeout = Duration::from_secs(15).min(deadline - now);
        let response = client
            .get(format!("{base}/model/prediction/{id}"))
            .bearer_auth(profile.api_key.trim())
            .timeout(poll_timeout)
            .send()
            .await
            .map_err(|error| ImageGenerateError::PollRequest {
                provider: PROVIDER,
                message: error.to_string(),
            })?;
        let (status, response_body) = read_provider_body(PROVIDER, response).await?;
        if !status.is_success() {
            return Err(ImageGenerateError::PollStatus {
                provider: PROVIDER,
                status: status.as_u16(),
                body: response_body.chars().take(ERROR_MESSAGE_CAP).collect(),
            });
        }
        match parse_atlas_response(&response_body, true)? {
            AtlasPrediction::Pending { .. } => {}
            AtlasPrediction::Completed { url } => return Ok(url),
            AtlasPrediction::Failed { state, detail } => {
                return Err(ImageGenerateError::PredictionFailed {
                    provider: PROVIDER,
                    state,
                    detail,
                });
            }
        }
    }

    Err(ImageGenerateError::PredictionTimeout { provider: PROVIDER })
}

fn atlas_request_body(
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> serde_json::Value {
    serde_json::json!({
        "model": profile.model,
        "prompt": prompt,
        "aspect_ratio": atlas_aspect_ratio(width, height),
        "resolution": "1k",
    })
}

fn atlas_aspect_ratio(width: Option<f64>, height: Option<f64>) -> &'static str {
    let (Some(width), Some(height)) = (width, height) else {
        return "auto";
    };
    let ratio = width / height;
    if ratio > 1.6 {
        "16:9"
    } else if ratio > 1.3 {
        "4:3"
    } else if ratio < 0.625 {
        "9:16"
    } else if ratio < 0.77 {
        "3:4"
    } else {
        "1:1"
    }
}

fn parse_atlas_response(body: &str, polling: bool) -> Result<AtlasPrediction, ImageGenerateError> {
    let json: serde_json::Value = serde_json::from_str(body).map_err(|error| {
        if polling {
            ImageGenerateError::PollParse {
                provider: PROVIDER,
                message: error.to_string(),
            }
        } else {
            ImageGenerateError::ResponseParse {
                provider: PROVIDER,
                message: error.to_string(),
            }
        }
    })?;
    if json
        .get("code")
        .and_then(serde_json::Value::as_i64)
        .is_some_and(|code| code != 200)
    {
        let message = json
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("request rejected");
        return Err(ImageGenerateError::Provider(format!(
            "{PROVIDER} returned API error: {message}"
        )));
    }

    let data = json.get("data").unwrap_or(&json);
    let state = data
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("starting")
        .to_ascii_lowercase();
    if matches!(state.as_str(), "completed" | "succeeded") {
        let url = data
            .get("outputs")
            .and_then(serde_json::Value::as_array)
            .and_then(|outputs| outputs.first())
            .and_then(serde_json::Value::as_str)
            .filter(|url| !url.trim().is_empty())
            .ok_or(ImageGenerateError::OutputMissing { provider: PROVIDER })?;
        return Ok(AtlasPrediction::Completed {
            url: url.to_string(),
        });
    }
    if matches!(
        state.as_str(),
        "failed" | "timeout" | "canceled" | "cancelled"
    ) {
        let detail = data
            .get("error")
            .and_then(serde_json::Value::as_str)
            .or_else(|| json.get("message").and_then(serde_json::Value::as_str))
            .unwrap_or("unknown error");
        return Ok(AtlasPrediction::Failed {
            state,
            detail: detail.to_string(),
        });
    }

    let id = data
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or(ImageGenerateError::MissingPredictionId { provider: PROVIDER })?;
    Ok(AtlasPrediction::Pending { id: id.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::agent_settings::{ImageGenProvider, ImageTestStatus};

    fn profile() -> ImageGenProfile {
        ImageGenProfile {
            id: "atlas".into(),
            name: "Atlas Cloud".into(),
            provider: ImageGenProvider::Atlas,
            api_key: "test-key".into(),
            model: "google/nano-banana-2-lite/text-to-image".into(),
            base_url: None,
            test_status: ImageTestStatus::Idle,
        }
    }

    #[test]
    fn request_uses_supported_atlas_parameters() {
        let body = atlas_request_body("a red pencil", &profile(), Some(1920.0), Some(1080.0));
        assert_eq!(body["model"], "google/nano-banana-2-lite/text-to-image");
        assert_eq!(body["prompt"], "a red pencil");
        assert_eq!(body["aspect_ratio"], "16:9");
        assert_eq!(body["resolution"], "1k");
        assert_eq!(atlas_aspect_ratio(None, None), "auto");
    }

    #[test]
    fn response_parser_handles_pending_completed_and_failed_states() {
        let pending = parse_atlas_response(
            r#"{"code":200,"data":{"id":"prediction-1","status":"processing"}}"#,
            false,
        )
        .expect("pending response");
        assert!(matches!(pending, AtlasPrediction::Pending { id } if id == "prediction-1"));

        let completed = parse_atlas_response(
            r#"{"code":200,"data":{"status":"completed","outputs":["https://cdn.example/image.png"]}}"#,
            true,
        )
        .expect("completed response");
        assert!(
            matches!(completed, AtlasPrediction::Completed { url } if url.ends_with("image.png"))
        );

        let failed = parse_atlas_response(
            r#"{"code":200,"data":{"id":"prediction-1","status":"failed","error":"quota exceeded"}}"#,
            true,
        )
        .expect("failed response");
        assert!(
            matches!(failed, AtlasPrediction::Failed { detail, .. } if detail == "quota exceeded")
        );

        let timed_out = parse_atlas_response(
            r#"{"code":200,"data":{"id":"prediction-1","status":"timeout"}}"#,
            true,
        )
        .expect("timeout response");
        assert!(matches!(timed_out, AtlasPrediction::Failed { state, .. } if state == "timeout"));
    }
}

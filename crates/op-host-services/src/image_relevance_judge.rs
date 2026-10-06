//! OpenAI-compatible multimodal image relevance judge, plus a sibling judge
//! backed by the user's configured chat provider.

use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use futures::stream::{self, StreamExt};
use op_ai::chat_provider::ChatProvider;
use op_image_enrich::net::{ImageRelevanceJudge, JudgedCandidate, RelevanceVerdict};
use op_orchestrator::{VisionCallRequest, VisionLlmClient, VisionResponse};

use crate::validation_providers::ChatVisionLlmClient;

const BASE_URL_ENV: &str = "OPENPENCIL_IMAGE_JUDGE_BASE_URL";
const API_KEY_ENV: &str = "OPENPENCIL_IMAGE_JUDGE_API_KEY";
const MODEL_ENV: &str = "OPENPENCIL_IMAGE_JUDGE_MODEL";
const MAX_RETRIES: usize = 2;
const MAX_TOKENS: u32 = 600;
const MAX_CONCURRENT_CALLS: usize = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

const JUDGE_PROMPT: &str = "You are an image relevance judge. Query: {query}. Intent: {intent}. \
Inspect the image and return ONLY strict JSON: {\"verdict\":\"on\"|\"weak\"|\"off\",\"reason\":\"<=12 words\"}. \
Use \"off\" when the main subject is different, including a toy, unrelated object, text collage, or wrong scene. \
Use \"on\" for a clear match and \"weak\" for an ambiguous or partial match.";

/// Render the shared judge instruction. Both judge implementations send the
/// identical prompt so their verdicts stay comparable.
fn judge_prompt(query: &str, intent: &str) -> String {
    JUDGE_PROMPT
        .replace("{query}", query)
        .replace("{intent}", intent)
}

/// A vision judge backed by an OpenAI chat-completions-compatible endpoint.
#[derive(Clone)]
pub struct OpenAiCompatVisionJudge {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatVisionJudge {
    /// Construct the judge only when all three opt-in settings are present.
    /// Invalid endpoint/client configuration is treated as unavailable so
    /// callers can retain the ordinary no-judge path.
    pub fn from_env() -> Option<Self> {
        let base_url = non_blank_env(BASE_URL_ENV)?;
        let api_key = non_blank_env(API_KEY_ENV)?;
        let model = non_blank_env(MODEL_ENV)?;
        let endpoint = chat_completions_endpoint(&base_url)?;
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .ok()?;
        Some(Self {
            client,
            endpoint,
            api_key,
            model,
        })
    }

    async fn judge_one(
        &self,
        index: usize,
        query: &str,
        intent: &str,
        thumb_jpeg: Vec<u8>,
    ) -> JudgedCandidate {
        let mut last_error = String::from("unknown judge failure");
        for _attempt in 0..=MAX_RETRIES {
            match self.request_one(query, intent, &thumb_jpeg).await {
                Ok((content, reasoning_content)) => {
                    if let Some((verdict, reason)) =
                        parse_verdict_response(content.as_deref(), reasoning_content.as_deref())
                    {
                        return JudgedCandidate {
                            index,
                            verdict,
                            reason,
                        };
                    }
                    last_error = "response did not contain a valid verdict JSON".to_string();
                }
                Err(error) => last_error = error,
            }
        }
        eprintln!(
            "[IMAGE_JUDGE] candidate={} unavailable: {}",
            index, last_error
        );
        JudgedCandidate {
            index,
            verdict: RelevanceVerdict::Weak,
            reason: "judge unavailable".to_string(),
        }
    }

    async fn request_one(
        &self,
        query: &str,
        intent: &str,
        thumb_jpeg: &[u8],
    ) -> Result<(Option<String>, Option<String>), String> {
        let image_url = format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(thumb_jpeg)
        );
        let prompt = judge_prompt(query, intent);
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {"type": "image_url", "image_url": {"url": image_url}}
                ]
            }]
        });
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|error| format!("invalid response JSON: {error}"))?;
        let message = value
            .get("choices")
            .and_then(serde_json::Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"));
        let content = message
            .and_then(|message| message.get("content"))
            .and_then(value_as_text);
        let reasoning_content = message
            .and_then(|message| message.get("reasoning_content"))
            .and_then(value_as_text);
        Ok((content, reasoning_content))
    }
}

impl ImageRelevanceJudge for OpenAiCompatVisionJudge {
    fn judge(&self, query: &str, intent: &str, thumbs_jpeg: &[Vec<u8>]) -> Vec<JudgedCandidate> {
        let query = query.to_string();
        let intent = intent.to_string();
        let jobs = thumbs_jpeg
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, thumb)| {
                let judge = self.clone();
                let query = query.clone();
                let intent = intent.clone();
                async move { judge.judge_one(index, &query, &intent, thumb).await }
            });
        // `judge` is invoked from inside the image runtime's own task (the
        // judge-aware fetch is already running under `block_on_image_runtime`),
        // so blocking here through that runtime again panics with "cannot start
        // a runtime from within a runtime" and every candidate is lost — the
        // 2026-09-05 A/B run saw k=0 on all 170 slots. `block_on_anywhere` is the
        // one sanctioned way to block from sync code whatever the ambient context.
        let mut judged = crate::chat_runtime::block_on_anywhere(async move {
            stream::iter(jobs)
                .buffer_unordered(MAX_CONCURRENT_CALLS)
                .collect::<Vec<_>>()
                .await
        });
        judged.sort_by_key(|candidate| candidate.index);
        judged
    }
}

/// A vision judge backed by the user's configured chat provider: each
/// candidate thumbnail is judged by one multimodal chat turn through
/// [`ChatVisionLlmClient`], reusing the same prompt template and verdict
/// parsing as [`OpenAiCompatVisionJudge`]. This is the desktop path — users
/// there configure a vision-capable provider in the product instead of
/// setting the env judge.
pub struct ChatVisionJudge {
    client: ChatVisionLlmClient,
}

impl ChatVisionJudge {
    pub fn new(provider: Arc<dyn ChatProvider>) -> Self {
        Self {
            client: ChatVisionLlmClient::new(provider),
        }
    }

    /// Attach the vision model id to every judge request (`None` keeps the
    /// provider's own default).
    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.client = self.client.with_model(model);
        self
    }

    fn judge_one(&self, index: usize, prompt: &str, thumb_jpeg: &[u8]) -> JudgedCandidate {
        let unavailable = |reason: String| {
            eprintln!("[IMAGE_JUDGE] candidate={} unavailable: {}", index, reason);
            JudgedCandidate {
                index,
                verdict: RelevanceVerdict::Weak,
                reason: "judge unavailable".to_string(),
            }
        };
        let request = VisionCallRequest {
            system: String::new(),
            message: prompt.to_string(),
            image_base64: base64::engine::general_purpose::STANDARD.encode(thumb_jpeg),
            // The client was configured with its model at construction.
            model: None,
            provider: None,
            timeout: REQUEST_TIMEOUT,
        };
        match self.client.validate(request) {
            VisionResponse::Text(text) => match parse_verdict_response(Some(text.as_str()), None) {
                Some((verdict, reason)) => JudgedCandidate {
                    index,
                    verdict,
                    reason,
                },
                None => unavailable("response did not contain a valid verdict JSON".to_string()),
            },
            VisionResponse::Skipped { reason } => unavailable(
                reason.unwrap_or_else(|| "vision provider skipped the call".to_string()),
            ),
        }
    }
}

impl ImageRelevanceJudge for ChatVisionJudge {
    fn judge(&self, query: &str, intent: &str, thumbs_jpeg: &[Vec<u8>]) -> Vec<JudgedCandidate> {
        let prompt = judge_prompt(query, intent);
        let indexed: Vec<(usize, &Vec<u8>)> = thumbs_jpeg.iter().enumerate().collect();
        let mut judged = Vec::with_capacity(thumbs_jpeg.len());
        // `ChatVisionLlmClient::validate` is a blocking sync call, so
        // concurrency comes from scoped threads in waves of
        // MAX_CONCURRENT_CALLS — the same cap the async env judge uses.
        std::thread::scope(|scope| {
            for chunk in indexed.chunks(MAX_CONCURRENT_CALLS) {
                let handles: Vec<_> = chunk
                    .iter()
                    .map(|&(index, thumb)| {
                        let prompt = prompt.as_str();
                        scope.spawn(move || self.judge_one(index, prompt, thumb))
                    })
                    .collect();
                for (&(index, _), handle) in chunk.iter().zip(handles) {
                    judged.push(handle.join().unwrap_or_else(|_| JudgedCandidate {
                        index,
                        verdict: RelevanceVerdict::Weak,
                        reason: "judge unavailable".to_string(),
                    }));
                }
            }
        });
        judged.sort_by_key(|candidate| candidate.index);
        judged
    }
}

fn non_blank_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn chat_completions_endpoint(base_url: &str) -> Option<String> {
    let endpoint = if base_url.ends_with("/chat/completions") {
        base_url.to_string()
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    };
    reqwest::Url::parse(&endpoint)
        .ok()
        .map(|url| url.to_string())
}

fn value_as_text(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

fn parse_verdict_response(
    content: Option<&str>,
    reasoning_content: Option<&str>,
) -> Option<(RelevanceVerdict, String)> {
    parse_verdict_text(content).or_else(|| parse_verdict_text(reasoning_content))
}

#[cfg(test)]
fn parse_candidate_response(
    index: usize,
    content: Option<&str>,
    reasoning_content: Option<&str>,
) -> JudgedCandidate {
    if let Some((verdict, reason)) = parse_verdict_response(content, reasoning_content) {
        return JudgedCandidate {
            index,
            verdict,
            reason,
        };
    }
    JudgedCandidate {
        index,
        verdict: RelevanceVerdict::Weak,
        reason: "judge unavailable".to_string(),
    }
}

fn parse_verdict_text(text: Option<&str>) -> Option<(RelevanceVerdict, String)> {
    let text = text?.trim();
    let json = serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .or_else(|| {
            let start = text.find('{')?;
            let end = text.rfind('}')?;
            (start < end).then(|| serde_json::from_str(&text[start..=end]).ok())?
        })?;
    let verdict = match json
        .get("verdict")?
        .as_str()?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "on" => RelevanceVerdict::On,
        "weak" => RelevanceVerdict::Weak,
        "off" => RelevanceVerdict::Off,
        _ => return None,
    };
    let reason = json
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .split_whitespace()
        .take(12)
        .collect::<Vec<_>>()
        .join(" ");
    Some((verdict, reason))
}

#[cfg(test)]
#[path = "image_relevance_judge_tests.rs"]
mod tests;

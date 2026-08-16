//! LLM clients for the headless smoke runner.
//!
//! Two `op_orchestrator::LlmClient` impls carved off `main.rs` to keep both
//! files under the 800-line cap:
//!
//! - [`SmokeLlmClient`] — the default path, `agent`'s `QueryEngine` over an
//!   `AnthropicProvider` / `OpenAiCompatProvider`.
//! - [`DirectOpenAiClient`] — `OPENPENCIL_SMOKE_DIRECT=1`, a plain
//!   non-streaming openai-compat POST that can send the provider-specific
//!   reasoning controls the QueryEngine cannot.

use std::sync::Arc;

use agent::abort::AbortController;
use agent::provider::Provider;
use agent::query::QueryEngine;
use agent::stream::Event;
use futures::channel::mpsc;
use futures::StreamExt;
use op_host_services::chat_builtin_http::apply_reasoning_wire_control;
use op_orchestrator::{CallRequest, LlmChunk, LlmClient, LlmError};

/// Whether this harness call asks the provider to reduce reasoning.
///
/// Mirrors the live design turn: `design_turn_thinking_mode` forces thinking
/// off exactly for models whose profile declares `thinking_disabled`, and
/// `chat_builtin_http` then hands that flag to
/// [`apply_reasoning_wire_control`]. The two env overrides are harness-only
/// arms on the same flag — they never pick a wire shape themselves, so a model
/// that rejects the `thinking` field cannot be sent one by way of an override.
///
/// - `OPENPENCIL_SMOKE_DISABLE_THINKING=1` asks for reduction even when the
///   profile does not (benchmarking a 方舟-hosted reasoning model clean).
/// - `OPENPENCIL_SMOKE_KEEP_THINKING=1` wins over both: ab-v9 showed
///   M3-nothink emits lazy minimal manifests, so the with-think arm has to be
///   benchmarkable (`strip_reasoning` handles the `<think>` blocks).
pub(crate) fn reduce_reasoning_for_smoke(model: &str) -> bool {
    reduce_reasoning(
        model,
        std::env::var("OPENPENCIL_SMOKE_DISABLE_THINKING").is_ok(),
        std::env::var("OPENPENCIL_SMOKE_KEEP_THINKING").is_ok(),
    )
}

fn reduce_reasoning(model: &str, force_disable: bool, keep_thinking: bool) -> bool {
    !keep_thinking
        && (force_disable || op_orchestrator::resolve_model_profile(model).thinking_disabled)
}

/// `LlmClient` impl for the smoke runner — `AnthropicProvider` under a
/// `QueryEngine`, with every call spawned onto the current tokio runtime.
/// Standalone — `op-host-desktop` no longer ships a desktop
/// `LlmClient`; its production path goes through
/// `chat_provider_llm::ChatProviderLlmClient` (wrapping the user's
/// selected chat CLI). The smoke needs to talk to a raw API endpoint
/// to validate orchestrator behaviour independently of any CLI, hence
/// this dedicated client.
pub(crate) struct SmokeLlmClient {
    pub(crate) provider: Arc<dyn Provider>,
    pub(crate) default_model: String,
}

impl LlmClient for SmokeLlmClient {
    fn call(
        &self,
        req: CallRequest,
    ) -> futures::stream::BoxStream<'static, Result<LlmChunk, LlmError>> {
        let (tx, rx) = mpsc::unbounded::<Result<LlmChunk, LlmError>>();
        if req.abort.is_set() {
            let _ = tx.unbounded_send(Err(LlmError {
                message: "aborted".into(),
                aborted: true,
            }));
            return Box::pin(rx);
        }
        let provider = self.provider.clone();
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());
        let system = req.system_prompt.clone();
        let user = req.user_prompt.clone();

        eprintln!(
            "[LLM] call: model={model} system_len={} user_len={}",
            system.len(),
            user.len()
        );

        tokio::spawn(async move {
            // QueryEngine 默认 4096 输出 token,对推理模型(MiniMax-M3 等)远不够
            // ——它先吐 <think>(常 ~3.5k token)再给 JSON,4096 会在答案前截断。
            // 生产路径(chat_provider_llm)用 8192 且关思考;benchmark 走 QueryEngine
            // 无法关思考,故给更宽预算让其 think 完还能产出 JSON。可用
            // OPENPENCIL_SMOKE_MAX_TOKENS 覆盖。
            let max_tokens = std::env::var("OPENPENCIL_SMOKE_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(16384);
            let engine = QueryEngine::new(provider, model)
                .with_system(system)
                .with_max_output_tokens(max_tokens);
            let abort = AbortController::new();
            let stream = match engine.run(user, abort).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[LLM] engine.run error: {e}");
                    let _ = tx.unbounded_send(Err(LlmError {
                        message: e.to_string(),
                        aborted: false,
                    }));
                    return;
                }
            };
            let mut stream = stream;
            // Optional raw-response capture for diagnosing weak-model
            // JSON malformations (set OPENPENCIL_SMOKE_DUMP=1).
            let dump = std::env::var("OPENPENCIL_SMOKE_DUMP").is_ok();
            let mut full = String::new();
            while let Some(item) = stream.next().await {
                let sent = match item {
                    Ok(Event::TextDelta { delta }) => {
                        if dump {
                            full.push_str(&delta);
                        }
                        tx.unbounded_send(Ok(LlmChunk::Text(delta)))
                    }
                    Ok(Event::Thinking { delta }) => {
                        tx.unbounded_send(Ok(LlmChunk::Thinking(delta)))
                    }
                    Ok(Event::Result { .. }) => break,
                    Ok(Event::Error { code, message }) => {
                        eprintln!("[LLM] event error: {code}: {message}");
                        tx.unbounded_send(Err(LlmError {
                            message: format!("{code}: {message}"),
                            aborted: false,
                        }))
                    }
                    Ok(_) => Ok(()),
                    Err(e) => {
                        eprintln!("[LLM] stream error: {e}");
                        tx.unbounded_send(Err(LlmError {
                            message: e.to_string(),
                            aborted: false,
                        }))
                    }
                };
                if sent.is_err() {
                    break;
                }
            }
            if dump && !full.is_empty() {
                eprintln!(
                    "[DUMP] ===== LLM response ({} chars) =====\n{full}\n[DUMP] ===== end =====",
                    full.len()
                );
            }
        });

        Box::pin(rx)
    }
}

/// Direct openai-compat `LlmClient` for the harness (OPENPENCIL_SMOKE_DIRECT=1).
///
/// The default [`SmokeLlmClient`] goes through the vendored `agent` QueryEngine,
/// which can't send the provider-specific reasoning controls — so a reasoning
/// model thinks itself out of budget. This client does a plain non-streaming
/// POST and applies the SAME control production applies (via
/// [`apply_reasoning_wire_control`], shared with
/// `chat_builtin_http::run_openai_chat`), so a thinking-reduced arm can be
/// validated end-to-end headless (no GUI, no submodule edit).
pub(crate) struct DirectOpenAiClient {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) default_model: String,
}

impl LlmClient for DirectOpenAiClient {
    fn call(
        &self,
        req: CallRequest,
    ) -> futures::stream::BoxStream<'static, Result<LlmChunk, LlmError>> {
        let (tx, rx) = mpsc::unbounded::<Result<LlmChunk, LlmError>>();
        if req.abort.is_set() {
            let _ = tx.unbounded_send(Err(LlmError {
                message: "aborted".into(),
                aborted: true,
            }));
            return Box::pin(rx);
        }
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let key = self.api_key.clone();
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());
        let system = req.system_prompt.clone();
        let user = req.user_prompt.clone();
        let max_tokens: u32 = std::env::var("OPENPENCIL_SMOKE_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(16384);
        let dump = std::env::var("OPENPENCIL_SMOKE_DUMP").is_ok();
        eprintln!(
            "[LLM] direct call: model={model} system_len={} user_len={}",
            system.len(),
            user.len()
        );
        tokio::spawn(async move {
            let mut body = serde_json::json!({
                "model": model,
                "stream": false,
                "max_tokens": max_tokens,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": user },
                ],
            });
            // Optional temperature override: read OPENPENCIL_LLM_TEMPERATURE
            // (f32 in range 0.0..=2.0). If set and valid, add to body; else
            // omit (preserve provider default).
            if let Ok(temp_str) = std::env::var("OPENPENCIL_LLM_TEMPERATURE") {
                if let Ok(temp) = temp_str.parse::<f32>() {
                    if (0.0..=2.0).contains(&temp) {
                        body["temperature"] = serde_json::json!(temp);
                    }
                }
            }
            // Reasoning models burn their whole output budget on thinking and
            // return truncated (or empty) JSON, so a design turn asks for it
            // reduced. Both the DECISION and the wire shape are production's:
            // `resolve_model_profile(...).thinking_disabled` is what
            // `design_turn_thinking_mode` reads, and
            // `apply_reasoning_wire_control` is the exact function
            // `chat_builtin_http` calls — MiniMax / GLM / DeepSeek / K2.5-2.6
            // get `thinking:{type:"disabled"}`, Kimi K3 gets the top-level
            // `reasoning_effort:"low"` it demands instead (sending `thinking`
            // to K3 is a 400).
            apply_reasoning_wire_control(&mut body, &model, reduce_reasoning_for_smoke(&model));
            // Connect + overall deadlines so a hung provider endpoint surfaces
            // as an error instead of pinning the headless harness forever
            // (mirrors the desktop's builtin_http_client).
            let client = reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            let resp = match client.post(&url).bearer_auth(&key).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.unbounded_send(Err(LlmError {
                        message: format!("POST {url}: {e}"),
                        aborted: false,
                    }));
                    return;
                }
            };
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                let head: String = text.chars().take(300).collect();
                let _ = tx.unbounded_send(Err(LlmError {
                    message: format!("http {status}: {head}"),
                    aborted: false,
                }));
                return;
            }
            let content = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| {
                    v["choices"][0]["message"]["content"]
                        .as_str()
                        .map(str::to_string)
                })
                .unwrap_or_default();
            if dump {
                eprintln!(
                    "[DUMP] ===== LLM response ({} chars) =====\n{content}\n[DUMP] ===== end =====",
                    content.len()
                );
            }
            if content.trim().is_empty() {
                let _ = tx.unbounded_send(Err(LlmError {
                    message: "empty content from provider".into(),
                    aborted: false,
                }));
            } else {
                let _ = tx.unbounded_send(Ok(LlmChunk::Text(content)));
            }
        });
        Box::pin(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// What the harness would put on the wire for `model`.
    fn harness_body(model: &str) -> serde_json::Value {
        let mut body = json!({ "model": model });
        apply_reasoning_wire_control(&mut body, model, reduce_reasoning(model, false, false));
        body
    }

    /// What the live design turn puts on the wire for `model`:
    /// `design_turn_thinking_mode` forces thinking off exactly when the profile
    /// declares it, and `chat_builtin_http` hands that flag to the same helper.
    fn production_body(model: &str) -> serde_json::Value {
        let mut body = json!({ "model": model });
        apply_reasoning_wire_control(
            &mut body,
            model,
            op_orchestrator::resolve_model_profile(model).thinking_disabled,
        );
        body
    }

    /// The harness is a benchmark: a control it sends that production does not
    /// (or a shape production would never send) makes every number it produces
    /// unattributable. This drifted twice already — a private capability table,
    /// then a private JSON shape.
    #[test]
    fn harness_sends_the_same_reasoning_control_as_a_live_design_turn() {
        for model in [
            "kimi-k3",
            "moonshot/kimi-k3",
            "kimi-k2.6",
            "glm-5.2",
            "ark/glm-5.1",
            "MiniMax-M3",
            "deepseek-v4-pro",
            "gpt-5.6-sol",
            "claude-opus-5",
            "qwen3-coder-plus",
        ] {
            assert_eq!(
                harness_body(model),
                production_body(model),
                "harness and production disagree for {model}"
            );
        }
    }

    /// Kimi K3 rejects `thinking` outright (`cannot specify both 'thinking'
    /// and 'reasoning_effort'`), so the harness must not be able to send it —
    /// not even through `OPENPENCIL_SMOKE_DISABLE_THINKING=1`, which used to
    /// force that exact field for every model.
    #[test]
    fn forcing_reduction_never_sends_kimi_k3_the_field_it_rejects() {
        let mut body = json!({ "model": "kimi-k3" });
        apply_reasoning_wire_control(
            &mut body,
            "kimi-k3",
            reduce_reasoning("kimi-k3", true, false),
        );
        assert_eq!(body["reasoning_effort"], json!("low"));
        assert!(body.get("thinking").is_none(), "{body}");

        // …and the opt-out still keeps reasoning fully on.
        let mut body = json!({ "model": "kimi-k3" });
        apply_reasoning_wire_control(
            &mut body,
            "kimi-k3",
            reduce_reasoning("kimi-k3", true, true),
        );
        assert_eq!(body, json!({ "model": "kimi-k3" }));
    }
}

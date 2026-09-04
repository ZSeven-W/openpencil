//! Bounded model-list discovery for configured built-in providers.
//!
//! Provider defaults remain useful offline fallbacks. This module only adds a
//! runtime catalog and deliberately keeps credentials, response bodies, and
//! endpoint URLs out of every public error.

use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

use futures::StreamExt;
use op_editor_core::{
    builtin_agent_preset, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey,
};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;

/// Upper bound for resolving, connecting, receiving, and parsing one model
/// catalog. Hosts that provide a screened HTTP client wrap their client setup
/// and [`execute_builtin_model_discovery`] in the same deadline.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_MODELS: usize = 512;
const MAX_MODEL_TEXT_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinModelOption {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinModelCatalog {
    pub models: Vec<BuiltinModelOption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinModelDiscoveryError {
    InvalidConfiguration,
    UnsupportedEndpoint,
    RequestFailed,
    ResponseTooLarge,
    InvalidResponse,
}

impl BuiltinModelDiscoveryError {
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::UnsupportedEndpoint)
    }
}

impl fmt::Display for BuiltinModelDiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidConfiguration => "model discovery configuration is invalid",
            Self::UnsupportedEndpoint => "provider does not expose model discovery",
            Self::RequestFailed => "model discovery request failed",
            Self::ResponseTooLarge => "model discovery response exceeded the size limit",
            Self::InvalidResponse => "provider returned an invalid model catalog",
        })
    }
}

impl std::error::Error for BuiltinModelDiscoveryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseShape {
    Generic,
    Gemini,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthScheme {
    Anthropic,
    Bearer,
    Gemini,
}

/// Prepared request whose credential-bearing headers stay private to this
/// crate. Host adapters may inspect only the endpoint in order to construct a
/// connect-time screened HTTP client.
pub struct BuiltinModelDiscoveryRequest {
    url: reqwest::Url,
    headers: HeaderMap,
    shape: ResponseShape,
}

impl BuiltinModelDiscoveryRequest {
    pub fn endpoint(&self) -> &str {
        self.url.as_str()
    }
}

/// Discover through a trusted, locally configured provider endpoint.
///
/// Redirects are disabled so a provider cannot move a credential-bearing
/// request onto another origin. The response body, model count, model text,
/// and total request duration are bounded below.
pub async fn discover_builtin_models(
    config: &BuiltinAgentConfig,
) -> Result<BuiltinModelCatalog, BuiltinModelDiscoveryError> {
    tokio::time::timeout(REQUEST_TIMEOUT, discover_trusted_with_deadline(config))
        .await
        .map_err(|_| BuiltinModelDiscoveryError::RequestFailed)?
}

async fn discover_trusted_with_deadline(
    config: &BuiltinAgentConfig,
) -> Result<BuiltinModelCatalog, BuiltinModelDiscoveryError> {
    let request = prepare_builtin_model_discovery(config)?;
    let client = trusted_http_client()?;
    execute_builtin_model_discovery(request, client).await
}

/// Prepare the provider-specific URL and authentication headers. Secrets are
/// never exposed through the public request surface or any error text.
pub fn prepare_builtin_model_discovery(
    config: &BuiltinAgentConfig,
) -> Result<BuiltinModelDiscoveryRequest, BuiltinModelDiscoveryError> {
    discovery_request(config)
}

/// Execute a prepared request with a host-owned HTTP client. This lets the web
/// daemon keep its DNS-rebinding guard while mobile and desktop trusted
/// settings reuse exactly the same authentication and bounded parser.
pub async fn execute_builtin_model_discovery(
    request: BuiltinModelDiscoveryRequest,
    client: reqwest::Client,
) -> Result<BuiltinModelCatalog, BuiltinModelDiscoveryError> {
    let response = client
        .get(request.url)
        .headers(request.headers)
        .send()
        .await
        .map_err(|_| BuiltinModelDiscoveryError::RequestFailed)?;

    if matches!(response.status().as_u16(), 404 | 405) {
        return Err(BuiltinModelDiscoveryError::UnsupportedEndpoint);
    }
    if !response.status().is_success() {
        return Err(BuiltinModelDiscoveryError::RequestFailed);
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_BODY_BYTES as u64)
    {
        return Err(BuiltinModelDiscoveryError::ResponseTooLarge);
    }

    let body = read_bounded(response).await?;
    parse_catalog(&body, request.shape)
}

fn trusted_http_client() -> Result<reqwest::Client, BuiltinModelDiscoveryError> {
    reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(REQUEST_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|_| BuiltinModelDiscoveryError::RequestFailed)
}

async fn read_bounded(response: reqwest::Response) -> Result<Vec<u8>, BuiltinModelDiscoveryError> {
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| BuiltinModelDiscoveryError::RequestFailed)?;
        if bytes.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
            return Err(BuiltinModelDiscoveryError::ResponseTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn discovery_request(
    config: &BuiltinAgentConfig,
) -> Result<BuiltinModelDiscoveryRequest, BuiltinModelDiscoveryError> {
    let base = parse_base_url(&config.base_url)?;
    let preset = builtin_agent_preset(config.preset);
    let is_preset_base = [Some(preset.base_url), preset.alt_base_url]
        .into_iter()
        .flatten()
        .any(|candidate| same_normalized_url(candidate, config.base_url.trim()));

    let (url, shape, auth) = if is_preset_base && config.preset != BuiltinAgentPresetKey::Custom {
        let (url, shape) = official_discovery_url(base, config.preset)?;
        let auth = match config.preset {
            BuiltinAgentPresetKey::Anthropic => AuthScheme::Anthropic,
            BuiltinAgentPresetKey::Gemini => AuthScheme::Gemini,
            _ => AuthScheme::Bearer,
        };
        (url, shape, auth)
    } else {
        (
            append_models(base)?,
            ResponseShape::Generic,
            if config.kind == BuiltinAgentKind::Anthropic {
                AuthScheme::Anthropic
            } else {
                AuthScheme::Bearer
            },
        )
    };
    let headers = authentication_headers(&config.api_key, auth)?;
    Ok(BuiltinModelDiscoveryRequest {
        url,
        headers,
        shape,
    })
}

fn parse_base_url(value: &str) -> Result<reqwest::Url, BuiltinModelDiscoveryError> {
    let url = reqwest::Url::parse(value.trim())
        .map_err(|_| BuiltinModelDiscoveryError::InvalidConfiguration)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(BuiltinModelDiscoveryError::InvalidConfiguration);
    }
    Ok(url)
}

fn official_discovery_url(
    mut url: reqwest::Url,
    preset: BuiltinAgentPresetKey,
) -> Result<(reqwest::Url, ResponseShape), BuiltinModelDiscoveryError> {
    let (path, query, shape) = match preset {
        BuiltinAgentPresetKey::Anthropic => {
            ("/v1/models", Some("limit=1000"), ResponseShape::Generic)
        }
        BuiltinAgentPresetKey::OpenAi => ("/v1/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::OpenRouter => (
            "/api/v1/models/user",
            Some("supported_parameters=tools"),
            ResponseShape::Generic,
        ),
        BuiltinAgentPresetKey::DeepSeek => ("/v1/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::Gemini => (
            "/v1beta/models",
            Some("pageSize=1000"),
            ResponseShape::Gemini,
        ),
        BuiltinAgentPresetKey::MiniMax => ("/v1/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::Zhipu => ("/api/paas/v4/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::GlmCoding => {
            ("/api/coding/paas/v4/models", None, ResponseShape::Generic)
        }
        BuiltinAgentPresetKey::Kimi => ("/v1/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::Bailian => {
            ("/compatible-mode/v1/models", None, ResponseShape::Generic)
        }
        BuiltinAgentPresetKey::BailianCoding => ("/v1/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::Doubao => ("/api/v3/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::ArkCoding => ("/api/coding/v3/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::Xiaomi => ("/v1/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::ModelScope => ("/v1/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::StepFun => ("/v1/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::StepFunCoding => {
            ("/step_plan/v1/models", None, ResponseShape::Generic)
        }
        BuiltinAgentPresetKey::Nvidia => ("/v1/models", None, ResponseShape::Generic),
        BuiltinAgentPresetKey::Custom => return Ok((append_models(url)?, ResponseShape::Generic)),
    };
    url.set_path(path);
    url.set_query(query);
    Ok((url, shape))
}

fn append_models(mut url: reqwest::Url) -> Result<reqwest::Url, BuiltinModelDiscoveryError> {
    let base_path = url.path().trim_end_matches('/');
    let path = if base_path.is_empty() {
        "/models".to_string()
    } else {
        format!("{base_path}/models")
    };
    url.set_path(&path);
    Ok(url)
}

fn same_normalized_url(left: &str, right: &str) -> bool {
    left.trim().trim_end_matches('/') == right.trim().trim_end_matches('/')
}

fn authentication_headers(
    api_key: &str,
    scheme: AuthScheme,
) -> Result<HeaderMap, BuiltinModelDiscoveryError> {
    let key = api_key.trim();
    let mut headers = HeaderMap::new();
    if key.is_empty() {
        return Ok(headers);
    }
    let mut value =
        HeaderValue::from_str(key).map_err(|_| BuiltinModelDiscoveryError::InvalidConfiguration)?;
    value.set_sensitive(true);
    match scheme {
        AuthScheme::Anthropic => {
            headers.insert("x-api-key", value);
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        }
        AuthScheme::Gemini => {
            headers.insert("x-goog-api-key", value);
        }
        AuthScheme::Bearer => {
            let mut bearer = HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|_| BuiltinModelDiscoveryError::InvalidConfiguration)?;
            bearer.set_sensitive(true);
            headers.insert(AUTHORIZATION, bearer);
        }
    }
    Ok(headers)
}

fn parse_catalog(
    body: &[u8],
    shape: ResponseShape,
) -> Result<BuiltinModelCatalog, BuiltinModelDiscoveryError> {
    let value: Value =
        serde_json::from_slice(body).map_err(|_| BuiltinModelDiscoveryError::InvalidResponse)?;
    let items = match shape {
        ResponseShape::Generic => value.get("data").and_then(Value::as_array),
        ResponseShape::Gemini => value.get("models").and_then(Value::as_array),
    }
    .ok_or(BuiltinModelDiscoveryError::InvalidResponse)?;

    let mut seen = HashSet::new();
    let mut models = Vec::new();
    for item in items {
        if models.len() == MAX_MODELS {
            break;
        }
        if shape == ResponseShape::Gemini
            && item
                .get("supportedGenerationMethods")
                .is_some_and(|methods| {
                    !methods.as_array().is_some_and(|methods| {
                        methods.iter().any(|method| method == "generateContent")
                    })
                })
        {
            continue;
        }
        let raw_id = if shape == ResponseShape::Gemini {
            item.get("baseModelId")
                .and_then(Value::as_str)
                .or_else(|| item.get("name").and_then(Value::as_str))
                .map(|id| id.strip_prefix("models/").unwrap_or(id))
        } else {
            item.get("id").and_then(Value::as_str)
        };
        let Some(id) = raw_id.map(str::trim).filter(|id| valid_model_text(id)) else {
            continue;
        };
        if explicit_non_chat_item(item)
            || explicit_non_chat_model(id)
            || !seen.insert(id.to_string())
        {
            continue;
        }
        let display_name = item
            .get("display_name")
            .or_else(|| item.get("displayName"))
            .or_else(|| item.get("name"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| valid_model_text(name))
            .unwrap_or(id);
        models.push(BuiltinModelOption {
            id: id.to_string(),
            display_name: display_name.to_string(),
        });
    }
    if models.is_empty() {
        Err(BuiltinModelDiscoveryError::InvalidResponse)
    } else {
        Ok(BuiltinModelCatalog { models })
    }
}

fn valid_model_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_MODEL_TEXT_BYTES && !value.chars().any(char::is_control)
}

fn explicit_non_chat_model(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    let leaf = lower.rsplit('/').next().unwrap_or(&lower);
    let contains_non_chat_marker = [
        "embed",
        "bge-",
        "rerank",
        "moderation",
        "guard",
        "safety",
        "reward",
        "detector",
        "whisper",
        "tts",
        "speech",
        "transcri",
        "image",
        "vision-embed",
        "ocr",
        "dall-e",
        "sora",
        "realtime",
        "audio",
        "asr",
        "stable-diffusion",
        "flux",
        "nvclip",
        "deplot",
        "parse",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    contains_non_chat_marker
        || leaf.starts_with("clip-")
        || leaf.contains("-clip-")
        || leaf.ends_with("-base")
        || leaf.contains("-base-")
        || leaf.ends_with("-pt")
        || leaf.contains("-pt-")
}

fn explicit_non_chat_item(item: &Value) -> bool {
    ["task", "pipeline_tag", "type"]
        .iter()
        .filter_map(|field| item.get(*field).and_then(Value::as_str))
        .any(explicit_non_chat_model)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(preset: BuiltinAgentPresetKey) -> BuiltinAgentConfig {
        let source = builtin_agent_preset(preset);
        BuiltinAgentConfig {
            id: "test".into(),
            preset,
            display_name: source.display_name.into(),
            kind: source.kind,
            api_key: "top-secret-key".into(),
            models: vec![source.model.into()],
            base_url: source.base_url.into(),
            enabled: true,
        }
    }

    #[test]
    fn official_policy_uses_native_gemini_catalog() {
        let request = discovery_request(&config(BuiltinAgentPresetKey::Gemini)).unwrap();
        assert_eq!(
            request.url.as_str(),
            "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000"
        );
        assert_eq!(request.shape, ResponseShape::Gemini);
        assert!(request.headers.contains_key("x-goog-api-key"));
        assert!(!request.headers.contains_key(AUTHORIZATION));
    }

    #[test]
    fn every_preset_has_a_bounded_same_origin_policy() {
        let cases = [
            (
                BuiltinAgentPresetKey::Anthropic,
                "https://api.anthropic.com/v1/models?limit=1000",
            ),
            (
                BuiltinAgentPresetKey::OpenAi,
                "https://api.openai.com/v1/models",
            ),
            (
                BuiltinAgentPresetKey::OpenRouter,
                "https://openrouter.ai/api/v1/models/user?supported_parameters=tools",
            ),
            (
                BuiltinAgentPresetKey::DeepSeek,
                "https://api.deepseek.com/v1/models",
            ),
            (
                BuiltinAgentPresetKey::Gemini,
                "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000",
            ),
            (
                BuiltinAgentPresetKey::MiniMax,
                "https://api.minimaxi.com/v1/models",
            ),
            (
                BuiltinAgentPresetKey::Zhipu,
                "https://open.bigmodel.cn/api/paas/v4/models",
            ),
            (
                BuiltinAgentPresetKey::GlmCoding,
                "https://open.bigmodel.cn/api/coding/paas/v4/models",
            ),
            (
                BuiltinAgentPresetKey::Kimi,
                "https://api.moonshot.cn/v1/models",
            ),
            (
                BuiltinAgentPresetKey::Bailian,
                "https://dashscope.aliyuncs.com/compatible-mode/v1/models",
            ),
            (
                BuiltinAgentPresetKey::BailianCoding,
                "https://coding.dashscope.aliyuncs.com/v1/models",
            ),
            (
                BuiltinAgentPresetKey::Doubao,
                "https://ark.cn-beijing.volces.com/api/v3/models",
            ),
            (
                BuiltinAgentPresetKey::ArkCoding,
                "https://ark.cn-beijing.volces.com/api/coding/v3/models",
            ),
            (
                BuiltinAgentPresetKey::Xiaomi,
                "https://api.xiaomimimo.com/v1/models",
            ),
            (
                BuiltinAgentPresetKey::ModelScope,
                "https://api-inference.modelscope.cn/v1/models",
            ),
            (
                BuiltinAgentPresetKey::StepFun,
                "https://api.stepfun.com/v1/models",
            ),
            (
                BuiltinAgentPresetKey::StepFunCoding,
                "https://api.stepfun.com/step_plan/v1/models",
            ),
            (
                BuiltinAgentPresetKey::Nvidia,
                "https://integrate.api.nvidia.com/v1/models",
            ),
        ];
        for (preset, expected) in cases {
            let request = discovery_request(&config(preset)).unwrap();
            assert_eq!(request.url.as_str(), expected, "{}", preset.as_str());
            if preset == BuiltinAgentPresetKey::Anthropic {
                assert!(request.headers.contains_key("x-api-key"));
                assert!(request.headers.contains_key("anthropic-version"));
            } else if preset == BuiltinAgentPresetKey::Gemini {
                assert!(request.headers.contains_key("x-goog-api-key"));
            } else {
                assert!(request.headers.contains_key(AUTHORIZATION));
            }
        }

        let mut custom = config(BuiltinAgentPresetKey::Custom);
        custom.base_url = "https://models.example/api".into();
        let request = discovery_request(&custom).unwrap();
        assert_eq!(request.url.as_str(), "https://models.example/api/models");
        assert!(request.headers.contains_key(AUTHORIZATION));
    }

    #[test]
    fn modified_preset_base_stays_on_origin_and_appends_models() {
        let mut provider = config(BuiltinAgentPresetKey::OpenAi);
        provider.base_url = "https://gateway.example/openai/v9".into();
        let request = discovery_request(&provider).unwrap();
        assert_eq!(
            request.url.as_str(),
            "https://gateway.example/openai/v9/models"
        );
    }

    #[test]
    fn modified_gemini_base_uses_openai_compatible_auth() {
        let mut provider = config(BuiltinAgentPresetKey::Gemini);
        provider.base_url = "https://gateway.example/gemini/v1".into();
        let request = discovery_request(&provider).unwrap();
        assert!(request.headers.contains_key(AUTHORIZATION));
        assert!(!request.headers.contains_key("x-goog-api-key"));
    }

    #[test]
    fn base_url_rejects_credential_and_query_material() {
        for invalid in [
            "https://user:pass@example.com/v1",
            "https://example.com/v1?key=secret",
            "https://example.com/v1#fragment",
        ] {
            let mut provider = config(BuiltinAgentPresetKey::Custom);
            provider.base_url = invalid.into();
            assert_eq!(
                discovery_request(&provider).err().unwrap(),
                BuiltinModelDiscoveryError::InvalidConfiguration
            );
        }
    }

    #[test]
    fn generic_parser_deduplicates_filters_and_bounds_text() {
        let long = "x".repeat(MAX_MODEL_TEXT_BYTES + 1);
        let body = serde_json::json!({ "data": [
            {"id":"chat-a", "display_name":"Chat A"},
            {"id":"chat-a", "name":"duplicate"},
            {"id":"text-embedding-3-large"},
            {"id":long},
            {"id":"chat-b", "name":"Chat B"}
        ]});
        let catalog = parse_catalog(body.to_string().as_bytes(), ResponseShape::Generic).unwrap();
        assert_eq!(
            catalog.models,
            vec![
                BuiltinModelOption {
                    id: "chat-a".into(),
                    display_name: "Chat A".into()
                },
                BuiltinModelOption {
                    id: "chat-b".into(),
                    display_name: "Chat B".into()
                },
            ]
        );
    }

    #[test]
    fn generic_parser_rejects_known_catalog_rows_that_cannot_use_chat_completions() {
        let body = serde_json::json!({ "data": [
            {"id":"baai/bge-m3"},
            {"id":"nvidia/nv-embedcode-7b-v1"},
            {"id":"nvidia/nv-embedqa-e5-v5"},
            {"id":"nvidia/nvclip"},
            {"id":"nvidia/deplot"},
            {"id":"nvidia/content-safety-model"},
            {"id":"vendor/foundation-base"},
            {"id":"vendor/pretrained-pt"},
            {"id":"vendor/chat-capable", "task":"embedding"},
            {"id":"nvidia/llama-3.3-nemotron-super-49b-v1"}
        ]});

        let catalog = parse_catalog(body.to_string().as_bytes(), ResponseShape::Generic).unwrap();

        assert_eq!(catalog.models.len(), 1);
        assert_eq!(
            catalog.models[0].id,
            "nvidia/llama-3.3-nemotron-super-49b-v1"
        );
    }

    #[test]
    fn gemini_parser_filters_explicit_non_generation_models() {
        let body = br#"{"models":[
            {"name":"models/gemini-chat","displayName":"Gemini Chat","supportedGenerationMethods":["generateContent"]},
            {"name":"models/gemini-embed","supportedGenerationMethods":["embedContent"]}
        ]}"#;
        let catalog = parse_catalog(body, ResponseShape::Gemini).unwrap();
        assert_eq!(catalog.models.len(), 1);
        assert_eq!(catalog.models[0].id, "gemini-chat");
    }

    #[test]
    fn gemini_parser_keeps_models_when_generation_methods_are_absent() {
        let body = br#"{"models":[
            {"name":"models/gemini-legacy","displayName":"Gemini Legacy"}
        ]}"#;
        let catalog = parse_catalog(body, ResponseShape::Gemini).unwrap();
        assert_eq!(catalog.models.len(), 1);
        assert_eq!(catalog.models[0].id, "gemini-legacy");
    }

    #[test]
    fn empty_or_fully_filtered_catalog_is_invalid() {
        for body in [
            r#"{"data":[]}"#,
            r#"{"data":[{"id":"text-embedding-3-small"}]}"#,
        ] {
            assert_eq!(
                parse_catalog(body.as_bytes(), ResponseShape::Generic).unwrap_err(),
                BuiltinModelDiscoveryError::InvalidResponse
            );
        }
    }

    #[test]
    fn errors_never_echo_secrets_or_transport_details() {
        let secret = "top-secret-key";
        for error in [
            BuiltinModelDiscoveryError::InvalidConfiguration,
            BuiltinModelDiscoveryError::UnsupportedEndpoint,
            BuiltinModelDiscoveryError::RequestFailed,
            BuiltinModelDiscoveryError::ResponseTooLarge,
            BuiltinModelDiscoveryError::InvalidResponse,
        ] {
            let rendered = error.to_string();
            assert!(!rendered.contains(secret));
            assert!(!rendered.contains("http"));
        }
        assert!(BuiltinModelDiscoveryError::UnsupportedEndpoint.is_unsupported());
        assert!(!BuiltinModelDiscoveryError::RequestFailed.is_unsupported());
    }
}

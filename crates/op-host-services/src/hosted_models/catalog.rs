use std::collections::HashSet;

use futures::StreamExt;
use op_auth_bridge::HostedAccess;
use op_editor_core::{HostedModelCapabilities, HostedModelRoute, HostedProtocol};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::HostedRuntimeError;

pub(super) const MAX_CATALOG_BYTES: usize = 128 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    object: String,
    data: Vec<Model>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    id: String,
    display_name: String,
    object: String,
    created: u64,
    owned_by: String,
    protocol: String,
    #[serde(default)]
    capabilities: Vec<String>,
}

pub(super) fn parse(
    bytes: &[u8],
    namespace: &str,
) -> Result<Vec<HostedModelRoute>, HostedRuntimeError> {
    if bytes.len() > MAX_CATALOG_BYTES {
        return Err(HostedRuntimeError::CatalogTooLarge);
    }
    let catalog: Catalog =
        serde_json::from_slice(bytes).map_err(|_| HostedRuntimeError::CatalogInvalid)?;
    if catalog.object != "list" || catalog.data.len() > op_editor_core::MAX_HOSTED_MODEL_ROUTES {
        return Err(HostedRuntimeError::CatalogInvalid);
    }
    let mut ids = HashSet::new();
    let mut routes = Vec::new();
    for model in catalog.data {
        if model.object != "model"
            || model.owned_by.len() > 128
            || model.created > i64::MAX as u64
            || model.capabilities.len() > 32
            || !ids.insert(model.id.clone())
        {
            return Err(HostedRuntimeError::CatalogInvalid);
        }
        let protocol = match model.protocol.as_str() {
            "openai_chat_completions" => HostedProtocol::OpenAiCompatible,
            "anthropic_messages" => HostedProtocol::AnthropicMessages,
            "openai_responses" => HostedProtocol::OpenAiResponses,
            _ => return Err(HostedRuntimeError::UnsupportedProtocol),
        };
        if model
            .capabilities
            .iter()
            .any(|value| value.len() > 64 || value.chars().any(char::is_control))
        {
            return Err(HostedRuntimeError::CatalogInvalid);
        }
        let has = |name: &str| model.capabilities.iter().any(|value| value == name);
        let digest = Sha256::digest(format!("{namespace}\0{}\0{}", model.protocol, model.id));
        let route_id = format!(
            "hosted-{}",
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        // Intersect server capabilities with this host's implemented transport.
        let capabilities = HostedModelCapabilities {
            image_input: false,
            tools: false,
            structured_output: false,
            reasoning: has("reasoning"),
        };
        routes.push(
            HostedModelRoute::new(
                route_id,
                model.id,
                model.display_name,
                protocol,
                capabilities,
            )
            .map_err(|_| HostedRuntimeError::CatalogInvalid)?,
        );
    }
    Ok(routes)
}

pub(super) async fn fetch(
    access: &HostedAccess,
    namespace: &str,
) -> Result<Vec<HostedModelRoute>, HostedRuntimeError> {
    let client = super::client()?;
    let response = client
        .get(format!("{}/models", access.gateway_base_url()))
        .bearer_auth(access.bearer())
        .send()
        .await
        .map_err(|_| HostedRuntimeError::Network)?;
    if !response.status().is_success() {
        return Err(HostedRuntimeError::GatewayRejected);
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_CATALOG_BYTES as u64)
    {
        return Err(HostedRuntimeError::CatalogTooLarge);
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| HostedRuntimeError::Network)?;
        if bytes.len().saturating_add(chunk.len()) > MAX_CATALOG_BYTES {
            return Err(HostedRuntimeError::CatalogTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    parse(&bytes, namespace)
}

pub(super) fn protocol_wire(protocol: HostedProtocol) -> &'static str {
    match protocol {
        HostedProtocol::OpenAiCompatible => "openai-compatible",
        HostedProtocol::AnthropicMessages => "anthropic-messages",
        HostedProtocol::OpenAiResponses => "openai-responses",
    }
}

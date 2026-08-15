//! Host-policy adapter for portable built-in-provider model discovery.
//!
//! Authentication, provider URL construction, response bounds, and catalog
//! parsing live in `op-builtin-model-discovery` so desktop and mobile use the
//! same implementation. This wrapper adds the web daemon's connect-time DNS
//! screening for browser-originated credentials.

use op_editor_core::BuiltinAgentConfig;

pub use op_builtin_model_discovery::{
    BuiltinModelCatalog, BuiltinModelDiscoveryError, BuiltinModelOption,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinModelAccess {
    Trusted,
    PublicOnly,
}

pub async fn discover_builtin_models(
    config: &BuiltinAgentConfig,
    access: BuiltinModelAccess,
) -> Result<BuiltinModelCatalog, BuiltinModelDiscoveryError> {
    tokio::time::timeout(
        op_builtin_model_discovery::REQUEST_TIMEOUT,
        discover_with_deadline(config, access),
    )
    .await
    .map_err(|_| BuiltinModelDiscoveryError::RequestFailed)?
}

async fn discover_with_deadline(
    config: &BuiltinAgentConfig,
    access: BuiltinModelAccess,
) -> Result<BuiltinModelCatalog, BuiltinModelDiscoveryError> {
    let request = op_builtin_model_discovery::prepare_builtin_model_discovery(config)?;
    let dial_policy = match access {
        BuiltinModelAccess::Trusted => crate::provider_dial::EndpointDialPolicy::Trusted,
        BuiltinModelAccess::PublicOnly => {
            crate::web_credentials::validate_web_provider_base_url(&config.base_url)
                .map_err(|_| BuiltinModelDiscoveryError::InvalidConfiguration)?;
            let allowlist =
                std::env::var(crate::web_credentials::WEB_AI_ENDPOINT_ALLOWLIST_ENV).ok();
            crate::provider_dial::web_dial_policy_for(config.base_url.trim(), allowlist.as_deref())
        }
    };
    let client = crate::provider_dial::client_for(dial_policy, request.endpoint())
        .await
        .map_err(|_| BuiltinModelDiscoveryError::RequestFailed)?;
    op_builtin_model_discovery::execute_builtin_model_discovery(request, client).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::{BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey};

    fn config(base_url: &str) -> BuiltinAgentConfig {
        BuiltinAgentConfig {
            id: "provider".into(),
            preset: BuiltinAgentPresetKey::Custom,
            display_name: "Provider".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: "sk-secret".into(),
            models: vec!["saved-model".into()],
            base_url: base_url.into(),
            enabled: true,
        }
    }

    #[test]
    fn trusted_adapter_uses_the_portable_configuration_guard() {
        let result = crate::chat_runtime::block_on_anywhere(discover_builtin_models(
            &config("https://user:password@example.com/v1"),
            BuiltinModelAccess::Trusted,
        ));
        assert_eq!(
            result.unwrap_err(),
            BuiltinModelDiscoveryError::InvalidConfiguration
        );
    }

    #[test]
    fn public_adapter_rejects_private_browser_endpoints_before_dialing() {
        let result = crate::chat_runtime::block_on_anywhere(discover_builtin_models(
            &config("http://127.0.0.1:11434/v1"),
            BuiltinModelAccess::PublicOnly,
        ));
        assert_eq!(
            result.unwrap_err(),
            BuiltinModelDiscoveryError::InvalidConfiguration
        );
    }
}

//! Configured-provider value types for the agent-settings modal:
//! built-in (API-key) agents, ACP-compatible external agents, and
//! image-generation profiles. Plain data + small helpers, no widget or
//! platform coupling. Carved off `agent_settings.rs` to keep every file
//! under the 800-line cap; re-exported from the spine so call sites keep
//! the `agent_settings::…` path.

use std::collections::BTreeMap;

use super::ImageTestStatus;
use crate::agent_settings_builtin_presets::{builtin_agent_preset, BuiltinAgentPresetKey};
use crate::chat::AgentProvider;

/// Built-in provider backend configured directly in OpenPencil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinAgentKind {
    Anthropic,
    OpenAiCompat,
}

impl BuiltinAgentKind {
    pub fn default_base_url(self) -> &'static str {
        match self {
            BuiltinAgentKind::Anthropic => "https://api.anthropic.com",
            BuiltinAgentKind::OpenAiCompat => "https://api.openai.com/v1",
        }
    }

    pub fn model_provider(self) -> AgentProvider {
        match self {
            BuiltinAgentKind::Anthropic => AgentProvider::ClaudeCode,
            BuiltinAgentKind::OpenAiCompat => AgentProvider::CodexCli,
        }
    }
}

/// One configured built-in Agent/API-key provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinAgentConfig {
    pub id: String,
    pub preset: BuiltinAgentPresetKey,
    pub display_name: String,
    pub kind: BuiltinAgentKind,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub enabled: bool,
}

impl BuiltinAgentConfig {
    pub fn base_url_editable(&self) -> bool {
        !matches!(
            self.preset,
            BuiltinAgentPresetKey::Anthropic | BuiltinAgentPresetKey::OpenAi
        )
    }

    pub fn ready(&self) -> bool {
        self.enabled && !self.api_key.trim().is_empty() && !self.model.trim().is_empty()
    }

    pub fn matches_config(
        &self,
        display_name: &str,
        api_key: &str,
        model: &str,
        kind: BuiltinAgentKind,
        base_url: &str,
    ) -> bool {
        self.kind == kind
            && self.display_name.trim() == display_name.trim()
            && self.api_key.trim() == api_key.trim()
            && self.model.trim() == model.trim()
            && self.base_url.trim().trim_end_matches('/') == base_url.trim().trim_end_matches('/')
    }

    pub fn matches_add_candidate(
        &self,
        _display_name: &str,
        api_key: &str,
        model: &str,
        kind: BuiltinAgentKind,
        base_url: &str,
    ) -> bool {
        self.matches_backend(api_key, model, kind, base_url)
    }

    fn matches_backend(
        &self,
        api_key: &str,
        model: &str,
        kind: BuiltinAgentKind,
        base_url: &str,
    ) -> bool {
        self.kind == kind
            && self.api_key.trim() == api_key.trim()
            && self.model.trim() == model.trim()
            && self.base_url.trim().trim_end_matches('/') == base_url.trim().trim_end_matches('/')
    }

    pub fn apply_preset(&mut self, key: BuiltinAgentPresetKey) {
        let preset = builtin_agent_preset(key);
        self.preset = preset.key;
        self.display_name = preset.display_name.into();
        self.kind = preset.kind;
        self.model = preset.model.into();
        self.base_url = preset.base_url.into();
    }

    pub fn set_kind_for_preset(&mut self, kind: BuiltinAgentKind) {
        self.kind = kind;
        if let Some(base_url) = builtin_agent_preset(self.preset).base_url_for_kind(kind) {
            self.base_url = base_url.into();
        } else if self.preset != BuiltinAgentPresetKey::Custom {
            self.base_url = kind.default_base_url().into();
        }
        if kind == BuiltinAgentKind::OpenAiCompat && self.model.starts_with("claude-") {
            self.model = "gpt-5.4".into();
        } else if kind == BuiltinAgentKind::Anthropic && self.model.starts_with("gpt-") {
            self.model = crate::agent_settings_builtin_presets::DEFAULT_ANTHROPIC_MODEL.into();
        }
    }

    pub fn toggle_kind_for_preset(&mut self) {
        if builtin_agent_preset(self.preset).alt_kind.is_none() {
            return;
        }
        let next = match self.kind {
            BuiltinAgentKind::Anthropic => BuiltinAgentKind::OpenAiCompat,
            BuiltinAgentKind::OpenAiCompat => BuiltinAgentKind::Anthropic,
        };
        self.set_kind_for_preset(next);
    }
}

/// ACP-compatible agent connection style mirrored from the TS
/// `AcpAgentConfig.connectionType` union.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpConnectionType {
    Local,
    Remote,
}

/// One configured ACP-compatible external agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpAgentConfig {
    pub id: String,
    pub display_name: String,
    pub connection_type: AcpConnectionType,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub url: Option<String>,
    pub enabled: bool,
    pub connected: bool,
}

impl AcpAgentConfig {
    pub fn ready(&self) -> bool {
        self.enabled
            && match self.connection_type {
                AcpConnectionType::Local => !self.command.trim().is_empty(),
                AcpConnectionType::Remote => self
                    .url
                    .as_deref()
                    .map(|url| !url.trim().is_empty())
                    .unwrap_or(false),
            }
    }

    pub fn args_text(&self) -> String {
        self.args.join(", ")
    }

    pub fn env_text(&self) -> String {
        self.env
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn set_args_text(&mut self, text: &str) {
        self.args = text
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
    }

    pub fn set_env_text(&mut self, text: &str) {
        self.env = text
            .lines()
            .filter_map(|line| {
                let (key, value) = line.split_once('=')?;
                let key = key.trim();
                (!key.is_empty()).then(|| (key.to_string(), value.trim().to_string()))
            })
            .collect();
    }
}

/// Image-generation service providers mirrored from the TS
/// `ImageGenProvider` union.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageGenProvider {
    OpenAi,
    Gemini,
    Replicate,
    Atlas,
    Custom,
}

impl ImageGenProvider {
    pub const ALL: [ImageGenProvider; 5] = [
        ImageGenProvider::OpenAi,
        ImageGenProvider::Gemini,
        ImageGenProvider::Replicate,
        ImageGenProvider::Atlas,
        ImageGenProvider::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ImageGenProvider::OpenAi => "OpenAI",
            ImageGenProvider::Gemini => "Google Gemini",
            ImageGenProvider::Replicate => "Replicate",
            ImageGenProvider::Atlas => "Atlas Cloud",
            ImageGenProvider::Custom => "Custom",
        }
    }

    pub fn default_model_placeholder(self) -> &'static str {
        match self {
            ImageGenProvider::OpenAi => "dall-e-3",
            ImageGenProvider::Gemini => "gemini-2.0-flash-preview-image-generation",
            ImageGenProvider::Replicate => "black-forest-labs/flux-1.1-pro",
            ImageGenProvider::Atlas => "google/nano-banana-2-lite/text-to-image",
            ImageGenProvider::Custom => "model-name",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

/// One image-generation configuration profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageGenProfile {
    pub id: String,
    pub name: String,
    pub provider: ImageGenProvider,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
    pub test_status: ImageTestStatus,
}

/// Openverse credential-registration help page opened from the Images
/// tab's Register link. The raw `auth_tokens/register/` endpoint only
/// accepts POST, so opening it in a browser (GET) lands on a 405 page —
/// point at the API reference's auth section instead, which documents
/// how to register an application for credentials.
pub const OPENVERSE_AUTH_DOCS_URL: &str = "https://api.openverse.org/v1/#tag/auth";

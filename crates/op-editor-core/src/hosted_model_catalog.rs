//! Transient OpenPencil-hosted model catalog state.
//!
//! Hosted routes are installed by an authenticated host at runtime. They are
//! deliberately separate from saved API-key providers and external CLI agents:
//! this module carries no credential, endpoint, or persistence representation.

use crate::{AgentProvider, EditorState, EditorUiState};
use std::collections::HashSet;
use std::fmt;

/// Maximum hosted routes accepted from one authenticated catalog response.
pub const MAX_HOSTED_MODEL_ROUTES: usize = 128;
/// Maximum UTF-8 byte length of one stable hosted route id.
pub const MAX_HOSTED_ROUTE_ID_BYTES: usize = 128;
/// Maximum UTF-8 byte length of one upstream model id.
pub const MAX_HOSTED_MODEL_ID_BYTES: usize = 256;
/// Maximum UTF-8 byte length of one model label rendered by the picker.
pub const MAX_HOSTED_MODEL_DISPLAY_BYTES: usize = 128;

/// Honest host-side error shown while catalog projection exists but the
/// authenticated gateway transport has not been wired into that host yet.
pub fn hosted_transport_unavailable_message(locale: op_i18n::Locale) -> &'static str {
    op_i18n::translate(locale, "ai.hostedTransportUnavailable")
}

/// Wire protocol used by one OpenPencil-hosted route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostedProtocol {
    OpenAiCompatible,
    AnthropicMessages,
    OpenAiResponses,
}

impl HostedProtocol {
    /// Compatibility hint for legacy model rows. Hosted selection never
    /// changes `chat_selected_agent`; the route itself remains authoritative.
    pub(crate) fn agent_provider_hint(self) -> AgentProvider {
        match self {
            HostedProtocol::AnthropicMessages => AgentProvider::ClaudeCode,
            HostedProtocol::OpenAiCompatible | HostedProtocol::OpenAiResponses => {
                AgentProvider::CodexCli
            }
        }
    }
}

/// Bounded capability set understood by the editor's hosted-model picker.
/// Text chat is implicit because every entry is selectable in the chat UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HostedModelCapabilities {
    pub image_input: bool,
    pub tools: bool,
    pub structured_output: bool,
    pub reasoning: bool,
}

/// One validated route returned by the authenticated hosted catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedModelRoute {
    route_id: String,
    model_id: String,
    display_name: String,
    protocol: HostedProtocol,
    capabilities: HostedModelCapabilities,
}

impl HostedModelRoute {
    pub fn new(
        route_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: impl Into<String>,
        protocol: HostedProtocol,
        capabilities: HostedModelCapabilities,
    ) -> Result<Self, HostedCatalogError> {
        let route_id = route_id.into();
        let model_id = model_id.into();
        let display_name = display_name.into();
        validate_route_id(&route_id)?;
        validate_text(
            &model_id,
            MAX_HOSTED_MODEL_ID_BYTES,
            HostedCatalogField::ModelId,
        )?;
        validate_text(
            &display_name,
            MAX_HOSTED_MODEL_DISPLAY_BYTES,
            HostedCatalogField::DisplayName,
        )?;
        Ok(Self {
            route_id,
            model_id,
            display_name,
            protocol,
            capabilities,
        })
    }

    pub fn route_id(&self) -> &str {
        &self.route_id
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn protocol(&self) -> HostedProtocol {
        self.protocol
    }

    pub fn capabilities(&self) -> HostedModelCapabilities {
        self.capabilities
    }
}

/// Runtime-only catalog wrapper. Its route storage is private so callers must
/// use [`EditorState::replace_hosted_model_catalog`] or
/// [`EditorState::clear_hosted_model_catalog`], which also rebuild selection.
#[derive(Debug, Clone, Default)]
pub struct HostedModelCatalog {
    routes: Vec<HostedModelRoute>,
}

impl HostedModelCatalog {
    fn validated(routes: Vec<HostedModelRoute>) -> Result<Self, HostedCatalogError> {
        if routes.len() > MAX_HOSTED_MODEL_ROUTES {
            return Err(HostedCatalogError::TooManyRoutes {
                count: routes.len(),
                max: MAX_HOSTED_MODEL_ROUTES,
            });
        }
        let mut route_ids = HashSet::with_capacity(routes.len());
        for route in &routes {
            if !route_ids.insert(route.route_id()) {
                return Err(HostedCatalogError::DuplicateRouteId {
                    route_id: route.route_id().to_string(),
                });
            }
        }
        Ok(Self { routes })
    }

    pub fn routes(&self) -> &[HostedModelRoute] {
        &self.routes
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    fn clear(&mut self) {
        self.routes.clear();
    }
}

/// Catalog field named by a validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedCatalogField {
    RouteId,
    ModelId,
    DisplayName,
}

impl fmt::Display for HostedCatalogField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            HostedCatalogField::RouteId => "route id",
            HostedCatalogField::ModelId => "model id",
            HostedCatalogField::DisplayName => "display name",
        })
    }
}

/// Rejection reason for an untrusted hosted catalog payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostedCatalogError {
    NotSignedIn,
    TooManyRoutes {
        count: usize,
        max: usize,
    },
    EmptyField {
        field: HostedCatalogField,
    },
    FieldTooLong {
        field: HostedCatalogField,
        bytes: usize,
        max: usize,
    },
    InvalidCharacters {
        field: HostedCatalogField,
    },
    DuplicateRouteId {
        route_id: String,
    },
}

impl fmt::Display for HostedCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostedCatalogError::NotSignedIn => {
                f.write_str("hosted model catalog requires a signed-in account")
            }
            HostedCatalogError::TooManyRoutes { count, max } => {
                write!(
                    f,
                    "hosted model catalog has {count} routes; maximum is {max}"
                )
            }
            HostedCatalogError::EmptyField { field } => {
                write!(f, "hosted model {field} cannot be empty")
            }
            HostedCatalogError::FieldTooLong { field, bytes, max } => {
                write!(f, "hosted model {field} is {bytes} bytes; maximum is {max}")
            }
            HostedCatalogError::InvalidCharacters { field } => {
                write!(f, "hosted model {field} contains invalid characters")
            }
            HostedCatalogError::DuplicateRouteId { route_id } => {
                write!(f, "hosted model route id is duplicated: {route_id}")
            }
        }
    }
}

impl std::error::Error for HostedCatalogError {}

impl EditorUiState {
    /// Authenticated runtime routes currently eligible for chat catalog build.
    pub fn hosted_model_routes(&self) -> &[HostedModelRoute] {
        self.hosted_model_catalog.routes()
    }
}

impl EditorState {
    /// Atomically validate and replace the authenticated hosted catalog.
    /// Invalid payloads leave the previous valid catalog untouched.
    pub fn replace_hosted_model_catalog(
        &mut self,
        routes: Vec<HostedModelRoute>,
    ) -> Result<(), HostedCatalogError> {
        if !self.editor_ui.account.is_signed_in() {
            self.clear_hosted_model_catalog();
            return Err(HostedCatalogError::NotSignedIn);
        }
        let catalog = HostedModelCatalog::validated(routes)?;
        self.cancel_removed_hosted_pending_turns(catalog.routes());
        self.editor_ui.hosted_model_catalog = catalog;
        self.rebuild_chat_models();
        Ok(())
    }

    /// Drop all account-scoped hosted routes and rebuild a stable fallback.
    pub fn clear_hosted_model_catalog(&mut self) {
        self.cancel_removed_hosted_pending_turns(&[]);
        self.editor_ui.hosted_model_catalog.clear();
        self.rebuild_chat_models();
    }

    fn cancel_removed_hosted_pending_turns(&mut self, routes: &[HostedModelRoute]) {
        let active_tab = self.chat.active_index();
        for (index, tab) in self.chat.tabs_mut().iter_mut().enumerate() {
            let removed = tab
                .selected_model_entry()
                .and_then(|entry| entry.hosted.as_ref())
                .is_some_and(|selected| !routes.contains(selected));
            if !removed {
                continue;
            }
            // Code generation resolves the active chat selection at launch;
            // retry resolves its own tab. Neither queue may adopt the fallback.
            if index == active_tab {
                self.codegen.pending_generate = false;
                self.codegen.pending_regenerate = false;
            }
            if let Some((message_index, subtask_id)) = tab.pending_subtask_retry.take() {
                if let Some(activity) = tab.messages.get_mut(message_index).and_then(|message| {
                    message
                        .activities
                        .iter_mut()
                        .find(|activity| activity.id == subtask_id)
                }) {
                    if activity.status == crate::ChatActivityStatus::Running {
                        activity.status = crate::ChatActivityStatus::Error;
                    }
                }
            }
            // Catalog rebuilding can select a BYOK/CLI fallback. A queued
            // Hosted turn must be consumed before that identity disappears,
            // otherwise the next host drain sends it through the fallback.
            if tab.pending_send.take().is_some() {
                tab.pending_attachments.clear();
                if let Some(message) = tab
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|message| message.streaming)
                {
                    message.streaming = false;
                }
            }
        }
    }
}

fn validate_route_id(value: &str) -> Result<(), HostedCatalogError> {
    validate_text(
        value,
        MAX_HOSTED_ROUTE_ID_BYTES,
        HostedCatalogField::RouteId,
    )?;
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
    }) {
        return Err(HostedCatalogError::InvalidCharacters {
            field: HostedCatalogField::RouteId,
        });
    }
    Ok(())
}

fn validate_text(
    value: &str,
    max: usize,
    field: HostedCatalogField,
) -> Result<(), HostedCatalogError> {
    if value.trim().is_empty() {
        return Err(HostedCatalogError::EmptyField { field });
    }
    if value.len() > max {
        return Err(HostedCatalogError::FieldTooLong {
            field,
            bytes: value.len(),
            max,
        });
    }
    if value.trim() != value || value.chars().any(char::is_control) {
        return Err(HostedCatalogError::InvalidCharacters { field });
    }
    Ok(())
}

#[cfg(test)]
#[path = "hosted_model_catalog_pending_tests.rs"]
mod pending_tests;

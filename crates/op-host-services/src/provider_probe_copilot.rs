//! GitHub Copilot connect-time probe.

use std::path::PathBuf;

use op_ai::agent_settings_state::AgentProvider;
use op_i18n::Locale;

use super::{config_path, t, tw, ProbeOutcome};
use crate::copilot_sdk_probe::{probe_copilot_cli, CopilotAuth};
use crate::model_discovery::resolve_cli;

pub(super) fn connect_copilot(locale: Locale) -> ProbeOutcome {
    let Some(exe) = resolve_cli("copilot") else {
        return ProbeOutcome::not_installed(
            AgentProvider::GithubCopilot,
            tw(
                locale,
                "providerProbe.cliNotFound",
                &[("name", "GitHub Copilot")],
            ),
        );
    };
    let probe = match probe_copilot_cli(&exe) {
        Ok(probe) => probe,
        Err(error) => {
            return ProbeOutcome::failed(friendly_copilot_error(locale, &error.to_string()));
        }
    };
    let models = probe.models;
    let auth = probe.auth;
    if models.is_empty() {
        return ProbeOutcome::failed(t(locale, "providerProbe.noModelsCopilot"));
    }
    let hint = copilot_config_hint();
    let info = copilot_connection_info(locale, auth.as_ref());
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some(info),
        hint_path: Some(hint),
        ..ProbeOutcome::default()
    }
}

fn copilot_config_hint() -> String {
    std::env::var_os("COPILOT_HOME")
        .filter(|home| !home.is_empty())
        .map(|home| {
            PathBuf::from(home)
                .join("config.json")
                .display()
                .to_string()
        })
        .unwrap_or_else(|| {
            config_path(
                "~/.copilot/config.json",
                "%USERPROFILE%\\.copilot\\config.json",
            )
        })
}

/// TS connectCopilot's status mapping (connect-agent.ts:799-819).
pub(super) fn copilot_connection_info(locale: Locale, auth: Option<&CopilotAuth>) -> String {
    if let Some(auth) = auth {
        if let Some(login) = auth.login.as_deref() {
            let method = auth
                .auth_type
                .as_deref()
                .map(|t| format!(" ({t})"))
                .unwrap_or_default();
            return tw(
                locale,
                "providerProbe.connectedAs",
                &[("login", login), ("method", &method)],
            );
        }
        if let Some(message) = auth.status_message.as_deref() {
            return message.to_string();
        }
    }
    t(locale, "providerProbe.connectedViaGithub")
}

/// TS `friendlyCopilotError` (connect-agent.ts:842-853).
pub(super) fn friendly_copilot_error(locale: Locale, raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("not found") || lower.contains("enoent") {
        return t(locale, "providerProbe.copilotNotFoundInstall");
    }
    if lower.contains("not authenticated")
        || lower.contains("authenticate first")
        || lower.contains("auth")
        || lower.contains("unauthenticated")
        || lower.contains("login")
    {
        return t(locale, "providerProbe.notAuthenticatedCopilot");
    }
    if lower.contains("timed out") || lower.contains("timedout") {
        return t(locale, "providerProbe.timedOut");
    }
    raw.to_string()
}

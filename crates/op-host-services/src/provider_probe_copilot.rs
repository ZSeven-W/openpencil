//! GitHub Copilot connect-time probe.

use std::io::{BufRead, BufReader, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;
use op_i18n::Locale;

use super::{config_path, t, tw, ProbeOutcome};
use crate::model_discovery::{
    extract_json_object, parse_copilot_model_list, resolve_cli, write_lsp_frame,
};

/// Auth status from `auth.getStatus` — the wire method the official
/// copilot-sdk's `getAuthStatus()` sends (client.js:549-555).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CopilotAuth {
    pub(super) login: Option<String>,
    pub(super) auth_type: Option<String>,
    pub(super) status_message: Option<String>,
}

const COPILOT_PROBE_TIMEOUT: Duration = Duration::from_secs(8);

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
    let Some((models, auth)) = copilot_probe_stdio(&exe) else {
        return ProbeOutcome::failed(friendly_copilot_error(locale, "Connection timed out"));
    };
    if models.is_empty() {
        return ProbeOutcome::failed(t(locale, "providerProbe.noModelsCopilot"));
    }
    let hint = config_path(
        "~/.config/github-copilot/config.json",
        "%USERPROFILE%\\.config\\github-copilot\\config.json",
    );
    let info = copilot_connection_info(locale, auth.as_ref());
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some(info),
        hint_path: Some(hint),
        ..ProbeOutcome::default()
    }
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

/// One `copilot --stdio` session: `connect` (id 1) → `models.list`
/// (id 2) → `auth.getStatus` (id 3), all pipelined. Returns `None`
/// when the model list never answered; auth is best-effort (TS
/// logs and continues when `getAuthStatus` fails).
fn copilot_probe_stdio(exe: &Path) -> Option<(Vec<ModelEntry>, Option<CopilotAuth>)> {
    let mut cmd = Command::new(exe);
    cmd.arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    crate::chat_spawn::hide_console_window(&mut cmd);
    let mut child = cmd.spawn().ok()?;
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let connect = r#"{"jsonrpc":"2.0","id":1,"method":"connect","params":{}}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"models.list","params":{}}"#;
    let auth = r#"{"jsonrpc":"2.0","id":3,"method":"auth.getStatus","params":{}}"#;
    write_lsp_frame(&mut stdin, connect).ok()?;
    write_lsp_frame(&mut stdin, list).ok()?;
    write_lsp_frame(&mut stdin, auth).ok()?;
    stdin.flush().ok();

    let deadline = Instant::now() + COPILOT_PROBE_TIMEOUT;
    let mut models: Option<Vec<ModelEntry>> = None;
    let mut auth_status: Option<CopilotAuth> = None;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if models.is_some() && auth_status.is_some() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(line) => {
                if models.is_none() {
                    if let Some(parsed) = parse_copilot_model_list(&line) {
                        models = Some(parsed);
                        continue;
                    }
                }
                if auth_status.is_none() {
                    if let Some(parsed) = parse_copilot_auth_status(&line) {
                        auth_status = Some(parsed);
                    }
                }
            }
            Err(_) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    models.map(|m| (m, auth_status))
}

/// Parse the `id:3` (`auth.getStatus`) response.
pub(super) fn parse_copilot_auth_status(line: &str) -> Option<CopilotAuth> {
    let json: serde_json::Value = serde_json::from_str(extract_json_object(line)?).ok()?;
    if json.get("id")?.as_i64()? != 3 {
        return None;
    }
    let result = json.get("result")?;
    Some(CopilotAuth {
        login: result
            .get("login")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        auth_type: result
            .get("authType")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        status_message: result
            .get("statusMessage")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
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

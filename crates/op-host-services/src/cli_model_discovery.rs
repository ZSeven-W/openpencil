//! Model catalogs for external coding CLIs whose integrations are newer than
//! the legacy provider set. Kept separate from `model_discovery.rs` so that
//! file stays below the repository's 800-line cap.
//!
//! Catalog queries run through `cli_probe_support::bounded_cli_output`, the
//! same bounded-subprocess runner `cli_provider_probe`'s connect probes use,
//! so a catalog query that hangs mid first-run OAuth surfaces the same
//! actionable `diagnose_timeout` message instead of silently discarding
//! whatever the CLI had already printed.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;
use op_ai::chat_provider::CliName;

use crate::cli_probe_error::CliProbeError;
use crate::cli_probe_support::{bounded_cli_output, diagnose_timeout, BoundedProbe};
use crate::model_discovery::resolve_cli;

/// `agy models` parsing. Split off at the 800-line cap; re-exported below
/// so `cli_model_discovery::parse_antigravity_models` stays the import path.
#[path = "cli_model_discovery_antigravity.rs"]
mod antigravity;

pub use antigravity::{note_antigravity_catalog_version, parse_antigravity_models};

#[cfg(test)]
use antigravity::{catalog_format_code, catalog_shape_change};

/// Matches `cli_provider_probe::MODELS_PROBE_TIMEOUT`: every query in this
/// module is a `models` call, and a `models` call is a network round trip.
/// Ten seconds cut off `agy`'s own error (11.04 s without a proxy) a beat
/// before it arrived.
const MODEL_QUERY_TIMEOUT: Duration = Duration::from_secs(20);

/// Query the installed Antigravity catalog. `agy models` has printed three
/// different shapes across releases — display names, one column of slugs,
/// and today's `id<TAB>display name` — all of which
/// [`parse_antigravity_models`] accepts while keeping the id column apart
/// from the label. Feeding a whole two-column row back as `--model` is
/// exactly the "model … is not recognized" failure this module exists to
/// avoid.
pub fn query_antigravity_models() -> Result<Vec<ModelEntry>, CliProbeError> {
    let exe = resolve_cli("agy").ok_or(CliProbeError::NotFound {
        provider: "Antigravity",
    })?;
    antigravity_models_from_exe(&exe, MODEL_QUERY_TIMEOUT)
}

/// Core of `query_antigravity_models`, with the executable path and timeout
/// injected so tests can point it at a fake `agy` (a `/bin/sh` script)
/// without waiting out the real `MODEL_QUERY_TIMEOUT`.
fn antigravity_models_from_exe(
    exe: &Path,
    timeout: Duration,
) -> Result<Vec<ModelEntry>, CliProbeError> {
    let output = match bounded_cli_output(CliName::Antigravity, exe, &["models"], timeout) {
        BoundedProbe::Completed(output) => output,
        BoundedProbe::TimedOut { stdout, stderr } => {
            return Err(CliProbeError::Timeout(diagnose_timeout(
                CliName::Antigravity,
                "Antigravity",
                "`agy`",
                timeout,
                &stdout,
                &stderr,
            )))
        }
        BoundedProbe::Failed => {
            return Err(CliProbeError::NotResponding {
                provider: "Antigravity",
            })
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            CliProbeError::QueryFailed {
                provider: "Antigravity",
                login_command: Some("`agy`"),
            }
        } else {
            CliProbeError::CliReported(stderr)
        });
    }
    require_antigravity_models(
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    )
}

pub fn discover_antigravity() -> Vec<ModelEntry> {
    if resolve_cli("agy").is_none() {
        return Vec::new();
    }
    query_antigravity_models().unwrap_or_else(|_| antigravity_default_model())
}

pub fn antigravity_default_model() -> Vec<ModelEntry> {
    vec![ModelEntry::new(
        AgentProvider::Antigravity,
        "default",
        "Antigravity default",
    )]
}

/// Shared by both catalogs: a leading list bullet is decoration, not part
/// of the value behind it.
fn trim_catalog_bullet(line: &str) -> (&str, bool) {
    let trimmed = line.trim_start();
    for bullet in ["* ", "- ", "• ", "● "] {
        if let Some(rest) = trimmed.strip_prefix(bullet) {
            return (rest.trim(), true);
        }
    }
    (trimmed.trim(), false)
}

/// Query the real Grok Build model catalog. The command is bounded because
/// model discovery runs during startup and must never strand its worker.
pub fn query_grok_models() -> Result<Vec<ModelEntry>, CliProbeError> {
    let exe = resolve_cli("grok").ok_or(CliProbeError::NotFound {
        provider: "Grok Build",
    })?;
    grok_models_from_exe(&exe, MODEL_QUERY_TIMEOUT)
}

/// Core of `query_grok_models`, with the executable path and timeout
/// injected so tests can point it at a fake `grok` (a `/bin/sh` script)
/// without waiting out the real `MODEL_QUERY_TIMEOUT`.
fn grok_models_from_exe(exe: &Path, timeout: Duration) -> Result<Vec<ModelEntry>, CliProbeError> {
    let output = match bounded_cli_output(CliName::GrokBuild, exe, &["models"], timeout) {
        BoundedProbe::Completed(output) => output,
        BoundedProbe::TimedOut { stdout, stderr } => {
            return Err(CliProbeError::Timeout(diagnose_timeout(
                CliName::GrokBuild,
                "Grok Build",
                "`grok`",
                timeout,
                &stdout,
                &stderr,
            )))
        }
        BoundedProbe::Failed => {
            return Err(CliProbeError::NotResponding {
                provider: "Grok Build",
            })
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            CliProbeError::QueryFailed {
                provider: "Grok Build",
                login_command: None,
            }
        } else {
            CliProbeError::CliReported(stderr)
        });
    }
    require_grok_models(
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    )
}

pub fn discover_grok() -> Vec<ModelEntry> {
    if resolve_cli("grok").is_none() {
        return Vec::new();
    }
    query_grok_models().unwrap_or_else(|_| grok_default_model())
}

fn grok_default_model() -> Vec<ModelEntry> {
    vec![ModelEntry::new(
        AgentProvider::GrokBuild,
        "default",
        "Grok Build default",
    )]
}

/// DeepSeek Harness catalog: `dsh` has NO verified model-listing
/// command (its verified surface is the one-shot
/// `dsh --profile headless "<prompt>"`), so discovery never spawns a
/// `models` query for it. Gated on the binary being installed — like
/// every fallback in this crate — the picker then lists the single
/// `default` entry, mirroring the `antigravity_default_model` shape.
pub fn discover_deepseek_harness() -> Vec<ModelEntry> {
    if resolve_cli("dsh").is_none() {
        return Vec::new();
    }
    deepseek_harness_default_model()
}

/// The one model entry the DeepSeek Harness card and chat picker
/// advertise. Shared by startup discovery and the connect probe.
pub fn deepseek_harness_default_model() -> Vec<ModelEntry> {
    vec![ModelEntry::new(
        AgentProvider::DeepSeekHarness,
        "default",
        "DeepSeek Harness default",
    )]
}

/// Parse `grok models`. Current builds print a human list, while some builds
/// expose JSON; accept both without mistaking headings or status prose for ids.
pub fn parse_grok_models(raw: &str) -> Vec<ModelEntry> {
    let mut ids = BTreeSet::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        let allow_scalar = value.is_array();
        collect_grok_ids(&value, &mut ids, allow_scalar);
    }

    let mut in_catalog = false;
    let mut catalog_ended = false;
    let mut saw_catalog_entry = false;
    for line in raw.lines() {
        let clean = strip_ansi(line);
        let clean = clean.trim();
        if clean.is_empty() {
            if in_catalog && saw_catalog_entry {
                in_catalog = false;
                catalog_ended = true;
            }
            continue;
        }
        if is_grok_catalog_separator(clean) {
            continue;
        }
        if is_grok_catalog_heading(clean) {
            in_catalog = true;
            catalog_ended = false;
            saw_catalog_entry = false;
            continue;
        }
        if is_grok_catalog_table_header(clean) {
            in_catalog = true;
            catalog_ended = false;
            saw_catalog_entry = false;
            continue;
        }
        if looks_like_grok_catalog_prose(clean) {
            if in_catalog && saw_catalog_entry {
                in_catalog = false;
                catalog_ended = true;
            }
            continue;
        }

        let (row, was_bullet) = trim_catalog_bullet(clean);
        let unheaded_bullet = was_bullet && !catalog_ended;
        let candidate = grok_catalog_row_id(row, in_catalog || unheaded_bullet);
        if let Some(id) = candidate.filter(|id| {
            in_catalog || unheaded_bullet || (!catalog_ended && is_official_grok_model_id(id))
        }) {
            ids.insert(id.to_string());
            if in_catalog || unheaded_bullet {
                saw_catalog_entry = true;
            }
        } else if in_catalog && saw_catalog_entry {
            in_catalog = false;
            catalog_ended = true;
        }
    }
    ids.into_iter()
        .map(|id| ModelEntry::new(AgentProvider::GrokBuild, id.clone(), id))
        .collect()
}

fn collect_grok_ids(value: &serde_json::Value, out: &mut BTreeSet<String>, allow_scalar: bool) {
    match value {
        serde_json::Value::String(value) if allow_scalar && is_grok_model_id(value) => {
            out.insert(value.clone());
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_grok_ids(value, out, allow_scalar);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if allow_scalar && matches!(key.as_str(), "id" | "model" | "name" | "alias") {
                    if let Some(value) = value.as_str().filter(|value| is_grok_model_id(value)) {
                        out.insert(value.to_string());
                    }
                } else if matches!(key.as_str(), "models" | "data" | "result" | "catalog") {
                    collect_grok_ids(value, out, true);
                }
            }
        }
        _ => {}
    }
}

fn is_grok_model_id(value: &str) -> bool {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    value.len() >= 2
        && value.len() <= 128
        && value
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && value
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':'))
        && !is_reserved_grok_catalog_word(&lower)
}

fn is_official_grok_model_id(value: &str) -> bool {
    value.to_ascii_lowercase().starts_with("grok-") && is_grok_model_id(value)
}

fn is_grok_catalog_heading(line: &str) -> bool {
    matches!(
        line.trim()
            .trim_end_matches(':')
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "models" | "available models" | "configured models" | "model catalog"
    )
}

fn is_grok_catalog_table_header(line: &str) -> bool {
    let Some(column) = first_grok_catalog_column(line) else {
        return false;
    };
    matches!(
        column.trim().to_ascii_lowercase().as_str(),
        "id" | "model" | "model id" | "name" | "alias"
    )
}

fn is_grok_catalog_separator(line: &str) -> bool {
    line.chars()
        .all(|c| c.is_ascii_whitespace() || matches!(c, '-' | '=' | '+' | '|'))
}

fn grok_catalog_row_id(row: &str, catalog_context: bool) -> Option<&str> {
    let row = row.trim().trim_matches('`');
    if row.is_empty() || looks_like_grok_catalog_prose(row) {
        return None;
    }

    if let Some(column) = first_grok_catalog_column(row) {
        return is_grok_model_id(column).then_some(column);
    }
    if is_grok_model_id(row) {
        return Some(row);
    }

    let (candidate, metadata) = row.split_once(char::is_whitespace)?;
    (catalog_context && is_grok_model_id(candidate) && is_grok_catalog_metadata(metadata))
        .then_some(candidate)
}

fn first_grok_catalog_column(row: &str) -> Option<&str> {
    if row.contains('|') {
        return row
            .split('|')
            .map(str::trim)
            .find(|column| !column.is_empty());
    }
    // A tab is always a column boundary. `grok models` prints bullet rows
    // today (verified: `Available models:` / `  * grok-4.5 (default)`), so
    // this is defensive — but it is the shape that broke the Antigravity
    // parser, and here the single-whitespace scan below would silently drop
    // the whole row rather than mis-read it.
    if let Some(gap) = row.find('\t') {
        return Some(row[..gap].trim());
    }

    let bytes = row.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index].is_ascii_whitespace() && bytes[index + 1].is_ascii_whitespace() {
            return Some(row[..index].trim());
        }
        index += 1;
    }
    None
}

fn is_grok_catalog_metadata(value: &str) -> bool {
    let lower = value
        .trim()
        .trim_matches(|c| matches!(c, '(' | ')' | '[' | ']'))
        .trim()
        .to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "default"
            | "selected"
            | "current"
            | "custom"
            | "configured"
            | "builtin"
            | "built-in"
            | "active"
            | "ready"
            | "recommended"
    )
}

fn looks_like_grok_catalog_prose(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "sign in",
        "log in",
        "authenticate",
        "authentication",
        "unauthorized",
        "credential",
        "api key",
        "default model:",
        "status:",
        "error:",
        "failed to",
        "request failed",
        "timed out",
        "not available",
        "unavailable",
        "loading",
        "checking",
        "no models",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn is_reserved_grok_catalog_word(value: &str) -> bool {
    matches!(
        value,
        "active"
            | "automatic"
            | "available"
            | "checking"
            | "connected"
            | "configured"
            | "current"
            | "default"
            | "disconnected"
            | "error"
            | "failed"
            | "fetching"
            | "id"
            | "loading"
            | "model"
            | "models"
            | "name"
            | "none"
            | "ready"
            | "recommended"
            | "selected"
            | "status"
            | "unknown"
            | "update"
    )
}

fn require_antigravity_models(
    stdout: &str,
    stderr: &str,
) -> Result<Vec<ModelEntry>, CliProbeError> {
    let models = parse_antigravity_models(stdout);
    if models.is_empty() {
        Err(catalog_error("Antigravity", "`agy`", stdout, stderr))
    } else {
        Ok(models)
    }
}

fn require_grok_models(stdout: &str, stderr: &str) -> Result<Vec<ModelEntry>, CliProbeError> {
    let models = parse_grok_models(stdout);
    if models.is_empty() {
        Err(catalog_error("Grok Build", "`grok`", stdout, stderr))
    } else {
        Ok(models)
    }
}

fn catalog_error(
    provider: &'static str,
    login_command: &'static str,
    stdout: &str,
    stderr: &str,
) -> CliProbeError {
    let diagnostics = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    let auth_required = [
        "sign in",
        "signin",
        "log in",
        "login",
        "authenticate",
        "authentication",
        "unauthorized",
        "credential",
        "api key",
    ]
    .iter()
    .any(|marker| diagnostics.contains(marker));
    if auth_required {
        CliProbeError::AuthRequired {
            provider,
            login_command,
        }
    } else if stdout.trim().is_empty() {
        CliProbeError::NoCatalog { provider }
    } else {
        CliProbeError::UnrecognizedCatalog { provider }
    }
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            let _ = chars.next();
            for code in chars.by_ref() {
                if code.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
#[path = "cli_model_discovery_tests.rs"]
mod tests;

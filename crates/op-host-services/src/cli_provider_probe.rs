//! Connect probes for Antigravity and Grok Build. Split from
//! `provider_probe.rs` to preserve that module's 800-line cap.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;
use op_ai::chat_provider::CliName;

use crate::chat_subprocess_safety;
use crate::model_discovery::resolve_cli;
use crate::provider_probe::ProbeOutcome;

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PROBE_OUTPUT_BYTES: usize = 1024 * 1024;

pub fn connect_antigravity() -> ProbeOutcome {
    let Some(exe) = resolve_cli("agy") else {
        return not_installed("Antigravity CLI not found", AgentProvider::Antigravity);
    };
    let Some(version) = cli_version(CliName::Antigravity, &exe, &["--version"]) else {
        return failed("Antigravity CLI not responding");
    };
    let models = match query_models(
        CliName::Antigravity,
        &exe,
        "Antigravity",
        "`agy`",
        crate::cli_model_discovery::parse_antigravity_models,
    ) {
        Ok(models) => models,
        Err(error) => return failed(&error),
    };
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some("Connected via Antigravity CLI".to_string()),
        hint_path: Some("~/.gemini/antigravity-cli/settings.json".to_string()),
        version: Some(version),
        ..ProbeOutcome::default()
    }
}

pub fn connect_grok_build() -> ProbeOutcome {
    let Some(exe) = resolve_cli("grok") else {
        return not_installed("Grok Build CLI not found", AgentProvider::GrokBuild);
    };
    let Some(version) = cli_version(CliName::GrokBuild, &exe, &["version"]) else {
        return failed("Grok Build CLI not responding");
    };
    let models = match query_models(
        CliName::GrokBuild,
        &exe,
        "Grok Build",
        "`grok`",
        crate::cli_model_discovery::parse_grok_models,
    ) {
        Ok(models) => models,
        Err(error) => return failed(&error),
    };
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some("Connected via Grok Build CLI".to_string()),
        hint_path: Some("~/.grok/config.toml".to_string()),
        version: Some(version),
        ..ProbeOutcome::default()
    }
}

fn not_installed(error: &str, provider: AgentProvider) -> ProbeOutcome {
    ProbeOutcome {
        error: Some(error.to_string()),
        not_installed: true,
        install_command: Some(crate::provider_probe::install_command(provider).to_string()),
        ..ProbeOutcome::default()
    }
}

fn failed(error: &str) -> ProbeOutcome {
    ProbeOutcome {
        error: Some(error.to_string()),
        ..ProbeOutcome::default()
    }
}

fn cli_version(cli: CliName, exe: &Path, args: &[&str]) -> Option<String> {
    let output = bounded_cli_output(cli, exe, args, PROBE_TIMEOUT)?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let version = if stdout.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };
    (!version.is_empty()).then(|| version.to_string())
}

fn query_models(
    cli: CliName,
    exe: &Path,
    provider: &str,
    login_command: &str,
    parse: fn(&str) -> Vec<ModelEntry>,
) -> Result<Vec<ModelEntry>, String> {
    let output = bounded_cli_output(cli, exe, &["models"], PROBE_TIMEOUT)
        .ok_or_else(|| format!("{provider} model query failed or timed out"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        if let Some(message) = chat_subprocess_safety::friendly_stderr_error(Some(cli), &stderr) {
            return Err(message);
        }
        return Err(if stderr.trim().is_empty() {
            format!("{provider} model query failed. Run {login_command} once to authenticate.")
        } else {
            stderr.trim().to_string()
        });
    }

    let models = parse(&stdout);
    if !models.is_empty() {
        return Ok(models);
    }
    Err(catalog_error(provider, login_command, &stdout, &stderr))
}

fn catalog_error(provider: &str, login_command: &str, stdout: &str, stderr: &str) -> String {
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
        format!(
            "{provider} model query requires authentication. Run {login_command} once to sign in."
        )
    } else if stdout.trim().is_empty() {
        format!("{provider} model query returned no model catalog")
    } else {
        format!("{provider} returned an unrecognized model catalog")
    }
}

/// Run a connection/version/model probe with the same explicit environment
/// policy as a real chat turn. This is intentionally separate from the legacy
/// catalog runner: an `env_clear` is essential here so Settings probes cannot
/// expose unrelated host secrets to a third-party coding-agent CLI.
fn bounded_cli_output(
    cli: CliName,
    exe: &Path,
    args: &[&str],
    timeout: Duration,
) -> Option<Output> {
    let mut command = Command::new(exe);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(chat_subprocess_safety::child_env(Some(cli))?);
    crate::chat_spawn::hide_console_window(&mut command);
    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let stderr = child.stderr.take()?;
    let stdout_reader = drain_pipe(stdout);
    let stderr_reader = drain_pipe(stderr);
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Some(Output {
                    status,
                    stdout: stdout_reader.join().unwrap_or_default(),
                    stderr: stderr_reader.join().unwrap_or_default(),
                });
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return None;
            }
        }
    }
}

/// Continue draining after the retained output cap so a verbose CLI cannot
/// fill an OS pipe and deadlock the bounded probe.
fn drain_pipe<R>(mut pipe: R) -> JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut retained = Vec::new();
        let mut chunk = [0_u8; 8192];
        loop {
            let Ok(count) = pipe.read(&mut chunk) else {
                break;
            };
            if count == 0 {
                break;
            }
            let remaining = MAX_PROBE_OUTPUT_BYTES.saturating_sub(retained.len());
            retained.extend_from_slice(&chunk[..count.min(remaining)]);
        }
        retained
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::agent_settings_state::AgentProvider;

    #[test]
    fn not_installed_outcome_carries_provider_guidance() {
        let outcome = not_installed("missing", AgentProvider::Antigravity);
        assert!(outcome.not_installed);
        assert_eq!(outcome.error.as_deref(), Some("missing"));
        assert_eq!(
            outcome.install_command.as_deref(),
            Some(crate::provider_probe::install_command(
                AgentProvider::Antigravity
            ))
        );
    }

    #[test]
    fn provider_variants_are_the_expected_catalog_owners() {
        assert_eq!(
            crate::cli_model_discovery::antigravity_default_model()[0].provider,
            AgentProvider::Antigravity
        );
    }

    #[test]
    fn empty_catalog_error_distinguishes_auth_from_bad_output() {
        assert!(catalog_error("Grok Build", "`grok`", "", "login required")
            .contains("requires authentication"));
        assert_eq!(
            catalog_error("Grok Build", "`grok`", "unexpected prose", ""),
            "Grok Build returned an unrecognized model catalog"
        );
    }
}

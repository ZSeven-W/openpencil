//! Shared bounded-subprocess plumbing for CLI connect probes
//! (`cli_provider_probe.rs`) and CLI model discovery
//! (`cli_model_discovery.rs`). Both run a short-lived external CLI with a
//! deadline and need the same "don't throw away captured output on timeout"
//! behavior, so the run/kill/diagnose logic lives here once instead of
//! forking in each caller.

use std::io::Read;
use std::path::Path;
use std::process::{Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use op_ai::chat_provider::CliName;

use crate::chat_subprocess_safety;

pub(crate) const MAX_PROBE_OUTPUT_BYTES: usize = 1024 * 1024;

/// How much of a timed-out probe's captured output to surface verbatim when
/// no known auth/permission marker matched, so a Settings-card error is at
/// least diagnosable instead of a bare "timed out".
const TIMEOUT_TAIL_CHARS: usize = 200;

/// Outcome of a bounded, piped-output CLI probe.
pub(crate) enum BoundedProbe {
    /// The process exited within the timeout; `Output` carries whatever
    /// stdout/stderr was captured up to `MAX_PROBE_OUTPUT_BYTES`.
    Completed(Output),
    /// The process was still running at the deadline (or `try_wait` errored)
    /// and was killed. Carries whatever stdout/stderr was captured before
    /// the kill — a CLI waiting on first-run OAuth typically already
    /// printed its auth prompt by then.
    TimedOut { stdout: Vec<u8>, stderr: Vec<u8> },
    /// Never got a running process to observe (env lookup, spawn, or pipe
    /// setup failed) — no output exists to retain.
    Failed,
}

/// Run a connection/version/model probe with the same explicit environment
/// policy as a real chat turn. This is intentionally separate from the legacy
/// catalog runner: an `env_clear` is essential here so Settings probes cannot
/// expose unrelated host secrets to a third-party coding-agent CLI.
pub(crate) fn bounded_cli_output(
    cli: CliName,
    exe: &Path,
    args: &[&str],
    timeout: Duration,
) -> BoundedProbe {
    let env = match cli {
        CliName::Codex => crate::chat_subprocess_quirks::codex_child_env(),
        _ => {
            let Some(env) = chat_subprocess_safety::child_env(Some(cli)) else {
                return BoundedProbe::Failed;
            };
            env
        }
    };
    let mut command = crate::chat_spawn::build_blocking_command(exe, args);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(env)
        // `env_clear` removes the PATH installed by the shared command
        // builder. Restore the binary-aware value last so an npm wrapper's
        // `#!/usr/bin/env node` uses the Node beside the resolved CLI.
        .env("PATH", crate::chat_spawn::runtime_path_for_binary(exe));
    // The shared tree cleanup can cover descendants only when the child leads
    // its own Unix process group. Without this, killing an npm/shell wrapper
    // leaves its grandchild holding the capture pipes open past the deadline.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    crate::chat_spawn::hide_console_window(&mut command);
    let Ok(mut child) = command.spawn() else {
        return BoundedProbe::Failed;
    };
    // Capture the group target before `try_wait` can reap the leader. A CLI
    // wrapper is allowed to exit after forking a helper that still owns our
    // pipes; after the reap, reconstructing the process group from the pid is
    // no longer reliable.
    let process_tree = op_process_io::ProcessTree::from_child(&child).ok();
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        let _ = op_process_io::terminate_process_tree(&mut child, Duration::ZERO);
        return BoundedProbe::Failed;
    };
    let stdout_reader = PipeCapture::spawn(stdout);
    let stderr_reader = PipeCapture::spawn(stderr);
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // A successful leader exit does not imply EOF: an inherited
                // descriptor can remain open in a descendant. Best-effort
                // tree cleanup closes the ordinary wrapper/helper case, and
                // the bounded drain below covers detached descendants.
                if let Some(tree) = process_tree {
                    let _ = tree.kill_after_leader_exit();
                }
                let (stdout, stderr) = finish_pipe_captures(stdout_reader, stderr_reader, deadline);
                return BoundedProbe::Completed(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(
                    Duration::from_millis(50)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Ok(None) | Err(_) => {
                // The shared helper reaps after any accepted signal but
                // returns immediately if neither the tree nor leader could be
                // signalled, avoiding an unbounded wait past this deadline.
                let _ = op_process_io::terminate_process_tree(&mut child, Duration::ZERO);
                let (stdout, stderr) = finish_pipe_captures(stdout_reader, stderr_reader, deadline);
                return BoundedProbe::TimedOut { stdout, stderr };
            }
        }
    }
}

/// A background drainer for one child pipe.
///
/// Reads past the retained cap so a verbose CLI cannot fill an OS pipe and
/// deadlock the probe. The bytes live behind a mutex rather than inside the
/// reader thread's return value so a timed-out probe can take what was
/// captured WITHOUT joining. Process-tree cleanup normally closes inherited
/// pipes, but a platform limitation or independently detached descendant must
/// still never let an unconditional reader join override the probe deadline.
/// Shared with `chat_spawn`'s login-shell env probe, which needs the same
/// guarantee against a blocking shell rc.
pub(crate) struct PipeCapture {
    retained: Arc<Mutex<Vec<u8>>>,
    reader: JoinHandle<()>,
}

impl PipeCapture {
    pub(crate) fn spawn<R>(mut pipe: R) -> Self
    where
        R: Read + Send + 'static,
    {
        let retained = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&retained);
        let reader = std::thread::spawn(move || {
            let mut chunk = [0_u8; 8192];
            loop {
                let Ok(count) = pipe.read(&mut chunk) else {
                    break;
                };
                if count == 0 {
                    break;
                }
                let Ok(mut retained) = sink.lock() else {
                    break;
                };
                let remaining = MAX_PROBE_OUTPUT_BYTES.saturating_sub(retained.len());
                retained.extend_from_slice(&chunk[..count.min(remaining)]);
            }
        });
        Self { retained, reader }
    }

    /// Everything captured before an absolute deadline. This deliberately
    /// never joins the reader: observing the leader exit does not prove that
    /// every descendant closed its inherited descriptor.
    pub(crate) fn finish_by(self, deadline: Instant) -> Vec<u8> {
        while !self.reader.is_finished() && Instant::now() < deadline {
            std::thread::sleep(
                Duration::from_millis(10).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
        take_retained(&self.retained)
    }
}

/// Drain both pipes concurrently up to the smaller of the probe's remaining
/// total budget and a short capture grace. Passing the same absolute deadline
/// to both readers prevents two sequential waits from doubling the budget.
pub(crate) fn finish_pipe_captures(
    stdout: PipeCapture,
    stderr: PipeCapture,
    probe_deadline: Instant,
) -> (Vec<u8>, Vec<u8>) {
    let capture_deadline = probe_deadline.min(Instant::now() + CAPTURE_GRACE);
    let stdout = stdout.finish_by(capture_deadline);
    let stderr = stderr.finish_by(capture_deadline);
    (stdout, stderr)
}

fn take_retained(retained: &Mutex<Vec<u8>>) -> Vec<u8> {
    retained
        .lock()
        .map(|mut retained| std::mem::take(&mut *retained))
        .unwrap_or_default()
}

/// Maximum share of the existing probe budget spent letting reader threads
/// drain bytes already in flight. This never extends the probe's total
/// deadline.
pub(crate) const CAPTURE_GRACE: Duration = Duration::from_millis(200);

/// Turn a timed-out probe's retained stdout/stderr into an actionable
/// message. Checked first against the same auth-prompt vocabulary a
/// completed, non-zero-exit probe uses (`friendly_stdout_error` /
/// `friendly_stderr_error`) — a CLI that is mid first-run OAuth typically
/// never exits within the probe budget, so the auth prompt only ever shows
/// up here, never on the completed-output path. Falls back to a generic
/// timeout message carrying a truncated tail of whatever the CLI printed.
pub(crate) fn diagnose_timeout(
    cli: CliName,
    provider: &str,
    login_command: &str,
    timeout: Duration,
    stdout: &[u8],
    stderr: &[u8],
) -> String {
    let stdout_text = String::from_utf8_lossy(stdout);
    let stderr_text = String::from_utf8_lossy(stderr);
    for line in stdout_text.lines() {
        if let Some(message) = chat_subprocess_safety::friendly_stdout_error(Some(cli), line) {
            return message;
        }
    }
    if let Some(message) = chat_subprocess_safety::friendly_stderr_error(Some(cli), &stderr_text) {
        return message;
    }
    let tail = tail_snippet(&stdout_text, &stderr_text);
    let timeout_secs = timeout.as_secs();
    if tail.is_empty() {
        format!(
            "{provider} CLI timed out after {timeout_secs}s with no output. \
             Run {login_command} once in a terminal to authenticate."
        )
    } else {
        format!(
            "{provider} CLI timed out after {timeout_secs}s. \
             Run {login_command} once in a terminal to authenticate. Last output: {tail}"
        )
    }
}

/// Last `TIMEOUT_TAIL_CHARS` characters of the combined stdout+stderr text,
/// trimmed — the freshest signal from a hung CLI, capped so a chatty
/// process can't blow up the Settings card's error text.
pub(crate) fn tail_snippet(stdout: &str, stderr: &str) -> String {
    let combined = format!("{stdout}{stderr}");
    let trimmed = combined.trim();
    let char_count = trimmed.chars().count();
    if char_count <= TIMEOUT_TAIL_CHARS {
        trimmed.to_string()
    } else {
        trimmed
            .chars()
            .skip(char_count - TIMEOUT_TAIL_CHARS)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Starting probe budget for the hung-CLI tests, and the ceiling
    /// [`timed_out_probe_with_output`] may escalate to.
    ///
    /// Test-harness numbers only — no production timeout reads them. These
    /// tests assert that the deadline is reached WITH the script's output
    /// already captured, which races process startup: a tight window lets a
    /// loaded machine reach the deadline before the shell's `printf` runs,
    /// and the capture comes back empty. So the tests start at the floor and
    /// retry with a doubled budget while the capture is empty, up to the cap
    /// (same approach as `cli_model_discovery_tests`).
    #[cfg(unix)]
    const PROBE_BUDGET: Duration = Duration::from_secs(4);
    #[cfg(unix)]
    const PROBE_BUDGET_CAP: Duration = Duration::from_secs(16);

    /// How long the fake CLI hangs after printing — comfortably past
    /// `PROBE_BUDGET_CAP` so the timeout branch is guaranteed even at full
    /// escalation, but finite so nothing can outlive the test run.
    ///
    /// The hang is spelled `exec sleep N`, not `sleep N`, so these tests
    /// isolate the deadline path. A separate regression below deliberately
    /// leaves a forked descendant holding both capture pipes after its leader
    /// exits.
    #[cfg(unix)]
    const FAKE_CLI_HANG_SECS: u32 = 30;

    /// Run the auth-prompt-then-hang script under an escalating deadline
    /// until the retained stdout actually holds the prompt, then hand back
    /// the captured streams plus the budget that produced them.
    ///
    /// Each attempt asserts the probe returned on its own deadline instead of
    /// outlasting the `FAKE_CLI_HANG_SECS` child, so "the probe is
    /// deadline-bounded" stays under test.
    #[cfg(unix)]
    fn timed_out_probe_with_output(cli: CliName) -> (Vec<u8>, Vec<u8>, Duration) {
        let script = format!(
            "printf 'Authentication required. Please visit the URL to log in:\\n'; \
             exec sleep {FAKE_CLI_HANG_SECS}"
        );
        let mut budget = PROBE_BUDGET;
        loop {
            let started = Instant::now();
            let probe = bounded_cli_output(cli, Path::new("/bin/sh"), &["-c", &script], budget);
            let elapsed = started.elapsed();
            let BoundedProbe::TimedOut { stdout, stderr } = probe else {
                panic!("expected the sleep to outlast the timeout");
            };
            assert!(
                elapsed < budget * 4,
                "probe must return on its own deadline ({budget:?}), not outlast the \
                 {FAKE_CLI_HANG_SECS}s script; took {elapsed:?}"
            );
            if String::from_utf8_lossy(&stdout).contains("Authentication required")
                || budget >= PROBE_BUDGET_CAP
            {
                return (stdout, stderr, budget);
            }
            budget = (budget * 2).min(PROBE_BUDGET_CAP);
        }
    }

    #[test]
    fn tail_snippet_truncates_to_last_n_chars() {
        let long = "a".repeat(500);
        let tail = tail_snippet(&long, "");
        assert_eq!(tail.len(), TIMEOUT_TAIL_CHARS);
        assert!(long.ends_with(&tail));

        // Under the cap: returned verbatim, trimmed.
        assert_eq!(tail_snippet("  short  ", ""), "short");
    }

    #[test]
    fn diagnose_timeout_surfaces_antigravity_stdout_auth_prompt() {
        let message = diagnose_timeout(
            CliName::Antigravity,
            "Antigravity",
            "`agy`",
            Duration::from_secs(10),
            b"Authentication required. Please visit the URL to log in:\n",
            b"",
        );
        assert_eq!(
            message,
            "Antigravity is not authenticated. Run `agy` once in a terminal."
        );
    }

    #[test]
    fn diagnose_timeout_surfaces_grok_stderr_auth_prompt() {
        let message = diagnose_timeout(
            CliName::GrokBuild,
            "Grok Build",
            "`grok login`",
            Duration::from_secs(10),
            b"",
            b"login required to continue",
        );
        assert_eq!(
            message,
            "Grok Build is not authenticated. Run `grok login` in a terminal."
        );
    }

    #[test]
    fn diagnose_timeout_falls_back_to_truncated_tail_when_no_marker_matches() {
        let message = diagnose_timeout(
            CliName::GrokBuild,
            "Grok Build",
            "`grok login`",
            Duration::from_secs(10),
            b"initializing sandbox...\nstill working\n",
            b"",
        );
        assert!(message.contains("timed out after 10s"));
        assert!(message.contains("`grok login`"));
        assert!(message.contains("still working"));
    }

    #[test]
    fn diagnose_timeout_reports_no_output_when_nothing_was_captured() {
        let message = diagnose_timeout(
            CliName::Antigravity,
            "Antigravity",
            "`agy`",
            Duration::from_secs(10),
            b"",
            b"",
        );
        assert!(message.contains("no output"));
        assert!(message.contains("`agy`"));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_cli_output_retains_captured_bytes_on_timeout() {
        // A CLI mid first-run OAuth: prints its prompt, then hangs well
        // past the probe budget. The kill-on-deadline path must not throw
        // away what the reader threads already captured.
        let (stdout, _stderr, _budget) = timed_out_probe_with_output(CliName::Antigravity);
        assert!(String::from_utf8_lossy(&stdout).contains("Authentication required"));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_cli_output_returns_at_its_deadline_when_the_pipe_never_reaches_eof() {
        // The shell and both descendants inherit the pipes. Tree cleanup
        // should close them; bounded capture remains the final backstop if a
        // platform cannot cover one descendant. Output captured before the
        // kill must still survive.
        let script = "printf 'Authentication required\\n'; sleep 30 & sleep 30";
        let mut budget = PROBE_BUDGET;
        loop {
            let started = Instant::now();
            let probe = bounded_cli_output(
                CliName::Antigravity,
                Path::new("/bin/sh"),
                &["-c", script],
                budget,
            );
            let elapsed = started.elapsed();
            let BoundedProbe::TimedOut { stdout, .. } = probe else {
                panic!("expected the sleeps to outlast the timeout");
            };
            assert!(
                elapsed < budget * 4,
                "the deadline, not the child, decides when the probe returns"
            );
            if String::from_utf8_lossy(&stdout).contains("Authentication required") {
                break;
            }
            assert!(
                budget < PROBE_BUDGET_CAP,
                "prompt output was not captured within {budget:?}"
            );
            budget = (budget * 2).min(PROBE_BUDGET_CAP);
        }
    }

    #[cfg(unix)]
    #[test]
    fn bounded_cli_output_drains_large_stdout_and_stderr_before_exit() {
        // A verbose CLI (e.g. a chatty catalog dump) must not deadlock the
        // probe by filling an OS pipe buffer before the process exits, and
        // draining must cap at MAX_PROBE_OUTPUT_BYTES per stream.
        let script = "i=0; while [ $i -lt 40000 ]; do \
                      printf '0123456789abcdef0123456789abcdef\\n'; \
                      printf 'fedcba9876543210fedcba9876543210\\n' >&2; \
                      i=$((i+1)); done";
        match bounded_cli_output(
            CliName::GrokBuild,
            Path::new("/bin/sh"),
            &["-c", script],
            // The script exits on its own, so a generous ceiling costs
            // nothing and keeps machine load from turning this
            // completed-output assertion into a timeout.
            PROBE_BUDGET_CAP,
        ) {
            BoundedProbe::Completed(output) => {
                assert!(output.status.success());
                assert_eq!(output.stdout.len(), MAX_PROBE_OUTPUT_BYTES);
                assert_eq!(output.stderr.len(), MAX_PROBE_OUTPUT_BYTES);
            }
            _ => panic!("large piped output should not deadlock or time out"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn bounded_cli_output_completes_normally_when_process_exits_in_time() {
        // Success path must behave exactly as before: a process that
        // exits inside the budget yields Completed with its output.
        match bounded_cli_output(
            CliName::GrokBuild,
            Path::new("/bin/sh"),
            &["-c", "echo hello"],
            PROBE_BUDGET_CAP,
        ) {
            BoundedProbe::Completed(output) => {
                assert!(output.status.success());
                assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
            }
            _ => panic!("expected Completed, not a timeout/failure path"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn completed_probe_does_not_wait_for_descendant_pipe_eof() {
        // The shell exits successfully while its background child retains
        // both inherited write ends. `try_wait` therefore observes a normal
        // leader exit before either capture reader can see EOF. The probe must
        // clean the captured process group and use a bounded drain, preserving
        // the leader's output without waiting for the 30-second descendant.
        let script = "printf 'ready\\n'; printf 'notice\\n' >&2; sleep 30 & exit 0";
        let started = Instant::now();
        let probe = bounded_cli_output(
            CliName::GrokBuild,
            Path::new("/bin/sh"),
            &["-c", script],
            PROBE_BUDGET_CAP,
        );
        let elapsed = started.elapsed();
        let BoundedProbe::Completed(output) = probe else {
            panic!("an exited leader must remain a completed probe");
        };
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ready");
        assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), "notice");
        assert!(
            elapsed < Duration::from_secs(10),
            "pipe EOF from the descendant must not determine completion; took {elapsed:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn cli_version_pipeline_surfaces_actionable_message_when_version_check_hangs() {
        // Regression test for the 0.8.1 Antigravity connect failure,
        // exercised through the exact two-step pipeline callers run
        // internally (bounded_cli_output → diagnose_timeout on the
        // TimedOut branch): a CLI stuck on first-run OAuth during
        // `agy --version` used to surface only "CLI not responding"; it
        // must now name the fix.
        let (stdout, stderr, probe_timeout) = timed_out_probe_with_output(CliName::Antigravity);
        let message = diagnose_timeout(
            CliName::Antigravity,
            "Antigravity",
            "`agy`",
            probe_timeout,
            &stdout,
            &stderr,
        );
        assert_eq!(
            message,
            "Antigravity is not authenticated. Run `agy` once in a terminal."
        );
    }
}

//! Failure-diagnosability regressions for the subprocess CLI bridge.
//!
//! Every case here drives the real [`SubprocessProvider::send`] loop
//! against a stand-in binary reproducing a stdio shape measured from a
//! real CLI, and asserts on the `ChatDelta::Error` a user would see.
//!
//! Unix-only: the stand-ins are `/bin/sh` scripts. The behaviour under
//! test is platform-independent.
#![cfg(unix)]

use super::*;

/// Antigravity's unauthenticated output, measured 2026-08-07 by running
/// the production argv (`--sandbox --print-timeout 90s --mode plan`)
/// with a private `--gemini_dir` and piped stdio. Two facts this fixture
/// encodes: the whole block lands on **stderr** (piped stdout came back
/// empty, 0 bytes), and the process exits 1.
///
/// The OAuth parameters are FAKE placeholders — the shape is what the
/// redaction has to survive, and no real credential belongs in a test.
const AGY_UNAUTHENTICATED: &str = r#"#!/bin/sh
cat >&2 <<'EOT'
Authentication required. Please visit the URL to log in:
  https://accounts.google.com/o/oauth2/auth?access_type=offline&client_id=000000000000-fakefakefakefake.apps.googleusercontent.com&code_challenge=FAKECODECHALLENGE0000&code_challenge_method=S256&prompt=consent&response_type=code&state=FAKESTATE0000

Waiting for authentication (timeout 60s)...
Or, paste the authorization code here and press Enter:
Error: authentication timed out.
Error: authentication failed or timed out
EOT
exit 1
"#;

/// A failure with no keyword the classifier knows — the case that used
/// to reach the user as a bare exit status. Carries a credential in the
/// same breath, because real CLIs dump their config when they crash.
const AGY_UNCLASSIFIABLE_CRASH: &str = r#"#!/bin/sh
cat >&2 <<'EOT'
panic: runtime error: index out of range [3] with length 0
  loaded profile from /tmp/turn/gemini/settings.json
  upstream=https://agent.example.test/v1/plan?api_key=fake-key-000111222&trace=abc
goroutine 1 [running]:
main.planOnce(0x14000112000)
EOT
exit 3
"#;

/// A stand-in `agy` on disk. Returns the containing directory (for
/// cleanup) and the executable path.
fn stub_cli(body: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    // A counter, not just a timestamp: these tests run in parallel and
    // the clock is coarse enough that two of them collided on one
    // directory, so one test silently executed another's stub.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "openpencil-exit-tests-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("agy");
    std::fs::write(&path, body).expect("write stub");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    (dir, path)
}

fn read_test_pids(path: &std::path::Path) -> Vec<i32> {
    std::fs::read_to_string(path)
        .expect("CLI pid file")
        .split_whitespace()
        .map(|pid| pid.parse().expect("numeric pid"))
        .collect()
}

/// Wait for a stub to write its pid file. A liveness bound, not the
/// property under test: under parallel `cargo test` on a loaded machine
/// the stub's spawn chain alone can take several seconds (an earlier 5s
/// bound expired for real under concurrent full-suite runs), so this is
/// generous — it only shapes how long a genuinely broken spawn takes to
/// report, never whether a healthy one passes.
///
/// Waits for the newline-terminated payload, not mere existence: the
/// shell's `>` redirection creates the file before the pid write lands,
/// so an existence check can hand the reader an empty file (observed
/// under load as a ParseIntError on an "Empty" pid).
fn wait_for_pid_file(path: &std::path::Path) {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        if std::fs::read_to_string(path).is_ok_and(|content| content.ends_with('\n')) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn process_alive(pid: i32) -> bool {
    // SAFETY: signal 0 is a read-only existence probe for an exact positive
    // pid written by this test's own child.
    (unsafe { libc::kill(pid, 0) }) == 0
        || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn assert_process_tree_reaped(pids: &[i32], context: &str) {
    // 5s: signal delivery plus init's reap of orphaned zombies can lag by
    // seconds on a loaded machine; a genuinely surviving process (the
    // regression this guards) still fails below, just a bit later.
    for _ in 0..500 {
        if pids.iter().copied().all(|pid| !process_alive(pid)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let survivors: Vec<_> = pids
        .iter()
        .copied()
        .filter(|pid| process_alive(*pid))
        .collect();
    for pid in &survivors {
        // SAFETY: exact still-live test child pid, force-killed only as test
        // cleanup before reporting the regression.
        let _ = unsafe { libc::kill(*pid, libc::SIGKILL) };
    }
    assert!(survivors.is_empty(), "{context}: {survivors:?}");
}

/// Run one generation turn against a stand-in and return the error text
/// the user would see.
///
/// Retries a spawn that races on `ETXTBSY`. Writing a stub and exec'ing
/// it from many threads at once means one thread's still-open write fd
/// can be inherited across another thread's `fork()` and briefly hold
/// the freshly-written stub open for write, so `execve` reports "Text
/// file busy". That is an artifact of the parallel write-then-exec
/// harness, not the stderr-drain behaviour under test, and the window is
/// microseconds — a retry clears it.
fn turn_error(body: &str) -> String {
    for _ in 0..16 {
        let message = turn_error_once(body);
        if !message.contains("Text file busy") && !message.contains("os error 26") {
            return message;
        }
    }
    turn_error_once(body)
}

fn turn_error_once(body: &str) -> String {
    let (dir, binary) = stub_cli(body);
    let provider = SubprocessProvider::for_cli_generation(CliName::Antigravity)
        .expect("antigravity has a subprocess template")
        .with_test_binary(binary.to_string_lossy().into_owned());
    let deltas: Vec<ChatDelta> = provider
        .send(ChatRequest {
            user_message: "design a landing page".into(),
            ..Default::default()
        })
        .collect();
    let _ = std::fs::remove_dir_all(dir);
    assert!(
        deltas
            .iter()
            .any(|delta| matches!(delta, ChatDelta::Done { stop_reason } if *stop_reason == StopReason::Aborted)),
        "a failed turn must end Aborted, got {deltas:?}"
    );
    deltas
        .iter()
        .find_map(|delta| match delta {
            ChatDelta::Error(message) => Some(message.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected an Error delta, got {deltas:?}"))
}

#[test]
fn unauthenticated_antigravity_reads_as_an_auth_problem_not_an_exit_code() {
    let message = turn_error(AGY_UNAUTHENTICATED);
    assert!(
        message.starts_with("Antigravity is not authenticated. Run `agy` once in a terminal."),
        "the auth block arrives on stderr; classifying only stdout leaves \
         the user with a bare exit status: {message}"
    );
    assert!(!message.contains("exited with status"), "{message}");
}

#[test]
fn a_classified_failure_still_shows_what_the_cli_actually_said() {
    // The verdict and the evidence travel together. Without the tail a
    // misclassification is undetectable — the reader sees a confident
    // sentence and nothing to check it against.
    let message = turn_error(AGY_UNAUTHENTICATED);
    assert!(message.contains("Authentication required"), "{message}");
    assert!(
        message.contains("authentication failed or timed out"),
        "{message}"
    );
    // Still redacted: the quoted block carries a live OAuth URL.
    assert!(
        message.contains("accounts.google.com/o/oauth2/auth?<redacted>"),
        "{message}"
    );
    for secret in ["client_id=", "code_challenge=", "state=FAKESTATE"] {
        assert!(!message.contains(secret), "leaked {secret:?} in {message}");
    }
}

#[test]
fn unclassifiable_failure_quotes_the_child_instead_of_only_its_exit_code() {
    let message = turn_error(AGY_UNCLASSIFIABLE_CRASH);
    assert!(
        message.starts_with("CLI exited with status 3"),
        "the exit status stays in the message: {message}"
    );
    // The evidence the old fallback threw away.
    assert!(message.contains("index out of range"), "{message}");
    assert!(message.contains("goroutine 1"), "{message}");
    // …with every credential-shaped fragment scrubbed out of it.
    for secret in [
        "api_key=fake",
        "fake-key-000111222",
        "?api_key",
        "trace=abc",
    ] {
        assert!(!message.contains(secret), "leaked {secret:?} in {message}");
    }
    assert!(
        message.contains("agent.example.test/v1/plan?<redacted>"),
        "{message}"
    );
}

#[test]
fn a_silent_child_says_so_rather_than_quoting_nothing() {
    let message = turn_error("#!/bin/sh\nexit 9\n");
    assert_eq!(message, "CLI exited with status 9 (no output captured)");
}

#[test]
fn quoted_output_is_length_capped_however_much_the_child_prints() {
    // 40k lines of stderr; the surfaced message must not grow with it.
    let body = "#!/bin/sh\nawk 'BEGIN{for(i=0;i<40000;i++) \
                print \"stderr noise line \" i > \"/dev/stderr\"}'\nexit 4\n";
    let message = turn_error(body);
    assert!(
        message.chars().count() <= 64 + op_util::cli_output::TAIL_MAX_CHARS,
        "message was {} chars: {message}",
        message.chars().count()
    );
    // Bounded, but still the part that matters: the child's last words.
    assert!(message.contains("stderr noise line 39999"), "{message}");
}

#[test]
fn a_childs_stderr_is_never_lost_to_the_drain_task_still_being_in_flight() {
    // The child writes stderr and dies in the same breath, so its two
    // pipes hit EOF together and the read loop races the drain task.
    // On an idle machine the drain always wins; under real load it does
    // not, and the failure mode is silent — a child that explained
    // itself reported as `(no output captured)`. Found for real by
    // running four crates' test binaries concurrently.
    //
    // The test has to CREATE the contention rather than hope for it: on
    // an idle machine the drain wins every time and this case passes
    // even with the fix reverted. Concurrent turns saturate the shared
    // runtime's workers the way the orchestrator's parallel subtasks do,
    // and that is exactly when the tail comes back empty.
    const THREADS: usize = 16;
    const TURNS_PER_THREAD: usize = 12;
    let lost = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let mut handles = Vec::new();
    for _ in 0..THREADS {
        let lost = std::sync::Arc::clone(&lost);
        handles.push(std::thread::spawn(move || {
            for _ in 0..TURNS_PER_THREAD {
                let message = turn_error(
                    "#!/bin/sh\necho 'fatal: upstream refused the plan request' >&2\nexit 5\n",
                );
                if !message.contains("upstream refused the plan request") {
                    lost.lock().expect("not poisoned").push(message);
                }
            }
        }));
    }
    for handle in handles {
        handle.join().expect("worker thread");
    }
    let lost = lost.lock().expect("not poisoned");
    assert!(
        lost.is_empty(),
        "{} of {} turns lost the child's stderr, e.g. {:?}",
        lost.len(),
        THREADS * TURNS_PER_THREAD,
        lost.first()
    );
}

#[test]
fn a_cli_that_diagnoses_itself_on_stdout_is_quoted_too() {
    // Same failure reported on stdout instead of stderr — the stream
    // split is not a stable contract across CLIs or across TTY vs pipe.
    let body = "#!/bin/sh\necho 'fatal: workspace policy rejected the request'\nexit 2\n";
    let message = turn_error(body);
    assert!(message.starts_with("CLI exited with status 2"), "{message}");
    assert!(
        message.contains("workspace policy rejected the request"),
        "{message}"
    );
}

#[test]
fn codex_terminal_error_events_finish_the_turn_as_aborted() {
    for event in [
        r#"{"type":"turn.failed","error":{"message":"usage limit reached"}}"#,
        r#"{"type":"error","message":"stream disconnected"}"#,
    ] {
        let body = format!("#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' '{event}'\n");
        let (dir, binary) = stub_cli(&body);
        let provider = SubprocessProvider::for_cli(CliName::Codex)
            .expect("codex subprocess template")
            .with_test_binary(binary.to_string_lossy().into_owned());
        let deltas: Vec<_> = provider
            .send(ChatRequest {
                user_message: "inspect the canvas".into(),
                ..Default::default()
            })
            .collect();
        let _ = std::fs::remove_dir_all(dir);

        assert!(
            matches!(deltas.first(), Some(ChatDelta::Error(_))),
            "terminal event must surface its error: {deltas:?}"
        );
        assert!(matches!(
            deltas.last(),
            Some(ChatDelta::Done {
                stop_reason: StopReason::Aborted
            })
        ));
        assert_eq!(
            deltas.len(),
            2,
            "one error and one terminal delta: {deltas:?}"
        );
    }
}

#[test]
fn cancelling_a_silent_subprocess_reaps_its_descendant() {
    let body = r#"#!/bin/sh
sleep 30 &
descendant=$!
printf '%s %s\n' "$$" "$descendant" > "$0.pids"
cat >/dev/null
wait "$descendant"
"#;
    let (dir, binary) = stub_cli(body);
    let pid_file = std::path::PathBuf::from(format!("{}.pids", binary.to_string_lossy()));
    let provider = SubprocessProvider::for_cli(CliName::Codex)
        .expect("codex subprocess template")
        .with_test_binary(binary.to_string_lossy().into_owned());
    let cancel = Arc::new(AtomicBool::new(false));
    let mut deltas = provider.send_cancellable(
        ChatRequest {
            user_message: "a silent turn".into(),
            ..Default::default()
        },
        Arc::clone(&cancel),
    );

    wait_for_pid_file(&pid_file);
    let pids = read_test_pids(&pid_file);
    assert_eq!(pids.len(), 2, "leader and descendant pids");

    cancel.store(true, std::sync::atomic::Ordering::Release);
    let started = std::time::Instant::now();
    assert!(deltas.next().is_none(), "cancelled iterator must terminate");
    // Far below the 30s the tree would otherwise live, which is the
    // property (cancellation is not tied to child lifetime) — but not so
    // tight that scheduler starvation under a loaded parallel test run
    // fails a cancellation that did work; a 1s bound flaked for real.
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "silent cancellation took {:?}",
        started.elapsed()
    );
    drop(deltas);

    assert_process_tree_reaped(&pids, "cancelled subprocess left descendants alive");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn cancelling_a_backpressured_prompt_write_reaps_its_process_tree() {
    let body = r#"#!/bin/sh
sleep 30 &
descendant=$!
printf '%s %s\n' "$$" "$descendant" > "$0.pids"
wait "$descendant"
"#;
    let (dir, binary) = stub_cli(body);
    let pid_file = std::path::PathBuf::from(format!("{}.pids", binary.to_string_lossy()));
    let provider = SubprocessProvider::for_cli(CliName::Codex)
        .expect("codex subprocess template")
        .with_test_binary(binary.to_string_lossy().into_owned());
    let cancel = Arc::new(AtomicBool::new(false));
    let mut deltas = provider.send_cancellable(
        ChatRequest {
            // Larger than normal pipe capacities, while neither process reads
            // stdin. Before the regression fix `feed().await` never observed
            // receiver cancellation and kept this tree alive.
            user_message: "x".repeat(8 * 1024 * 1024),
            ..Default::default()
        },
        Arc::clone(&cancel),
    );

    wait_for_pid_file(&pid_file);
    let pids = read_test_pids(&pid_file);
    assert_eq!(pids.len(), 2, "leader and descendant pids");

    cancel.store(true, std::sync::atomic::Ordering::Release);
    assert!(deltas.next().is_none(), "cancelled iterator must terminate");
    drop(deltas);
    assert_process_tree_reaped(&pids, "blocked stdin cancellation left processes alive");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn stdout_eof_does_not_allow_a_live_process_tree_to_outlast_exit_grace() {
    let body = r#"#!/bin/sh
cat >/dev/null
sleep 30 </dev/null >/dev/null 2>&1 &
descendant=$!
printf '%s %s\n' "$$" "$descendant" > "$0.pids"
exec 1>&-
exec 2>&-
wait "$descendant"
"#;
    let (dir, binary) = stub_cli(body);
    let pid_file = std::path::PathBuf::from(format!("{}.pids", binary.to_string_lossy()));
    let provider = SubprocessProvider::for_cli(CliName::Codex)
        .expect("codex subprocess template")
        .with_test_binary(binary.to_string_lossy().into_owned());

    let started = std::time::Instant::now();
    let _deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "close stdout, then stay alive".into(),
            ..Default::default()
        })
        .collect();
    // The elapsed bound covers the WHOLE turn, not just the post-EOF
    // reap: spawn, prompt feed, EOF detection, and up to two EXIT_GRACE
    // waits (~4s of legitimate fixed budget) all land inside it, and a
    // loaded machine adds scheduling/timer lag on top. What the test must
    // prove is only that the turn never waits out the still-live 30s
    // descendant after stdout EOF — so the bound stays far below the
    // descendant's lifetime while leaving real headroom over the fixed
    // grace budget. The previous 15s-sleep/8s-bound pairing left ~4s of
    // slack and flaked under parallel `cargo test` on loaded machines.
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "post-EOF child wait exceeded exit grace: {:?}",
        started.elapsed()
    );
    let pids = read_test_pids(&pid_file);
    assert_process_tree_reaped(&pids, "post-EOF cleanup left processes alive");
    let _ = std::fs::remove_dir_all(dir);
}

/// The unknown-status rule, kept as a test because the platform that exposed
/// it (Windows `cmd /c <missing-binary>`) is not the platform most of this is
/// developed on — a regression here would otherwise only surface on CI, as a
/// silent `Done` rather than an error.
#[test]
fn an_unfinished_child_with_no_readable_status_is_a_failure() {
    assert!(
        crate::chat_subprocess_exit::unfinished_child_is_failure(None),
        "a child that neither finished cleanly nor left a readable status must \
         be reported, never passed off as success"
    );
}

#[test]
fn a_clean_exit_on_the_unfinished_path_is_not_a_failure() {
    let ok = std::process::Command::new(if cfg!(windows) { "cmd" } else { "true" })
        .args(if cfg!(windows) {
            vec!["/c", "exit 0"]
        } else {
            vec![]
        })
        .status()
        .expect("spawn a trivially successful process");
    assert!(!crate::chat_subprocess_exit::unfinished_child_is_failure(
        Some(&ok)
    ));
}

#[test]
fn a_nonzero_exit_on_the_unfinished_path_is_a_failure() {
    let bad = std::process::Command::new(if cfg!(windows) { "cmd" } else { "false" })
        .args(if cfg!(windows) {
            vec!["/c", "exit 1"]
        } else {
            vec![]
        })
        .status()
        .expect("spawn a trivially failing process");
    assert!(crate::chat_subprocess_exit::unfinished_child_is_failure(
        Some(&bad)
    ));
}

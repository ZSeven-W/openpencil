use super::*;

/// Deliberately short: these assert on the deadline firing, not on
/// any real shell finishing.
const TIMEOUT: Duration = Duration::from_millis(400);

#[test]
fn capture_reads_stdout_and_tolerates_stderr_noise() {
    let script = "printf 'A=1\\nB=2\\n'; printf '_encode: command not found\\n' >&2";
    let EnvDump::Completed(text) = capture_env_dump("/bin/sh", &["-c", script], TIMEOUT) else {
        panic!("a script that exits inside the budget must complete");
    };
    let map = parse_env_dump(&text);
    assert_eq!(map.get("A").map(String::as_str), Some("1"));
    assert_eq!(map.get("B").map(String::as_str), Some("2"));
}

#[test]
fn completed_env_dump_does_not_wait_for_descendant_pipe_eof() {
    // A shell rc can fork a helper and return. The helper inherits both
    // pipes, so leader exit alone does not make a blocking reader join
    // safe. Preserve the dump and finish without waiting for `sleep`.
    let script = "printf 'A=1\\n'; sleep 30 & exit 0";
    let started = std::time::Instant::now();
    let dump = capture_env_dump("/bin/sh", &["-c", script], Duration::from_secs(10));
    let elapsed = started.elapsed();
    let EnvDump::Completed(text) = dump else {
        panic!("an exited shell with a valid dump must complete");
    };
    assert_eq!(
        parse_env_dump(&text).get("A").map(String::as_str),
        Some("1")
    );
    assert!(
        elapsed < Duration::from_secs(15),
        "the helper's pipe EOF must not control startup; took {elapsed:?}"
    );
}

#[test]
fn capture_times_out_instead_of_hanging_on_a_blocking_rc() {
    let started = std::time::Instant::now();
    let dump = capture_env_dump("/bin/sh", &["-c", "sleep 30"], TIMEOUT);
    assert!(
        matches!(dump, EnvDump::TimedOut),
        "a shell still running at the deadline must be killed, not awaited"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the deadline, not the child, decides when the probe returns"
    );
}

#[test]
fn capture_reports_failure_for_nonzero_exit_and_missing_program() {
    assert!(matches!(
        capture_env_dump("/bin/sh", &["-c", "printf 'A=1\\n'; exit 3"], TIMEOUT),
        EnvDump::Failed
    ));
    assert!(matches!(
        capture_env_dump(
            "/nonexistent/shell-that-is-not-installed",
            &["-ilc"],
            TIMEOUT
        ),
        EnvDump::Failed
    ));
}

#[test]
fn a_dump_with_no_entries_falls_back_to_the_unrepaired_env() {
    // The fallback contract behind `login_shell_env`: an empty parse
    // must read as "no login-shell env", so `effective_path_env`
    // keeps the current process PATH instead of clearing it.
    let EnvDump::Completed(text) =
        capture_env_dump("/bin/sh", &["-c", "printf 'no entries here\\n'"], TIMEOUT)
    else {
        panic!("the script exits inside the budget");
    };
    assert!(parse_env_dump(&text).is_empty());
    assert_eq!(
        effective_path_env().is_empty(),
        std::env::var("PATH").unwrap_or_default().is_empty()
    );
}

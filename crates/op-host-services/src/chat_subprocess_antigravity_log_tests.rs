use super::*;

/// Verbatim shape of what the CLI wrote on 2026-08-27 when a turn died on a
/// server-side precondition — including the `logging before google.Init`
/// prefix it stamps on every line and the duplicated cause.
const REAL_TAIL: &str = concat!(
    "ERROR: logging before google.Init: I0827 20:54:34.894019       1 conversation_manager.go:748] Streaming conversation 6cbd7aa6\n",
    "ERROR: logging before google.Init: W0827 20:54:34.903557     360 declarative_config_loader.go:251] skipping component during resolution: empty component: prompt section \"user_rules\"\n",
    "ERROR: logging before google.Init: E0827 20:54:36.388855     360 errorreport.go:223] agent executor error: calling model: FAILED_PRECONDITION (code 400): User location is not supported for the API use.\n",
    "ERROR: logging before google.Init: E0827 20:54:36.389446     360 errorreport.go:223] calling model: FAILED_PRECONDITION (code 400): User location is not supported for the API use.\n",
    "ERROR: logging before google.Init: I0827 20:54:36.502161       1 manager.go:751] CLI store manager shutting down\n",
);

/// Same temp-dir idiom the sibling safety tests use — no dev-dependency, and
/// the name is unique per process and per call.
fn write_log(label: &str, body: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "openpencil-agylog-{label}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("agy.log");
    std::fs::write(&path, body).expect("log written");
    path
}

#[test]
fn the_real_cause_is_lifted_out_of_a_verbose_log() {
    let path = write_log("cause", REAL_TAIL);
    let found = antigravity_log_error(&path).expect("an error line is present");
    assert!(
        found.contains("User location is not supported for the API use"),
        "the actual cause must survive: {found}"
    );
    assert!(
        !found.contains("Streaming conversation"),
        "info lines must not be quoted: {found}"
    );
    assert!(
        !found.contains("skipping component"),
        "warnings must not be quoted — a failing turn logs many: {found}"
    );
}

#[test]
fn the_duplicated_cause_is_quoted_once() {
    let path = write_log("dedup", REAL_TAIL);
    let found = antigravity_log_error(&path).expect("an error line is present");
    let occurrences = found.matches("User location is not supported").count();
    assert_eq!(
        occurrences, 1,
        "the executor wrapper and the inner call log the same cause: {found}"
    );
}

#[test]
fn a_log_with_nothing_wrong_says_nothing() {
    let path = write_log(
        "quiet",
        "ERROR: logging before google.Init: I0827 20:54:34.894019 1 server.go:1] all good\n",
    );
    assert_eq!(antigravity_log_error(&path), None);
}

#[test]
fn a_missing_log_is_silent_not_an_error() {
    // The log is a diagnostic aid. Failing to read it must never displace the
    // failure the caller is already reporting.
    let present = write_log("absent", "");
    let missing = present.parent().expect("temp dir").join("absent.log");
    assert_eq!(antigravity_log_error(&missing), None);
}

#[test]
fn a_huge_log_is_read_from_the_tail() {
    let mut body = String::new();
    for i in 0..40_000 {
        body.push_str(&format!(
            "ERROR: logging before google.Init: I0827 20:54:34.000000 1 noise.go:{i}] filler line {i}\n"
        ));
    }
    body.push_str(REAL_TAIL);
    let path = write_log("huge", &body);
    assert!(std::fs::metadata(&path).unwrap().len() > LOG_TAIL_BYTES);
    let found = antigravity_log_error(&path).expect("the tail carries the error");
    assert!(found.contains("User location is not supported"), "{found}");
}

#[test]
fn the_quote_is_length_capped() {
    let long = "x".repeat(4_000);
    let path = write_log(
        "capped",
        &format!("ERROR: logging before google.Init: E0827 20:54:36.0 1 a.go:1] {long}\n"),
    );
    let found = antigravity_log_error(&path).expect("an error line is present");
    assert!(
        found.chars().count() <= MAX_CHARS,
        "the quote lands in a chat bubble: {}",
        found.chars().count()
    );
}

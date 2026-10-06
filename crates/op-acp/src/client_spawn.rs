//! Local ACP agent spawn with a bounded retry for the Linux `ETXTBSY`
//! fork/exec race. A sibling module so `client.rs` stays under the
//! 800-line cap.

use std::io;
use std::time::Duration;

use tokio::process::{Child, Command};

/// ETXTBSY spawn retry budget: 10 attempts, 20 ms apart (~200 ms total).
const SPAWN_ATTEMPTS: usize = 10;
/// Pause between ETXTBSY spawn attempts.
const SPAWN_BACKOFF: Duration = Duration::from_millis(20);

/// Spawn a local agent command, absorbing a transient `ETXTBSY`
/// ("Text file busy") failure.
///
/// A concurrent `fork` can inherit this process's still-open write fd on
/// a freshly written agent script (unique temp dirs written by parallel
/// test threads, or a user replacing an agent binary between runs); the
/// kernel refuses to `exec` the file until that child execs, and the
/// race clears within milliseconds. Only that error is retried, up to
/// [`SPAWN_ATTEMPTS`] — bounded so the async caller blocks briefly.
/// Every other error, and the final ETXTBSY once the budget is gone,
/// is surfaced unchanged.
pub(super) fn spawn_with_etxtbsy_retry(cmd: &mut Command) -> io::Result<Child> {
    retry_etxtbsy(|| cmd.spawn())
}

/// Drive `spawn` against the ETXTBSY retry budget. Generic so tests can
/// inject a fake spawner instead of reproducing the kernel race.
fn retry_etxtbsy<T>(mut spawn: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    // `1..SPAWN_ATTEMPTS` retries plus this final attempt = 10 spawns.
    for _ in 1..SPAWN_ATTEMPTS {
        match spawn() {
            Err(error) if is_etxtbsy(&error) => std::thread::sleep(SPAWN_BACKOFF),
            result => return result,
        }
    }
    spawn()
}

/// ETXTBSY by stable `ErrorKind`, or by raw errno — 26 on both Linux and
/// macOS; older std mappings can also leave the kind `Uncategorized`.
#[cfg(unix)]
fn is_etxtbsy(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::ExecutableFileBusy || error.raw_os_error() == Some(26)
}

/// Windows has no ETXTBSY; the kind check is the portable half.
#[cfg(not(unix))]
fn is_etxtbsy(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::ExecutableFileBusy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etxtbsy_is_retried_until_the_spawn_succeeds() {
        // Built by kind, not raw errno: on Windows errno 26 in the raw
        // namespace is ERROR_NOT_READY, an unrelated condition.
        let mut script = [
            Err(io::Error::new(
                io::ErrorKind::ExecutableFileBusy,
                "text file busy",
            )),
            Err(io::Error::new(
                io::ErrorKind::ExecutableFileBusy,
                "text file busy",
            )),
            Ok(()),
        ]
        .into_iter();
        let mut calls = 0usize;
        let spawned = retry_etxtbsy(|| {
            calls += 1;
            script.next().unwrap()
        });
        assert!(spawned.is_ok(), "the third spawn attempt must succeed");
        assert_eq!(calls, 3, "two ETXTBSY retries, then the successful spawn");
    }

    #[test]
    fn other_spawn_errors_surface_immediately_and_unchanged() {
        let mut calls = 0usize;
        let result: io::Result<()> = retry_etxtbsy(|| {
            calls += 1;
            Err(io::Error::new(io::ErrorKind::NotFound, "no such agent"))
        });
        let error = result.unwrap_err();
        assert_eq!(calls, 1, "a non-ETXTBSY error must not be retried");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert_eq!(error.to_string(), "no such agent");
    }

    #[test]
    fn etxtbsy_past_the_budget_surfaces_the_original_error() {
        let mut calls = 0usize;
        let result: io::Result<()> = retry_etxtbsy(|| {
            calls += 1;
            Err(io::Error::new(
                io::ErrorKind::ExecutableFileBusy,
                "still busy",
            ))
        });
        assert_eq!(calls, SPAWN_ATTEMPTS, "spawn attempts are capped");
        let error = result.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::ExecutableFileBusy);
        assert_eq!(
            error.to_string(),
            "still busy",
            "the original error is surfaced after the budget is gone"
        );
    }

    /// The raw errno arm of the classifier. Unix-only: errno 26 is
    /// `ETXTBSY` on Linux and macOS, while 13 (`EACCES`) must stay
    /// unclassified so a permissions problem is never retried.
    #[cfg(unix)]
    #[test]
    fn raw_etxtbsy_errno_classifies_as_busy() {
        assert!(is_etxtbsy(&io::Error::from_raw_os_error(26)));
        assert!(!is_etxtbsy(&io::Error::from_raw_os_error(13)));
        assert!(!is_etxtbsy(&io::Error::new(
            io::ErrorKind::NotFound,
            "no such agent"
        )));
    }
}

//! Environment selection and terminal cleanup shared by subprocess CLIs.

use op_ai::chat_provider::{ChatDelta, CliName};
use op_process_io::LineStreamChild;
use tokio::sync::mpsc;

use crate::chat_subprocess_quirks as quirks;
use crate::chat_subprocess_safety as safety;

pub(crate) fn child_env_for_cli(cli: Option<CliName>) -> Vec<(String, String)> {
    match cli {
        Some(CliName::Codex) => quirks::codex_child_env(),
        Some(CliName::Antigravity | CliName::GrokBuild | CliName::Dsh) => {
            safety::child_env(cli).unwrap_or_default()
        }
        _ => crate::chat_spawn::scrubbed_child_env(),
    }
}

/// Reap a child after an in-stream terminal event without letting a CLI keep
/// the chat task alive. Receiver cancellation wins immediately; otherwise the
/// caller-provided deadline bounds the wait before the full process tree is
/// force-terminated by [`LineStreamChild::start_kill`].
pub(crate) async fn wait_for_terminal_exit(
    child: &mut LineStreamChild,
    deadline: tokio::time::Instant,
    tx: &mpsc::Sender<ChatDelta>,
) -> Option<std::process::ExitStatus> {
    let budget = deadline.saturating_duration_since(tokio::time::Instant::now());
    {
        let graceful = child.kill_graceful(budget);
        tokio::pin!(graceful);
        tokio::select! {
            biased;
            _ = tx.closed() => {},
            status = &mut graceful => return status.ok(),
            _ = tokio::time::sleep_until(deadline) => {},
        }
    }
    let _ = child.start_kill();
    tokio::time::timeout(safety::EXIT_GRACE, child.wait())
        .await
        .ok()?
        .ok()
}

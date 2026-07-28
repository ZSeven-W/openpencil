//! Cancellation and serialization for memory-heavy Figma import workers.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// Process-local gate for the decode and conversion phases. A cancelled
/// worker may still be inside non-interruptible Kiwi decode, so its
/// replacement must wait rather than doubling peak memory.
static WORKER_GATE: Mutex<()> = Mutex::new(());

#[derive(Clone, Default)]
pub(super) struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub(super) fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    /// Wait for the prior heavy phase, then re-check cancellation before
    /// allowing this worker to allocate its source tree.
    pub(super) fn worker_permit(&self) -> Option<MutexGuard<'static, ()>> {
        if self.is_cancelled() {
            return None;
        }
        let permit = WORKER_GATE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (!self.is_cancelled()).then_some(permit)
    }
}

/// Wait until no Figma prepare/convert/publish worker owns the process gate.
///
/// Collaboration transitions call this only after cancelling the associated
/// token. Once it returns, an old standalone import cannot publish an
/// adjacent `.op` after the shared session becomes active.
pub(super) fn wait_until_idle() {
    drop(
        WORKER_GATE
            .lock()
            .unwrap_or_else(|error| error.into_inner()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn replacement_cannot_overlap_cancelled_live_worker() {
        let old_token = CancellationToken::default();
        let old_permit = old_token.worker_permit().unwrap();
        old_token.cancel();
        assert!(
            WORKER_GATE.try_lock().is_err(),
            "cancelling must not release a still-running heavy phase"
        );

        drop(old_permit);
        let replacement = CancellationToken::default();
        assert!(replacement.worker_permit().is_some());
    }

    #[test]
    fn cancelled_waiter_never_enters_heavy_phase() {
        let permit = WORKER_GATE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let token = CancellationToken::default();
        let worker_token = token.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (entered_tx, entered_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            ready_tx.send(()).unwrap();
            if let Some(_permit) = worker_token.worker_permit() {
                entered_tx.send(()).unwrap();
            }
        });

        ready_rx.recv().unwrap();
        token.cancel();
        drop(permit);
        worker.join().unwrap();
        assert!(
            entered_rx.try_recv().is_err(),
            "a cancelled replacement must not enter after the prior worker exits"
        );
    }
}

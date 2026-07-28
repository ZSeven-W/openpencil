use std::time::{Duration, Instant};

use op_collab::{Bye, ByeReason, CollabMessage, Epoch, FrameEnvelope, SessionId};
use op_collab_transport::{ConnectionDriver, QueueError, RuntimeError};

use super::TERMINAL_FLUSH_TIMEOUT;

/// One authenticated terminal frame followed by a bounded socket drain.
pub(super) struct TerminalDrain {
    frame: FrameEnvelope,
    deadline: Instant,
    queued: bool,
}

impl TerminalDrain {
    pub(super) fn new(
        session_id: SessionId,
        epoch: Epoch,
        reason: ByeReason,
        now: Instant,
    ) -> Self {
        Self {
            frame: FrameEnvelope::new(session_id, epoch, CollabMessage::Bye(Bye { reason })),
            deadline: now
                .checked_add(TERMINAL_FLUSH_TIMEOUT)
                .unwrap_or(now + Duration::from_millis(1)),
            queued: false,
        }
    }

    pub(super) const fn deadline(&self) -> Instant {
        self.deadline
    }

    #[cfg(test)]
    pub(super) const fn queued(&self) -> bool {
        self.queued
    }

    pub(super) fn try_queue(
        &mut self,
        driver: &mut ConnectionDriver,
        now: Instant,
    ) -> Result<(), RuntimeError> {
        if self.queued {
            return Ok(());
        }
        self.accept_queue_result(driver.queue_frame(&self.frame, now))
    }

    fn accept_queue_result(
        &mut self,
        result: Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        if self.queued {
            return Ok(());
        }
        match result {
            Ok(()) => {
                self.queued = true;
                Ok(())
            }
            Err(RuntimeError::Queue(QueueError::Full | QueueError::ByteBudget)) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub(super) fn complete(&self, driver: &ConnectionDriver, now: Instant) -> bool {
        now >= self.deadline || self.queued && !driver.has_pending_output()
    }

    #[cfg(test)]
    fn reason(&self) -> ByeReason {
        let CollabMessage::Bye(bye) = self.frame.body() else {
            unreachable!("terminal drain always owns Bye")
        };
        bye.reason
    }
}

pub(super) const fn retirement_ready(stop_requested: bool, has_pending_output: bool) -> bool {
    stop_requested && !has_pending_output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_queue_retries_the_exact_terminal_reason() {
        let now = Instant::now();
        let mut drain = TerminalDrain::new(
            SessionId::from("terminal-retry"),
            Epoch(4),
            ByeReason::AuthenticationExpired,
            now,
        );
        drain
            .accept_queue_result(Err(QueueError::Full.into()))
            .unwrap();
        assert!(!drain.queued());
        drain.accept_queue_result(Ok(())).unwrap();
        assert!(drain.queued());
        assert_eq!(drain.reason(), ByeReason::AuthenticationExpired);
    }

    #[test]
    fn disconnected_normal_lane_waits_for_prequeued_output() {
        assert!(!retirement_ready(true, true));
        assert!(retirement_ready(true, false));
        assert!(!retirement_ready(false, false));
    }

    #[test]
    fn terminal_flush_has_a_short_monotonic_deadline() {
        let now = Instant::now();
        let drain = TerminalDrain::new(
            SessionId::from("terminal-deadline"),
            Epoch(1),
            ByeReason::Normal,
            now,
        );
        assert!(drain.deadline() > now);
        assert!(drain.deadline() <= now + Duration::from_secs(2));
    }
}

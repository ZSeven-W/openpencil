//! Browser preview-session frame pumping.

#![cfg_attr(not(feature = "canvaskit"), allow(dead_code))]

/// Whether a preview pump should keep requesting animation frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PumpSchedule {
    Schedule,
    Idle,
}

/// Turn the session's wake deadline into the pump's self-rescheduling choice.
/// A missing deadline is the idle state; it must not become a busy rAF loop.
pub(crate) fn schedule_for_deadline(deadline_ms: Option<u64>) -> PumpSchedule {
    if deadline_ms.is_some() {
        PumpSchedule::Schedule
    } else {
        PumpSchedule::Idle
    }
}

/// The paint-side self-perpetuating clauses: mode merge, screen transition,
/// and one preview-session wake clause. Keeping the preview clause here makes
/// its single contribution explicit and unit-testable.
pub(crate) fn needs_next_paint(
    mode_animating: bool,
    screen_animating: bool,
    preview_wake_pending: bool,
) -> bool {
    mode_animating || screen_animating || preview_wake_pending
}

#[cfg(feature = "canvaskit")]
mod runtime {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use crate::repaint_ctx::RepaintContext;

    thread_local! {
        /// One pump per mounted page. The rAF closure owns itself and this is
        /// only a duplicate-start guard.
        static RUNNING: Cell<bool> = const { Cell::new(false) };
    }

    /// Start the self-terminating preview pump when the session has work.
    pub(crate) fn ensure<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
        {
            let Ok(borrowed) = inner.try_borrow() else {
                return;
            };
            let host_deadline = borrowed.host().next_animation_deadline_ms();
            if !matches!(
                super::schedule_for_deadline(borrowed.host().preview_wake_deadline_ms()),
                super::PumpSchedule::Schedule
            ) || host_deadline.is_none()
            {
                return;
            }
        }
        if RUNNING.with(Cell::get) {
            return;
        }
        RUNNING.with(|running| running.set(true));

        let inner = inner.clone();
        crate::raf_pump::start(Rc::new(move || {
            let Ok(mut inner_mut) = inner.try_borrow_mut() else {
                // A DOM event owns the shell; retry on the next browser frame.
                return true;
            };
            let now = crate::listener::now_ms_perf();
            let unix = crate::listener::now_unix_secs();
            inner_mut.host_mut().set_clocks(now, unix);
            let _ = inner_mut.repaint();

            // The paint pass calls PreviewSession::pump exactly once for this
            // painted frame. Check afterward so the terminal frame is painted
            // before the pump goes idle.
            let alive = matches!(
                super::schedule_for_deadline(inner_mut.host().preview_wake_deadline_ms()),
                super::PumpSchedule::Schedule
            );
            if !alive {
                RUNNING.with(|running| running.set(false));
            }
            alive
        }));
    }
}

#[cfg(feature = "canvaskit")]
pub(crate) use runtime::ensure;

#[cfg(test)]
mod tests {
    use super::{needs_next_paint, schedule_for_deadline, PumpSchedule};

    #[test]
    fn missing_deadline_idles_the_preview_pump() {
        assert_eq!(schedule_for_deadline(None), PumpSchedule::Idle);
    }

    #[test]
    fn reported_deadline_schedules_the_preview_pump() {
        assert_eq!(schedule_for_deadline(Some(16)), PumpSchedule::Schedule);
    }

    #[test]
    fn paint_clause_includes_one_live_preview_wake() {
        assert!(needs_next_paint(false, false, true));
        assert!(!needs_next_paint(false, false, false));
    }
}

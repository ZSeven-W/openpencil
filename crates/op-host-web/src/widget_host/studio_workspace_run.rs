//! The Studio workspace's phase pump on the web host.
//!
//! Desktop observes a run's "everything is done" edge every frame from its
//! winit loop (`app_handler/redraw.rs`). The browser has no such loop: the
//! `studio_web` rAF pump calls [`WidgetHost::drive_workspace_run`] each frame
//! while a run is Generating and stops once the phase settles.
//!
//! One thing differs from desktop and is handled here: on web the design is
//! drawn by the serving daemon and reaches this document through the 400 ms
//! live-sync pull, which can land AFTER the chat stream's terminal event. A
//! verdict taken on that edge would count the boards before they arrive and
//! call a successful run 失败. The pump therefore keeps treating the run as
//! generating for [`WEB_SETTLE_GRACE_MS`] after the turn goes idle (the
//! camera keeps tracking the boards as they land), and settles after that.
//! An errored turn settles immediately — there is nothing to wait for.

use super::WidgetHost;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::workspace_run::{
    assistant_streaming, awaiting_launch, last_assistant_failed, produced_board_count,
};
use op_editor_core::{ChatRole, QualityReport, WorkspacePhase};
use op_editor_ui::widgets::host_overlay_geometry::zoom_to_fit;

/// How long the pump keeps a run open after its turn went idle, so the
/// live-sync pull can deliver the boards the daemon drew. Five pull ticks.
pub(crate) const WEB_SETTLE_GRACE_MS: u64 = 2_000;

/// What one pump frame decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceRunTick {
    /// Something visible changed; repaint.
    pub changed: bool,
    /// The run is still Generating; keep the pump alive.
    pub keep_pumping: bool,
}

impl WidgetHost {
    /// Whether a workspace run is waiting on the pump.
    pub fn workspace_run_generating(&self) -> bool {
        let workspace = &self.editor_state.editor_ui.workspace;
        workspace.active && workspace.phase == WorkspacePhase::Generating
    }

    /// Stamp a newly launched chat turn onto the workspace it belongs to: a
    /// follow-up or retry puts the workspace back into Generating under the
    /// turn's own epoch, so a stopped turn's late edges cannot settle it.
    pub fn stamp_workspace_run(&mut self, epoch: u64) -> bool {
        let workspace = &mut self.editor_state.editor_ui.workspace;
        if !workspace.active || workspace.run_epoch == epoch {
            return false;
        }
        workspace.resume_generating(epoch);
        self.workspace_idle_since_ms = None;
        self.mark_dirty();
        true
    }

    /// The user pressed Stop: a generating workspace is now Stopped, and the
    /// idle edge that follows must not flip it to Done or Failed.
    pub fn stop_workspace_run(&mut self) -> bool {
        let epoch = self.editor_state.editor_ui.workspace.run_epoch;
        if self.editor_state.editor_ui.workspace.mark_stopped(epoch) {
            self.workspace_idle_since_ms = None;
            self.mark_dirty();
            return true;
        }
        false
    }

    /// The daemon's audited quality report for the chat turn launched as
    /// `epoch`. The workspace keeps it only when that turn is still its run
    /// (the chip then shows once the run settles Done); a report for an
    /// older run is never pinned on a newer one. Either way the turn's own
    /// streaming reply gets the one-line summary, as desktop appends it.
    pub fn apply_run_quality(
        &mut self,
        epoch: u64,
        running_tab: Option<usize>,
        report: QualityReport,
    ) -> bool {
        let locale = self.editor_state.editor_ui.effective_locale();
        let line = (!report.is_empty()).then(|| report.transcript_line(locale));
        let mut changed = false;
        if let Some(line) = line {
            let chat = self.editor_state.chat.run_tab_mut(running_tab);
            if let Some(reply) = chat.messages.iter_mut().rev().find(|m| m.streaming) {
                if !reply.content.contains(&line) {
                    if !reply.content.trim().is_empty() {
                        reply.content.push_str("\n\n");
                    }
                    reply.content.push_str(&line);
                    changed = true;
                }
            }
        }
        let workspace = &mut self.editor_state.editor_ui.workspace;
        if workspace.active && workspace.run_epoch == epoch {
            workspace.quality = Some(report);
            workspace.quality_open = false;
            changed = true;
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// One pump frame. `turn_in_flight` is whether the web chat still holds
    /// an open stream for this page.
    pub fn drive_workspace_run(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
        turn_in_flight: bool,
        now_ms: u64,
    ) -> WorkspaceRunTick {
        if !self.workspace_run_generating() {
            self.workspace_idle_since_ms = None;
            return WorkspaceRunTick {
                changed: false,
                keep_pumping: false,
            };
        }
        let busy = turn_in_flight
            || awaiting_launch(&self.editor_state)
            || assistant_streaming(&self.editor_state);
        let failed = !busy && self.last_turn_failed();
        let generating = if busy {
            self.workspace_idle_since_ms = None;
            true
        } else {
            let since = *self.workspace_idle_since_ms.get_or_insert(now_ms);
            !failed && now_ms.saturating_sub(since) < WEB_SETTLE_GRACE_MS
        };
        let mut changed = self.pump_workspace_generation(viewport_w, viewport_h, generating);
        if generating {
            return WorkspaceRunTick {
                changed,
                keep_pumping: true,
            };
        }
        self.workspace_idle_since_ms = None;
        let boards = produced_board_count(&self.editor_state);
        changed |= self.settle_workspace_idle_edge(boards, failed, viewport_w, viewport_h);
        WorkspaceRunTick {
            changed,
            keep_pumping: false,
        }
    }

    /// The desktop verdict plus the web transport's own failure shape: a
    /// stream that errored ends with its bubble rewritten to `error: …`.
    fn last_turn_failed(&self) -> bool {
        if last_assistant_failed(&self.editor_state) {
            return true;
        }
        self.editor_state
            .chat
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ChatRole::Assistant)
            .is_some_and(|message| message.content.trim_start().starts_with("error:"))
    }

    /// Refresh boards, clamp the selection, and — while generating — refit
    /// the camera whenever the content moves, so the user watches boards
    /// land. Mirrors the native pump; the scene is only rebuilt while a run
    /// is live.
    pub(in crate::widget_host) fn pump_workspace_generation(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
        generating: bool,
    ) -> bool {
        let workspace = &self.editor_state.editor_ui.workspace;
        if !workspace.active || !workspace.visible {
            return false;
        }
        let count = active_page_boards(&self.editor_state).len();
        let selected = workspace.selected.min(count.saturating_sub(1));
        if generating {
            self.refresh_layout_scene();
        }
        let bounds = generating
            .then(|| self.layout_scene.content_bounds())
            .flatten()
            .map(|content| {
                (
                    content.origin.x,
                    content.origin.y,
                    content.size.x,
                    content.size.y,
                )
            });
        let bounds_moved = match (bounds, self.editor_state.editor_ui.workspace.fitted_bounds) {
            (Some(now), Some(then)) => {
                (now.0 - then.0).abs()
                    + (now.1 - then.1).abs()
                    + (now.2 - then.2).abs()
                    + (now.3 - then.3).abs()
                    > 0.5
            }
            (Some(_), None) => true,
            _ => false,
        };
        let refit_due = generating && count > 0 && bounds_moved;
        if !refit_due && selected == self.editor_state.editor_ui.workspace.selected {
            return false;
        }
        let workspace = &mut self.editor_state.editor_ui.workspace;
        workspace.selected = selected;
        if refit_due {
            workspace.fitted_board_count = count;
            workspace.fitted_bounds = bounds;
            // While the run streams, "watch the boards land" is an
            // all-boards camera whatever view the family settles into.
            zoom_to_fit(
                &mut self.editor_state,
                &self.layout_scene,
                viewport_w,
                viewport_h,
            );
        }
        self.mark_dirty();
        true
    }

    /// Resolve the phase on the idle edge: zero boards or an error is a
    /// failure. A finished run hands the camera to the family's own view.
    fn settle_workspace_idle_edge(
        &mut self,
        board_count: usize,
        failed: bool,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let workspace = &mut self.editor_state.editor_ui.workspace;
        let epoch = workspace.run_epoch;
        let changed = if board_count == 0 || failed {
            workspace.mark_failed(epoch)
        } else {
            workspace.mark_done(epoch)
        };
        if changed {
            if self.editor_state.editor_ui.workspace.phase == WorkspacePhase::Done {
                self.apply_workspace_fit(viewport_w, viewport_h);
            }
            self.mark_dirty();
        }
        changed
    }
}

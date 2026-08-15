//! Event-loop scheduling + render-context bring-up for `DesktopApp`.
//! Carved out of the `app_handler.rs` spine to keep it under the
//! 800-line cap; pure code motion, no behavior change.

use crate::{figma_import_session, git_jobs, DesktopApp, INITIAL_VIEWPORT_H, INITIAL_VIEWPORT_W};
use op_host_native::{NativeBackend, ProviderError, SharedSkiaContext, SharedSkiaError};
use std::time::{Duration, Instant};
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, ControlFlow};

fn render_surface_not_ready(err: &SharedSkiaError) -> bool {
    matches!(
        err,
        SharedSkiaError::Provider(ProviderError::SurfaceNotReady { .. })
    )
}

/// Pop a native error dialog when the GL/Skia render context can't be
/// built, so a fatal graphics-init failure is visible instead of a
/// silent flash-exit. This matters most on Windows: the binary is a GUI
/// subsystem app (`windows_subsystem = "windows"`) with no attached
/// console, so `eprintln!` / tracing-to-stderr produce no output on a
/// double-click launch. Bilingual EN/中文 body — the failure happens
/// before the editor locale is meaningfully in play, and it must read
/// for the affected users (typically machines with a missing/old GPU
/// driver or a software-only OpenGL renderer).
fn show_gpu_init_error_dialog(err: &SharedSkiaError) {
    let body = format!(
        "OpenPencil could not initialize the graphics (OpenGL) backend and has to close.\n\
         无法初始化图形 (OpenGL) 渲染引擎，程序即将退出。\n\n\
         This usually means the GPU driver is missing or too old, or the machine only \
         has a software renderer (common on virtual machines / remote desktop).\n\
         通常是显卡驱动缺失或过旧，或运行在只有软件渲染的环境（虚拟机 / 远程桌面）。\n\n\
         Please update your graphics driver and try again.\n\
         请更新显卡驱动后重试。\n\n\
         Details: {err}"
    );
    crate::message_dialog::alert(
        "OpenPencil — Graphics initialization failed / 图形初始化失败",
        &body,
        rfd::MessageLevel::Error,
    );
}

impl DesktopApp {
    pub(super) fn refresh_host_clock(&mut self) {
        let now_ms = self.clock_start.elapsed().as_millis() as u64;
        self.host.set_now_ms(now_ms);
    }

    /// Next wake instant on a fixed `period_ms` grid anchored at
    /// `clock_start`. Consecutive loop turns inside one period produce
    /// the SAME instant, so casement's `EventLoopWaker::start_at`
    /// dedup skips the CFRunLoop timer re-arm — a sliding
    /// `Instant::now() + period` deadline re-armed the OS timer on
    /// every loop turn (measured ~40% of the main thread during a
    /// gesture on a 38k-node document).
    pub(super) fn periodic_wake_instant(&self, period_ms: u64) -> Instant {
        let elapsed = self.clock_start.elapsed().as_millis() as u64;
        self.clock_start + Duration::from_millis(((elapsed / period_ms) + 1) * period_ms)
    }

    /// Choose the event loop's next wake based on active work. Called
    /// at the end of every painted frame AND after a timed wake that
    /// produced no redraw — leaving an elapsed `WaitUntil` in place
    /// made casement re-fire it immediately, spinning the loop at
    /// full speed until the next real event.
    pub(super) fn schedule_next_wake(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.collab_runtime.next_presence_deadline() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            return;
        }
        // Chat / design / Figma-import worker active → wake ~30 fps to
        // pump results and animate the loading overlay's spinner.
        if self.current_chat.is_some()
            || self.current_design.is_some()
            || self.current_codegen.is_some()
            || self.current_design_md.is_some()
            || !self.sub_agents.is_empty()
            // Poll the drag position while an image file hovers the window.
            || self.hovered_image_drop
        {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.periodic_wake_instant(33)));
        } else if self
            .current_figma_import
            .as_ref()
            .is_some_and(figma_import_session::FigmaImportSession::is_worker_pending)
            || self.current_html_import.is_some()
            || self.pending_figma_paste.is_some()
            || self.pending_html_paste.is_some()
        {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.periodic_wake_instant(100)));
        } else if let Some(deadline_ms) = self.host.next_animation_deadline_ms() {
            let deadline = self.clock_start + Duration::from_millis(deadline_ms);
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        } else if self.update_probe.is_pending()
            || self.host.auth_flow_active()
            || self.model_probe.is_pending()
            || self.image_search.is_pending()
            || self.image_panel.is_pending()
            // Remote-image fetches in flight, or fresh misses the last
            // paint recorded (the next pump picks them up).
            || self.remote_images.is_pending()
            || op_editor_ui::widgets::canvas_viewport_image::has_pending_remote_image_requests()
            || self.collab_avatars.is_pending()
            || op_editor_ui::collab_avatar_runtime::has_pending_collab_avatar_requests()
            || self.image_decodes.is_pending()
            || op_editor_ui::widgets::canvas_viewport_image::has_pending_decodes()
            || self
                .iconify_job
                .as_ref()
                .is_some_and(crate::iconify_host::IconifyJob::is_pending)
            || self.provider_connect_pending()
            || self.acp_agent_connect_pending()
            || self.builtin_model_refresh_pending()
            || self
                .git_pull_job
                .as_ref()
                .is_some_and(git_jobs::GitPullJob::is_pending)
            || self
                .git_push_job
                .as_ref()
                .is_some_and(git_jobs::GitPushJob::is_pending)
            || self
                .git_status_job
                .as_ref()
                .is_some_and(git_jobs::GitStatusJob::is_pending)
            || self
                .git_diff_job
                .as_ref()
                .is_some_and(git_jobs::GitDiffJob::is_pending)
        {
            // Keep waking ~2 Hz until background probes/jobs land so
            // results are drained even while the app is idle.
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.periodic_wake_instant(500)));
        } else if self.host.editor_state().editor_ui.git_panel.open {
            // While the Git panel is open, wake every 2 s for the
            // periodic repository re-snapshot.
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.periodic_wake_instant(2000)));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    pub(super) fn timed_wake_needs_redraw(&self, cause: &StartCause) -> bool {
        match cause {
            StartCause::ResumeTimeReached { .. } => true,
            StartCause::WaitCancelled {
                requested_resume: Some(deadline),
                ..
            } => Instant::now() >= *deadline,
            _ => false,
        }
    }

    pub(crate) fn resume_time_needs_redraw(&self) -> bool {
        self.current_chat.is_some()
            || self.current_design.is_some()
            || self.current_codegen.is_some()
            || self.current_design_md.is_some()
            || !self.sub_agents.is_empty()
            || self
                .current_figma_import
                .as_ref()
                .is_some_and(figma_import_session::FigmaImportSession::is_worker_pending)
            || self.current_html_import.is_some()
            || self.pending_figma_paste.is_some()
            || self.pending_html_paste.is_some()
            || self.host.next_animation_deadline_ms().is_some()
            // The top-bar tooltip's dwell deadline retires the moment it
            // elapses, so by the time this wake runs the clock has
            // already moved past it and the deadline above reports
            // nothing. Ask about the tooltip itself or the wake we
            // scheduled for it would be thrown away unpainted.
            || self.host.top_bar_tooltip_showing()
            || self.update_probe.is_pending()
            || self.host.auth_flow_active()
            || self.model_probe.is_pending()
            || self.image_search.is_pending()
            || self.image_panel.is_pending()
            || self.collab_avatars.is_pending()
            || op_editor_ui::collab_avatar_runtime::has_pending_collab_avatar_requests()
            || self.image_decodes.is_pending()
            || op_editor_ui::widgets::canvas_viewport_image::has_pending_decodes()
            || self
                .iconify_job
                .as_ref()
                .is_some_and(crate::iconify_host::IconifyJob::is_pending)
            || self.provider_connect_pending()
            || self.acp_agent_connect_pending()
            || self.builtin_model_refresh_pending()
            || self
                .git_pull_job
                .as_ref()
                .is_some_and(git_jobs::GitPullJob::is_pending)
            || self
                .git_push_job
                .as_ref()
                .is_some_and(git_jobs::GitPushJob::is_pending)
            || self
                .git_status_job
                .as_ref()
                .is_some_and(git_jobs::GitStatusJob::is_pending)
            || self
                .git_diff_job
                .as_ref()
                .is_some_and(git_jobs::GitDiffJob::is_pending)
            || self.host.editor_state().editor_ui.git_panel.open
    }

    pub(super) fn try_init_render_context(&mut self, event_loop: &ActiveEventLoop) -> bool {
        if self.ctx.is_some() && self.backend.is_some() {
            return true;
        }
        let Some(window) = self.window.as_ref() else {
            return false;
        };
        let size = window.inner_size();
        if size.width <= 1 || size.height <= 1 {
            self.defer_render_context_init(event_loop);
            return false;
        }

        let dpi = window.scale_factor() as f32;
        match SharedSkiaContext::new_desktop(window) {
            Ok(ctx) => {
                self.dpi = dpi;
                self.ctx = Some(ctx);
                self.backend = Some(NativeBackend::with_dpi(dpi));
                true
            }
            Err(err) if render_surface_not_ready(&err) => {
                self.defer_render_context_init(event_loop);
                false
            }
            Err(err) => {
                eprintln!("openpencil-desktop: SharedSkiaContext::new_desktop failed: {err}");
                // The GUI subsystem has no console (`windows_subsystem =
                // "windows"`), so the `eprintln!` above goes nowhere on a
                // double-click launch — the app would just flash and
                // vanish. Surface the failure in a native dialog so the
                // user sees a real reason (GPU driver / OpenGL) instead of
                // a mystery crash.
                show_gpu_init_error_dialog(&err);
                self.error = Some(err);
                event_loop.exit();
                false
            }
        }
    }

    pub(super) fn defer_render_context_init(&self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(
                INITIAL_VIEWPORT_W as f64,
                INITIAL_VIEWPORT_H as f64,
            ));
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(50),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_init_retries_only_surface_not_ready() {
        let not_ready = op_host_native::SharedSkiaError::Provider(
            op_host_native::ProviderError::SurfaceNotReady {
                width: 1,
                height: 1,
            },
        );
        assert!(render_surface_not_ready(&not_ready));

        assert!(!render_surface_not_ready(
            &op_host_native::SharedSkiaError::GlInterface
        ));
    }
}

//! The Studio generation workspace on the web host: its phase pump, chrome
//! tiers and paint.

use super::studio_workspace_paint::thumb_fit;
use super::studio_workspace_run::WEB_SETTLE_GRACE_MS;
use super::WidgetHost;
use op_editor_core::scene_template_append::template_boards;
use op_editor_core::scene_template_catalog::scene_template_document;
use op_editor_core::{ChatMessage, HomeFamily, WorkspacePhase, WorkspaceView};
use op_editor_ui::widgets::WorkspaceSurface;
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

const W: f32 = 1440.0;
const H: f32 = 900.0;

/// A host with a workspace run queued from Home, exactly as a Home send
/// leaves it (the brief waiting in `pending_send`).
fn running_host() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    host.editor_state.editor_ui.agent_settings.web_served_models = true;
    host.editor_state.editor_ui.home.visible = true;
    host.editor_state
        .editor_ui
        .home
        .set_task(HomeFamily::Presentation, 0);
    for c in "quarterly review".chars() {
        host.apply_text(c);
    }
    assert!(host.apply_send());
    host
}

/// What the web chat drain does: take the send and stamp the turn.
fn launch(host: &mut WidgetHost, generation: u64) {
    host.editor_state.chat.pending_send.take().expect("queued");
    assert!(host.stamp_workspace_run(generation));
}

/// What the live-sync pull does once the daemon drew the design.
fn land_boards(host: &mut WidgetHost) {
    let source = scene_template_document("slide-deck").expect("embedded on native");
    let boards = template_boards(source, "slide-deck").expect("boards");
    assert!(host.editor_state.adopt_template_boards(boards));
    host.mark_dirty();
}

fn finish_stream(host: &mut WidgetHost) {
    for message in host.editor_state.chat.messages.iter_mut() {
        message.streaming = false;
    }
}

#[test]
fn a_queued_brief_keeps_the_pump_alive_until_it_launches() {
    let mut host = running_host();
    let tick = host.drive_workspace_run(W, H, false, 1_000);
    assert!(tick.keep_pumping, "awaiting launch is not idle");
    assert_eq!(
        host.editor_state.editor_ui.workspace.phase,
        WorkspacePhase::Generating
    );
}

#[test]
fn a_finished_run_settles_done_only_after_the_sync_grace() {
    let mut host = running_host();
    launch(&mut host, 7);
    assert!(host.drive_workspace_run(W, H, true, 1_000).keep_pumping);
    finish_stream(&mut host);
    // The stream ended but the daemon's boards have not synced yet.
    let idle = host.drive_workspace_run(W, H, false, 2_000);
    assert!(idle.keep_pumping, "inside the grace window");
    land_boards(&mut host);
    assert!(host.drive_workspace_run(W, H, false, 2_500).keep_pumping);
    let settled = host.drive_workspace_run(W, H, false, 2_000 + WEB_SETTLE_GRACE_MS);
    assert!(!settled.keep_pumping);
    assert!(settled.changed);
    assert_eq!(
        host.editor_state.editor_ui.workspace.phase,
        WorkspacePhase::Done
    );
}

#[test]
fn a_run_that_drew_nothing_settles_failed() {
    let mut host = running_host();
    launch(&mut host, 3);
    finish_stream(&mut host);
    host.drive_workspace_run(W, H, false, 10_000);
    host.drive_workspace_run(W, H, false, 10_000 + WEB_SETTLE_GRACE_MS);
    assert_eq!(
        host.editor_state.editor_ui.workspace.phase,
        WorkspacePhase::Failed
    );
}

#[test]
fn an_errored_stream_fails_without_waiting() {
    let mut host = running_host();
    launch(&mut host, 4);
    finish_stream(&mut host);
    host.editor_state.chat.messages.push(ChatMessage::assistant(
        "error: AI stream request failed to start",
    ));
    let tick = host.drive_workspace_run(W, H, false, 5_000);
    assert!(!tick.keep_pumping);
    assert_eq!(
        host.editor_state.editor_ui.workspace.phase,
        WorkspacePhase::Failed
    );
}

#[test]
fn stop_wins_over_the_idle_edge_and_a_new_turn_resumes() {
    let mut host = running_host();
    launch(&mut host, 5);
    assert!(host.stop_workspace_run());
    assert_eq!(
        host.editor_state.editor_ui.workspace.phase,
        WorkspacePhase::Stopped
    );
    assert!(!host.drive_workspace_run(W, H, false, 1_000).keep_pumping);
    assert_eq!(
        host.editor_state.editor_ui.workspace.phase,
        WorkspacePhase::Stopped
    );
    assert!(host.stamp_workspace_run(6));
    assert_eq!(
        host.editor_state.editor_ui.workspace.phase,
        WorkspacePhase::Generating
    );
}

#[test]
fn the_docked_canvas_starts_under_the_workspace_toolbar() {
    let host = running_host();
    let (_, top, _, _) = host.canvas_region(W, H);
    assert_eq!(
        top,
        op_editor_core::WORKSPACE_HEADER_H + op_editor_core::WORKSPACE_TOOLBAR_H
    );
}

#[test]
fn back_raises_home_over_a_live_workspace_and_home_returns_to_it() {
    let mut host = running_host();
    let back = {
        let surface = WorkspaceSurface::for_editor(&host.editor_state).expect("visible");
        surface.layout(W, H).back
    };
    let at = Point2D::new(
        back.origin.x + back.size.x / 2.0,
        back.origin.y + back.size.y / 2.0,
    );
    assert!(host.apply_press(at.x, at.y, W, H));
    assert!(host.home_visible());
    assert!(
        host.editor_state.editor_ui.workspace.active,
        "the run keeps going"
    );
    // Home's footer link reads 回到工作区 while a workspace is active.
    let footer = op_editor_ui::widgets::HomeSurface::for_editor(&host.editor_state)
        .expect("home")
        .layout(W, H)
        .use_example;
    let at = Point2D::new(
        footer.origin.x + footer.size.x / 2.0,
        footer.origin.y + footer.size.y / 2.0,
    );
    assert!(host.apply_press(at.x, at.y, W, H));
    assert!(!host.home_visible());
    assert!(host.workspace_visible());
}

#[test]
fn the_hidden_top_bar_cannot_take_presses_while_docked() {
    let mut host = running_host();
    // Where the professional TopBar's file button sits.
    assert!(host.apply_press(20.0, 20.0, W, H) || !host.editor_state.editor_ui.file_menu_open);
    assert!(!host.editor_state.editor_ui.file_menu_open);
}

#[test]
fn long_page_wheel_pans_instead_of_zooming() {
    let mut host = running_host();
    host.editor_state.editor_ui.workspace.view = WorkspaceView::LongPage;
    let zoom = host.editor_state.viewport.zoom;
    let pan_y = host.editor_state.viewport.pan_y;
    let (x, y, w, h) = host.canvas_region(W, H);
    assert!(host.apply_workspace_long_page_wheel(x + w / 2.0, y + h / 2.0, -120.0, false, W, H));
    assert_eq!(host.editor_state.viewport.zoom, zoom);
    assert_eq!(host.editor_state.viewport.pan_y, pan_y - 120.0);
    assert!(!host.apply_workspace_long_page_wheel(x + 5.0, y + 5.0, -120.0, true, W, H));
}

#[test]
fn thumbs_letterbox_the_board_into_the_plate() {
    let plate = Rect::xywh(10.0, 20.0, 160.0, 90.0);
    let (origin, zoom) = thumb_fit(plate, Rect::xywh(0.0, 0.0, 1920.0, 1080.0)).expect("fit");
    assert!((zoom - 160.0 / 1920.0).abs() < 1e-6);
    assert_eq!(origin, Point2D::new(10.0, 20.0));
    let (origin, _) = thumb_fit(plate, Rect::xywh(0.0, 0.0, 1080.0, 1080.0)).expect("fit");
    assert!(origin.x > 10.0, "a square board is centred horizontally");
    assert!(thumb_fit(plate, Rect::xywh(0.0, 0.0, 0.0, 10.0)).is_none());
}

#[derive(Default)]
struct CountingBackend {
    draws: usize,
    clips: Vec<Rect>,
}

impl RenderBackend for CountingBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {
        self.draws += 1;
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {
        self.draws += 1;
    }
    fn clip_rect(&mut self, rect: Rect) {
        self.clips.push(rect);
    }
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
        self.draws += 1;
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

#[test]
fn home_and_the_workspace_paint_through_the_web_pass() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.home.visible = true;
    host.set_now_ms(1_000);
    let mut backend = CountingBackend::default();
    host.paint_editor(&mut backend, W, H);
    assert!(backend.draws > 10, "Home painted");
    assert_eq!(host.editor_state.editor_ui.home.shown_at_ms, 1_000);

    let mut host = running_host();
    launch(&mut host, 1);
    land_boards(&mut host);
    host.set_now_ms(2_000);
    let mut backend = CountingBackend::default();
    host.paint_editor(&mut backend, W, H);
    assert!(backend.draws > 10, "workspace painted");
    assert!(host.editor_state.editor_ui.workspace.shown_at_ms > 0);
    // Every visible strip slot got its board preview (clipped to the plate).
    let layout = WorkspaceSurface::for_editor(&host.editor_state)
        .expect("visible")
        .layout(W, H);
    assert!(layout.strip.is_some(), "a deck has a strip");
    let boards = op_editor_core::preview_slideshow::active_page_boards(&host.editor_state).len();
    let previews = layout
        .thumbs
        .iter()
        .take(boards)
        .filter(|thumb| {
            let plate = Rect::xywh(
                thumb.origin.x,
                thumb.origin.y,
                thumb.size.x,
                thumb.size.y - 16.0,
            );
            backend.clips.contains(&plate)
        })
        .count();
    assert_eq!(previews, layout.thumbs.len().min(boards));
}

#[test]
fn retrying_a_stopped_run_redraws_on_a_fresh_page_inside_the_same_workspace() {
    let mut host = running_host();
    launch(&mut host, 2);
    land_boards(&mut host);
    assert!(host.stop_workspace_run());
    let retry = {
        let surface = WorkspaceSurface::for_editor(&host.editor_state).expect("visible");
        let layout = surface.layout(W, H);
        surface
            .banner_buttons(&layout)
            .expect("a stopped run offers Retry")
            .0
    };
    assert!(host.apply_press(retry.origin.x + 4.0, retry.origin.y + 4.0, W, H));
    assert!(
        host.take_home_replace_confirm().is_none(),
        "retry never asks"
    );
    assert!(op_editor_core::blank_starter::active_page_is_blank_starter(
        &host.editor_state
    ));
    let workspace = &host.editor_state.editor_ui.workspace;
    assert!(
        workspace.active && workspace.visible,
        "still the same workspace"
    );
    assert_eq!(workspace.phase, WorkspacePhase::Generating);
    assert!(host.editor_state.chat.pending_send.is_some());
    assert_eq!(
        host.editor_state.chat.launch_route,
        op_editor_core::LaunchRoute::Orchestrator
    );
}

fn audited_report(note: &str) -> op_editor_core::QualityReport {
    let mut report = op_editor_core::QualityReport::default();
    report.ingest_repairs(
        &["overflow".to_string()],
        &[op_editor_core::QualityRepairRecord {
            pass: "geometry-validation".into(),
            family: "overflow".into(),
            node_id: "title".into(),
            node_name: Some("Title".into()),
            detail: "width 420 → 327".into(),
        }],
        &[note.to_string()],
    );
    report.ingest_audit(&[], Vec::new());
    report
}

#[test]
fn the_daemons_quality_report_shows_on_its_own_run_once_done() {
    let mut host = running_host();
    launch(&mut host, 7);
    let report = audited_report("first");
    let locale = host.editor_state.editor_ui.effective_locale();
    let line = report.transcript_line(locale);
    assert!(host.apply_run_quality(7, None, report.clone()));
    let workspace = &host.editor_state.editor_ui.workspace;
    assert_eq!(workspace.quality.as_ref(), Some(&report));
    assert!(
        workspace.finished_quality().is_none(),
        "no chip while the run is still generating"
    );
    let reply = host
        .editor_state
        .chat
        .messages
        .iter()
        .rev()
        .find(|m| m.streaming)
        .expect("streaming reply");
    assert!(reply.content.contains(&line), "summary line in the reply");

    finish_stream(&mut host);
    land_boards(&mut host);
    host.drive_workspace_run(W, H, false, 5_000);
    host.drive_workspace_run(W, H, false, 5_000 + WEB_SETTLE_GRACE_MS);
    let workspace = &host.editor_state.editor_ui.workspace;
    assert_eq!(workspace.phase, WorkspacePhase::Done);
    assert_eq!(workspace.finished_quality(), Some(&report));

    // A late report from an older turn never replaces this run's report.
    host.apply_run_quality(6, None, audited_report("stale"));
    assert_eq!(
        host.editor_state.editor_ui.workspace.quality.as_ref(),
        Some(&report)
    );
}

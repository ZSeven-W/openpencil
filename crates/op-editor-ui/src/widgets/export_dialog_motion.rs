//! Motion controls for the shared export dialog.

use op_editor_core::motion_export::{
    motion_frame_count, MotionExportTrigger, MAX_MOTION_EXPORT_FRAMES, MAX_MP4_DURATION_MS,
};

use super::*;

pub(super) const SECTION_HEIGHT: f32 = 68.0;
pub(super) const SETTINGS_HEIGHT: f32 = 204.0;
const FIRST_ROW_Y: f32 = HEADER_HEIGHT + 2.0 * SECTION_HEIGHT;
const ROW_WIDTH: f32 = DIALOG_WIDTH - 2.0 * PAD;
const FRAME_RATES: [u32; 4] = [12, 24, 30, 60];
const DURATIONS: [Option<u64>; 4] = [None, Some(3_000), Some(5_000), Some(10_000)];

impl ExportDialog {
    pub(super) fn motion_hit_enabled(&self, hit: ExportDialogHit, ui: &EditorUiState) -> bool {
        match hit {
            ExportDialogHit::Export => !self.motion_frame_limit_exceeded(ui),
            ExportDialogHit::MotionFormat(MotionExportFormat::Mp4) => {
                ui.motion_export.mp4_available
            }
            ExportDialogHit::RepeatMotion => ui
                .motion_export
                .format
                .is_some_and(MotionExportFormat::supports_repeat),
            ExportDialogHit::MotionBackground(_) => {
                ui.motion_export.format != Some(MotionExportFormat::Mp4)
            }
            _ => true,
        }
    }

    pub(super) fn motion_frame_limit_exceeded(&self, ui: &EditorUiState) -> bool {
        if !self.show_motion_settings {
            return false;
        }
        self.motion_choices
            .get(ui.motion_export.choice)
            .or_else(|| self.motion_choices.first())
            .is_some_and(|choice| {
                let duration = ui.motion_export.effective_duration(choice);
                if ui.motion_export.format == Some(MotionExportFormat::Mp4) {
                    duration > MAX_MP4_DURATION_MS
                } else {
                    motion_frame_count(duration, ui.motion_export.fps)
                        > u64::from(MAX_MOTION_EXPORT_FRAMES)
                }
            })
    }

    fn motion_row(&self, row: usize, x: f32, width: f32) -> Rect {
        Rect::xywh(
            self.rect.origin.x + PAD + x,
            self.rect.origin.y + FIRST_ROW_Y + SECTION_HEIGHT * row as f32 + 16.0,
            width,
            PILL_HEIGHT,
        )
    }

    fn motion_formats(&self) -> impl Iterator<Item = (Rect, ExportDialogHit)> + '_ {
        MotionExportFormat::ALL
            .into_iter()
            .enumerate()
            .map(|(index, format)| {
                (
                    self.motion_row(
                        0,
                        (SCALE_PILL_WIDTH + PILL_GAP) * index as f32,
                        SCALE_PILL_WIDTH,
                    ),
                    ExportDialogHit::MotionFormat(format),
                )
            })
    }

    fn motion_settings(&self) -> impl Iterator<Item = (Rect, ExportDialogHit)> + '_ {
        let clip_w = PILL_HEIGHT;
        let durations_x = 168.0;
        std::iter::once((
            self.motion_row(1, 0.0, clip_w),
            ExportDialogHit::PreviousClip,
        ))
        .chain(std::iter::once((
            self.motion_row(1, ROW_WIDTH - clip_w, clip_w),
            ExportDialogHit::NextClip,
        )))
        .chain(FRAME_RATES.into_iter().enumerate().map(|(index, rate)| {
            (
                self.motion_row(2, index as f32 * 40.0, 34.0),
                ExportDialogHit::FrameRate(rate),
            )
        }))
        .chain([
            (
                self.motion_row(2, durations_x, 106.0),
                ExportDialogHit::MotionDuration,
            ),
            (
                self.motion_row(2, 282.0, ROW_WIDTH - 282.0),
                ExportDialogHit::RepeatMotion,
            ),
            (
                self.motion_row(3, 0.0, 90.0),
                ExportDialogHit::MotionBackground(MotionExportBackground::Transparent),
            ),
            (
                self.motion_row(3, 98.0, 62.0),
                ExportDialogHit::MotionBackground(MotionExportBackground::White),
            ),
            (
                self.motion_row(3, 172.0, 180.0),
                ExportDialogHit::ReducedMotion,
            ),
        ])
    }

    pub(super) fn hit_test_motion(&self, point: Point2D) -> Option<ExportDialogHit> {
        if self.motion_choices.is_empty() {
            return None;
        }
        self.motion_formats()
            .chain(self.motion_settings().filter(|(_, hit)| {
                self.show_motion_settings
                    && (self.motion_choices.len() > 1
                        || !matches!(
                            hit,
                            ExportDialogHit::PreviousClip | ExportDialogHit::NextClip
                        ))
            }))
            .find_map(|(rect, hit)| rect.contains(point).then_some(hit))
    }

    pub(super) fn press_motion(&self, hit: ExportDialogHit, ui: &mut EditorUiState) {
        let motion = &mut ui.motion_export;
        match hit {
            ExportDialogHit::MotionFormat(format) => {
                motion.format = Some(format);
                ui.export_dialog_hover = None;
            }
            ExportDialogHit::PreviousClip => {
                motion.choice = motion
                    .choice
                    .checked_sub(1)
                    .filter(|&index| index < self.motion_choices.len())
                    .unwrap_or(self.motion_choices.len() - 1);
            }
            ExportDialogHit::NextClip => {
                motion.choice = (motion.choice + 1) % self.motion_choices.len()
            }
            ExportDialogHit::FrameRate(rate) => motion.fps = rate,
            ExportDialogHit::MotionDuration => {
                let current = DURATIONS
                    .iter()
                    .position(|duration| *duration == motion.duration_ms)
                    .unwrap_or(0);
                motion.duration_ms = DURATIONS[(current + 1) % DURATIONS.len()];
            }
            ExportDialogHit::RepeatMotion => motion.repeat = !motion.repeat,
            ExportDialogHit::MotionBackground(background) => motion.background = background,
            ExportDialogHit::ReducedMotion => motion.reduced_motion = !motion.reduced_motion,
            _ => {}
        }
    }

    pub(super) fn paint_motion(
        &self,
        backend: &mut dyn RenderBackend,
        theme: &Theme,
        ui: &EditorUiState,
    ) {
        let Some(choice) = self
            .motion_choices
            .get(ui.motion_export.choice)
            .or_else(|| self.motion_choices.first())
        else {
            return;
        };
        let tr = |key: &'static str| op_i18n::translate(ui.effective_locale(), key);
        let row_label = |backend: &mut dyn RenderBackend, row, x, label| {
            let rect = self.motion_row(row, x, 0.0);
            let layout = TextLayout::single_run(
                label,
                FONT_FAMILY,
                12.0,
                theme.muted_foreground.to_jian(),
                Point2D::new(0.0, 0.0),
            );
            backend.draw_text(&layout, Point2D::new(rect.origin.x, rect.origin.y - 16.0));
        };
        let paint = |backend: &mut dyn RenderBackend, rect, hit, label: &str, active| {
            let button = crate::widgets::editor_state_ext::export_dialog_button(hit);
            self.paint_pill(
                backend,
                theme,
                rect,
                label,
                active,
                ui.export_dialog_hover == Some(button),
                export_dialog_pressed(ui, button),
            );
        };
        row_label(backend, 0, 0.0, tr("motion.section.title"));
        for (rect, hit) in self.motion_formats() {
            if let ExportDialogHit::MotionFormat(format) = hit {
                if format == MotionExportFormat::Mp4 && !ui.motion_export.mp4_available {
                    self.paint_disabled_pill(backend, theme, rect, format.label());
                    continue;
                }
                paint(
                    backend,
                    rect,
                    hit,
                    format.label(),
                    ui.motion_export.format == Some(format),
                );
            }
        }
        if !ui.motion_export.mp4_available {
            let hint = self.motion_row(0, 288.0, ROW_WIDTH - 288.0);
            backend.save();
            backend.clip_rect(hint);
            paint_centered_label(
                backend,
                tr("export.motionNoEncoder"),
                10.0,
                theme.muted_foreground,
                hint,
            );
            backend.restore();
        }
        if !self.show_motion_settings {
            return;
        }

        row_label(backend, 1, 0.0, tr("motion.field.trigger"));
        row_label(backend, 2, 0.0, "FPS");
        row_label(backend, 2, 168.0, tr("motion.field.duration"));
        let show_repeat = ui
            .motion_export
            .format
            .is_some_and(MotionExportFormat::supports_repeat);
        if show_repeat {
            row_label(backend, 2, 282.0, tr("motion.timeline.loop"));
        }
        row_label(backend, 3, 0.0, tr("export.motionBackground"));
        row_label(backend, 3, 172.0, tr("motion.section.title"));
        let trigger = match &choice.trigger {
            MotionExportTrigger::Mount => tr("motion.trigger.mount"),
            MotionExportTrigger::InView => tr("motion.trigger.inView"),
            MotionExportTrigger::Manual(id) => id,
        };
        let label = format!("{} · {trigger}", choice.root_name);
        let rect = self.motion_row(
            1,
            PILL_HEIGHT + PILL_GAP,
            ROW_WIDTH - 2.0 * (PILL_HEIGHT + PILL_GAP),
        );
        backend.save();
        backend.clip_rect(rect);
        paint_centered_label(backend, &label, 12.0, theme.foreground, rect);
        backend.restore();

        let duration = ui.motion_export.effective_duration(choice);
        for (rect, hit) in self.motion_settings() {
            match hit {
                ExportDialogHit::PreviousClip | ExportDialogHit::NextClip => {
                    let label = if hit == ExportDialogHit::PreviousClip {
                        "‹"
                    } else {
                        "›"
                    };
                    if self.motion_choices.len() == 1 {
                        self.paint_disabled_pill(backend, theme, rect, label);
                    } else {
                        paint(backend, rect, hit, label, false);
                    }
                }
                ExportDialogHit::FrameRate(rate) => paint(
                    backend,
                    rect,
                    hit,
                    &rate.to_string(),
                    rate == ui.motion_export.fps,
                ),
                ExportDialogHit::MotionDuration => paint(
                    backend,
                    rect,
                    hit,
                    &format!("{:.2} s", duration as f64 / 1000.0),
                    ui.motion_export.duration_ms.is_some(),
                ),
                ExportDialogHit::RepeatMotion if show_repeat => paint(
                    backend,
                    rect,
                    hit,
                    if ui.motion_export.repeat {
                        "∞"
                    } else {
                        "1×"
                    },
                    ui.motion_export.repeat,
                ),
                ExportDialogHit::MotionBackground(background) => {
                    let is_mp4 = ui.motion_export.format == Some(MotionExportFormat::Mp4);
                    let label = tr(match background {
                        MotionExportBackground::Transparent => "export.motionTransparent",
                        MotionExportBackground::White => "export.motionWhite",
                    });
                    if is_mp4 && background == MotionExportBackground::Transparent {
                        self.paint_disabled_pill(backend, theme, rect, label);
                    } else {
                        let current = if is_mp4 {
                            MotionExportBackground::White
                        } else {
                            ui.motion_export.background
                        };
                        paint(backend, rect, hit, label, current == background);
                    }
                }
                ExportDialogHit::ReducedMotion => paint(
                    backend,
                    rect,
                    hit,
                    tr("export.motionReduced"),
                    ui.motion_export.reduced_motion,
                ),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::FileAction;

    fn state() -> EditorState {
        let doc = jian_ops_schema::load_str(r#"{"version":"1.1","children":[
        {"type":"frame","id":"board","name":"Board","width":200,"height":180,"children":[
          {"type":"rectangle","id":"layer","width":80,"height":60,"animations":[
            {"trigger":"mount","durationMs":400,"keyframes":[{"offset":0,"opacity":0},{"offset":1,"opacity":1}]},
            {"trigger":"manual","id":"orbit","durationMs":500,"iterations":"infinite",
             "keyframes":[{"offset":0,"rotation":0},{"offset":1,"rotation":360}]}
          ]}
        ]}]}"#).unwrap().value;
        let mut state = EditorState::from_document(doc);
        state.editor_ui.open_export_dialog();
        state
    }

    fn point(dialog: &ExportDialog, rect: Rect) -> Point2D {
        let offset = rect.origin + rect.size * 0.5 - dialog.rect.origin;
        dialog.rect.origin + offset * dialog.fit
    }

    fn select(state: &mut EditorState, hit: ExportDialogHit) {
        let dialog = ExportDialog::for_editor(state, 1200.0, 800.0);
        let rect = dialog
            .motion_formats()
            .chain(dialog.motion_settings())
            .find_map(|(rect, candidate)| (hit == candidate).then_some(rect))
            .unwrap();
        let point = point(&dialog, rect);
        assert!(dialog.press(point, &mut state.editor_ui));
    }

    #[test]
    fn export_dialog_captures_the_chosen_clip_and_clears_static_choice_only_on_request() {
        let mut state = state();
        let static_format = state.editor_ui.export_format;
        select(
            &mut state,
            ExportDialogHit::MotionFormat(MotionExportFormat::Apng),
        );
        select(&mut state, ExportDialogHit::NextClip);
        select(&mut state, ExportDialogHit::FrameRate(24));
        select(&mut state, ExportDialogHit::RepeatMotion);
        select(
            &mut state,
            ExportDialogHit::MotionBackground(MotionExportBackground::White),
        );
        select(&mut state, ExportDialogHit::ReducedMotion);
        let dialog = ExportDialog::for_editor(&state, 1200.0, 800.0);
        assert!(dialog.press(
            point(&dialog, dialog.button_rects().1),
            &mut state.editor_ui
        ));
        assert_eq!(
            state.editor_ui.pending_file_action,
            Some(FileAction::ExportMotionConfirm)
        );
        let request = state.editor_ui.motion_export.pending.unwrap();
        assert_eq!(request.root_id, "board");
        assert_eq!(request.trigger, MotionExportTrigger::Manual("orbit".into()));
        assert_eq!(request.format, MotionExportFormat::Apng);
        assert_eq!(request.fps, 24);
        assert_eq!(request.duration_ms, Some(3_000));
        assert!(!request.repeat);
        assert_eq!(request.background, MotionExportBackground::White);
        assert!(request.reduced_motion);
        assert_eq!(state.editor_ui.export_format, static_format);
    }

    #[test]
    fn export_dialog_fits_small_viewports_and_rejects_the_extra_endpoint_frame() {
        let mut state = state();
        state.editor_ui.motion_export.format = Some(MotionExportFormat::Gif);
        state.editor_ui.motion_export.fps = 60;
        state.editor_ui.motion_export.duration_ms = Some(10_000);
        let dialog = ExportDialog::for_editor(&state, 360.0, 325.0);
        assert!(dialog.rect().size.x <= 360.0 && dialog.rect().size.y <= 325.0);
        let export = point(&dialog, dialog.button_rects().1);
        assert_eq!(dialog.hit_test(export), Some(ExportDialogHit::Export));
        assert_eq!(dialog.hit_test_for_ui(export, &state.editor_ui), None);
        state.editor_ui.motion_export.duration_ms = Some(5_000);
        assert_eq!(
            dialog.hit_test_for_ui(export, &state.editor_ui),
            Some(ExportDialogHit::Export)
        );
    }
    #[test]
    fn mp4_requires_an_encoder_and_does_not_offer_transparency_or_repeat() {
        let mut state = state();
        let dialog = ExportDialog::for_editor(&state, 1200.0, 800.0);
        let mp4 = dialog
            .motion_formats()
            .find_map(|(rect, hit)| {
                (hit == ExportDialogHit::MotionFormat(MotionExportFormat::Mp4))
                    .then_some(point(&dialog, rect))
            })
            .unwrap();
        assert_eq!(dialog.hit_test_for_ui(mp4, &state.editor_ui), None);
        state.editor_ui.motion_export.mp4_available = true;
        assert!(dialog.press(mp4, &mut state.editor_ui));
        let dialog = ExportDialog::for_editor(&state, 1200.0, 800.0);
        for (rect, hit) in dialog.motion_settings() {
            if matches!(
                hit,
                ExportDialogHit::RepeatMotion | ExportDialogHit::MotionBackground(_)
            ) {
                assert_eq!(
                    dialog.hit_test_for_ui(point(&dialog, rect), &state.editor_ui),
                    None
                );
            }
        }
    }
}

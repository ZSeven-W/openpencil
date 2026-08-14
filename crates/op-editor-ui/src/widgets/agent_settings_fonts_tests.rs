use super::*;
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::missing_fonts::{MissingFontEntry, MissingFontsPrompt};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::EditorState;
use std::sync::Arc;

#[derive(Default)]
struct CaptureBackend {
    text: Vec<(String, f32)>,
    clips: Vec<Rect>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
        self.text.extend(
            layout
                .runs()
                .iter()
                .map(|run| (run.content.clone(), run.font_size)),
        );
    }
    fn clip_rect(&mut self, rect: Rect) {
        self.clips.push(rect);
    }
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
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

fn content_rect(width: f32) -> Rect {
    Rect::xywh(16.0, 20.0, width - 32.0, 900.0)
}

fn state_with_missing(families: &[&str]) -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.missing_fonts_prompt = Some(MissingFontsPrompt {
        entries: families
            .iter()
            .map(|family| MissingFontEntry {
                family: (*family).to_owned(),
                run_count: 2,
                mismatch_note: None,
                resolved: false,
            })
            .collect(),
    });
    state
}

fn touch_state(class: EditorSizeClass) -> EditorState {
    let mut state = state_with_missing(&["A Very Long Missing Font Family For A Narrow Phone"]);
    state.editor_ui.touch = true;
    state.editor_ui.size_class = class;
    state.editor_ui.imported_font_families = Arc::new(vec!["Katibeh".into()]);
    state.editor_ui.system_font_families = Arc::new(vec!["Arial".into()]);
    state.editor_ui.font_import_supported = true;
    state
}

fn painted_text(state: &EditorState, width: f32) -> Vec<(String, f32)> {
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    paint_fonts_tab(
        &mut cx,
        &Theme::dark(),
        &state.editor_ui,
        content_rect(width),
    );
    backend.text
}

fn painted_capture(state: &EditorState, width: f32) -> CaptureBackend {
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    paint_fonts_tab(
        &mut cx,
        &Theme::dark(),
        &state.editor_ui,
        content_rect(width),
    );
    backend
}

#[test]
fn tab_renders_missing_rows_and_empty_copy() {
    let state = state_with_missing(&["Katibeh", "Inter Tight"]);
    let text = painted_text(&state, 492.0);
    assert!(text.iter().any(|(value, _)| value == "Katibeh"));
    assert!(text.iter().any(|(value, _)| value == "Inter Tight"));

    let empty = EditorState::new();
    let text = painted_text(&empty, 492.0);
    assert!(text
        .iter()
        .any(|(value, _)| { value == translate(&empty.editor_ui, "missingFonts.noneMissing") }));
}

#[test]
fn desktop_font_geometry_keeps_legacy_density() {
    let mut state = state_with_missing(&["Katibeh"]);
    state.editor_ui.imported_font_families = Arc::new(vec!["Imported".into()]);
    let content = content_rect(492.0);
    let missing = missing_row_rect(content, &state.editor_ui, 0);
    assert_eq!(missing.size.y, 72.0);
    assert_eq!(
        settings_row_button_rect(missing, &state.editor_ui).size.y,
        28.0
    );
    assert_eq!(imported_row_rect(content, &state.editor_ui, 0).size.y, 44.0);
    assert_eq!(
        imported_remove_rect(content, &state.editor_ui, 0).size.y,
        28.0
    );
    assert_eq!(content_height(&state.editor_ui), 252.0);
}

#[test]
fn phone_and_pad_font_controls_are_real_44pt_targets() {
    for (class, width) in [
        (EditorSizeClass::Compact, 390.0_f32),
        (EditorSizeClass::Medium, 834.0_f32),
    ] {
        let state = touch_state(class);
        let content = content_rect(width);
        let missing = missing_row_rect(content, &state.editor_ui, 0);
        let choose = settings_row_button_rect(missing, &state.editor_ui);
        let imported = imported_row_rect(content, &state.editor_ui, 0);
        let remove = imported_remove_rect(content, &state.editor_ui, 0);

        assert!(missing.size.y >= 44.0);
        assert!(choose.size.x >= 44.0 && choose.size.y >= 44.0);
        assert!(imported.size.y >= 44.0);
        assert!(remove.size.x >= 44.0 && remove.size.y >= 44.0);
        assert!(missing.origin.y + missing.size.y <= imported.origin.y);
        assert!(
            content_height(&state.editor_ui)
                >= imported.origin.y + imported.size.y - content.origin.y
        );

        for (target, expected) in [
            (choose, FontsHit::ChooseFont(0)),
            (remove, FontsHit::RemoveImportedFont(0)),
        ] {
            let point = Point2D::new(
                target.origin.x + target.size.x / 2.0,
                target.origin.y + target.size.y / 2.0,
            );
            assert_eq!(
                hit_test(
                    Rect::xywh(0.0, 0.0, width, 1_112.0),
                    content,
                    &state.editor_ui,
                    point,
                    0.0,
                ),
                expected,
            );
        }

        let text = painted_text(&state, width);
        assert!(
            text.iter().all(|(_, size)| *size >= 14.0),
            "touch settings text fell below 14pt: {text:?}"
        );
    }
}

#[test]
fn settings_touch_picker_shares_44pt_paint_and_hit_geometry() {
    for (class, width, height) in [
        (EditorSizeClass::Compact, 390.0_f32, 844.0_f32),
        (EditorSizeClass::Medium, 834.0_f32, 1_112.0_f32),
    ] {
        let mut state = touch_state(class);
        state
            .editor_ui
            .open_missing_font_picker(0, op_editor_core::MissingFontSurface::Settings);
        let panel = Rect::xywh(0.0, 0.0, width, height);
        let content = content_rect(width);
        let layout = picker_layout(panel, content, &state.editor_ui, 0.0).expect("picker");

        assert!(layout.touch_controls);
        assert_eq!(layout.search.size.y, 44.0);
        let bounds = picker_bounds(panel, content);
        assert!(layout.popup.origin.y >= bounds.origin.y);
        assert!(
            layout.popup.origin.y + layout.popup.size.y <= bounds.origin.y + bounds.size.y + 0.01
        );
        for (row, rect) in &layout.rows {
            if matches!(
                row,
                crate::widgets::property_panel_typography::FontPickerRow::Entry(_)
                    | crate::widgets::property_panel_typography::FontPickerRow::ImportAction
                    | crate::widgets::property_panel_typography::FontPickerRow::RemoveEntry(_)
            ) {
                assert!(
                    rect.size.x >= 44.0
                        || !matches!(
                            row,
                            crate::widgets::property_panel_typography::FontPickerRow::RemoveEntry(
                                _
                            )
                        )
                );
                assert_eq!(rect.size.y, 44.0, "{row:?}");
            }
        }

        let (entry, entry_rect) = layout
            .rows
            .iter()
            .find(|(row, _)| {
                matches!(
                    row,
                    crate::widgets::property_panel_typography::FontPickerRow::Entry(_)
                )
            })
            .expect("entry");
        let crate::widgets::property_panel_typography::FontPickerRow::Entry(index) = entry else {
            unreachable!()
        };
        let point = Point2D::new(entry_rect.origin.x + 12.0, entry_rect.origin.y + 22.0);
        assert_eq!(
            hit_test(panel, content, &state.editor_ui, point, 0.0),
            FontsHit::SelectFont(*index),
        );

        let (_, remove) = layout
            .rows
            .iter()
            .find(|(row, _)| {
                matches!(
                    row,
                    crate::widgets::property_panel_typography::FontPickerRow::RemoveEntry(_)
                )
            })
            .expect("imported remove");
        let point = Point2D::new(remove.origin.x + 22.0, remove.origin.y + 22.0);
        assert_eq!(
            hit_test(panel, content, &state.editor_ui, point, 0.0),
            FontsHit::RemoveImportedFont(0),
        );
    }
}

#[test]
fn touch_picker_stays_inside_a_keyboard_shortened_body() {
    let mut state = touch_state(EditorSizeClass::Compact);
    state
        .editor_ui
        .open_missing_font_picker(0, op_editor_core::MissingFontSurface::Settings);
    let panel = Rect::xywh(0.0, 0.0, 390.0, 360.0);
    let content = Rect::xywh(16.0, 120.0, 358.0, 180.0);
    let bounds = picker_bounds(panel, content);
    let layout = picker_layout(panel, content, &state.editor_ui, 0.0).expect("short picker");

    assert_eq!(layout.search.size.y, 44.0);
    assert!(layout.popup.origin.y >= bounds.origin.y);
    assert!(layout.popup.origin.y + layout.popup.size.y <= bounds.origin.y + bounds.size.y + 0.01);
    assert!(layout.viewport.size.y <= bounds.size.y - layout.search.size.y + 0.01);
    assert!(layout.max_scroll > 0.0);
}

#[test]
fn narrow_touch_imported_family_is_clipped_before_the_delete_target() {
    const LONG_FAMILY: &str =
        "Extremely Long Imported Brand Typeface That Must Never Cover The Delete Button";
    for width in [320.0_f32, 390.0_f32] {
        let mut state = state_with_missing(&["Inter"]);
        state.editor_ui.touch = true;
        state.editor_ui.size_class = EditorSizeClass::Compact;
        state.editor_ui.imported_font_families = Arc::new(vec![LONG_FAMILY.into()]);
        let content = content_rect(width);
        let row = imported_row_rect(content, &state.editor_ui, 0);
        let remove = imported_remove_rect(content, &state.editor_ui, 0);
        let capture = painted_capture(&state, width);

        assert!(!capture.text.iter().any(|(text, _)| text == LONG_FAMILY));
        assert!(capture
            .text
            .iter()
            .any(|(text, size)| *size == 15.0 && text.ends_with('…')));
        assert!(capture.clips.iter().any(|clip| {
            (clip.origin.y - (row.origin.y + 1.0)).abs() < 0.01
                && clip.origin.x + clip.size.x <= remove.origin.x - 16.0 + 0.01
        }));
    }
}

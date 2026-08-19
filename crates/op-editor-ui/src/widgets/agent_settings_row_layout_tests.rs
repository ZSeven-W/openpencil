//! Traversal layout audit for the settings modal.
//!
//! Every earlier row-collision bug was found by eye and fixed with a
//! hand-picked coordinate, which is why the same defect kept reappearing
//! on a different row. These tests do not name coordinates: they paint a
//! tab, ask its module where its row boxes are, and assert the invariants
//! that make a row list readable —
//!
//! 1. every line of text a row paints sits inside that row's own box,
//!    ascender to descender;
//! 2. within a row, consecutive lines never overlap ink — this is the
//!    one the shipped squeeze actually violated: both lines were inside
//!    a 54 px box, but the label's descender and the status line's
//!    ascender crossed, so the two read as one blob;
//! 3. row boxes never overlap;
//! 4. a section header leaves its first row's ascender clear.
//!
//! A regression anywhere in the modal trips these, including on rows
//! nobody thought to write a test for.

use crate::widgets::agent_settings_panel::AgentSettingsPanel;
use crate::widgets::agent_settings_rows::{
    row_height, RowLines, ASCENT_RATIO, DESCENT_RATIO, SECTION_HEADER_H,
};
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_settings::AgentSettingsTab;
use op_editor_core::EditorState;

/// Records every painted run's baseline and size so the audit can place
/// each line against the row box it belongs to.
#[derive(Default)]
struct RunCapture {
    runs: Vec<(String, f32, Point2D)>,
}

impl RenderBackend for RunCapture {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, point: Point2D) {
        if let Some(run) = layout.runs().first() {
            if !run.content.trim().is_empty() {
                self.runs.push((run.content.clone(), run.font_size, point));
            }
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        text.chars().count() as f32 * font_size * 0.55
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn painted_runs(state: &EditorState) -> (Rect, Vec<(String, f32, Point2D)>) {
    let panel = AgentSettingsPanel::for_editor(state);
    let rect = panel.rect(1400.0, 1000.0);
    let mut backend = RunCapture::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint(&mut cx, rect);
    (rect, backend.runs)
}

/// A run's ink extent: baseline minus ascent, baseline plus descent.
fn ink_span(font_size: f32, baseline_y: f32) -> (f32, f32) {
    (
        baseline_y - font_size * ASCENT_RATIO,
        baseline_y + font_size * DESCENT_RATIO,
    )
}

/// Assert that every run whose baseline falls in a row box also fits
/// inside it vertically, and that the boxes tile without overlap.
fn audit(label: &str, boxes: &[(Rect, RowLines)], runs: &[(String, f32, Point2D)]) {
    assert!(!boxes.is_empty(), "{label}: no row boxes to audit");

    for pair in boxes.windows(2) {
        let (a, _) = pair[0];
        let (b, _) = pair[1];
        assert!(
            a.origin.y + a.size.y <= b.origin.y + 0.01,
            "{label}: row boxes overlap — {:?} runs into {:?}",
            a.origin.y..a.origin.y + a.size.y,
            b.origin.y
        );
    }

    let mut audited = 0usize;
    for (row, lines) in boxes.iter().copied() {
        // Every run whose baseline lands in this box, in paint order down
        // the row.
        let mut in_row: Vec<(&String, f32, f32, f32, f32)> = runs
            .iter()
            .filter(|(_, _, at)| {
                at.y > row.origin.y
                    && at.y <= row.origin.y + row.size.y
                    && at.x >= row.origin.x - 0.01
            })
            .map(|(text, font_size, at)| {
                // Same estimate the capture backend measures with, so the
                // horizontal spans are consistent with what it reported.
                let width = text.chars().count() as f32 * font_size * 0.55;
                (text, *font_size, at.y, at.x, at.x + width)
            })
            .collect();
        in_row.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

        for (text, font_size, baseline, _, _) in &in_row {
            let (top, bottom) = ink_span(*font_size, *baseline);
            assert!(
                top >= row.origin.y - 0.01,
                "{label}: {text:?} ({font_size}pt) rises {:.2}px above its {lines:?} row box",
                row.origin.y - top
            );
            assert!(
                bottom <= row.origin.y + row.size.y + 0.01,
                "{label}: {text:?} ({font_size}pt) drops {:.2}px below its {lines:?} row box",
                bottom - (row.origin.y + row.size.y)
            );
            audited += 1;
        }

        // Distinct lines within the row must not collide. Only compare
        // runs that actually share horizontal space — a row's right-hand
        // controls (port label, button caption) sit on their own
        // baselines beside the label, and stacking them against it would
        // report a collision that no reader can see.
        for (i, upper) in in_row.iter().enumerate() {
            for lower in in_row.iter().skip(i + 1) {
                let (upper_text, upper_size, upper_baseline, upper_x0, upper_x1) = upper;
                let (lower_text, lower_size, lower_baseline, lower_x0, lower_x1) = lower;
                if (lower_baseline - upper_baseline).abs() < 0.01 {
                    continue;
                }
                if lower_x0 >= upper_x1 || upper_x0 >= lower_x1 {
                    continue;
                }
                let (_, upper_bottom) = ink_span(*upper_size, *upper_baseline);
                let (lower_top, _) = ink_span(*lower_size, *lower_baseline);
                assert!(
                    lower_top >= upper_bottom - 0.01,
                    "{label}: {lower_text:?} overlaps {upper_text:?} by {:.2}px — \
                     the two lines of a {lines:?} row read as one blob",
                    upper_bottom - lower_top
                );
            }
        }
    }
    assert!(
        audited >= boxes.len(),
        "{label}: only {audited} runs landed in {} row boxes — the audit is not reaching the rows",
        boxes.len()
    );
}

fn state_for(tab: AgentSettingsTab) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.locale = op_i18n::Locale::EnUs;
    state.editor_ui.agent_settings.tab = tab;
    state
}

#[test]
fn mcp_rows_keep_their_text_inside_their_boxes() {
    let mut state = state_for(AgentSettingsTab::Mcp);
    state.editor_ui.agent_settings.mcp_server.running = true;
    let (rect, runs) = painted_runs(&state);
    let content = crate::widgets::agent_settings_panel::content_viewport(rect);
    audit(
        "MCP",
        &crate::widgets::agent_settings_mcp::row_boxes(
            content,
            state.editor_ui.external_cli_available,
        ),
        &runs,
    );
}

#[test]
fn system_rows_keep_their_text_inside_their_boxes() {
    let state = state_for(AgentSettingsTab::System);
    let (rect, runs) = painted_runs(&state);
    let content = crate::widgets::agent_settings_panel::content_viewport(rect);
    audit(
        "System",
        &crate::widgets::agent_settings_system::row_boxes(content),
        &runs,
    );
}

#[test]
fn agents_provider_rows_keep_their_text_inside_their_boxes() {
    let state = state_for(AgentSettingsTab::Agents);
    let (rect, runs) = painted_runs(&state);
    let boxes = crate::widgets::agent_settings_panel::provider_row_boxes(rect);
    audit("Agents providers", &boxes, &runs);
}

#[test]
fn a_section_header_leaves_its_first_row_clear() {
    // The MCP integrations header is the one that shipped clipping the
    // Claude Code row under it. Assert the general contract: no run in the
    // first row may rise into the header's own band.
    let mut state = state_for(AgentSettingsTab::Mcp);
    state.editor_ui.agent_settings.mcp_server.running = true;
    let (rect, runs) = painted_runs(&state);
    let content = crate::widgets::agent_settings_panel::content_viewport(rect);
    let header_top = crate::widgets::agent_settings_mcp::integrations_top(content);
    let first_row = crate::widgets::agent_settings_mcp::cli_row_rect(content, 0);

    assert!(
        first_row.origin.y >= header_top + SECTION_HEADER_H - 0.01,
        "the first row starts at {} but the header band ends at {}",
        first_row.origin.y,
        header_top + SECTION_HEADER_H
    );

    for (text, font_size, at) in &runs {
        if at.y <= first_row.origin.y || at.y > first_row.origin.y + first_row.size.y {
            continue;
        }
        let (top, _) = ink_span(*font_size, at.y);
        assert!(
            top >= header_top + SECTION_HEADER_H - 0.01,
            "{text:?} in the first row rises into the section header band"
        );
    }
}

#[test]
fn every_two_line_row_is_taller_than_every_one_line_row() {
    // The contract the three shipped collisions all violated: a row that
    // carries two lines must not be laid out in a one-line box.
    assert!(
        row_height(RowLines::Two) > row_height(RowLines::One),
        "a two-line row must reserve more height than a one-line row"
    );
}

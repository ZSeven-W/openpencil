//! Tests for the #18/#19 VariablesPanel management UX: row overflow
//! menu (Rename / Delete), search filter, scrolling, panel resize,
//! and variant-column-targeted color editing.

use super::WidgetHostNative;
use jian_ops_schema::variable::{VariableKind, VariableScalar, VariableValue};
use op_editor_core::editor_ui_state::VariableRowFocus;
use op_editor_ui::widgets::variables_panel::{
    VariablesPanel, VariablesPanelHit, VariablesResizeEdge,
};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1280.0;
const VIEWPORT_H: f32 = 900.0;

/// Host with the panel open, a 2-variant Theme-1 axis pinned to
/// Default, and one color variable.
fn two_variant_color_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into(), "Variant-1".into()]);
    state.editor_ui.variables_current_axis = Some("Theme-1".into());
    state
        .ui
        .variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    assert!(state.create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#112233".into()),
    ));
    host
}

/// Locate the panel point that hit-tests to `want` by scanning the
/// panel rect — keeps the tests robust against geometry tweaks.
fn point_for_hit(host: &WidgetHostNative, want: &VariablesPanelHit) -> (f32, f32) {
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    let panel = VariablesPanel::for_editor(host.editor_state());
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x < rect.origin.x + rect.size.x {
            if panel
                .hit_test(rect, Point2D::new(x, y))
                .is_some_and(|hit| &hit == want)
            {
                return (x, y);
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no panel point maps to {want:?}");
}

fn themed_value_for<'a>(
    state: &'a op_editor_core::EditorState,
    name: &str,
    variant: &str,
) -> Option<&'a VariableScalar> {
    let def = state.doc.variables.as_ref()?.get(name)?;
    let VariableValue::Themed(entries) = &def.value else {
        return None;
    };
    entries
        .iter()
        .find(|e| {
            e.theme
                .as_ref()
                .and_then(|t| t.get("Theme-1"))
                .is_some_and(|v| v == variant)
        })
        .map(|e| &e.value)
}

// --- #19: variant-column-targeted color editing ---------------------

#[test]
fn color_swatch_press_under_variant_column_targets_that_variant() {
    let mut host = two_variant_color_host();
    let (x, y) = point_for_hit(
        &host,
        &VariablesPanelHit::ColorSwatch { row: 0, variant: 1 },
    );
    assert!(host.apply_press(x, y, VIEWPORT_W, VIEWPORT_H));
    let picker = host
        .editor_state()
        .ui
        .color_picker
        .as_ref()
        .expect("variant swatch press opens the HSV picker");
    assert_eq!(picker.variable.as_deref(), Some("color-1"));
    assert_eq!(
        picker.variable_theme,
        Some(("Theme-1".to_string(), "Variant-1".to_string()))
    );

    // Drag to pure red — the write lands in Variant-1, NOT the
    // active (Default) column. This was the #19 wrong-cell write.
    assert!(host.editor_state_mut().color_picker_set_hsv(0.0, 1.0, 1.0));
    assert_eq!(
        themed_value_for(host.editor_state(), "color-1", "Variant-1"),
        Some(&VariableScalar::Str("#ff0000".into()))
    );
    assert_eq!(
        themed_value_for(host.editor_state(), "color-1", "Default"),
        Some(&VariableScalar::Str("#112233".into()))
    );
}

#[test]
fn color_hex_cell_press_starts_inline_edit_and_commits_to_variant() {
    let mut host = two_variant_color_host();
    let (x, y) = point_for_hit(&host, &VariablesPanelHit::ValueCell { row: 0, variant: 1 });
    assert!(host.apply_press(x, y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_focus,
        Some(VariableRowFocus::ColorCell { row: 0, variant: 1 })
    );
    // Draft seeds with the column's 6-char hex (TS toHex7).
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "#112233"
    );

    // Replace with a new full hex and commit on Enter.
    for _ in 0..7 {
        assert!(host.apply_backspace());
    }
    for c in "#abcdef".chars() {
        assert!(host.apply_text(c), "char {c} should be accepted");
    }
    // Non-hex chars are rejected by the gate.
    assert!(!host.apply_text('g'));
    assert!(host.apply_send());
    assert_eq!(
        themed_value_for(host.editor_state(), "color-1", "Variant-1"),
        Some(&VariableScalar::Str("#abcdef".into()))
    );
    assert_eq!(
        themed_value_for(host.editor_state(), "color-1", "Default"),
        Some(&VariableScalar::Str("#112233".into()))
    );
}

#[test]
fn color_hex_cell_partial_draft_reverts_without_writing() {
    let mut host = two_variant_color_host();
    let (x, y) = point_for_hit(&host, &VariablesPanelHit::ValueCell { row: 0, variant: 0 });
    assert!(host.apply_press(x, y, VIEWPORT_W, VIEWPORT_H));
    for _ in 0..7 {
        let _ = host.apply_backspace();
    }
    for c in "#ab".chars() {
        assert!(host.apply_text(c));
    }
    let depth = host.editor_state().history.past.len();
    assert!(host.apply_send());
    // Partial hex never commits (TS `/^#[0-9a-fA-F]{6}$/` gate) and
    // never pushes history.
    assert_eq!(host.editor_state().history.past.len(), depth);
    let def = host
        .editor_state()
        .doc
        .variables
        .as_ref()
        .unwrap()
        .get("color-1")
        .unwrap();
    assert_eq!(
        def.value,
        VariableValue::Scalar(VariableScalar::Str("#112233".into()))
    );
}

// --- #18: row overflow menu -----------------------------------------

#[test]
fn row_menu_toggle_then_delete_removes_variable_with_undo() {
    let mut host = two_variant_color_host();
    let (bx, by) = point_for_hit(&host, &VariablesPanelHit::RowMenuToggle(0));
    assert!(host.apply_press(bx, by, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.variables_row_menu, Some(0));

    let (dx, dy) = point_for_hit(&host, &VariablesPanelHit::RowMenuDelete(0));
    assert!(host.apply_press(dx, dy, VIEWPORT_W, VIEWPORT_H));
    assert!(!host
        .editor_state()
        .doc
        .variables
        .as_ref()
        .is_some_and(|vars| vars.contains_key("color-1")));
    assert_eq!(host.editor_state().editor_ui.variables_row_menu, None);

    // Single undo step restores the variable (snapshot-based).
    assert!(host.editor_state_mut().undo());
    assert!(host
        .editor_state()
        .doc
        .variables
        .as_ref()
        .is_some_and(|vars| vars.contains_key("color-1")));
}

#[test]
fn row_menu_rename_seeds_select_all_name_focus() {
    let mut host = two_variant_color_host();
    let (bx, by) = point_for_hit(&host, &VariablesPanelHit::RowMenuToggle(0));
    assert!(host.apply_press(bx, by, VIEWPORT_W, VIEWPORT_H));
    let (rx, ry) = point_for_hit(&host, &VariablesPanelHit::RowMenuRename(0));
    assert!(host.apply_press(rx, ry, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_focus,
        Some(VariableRowFocus::Name(0))
    );
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "color-1"
    );
    // TS focuses AND `.select()`s the rename input.
    assert!(host
        .editor_state()
        .editor_ui
        .variable_row_input
        .is_select_all());
    assert_eq!(host.editor_state().editor_ui.variables_row_menu, None);
}

#[test]
fn row_menu_button_press_again_closes_menu() {
    let mut host = two_variant_color_host();
    let (bx, by) = point_for_hit(&host, &VariablesPanelHit::RowMenuToggle(0));
    assert!(host.apply_press(bx, by, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.variables_row_menu, Some(0));
    assert!(host.apply_press(bx, by, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.variables_row_menu, None);
}

// --- #18: search filter ----------------------------------------------

fn many_vars_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    for i in 1..=7 {
        assert!(state.create_variable(
            &format!("color-{i}"),
            VariableKind::Color,
            VariableScalar::Str("#000000".into()),
        ));
    }
    assert!(state.create_variable("spacing", VariableKind::Number, VariableScalar::Num(8.0),));
    host
}

fn panel_first_row_center_y(panel: &VariablesPanel, rect: op_editor_ui::Rect) -> f32 {
    // rows viewport top + half a row.
    rect.origin.y + 44.0 + 36.0 + if panel.search_visible() { 44.0 } else { 0.0 } + 22.0
}

#[test]
fn search_box_appears_past_six_rows_and_filters_with_source_indices() {
    let mut host = many_vars_host();
    let panel = VariablesPanel::for_editor(host.editor_state());
    assert!(panel.search_visible());
    assert_eq!(panel.row_count(), 8);

    // Focus the box and type a filter.
    let (sx, sy) = point_for_hit(&host, &VariablesPanelHit::SearchBox);
    assert!(host.apply_press(sx, sy, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state().editor_ui.variables_search_focus);
    for c in "spac".chars() {
        assert!(host.apply_text(c));
    }
    assert_eq!(host.editor_state().editor_ui.variables_search, "spac");
    let panel = VariablesPanel::for_editor(host.editor_state());
    assert_eq!(panel.row_count(), 1);
    // The surviving row still reports its UNFILTERED index ("spacing"
    // sorts after the 7 color-* names → source 7), so host lookups
    // stay correct.
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    let hit = panel.hit_test(
        rect,
        Point2D::new(rect.origin.x + 60.0, panel_first_row_center_y(&panel, rect)),
    );
    assert_eq!(hit, Some(VariablesPanelHit::NameCell(7)));

    // Backspace edits the live filter; Escape blurs but keeps it.
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().editor_ui.variables_search, "spa");
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.variables_search_focus);
    assert_eq!(host.editor_state().editor_ui.variables_search, "spa");
}

#[test]
fn search_box_stays_visible_while_filter_narrows_below_threshold() {
    // DIVERGENCE-BY-DESIGN from TS `variables-panel.tsx:153` (which
    // unmounts the box when the FILTERED list drops to <=6 and
    // strands the typed filter).
    let mut host = many_vars_host();
    host.editor_state_mut().editor_ui.variables_search = "spacing".into();
    let panel = VariablesPanel::for_editor(host.editor_state());
    assert_eq!(panel.row_count(), 1);
    assert!(panel.search_visible());
}

// --- #18: scrolling ----------------------------------------------------

#[test]
fn wheel_over_panel_scrolls_rows_and_clamps() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    for i in 1..=20 {
        assert!(state.create_variable(
            &format!("color-{i:02}"),
            VariableKind::Color,
            VariableScalar::Str("#000000".into()),
        ));
    }
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    let cx = rect.origin.x + rect.size.x / 2.0;
    let cy = rect.origin.y + rect.size.y / 2.0;

    // Wheel scroll-down advances the list.
    assert!(host.apply_wheel(cx, cy, -60.0, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state().editor_ui.variables_scroll > 0.0);

    // Huge scroll clamps to max.
    assert!(host.apply_wheel(cx, cy, -1.0e6, VIEWPORT_W, VIEWPORT_H));
    let panel = VariablesPanel::for_editor(host.editor_state());
    let max = panel.max_scroll(rect);
    assert!(max > 0.0);
    assert_eq!(host.editor_state().editor_ui.variables_scroll, max);

    // Scroll back past the top clamps to 0.
    assert!(host.apply_wheel(cx, cy, 1.0e6, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.variables_scroll, 0.0);

    // A scrolled list maps hits through the offset: with max scroll,
    // the LAST row sits just above the footer.
    host.editor_state_mut().editor_ui.variables_scroll = max;
    let panel = VariablesPanel::for_editor(host.editor_state());
    let footer_top = rect.origin.y + rect.size.y - 40.0;
    let hit = panel.hit_test(rect, Point2D::new(rect.origin.x + 60.0, footer_top - 20.0));
    assert_eq!(hit, Some(VariablesPanelHit::NameCell(19)));
}

// --- #18: panel resize ---------------------------------------------------

#[test]
fn edge_press_resizes_panel_and_release_ends_the_drag() {
    let mut host = two_variant_color_host();
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    // Press the right edge strip.
    let edge_x = rect.origin.x + rect.size.x - 2.0;
    let edge_y = rect.origin.y + rect.size.y / 2.0;
    assert!(host.apply_press(edge_x, edge_y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.variables_resize, Some(VariablesResizeEdge::Right));

    // Drag left 200px → panel narrows (but never below the TS 480 min).
    assert!(host.apply_cursor_move(edge_x - 200.0, edge_y));
    let resized = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    assert!(resized.size.x < rect.size.x);
    assert!(resized.size.x >= 480.0);

    assert!(host.apply_release());
    assert_eq!(host.variables_resize, None);

    // Dragging far past the minimum clamps at 480x240.
    let corner_x = resized.origin.x + resized.size.x - 4.0;
    let corner_y = resized.origin.y + resized.size.y - 4.0;
    assert!(host.apply_press(corner_x, corner_y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.variables_resize, Some(VariablesResizeEdge::Corner));
    assert!(host.apply_cursor_move(resized.origin.x + 10.0, resized.origin.y + 10.0));
    let clamped = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    assert_eq!(clamped.size.x, 480.0);
    assert_eq!(clamped.size.y, 240.0);
    let _ = host.apply_release();
}

// --- rust-only: boolean variant cell --------------------------------------

#[test]
fn boolean_value_cell_toggles_clicked_variant_only() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into(), "Variant-1".into()]);
    state.editor_ui.variables_current_axis = Some("Theme-1".into());
    state
        .ui
        .variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    assert!(state.create_variable("flag", VariableKind::Boolean, VariableScalar::Bool(false)));

    let (x, y) = point_for_hit(&host, &VariablesPanelHit::ValueCell { row: 0, variant: 1 });
    assert!(host.apply_press(x, y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        themed_value_for(host.editor_state(), "flag", "Variant-1"),
        Some(&VariableScalar::Bool(true))
    );
    // Active (Default) column resolution unchanged.
    assert_eq!(
        host.editor_state().resolve_variable("flag"),
        Some(&VariableScalar::Bool(false))
    );
}

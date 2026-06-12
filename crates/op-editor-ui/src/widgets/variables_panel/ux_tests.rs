//! Widget-level tests for the #18 management UX: search filter +
//! source-index stability, scroll math, resize edges, color-cell
//! sub-targets, the row overflow menu, and alpha-derived opacity.

use super::*;
use jian_ops_schema::variable::{VariableKind, VariableScalar};

fn panel_rect(panel: &VariablesPanel) -> Rect {
    Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, VARIABLES_PANEL_DEFAULT_HEIGHT),
    }
}

fn state_with_n_colors(n: usize) -> EditorState {
    let mut state = EditorState::new();
    for i in 1..=n {
        assert!(state.create_variable(
            &format!("color-{i:02}"),
            VariableKind::Color,
            VariableScalar::Str("#336699".into()),
        ));
    }
    state
}

#[test]
fn search_box_hidden_below_threshold_then_visible() {
    let state = state_with_n_colors(6);
    let panel = VariablesPanel::for_editor(&state);
    assert!(!panel.search_visible(), "6 rows stay below the threshold");

    let state = state_with_n_colors(7);
    let panel = VariablesPanel::for_editor(&state);
    assert!(panel.search_visible(), "7 rows show the search box");
    let rect = panel_rect(&panel);
    let input = panel.search_input_rect(rect);
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(input.origin.x + 4.0, input.origin.y + 4.0)
        ),
        Some(VariablesPanelHit::SearchBox)
    );
}

#[test]
fn filtered_rows_keep_unfiltered_source_indices() {
    let mut state = state_with_n_colors(7);
    assert!(state.create_variable("spacing", VariableKind::Number, VariableScalar::Num(8.0)));
    state.editor_ui.variables_search = "SPACING".into(); // case-insensitive
    let panel = VariablesPanel::for_editor(&state);
    assert_eq!(panel.row_count(), 1);
    let rect = panel_rect(&panel);
    let viewport = panel.rows_viewport(rect);
    let hit = panel.hit_test(
        rect,
        Point2D::new(rect.origin.x + 60.0, viewport.origin.y + 22.0),
    );
    // "spacing" sorts after color-01..07 → unfiltered index 7.
    assert_eq!(hit, Some(VariablesPanelHit::NameCell(7)));
}

#[test]
fn scroll_clamps_and_offsets_row_hits() {
    let mut state = state_with_n_colors(20);
    state.editor_ui.variables_scroll = 1.0e9; // stale offset self-corrects
    let panel = VariablesPanel::for_editor(&state);
    let rect = panel_rect(&panel);
    let max = panel.max_scroll(rect);
    assert!(max > 0.0);
    assert_eq!(panel.effective_scroll(rect), max);
    // At max scroll the last row hugs the footer.
    let viewport = panel.rows_viewport(rect);
    let hit = panel.hit_test(
        rect,
        Point2D::new(
            rect.origin.x + 60.0,
            viewport.origin.y + viewport.size.y - 10.0,
        ),
    );
    assert_eq!(hit, Some(VariablesPanelHit::NameCell(19)));
    // Points below the rows viewport (the footer) are not rows.
    let hit_footer = panel.hit_test(
        rect,
        Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            viewport.origin.y + viewport.size.y + 10.0,
        ),
    );
    assert!(!matches!(hit_footer, Some(VariablesPanelHit::NameCell(_))));
}

#[test]
fn resize_edges_map_corner_first() {
    let state = state_with_n_colors(1);
    let panel = VariablesPanel::for_editor(&state);
    let rect = panel_rect(&panel);
    let right = rect.origin.x + rect.size.x;
    let bottom = rect.origin.y + rect.size.y;
    assert_eq!(
        panel.resize_edge_at(rect, Point2D::new(right - 2.0, bottom - 2.0)),
        Some(VariablesResizeEdge::Corner)
    );
    assert_eq!(
        panel.resize_edge_at(rect, Point2D::new(right - 2.0, rect.origin.y + 100.0)),
        Some(VariablesResizeEdge::Right)
    );
    assert_eq!(
        panel.resize_edge_at(rect, Point2D::new(rect.origin.x + 100.0, bottom - 2.0)),
        Some(VariablesResizeEdge::Bottom)
    );
    assert_eq!(
        panel.resize_edge_at(
            rect,
            Point2D::new(rect.origin.x + 100.0, rect.origin.y + 100.0)
        ),
        None
    );
    // hit_test routes edges before anything else.
    assert_eq!(
        panel.hit_test(rect, Point2D::new(right - 2.0, bottom - 2.0)),
        Some(VariablesPanelHit::Resize(VariablesResizeEdge::Corner))
    );
}

#[test]
fn color_cell_splits_into_swatch_and_hex_targets() {
    let state = state_with_n_colors(1);
    let panel = VariablesPanel::for_editor(&state);
    let rect = panel_rect(&panel);
    let cell = panel.value_cell_rect_at(rect, 0, 0, 1);
    // Swatch zone → picker target.
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(cell.origin.x + 8.0, cell.origin.y + SWATCH_SIZE)
        ),
        Some(VariablesPanelHit::ColorSwatch { row: 0, variant: 0 })
    );
    // Hex text zone → inline edit target.
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(cell.origin.x + SWATCH_SIZE + 20.0, cell.origin.y + 20.0)
        ),
        Some(VariablesPanelHit::ValueCell { row: 0, variant: 0 })
    );
    // The rest of the cell falls back to the row.
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(cell.origin.x + cell.size.x - 10.0, cell.origin.y + 20.0)
        ),
        Some(VariablesPanelHit::Row(0))
    );
}

#[test]
fn row_menu_button_and_open_menu_route_hits() {
    let mut state = state_with_n_colors(2);
    let panel = VariablesPanel::for_editor(&state);
    let rect = panel_rect(&panel);
    let button = panel.row_menu_button_rect(rect, 1);
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(button.origin.x + 5.0, button.origin.y + 5.0)
        ),
        Some(VariablesPanelHit::RowMenuToggle(1))
    );
    // Open the menu for row 1 — its overlay maps Rename / Delete.
    state.editor_ui.variables_row_menu = Some(1);
    let panel = VariablesPanel::for_editor(&state);
    let (display_idx, menu) = panel.row_menu_rect(rect).expect("menu anchors to its row");
    assert_eq!(display_idx, 1);
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(menu.origin.x + 10.0, menu.origin.y + 10.0)
        ),
        Some(VariablesPanelHit::RowMenuRename(1))
    );
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(menu.origin.x + 10.0, menu.origin.y + 40.0)
        ),
        Some(VariablesPanelHit::RowMenuDelete(1))
    );
}

#[test]
fn row_menu_anchors_to_filtered_position() {
    // With a filter narrowing 8 rows to 1, the open menu of the
    // surviving SOURCE row anchors at display position 0.
    let mut state = state_with_n_colors(7);
    assert!(state.create_variable("spacing", VariableKind::Number, VariableScalar::Num(8.0)));
    state.editor_ui.variables_search = "spacing".into();
    state.editor_ui.variables_row_menu = Some(7);
    let panel = VariablesPanel::for_editor(&state);
    let rect = panel_rect(&panel);
    let (display_idx, _) = panel.row_menu_rect(rect).expect("menu still anchors");
    assert_eq!(display_idx, 0);
    // A filtered-out row's menu has no anchor.
    state.editor_ui.variables_row_menu = Some(0);
    let panel = VariablesPanel::for_editor(&state);
    assert!(panel.row_menu_rect(rect).is_none());
}

#[test]
fn alpha_percent_derives_from_rrggbbaa() {
    use super::paint::scalar_alpha_percent;
    assert_eq!(
        scalar_alpha_percent(&VariableScalar::Str("#11223380".into())),
        Some(50)
    );
    assert_eq!(
        scalar_alpha_percent(&VariableScalar::Str("#112233ff".into())),
        Some(100)
    );
    // 6-char hex (no alpha channel) → None → caller paints 100 %.
    assert_eq!(
        scalar_alpha_percent(&VariableScalar::Str("#112233".into())),
        None
    );
    assert_eq!(scalar_alpha_percent(&VariableScalar::Num(1.0)), None);
}

#[test]
fn no_match_state_reports_empty_filtered_rows() {
    let mut state = state_with_n_colors(7);
    state.editor_ui.variables_search = "zzz".into();
    let panel = VariablesPanel::for_editor(&state);
    assert_eq!(panel.row_count(), 0);
    // The search box must remain reachable to clear the filter
    // (divergence from TS, which unmounts it — see search_visible).
    assert!(panel.search_visible());
}

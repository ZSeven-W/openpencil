//! Row-map, geometry and label tests for `file_menu.rs`.
//!
//! Split out of the widget file to keep it under the 800-line cap.

use super::*;
use jian_widgets::components::menu::MenuHit;

fn menu_panel(menu: &FileMenu<'_>) -> Rect {
    Rect {
        origin: Point2D::new(100.0, 50.0),
        size: Point2D::new(MENU_WIDTH, menu.height()),
    }
}

#[test]
fn hit_uses_shared_menu_state_protocol() {
    let mut ui = EditorUiState::default();
    ui.file_menu.hover = Some(5);
    let menu = FileMenu::for_editor_ui(
        &ui,
        vec![
            RecentEntry {
                name: "one.op".to_string(),
                age: "now".to_string(),
            },
            RecentEntry {
                name: "two.op".to_string(),
                age: "now".to_string(),
            },
        ],
    );
    assert_eq!(menu.menu.hover, Some(5));

    let panel = menu_panel(&menu);
    let divider = DIVIDER_GAP * 2.0 + 1.0;
    let recent_y = panel.origin.y
        + PAD_Y
        + ROW_HEIGHT * 3.0
        + divider
        + ROW_HEIGHT * 2.0
        + divider
        + ROW_HEIGHT
        + divider
        + HEADER_HEIGHT
        + ROW_HEIGHT * 0.5;
    assert_eq!(
        menu.hit(panel, Point2D::new(panel.origin.x + 20.0, recent_y)),
        MenuHit::Row(6)
    );
    assert_eq!(menu.choice_for_row(6), Some(FileMenuChoice::OpenRecent(0)));

    let header_y = recent_y - ROW_HEIGHT * 0.5 - HEADER_HEIGHT * 0.5;
    assert_eq!(
        menu.hit(panel, Point2D::new(panel.origin.x + 20.0, header_y)),
        MenuHit::Inside
    );
    assert_eq!(
        menu.hit(panel, Point2D::new(panel.origin.x - 1.0, header_y)),
        MenuHit::Outside
    );
}

fn two_recents() -> Vec<RecentEntry> {
    vec![
        RecentEntry {
            name: "one.op".to_string(),
            age: "now".to_string(),
        },
        RecentEntry {
            name: "two.op".to_string(),
            age: "now".to_string(),
        },
    ]
}

/// y of the row directly under Export image — where the batch row
/// paints when the host supports it.
fn export_all_row_y(panel: Rect) -> f32 {
    let divider = DIVIDER_GAP * 2.0 + 1.0;
    panel.origin.y
        + PAD_Y
        // New + New from template + Open
        + ROW_HEIGHT * 3.0
        + divider
        + ROW_HEIGHT * 2.0
        + divider
        + ROW_HEIGHT
        + ROW_HEIGHT * 0.5
}

#[test]
fn hosts_without_batch_export_keep_the_original_row_map() {
    let ui = EditorUiState::default();
    assert!(!ui.batch_frame_export_supported);
    let menu = FileMenu::for_editor_ui(&ui, two_recents());
    let panel = menu_panel(&menu);

    assert_eq!(
        menu.choice_for_row(1),
        Some(FileMenuChoice::NewFromTemplate)
    );
    assert_eq!(menu.choice_for_row(5), Some(FileMenuChoice::ExportImage));
    assert_eq!(menu.choice_for_row(6), Some(FileMenuChoice::OpenRecent(0)));
    assert_eq!(menu.choice_for_row(8), Some(FileMenuChoice::ClearRecent));
    // The row under Export image is the divider gutter, not a row.
    assert_eq!(
        menu.hit(
            panel,
            Point2D::new(panel.origin.x + 20.0, export_all_row_y(panel))
        ),
        MenuHit::Inside
    );
}

#[test]
fn batch_export_row_paints_under_export_image_and_shifts_the_recents() {
    let ui = EditorUiState {
        batch_frame_export_supported: true,
        ..Default::default()
    };
    let menu = FileMenu::for_editor_ui(&ui, two_recents());
    let panel = menu_panel(&menu);

    assert_eq!(
        menu.choice_for_row(6),
        Some(FileMenuChoice::ExportAllFrames)
    );
    assert_eq!(menu.choice_for_row(7), Some(FileMenuChoice::OpenRecent(0)));
    assert_eq!(menu.choice_for_row(9), Some(FileMenuChoice::ClearRecent));

    // Hit-test agrees with the paint walk: the row right below
    // Export image is the batch row.
    assert_eq!(
        menu.hit(
            panel,
            Point2D::new(panel.origin.x + 20.0, export_all_row_y(panel))
        ),
        MenuHit::Row(6)
    );

    let plain_ui = EditorUiState::default();
    let without = FileMenu::for_editor_ui(&plain_ui, two_recents());
    assert_eq!(menu.height(), without.height() + ROW_HEIGHT);
}

#[test]
fn batch_export_label_names_the_selected_frames_only_from_two_up() {
    let ui = EditorUiState {
        batch_frame_export_supported: true,
        locale: op_i18n::Locale::EnUs,
        ..Default::default()
    };
    let all = FileMenu::for_editor_ui(&ui, vec![]);
    assert_eq!(all.export_all_label(), "Export all frames");
    assert_eq!(
        FileMenu::for_editor_ui(&ui, vec![])
            .with_selected_frames(1)
            .export_all_label(),
        "Export all frames"
    );
    assert_eq!(
        FileMenu::for_editor_ui(&ui, vec![])
            .with_selected_frames(3)
            .export_all_label(),
        "Export 3 frames"
    );
}

#[test]
fn recent_columns_keep_an_exact_gap_before_the_right_aligned_age() {
    let age_width = 42.0;
    let (name_x, name_budget, age_x) = recent_row_columns(0.0, age_width);
    let name_right = name_x + name_budget;

    assert_eq!(age_x + age_width, MENU_WIDTH - PAD_X);
    assert_eq!(age_x - name_right, RECENT_COLUMN_GAP);
}

#[test]
fn measured_truncation_stays_inside_its_budget() {
    let measure = |text: &str| text.chars().count() as f32 * 7.0;
    let output =
        truncate_to_width_measured("openpencil-super-long-project-file-name.op", 98.0, measure);

    assert!(output.ends_with('…'), "{output:?}");
    assert!(measure(&output) <= 98.0, "{output:?}");
    assert_eq!(truncate_to_width_measured("abc", 0.0, measure), "");
}

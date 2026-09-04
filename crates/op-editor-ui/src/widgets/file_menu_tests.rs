//! Row-map, geometry and label tests for `file_menu.rs`.
//!
//! Split out of the widget file to keep it under the 800-line cap.

use super::*;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, RenderBackend, TextLayout};
use jian_widgets::components::menu::MenuHit;
use op_editor_core::scene_template_catalog::TemplateScene;

fn menu_panel(menu: &FileMenu<'_>) -> Rect {
    Rect {
        origin: Point2D::new(100.0, 50.0),
        size: Point2D::new(MENU_WIDTH, menu.height()),
    }
}

#[derive(Default)]
struct TextCaptureBackend {
    texts: Vec<(String, Point2D)>,
}

impl RenderBackend for TextCaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, point: Point2D) {
        self.texts
            .extend(layout.runs().iter().map(|run| (run.content.clone(), point)));
    }
    fn clip_rect(&mut self, _: Rect) {}
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
fn desktop_template_save_row_sits_after_save_as_and_shifts_later_rows() {
    let ui = EditorUiState {
        ..Default::default()
    };
    let mut ui = ui;
    ui.scene_template_center.save_current_supported = true;
    let menu = FileMenu::for_editor_ui(&ui, two_recents());
    let panel = menu_panel(&menu);
    let divider = DIVIDER_GAP * 2.0 + 1.0;
    let row_y =
        panel.origin.y + PAD_Y + ROW_HEIGHT * 3.0 + divider + ROW_HEIGHT * 2.0 + ROW_HEIGHT * 0.5;

    assert_eq!(menu.choice_for_row(5), Some(FileMenuChoice::SaveAsTemplate));
    assert_eq!(menu.choice_for_row(6), Some(FileMenuChoice::ExportImage));
    assert_eq!(menu.choice_for_row(7), Some(FileMenuChoice::OpenRecent(0)));
    assert_eq!(menu.choice_for_row(9), Some(FileMenuChoice::ClearRecent));
    assert_eq!(
        menu.hit(panel, Point2D::new(panel.origin.x + 20.0, row_y)),
        MenuHit::Row(5)
    );

    let plain_ui = EditorUiState::default();
    let without = FileMenu::for_editor_ui(&plain_ui, two_recents());
    assert_eq!(menu.height(), without.height() + ROW_HEIGHT);

    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    menu.paint(&mut cx, panel);
    let label_y = |label: &str| {
        backend
            .texts
            .iter()
            .find_map(|(text, point)| (text == label).then_some(point.y))
            .unwrap_or_else(|| panic!("label {label:?} did not paint"))
    };
    let save_as_y = label_y(t(&ui, "saveAs"));
    let template_y = label_y(t(&ui, "saveAsTemplate"));
    let export_y = label_y(t(&ui, "exportImage"));
    assert!(save_as_y < template_y && template_y < export_y);
}

#[test]
fn template_save_composes_with_batch_and_deck_rows() {
    let mut ui = desktop_ui(Some(TemplateScene::Slides));
    ui.scene_template_center.save_current_supported = true;
    let menu = FileMenu::for_editor_ui(&ui, two_recents());

    assert_eq!(menu.choice_for_row(5), Some(FileMenuChoice::SaveAsTemplate));
    assert_eq!(menu.choice_for_row(6), Some(FileMenuChoice::ExportImage));
    assert_eq!(
        menu.choice_for_row(7),
        Some(FileMenuChoice::ExportAllFrames)
    );
    assert_eq!(
        menu.choice_for_row(8),
        Some(FileMenuChoice::ExportSlideshowHtml)
    );
    assert_eq!(menu.choice_for_row(9), Some(FileMenuChoice::ExportPptx));
    assert_eq!(menu.choice_for_row(10), Some(FileMenuChoice::OpenRecent(0)));
    assert_eq!(menu.choice_for_row(12), Some(FileMenuChoice::ClearRecent));

    let without_template = desktop_ui(Some(TemplateScene::Slides));
    let without = FileMenu::for_editor_ui(&without_template, two_recents());
    assert_eq!(menu.height(), without.height() + ROW_HEIGHT);
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

/// y of the second row under Export image — where the deck-slideshow
/// row paints on a desktop host that also offers the batch row.
fn deck_html_row_y(panel: Rect) -> f32 {
    export_all_row_y(panel) + ROW_HEIGHT
}

/// y of the PowerPoint row, directly under the slideshow one.
fn deck_pptx_row_y(panel: Rect) -> f32 {
    deck_html_row_y(panel) + ROW_HEIGHT
}

/// A desktop-shaped host: both export capabilities advertised.
fn desktop_ui(scenario: Option<TemplateScene>) -> EditorUiState {
    EditorUiState {
        batch_frame_export_supported: true,
        deck_html_export_supported: true,
        scenario,
        ..Default::default()
    }
}

#[test]
fn the_deck_rows_paint_under_the_batch_row_and_shift_the_recents() {
    let ui = desktop_ui(Some(TemplateScene::Slides));
    let menu = FileMenu::for_editor_ui(&ui, two_recents());
    let panel = menu_panel(&menu);

    assert_eq!(
        menu.choice_for_row(6),
        Some(FileMenuChoice::ExportAllFrames)
    );
    assert_eq!(
        menu.choice_for_row(7),
        Some(FileMenuChoice::ExportSlideshowHtml)
    );
    assert_eq!(menu.choice_for_row(8), Some(FileMenuChoice::ExportPptx));
    assert_eq!(menu.choice_for_row(9), Some(FileMenuChoice::OpenRecent(0)));
    assert_eq!(menu.choice_for_row(11), Some(FileMenuChoice::ClearRecent));

    // Hit-test agrees with the paint walk.
    assert_eq!(
        menu.hit(
            panel,
            Point2D::new(panel.origin.x + 20.0, deck_html_row_y(panel))
        ),
        MenuHit::Row(7)
    );
    assert_eq!(
        menu.hit(
            panel,
            Point2D::new(panel.origin.x + 20.0, deck_pptx_row_y(panel))
        ),
        MenuHit::Row(8)
    );

    let batch_only_ui = EditorUiState {
        batch_frame_export_supported: true,
        ..Default::default()
    };
    let without = FileMenu::for_editor_ui(&batch_only_ui, two_recents());
    assert_eq!(menu.height(), without.height() + ROW_HEIGHT * 2.0);
}

#[test]
fn only_a_deck_document_is_offered_the_deck_exports() {
    for scenario in [None, Some(TemplateScene::Carousel)] {
        let ui = desktop_ui(scenario);
        let menu = FileMenu::for_editor_ui(&ui, two_recents());
        let panel = menu_panel(&menu);

        // Rows 7 and 8 are the recent files again, not deck-export rows.
        assert_eq!(
            menu.choice_for_row(7),
            Some(FileMenuChoice::OpenRecent(0)),
            "scenario={scenario:?}"
        );
        assert_eq!(
            menu.choice_for_row(8),
            Some(FileMenuChoice::OpenRecent(1)),
            "scenario={scenario:?}"
        );
        // The first place a deck row could paint is the divider gutter
        // under the batch row.
        assert_eq!(
            menu.hit(
                panel,
                Point2D::new(panel.origin.x + 20.0, deck_html_row_y(panel))
            ),
            MenuHit::Inside,
            "scenario={scenario:?}"
        );
    }
}

#[test]
fn a_host_without_the_exporter_never_paints_the_deck_rows() {
    // Web: a deck document, but no save picker + offscreen rasteriser.
    let ui = EditorUiState {
        scenario: Some(TemplateScene::Slides),
        ..Default::default()
    };
    assert!(!ui.deck_html_export_supported);
    let menu = FileMenu::for_editor_ui(&ui, two_recents());

    assert_eq!(menu.choice_for_row(5), Some(FileMenuChoice::ExportImage));
    assert_eq!(menu.choice_for_row(6), Some(FileMenuChoice::OpenRecent(0)));
    let plain_ui = EditorUiState::default();
    let plain = FileMenu::for_editor_ui(&plain_ui, two_recents());
    assert_eq!(menu.height(), plain.height());
}

#[test]
fn the_deck_rows_sit_directly_under_export_image_when_batch_export_is_absent() {
    let ui = EditorUiState {
        deck_html_export_supported: true,
        scenario: Some(TemplateScene::Slides),
        ..Default::default()
    };
    let menu = FileMenu::for_editor_ui(&ui, two_recents());
    let panel = menu_panel(&menu);

    assert_eq!(
        menu.choice_for_row(6),
        Some(FileMenuChoice::ExportSlideshowHtml)
    );
    assert_eq!(menu.choice_for_row(7), Some(FileMenuChoice::ExportPptx));
    assert_eq!(menu.choice_for_row(8), Some(FileMenuChoice::OpenRecent(0)));
    assert_eq!(
        menu.hit(
            panel,
            Point2D::new(panel.origin.x + 20.0, export_all_row_y(panel))
        ),
        MenuHit::Row(6)
    );
    assert_eq!(
        menu.hit(
            panel,
            Point2D::new(panel.origin.x + 20.0, deck_html_row_y(panel))
        ),
        MenuHit::Row(7)
    );
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

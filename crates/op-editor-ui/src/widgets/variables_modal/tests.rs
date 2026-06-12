use super::*;
use crate::widgets::PaintCx;
use jian_ops_schema::variable::{VariableKind, VariableScalar};

#[derive(Default)]
struct CaptureBackend {
    svg_origins: Vec<Point2D>,
    svg_sizes: Vec<f32>,
    texts: Vec<String>,
}

impl crate::RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &crate::TextLayout, _: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push(run.content.clone());
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, origin: Point2D, size: f32, _: Color, _: f32) {
        self.svg_origins.push(origin);
        self.svg_sizes.push(size);
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn test_label_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.58
}

#[test]
fn modal_rect_matches_large_centered_manager_shape() {
    let modal = VariablesModal::for_editor(&EditorState::new());
    let rect = modal.rect(1920.0, 1080.0);
    assert_eq!(rect.size.x, VARIABLES_MODAL_MAX_W);
    assert_eq!(rect.size.y, VARIABLES_MODAL_MAX_H);
    assert!(rect.origin.y >= TOP_BAR_HEIGHT);
    assert!(rect.origin.x > 100.0);
}

#[test]
fn modal_hit_test_resolves_close_and_footer_add() {
    let modal = VariablesModal::for_editor(&EditorState::new());
    let rect = modal.rect(1200.0, 800.0);
    let close = close_rect(rect);
    assert_eq!(
        modal.hit_test(rect, close.origin + close.size / 2.0),
        VariablesModalHit::Close
    );
    let add = footer_add_rect(rect);
    assert_eq!(
        modal.hit_test(rect, add.origin + add.size / 2.0),
        VariablesModalHit::AddVariable
    );
}

#[test]
fn modal_hit_test_resolves_variable_rows() {
    let mut state = EditorState::new();
    state.create_variable(
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff0000".into()),
    );
    let modal = VariablesModal::for_editor(&state);
    let rect = modal.rect(1200.0, 800.0);
    let body = body_rect(rect);
    assert_eq!(
        modal.hit_test(
            rect,
            Point2D::new(body.origin.x + PAD_X + 4.0, body.origin.y + ROW_H / 2.0)
        ),
        VariablesModalHit::Row(0)
    );
}

#[test]
fn open_preset_menu_wins_the_hit_test_over_the_header() {
    let mut state = EditorState::new();
    state.create_variable(
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff0000".into()),
    );
    assert!(state.save_theme_preset("kit", 1));
    state.editor_ui.variables_preset_menu_open = true;
    let modal = VariablesModal::for_editor(&state);
    let rect = modal.rect(1200.0, 800.0);
    let anchor = preset_button_rect(rect);
    let menu = ThemePresetMenu::for_editor(&state);
    let menu_rect = menu.menu_rect(anchor);

    // First row inside the dropdown = save-current.
    assert_eq!(
        modal.hit_test(
            rect,
            Point2D::new(menu_rect.origin.x + 20.0, menu_rect.origin.y + 16.0)
        ),
        VariablesModalHit::Preset(PresetMenuHit::SaveCurrent)
    );
    // The preset button itself stays a PresetMenu toggle hit.
    assert_eq!(
        modal.hit_test(rect, anchor.origin + anchor.size / 2.0),
        VariablesModalHit::PresetMenu
    );
    // Closed menu — header press resolves normally again.
    state.editor_ui.variables_preset_menu_open = false;
    let closed = VariablesModal::for_editor(&state);
    assert_eq!(
        closed.hit_test(
            rect,
            Point2D::new(menu_rect.origin.x + 20.0, menu_rect.origin.y + 16.0)
        ),
        VariablesModalHit::Inside
    );
}

#[test]
fn dropdown_chevrons_follow_localized_labels() {
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::ZhCn;
    let modal = VariablesModal::for_editor(&state);
    let rect = modal.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    modal.paint(&mut cx, rect);

    let preset = preset_button_rect(rect);
    let preset_expected = preset.origin.x + 28.0 + test_label_width("预设", 15.0) + 7.0;
    let preset_chevron = backend
        .svg_origins
        .iter()
        .zip(backend.svg_sizes.iter())
        .find(|(origin, size)| {
            (**size - 18.0).abs() < f32::EPSILON
                && origin.x > preset.origin.x + 28.0
                && origin.x < preset.origin.x + preset.size.x
                && origin.y >= preset.origin.y
                && origin.y < preset.origin.y + preset.size.y
        })
        .map(|(origin, _)| *origin)
        .expect("preset chevron should paint inside the preset button");
    assert!(
        (preset_chevron.x - preset_expected).abs() <= 1.0,
        "preset chevron should sit after the localized label; got {}, expected {}",
        preset_chevron.x,
        preset_expected
    );

    let add = footer_add_rect(rect);
    let add_expected = add.origin.x + 28.0 + test_label_width("添加变量", 15.0) + 7.0;
    let add_chevron = backend
        .svg_origins
        .iter()
        .zip(backend.svg_sizes.iter())
        .find(|(origin, size)| {
            (**size - 18.0).abs() < f32::EPSILON
                && origin.x > add.origin.x + 28.0
                && origin.x < add.origin.x + add.size.x
                && origin.y >= add.origin.y
                && origin.y < add.origin.y + add.size.y
        })
        .map(|(origin, _)| *origin)
        .expect("add-variable chevron should paint inside the add-variable button");
    assert!(
        (add_chevron.x - add_expected).abs() <= 1.0,
        "add-variable chevron should sit after the localized label; got {}, expected {}",
        add_chevron.x,
        add_expected
    );
}

#[test]
fn open_preset_menu_paints_saved_names_and_io_rows() {
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::EnUs;
    state.create_variable(
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff0000".into()),
    );
    assert!(state.save_theme_preset("Brand kit", 1));
    state.editor_ui.variables_preset_menu_open = true;
    let modal = VariablesModal::for_editor(&state);
    let rect = modal.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    modal.paint(&mut cx, rect);

    for expected in [
        "Save Current as Preset…",
        "Brand kit",
        "Import from File…",
        "Export to File…",
    ] {
        assert!(
            backend.texts.iter().any(|t| t == expected),
            "open preset menu should paint {expected:?}; painted: {:?}",
            backend.texts
        );
    }
}

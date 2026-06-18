use crate::widgets::{DesignMdHit, DesignMdPanel, PaintCx};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::{ButtonPressTarget, DesignMdButton, EditorState};

#[derive(Default)]
struct CaptureBackend {
    round_fills: Vec<(Rect, f32, Color)>,
    texts: Vec<(String, Point2D)>,
    icons: Vec<(Point2D, f32)>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}

    fn end_frame(&mut self) {}

    fn fill_rect(&mut self, _rect: Rect, _color: Color) {}

    fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {}

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push((run.content.clone(), origin));
        }
    }

    fn clip_rect(&mut self, _rect: Rect) {}

    fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {}

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.round_fills.push((rect, radius, color));
    }

    fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {}

    fn stroke_svg_path(
        &mut self,
        _d: &str,
        top_left: Point2D,
        size: f32,
        _color: Color,
        _width: f32,
    ) {
        if !self.icons.iter().any(|(p, s)| {
            (p.x - top_left.x).abs() < 0.01
                && (p.y - top_left.y).abs() < 0.01
                && (*s - size).abs() < 0.01
        }) {
            self.icons.push((top_left, size));
        }
    }

    fn save(&mut self) {}

    fn restore(&mut self) {}

    fn translate(&mut self, _offset: Point2D) {}

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn open_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.design_md_panel_open = true;
    state.doc.design_md = Some(op_editor_core::parse_design_md(
        "# Brief\n\n## Visual Theme\nWarm system",
    ));
    state
}

fn long_palette_state() -> EditorState {
    let mut markdown = String::from("# Design System: Long\n\n## Color Palette\n");
    for index in 0..40 {
        markdown.push_str(&format!(
            "- **color-{index:02}** (#{index:02X}{index:02X}{index:02X}) - role {index}\n"
        ));
    }
    let mut state = EditorState::default();
    state.editor_ui.design_md_panel_open = true;
    state.doc.design_md = Some(op_editor_core::parse_design_md(&markdown));
    state
}

fn empty_open_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.design_md_panel_open = true;
    state.doc.design_md = None;
    state
}

fn find_hit(panel: &DesignMdPanel<'_>, rect: Rect, target: DesignMdHit) -> Option<Point2D> {
    let mut y = rect.origin.y;
    while y <= rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x <= rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if panel.hit_test(rect, point) == Some(target) {
                return Some(point);
            }
            x += 3.0;
        }
        y += 3.0;
    }
    None
}

fn approx_label_width(label: &str) -> f32 {
    label
        .chars()
        .map(|ch| if ch.is_ascii() { 6.0 } else { 11.0 })
        .sum()
}

fn painted_text_y(state: &EditorState, label: &str) -> f32 {
    let panel = DesignMdPanel::for_editor(state).expect("open");
    let rect = Rect::xywh(0.0, 0.0, 480.0, 560.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    backend
        .texts
        .iter()
        .find_map(|(text, origin)| (text == label).then_some(origin.y))
        .expect("text should paint")
}

#[test]
fn for_editor_picks_up_pressed_design_md_button() {
    let mut state = open_state();
    state.editor_ui.pressed_button = Some(ButtonPressTarget::DesignMd(DesignMdButton::Import));

    let panel = DesignMdPanel::for_editor(&state).expect("open");

    assert_eq!(panel.pressed, Some(DesignMdButton::Import));
}

#[test]
fn header_auto_generate_button_is_hittable_and_hoverable() {
    let state = open_state();
    let panel = DesignMdPanel::for_editor(&state).expect("open");
    let rect = Rect::xywh(0.0, 0.0, 480.0, 560.0);
    let point = find_hit(&panel, rect, DesignMdHit::AutoGenerate)
        .expect("auto-generate header button should be hittable");

    assert_eq!(
        panel.hover_at(rect, point),
        Some(DesignMdButton::AutoGenerate)
    );
}

#[test]
fn content_paint_uses_design_md_scroll_offset() {
    let mut state = long_palette_state();
    let panel = DesignMdPanel::for_editor(&state).expect("open");
    let rect = Rect::xywh(0.0, 0.0, 480.0, 560.0);
    assert!(panel.max_scroll(rect) > 96.0);
    let unscrolled_y = painted_text_y(&state, "color-12");

    state.editor_ui.design_md_scroll.offset = 96.0;
    let scrolled_y = painted_text_y(&state, "color-12");

    assert!((unscrolled_y - scrolled_y - 96.0).abs() < 0.01);
}

#[test]
fn empty_state_shows_import_and_auto_generate_actions() {
    let state = empty_open_state();
    let panel = DesignMdPanel::for_editor(&state).expect("open");
    let rect = Rect::xywh(0.0, 0.0, 480.0, 560.0);

    assert!(
        find_hit(&panel, rect, DesignMdHit::Import).is_some(),
        "empty state should expose import design.md action"
    );
    assert!(
        find_hit(&panel, rect, DesignMdHit::AutoGenerate).is_some(),
        "empty state should expose auto-generate action"
    );
}

#[test]
fn empty_state_action_button_contents_are_centered() {
    let state = empty_open_state();
    let panel = DesignMdPanel::for_editor(&state).expect("open");
    let rect = Rect::xywh(0.0, 0.0, 480.0, 560.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    for label in ["导入 design.md", "自动生成"] {
        let (text, text_origin) = backend
            .texts
            .iter()
            .find(|(text, _)| text == label)
            .expect("empty action label should paint");
        let button = backend
            .round_fills
            .iter()
            .map(|(rect, _, _)| *rect)
            .find(|button| {
                (button.size.x - 138.0).abs() < 0.01
                    && (button.size.y - 32.0).abs() < 0.01
                    && button.contains(*text_origin)
            })
            .expect("empty action button background should contain its label");
        let (icon_origin, _) = backend
            .icons
            .iter()
            .find(|(origin, size)| (*size - 14.0).abs() < 0.01 && button.contains(*origin))
            .expect("empty action button should paint an icon");
        let content_left = icon_origin.x;
        let content_right = text_origin.x + approx_label_width(text);
        let content_center = (content_left + content_right) / 2.0;
        let button_center = button.origin.x + button.size.x / 2.0;

        assert!(
            (content_center - button_center).abs() <= 2.0,
            "{label} content center should align with button center; content={content_center}, button={button_center}"
        );
    }
}

#[test]
fn pressed_section_header_paints_pressed_feedback() {
    let mut state = open_state();
    state.editor_ui.pressed_button = Some(ButtonPressTarget::DesignMd(
        DesignMdButton::ToggleSection(0),
    ));
    let panel = DesignMdPanel::for_editor(&state).expect("open");
    let rect = Rect::xywh(0.0, 0.0, 480.0, 560.0);
    let expected = panel
        .theme
        .button_hover
        .with_alpha(panel.theme.button_hover.a * 1.8);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(_, radius, color)| *radius == 7.0 && *color == expected),
        "pressed section header should paint shared pressed feedback"
    );
}

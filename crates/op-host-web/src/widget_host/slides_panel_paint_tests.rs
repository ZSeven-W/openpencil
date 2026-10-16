//! Content-level regressions: preview pixels must come from the board paired
//! with each row, stay inside the list, and reflect edits and page changes.

use super::WidgetHost;
use op_editor_core::{EditorState, LeftPanelTab};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
struct Capture {
    fills: Vec<(Rect, Color)>,
    clips: Vec<Rect>,
    texts: Vec<String>,
    depth: usize,
}

impl RenderBackend for Capture {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.fills.push((rect, color));
    }
    fn fill_round_rect(&mut self, rect: Rect, _: f32, color: Color) {
        self.fills.push((rect, color));
    }
    fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
        self.texts
            .extend(layout.runs().iter().map(|run| run.content.clone()));
    }
    fn clip_rect(&mut self, rect: Rect) {
        self.clips.push(rect);
    }
    fn save(&mut self) {
        self.depth += 1;
    }
    fn restore(&mut self) {
        self.depth = self.depth.checked_sub(1).expect("balanced preview clips");
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn host() -> WidgetHost {
    let document = jian_ops_schema::load_str(
        r##"{
        "version":"1.0.0", "children":[
            {"type":"frame","id":"phone","x":80,"y":20,"width":390,"height":844,
             "fill":[{"type":"solid","color":"#ff0000"}], "children":[
                {"type":"text","id":"title","x":24,"y":48,"width":300,"height":70,
                 "content":"Phone content","fontSize":32,
                 "fill":[{"type":"solid","color":"#ffffff"}]}]},
            {"type":"frame","id":"wide","x":600,"y":20,"width":1600,"height":900,
             "fill":[{"type":"solid","color":"#0000ff"}],"children":[]}
        ]
    }"##,
    )
    .unwrap()
    .value;
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::from_document(document);
    host.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    host.editor_state.editor_ui.host_owns_chat = true;
    host.mark_dirty();
    host
}

#[test]
fn previews_paint_each_boards_content_at_its_own_aspect() {
    let mut host = host();
    let frame = host.slides_panel_frame(1400.0, 900.0).unwrap();
    let mut backend = Capture::default();
    host.paint_slides_panel(&mut backend, &frame);
    for (index, color) in [Color::RED, Color::BLUE].into_iter().enumerate() {
        let (rect, _) = backend
            .fills
            .iter()
            .find(|(_, fill)| *fill == color)
            .expect("real board fill must replace the placeholder");
        let thumb = frame.layout.thumb_rect(index);
        assert!((rect.origin.x - thumb.origin.x).abs() < 0.01);
        assert!((rect.origin.y - thumb.origin.y).abs() < 0.01);
        assert!((rect.size.x - thumb.size.x).abs() < 0.01);
        assert!((rect.size.y - thumb.size.y).abs() < 0.01);
    }
    assert!(backend
        .texts
        .iter()
        .any(|text| text.contains("Phone content")));
    assert_eq!(backend.depth, 0);
}

#[test]
fn previews_use_current_scene_after_an_edit_and_stop_at_the_footer() {
    let mut host = host();
    let before = host.slides_panel_frame(1400.0, 270.0).unwrap();
    let mut first = Capture::default();
    host.paint_slides_panel(&mut first, &before);
    assert!(first.fills.iter().any(|(_, fill)| *fill == Color::RED));
    // Remove the first board as an edit. The former second board must now
    // occupy row zero immediately, with no cache retaining the old picture.
    host.editor_state.active_children_mut().remove(0);
    host.editor_state.mark_document_changed();
    host.mark_dirty();
    let frame = host.slides_panel_frame(1400.0, 270.0).unwrap();
    let mut after = Capture::default();
    host.paint_slides_panel(&mut after, &frame);
    assert!(!after.fills.iter().any(|(_, fill)| *fill == Color::RED));
    assert!(after.fills.iter().any(|(_, fill)| *fill == Color::BLUE));
    assert!(after
        .clips
        .contains(&frame.layout.visible_thumb_rect(0).unwrap()));
    assert_eq!(after.depth, 0);
}

#[test]
fn switching_pages_replaces_the_preview_content_and_board_ids() {
    let mut host = host();
    let doc = jian_ops_schema::load_str(
        r##"{
        "version":"1.0.0","children":[],"pages":[
            {"id":"p1","name":"First","children":[
                {"type":"frame","id":"red","width":390,"height":844,
                 "fill":[{"type":"solid","color":"#ff0000"}],"children":[]}]},
            {"id":"p2","name":"Second","children":[
                {"type":"frame","id":"blue","width":960,"height":640,
                 "fill":[{"type":"solid","color":"#0000ff"}],"children":[]}]}
        ]
    }"##,
    )
    .unwrap()
    .value;
    host.editor_state = EditorState::from_document(doc);
    host.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    host.mark_dirty();
    let first = host.slides_panel_frame(1400.0, 900.0).unwrap();
    assert_eq!(first.chips[0].id, "red");
    let mut pixels = Capture::default();
    host.paint_slides_panel(&mut pixels, &first);
    assert!(pixels.fills.iter().any(|(_, color)| *color == Color::RED));

    assert!(host.editor_state.set_active_page(1));
    host.mark_dirty();
    let second = host.slides_panel_frame(1400.0, 900.0).unwrap();
    assert_eq!(second.chips[0].id, "blue");
    let mut pixels = Capture::default();
    host.paint_slides_panel(&mut pixels, &second);
    assert!(!pixels.fills.iter().any(|(_, color)| *color == Color::RED));
    assert!(pixels.fills.iter().any(|(_, color)| *color == Color::BLUE));
}

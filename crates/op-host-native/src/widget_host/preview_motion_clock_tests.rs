//! Native preview lifecycle-motion coverage against the production paint path.

#![cfg(all(test, feature = "gl-host", not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::EditorState;
use op_editor_ui::{Color, ImageBlendMode, Point2D, Rect, RenderBackend, TextLayout};

const VIEWPORT_W: f32 = 1000.0;
const VIEWPORT_H: f32 = 700.0;

fn document() -> jian_ops_schema::PenDocument {
    jian_ops_schema::load_str(
        r##"{
            "version":"1.1","formatVersion":"1.1","id":"motion-clock",
            "app":{"name":"motion-clock","version":"1","id":"motion-clock"},
            "children":[{
                "type":"frame","id":"screen","x":0,"y":0,"width":390,"height":844,
                "fill":[{"type":"solid","color":"#ffffff"}],
                "children":[{
                    "type":"rectangle","id":"hero","x":16,"y":24,"width":358,"height":180,
                    "opacity":1,"fill":[{"type":"solid","color":"#111111"}],
                    "animations":[{"trigger":"mount","keyframes":[
                        {"offset":0,"values":{"opacity":0}},
                        {"offset":1,"values":{"opacity":1}}
                    ],"durationMs":400,"easing":"linear"}]
                }]
            }]
        }"##,
    )
    .expect("native motion-clock fixture parses")
    .value
}

#[derive(Default)]
struct OpacityCaptureBackend {
    composite_alphas: Vec<f32>,
}

impl RenderBackend for OpacityCaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn push_composite_layer(&mut self, _: Rect, opacity: f32, _: ImageBlendMode) {
        self.composite_alphas.push(opacity);
    }
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

#[test]
fn mount_motion_is_visible_after_enter_at_a_late_native_host_clock() {
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(document()));
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    host.set_now_ms(600_000);
    assert!(host.enter_preview((VIEWPORT_W, VIEWPORT_H)));

    host.set_now_ms(600_100);
    let _ = host.pump_preview();

    let canvas_rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VIEWPORT_W, VIEWPORT_H),
    };
    let mut backend = OpacityCaptureBackend::default();
    host.paint_device_frame(&mut backend, canvas_rect);
    assert!(
        host.preview_scene_for_test()
            .active_page()
            .and_then(|page| page.find("hero"))
            .is_some_and(|hero| hero.opacity.abs() < 0.001),
        "hero stays at its first keyframe under the merge"
    );

    host.set_now_ms(600_230);
    let _ = host.pump_preview();
    host.settle_mode_transition();
    let mut settled_backend = OpacityCaptureBackend::default();
    host.paint_device_frame(&mut settled_backend, canvas_rect);
    assert!(
        host.preview_scene_for_test()
            .active_page()
            .and_then(|page| page.find("hero"))
            .is_some_and(|hero| hero.opacity.abs() < 0.001),
        "hero starts at its first keyframe when the merge settles"
    );

    host.set_now_ms(600_330);
    assert!(
        host.pump_preview(),
        "the mount track should request a repaint"
    );
    let mut mid_backend = OpacityCaptureBackend::default();
    host.paint_device_frame(&mut mid_backend, canvas_rect);
    let mid_opacity = host
        .preview_scene_for_test()
        .active_page()
        .and_then(|page| page.find("hero"))
        .map(|hero| hero.opacity)
        .expect("hero node");
    assert!(
        mid_opacity > 0.0 && mid_opacity < 1.0,
        "hero mount opacity must be mid-tween after the merge: {mid_opacity}"
    );

    host.set_now_ms(600_900);
    assert!(host.pump_preview(), "the mount completion should repaint");
    let mut final_backend = OpacityCaptureBackend::default();
    host.paint_device_frame(&mut final_backend, canvas_rect);
    let final_opacity = host
        .preview_scene_for_test()
        .active_page()
        .and_then(|page| page.find("hero"))
        .map(|hero| hero.opacity)
        .expect("hero node");
    assert!((final_opacity - 1.0).abs() < 0.001);
}

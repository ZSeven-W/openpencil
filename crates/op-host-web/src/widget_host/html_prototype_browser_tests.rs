use super::{frame_rgba, init_backend, install_frame_clock, FrameClock};
use crate::canvaskit::BrowserMeasure;
use jian_core::action::services::Router;
use jian_core::geometry::point;
use jian_core::gesture::pointer::{PointerEvent, PointerPhase};
use js_sys::{Promise, Uint8Array};
use op_codegen::{Codegen, Html};
use op_editor_ui::{Point2D, Rect, RenderBackend};
use op_preview_core::{PreviewInput, PreviewInputEnvelope, PreviewSession};
use std::rc::Rc;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::HtmlCanvasElement;

const SOURCE: &str =
    include_str!("../../../op-preview-core/tests/fixtures/motion/html_dynamic_prototype.op");
const SIDE: u32 = 160;

#[wasm_bindgen(module = "/tests/html-prototype-parity.js")]
extern "C" {
    #[wasm_bindgen(js_name = mountPrototype)]
    fn mount_prototype(html: &str) -> Promise;
    #[wasm_bindgen(js_name = disposePrototype)]
    fn dispose_prototype();
    #[wasm_bindgen(js_name = setPrototypeTime)]
    fn set_prototype_time(now: f64);
    #[wasm_bindgen(js_name = advancePrototypeFrame)]
    fn advance_prototype_frame() -> Promise;
    #[wasm_bindgen(js_name = tapPrototype)]
    fn tap_prototype(x: f32, y: f32);
    #[wasm_bindgen(js_name = prototypeRoute)]
    fn prototype_route() -> String;
    #[wasm_bindgen(js_name = prototypeRgba)]
    fn prototype_rgba() -> Promise;
    #[wasm_bindgen(js_name = capturePrototypePair)]
    fn capture_prototype_pair(now: f64) -> Promise;
}

struct PrototypeFrame;

impl Drop for PrototypeFrame {
    fn drop(&mut self) {
        dispose_prototype();
    }
}

#[wasm_bindgen_test(async)]
async fn html_prototype_dynamic_routes_match_play_frame_by_frame() {
    let script = concat!(
        include_str!("../../assets/canvaskit/canvaskit.js"),
        "\nglobalThis.CanvasKitInit = CanvasKitInit;"
    );
    js_sys::eval(script).unwrap();
    install_frame_clock(include_bytes!("../../assets/canvaskit/canvaskit.wasm"));
    let _clock = FrameClock;
    let doc = jian_ops_schema::load_str(SOURCE).unwrap().value;
    JsFuture::from(mount_prototype(&Html.generate(&doc)))
        .await
        .unwrap();
    let _prototype = PrototypeFrame;

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    canvas.set_id("play-prototype");
    canvas.set_width(SIDE);
    canvas.set_height(SIDE);
    canvas
        .set_attribute(
            "style",
            "position:fixed;left:160px;top:0;width:160px;height:160px",
        )
        .unwrap();
    document.body().unwrap().append_child(&canvas).unwrap();
    let mut backend = init_backend("play-prototype", 1.0, SIDE, SIDE)
        .await
        .unwrap();
    let size = (SIDE as f32, SIDE as f32);
    let mut play = PreviewSession::enter(
        &doc,
        size,
        &Default::default(),
        0,
        true,
        false,
        Rc::new(BrowserMeasure::new(backend.op_ck())),
        0,
    )
    .unwrap();
    assert!(play.is_app_mode());
    play.begin_lifecycle(0);

    let clip = Rect::xywh(0.0, 0.0, size.0, size.1);
    for now in (0..=960).step_by(16) {
        set_prototype_time(now as f64);
        play.pump(now);
        if matches!(now, 32 | 416 | 640 | 832) {
            tap_prototype(24.0, 20.0);
            for phase in [PointerPhase::Down, PointerPhase::Up] {
                let event = PointerEvent::simple_at(1, phase, point(24.0, 20.0), now);
                play.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(event)));
            }
        }
        play.reconcile(now);
        JsFuture::from(advance_prototype_frame()).await.unwrap();
        let expected_path = match now {
            224..=447 => "/detail",
            448..=639 => "/alternate",
            _ => "/",
        };
        assert_eq!(
            play.current_path_for_test(),
            expected_path,
            "Play route at {now}ms"
        );
        assert_eq!(prototype_route(), expected_path, "HTML route at {now}ms");
        let route = play.router_for_test().current();
        assert_eq!(route.stack.len(), if expected_path == "/" { 1 } else { 2 });

        let (root, _) = play.framed_root().unwrap();
        backend.begin_frame();
        play.paint_framed_animated(
            &mut backend,
            &root,
            clip,
            Point2D::ZERO,
            1.0,
            None,
            None,
            now,
        );
        backend.end_frame();
        let play_frame = frame_rgba("play-prototype", SIDE, SIDE).to_vec();
        let html_frame = Uint8Array::new(&JsFuture::from(prototype_rgba()).await.unwrap()).to_vec();
        assert_eq!(play_frame.len(), (SIDE * SIDE * 4) as usize);
        assert_eq!(html_frame.len(), play_frame.len());
        let mismatch = play_frame.iter().zip(&html_frame).position(|(a, b)| a != b);
        assert_eq!(
            mismatch.map(|i| (
                i / 4 % SIDE as usize,
                i / 4 / SIDE as usize,
                i % 4,
                play_frame[i],
                html_frame[i]
            )),
            None,
            "RGBA at {now}ms, {expected_path} (x, y, channel, Play, HTML)",
        );
        JsFuture::from(capture_prototype_pair(now as f64))
            .await
            .unwrap();
    }
    canvas.remove();
}

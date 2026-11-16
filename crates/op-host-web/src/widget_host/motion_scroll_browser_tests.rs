use super::WidgetHost;
use crate::canvaskit::init_backend;
use js_sys::{Promise, Uint8Array};
use op_editor_core::EditorState;
use op_editor_ui::{Point2D, Rect, RenderBackend};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::HtmlCanvasElement;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

const SOURCE: &str =
    include_str!("../../../op-preview-core/tests/fixtures/motion/standalone_scroll.op");
const WIDTH: u32 = 390;
const HEIGHT: u32 = 844;
const PLAY_WIDTH: f32 = 1200.0;
const PLAY_HEIGHT: f32 = 1060.0;

#[wasm_bindgen(module = "/tests/motion-parity.js")]
extern "C" {
    #[wasm_bindgen(js_name = installFrameClock)]
    fn install_frame_clock(wasm: &[u8]);
    #[wasm_bindgen(js_name = restoreFrameClock)]
    fn restore_frame_clock();
    #[wasm_bindgen(js_name = setFrameTime)]
    fn set_frame_time(now: f64);
    #[wasm_bindgen(js_name = presentFrame)]
    fn present_frame() -> Promise;
    fn wheel(canvas: &HtmlCanvasElement, delta_y: f32);
    #[wasm_bindgen(js_name = frameRgba)]
    fn frame_rgba(id: &str, width: u32, height: u32) -> Uint8Array;
}

struct FrameClock;

impl Drop for FrameClock {
    fn drop(&mut self) {
        restore_frame_clock();
    }
}

fn test_canvas(id: &str) -> HtmlCanvasElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    canvas.set_id(id);
    canvas.set_width(WIDTH);
    canvas.set_height(HEIGHT);
    canvas.style().set_property("width", "390px").unwrap();
    canvas.style().set_property("height", "844px").unwrap();
    document.body().unwrap().append_child(&canvas).unwrap();
    canvas
}

fn expect_pixel(frame: &[u8], x: usize, y: usize, rgb: [u8; 3]) {
    let offset = (y * WIDTH as usize + x) * 4;
    assert_eq!(&frame[offset..offset + 3], &rgb, "point ({x}, {y})");
}

#[wasm_bindgen_test(async)]
async fn standalone_wheel_and_web_play_match_every_browser_frame() {
    let script = concat!(
        include_str!("../../assets/canvaskit/canvaskit.js"),
        "\nglobalThis.CanvasKitInit = CanvasKitInit;"
    );
    js_sys::eval(script).unwrap();
    install_frame_clock(include_bytes!("../../assets/canvaskit/canvaskit.wasm"));
    let _clock = FrameClock;
    let jian_canvas = test_canvas("jian-scroll");
    let play_canvas = test_canvas("play-scroll");
    let jian = jian_host_web::mount_jian(
        jian_canvas.clone(),
        JsValue::from_str(SOURCE),
        JsValue::UNDEFINED,
    )
    .await
    .unwrap();

    let mut backend = init_backend("play-scroll", 1.0, WIDTH, HEIGHT)
        .await
        .unwrap();
    let doc = jian_ops_schema::load_str(SOURCE).unwrap().value;
    let mut play = WidgetHost::new();
    play.editor_state = EditorState::from_document(doc);
    assert!(play.enter_preview_from_browser(PLAY_WIDTH, PLAY_HEIGHT, Some(backend.op_ck())));
    // Settle the editor's entry chrome before comparing content surfaces.
    play.set_now_ms(300);
    play.settle_mode_transition();
    let frame = play.preview_device_frame.as_ref().unwrap();
    assert_eq!(frame.fit, 1.0);
    assert_eq!(frame.viewport_h, HEIGHT as f32);
    assert_eq!(op_preview_core::device_frame::scroll_max(frame), 336.0);
    let clip = Rect::xywh(0.0, 0.0, WIDTH as f32, HEIGHT as f32);
    let mut expected_scroll = 0.0_f32;

    for now in (0..=960).step_by(16) {
        let play_now = now + 300;
        play.set_now_ms(play_now);
        let session = play.preview.as_mut().unwrap();
        session.pump(play_now);
        set_frame_time(now as f64);
        if let Some(delta) = match now {
            64 => Some(96.0),
            256 => Some(96.0),
            448 => Some(-192.0),
            496 => Some(192.0),
            704 => Some(1_000.0),
            832 => Some(-1_000.0),
            _ => None,
        } {
            wheel(&jian_canvas, delta);
            assert!(play.apply_wheel(32.0, 32.0, -delta, PLAY_WIDTH, PLAY_HEIGHT));
            expected_scroll = (expected_scroll + delta).clamp(0.0, 336.0);
            assert_eq!(play.preview_scroll_y, expected_scroll);
        }
        JsFuture::from(present_frame()).await.unwrap();
        backend.begin_frame();
        let session = play.preview.as_ref().unwrap();
        session.paint_framed_animated(
            &mut backend,
            "screen",
            clip,
            Point2D::new(0.0, -play.preview_scroll_y),
            1.0,
            None,
            None,
            play_now,
        );
        backend.end_frame();

        let play_frame = frame_rgba("play-scroll", WIDTH, HEIGHT).to_vec();
        let jian_frame = frame_rgba("jian-scroll", WIDTH, HEIGHT).to_vec();
        assert_eq!(play_frame.len(), (WIDTH * HEIGHT * 4) as usize);
        assert_eq!(jian_frame.len(), play_frame.len());
        let difference = play_frame.iter().zip(&jian_frame).position(|(a, b)| a != b);
        assert_eq!(
            difference.map(|i| (
                i / 4 % WIDTH as usize,
                i / 4 / WIDTH as usize,
                i % 4,
                play_frame[i],
                jian_frame[i]
            )),
            None,
            "at {now}ms, scroll {expected_scroll}"
        );

        match now {
            0 | 832 => expect_pixel(&jian_frame, 40, 24, [64, 64, 160]),
            144 => expect_pixel(&jian_frame, 76, 806, [160, 64, 32]),
            336 | 576 => expect_pixel(&jian_frame, 76, 818, [32, 64, 160]),
            496 => {
                expect_pixel(&jian_frame, 40, 710, [255, 255, 255]);
                expect_pixel(&jian_frame, 76, 710, [160, 64, 32]);
            }
            704 => expect_pixel(&jian_frame, 40, 820, [32, 112, 64]),
            _ => {}
        }
    }
    jian.dispose();
    jian_canvas.remove();
    play_canvas.remove();
}

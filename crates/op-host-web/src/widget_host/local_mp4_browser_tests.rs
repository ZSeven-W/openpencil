use super::{frame_rgba, init_backend, install_frame_clock, FrameClock};
use crate::canvaskit::motion_export::{encode_mp4, LocalMotionExport};
use mp4::{MediaType, Mp4Reader};
use op_editor_core::motion_export::{
    MotionExportBackground, MotionExportFormat, MotionExportRequest, MotionExportTrigger,
};
use op_editor_core::EditorState;
use op_editor_ui::{Color, Rect, RenderBackend};
use std::io::Cursor;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::HtmlCanvasElement;

const SOURCE: &str =
    include_str!("../../../op-preview-core/tests/fixtures/motion/timeline_export_parity.op");

#[wasm_bindgen_test(async)]
async fn browser_encodes_h264_with_the_shared_clock_and_keeps_its_display() {
    let script = concat!(
        include_str!("../../assets/canvaskit/canvaskit.js"),
        "\nglobalThis.CanvasKitInit = CanvasKitInit;"
    );
    js_sys::eval(script).unwrap();
    install_frame_clock(include_bytes!("../../assets/canvaskit/canvaskit.wasm"));
    let _clock = FrameClock;
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    canvas.set_id("mp4-display");
    canvas.set_width(32);
    canvas.set_height(32);
    document.body().unwrap().append_child(&canvas).unwrap();
    let mut backend = init_backend("mp4-display", 1.0, 32, 32).await.unwrap();
    backend.begin_frame();
    backend.fill_rect(Rect::xywh(0.0, 0.0, 32.0, 32.0), Color::WHITE);
    backend.end_frame();
    let before = frame_rgba("mp4-display", 32, 32).to_vec();

    let doc = jian_ops_schema::load_str(SOURCE).unwrap().value;
    let state = EditorState::from_document(doc);
    let request = MotionExportRequest {
        root_id: "entry".into(),
        format: MotionExportFormat::Mp4,
        trigger: MotionExportTrigger::Mount,
        fps: 30,
        scale: 2.0,
        duration_ms: None,
        repeat: false,
        background: MotionExportBackground::White,
        reduced_motion: None,
    };
    let plan = LocalMotionExport::prepare(&state, &request).unwrap();
    let bytes = encode_mp4(backend.op_ck().clone(), plan).await.unwrap();
    assert_eq!(frame_rgba("mp4-display", 32, 32).to_vec(), before);

    let size = bytes.len() as u64;
    let mut reader = Mp4Reader::read_header(Cursor::new(bytes), size).unwrap();
    let track = &reader.tracks()[&1];
    assert_eq!(track.media_type().unwrap(), MediaType::H264);
    assert_eq!((track.width(), track.height()), (320, 256));
    assert_eq!(track.timescale(), 30);
    assert_eq!(reader.sample_count(1).unwrap(), 16);
    for index in 0..16 {
        let sample = reader.read_sample(1, index + 1).unwrap().unwrap();
        assert_eq!(
            (sample.start_time, sample.duration, sample.rendering_offset),
            (u64::from(index), 1, 0)
        );
    }
    canvas.remove();
}

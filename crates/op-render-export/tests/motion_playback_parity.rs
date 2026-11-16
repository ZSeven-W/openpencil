#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

use std::io::Cursor;

use jian_core::geometry::point;
use jian_core::gesture::pointer::{PointerEvent, PointerPhase};
use op_editor_core::editor_ui_state::MotionTimelineTab;
use op_editor_core::motion_export::{
    MotionExportBackground, MotionExportFormat, MotionExportRequest, MotionExportTrigger,
};
use op_editor_core::EditorState;
use op_editor_ui::layout_scene::LayoutScene;
use op_editor_ui::widgets::canvas_paint_scene::canvas_paint_scene;
use op_editor_ui::widgets::{CanvasViewport, PaintCx, Widget};
use op_editor_ui::{Point2D, Rect};
use op_host_native::{NativeBackend, NativeFrameBackend};
use op_preview_core::{PreviewInput, PreviewInputEnvelope, PreviewSession};
use op_render_export::{export_editor_state, EditorExportScope};
use skia_safe::{surfaces, AlphaType, Color, ColorType, ImageInfo, Surface};

#[path = "motion_playback_parity/mp4.rs"]
mod mp4;

const DOCUMENT: &str =
    include_str!("../../op-preview-core/tests/fixtures/motion/timeline_export_parity.op");
const WIDTH: usize = 160;
const HEIGHT: usize = 128;
const FRAME_STEP_MS: u64 = 20;
const FPS: u32 = 50;
const DURATION_MS: u64 = 500;

struct Case {
    page: usize,
    root: &'static str,
    tab: MotionTimelineTab,
    trigger: MotionExportTrigger,
}

impl Case {
    fn manual_id(&self) -> Option<String> {
        match &self.trigger {
            MotionExportTrigger::Manual(id) => Some(id.clone()),
            _ => None,
        }
    }
}

fn cases() -> [Case; 3] {
    [
        Case {
            page: 0,
            root: "entry",
            tab: MotionTimelineTab::Mount,
            trigger: MotionExportTrigger::Mount,
        },
        Case {
            page: 1,
            root: "manual-screen",
            tab: MotionTimelineTab::Manual,
            trigger: MotionExportTrigger::Manual("reveal".into()),
        },
        Case {
            page: 2,
            root: "visible-screen",
            tab: MotionTimelineTab::InView,
            trigger: MotionExportTrigger::InView,
        },
    ]
}

fn enter(case: &Case) -> (EditorState, PreviewSession) {
    let doc = jian_ops_schema::load_str(DOCUMENT).unwrap().value;
    let mut play = PreviewSession::enter(
        &doc,
        (WIDTH as f32, HEIGHT as f32),
        &Default::default(),
        case.page,
        false,
        false,
        op_pen_loader::layout_measure_backend(),
        0,
    )
    .unwrap();
    let mut editor = EditorState::from_document(doc);
    editor.ui.active_page_index = case.page;
    editor.viewport.pan_x = 0.0;
    editor.viewport.pan_y = 0.0;
    editor.viewport.zoom = 1.0;
    editor.toggle_motion_timeline();
    editor.set_motion_timeline_tab(case.tab, case.manual_id());
    assert_eq!(play.framed_root().unwrap().0, case.root);
    assert!(!editor.motion_timeline_projection().strips.is_empty());

    play.begin_lifecycle(0);
    if case.manual_id().is_some() {
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let event = PointerEvent::simple_at(1, phase, point(22.0, 20.0), 0);
            play.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(event)));
        }
        assert!(
            play.active_animation_track_count() > 0,
            "the authored button must start the clip"
        );
    }
    (editor, play)
}

fn pixels(surface: &mut Surface) -> Vec<u8> {
    let info = ImageInfo::new(
        (WIDTH as i32, HEIGHT as i32),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    let mut rgba = vec![0; WIDTH * HEIGHT * 4];
    assert!(surface.read_pixels(&info, &mut rgba, WIDTH * 4, (0, 0)));
    rgba
}

fn assert_pixels(case: &Case, time: u64, source: &str, play: &[u8], other: &[u8]) {
    assert_eq!(
        other.len(),
        play.len(),
        "{} {source} at {time}ms",
        case.root
    );
    assert_eq!(
        play.iter().zip(other).position(|(a, b)| a != b).map(|i| (
            i / 4 % WIDTH,
            i / 4 / WIDTH,
            i % 4,
            play[i],
            other[i],
        )),
        None,
        "{} {source} at {time}ms (x, y, channel, Play, actual)",
        case.root
    );
}

fn matched_play_frames(
    case: &Case,
    editor: &mut EditorState,
    play: &mut PreviewSession,
    base: &LayoutScene,
) -> Vec<Vec<u8>> {
    let viewport = Rect::xywh(0.0, 0.0, WIDTH as f32, HEIGHT as f32);
    let mut backend = NativeBackend::with_dpi(1.0);
    let mut surface = surfaces::raster_n32_premul((WIDTH as i32, HEIGHT as i32)).unwrap();
    let mut frames = Vec::new();
    for time in (0..=DURATION_MS).step_by(FRAME_STEP_MS as usize) {
        play.pump(time);
        surface.canvas().clear(Color::WHITE);
        play.paint_framed_animated(
            &mut NativeFrameBackend::new(&mut backend, surface.canvas()),
            case.root,
            viewport,
            Point2D::ZERO,
            1.0,
            None,
            None,
            time,
        );
        let play_frame = pixels(&mut surface);

        editor.scrub_motion_playhead(time as f64, time);
        let timeline_scene = canvas_paint_scene(editor, base, None, time);
        let mut canvas =
            CanvasViewport::from_editor(editor, timeline_scene.as_ref().unwrap_or(base));
        canvas.now_ms = time;
        surface.canvas().clear(Color::WHITE);
        canvas.paint(
            &mut PaintCx {
                backend: &mut NativeFrameBackend::new(&mut backend, surface.canvas()),
            },
            viewport,
        );
        assert_pixels(
            case,
            time,
            "canvas timeline",
            &play_frame,
            &pixels(&mut surface),
        );
        frames.push(play_frame);
    }
    assert!(
        frames.windows(2).filter(|pair| pair[0] != pair[1]).count() > 5,
        "{} must actually animate",
        case.root
    );
    frames
}

fn export(case: &Case, editor: &EditorState, format: MotionExportFormat) -> Vec<u8> {
    export_editor_state(
        editor,
        &EditorExportScope::Motion(MotionExportRequest {
            root_id: case.root.into(),
            format,
            trigger: case.trigger.clone(),
            fps: FPS,
            scale: 1.0,
            duration_ms: Some(DURATION_MS),
            repeat: false,
            background: MotionExportBackground::White,
            reduced_motion: None,
        }),
    )
    .unwrap()
    .bytes
}

fn read_png(decoder: &mut png::Reader<impl std::io::BufRead + std::io::Seek>) -> Vec<u8> {
    let mut out = vec![0; decoder.output_buffer_size().unwrap()];
    let info = decoder.next_frame(&mut out).unwrap();
    assert_eq!((info.width as usize, info.height as usize), (WIDTH, HEIGHT));
    let data = &out[..info.buffer_size()];
    match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect(),
        other => panic!("unexpected export color type {other:?}"),
    }
}

fn compare_lossless(case: &Case, editor: &EditorState, expected: &[Vec<u8>]) {
    let mut archive = zip::ZipArchive::new(Cursor::new(export(
        case,
        editor,
        MotionExportFormat::PngSequence,
    )))
    .unwrap();
    assert_eq!(archive.len(), expected.len());
    let mut apng = png::Decoder::new(Cursor::new(export(case, editor, MotionExportFormat::Apng)))
        .read_info()
        .unwrap();
    assert_eq!(
        apng.info().animation_control.unwrap().num_frames as usize,
        expected.len()
    );
    for (index, play) in expected.iter().enumerate() {
        let time = index as u64 * FRAME_STEP_MS;
        let mut file = archive.by_name(&format!("frame-{index:04}.png")).unwrap();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut bytes).unwrap();
        let mut png = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
        assert_pixels(case, time, "PNG sequence", play, &read_png(&mut png));
        assert_pixels(case, time, "APNG", play, &read_png(&mut apng));
        let control = apng.info().frame_control.unwrap();
        assert_eq!(
            u64::from(control.delay_num) * 1000,
            FRAME_STEP_MS * u64::from(control.delay_den)
        );
    }
}

fn compare_gif(case: &Case, editor: &EditorState, expected: &[Vec<u8>]) {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options
        .read_info(Cursor::new(export(case, editor, MotionExportFormat::Gif)))
        .unwrap();
    for (index, play) in expected.iter().enumerate() {
        let frame = decoder.read_next_frame().unwrap().expect("GIF frame");
        assert_eq!(
            (frame.width as usize, frame.height as usize),
            (WIDTH, HEIGHT)
        );
        assert_eq!(u64::from(frame.delay) * 10, FRAME_STEP_MS);
        assert_pixels(
            case,
            index as u64 * FRAME_STEP_MS,
            "GIF",
            play,
            &frame.buffer,
        );
    }
    assert!(decoder.read_next_frame().unwrap().is_none());
}

#[test]
fn timeline_play_and_rust_exports_match_the_same_op() {
    for case in cases() {
        let (mut editor, mut play) = enter(&case);
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&editor);
        let frames = matched_play_frames(&case, &mut editor, &mut play, &scene);
        compare_lossless(&case, &editor, &frames);
        compare_gif(&case, &editor, &frames);
        mp4::compare(&case, &editor, &frames);
    }
}

#![cfg(not(target_os = "windows"))]

use jian_core::geometry::{point, size};
use jian_core::gesture::pointer::{PointerEvent, PointerPhase};
use jian_core::render::{paint_scene_commands, RenderBackend};
use jian_core::screens::{reconcile_screens_with_layout, ScreenRouter};
use jian_core::Runtime;
use jian_skia::SkiaBackend;
use op_editor_ui::{Point2D, Rect};
use op_host_native::backend::{NativeBackend, NativeFrameBackend};
use op_preview_core::{PreviewInput, PreviewInputEnvelope, PreviewSession};
use std::rc::Rc;

const SOURCE: &str =
    include_str!("../../op-preview-core/tests/fixtures/motion/standalone_playback.op");
const WIDTH: usize = 160;
const HEIGHT: usize = 128;

#[test]
fn standalone_jian_and_openpencil_play_render_the_same_op_frame_by_frame() {
    let size_px = (WIDTH as f32, HEIGHT as f32);
    let doc = jian_ops_schema::load_str(SOURCE).unwrap().value;
    let mut play = PreviewSession::enter(
        &doc,
        size_px,
        &Default::default(),
        0,
        true,
        false,
        Rc::new(jian_skia::SkiaMeasure::new()),
        0,
    )
    .unwrap();
    play.begin_lifecycle(0);
    assert!(play.is_app_mode());

    let mut jian = Runtime::new();
    jian.enable_standalone_motion();
    jian.set_viewport_size_without_relayout(size_px);
    jian.load_str(SOURCE).unwrap();
    jian.build_layout_with(Rc::new(jian_skia::SkiaMeasure::new()), size_px)
        .unwrap();
    jian.rebuild_spatial();
    let table = jian.screen_table().expect("standalone screen table");
    let mut mounted = table.entry_path().to_owned();
    let router = Rc::new(ScreenRouter::new(&mounted, table.paths()));
    jian.nav = router.clone();

    let mut jian_backend = SkiaBackend::new();
    let mut jian_surface = jian_backend.new_surface(size(size_px.0, size_px.1));
    let mut play_backend = NativeBackend::new(SkiaBackend::new(), 1.0);
    let mut play_surface = jian_backend.new_surface(size(size_px.0, size_px.1));
    let mut play_rgba = vec![0; WIDTH * HEIGHT * 4];
    let mut jian_rgba = play_rgba.clone();
    let mut distinct = [0usize; 4];

    for now in (0..=800).step_by(16) {
        play.pump(now);
        jian.note_time(now);
        if let Some((x, y)) = match now {
            192 => Some((24., 56.)),
            384 => Some((24., 88.)),
            576 => Some((24., 24.)),
            _ => None,
        } {
            for phase in [PointerPhase::Down, PointerPhase::Up] {
                let event = PointerEvent::simple_at(1, phase, point(x, y), now);
                play.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(
                    event.clone(),
                )));
                jian.dispatch_pointer(event);
            }
        }
        play.reconcile(now);
        reconcile_screens_with_layout(&mut jian, &router, &table, &mut mounted).unwrap();
        jian.pump(now);
        let (root, _) = play.framed_root().unwrap();
        assert_eq!(
            root,
            if (384..576).contains(&now) {
                "detail"
            } else {
                "home"
            }
        );
        assert_eq!(
            play.transition_active(),
            (384..544).contains(&now) || (576..736).contains(&now)
        );

        play_surface.canvas().clear(skia_safe::Color::WHITE);
        play.paint_framed_animated(
            &mut NativeFrameBackend::new(&mut play_backend, play_surface.canvas()),
            &root,
            Rect::xywh(0.0, 0.0, size_px.0, size_px.1),
            Point2D::new(0.0, 0.0),
            1.0,
            None,
            None,
            now,
        );
        let previous = play_rgba.clone();
        assert!(play_surface.read_rgba8(&mut play_rgba));
        if previous != play_rgba {
            distinct[(now / 192).min(3) as usize] += 1;
        }
        jian_backend.begin_frame(&mut jian_surface, 0xffffffff);
        paint_scene_commands(&mut jian_backend, jian.standalone_paint_commands());
        jian_backend.end_frame(&mut jian_surface);
        assert!(jian_surface.read_rgba8(&mut jian_rgba));
        assert_eq!(
            play_rgba
                .iter()
                .zip(&jian_rgba)
                .position(|(a, b)| a != b)
                .map(|index| (
                    index / 4 % WIDTH,
                    index / 4 / WIDTH,
                    index % 4,
                    play_rgba[index],
                    jian_rgba[index]
                )),
            None,
            "{now}ms: first mismatching pixel (x, y, channel, Play, Jian)"
        );
        jian.frame_presented();
    }
    assert!(distinct.into_iter().all(|count| count >= 5), "{distinct:?}");
    assert_eq!(jian.next_wake_ms(), None);
}

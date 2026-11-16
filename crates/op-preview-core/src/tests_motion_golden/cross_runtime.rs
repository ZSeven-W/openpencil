use super::{
    apply_event, enter, fixtures, phone_frame, EventKind, SESSION_END_MS, SESSION_STEP_MS,
};
use jian_core::geometry::point;
use jian_core::gesture::pointer::{PointerEvent, PointerKind, PointerPhase};
use jian_core::motion::{InViewWindow, MotionOverrides};
use jian_core::Runtime;
use serde_json::Value;

fn assert_overrides(name: &str, now: u64, play: MotionOverrides, jian: MotionOverrides) {
    assert_eq!(
        play.keys().collect::<Vec<_>>(),
        jian.keys().collect::<Vec<_>>(),
        "{name} at {now}ms: Play {play:?}, Jian {jian:?}"
    );
    for (key, value) in play {
        let other = &jian[&key];
        if let (Some(play), Some(jian)) = (value.as_f64(), other.as_f64()) {
            assert!(
                (play - jian).abs() < 1e-6,
                "{name} at {now}ms {key:?}: Play {play}, Jian {jian}"
            );
        } else {
            assert_eq!(&value, other, "{name} at {now}ms {key:?}");
        }
    }
}

#[test]
fn same_op_files_match_every_playback_sample() {
    for (name, fixture) in fixtures() {
        let mut play = enter(name, fixture.text, fixture.canvas);
        let mut jian = Runtime::new();
        jian.enable_standalone_motion();
        jian.set_viewport_size_without_relayout(fixture.canvas);
        jian.load_str(fixture.text).unwrap();
        jian.build_layout_with(super::test_measure(), fixture.canvas)
            .unwrap();
        jian.rebuild_spatial();
        let window_height = fixture.phone_window.then(|| {
            let frame = phone_frame(&play);
            play.sync_in_view_viewport(Some(&frame), 0.0);
            jian.set_standalone_in_view_window(Some(InViewWindow::new(
                0.0,
                0.0,
                frame.viewport_h,
                Vec::new(),
            )));
            frame.viewport_h
        });

        let mut animated_frames = 0;
        for now in (0..=SESSION_END_MS).step_by(SESSION_STEP_MS as usize) {
            play.pump(now);
            jian.pump(now);
            for event in fixture.events.iter().filter(|event| event.at_ms == now) {
                apply_event(&mut play, event, now);
                match event.kind {
                    EventKind::DeviceScroll { scroll_y } => {
                        jian.set_standalone_in_view_window(Some(InViewWindow::new(
                            scroll_y,
                            0.0,
                            window_height.unwrap(),
                            Vec::new(),
                        )));
                    }
                    EventKind::Hover { x, y } => {
                        jian.dispatch_pointer(PointerEvent {
                            kind: PointerKind::Mouse,
                            t_ms: now,
                            ..PointerEvent::simple(1, PointerPhase::Hover, point(x, y))
                        });
                    }
                    EventKind::Tap { x, y } => {
                        for phase in [PointerPhase::Down, PointerPhase::Up] {
                            jian.dispatch_pointer(PointerEvent {
                                t_ms: now,
                                ..PointerEvent::simple(2, phase, point(x, y))
                            });
                        }
                    }
                }
            }
            let _ = play.preview_scene_for_test();
            jian.pump(now);
            let play_overrides = play.animation.overrides();
            animated_frames += usize::from(play_overrides.values().any(|v| v != &Value::Null));
            assert_overrides(
                name,
                now,
                play_overrides,
                jian.standalone_motion_overrides(),
            );
        }
        assert!(animated_frames > 0, "{name} must exercise visible motion");
    }
}

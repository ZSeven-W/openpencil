//! Kinematics tests for the generation cursor's waypoint and idle motion.

use crate::widgets::canvas_agent_cursor_motion::{
    cursor_kinematics, ease_out_cubic, parked_cursor_position, ParkedWindow, Waypoint,
};
use crate::{Point2D, Rect};

fn wp(start_ms: u64, x: f32, y: f32) -> Waypoint {
    Waypoint {
        start_ms,
        pos: Point2D::new(x, y),
        rect: Rect::xywh(x - 5.0, y - 5.0, 10.0, 10.0),
    }
}

#[test]
fn empty_queue_yields_no_cursor() {
    assert!(cursor_kinematics(&[], 1_000).is_none());
}

#[test]
fn cursor_arrives_exactly_at_reveal_start() {
    let wps = [wp(1_000, 100.0, 50.0), wp(1_400, 300.0, 90.0)];
    let kin = cursor_kinematics(&wps, 1_400).unwrap();
    assert!((kin.pos.x - 300.0).abs() < 0.01 && (kin.pos.y - 90.0).abs() < 0.01);
    assert_eq!(kin.current, Some(1));
    assert_eq!(
        kin.parked,
        Some(ParkedWindow {
            since_ms: 1_400,
            until_ms: None,
        })
    );
}

#[test]
fn cursor_flies_between_waypoints_without_idle_motion() {
    let wps = [wp(1_000, 100.0, 50.0), wp(1_300, 300.0, 50.0)];
    let kin = cursor_kinematics(&wps, 1_150).unwrap();
    assert!(kin.pos.x > 100.0 && kin.pos.x < 300.0);
    assert_eq!(kin.current, Some(0));
    assert!(kin.parked.is_none(), "real flights never inherit idle sway");
}

#[test]
fn long_gaps_expose_a_park_window_and_still_arrive_on_time() {
    let wps = [wp(1_000, 100.0, 50.0), wp(5_000, 300.0, 50.0)];
    let hold = cursor_kinematics(&wps, 1_500).unwrap();
    assert!((hold.pos.x - 100.0).abs() < 0.01);
    assert!((hold.alpha - 1.0).abs() < 0.01);
    assert_eq!(hold.parked.unwrap().until_ms, Some(4_650));
    let arrive = cursor_kinematics(&wps, 5_000).unwrap();
    assert!((arrive.pos.x - 300.0).abs() < 0.01);
}

#[test]
fn entry_fades_in_toward_first_waypoint_without_idle_motion() {
    let wps = [wp(2_000, 100.0, 50.0)];
    assert!(cursor_kinematics(&wps, 1_700).is_none());
    let kin = cursor_kinematics(&wps, 1_875).unwrap();
    assert!(kin.alpha > 0.0 && kin.alpha < 1.0);
    assert!(kin.pos.x < 100.0 && kin.pos.y < 50.0);
    assert!(kin.current.is_none());
    assert!(kin.parked.is_none());
}

#[test]
fn exhausted_queue_stays_parked_for_the_live_run() {
    let wps = [wp(1_000, 100.0, 50.0)];
    for probe in [1_500u64, 2_500, 30_000] {
        let kin = cursor_kinematics(&wps, probe).unwrap();
        assert!((kin.alpha - 1.0).abs() < 0.01);
        assert_eq!(kin.pos, Point2D::new(100.0, 50.0));
        assert_eq!(kin.current, Some(0));
        assert_eq!(kin.parked.unwrap().until_ms, None);
    }
}

#[test]
fn idle_sway_starts_at_anchor_moves_slightly_and_stays_bounded() {
    let anchor = Point2D::new(100.0, 50.0);
    let parked = ParkedWindow {
        since_ms: 1_000,
        until_ms: None,
    };
    assert_eq!(parked_cursor_position(anchor, parked, 1_000), anchor);
    assert_eq!(parked_cursor_position(anchor, parked, 1_200), anchor);

    let moved = parked_cursor_position(anchor, parked, 1_500);
    assert!((moved.x - anchor.x).abs() > 0.1 || (moved.y - anchor.y).abs() > 0.1);
    assert!((moved.x - anchor.x).abs() <= 3.0);
    assert!((moved.y - anchor.y).abs() <= 1.5);
}

#[test]
fn idle_sway_reaches_the_authored_three_by_one_point_five_pixel_amplitude() {
    let anchor = Point2D::new(100.0, 50.0);
    let parked = ParkedWindow {
        since_ms: 1_000,
        until_ms: None,
    };

    let x_peak = parked_cursor_position(anchor, parked, 1_670);
    let y_peak = parked_cursor_position(anchor, parked, 1_445);

    assert!((x_peak.x - anchor.x - 3.0).abs() < 0.001);
    assert!((y_peak.y - anchor.y - 1.5).abs() < 0.001);
}

#[test]
fn idle_sway_returns_to_anchor_before_departure() {
    let anchor = Point2D::new(100.0, 50.0);
    let parked = ParkedWindow {
        since_ms: 1_000,
        until_ms: Some(5_000),
    };
    let near_departure = parked_cursor_position(anchor, parked, 4_999);
    assert!((near_departure.x - anchor.x).abs() < 0.001);
    assert!((near_departure.y - anchor.y).abs() < 0.001);
    assert_eq!(parked_cursor_position(anchor, parked, 5_000), anchor);
}

#[test]
fn equal_start_waypoints_coalesce_on_the_last_placement() {
    let wps = [
        wp(1_000, 100.0, 50.0),
        wp(1_000, 200.0, 80.0),
        wp(1_400, 400.0, 90.0),
    ];
    let at_shared_start = cursor_kinematics(&wps, 1_000).unwrap();
    assert!((at_shared_start.pos.x - 200.0).abs() < 0.01);
    let mid_flight = cursor_kinematics(&wps, 1_200).unwrap();
    assert!(mid_flight.pos.x > 200.0);
}

#[test]
fn ease_out_cubic_hits_endpoints() {
    assert!(ease_out_cubic(0.0).abs() < 1e-6);
    assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-6);
    assert!(ease_out_cubic(0.5) > 0.5);
}

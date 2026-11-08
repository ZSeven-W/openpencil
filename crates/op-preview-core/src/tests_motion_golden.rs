//! Motion golden-vector generator (motion plan step W0 / Jian T0).
//!
//! Records today's preview motion behaviour as fixed JSON before the runtime
//! moves into Jian's `jian-motion` crate. The port must reproduce these
//! vectors exactly, so this module only reads the production code paths —
//! it never re-implements a curve or a sampling rule.
//!
//! The generator is `#[ignore]`d and writes nothing unless asked:
//!
//! ```sh
//! OP_MOTION_GOLDEN_OUT=<dir> cargo test -p op-preview-core tests_motion_golden -- --ignored
//! ```
//!
//! It writes `curves.json`, `tracks.json`, `sessions.json` and
//! `transitions.json` into `<dir>`. The non-ignored test below keeps the
//! generator compiling and its fixtures loadable on every test run.

#![cfg(test)]

use super::{test_measure, PreviewSession};
use crate::animation::golden_access;
use crate::device_frame::{compute_frame_geometry, DeviceFrame};
use crate::transition::{
    ScreenTransition, TransitionKind, PUSH_POP_DURATION_MS, REPLACE_DURATION_MS,
};
use jian_core::action::animation_registry::animatable_property_registry;
use jian_core::action::services::{
    AnimationDirection, AnimationFillMode, AnimationProperty, AnimationRequest,
};
use jian_core::gesture::pointer::{PointerKind, PointerPhase};
use jian_ops_schema::motion::Easing as SchemaEasing;
use op_editor_core::PreviewDeviceKind;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode, ScenePage};
use op_editor_ui::{Point2D, Rect};
use serde_json::{json, Value};

const SOURCE_BASELINE: &str = "openpencil@742bc1433 (op-preview-core)";
const CURVE_SAMPLES: usize = 1001;
const TRACK_STEP_MS: u64 = 8;
const SESSION_STEP_MS: u64 = 16;
const SESSION_END_MS: u64 = 2000;
const TRANSITION_STEP_MS: u64 = 8;
const TRANSITION_START_MS: u64 = 1000;

const STAGGER_ENTRANCE: &str = include_str!("../tests/fixtures/motion/stagger_entrance.op");
const IN_VIEW_LIST: &str = include_str!("../tests/fixtures/motion/in_view_list.op");
const FORM_TRANSITIONS: &str = include_str!("../tests/fixtures/motion/form_transitions.op");

#[test]
#[ignore = "writes golden vectors; run with OP_MOTION_GOLDEN_OUT=<dir> -- --ignored"]
fn tests_motion_golden_generate() {
    let out = std::env::var("OP_MOTION_GOLDEN_OUT")
        .expect("set OP_MOTION_GOLDEN_OUT to the output directory");
    let out = std::path::PathBuf::from(out);
    std::fs::create_dir_all(&out).expect("create output directory");
    for (name, value) in [
        ("curves.json", curves()),
        ("tracks.json", tracks()),
        ("sessions.json", sessions()),
        ("transitions.json", transitions()),
    ] {
        let text = serde_json::to_string_pretty(&value).expect("serialize golden vectors");
        std::fs::write(out.join(name), text + "\n").expect("write golden vectors");
    }
}

#[test]
fn tests_motion_golden_fixtures_load_and_animate() {
    for (name, source) in fixtures() {
        let session = enter(name, source.text, source.canvas);
        assert!(
            session.active_animation_track_count() > 0 || !source.events.is_empty(),
            "{name}: fixture should animate at mount or through its events"
        );
    }
}

fn envelope(kind: &str) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    map.insert("kind".into(), json!(kind));
    map.insert("source".into(), json!(SOURCE_BASELINE));
    map
}

// ---------------------------------------------------------------------------
// curves.json — `ease()` for every schema easing.

fn easings() -> Vec<SchemaEasing> {
    vec![
        SchemaEasing::Linear,
        SchemaEasing::Ease,
        SchemaEasing::EaseIn,
        SchemaEasing::EaseOut,
        SchemaEasing::EaseInOut,
        SchemaEasing::Standard,
        SchemaEasing::Emphasized,
        SchemaEasing::EmphasizedDecelerate,
        SchemaEasing::EmphasizedAccelerate,
        SchemaEasing::CubicBezier(0.25, 0.1, 0.25, 1.0),
        // y overshoots 1: the runtime clamps the curve output to [0, 1].
        SchemaEasing::CubicBezier(0.34, 1.56, 0.64, 1.0),
        // y undershoots 0.
        SchemaEasing::CubicBezier(0.6, -0.28, 0.735, 0.045),
    ]
}

fn progress_at(index: usize) -> f32 {
    index as f32 / (CURVE_SAMPLES - 1) as f32
}

fn curves() -> Value {
    let curves: Vec<Value> = easings()
        .iter()
        .map(|easing| {
            let runtime = crate::motion::easing_from_schema(easing);
            let values: Vec<f32> = (0..CURVE_SAMPLES)
                .map(|index| golden_access::ease(runtime, progress_at(index)))
                .collect();
            json!({ "easing": easing, "values": values })
        })
        .collect();
    let mut map = envelope("curves");
    map.insert(
        "progress".into(),
        json!("values[i] = ease(i as f32 / 1000.0), f32 arithmetic"),
    );
    map.insert("samples".into(), json!(CURVE_SAMPLES));
    map.insert("curves".into(), Value::Array(curves));
    Value::Object(map)
}

// ---------------------------------------------------------------------------
// tracks.json — `sample_track()` over a set of requests.

struct TrackCase {
    name: &'static str,
    property: &'static str,
    from: Value,
    to: Value,
    stops: Option<Vec<(f32, Value)>>,
    duration_ms: u64,
    delay_ms: u64,
    easing: SchemaEasing,
    iterations: u32,
    direction: AnimationDirection,
    fill_mode: AnimationFillMode,
}

impl TrackCase {
    fn stops(name: &'static str, property: &'static str, stops: Vec<(f32, Value)>) -> Self {
        let from = stops
            .first()
            .map(|stop| stop.1.clone())
            .unwrap_or(Value::Null);
        let to = stops
            .last()
            .map(|stop| stop.1.clone())
            .unwrap_or(Value::Null);
        Self {
            name,
            property,
            from,
            to,
            stops: Some(stops),
            duration_ms: 400,
            delay_ms: 0,
            easing: SchemaEasing::Linear,
            iterations: 1,
            direction: AnimationDirection::Normal,
            fill_mode: AnimationFillMode::Forwards,
        }
    }

    fn duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    fn delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    fn easing(mut self, easing: SchemaEasing) -> Self {
        self.easing = easing;
        self
    }

    fn iterations(mut self, iterations: u32, direction: AnimationDirection) -> Self {
        self.iterations = iterations;
        self.direction = direction;
        self
    }

    fn fill(mut self, fill_mode: AnimationFillMode) -> Self {
        self.fill_mode = fill_mode;
        self
    }

    fn request(&self) -> AnimationRequest {
        let descriptor = animatable_property_registry()
            .get(self.property)
            .expect("registered animation property");
        AnimationRequest {
            target: "node".into(),
            property: AnimationProperty::from_registered(self.property, descriptor.apply),
            from: Some(self.from.clone()),
            to: self.to.clone(),
            stops: self.stops.clone(),
            duration_ms: self.duration_ms,
            delay_ms: self.delay_ms,
            easing: crate::motion::easing_from_schema(&self.easing),
            iterations: self.iterations,
            direction: self.direction,
            fill_mode: self.fill_mode,
            requested_at_ms: 0,
        }
    }
}

fn number_stops(values: &[(f32, f64)]) -> Vec<(f32, Value)> {
    values
        .iter()
        .map(|(offset, value)| (*offset, json!(value)))
        .collect()
}

fn track_cases() -> Vec<TrackCase> {
    use AnimationDirection::{Alternate, AlternateReverse, Normal, Reverse};
    use AnimationFillMode::{Backwards, Both, Forwards, None as NoFill};
    let two = || number_stops(&[(0.0, 0.0), (1.0, 100.0)]);
    let three = || number_stops(&[(0.0, 0.0), (0.5, 100.0), (1.0, 40.0)]);
    let mut cases = vec![TrackCase::stops("two_stop_linear", "translateY", two())];
    for (index, easing) in easings().into_iter().enumerate() {
        let name: &'static str = Box::leak(format!("three_stop_easing_{index}").into_boxed_str());
        cases.push(TrackCase::stops(name, "translateX", three()).easing(easing));
    }
    cases.extend([
        TrackCase::stops(
            "five_stop_ease_in_out",
            "opacity",
            number_stops(&[(0.0, 0.0), (0.25, 1.0), (0.5, 0.2), (0.75, 0.8), (1.0, 0.5)]),
        )
        .duration(800)
        .easing(SchemaEasing::EaseInOut),
        TrackCase::stops(
            "color_fill_ease",
            "fill",
            vec![(0.0, json!("#ff0000")), (1.0, json!("#0000ff"))],
        )
        .duration(300)
        .easing(SchemaEasing::Ease),
        TrackCase::stops(
            "color_alpha_three_stop",
            "fill",
            vec![
                (0.0, json!("#ff000080")),
                (0.5, json!("#00ff00ff")),
                (1.0, json!("#0000ff00")),
            ],
        )
        .duration(300)
        .easing(SchemaEasing::Standard),
        TrackCase::stops(
            "rotation_degrees",
            "rotation",
            number_stops(&[(0.0, -8.0), (1.0, 0.0)]),
        )
        .easing(SchemaEasing::EmphasizedDecelerate),
        TrackCase::stops(
            "corner_radius",
            "cornerRadius",
            number_stops(&[(0.0, 4.0), (1.0, 24.0)]),
        ),
        TrackCase::stops(
            "scale_overshoot_stops",
            "scaleX",
            number_stops(&[(0.0, 0.8), (0.6, 1.05), (1.0, 1.0)]),
        )
        .easing(SchemaEasing::Standard),
        TrackCase::stops("iterations_3_normal", "translateY", two())
            .duration(200)
            .easing(SchemaEasing::EaseIn)
            .iterations(3, Normal),
        TrackCase::stops("iterations_3_reverse", "translateY", two())
            .duration(200)
            .easing(SchemaEasing::EaseIn)
            .iterations(3, Reverse),
        TrackCase::stops("iterations_3_alternate", "translateY", two())
            .duration(200)
            .easing(SchemaEasing::EaseIn)
            .iterations(3, Alternate),
        TrackCase::stops("iterations_3_alternate_reverse", "translateY", two())
            .duration(200)
            .easing(SchemaEasing::EaseIn)
            .iterations(3, AlternateReverse),
        TrackCase::stops(
            "fill_none_delay",
            "opacity",
            number_stops(&[(0.0, 0.0), (1.0, 1.0)]),
        )
        .delay(100)
        .fill(NoFill),
        TrackCase::stops(
            "fill_forwards_delay",
            "opacity",
            number_stops(&[(0.0, 0.0), (1.0, 1.0)]),
        )
        .delay(100)
        .fill(Forwards),
        TrackCase::stops(
            "fill_backwards_delay",
            "opacity",
            number_stops(&[(0.0, 0.0), (1.0, 1.0)]),
        )
        .delay(100)
        .fill(Backwards),
        TrackCase::stops(
            "fill_both_delay",
            "opacity",
            number_stops(&[(0.0, 0.0), (1.0, 1.0)]),
        )
        .delay(100)
        .fill(Both),
        TrackCase::stops(
            "duration_zero_forwards",
            "opacity",
            number_stops(&[(0.0, 0.0), (1.0, 1.0)]),
        )
        .duration(0),
        TrackCase::stops(
            "duration_zero_none",
            "opacity",
            number_stops(&[(0.0, 0.0), (1.0, 1.0)]),
        )
        .duration(0)
        .fill(NoFill),
    ]);
    // A from/to request without stops (the `animate` action shape).
    let mut from_to = TrackCase::stops("from_to_without_stops", "translateX", two())
        .duration(200)
        .easing(SchemaEasing::Standard);
    from_to.stops = None;
    from_to.from = json!(10.0);
    from_to.to = json!(90.0);
    cases.push(from_to);
    cases
}

fn direction_name(direction: AnimationDirection) -> &'static str {
    match direction {
        AnimationDirection::Normal => "normal",
        AnimationDirection::Reverse => "reverse",
        AnimationDirection::Alternate => "alternate",
        AnimationDirection::AlternateReverse => "alternateReverse",
    }
}

fn fill_name(fill_mode: AnimationFillMode) -> &'static str {
    match fill_mode {
        AnimationFillMode::None => "none",
        AnimationFillMode::Forwards => "forwards",
        AnimationFillMode::Backwards => "backwards",
        AnimationFillMode::Both => "both",
    }
}

fn tracks() -> Value {
    let cases: Vec<Value> = track_cases()
        .iter()
        .map(|case| {
            let request = case.request();
            let end = case.delay_ms + case.duration_ms * u64::from(case.iterations);
            let samples: Vec<Value> = (0..=end + 48)
                .step_by(TRACK_STEP_MS as usize)
                .map(|now_ms| {
                    let (value, finished) = golden_access::sample(&request, now_ms);
                    json!({ "t": now_ms, "value": value, "finished": finished })
                })
                .collect();
            json!({
                "name": case.name,
                "request": {
                    "property": case.property,
                    "from": case.from,
                    "to": case.to,
                    "stops": case.stops.as_ref().map(|stops| stops
                        .iter()
                        .map(|(offset, value)| json!([offset, value]))
                        .collect::<Vec<_>>()),
                    "durationMs": case.duration_ms,
                    "delayMs": case.delay_ms,
                    "easing": case.easing,
                    "iterations": case.iterations,
                    "direction": direction_name(case.direction),
                    "fillMode": fill_name(case.fill_mode),
                    "requestedAtMs": 0,
                },
                "samples": samples,
            })
        })
        .collect();
    let mut map = envelope("tracks");
    map.insert(
        "sampling".into(),
        json!("value/finished = sample_track(request, t) for t in 0..=end+48 step 8; value null = no override"),
    );
    map.insert("cases".into(), Value::Array(cases));
    Value::Object(map)
}

// ---------------------------------------------------------------------------
// sessions.json — whole preview sessions driven on a fixed clock.

#[derive(Clone, Copy)]
enum EventKind {
    /// Scroll the phone frame's visible window (in-view triggers).
    DeviceScroll {
        scroll_y: f32,
    },
    Hover {
        x: f32,
        y: f32,
    },
    Tap {
        x: f32,
        y: f32,
    },
}

#[derive(Clone, Copy)]
struct SessionEvent {
    at_ms: u64,
    kind: EventKind,
}

struct Fixture {
    text: &'static str,
    canvas: (f32, f32),
    /// Present the root in a phone frame and feed its visible window to
    /// in-view triggers, the way the native and web hosts do.
    phone_window: bool,
    events: Vec<SessionEvent>,
}

fn fixtures() -> Vec<(&'static str, Fixture)> {
    let scroll = |at_ms, scroll_y| SessionEvent {
        at_ms,
        kind: EventKind::DeviceScroll { scroll_y },
    };
    let hover = |at_ms, x, y| SessionEvent {
        at_ms,
        kind: EventKind::Hover { x, y },
    };
    vec![
        (
            "stagger_entrance",
            Fixture {
                text: STAGGER_ENTRANCE,
                canvas: (400.0, 800.0),
                phone_window: false,
                events: Vec::new(),
            },
        ),
        (
            "in_view_list",
            Fixture {
                text: IN_VIEW_LIST,
                canvas: (400.0, 400.0),
                phone_window: true,
                events: vec![scroll(208, 300.0), scroll(800, 0.0), scroll(1200, 700.0)],
            },
        ),
        (
            "form_transitions",
            Fixture {
                text: FORM_TRANSITIONS,
                canvas: (400.0, 400.0),
                phone_window: false,
                events: vec![
                    hover(96, 20.0, 20.0),
                    hover(192, 300.0, 300.0),
                    SessionEvent {
                        at_ms: 496,
                        kind: EventKind::Tap { x: 30.0, y: 116.0 },
                    },
                    hover(800, 20.0, 220.0),
                    hover(1104, 300.0, 300.0),
                ],
            },
        ),
    ]
}

fn enter(name: &str, text: &str, canvas: (f32, f32)) -> PreviewSession {
    let document = jian_ops_schema::load_str(text)
        .unwrap_or_else(|error| panic!("{name}: fixture must load: {error}"))
        .value;
    let mut session = PreviewSession::enter(
        &document,
        canvas,
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
        0,
    )
    .unwrap_or_else(|error| panic!("{name}: preview session: {error:?}"));
    session.begin_lifecycle(0);
    session
}

/// The phone frame a host would build around the session's framed root.
fn phone_frame(session: &PreviewSession) -> DeviceFrame {
    let (_, root) = session.framed_root().expect("framed root");
    let canvas = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, 1000.0),
    };
    compute_frame_geometry(PreviewDeviceKind::Phone, canvas, root, None, None)
}

fn apply_event(session: &mut PreviewSession, event: &SessionEvent, now_ms: u64) {
    match event.kind {
        EventKind::DeviceScroll { scroll_y } => {
            let frame = phone_frame(session);
            let _ = session.sync_in_view_viewport(Some(&frame), scroll_y);
        }
        EventKind::Hover { x, y } => {
            let _ = session.dispatch_pointer_for_id_at(
                1,
                PointerKind::Mouse,
                x,
                y,
                PointerPhase::Hover,
                now_ms,
            );
        }
        EventKind::Tap { x, y } => {
            for phase in [PointerPhase::Down, PointerPhase::Up] {
                let _ =
                    session.dispatch_pointer_for_id_at(2, PointerKind::Touch, x, y, phase, now_ms);
            }
        }
    }
}

fn event_json(event: &SessionEvent) -> Value {
    match event.kind {
        EventKind::DeviceScroll { scroll_y } => {
            json!({ "atMs": event.at_ms, "kind": "deviceScroll", "scrollY": scroll_y })
        }
        EventKind::Hover { x, y } => {
            json!({ "atMs": event.at_ms, "kind": "hover", "x": x, "y": y })
        }
        EventKind::Tap { x, y } => json!({ "atMs": event.at_ms, "kind": "tap", "x": x, "y": y }),
    }
}

fn color_json(color: op_editor_ui::Color) -> Value {
    json!([color.r, color.g, color.b, color.a])
}

fn node_json(node: &SceneNode, out: &mut serde_json::Map<String, Value>) {
    if !node.id.is_empty() {
        out.insert(
            node.id.clone(),
            json!({
                "x": node.bounds.origin.x,
                "y": node.bounds.origin.y,
                "w": node.bounds.size.x,
                "h": node.bounds.size.y,
                "opacity": node.opacity,
                "fill": node.fill.map(color_json),
                "stroke": node.stroke.as_ref().map(|stroke| color_json(stroke.color)),
                "cornerRadius": node.corner_radius,
                "rotation": node.rotation,
                "hidden": node.hidden,
                "toggleProgress": node.widget.as_ref().and_then(|widget| widget.toggle_progress),
            }),
        );
    }
    for child in &node.children {
        node_json(child, out);
    }
}

fn nodes_json(scene: &LayoutScene) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(page) = scene.active_page() {
        for node in &page.children {
            node_json(node, &mut out);
        }
    }
    Value::Object(out)
}

fn overrides_json(session: &PreviewSession) -> Value {
    let overrides: Vec<Value> = golden_access::overrides(&session.animation)
        .into_iter()
        .map(|(target, property, value)| {
            json!({ "target": target, "property": property, "value": value })
        })
        .collect();
    Value::Array(overrides)
}

fn run_session(name: &str, fixture: &Fixture) -> Value {
    let mut session = enter(name, fixture.text, fixture.canvas);
    let window = fixture.phone_window.then(|| {
        let frame = phone_frame(&session);
        let _ = session.sync_in_view_viewport(Some(&frame), 0.0);
        json!({ "device": "phone", "topInset": 0.0, "height": frame.viewport_h, "pinned": [] })
    });
    let mut steps = Vec::new();
    let mut now_ms = 0;
    while now_ms <= SESSION_END_MS {
        let _ = session.pump(now_ms);
        for event in fixture.events.iter().filter(|event| event.at_ms == now_ms) {
            apply_event(&mut session, event, now_ms);
        }
        let scene = session.preview_scene_for_test();
        steps.push(json!({
            "t": now_ms,
            "trackCount": session.active_animation_track_count(),
            "nextDeadlineMs": session.animation.next_deadline_ms(),
            "overrides": overrides_json(&session),
            "nodes": nodes_json(&scene),
        }));
        now_ms += SESSION_STEP_MS;
    }
    let document: Value = serde_json::from_str(fixture.text).expect("fixture JSON");
    json!({
        "name": name,
        "canvas": [fixture.canvas.0, fixture.canvas.1],
        "document": document,
        "inViewWindow": window,
        "events": fixture.events.iter().map(event_json).collect::<Vec<_>>(),
        "steps": steps,
    })
}

fn sessions() -> Value {
    let sessions: Vec<Value> = fixtures()
        .iter()
        .map(|(name, fixture)| run_session(name, fixture))
        .collect();
    let mut map = envelope("sessions");
    map.insert(
        "protocol".into(),
        json!("PreviewSession::enter(doc, canvas, {}, 0, false, false, test_measure(), 0); begin_lifecycle(0); when inViewWindow is set, sync_in_view_viewport(phone frame, 0) (deviceScroll events re-sync it with scrollY); then for t in 0..=2000 step 16: pump(t); apply the events scheduled at t in order; read preview_scene_for_test(); record trackCount, nextDeadlineMs, overrides (target, property, value) and the overlaid scene nodes"),
    );
    map.insert("sessions".into(), Value::Array(sessions));
    Value::Object(map)
}

// ---------------------------------------------------------------------------
// transitions.json — screen push / pop / replace timing.

fn empty_page() -> ScenePage {
    ScenePage {
        id: "p".into(),
        name: "P".into(),
        children: Vec::new(),
    }
}

fn transitions() -> Value {
    let kinds = [
        (TransitionKind::Push, "push", PUSH_POP_DURATION_MS),
        (TransitionKind::Pop, "pop", PUSH_POP_DURATION_MS),
        (TransitionKind::Replace, "replace", REPLACE_DURATION_MS),
    ];
    let transitions: Vec<Value> = kinds
        .iter()
        .map(|(kind, name, duration_ms)| {
            let transition = ScreenTransition::start(*kind, empty_page(), TRANSITION_START_MS);
            let samples: Vec<Value> = (TRANSITION_START_MS
                ..=TRANSITION_START_MS + duration_ms + 16)
                .step_by(TRANSITION_STEP_MS as usize)
                .map(|now_ms| {
                    let (linear, eased) = transition.progress_for_golden(now_ms);
                    json!({
                        "t": now_ms - TRANSITION_START_MS,
                        "linear": linear,
                        "eased": eased,
                        "active": transition.is_active(now_ms),
                        "nextDeadlineMs": transition
                            .next_deadline_ms(now_ms)
                            .map(|deadline| deadline - TRANSITION_START_MS),
                    })
                })
                .collect();
            json!({ "kind": name, "durationMs": duration_ms, "samples": samples })
        })
        .collect();
    let mut map = envelope("transitions");
    map.insert(
        "layers".into(),
        json!({
            "push": "outgoing static below; incoming above at dx = width * (1 - eased)",
            "pop": "incoming static below; outgoing above at dx = width * eased",
            "replace": "outgoing alpha = 1 - linear below; incoming alpha = linear above",
        }),
    );
    map.insert("transitions".into(), Value::Array(transitions));
    Value::Object(map)
}

//! Preview widget-state paint coverage.

#![cfg(test)]

use super::{test_measure, PreviewSession};
use jian_core::gesture::pointer::PointerPhase;
use op_editor_ui::{Color, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
struct WidgetPaintRecorder {
    round_fills: Vec<(Rect, Color)>,
}

impl RenderBackend for WidgetPaintRecorder {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn scale(&mut self, _: Point2D, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, _: f32, color: Color) {
        self.round_fills.push((rect, color));
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
    fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
    fn measure_text_weighted(&mut self, _: &str, _: f32, _: u16) -> f32 {
        0.0
    }
}

fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn state_doc() -> jian_ops_schema::PenDocument {
    let source = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "states",
        "app": { "name": "states", "version": "1", "id": "states" },
        "children": [{
            "type": "frame", "id": "screen", "width": 240, "height": 180,
            "children": [
                {
                    "type": "switch", "id": "switch", "x": 20, "y": 20,
                    "width": 44, "height": 24, "checked": true,
                    "fill": [{ "type": "solid", "color": "#102030" }],
                    "states": {
                        "hover": { "fill": [{ "type": "solid", "color": "#204060" }] },
                        "pressed": { "fill": [{ "type": "solid", "color": "#306090" }] },
                        "focused": { "fill": [{ "type": "solid", "color": "#4080C0" }] },
                        "disabled": { "fill": [{ "type": "solid", "color": "#506070" }] }
                    }
                },
                {
                    "type": "switch", "id": "disabled-switch", "x": 20, "y": 70,
                    "width": 44, "height": 24, "checked": true, "enabled": false,
                    "fill": [{ "type": "solid", "color": "#102030" }],
                    "states": {
                        "hover": { "fill": [{ "type": "solid", "color": "#204060" }] },
                        "pressed": { "fill": [{ "type": "solid", "color": "#306090" }] },
                        "disabled": { "fill": [{ "type": "solid", "color": "#506070" }] }
                    }
                }
            ]
        }]
    }"##;
    jian_ops_schema::load_str(source)
        .expect("state fixture parses")
        .value
}

fn no_state_doc() -> jian_ops_schema::PenDocument {
    let source = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "derived",
        "app": { "name": "derived", "version": "1", "id": "derived" },
        "children": [{
            "type": "switch", "id": "switch", "width": 44, "height": 24,
            "checked": true, "fill": [{ "type": "solid", "color": "#101010" }]
        }]
    }"##;
    jian_ops_schema::load_str(source)
        .expect("derived fixture parses")
        .value
}

fn session_for(doc: &jian_ops_schema::PenDocument) -> PreviewSession {
    PreviewSession::enter(
        doc,
        (400.0, 240.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
        0,
    )
    .expect("enter preview")
}

fn paint_switch_fills(session: &mut PreviewSession) -> Vec<(Rect, Color)> {
    let mut recorder = WidgetPaintRecorder::default();
    session.paint_scene(
        &mut recorder,
        Rect::xywh(0.0, 0.0, 400.0, 240.0),
        (0.0, 0.0),
        1.0,
        0,
    );
    recorder.round_fills
}

fn switch_track_at(session: &mut PreviewSession, y: f32) -> Color {
    switch_paint_at(session, y).track
}

struct SwitchPaint {
    track: Color,
    knob_rect: Rect,
    knob: Color,
}

fn switch_paint_at(session: &mut PreviewSession, y: f32) -> SwitchPaint {
    let fills = paint_switch_fills(session);
    let idx = fills
        .iter()
        .position(|(rect, _)| (rect.origin.y - y).abs() < 0.1)
        .expect("switch track should paint");
    let track = fills[idx].1;
    let (knob_rect, knob) = fills
        .get(idx + 1)
        .copied()
        .expect("switch knob should paint");
    SwitchPaint {
        track,
        knob_rect,
        knob,
    }
}

fn tap_switch(session: &mut PreviewSession, id: &str) {
    let (x, y, w, h) = session.node_rect(id).expect("switch runtime rect");
    assert!(
        session.dispatch_tap(x + w / 2.0, y + h / 2.0),
        "tap on the switch should be handled"
    );
    // Leave the pointer so hover/pressed overlays do not tint the track.
    session.dispatch_pointer_phase(200.0, 200.0, PointerPhase::Hover);
}

fn color_channel_between(mid: f32, a: f32, b: f32) -> bool {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    mid + 1e-4 >= lo && mid - 1e-4 <= hi
}

fn color_strictly_between(mid: Color, a: Color, b: Color) -> bool {
    color_channel_between(mid.r, a.r, b.r)
        && color_channel_between(mid.g, a.g, b.g)
        && color_channel_between(mid.b, a.b, b.b)
        && color_channel_between(mid.a, a.a, b.a)
        && mid != a
        && mid != b
}

fn toggle_doc(transition: Option<&str>, reduced: bool) -> jian_ops_schema::PenDocument {
    let motion = if reduced {
        r#""motion": "reduced","#
    } else {
        ""
    };
    let transition = transition.unwrap_or("");
    let source = format!(
        r##"{{
        "version": "1.1",
        "formatVersion": "1.1",
        {motion}
        "id": "toggle",
        "app": {{ "name": "toggle", "version": "1", "id": "toggle" }},
        "children": [{{
            "type": "switch", "id": "switch", "width": 44, "height": 24,
            "checked": false,
            "fill": [{{ "type": "solid", "color": "#102030" }}],
            "stroke": {{ "thickness": 1, "fill": [{{ "type": "solid", "color": "#aabbcc" }}] }}
            {transition}
        }}]
    }}"##
    );
    jian_ops_schema::load_str(&source)
        .expect("toggle fixture parses")
        .value
}

fn live_session_for(doc: &jian_ops_schema::PenDocument) -> PreviewSession {
    let mut session = session_for(doc);
    session.begin_lifecycle(0);
    session
}

#[test]
fn authored_widget_states_paint_with_priority_and_leave_document_unchanged() {
    let doc = state_doc();
    let before = serde_json::to_string(&doc).expect("serialize before");
    let mut session = session_for(&doc);
    session.set_now_ms(0);

    assert_eq!(
        switch_track_at(&mut session, 20.0),
        Color::rgba_u8(16, 32, 48, 1.0)
    );

    session.dispatch_pointer_for_id_at(
        1,
        jian_core::gesture::pointer::PointerKind::Mouse,
        40.0,
        32.0,
        PointerPhase::Hover,
        1,
    );
    assert_eq!(session.interaction().hovered_node(), Some("switch"));
    assert_eq!(
        switch_track_at(&mut session, 20.0),
        Color::rgba_u8(32, 64, 96, 1.0)
    );

    session.dispatch_pointer_for_id_at(
        2,
        jian_core::gesture::pointer::PointerKind::Mouse,
        40.0,
        32.0,
        PointerPhase::Down,
        2,
    );
    assert_eq!(
        switch_track_at(&mut session, 20.0),
        Color::rgba_u8(48, 96, 144, 1.0)
    );

    session.focus_next();
    assert_eq!(
        switch_track_at(&mut session, 20.0),
        Color::rgba_u8(48, 96, 144, 1.0)
    );

    session.dispatch_pointer_for_id_at(
        3,
        jian_core::gesture::pointer::PointerKind::Mouse,
        40.0,
        82.0,
        PointerPhase::Hover,
        3,
    );
    session.dispatch_pointer_for_id_at(
        4,
        jian_core::gesture::pointer::PointerKind::Mouse,
        40.0,
        82.0,
        PointerPhase::Down,
        4,
    );
    let disabled_track = switch_track_at(&mut session, 70.0);
    assert_eq!(
        disabled_track,
        Color::rgba_u8(80, 96, 112, 1.0),
        "disabled must win over pressed and hover"
    );

    assert_eq!(
        before,
        serde_json::to_string(&doc).expect("serialize after"),
        "preview state paint must not mutate the document"
    );
}

#[test]
fn theme_derived_overlay_paints_when_state_override_is_absent() {
    let doc = no_state_doc();
    let mut session = session_for(&doc);
    let base = switch_track_at(&mut session, 0.0);

    session.dispatch_pointer_for_id_at(
        1,
        jian_core::gesture::pointer::PointerKind::Mouse,
        22.0,
        12.0,
        PointerPhase::Hover,
        1,
    );
    let hovered = switch_track_at(&mut session, 0.0);
    let theme = jian_core::render::widget_style::WidgetTheme::default();
    let overlay = theme.hover_overlay;
    let overlay_r = f32::from(overlay.r()) / 255.0;
    let overlay_g = f32::from(overlay.g()) / 255.0;
    let overlay_b = f32::from(overlay.b()) / 255.0;
    let overlay_a = f32::from(overlay.a()) / 255.0;
    let expected = Color {
        r: base.r * (1.0 - overlay_a) + overlay_r * overlay_a,
        g: base.g * (1.0 - overlay_a) + overlay_g * overlay_a,
        b: base.b * (1.0 - overlay_a) + overlay_b * overlay_a,
        a: 1.0,
    };
    assert!(
        (hovered.r - expected.r).abs() < 0.005
            && (hovered.g - expected.g).abs() < 0.005
            && (hovered.b - expected.b).abs() < 0.005
            && (hovered.a - expected.a).abs() < 0.005,
        "hover should use the theme overlay: expected {expected:?}, got {hovered:?}"
    );
}

#[test]
fn focused_widget_state_paints_when_focus_enters_the_widget() {
    let doc = state_doc();
    let mut session = session_for(&doc);
    session.focus_next();

    assert_eq!(
        switch_track_at(&mut session, 20.0),
        Color::rgba_u8(64, 128, 192, 1.0),
        "focused must resolve when no higher-priority state is active"
    );
}

#[test]
fn hover_state_change_requests_an_immediate_repaint() {
    let doc = state_doc();
    let mut session = session_for(&doc);
    let mut event = jian_core::gesture::PointerEvent::simple_at(
        1,
        PointerPhase::Hover,
        jian_core::geometry::point(40.0, 32.0),
        1,
    );
    event.kind = jian_core::gesture::pointer::PointerKind::Mouse;
    let changed = session.dispatch_input(super::PreviewInputEnvelope::new(
        super::PreviewInput::Pointer(event),
    ));
    assert!(changed.needs_redraw, "hover enter must request a repaint");

    let mut repeat = jian_core::gesture::PointerEvent::simple_at(
        1,
        PointerPhase::Hover,
        jian_core::geometry::point(40.0, 32.0),
        2,
    );
    repeat.kind = jian_core::gesture::pointer::PointerKind::Mouse;
    let unchanged = session.dispatch_input(super::PreviewInputEnvelope::new(
        super::PreviewInput::Pointer(repeat),
    ));
    assert!(
        !unchanged.needs_redraw,
        "a stationary hover must not request extra work"
    );
}

#[test]
fn switch_toggle_tweens_knob_and_track_over_short3() {
    let doc = toggle_doc(None, false);
    let mut session = live_session_for(&doc);
    let off = switch_paint_at(&mut session, 0.0);
    let off_x = off.knob_rect.origin.x;

    tap_switch(&mut session, "switch");
    let at_tap = switch_paint_at(&mut session, 0.0);
    assert!(
        (at_tap.knob_rect.origin.x - off_x).abs() < 1.0,
        "knob must still be near the off x at t=0, got {} (off={off_x})",
        at_tap.knob_rect.origin.x
    );

    session.pump(75);
    let mid = switch_paint_at(&mut session, 0.0);
    session.pump(150);
    let on = switch_paint_at(&mut session, 0.0);
    let on_x = on.knob_rect.origin.x;

    assert!(
        mid.knob_rect.origin.x > off_x && mid.knob_rect.origin.x < on_x,
        "knob x at 75ms must sit between off ({off_x}) and on ({on_x}), got {}",
        mid.knob_rect.origin.x
    );
    assert!(
        color_strictly_between(mid.track, off.track, on.track),
        "track colour at 75ms must sit between inactive {:?} and active {:?}, got {:?}",
        off.track,
        on.track,
        mid.track
    );
    assert!(
        color_strictly_between(mid.knob, off.knob, on.knob),
        "knob colour at 75ms must sit between ends"
    );
    assert!(
        (on.knob_rect.origin.x - (off_x + 20.0)).abs() < 0.5,
        "on-look knob should sit 20px right of off, off={off_x} on={}",
        on.knob_rect.origin.x
    );
    assert_eq!(on.track, Color::rgba_u8(16, 32, 48, 1.0));
    assert_eq!(off.track, Color::rgba_u8(170, 187, 204, 1.0));
}

#[test]
fn switch_toggle_uses_node_transition_duration() {
    let doc = toggle_doc(
        Some(r#","transition": { "durationMs": 400, "easing": "linear" }"#),
        false,
    );
    let mut session = live_session_for(&doc);
    let off = switch_paint_at(&mut session, 0.0);
    tap_switch(&mut session, "switch");
    let _ = switch_paint_at(&mut session, 0.0);
    session.pump(150);
    let mid = switch_paint_at(&mut session, 0.0);
    session.pump(400);
    let on = switch_paint_at(&mut session, 0.0);
    assert!(
        mid.knob_rect.origin.x > off.knob_rect.origin.x
            && mid.knob_rect.origin.x < on.knob_rect.origin.x,
        "400ms transition must still be mid-tween at 150ms, off={} mid={} on={}",
        off.knob_rect.origin.x,
        mid.knob_rect.origin.x,
        on.knob_rect.origin.x
    );
    assert!(
        color_strictly_between(mid.track, off.track, on.track),
        "track must still be between ends at 150ms of a 400ms tween"
    );
}

#[test]
fn switch_toggle_is_instant_when_motion_is_reduced() {
    let doc = toggle_doc(None, true);
    let mut session = live_session_for(&doc);
    tap_switch(&mut session, "switch");
    let at_tap = switch_paint_at(&mut session, 0.0);
    assert_eq!(
        at_tap.track,
        Color::rgba_u8(16, 32, 48, 1.0),
        "reduced motion must paint the on look at t=0 after the tap"
    );
    let pad = 2.0_f32;
    let d = (24.0_f32 - pad * 2.0).max(2.0);
    let on_x = 44.0 - d - pad;
    assert!(
        (at_tap.knob_rect.origin.x - on_x).abs() < 0.01,
        "reduced-motion knob must sit at the on x {on_x}, got {}",
        at_tap.knob_rect.origin.x
    );
    assert_eq!(session.active_animation_track_count(), 0);
}

#[test]
fn switch_toggle_retargets_from_sampled_progress() {
    let doc = toggle_doc(None, false);
    let mut session = live_session_for(&doc);
    let off = switch_paint_at(&mut session, 0.0);
    tap_switch(&mut session, "switch");
    let _ = switch_paint_at(&mut session, 0.0);
    session.pump(75);
    let mid = switch_paint_at(&mut session, 0.0);
    tap_switch(&mut session, "switch");
    let retargeted = switch_paint_at(&mut session, 0.0);
    assert!(
        (retargeted.knob_rect.origin.x - mid.knob_rect.origin.x).abs() < 1.0,
        "retarget must start from the sampled knob x {}, jumped to {}",
        mid.knob_rect.origin.x,
        retargeted.knob_rect.origin.x
    );
    session.pump(150);
    let returning = switch_paint_at(&mut session, 0.0);
    assert!(
        returning.knob_rect.origin.x < retargeted.knob_rect.origin.x,
        "after retarget the knob must travel back toward off, from {} to {}",
        retargeted.knob_rect.origin.x,
        returning.knob_rect.origin.x
    );
    session.pump(225);
    let back = switch_paint_at(&mut session, 0.0);
    assert!(
        (back.knob_rect.origin.x - off.knob_rect.origin.x).abs() < 0.01,
        "return tween should land on the off look, got {} expected {}",
        back.knob_rect.origin.x,
        off.knob_rect.origin.x
    );
}

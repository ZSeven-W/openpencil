use super::*;
use crate::widgets::motion_timeline_layout::{
    HEADER_H, KEYFRAME_R, LABEL_W, ROW_H, RULER_H, TRACK_PAD,
};
use crate::{Point2D, Rect};
use op_editor_core::EditorCommand;
use serde_json::json;

fn fade(delay: u64) -> serde_json::Value {
    json!({
        "trigger": "mount", "durationMs": 400, "delayMs": delay, "easing": "linear",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0}},
            {"offset": 1, "values": {"opacity": 1}}
        ]
    })
}

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "home", "name": "Home", "width": 390,
                "height": 844, "children": [
                {"type": "rectangle", "id": "a", "name": "Card", "width": 10, "height": 10,
                 "animations": [fade(0), fade(200)]},
                {"type": "rectangle", "id": "b", "width": 10, "height": 10,
                 "animations": [fade(100)]}
            ]}]
        })
        .to_string(),
    )
    .expect("document")
    .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.locale = op_editor_core::Locale::EnUs;
    state.toggle_motion_timeline();
    state
}

const PANEL: Rect = Rect::xywh(0.0, 600.0, 1000.0, 220.0);

#[test]
fn rows_name_their_node_and_number_its_strips() {
    let view = MotionTimelineView::for_editor(&state());
    let labels: Vec<&str> = view.rows.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, ["Card · 1", "Card · 2", "b"]);
    assert_eq!(view.scope_label.as_deref(), Some("Home"));
    assert_eq!(view.clip_ms, 600.0);
    assert_eq!(view.empty_message(), None);
    assert_eq!(view.tab_label(MotionTimelineTab::Mount), "On load");
}

#[test]
fn a_drag_previews_where_the_release_would_put_the_strip() {
    let mut state = state();
    state.begin_motion_timeline_drag(MotionTimelineDrag::Strip {
        node_id: "b".into(),
        animation: 0,
        delta_ms: 0.0,
    });
    state.update_motion_timeline_drag(300.0, 0);
    let view = MotionTimelineView::for_editor(&state);
    let b = &view.rows[2].strip;
    assert_eq!((b.start_ms, b.end_ms), (400.0, Some(800.0)));
    // The clip stretches to show it.
    assert_eq!(view.clip_ms, 800.0);
    // Nothing is written yet.
    assert_eq!(state.motion_timeline_projection().strips[2].start_ms, 100.0);

    state.cancel_motion_timeline_drag();
    state.begin_motion_timeline_drag(MotionTimelineDrag::StripEnd {
        node_id: "b".into(),
        animation: 0,
        end_ms: 500.0,
    });
    state.update_motion_timeline_drag(900.0, 0);
    let b = MotionTimelineView::for_editor(&state).rows[2].strip.clone();
    assert_eq!(b.keyframe_ms, [100.0, 900.0]);
}

#[test]
fn the_layout_maps_time_to_lanes_and_back() {
    let view = MotionTimelineView::for_editor(&state());
    let layout = TimelineLayout::new(PANEL, &view);
    let zero = PANEL.origin.x + LABEL_W + TRACK_PAD;
    assert_eq!(layout.x_for_ms(0.0), zero);
    assert_eq!(layout.x_for_ms(400.0), zero + 400.0 * view.zoom);
    assert_eq!(layout.ms_for_x(zero + 100.0), 100.0 / f64::from(view.zoom));
    assert_eq!(layout.ms_for_x(0.0), 0.0);
    assert_eq!(layout.rows.len(), 3);
    assert_eq!(
        layout.rows[1].lane.origin.y,
        PANEL.origin.y + HEADER_H + RULER_H + ROW_H
    );
}

#[test]
fn hits_resolve_controls_keyframes_handles_and_strips() {
    let view = MotionTimelineView::for_editor(&state());
    let layout = TimelineLayout::new(PANEL, &view);
    let center = |r: Rect| Point2D::new(r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0);
    assert_eq!(
        layout.hit(center(layout.play), &view),
        Some(TimelineHit::PlayPause)
    );
    assert_eq!(
        layout.hit(center(layout.tabs[1].1), &view),
        Some(TimelineHit::Tab(MotionTimelineTab::InView))
    );
    assert_eq!(
        layout.hit(center(layout.ruler), &view),
        Some(TimelineHit::Ruler)
    );
    let b = &layout.rows[2];
    assert_eq!(
        layout.hit(b.keyframes[0], &view),
        Some(TimelineHit::Keyframe(TimelineKeyframeRef {
            node_id: "b".into(),
            animation: 0,
            keyframe: 0,
        }))
    );
    // Away from the keyframes: the middle of the strip, then its end grip.
    let middle = Point2D::new(
        (b.keyframes[0].x + b.keyframes[1].x) / 2.0,
        b.keyframes[0].y,
    );
    assert_eq!(
        layout.hit(middle, &view),
        Some(TimelineHit::Strip {
            node_id: "b".into(),
            animation: 0
        })
    );
    // The grip sits just past the last keyframe's diamond.
    let end = center(b.end_handle);
    assert!(end.x > b.keyframes[1].x + KEYFRAME_R);
    assert_eq!(
        layout.hit(end, &view),
        Some(TimelineHit::StripEnd {
            node_id: "b".into(),
            animation: 0
        })
    );
    assert_eq!(
        layout.hit(
            Point2D::new(PANEL.origin.x + 900.0, b.keyframes[0].y),
            &view
        ),
        Some(TimelineHit::Background)
    );
    assert_eq!(layout.hit(Point2D::new(10.0, 10.0), &view), None);
}

#[test]
fn an_empty_tab_explains_itself() {
    let mut state = state();
    state.set_motion_timeline_tab(MotionTimelineTab::Manual, None);
    let view = MotionTimelineView::for_editor(&state);
    assert_eq!(view.empty_message(), Some("Choose a motion id"));
    state.set_motion_timeline_tab(MotionTimelineTab::InView, None);
    let view = MotionTimelineView::for_editor(&state);
    assert_eq!(view.empty_message(), Some("No motion on this screen"));
    // Selecting a node outside the scope's subtree does not matter here.
    let _ = state.apply(EditorCommand::SetSelection {
        node_id: op_editor_core::NodeId::new("a"),
    });
}

#[test]
fn ruler_labels_keep_only_the_decimals_they_need() {
    use crate::widgets::motion_timeline_paint::seconds_label;
    assert_eq!(seconds_label(0.0), "0 s");
    assert_eq!(seconds_label(250.0), "0.25 s");
    assert_eq!(seconds_label(500.0), "0.5 s");
    assert_eq!(seconds_label(2000.0), "2 s");
}

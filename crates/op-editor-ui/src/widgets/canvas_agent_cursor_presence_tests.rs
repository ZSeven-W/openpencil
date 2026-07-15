//! Regression tests for the Agent cursor's pre-node viewport presence.

use crate::layout_scene::{NodeKind, SceneNode};
use crate::widgets::canvas_agent_cursor::cursor_sprites_with_idle_anchor;
use crate::{Point2D, Rect};
use op_editor_core::agent_indicators::{AgentIndicators, AgentTag};

fn tag(color: &str, name: &str) -> AgentTag {
    AgentTag {
        color: color.to_string(),
        name: name.to_string(),
    }
}

fn scene() -> Vec<SceneNode> {
    let mut child = SceneNode::leaf("c1", NodeKind::Rect);
    child.bounds = Rect::xywh(10.0, 30.0, 60.0, 60.0);
    let mut frame = SceneNode::leaf("f1", NodeKind::Frame);
    frame.bounds = Rect::xywh(0.0, 20.0, 200.0, 300.0);
    frame.children = vec![child];
    vec![frame]
}

#[test]
fn confirmed_agent_parks_at_viewport_center_before_first_reveal() {
    let mut indicators = AgentIndicators::default();
    indicators.run_active = true;
    indicators.cursor_agent = Some(tag("#4ECDC4", "Mochi"));
    let center = Point2D::new(420.0, 310.0);

    assert!(
        cursor_sprites_with_idle_anchor(&[], &indicators, Point2D::ZERO, 1.0, 500, None,)
            .is_empty()
    );
    let sprites =
        cursor_sprites_with_idle_anchor(&[], &indicators, Point2D::ZERO, 1.0, 500, Some(center));

    assert_eq!(sprites.len(), 1);
    assert_eq!(sprites[0].name.as_deref(), Some("Mochi"));
    assert!((sprites[0].pos.x - center.x).abs() <= 3.0);
    assert!((sprites[0].pos.y - center.y).abs() <= 1.5);
    assert!(sprites[0].current_rect.is_none());

    indicators.cursor_agent = None;
    assert!(cursor_sprites_with_idle_anchor(
        &[],
        &indicators,
        Point2D::ZERO,
        1.0,
        500,
        Some(center),
    )
    .is_empty());
}

#[test]
fn first_reveal_flies_from_the_viewport_center_to_its_node() {
    let mut indicators = AgentIndicators::default();
    indicators.run_active = true;
    indicators.cursor_agent = Some(tag("#4ECDC4", "Mochi"));
    indicators.reveals.insert("c1".into(), 1_000);
    let center = Point2D::new(420.0, 310.0);
    let roots = scene();

    let parked =
        cursor_sprites_with_idle_anchor(&roots, &indicators, Point2D::ZERO, 1.0, 600, Some(center));
    assert!((parked[0].pos.x - center.x).abs() <= 3.0);
    assert!((parked[0].pos.y - center.y).abs() <= 1.5);

    let flying =
        cursor_sprites_with_idle_anchor(&roots, &indicators, Point2D::ZERO, 1.0, 800, Some(center));
    assert!(flying[0].pos.x > 40.0 && flying[0].pos.x < center.x);
    assert!(flying[0].pos.y > 60.0 && flying[0].pos.y < center.y);
    assert!(flying[0].current_rect.is_none());

    let arrived = cursor_sprites_with_idle_anchor(
        &roots,
        &indicators,
        Point2D::ZERO,
        1.0,
        1_000,
        Some(center),
    );
    assert!((arrived[0].pos.x - 40.0).abs() < 0.01);
    assert!((arrived[0].pos.y - 60.0).abs() < 0.01);
    assert!(arrived[0].current_rect.is_some());
}

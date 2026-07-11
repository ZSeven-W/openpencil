use super::canvas_generation_scan::{generating_paint_sets, is_placeholder_section, scan_phase};
use crate::layout_scene::{NodeKind, SceneNode};
use op_editor_core::agent_indicators::{AgentIndicators, AgentTag};

#[test]
fn scan_phase_wraps_and_advances_monotonically() {
    let start = scan_phase(0, 1_200);
    assert_eq!(start, scan_phase(1_200, 1_200));
    assert_eq!(start, 0.0);

    let samples = [0, 200, 600, 1_199].map(|now_ms| scan_phase(now_ms, 1_200));
    assert!(samples.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(samples.iter().all(|phase| (0.0..=1.0).contains(phase)));
}

#[test]
fn placeholder_section_requires_an_empty_scene_frame() {
    let empty_frame = SceneNode::leaf("empty", NodeKind::Frame);
    assert!(is_placeholder_section(&empty_frame));

    let mut populated_frame = SceneNode::leaf("populated", NodeKind::Frame);
    populated_frame
        .children
        .push(SceneNode::leaf("content", NodeKind::Rect));
    assert!(!is_placeholder_section(&populated_frame));
}

#[test]
fn non_frame_leaf_is_not_a_placeholder_section() {
    let empty_rect = SceneNode::leaf("rect", NodeKind::Rect);
    assert!(!is_placeholder_section(&empty_rect));
}

#[test]
fn generating_descendants_exclude_the_claimed_root_and_skip_idle_allocation() {
    let mut nested = SceneNode::leaf("section", NodeKind::Frame);
    nested
        .children
        .push(SceneNode::leaf("content", NodeKind::Rect));
    let mut root = SceneNode::leaf("root", NodeKind::Frame);
    root.children.push(nested);

    let mut indicators = AgentIndicators::default();
    assert!(generating_paint_sets(&[root.clone()], Some(&indicators)).is_none());

    indicators.run_active = true;
    indicators.frames.insert(
        "root".into(),
        AgentTag {
            color: "#4ECDC4".into(),
            name: "Mochi".into(),
        },
    );
    let ids = generating_paint_sets(&[root], Some(&indicators))
        .unwrap()
        .scan;
    assert!(!ids.contains("root"));
    assert!(ids.contains("section"));
    assert!(ids.contains("content"));
}

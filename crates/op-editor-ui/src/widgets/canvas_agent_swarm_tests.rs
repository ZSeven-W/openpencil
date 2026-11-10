use super::*;
use crate::layout_scene::{NodeKind, SceneNode};
use op_editor_core::agent_indicators::{AgentIndicators, AgentTag};

fn tag(color: &str, name: &str) -> AgentTag {
    AgentTag {
        color: color.to_string(),
        name: name.to_string(),
    }
}

fn live(name: &str) -> AgentIndicators {
    let mut indicators = AgentIndicators::default();
    indicators.run_active = true;
    indicators.cursor_agent = Some(tag("#123456", name));
    indicators
}

fn viewport() -> Rect {
    Rect::xywh(0.0, 0.0, 1200.0, 800.0)
}

/// A 400x800 board with one landed section at the top.
fn board() -> Vec<SceneNode> {
    let mut section = SceneNode::leaf("hero", NodeKind::Rect);
    section.bounds = Rect::xywh(100.0, 50.0, 400.0, 200.0);
    let mut root = SceneNode::leaf("root", NodeKind::Frame);
    root.bounds = Rect::xywh(100.0, 50.0, 400.0, 800.0);
    root.children = vec![section];
    vec![root]
}

fn frame_at(roots: &[SceneNode], indicators: &AgentIndicators, now_ms: u64) -> SwarmFrame {
    swarm_frame(
        true,
        roots,
        indicators,
        Point2D::ZERO,
        1.0,
        viewport(),
        None,
        now_ms,
    )
}

#[test]
fn off_switch_idle_run_and_unconfirmed_agent_show_nothing() {
    let roots = board();
    let on = live("Kiki");
    let off = swarm_frame(
        false,
        &roots,
        &on,
        Point2D::ZERO,
        1.0,
        viewport(),
        None,
        500,
    );
    assert_eq!(off, SwarmFrame::default());

    let mut idle = live("Kiki");
    idle.run_active = false;
    assert_eq!(frame_at(&roots, &idle, 500), SwarmFrame::default());

    let mut anonymous = live("Kiki");
    anonymous.cursor_agent = None;
    assert_eq!(frame_at(&roots, &anonymous, 500), SwarmFrame::default());
}

#[test]
fn decoys_step_aside_for_real_concurrent_agents() {
    let roots = board();
    let mut team = live("Kiki");
    team.frames.insert("root".into(), tag("#111111", "Kiki"));
    assert_eq!(frame_at(&roots, &team, 500).cursors.len(), DECOYS);
    team.frames.insert("other".into(), tag("#222222", "Fern"));
    assert_eq!(frame_at(&roots, &team, 500), SwarmFrame::default());
}

#[test]
fn decoys_carry_distinct_names_and_never_the_real_agents() {
    let frame = frame_at(&board(), &live("nova"), 500);
    let names: Vec<_> = frame.cursors.iter().map(|c| c.name).collect();
    assert_eq!(names.len(), DECOYS);
    assert!(!names.contains(&"Nova"), "{names:?}");
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(unique.len(), DECOYS);
}

#[test]
fn before_any_board_exists_decoys_roam_the_viewport_without_skeleton() {
    let frame = frame_at(&[], &live("Kiki"), 4_000);
    let stage = frame.stage.expect("stand-in stage");
    assert!(frame.skeleton.is_empty());
    assert_eq!(frame.cursors.len(), DECOYS);
    for cursor in &frame.cursors {
        assert!(
            stage.contains(cursor.pos),
            "{:?} outside {stage:?}",
            cursor.pos
        );
    }
}

#[test]
fn starter_ghost_stands_in_for_the_board_and_gets_a_skeleton() {
    let frame = swarm_frame(
        true,
        &[],
        &live("Kiki"),
        Point2D::new(10.0, 20.0),
        0.5,
        viewport(),
        Some([0.0, 0.0, 800.0, 1200.0]),
        0,
    );
    assert_eq!(frame.stage, Some(Rect::xywh(10.0, 20.0, 400.0, 600.0)));
    assert!(!frame.skeleton.is_empty());
}

#[test]
fn skeleton_fills_only_the_band_below_landed_content() {
    let roots = board();
    let mut indicators = live("Kiki");
    indicators.reveals.insert("hero".into(), 1_000);

    // Before the section's reveal starts the whole board is still empty.
    // Unclaimed and nothing revealed yet: no board to work on.
    let early = frame_at(&roots, &indicators, 500).skeleton;
    assert!(
        early.is_empty(),
        "a board with no started reveal is not chosen"
    );

    indicators
        .frames
        .insert("root".into(), tag("#111111", "Kiki"));
    let before = swarm_frame(
        true,
        &roots,
        &indicators,
        Point2D::ZERO,
        1.0,
        viewport(),
        None,
        500,
    );
    let after = frame_at(&roots, &indicators, 1_500);
    assert!(before.skeleton.len() > after.skeleton.len());
    let content_bottom = 250.0;
    assert!(!after.skeleton.is_empty());
    for block in &after.skeleton {
        assert!(block.origin.y >= content_bottom, "{block:?}");
        assert!(block.origin.y + block.size.y <= 850.0, "{block:?}");
    }
}

#[test]
fn full_board_drops_the_skeleton_but_keeps_the_decoys() {
    let mut roots = board();
    roots[0].children[0].bounds = Rect::xywh(100.0, 50.0, 400.0, 790.0);
    let mut indicators = live("Kiki");
    indicators
        .frames
        .insert("root".into(), tag("#111111", "Kiki"));
    let frame = frame_at(&roots, &indicators, 500);
    assert!(frame.skeleton.is_empty());
    assert_eq!(frame.cursors.len(), DECOYS);
}

#[test]
fn most_recently_revealed_board_is_the_stage() {
    let mut roots = board();
    let mut second = roots[0].clone();
    second.id = "root2".into();
    second.bounds = Rect::xywh(600.0, 50.0, 400.0, 800.0);
    second.children[0].id = "hero2".into();
    second.children[0].bounds = Rect::xywh(600.0, 50.0, 400.0, 200.0);
    roots.push(second);
    let mut indicators = live("Kiki");
    indicators.reveals.insert("hero".into(), 100);
    indicators.reveals.insert("hero2".into(), 900);
    assert_eq!(
        frame_at(&roots, &indicators, 500).stage,
        Some(Rect::xywh(100.0, 50.0, 400.0, 800.0)),
        "a reveal that has not started yet does not move the stage"
    );
    assert_eq!(
        frame_at(&roots, &indicators, 1_000).stage,
        Some(Rect::xywh(600.0, 50.0, 400.0, 800.0))
    );
}

#[test]
fn decoy_motion_is_deterministic_continuous_and_stays_on_stage() {
    let roots = board();
    let mut indicators = live("Kiki");
    indicators
        .frames
        .insert("root".into(), tag("#111111", "Kiki"));
    let stage = Rect::xywh(100.0, 50.0, 400.0, 800.0);
    let mut previous = frame_at(&roots, &indicators, 0);
    assert_eq!(previous, frame_at(&roots, &indicators, 0));
    let mut travelled = 0.0f32;
    for step in 1..=600u64 {
        let frame = frame_at(&roots, &indicators, step * 16);
        for (now, before) in frame.cursors.iter().zip(&previous.cursors) {
            let (dx, dy) = (now.pos.x - before.pos.x, now.pos.y - before.pos.y);
            let moved = (dx * dx + dy * dy).sqrt();
            assert!(moved < 120.0, "{} jumped {moved}px in one frame", now.name);
            assert!(stage.contains(now.pos), "{:?} left the board", now.pos);
            travelled += moved;
        }
        previous = frame;
    }
    assert!(travelled > 1_000.0, "decoys barely moved: {travelled}px");
}

#[test]
fn wireframe_layout_stays_inside_the_stage_and_is_capped() {
    for stage in [
        Rect::xywh(0.0, 0.0, 390.0, 844.0),
        Rect::xywh(50.0, 50.0, 1440.0, 9_000.0),
        Rect::xywh(0.0, 0.0, 40.0, 30.0),
    ] {
        let blocks = layout_blocks(stage);
        assert!(blocks.len() <= MAX_BLOCKS + 3);
        for block in &blocks {
            assert!(block.origin.x >= stage.origin.x);
            assert!(block.origin.y >= stage.origin.y);
            assert!(block.origin.x + block.size.x <= stage.origin.x + stage.size.x + 0.01);
            assert!(block.origin.y + block.size.y <= stage.origin.y + stage.size.y + 0.01);
        }
    }
}

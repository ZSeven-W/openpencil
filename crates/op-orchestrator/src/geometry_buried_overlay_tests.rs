//! `collect_buried_overlay_fixes` — the 星图 starfield repair.
//!
//! Fixture is `0808-k3-2.op`'s 星空视窗容器 reduced to the stack that fails:
//! an opaque 297px circle at `children[0]` (topmost) with three floating
//! controls authored after it.

use super::*;
use serde_json::json;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn starfield() -> (Value, HashMap<String, Rect>) {
    let stack = json!({
        "type":"frame","id":"stack","name":"星空视窗容器","layout":"none",
        "width":327,"height":304,"clipContent":false,
        "children":[
            {"type":"frame","id":"circle","name":"圆形星空视窗","layout":"none",
             "x":15,"y":0,"width":297,"height":297,"clipContent":true,"cornerRadius":149,
             "fill":[{"type":"radial_gradient","stops":[
                 {"offset":0.0,"color":"#3B1B6E"},{"offset":1.0,"color":"$--background"}]}],
             "children":[]},
            {"type":"frame","id":"compass","name":"方位标签","layout":"horizontal",
             "x":96,"y":14,"cornerRadius":14,
             "fill":[{"type":"solid","color":"#1A0D2EE6"}],
             "children":[{"type":"text","id":"compasst","content":"北 · 天顶"}]},
            {"type":"frame","id":"gyro","name":"陀螺仪切换","layout":"horizontal",
             "x":24,"y":240,"cornerRadius":16,
             "fill":[{"type":"solid","color":"$--primary"}],
             "children":[
                {"type":"icon_font","id":"gyroi","iconFontName":"rotate-3d","width":14,"height":14},
                {"type":"text","id":"gyrot","content":"陀螺仪 开"}
             ]}
        ]
    });
    // Resolved rects as the real engine produced them (probe output, offset to
    // a local origin for readability).
    let rects = HashMap::from([
        ("stack".to_string(), rect(0.0, 0.0, 327.0, 304.0)),
        ("circle".to_string(), rect(15.0, 0.0, 297.0, 297.0)),
        ("compass".to_string(), rect(96.0, 14.0, 126.0, 29.0)),
        ("gyro".to_string(), rect(24.0, 240.0, 95.0, 33.0)),
    ]);
    (stack, rects)
}

fn moved_ids(cmds: &[EditorCommand]) -> Vec<String> {
    cmds.iter()
        .filter_map(|cmd| match cmd {
            EditorCommand::MoveNode {
                node_id,
                target_parent,
                index,
                ..
            } => {
                assert_eq!(target_parent.as_str(), "stack");
                assert_eq!(*index, Some(0));
                Some(node_id.as_str().to_string())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn controls_buried_under_the_starfield_circle_are_rescued() {
    let (stack, rects) = starfield();
    let mut cmds = Vec::new();
    collect_buried_overlay_fixes(&stack, &rects, &mut cmds);
    // Back-to-front walk, so re-inserting each at 0 leaves the original
    // relative order (compass before gyro) intact.
    assert_eq!(moved_ids(&cmds), vec!["gyro", "compass"]);
}

#[test]
fn a_deck_back_layer_is_left_where_it_is() {
    // The DELIBERATE version of "a later sibling is covered": a decorative,
    // EMPTY back card peeking behind an opaque front card. The corpus forbids
    // content in those layers, and that is the line this pass reads.
    let deck = json!({
        "type":"frame","id":"deck","layout":"none","width":311,"height":180,
        "children":[
            {"type":"frame","id":"front","x":0,"y":0,"width":311,"height":170,
             "cornerRadius":16,"fill":[{"type":"solid","color":"$--card"}],
             "children":[{"type":"text","id":"frontt","content":"Front"}]},
            {"type":"frame","id":"back","x":10,"y":10,"width":311,"height":170,
             "cornerRadius":16,"fill":[{"type":"solid","color":"$--muted"}],
             "children":[]}
        ]
    });
    let rects = HashMap::from([
        ("deck".to_string(), rect(0.0, 0.0, 311.0, 180.0)),
        ("front".to_string(), rect(0.0, 0.0, 311.0, 170.0)),
        ("back".to_string(), rect(10.0, 10.0, 311.0, 170.0)),
    ]);
    let mut cmds = Vec::new();
    collect_buried_overlay_fixes(&deck, &rects, &mut cmds);
    assert!(cmds.is_empty(), "{cmds:?}");
}

#[test]
fn a_deck_back_layer_with_text_on_it_is_still_a_deck() {
    // `0724-1-gm-2`'s word-card deck, verbatim from
    // `loop_finalize_tests::loop_finalize_restores_nested_front_card_surface_
    // without_reordering_stack`: the back card carries an example sentence, so
    // "bears content" is true of BOTH layers and cannot separate them. Their
    // AREAS can — 317x148 behind 345x148 is a peer, not an overlay.
    let deck = json!({
        "type":"frame","id":"stack","layout":"none","width":345,"height":168,
        "children":[
            {"type":"frame","id":"front","x":0,"y":18,"width":345,"height":148,
             "cornerRadius":20,"fill":[{"type":"solid","color":"$--card"}],
             "children":[{"type":"text","id":"word","content":"Resilient"}]},
            {"type":"frame","id":"back","role":"card","x":14,"y":0,"width":317,"height":148,
             "cornerRadius":18,"fill":[{"type":"solid","color":"$--accent"}],
             "children":[{"type":"text","id":"backt","content":"Example"}]}
        ]
    });
    let rects = HashMap::from([
        ("stack".to_string(), rect(0.0, 0.0, 345.0, 168.0)),
        ("front".to_string(), rect(0.0, 18.0, 345.0, 148.0)),
        ("back".to_string(), rect(14.0, 0.0, 317.0, 148.0)),
    ]);
    let mut cmds = Vec::new();
    collect_buried_overlay_fixes(&deck, &rects, &mut cmds);
    assert!(
        cmds.is_empty(),
        "a peer-sized layer is a stack composition, not a buried overlay: {cmds:?}"
    );
}

#[test]
fn a_ring_stack_is_left_alone() {
    // Track + progress arc + centred label: the arcs carry no content, and the
    // label is already `children[0]`. Nothing to rescue.
    let ring = json!({
        "type":"frame","id":"ring","layout":"none","width":80,"height":80,
        "children":[
            {"type":"frame","id":"label","x":0,"y":0,"width":80,"height":80,
             "children":[{"type":"text","id":"labelt","content":"72%"}]},
            {"type":"ellipse","id":"progress","x":0,"y":0,"width":80,"height":80,
             "innerRadius":0.8,"fill":[{"type":"solid","color":"$--primary"}]},
            {"type":"ellipse","id":"track","x":0,"y":0,"width":80,"height":80,
             "innerRadius":0.8,"fill":[{"type":"solid","color":"$--muted"}]}
        ]
    });
    let rects = HashMap::from([
        ("ring".to_string(), rect(0.0, 0.0, 80.0, 80.0)),
        ("label".to_string(), rect(0.0, 0.0, 80.0, 80.0)),
        ("progress".to_string(), rect(0.0, 0.0, 80.0, 80.0)),
        ("track".to_string(), rect(0.0, 0.0, 80.0, 80.0)),
    ]);
    let mut cmds = Vec::new();
    collect_buried_overlay_fixes(&ring, &rects, &mut cmds);
    assert!(cmds.is_empty(), "{cmds:?}");
}

#[test]
fn a_translucent_scrim_does_not_count_as_burial() {
    // A scrim exists to be seen THROUGH. Content under an 0x55-alpha wash is
    // still readable, so moving it would reorder a deliberate composition.
    let hero = json!({
        "type":"frame","id":"hero","layout":"none","width":300,"height":200,
        "children":[
            {"type":"frame","id":"scrim","x":0,"y":0,"width":300,"height":200,
             "fill":[{"type":"solid","color":"#00000055"}],"children":[]},
            {"type":"frame","id":"caption","x":16,"y":150,"width":200,"height":30,
             "children":[{"type":"text","id":"capt","content":"Caption"}]}
        ]
    });
    let rects = HashMap::from([
        ("hero".to_string(), rect(0.0, 0.0, 300.0, 200.0)),
        ("scrim".to_string(), rect(0.0, 0.0, 300.0, 200.0)),
        ("caption".to_string(), rect(16.0, 150.0, 200.0, 30.0)),
    ]);
    let mut cmds = Vec::new();
    collect_buried_overlay_fixes(&hero, &rects, &mut cmds);
    assert!(cmds.is_empty(), "{cmds:?}");
}

#[test]
fn a_corner_badge_tucked_behind_an_edge_is_a_composition() {
    // Only ~25% of the badge is behind the card — a deliberate half-tuck, well
    // under `MIN_BURIED_FRACTION`.
    let card = json!({
        "type":"frame","id":"card","layout":"none","width":200,"height":120,
        "children":[
            {"type":"frame","id":"surface","x":0,"y":0,"width":200,"height":120,
             "fill":[{"type":"solid","color":"$--card"}],"children":[]},
            {"type":"frame","id":"badge","x":184,"y":-8,"width":32,"height":32,
             "children":[{"type":"text","id":"badget","content":"3"}]}
        ]
    });
    let rects = HashMap::from([
        ("card".to_string(), rect(0.0, 0.0, 200.0, 120.0)),
        ("surface".to_string(), rect(0.0, 0.0, 200.0, 120.0)),
        ("badge".to_string(), rect(184.0, -8.0, 32.0, 32.0)),
    ]);
    let mut cmds = Vec::new();
    collect_buried_overlay_fixes(&card, &rects, &mut cmds);
    assert!(cmds.is_empty(), "{cmds:?}");
}

#[test]
fn a_flex_stack_is_out_of_scope() {
    // Only `layout:none` paints in reverse; flow children never overlap by
    // index, so the whole rule is meaningless there.
    let (mut row, rects) = starfield();
    row["layout"] = json!("horizontal");
    let mut cmds = Vec::new();
    collect_buried_overlay_fixes(&row, &rects, &mut cmds);
    assert!(cmds.is_empty(), "{cmds:?}");
}

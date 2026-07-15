//! `EditorCommand::RefineDesign` tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{flex_frame, rect, state_with};
use crate::walkers::find_node;

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn refine_design_sanitizes_auto_layout_child_positions() {
    let mut s = state_with(vec![flex_frame(
        "root",
        "Root",
        0.0,
        0.0,
        375.0,
        100.0,
        vec![rect("child", "Child", 24.0, 32.0, 120.0, 40.0)],
    )]);

    assert!(s.apply(EditorCommand::RefineDesign {
        root_id: id("root"),
        canvas_width: Some(375),
        page_id: None,
    }));

    let child = find_node(s.active_children(), &id("child")).expect("child remains");
    assert_eq!(child.base().x, None);
    assert_eq!(child.base().y, None);
}

use jian_ops_schema::node::PenNode;

/// Parse a single node fixture by wrapping it in a canonical doc.
fn node_from(json: &str) -> PenNode {
    let doc = format!(r#"{{"version":"1.0.0","children":[{json}]}}"#);
    jian_ops_schema::load_str(&doc)
        .expect("fixture parses")
        .value
        .children
        .remove(0)
}

#[test]
fn refine_remints_duplicate_and_blank_ids() {
    let mut root = node_from(
        r##"{"type":"frame","id":"root","name":"R","width":400,"height":300,"children":[
            {"type":"rectangle","id":"dup","width":10,"height":10},
            {"type":"rectangle","id":"dup","width":10,"height":10},
            {"type":"text","id":"  ","content":"hi"}
        ]}"##,
    );
    let fixes = crate::command_refine::refine_subtree(&mut root);
    use crate::pen_node_ext::PenNodeExt;
    let children = root.children().expect("children");
    assert_eq!(children[0].base().id, "dup");
    assert_eq!(children[1].base().id, "dup-2", "duplicate gets -2 suffix");
    assert_eq!(
        children[2].base().id,
        "text-node",
        "blank id normalizes to {{type}}-node"
    );
    assert!(fixes.iter().any(|f| f.fix.contains("Reminted")));
}

#[test]
fn refine_clamps_children_into_screen_frame() {
    // 375x800 mobile screen, free layout, child way outside.
    let mut root = node_from(
        r##"{"type":"frame","id":"screen","name":"S","width":375,"height":800,"children":[
            {"type":"rectangle","id":"r","x":900,"y":-500,"width":100,"height":50}
        ]}"##,
    );
    let fixes = crate::command_refine::refine_subtree(&mut root);
    use crate::pen_node_ext::PenNodeExt;
    let child = &root.children().unwrap()[0];
    let x = child.base().x.unwrap();
    let y = child.base().y.unwrap();
    // max x = 375 - 100 + 37.5 = 312.5 ; min y = -80.
    assert!((x - 312.5).abs() < 0.01, "x clamped (got {x})");
    assert!((y + 80.0).abs() < 0.01, "y clamped (got {y})");
    assert!(fixes.iter().any(|f| f.fix.contains("Clamped")));
}

#[test]
fn refine_strips_emoji_from_text_content() {
    let mut root = node_from(
        r##"{"type":"frame","id":"f","name":"F","width":200,"height":100,"children":[
            {"type":"text","id":"t","content":"Hello 🚀  world ✨"}
        ]}"##,
    );
    let fixes = crate::command_refine::refine_subtree(&mut root);
    use crate::pen_node_ext::PenNodeExt;
    let PenNode::Text(text) = &root.children().unwrap()[0] else {
        panic!("text survives");
    };
    assert_eq!(
        text.content,
        jian_ops_schema::node::text::TextContent::Plain("Hello world".into())
    );
    assert!(fixes.iter().any(|f| f.fix.contains("emoji")));
}

//! Regression: a transparent wrapper must not re-add the rail's gutter.
//!
//! `strip_wrapper_double_inset` owns this contract, but it runs BEFORE the
//! mobile chrome / content-rail passes that establish the rail's own gutter.
//! A section that only becomes a padded rail during those later passes used
//! to keep an inner transparent wrapper's own horizontal padding, so the card
//! inside it rendered narrower than every sibling section on the same rail.
//! Measured on `0727-1-gm`: a Quick Add card at 279px against 327px siblings.

use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn plan() -> crate::plan::OrchestratorPlan {
    crate::plan::OrchestratorPlan {
        shared_context: None,
        root_frame: crate::plan::RootFrameSpec {
            id: "root".into(),
            name: "Habit Tracker".into(),
            width: 375.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
        palette: None,
    }
}

fn insert(sink: &mut VecDocSink, tree: Value) {
    let node: PenNode = serde_json::from_value(tree).expect("fixture json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
}

fn node_by_name<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a PenNode> {
    for n in nodes {
        if n.base().name.as_deref() == Some(name) {
            return Some(n);
        }
        if let Some(hit) = n.children().and_then(|c| node_by_name(c, name)) {
            return Some(hit);
        }
    }
    None
}

fn horizontal_padding(sink: &VecDocSink, name: &str) -> (f64, f64) {
    let node = node_by_name(sink.state.active_children(), name).expect("node survives cleanup");
    let props = match node {
        PenNode::Frame(n) => &n.container,
        _ => panic!("{name} is a frame"),
    };
    let sides = props
        .padding
        .as_ref()
        .map(padding_sides)
        .unwrap_or_default();
    (sides[1], sides[3])
}

/// A card surface — opaque, so it owns its inner padding legitimately.
fn card(id: &str) -> Value {
    json!({
        "type": "frame", "id": id, "name": id,
        "width": "fill_container", "height": "fit_content",
        "layout": "vertical", "padding": 20, "gap": 16, "cornerRadius": 20,
        "fill": [{"type": "solid", "color": "#FFFFFF"}],
        "children": [
            {"type": "text", "id": format!("{id}-t"), "content": "Quick Add",
             "width": "fill_container", "height": 24}
        ]
    })
}

/// A rail-width section that already carries the screen gutter, so the
/// resulting sibling widths are directly comparable.
fn padded_section(id: &str, name: &str) -> Value {
    json!({
        "type": "frame", "id": id, "name": name,
        "width": "fill_container", "height": "fit_content",
        "layout": "vertical", "padding": [0, 24], "gap": 16,
        "children": [card(&format!("{id}-card"))]
    })
}

fn mobile_screen_with_wrapped_sheet() -> Value {
    json!({
        "type": "frame", "id": "root", "name": "Habit Tracker",
        "width": 375, "height": 812, "layout": "vertical",
        "children": [
            {"type": "frame", "id": "status", "name": "Status Bar",
             "width": "fill_container", "height": 62, "layout": "none",
             "children": [
                 {"type": "text", "id": "clock", "content": "9:41",
                  "width": "fit_content", "height": 20}
             ]},
            padded_section("header", "Header & Daily Progress Summary"),
            padded_section("habits", "Today's Habits & Rituals"),
            // The defect: a transparent section with NO gutter of its own yet
            // (the rail pass gives it one later), holding a transparent
            // wrapper that already re-adds the same 24px gutter.
            {"type": "frame", "id": "content", "name": "App Content",
             "width": "fill_container", "height": "fit_content",
             "layout": "vertical",
             "children": [
                 {"type": "frame", "id": "sheet", "name": "Quick Add Sheet Container",
                  "width": "fill_container", "height": "fit_content",
                  "layout": "vertical", "padding": [16, 24, 0, 24],
                  "children": [card("quick-add")]},
                 // A second child: the sheet is no longer the section's only
                 // child, so the single-child gutter collapse cannot see it.
                 {"type": "frame", "id": "spacer", "name": "Tab Bar Spacer",
                  "width": "fill_container", "height": 72, "layout": "none",
                  "children": []}
             ]}
        ]
    })
}

#[test]
fn wrapper_inside_a_late_railed_section_loses_its_duplicate_gutter() {
    let mut sink = VecDocSink::new();
    insert(&mut sink, mobile_screen_with_wrapped_sheet());

    crate::cleanup::run_cleanup_passes(&mut sink, &plan(), &["root"]);

    let (section_r, section_l) = horizontal_padding(&sink, "App Content");
    assert!(
        section_r > 0.0 && section_l > 0.0,
        "the section owns the rail gutter after cleanup, got ({section_r}, {section_l})"
    );
    assert_eq!(
        horizontal_padding(&sink, "Quick Add Sheet Container"),
        (0.0, 0.0),
        "the transparent wrapper must not re-add the gutter its section already owns"
    );
}

/// Resolved width of the named node, via the same jian flex pass
/// `snapshot_layout` uses — the fact the defect is actually judged on. Looked
/// up by name because the cleanup root transforms re-key the whole subtree.
fn resolved_width(sink: &VecDocSink, name: &str) -> f64 {
    fn walk(nodes: &[jian_scene::layout_scene::SceneNode], id: &str) -> Option<f64> {
        for n in nodes {
            if n.id == id {
                return Some(f64::from(n.aggregate_bounds().size.x));
            }
            if let Some(hit) = walk(&n.children, id) {
                return Some(hit);
            }
        }
        None
    }
    let id = node_by_name(sink.state.active_children(), name)
        .expect("node survives cleanup")
        .id_str()
        .to_string();
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
    let page = scene.active_page().expect("active page");
    walk(&page.children, &id).expect("node has a resolved rect")
}

#[test]
fn wrapped_card_resolves_to_the_same_width_as_its_rail_siblings() {
    let mut sink = VecDocSink::new();
    insert(&mut sink, mobile_screen_with_wrapped_sheet());

    crate::cleanup::run_cleanup_passes(&mut sink, &plan(), &["root"]);

    let sibling = resolved_width(&sink, "header-card");
    let wrapped = resolved_width(&sink, "quick-add");
    assert_eq!(
        wrapped, sibling,
        "the wrapped card must sit on the same rail as its siblings \
         (was 279 against 327 before the post-rail re-run)"
    );
}

#[test]
fn opaque_card_keeps_its_own_inner_padding() {
    let mut sink = VecDocSink::new();
    insert(&mut sink, mobile_screen_with_wrapped_sheet());

    crate::cleanup::run_cleanup_passes(&mut sink, &plan(), &["root"]);

    assert_eq!(
        horizontal_padding(&sink, "quick-add"),
        (20.0, 20.0),
        "a filled, rounded card's padding is its own inset, never a rail gutter"
    );
}

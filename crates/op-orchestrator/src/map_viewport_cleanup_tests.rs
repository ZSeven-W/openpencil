use super::{plan, req};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::AbortFlag;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, PenNodeExt};

fn named<'a>(nodes: &'a [PenNode], name: &str) -> &'a PenNode {
    fn find<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a PenNode> {
        nodes.iter().find_map(|n| {
            if n.base().name.as_deref() == Some(name) {
                Some(n)
            } else {
                n.children().and_then(|children| find(children, name))
            }
        })
    }
    find(nodes, name).unwrap_or_else(|| panic!("missing {name}"))
}

#[test]
fn real_generated_navigation_dock_survives_complete_cleanup_as_a_floating_card() {
    let mut plan = plan();
    crate::plan_normalize::normalize(&mut plan, &req());
    let mut sink = VecDocSink::new();
    for cmd in crate::scaffold::build_scaffold(&plan, true).unwrap() {
        if let EditorCommand::InsertSubtree { nodes, .. } = cmd {
            sink.state.doc.children = nodes;
        }
    }
    // Exact GLM output: the model called its trip summary "bottom-nav-dock".
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        include_str!("test_fixtures/navigation-dock-generated.js").into(),
    )]);
    let outcome = futures::executor::block_on(crate::subagent::run_subtask(
        &plan.subtasks[0],
        &plan,
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    let before =
        serde_json::to_value(named(sink.state.active_children(), "bottom-nav-dock")).unwrap();
    assert_eq!(before["y"], 646.0);
    crate::cleanup::run_cleanup_passes(&mut sink, &plan, &["page"]);
    let nodes = sink.state.active_children();
    let dock = named(nodes, "bottom-nav-dock");
    let after = serde_json::to_value(dock).unwrap();
    assert_ne!(after["role"], "bottom-tab-bar");
    assert_eq!(after["layout"], "vertical");
    assert_eq!(after["x"], 16.0);
    assert_eq!(after["width"], 343.0);
    assert!(after["y"].as_f64().unwrap() > 500.0);
    let rects = crate::image_fallback_policy::resolved_rects(&sink.state);
    let bounds = rects[dock.id_str()];
    let viewport = nodes[0]
        .children()
        .unwrap()
        .iter()
        .find(|n| {
            n.base()
                .explain
                .as_deref()
                .is_some_and(|s| s.contains("op-navigation-viewport"))
        })
        .unwrap();
    let viewport_bounds = rects[viewport.id_str()];
    assert!(bounds.y + bounds.height <= viewport_bounds.y + viewport_bounds.height - 23.5);
    if let Ok(path) = std::env::var("OPENPENCIL_NAVIGATION_REPLAY_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&sink.state.doc).unwrap()).unwrap();
    }
}

use super::*;
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use serde_json::json;

fn sink_with_root(root: serde_json::Value) -> VecDocSink {
    let node: PenNode = serde_json::from_value(root).expect("valid root");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

fn find_by_name<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a PenNode> {
    for n in nodes {
        if n.base().name.as_deref() == Some(name) {
            return Some(n);
        }
        if let Some(children) = n.children() {
            if let Some(hit) = find_by_name(children, name) {
                return Some(hit);
            }
        }
    }
    None
}

fn child_names(node: &PenNode) -> Vec<String> {
    node.children()
        .into_iter()
        .flatten()
        .map(|c| c.base().name.clone().unwrap_or_default())
        .collect()
}

/// The de-identified `0718-1-glm-1.op` shape: a section wrapper whose only
/// child is a "header row" — title text on one side, the ENTIRE checklist
/// misnested as the row's other flex child, the checklist redundantly
/// repeating the title as its own first child.
fn duplicate_title_section() -> serde_json::Value {
    json!({
        "type": "frame", "id": "wrapper", "name": "SectionWrapper",
        "width": "fill_container", "height": "fit_content", "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "header-row", "name": "HeaderRow",
                "width": "fill_container", "height": "fit_content",
                "layout": "horizontal", "justifyContent": "space_between",
                "children": [
                    { "type": "text", "id": "title", "name": "Title", "content": "Must-See",
                      "width": "fit_content", "height": "fit_content" },
                    {
                        "type": "frame", "id": "checklist", "name": "ChecklistSection",
                        "width": "fill_container", "height": "fit_content", "layout": "vertical",
                        "children": [
                            { "type": "text", "id": "dup-title", "name": "DupTitle",
                              "content": "Must-See", "width": "fit_content", "height": "fit_content" },
                            { "type": "text", "id": "item-1", "name": "Item1", "content": "Fushimi Inari",
                              "width": "fit_content", "height": "fit_content" },
                            { "type": "text", "id": "item-2", "name": "Item2", "content": "Arashiyama",
                              "width": "fit_content", "height": "fit_content" }
                        ]
                    }
                ]
            }
        ]
    })
}

#[test]
fn duplicate_title_header_row_gets_repaired() {
    let mut sink = sink_with_root(duplicate_title_section());
    let wrapper_id = find_by_name(sink.state.active_children(), "SectionWrapper")
        .unwrap()
        .id_str()
        .to_string();

    let repaired = repair_section_shell_fill_ownership(&mut sink, &wrapper_id);
    assert_eq!(repaired, 1);

    let wrapper = find_by_name(sink.state.active_children(), "SectionWrapper").unwrap();
    assert_eq!(
        child_names(wrapper),
        vec!["HeaderRow", "ChecklistSection"],
        "checklist promoted to be the header row's sibling, right after it"
    );

    let header_row = find_by_name(sink.state.active_children(), "HeaderRow").unwrap();
    assert_eq!(
        child_names(header_row),
        vec!["Title"],
        "header row keeps only its own title"
    );

    let checklist = find_by_name(sink.state.active_children(), "ChecklistSection").unwrap();
    assert_eq!(
        child_names(checklist),
        vec!["Item1", "Item2"],
        "the duplicate title inside the checklist is gone, real items untouched"
    );
    assert!(
        find_by_name(sink.state.active_children(), "DupTitle").is_none(),
        "the duplicate title node itself must be deleted, not just unlinked"
    );
}

#[test]
fn distinct_titles_are_left_untouched() {
    // Same shape, but the checklist's first child is a DIFFERENT string —
    // a legitimate "title, then a container whose first item happens to be
    // text" pattern, not the duplicate-title bug.
    let mut root = duplicate_title_section();
    root["children"][0]["children"][1]["children"][0]["content"] = json!("On track");
    let mut sink = sink_with_root(root);
    let wrapper_id = find_by_name(sink.state.active_children(), "SectionWrapper")
        .unwrap()
        .id_str()
        .to_string();

    let repaired = repair_section_shell_fill_ownership(&mut sink, &wrapper_id);
    assert_eq!(repaired, 0);
    let header_row = find_by_name(sink.state.active_children(), "HeaderRow").unwrap();
    assert_eq!(child_names(header_row), vec!["Title", "ChecklistSection"]);
}

#[test]
fn vertical_row_with_the_same_shape_is_untouched() {
    // The bug is specifically about a HORIZONTAL row misnesting a vertical
    // content body as a flex sibling — the same duplicate-text shape inside
    // a vertical stack is an ordinary (if odd) authored structure, not this
    // failure mode.
    let mut root = duplicate_title_section();
    root["children"][0]["layout"] = json!("vertical");
    let mut sink = sink_with_root(root);
    let wrapper_id = find_by_name(sink.state.active_children(), "SectionWrapper")
        .unwrap()
        .id_str()
        .to_string();

    let repaired = repair_section_shell_fill_ownership(&mut sink, &wrapper_id);
    assert_eq!(repaired, 0);
}

#[test]
fn three_child_row_is_untouched() {
    // Scoped tight to the observed 2-child shape — a row with a third
    // child (e.g. a trailing chevron icon) is not this failure mode.
    let mut root = duplicate_title_section();
    root["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(
            json!({ "type": "icon_font", "id": "chevron", "name": "Chevron",
                       "iconFontName": "chevron-right", "width": 16, "height": 16 }),
        );
    let mut sink = sink_with_root(root);
    let wrapper_id = find_by_name(sink.state.active_children(), "SectionWrapper")
        .unwrap()
        .id_str()
        .to_string();

    let repaired = repair_section_shell_fill_ownership(&mut sink, &wrapper_id);
    assert_eq!(repaired, 0);
}

#[test]
fn repair_is_idempotent() {
    let mut sink = sink_with_root(duplicate_title_section());
    let wrapper_id = find_by_name(sink.state.active_children(), "SectionWrapper")
        .unwrap()
        .id_str()
        .to_string();

    assert_eq!(
        repair_section_shell_fill_ownership(&mut sink, &wrapper_id),
        1
    );
    assert_eq!(
        repair_section_shell_fill_ownership(&mut sink, &wrapper_id),
        0,
        "nothing left to fix on a second run"
    );
}

/// End-to-end: the repair is wired into `run_cleanup_passes` and its
/// output survives the rest of the shared cleanup pipeline (no later pass
/// re-collapses the promoted checklist back into the header row).
#[test]
fn wired_into_run_cleanup_passes_end_to_end() {
    let mut sink = sink_with_root(duplicate_title_section());
    let root_ids: Vec<String> = sink
        .state
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();
    let root_id_refs: Vec<&str> = root_ids.iter().map(String::as_str).collect();
    let plan = crate::plan::OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
        palette: None,
    };
    crate::cleanup::run_cleanup_passes(&mut sink, &plan, &root_id_refs);

    let value = serde_json::to_value(sink.state.active_children()).expect("serialize");
    let title_count = value.to_string().matches("\"Must-See\"").count();
    assert_eq!(
        title_count, 1,
        "exactly one \"Must-See\" title must survive the full cleanup pipeline: {value}"
    );
    let checklist = find_by_name(sink.state.active_children(), "ChecklistSection").unwrap();
    assert!(
        child_names(checklist).contains(&"Item1".to_string()),
        "checklist content survives cleanup: {value}"
    );
}

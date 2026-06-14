use super::register_new_node_reveals;
use std::collections::HashSet;

#[test]
fn reveal_schedule_streams_large_subtrees_without_long_tail() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let ids_before = HashSet::from(["root".to_string()]);
    let mut state = op_editor_core::EditorState::new();
    let children: Vec<serde_json::Value> = (0..20)
        .map(|i| {
            serde_json::json!({
                "type": "text",
                "id": format!("label-{i}"),
                "content": format!("Item {i}"),
                "fontSize": 14
            })
        })
        .collect();
    state.doc.children = vec![serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "frame",
            "id": "section",
            "name": "Section",
            "width": "fill_container",
            "height": "fit_content",
            "children": children
        }]
    }))
    .expect("fixture parses")];

    register_new_node_reveals(&ids_before, &state, Some(epoch), 1_000);

    let snapshot = op_editor_core::agent_indicators::snapshot_at(1_000);
    let section = *snapshot.reveals.get("section").expect("section reveal");
    let last_label = *snapshot.reveals.get("label-19").expect("last label reveal");
    assert!(
        last_label - section >= 300,
        "large nested subtrees should stay visibly streamed instead of arriving in one burst"
    );
    assert!(
        last_label - section <= 760,
        "large nested subtrees should avoid a slow reveal queue"
    );
    assert!(
        snapshot.reveals.values().all(|start| *start < 2_000),
        "new content should finish entering within a responsive window"
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn reveal_schedule_keeps_nested_stream_order_across_sibling_groups() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let ids_before = HashSet::from(["root".to_string()]);
    let mut state = op_editor_core::EditorState::new();
    state.doc.children = vec![serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "frame",
            "id": "row-0",
            "name": "Row 0",
            "children": [{
                "type": "text",
                "id": "label-0",
                "content": "A"
            }]
        }, {
            "type": "frame",
            "id": "row-1",
            "name": "Row 1",
            "children": [{
                "type": "text",
                "id": "label-1",
                "content": "B"
            }]
        }]
    }))
    .expect("fixture parses")];

    register_new_node_reveals(&ids_before, &state, Some(epoch), 1_000);

    let snapshot = op_editor_core::agent_indicators::snapshot_at(1_000);
    let row_0 = *snapshot.reveals.get("row-0").expect("row 0 reveal");
    let label_0 = *snapshot.reveals.get("label-0").expect("label 0 reveal");
    let row_1 = *snapshot.reveals.get("row-1").expect("row 1 reveal");
    let label_1 = *snapshot.reveals.get("label-1").expect("label 1 reveal");

    assert!(
        row_0 < label_0 && label_0 < row_1 && row_1 < label_1,
        "nested reveals should follow visual stream order instead of sharing sibling-local slots"
    );
    assert!(
        [label_0 - row_0, row_1 - label_0, label_1 - row_1]
            .into_iter()
            .all(|gap| gap >= 16 && gap <= op_editor_core::agent_indicators::REVEAL_CHILD_RUNWAY_MS),
        "nested stream items should keep frame cadence while giving new containers a runway"
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn reveal_schedule_gives_new_container_children_an_entrance_runway() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let ids_before = HashSet::from(["root".to_string()]);
    let mut state = op_editor_core::EditorState::new();
    state.doc.children = vec![serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "frame",
            "id": "status-bar",
            "name": "Status Bar",
            "children": [{
                "type": "text",
                "id": "time",
                "content": "9:41"
            }, {
                "type": "frame",
                "id": "levels",
                "name": "Levels"
            }, {
                "type": "frame",
                "id": "battery",
                "name": "Battery"
            }]
        }]
    }))
    .expect("fixture parses")];

    register_new_node_reveals(&ids_before, &state, Some(epoch), 1_000);

    let snapshot = op_editor_core::agent_indicators::snapshot_at(1_000);
    let status = *snapshot
        .reveals
        .get("status-bar")
        .expect("status bar reveal");
    let starts = ["time", "levels", "battery"].map(|id| {
        *snapshot
            .reveals
            .get(id)
            .unwrap_or_else(|| panic!("{id} reveal"))
    });

    assert!(
        starts[0] - status >= 80,
        "children of a new container need their own runway instead of starting inside the parent's first beat"
    );
    assert!(
        starts.windows(2).all(|pair| pair[1] - pair[0] >= 16),
        "container children should continue one-per-frame after the runway"
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn reveal_schedule_does_not_spend_stream_slots_on_structure_only_containers() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let ids_before = HashSet::from(["root".to_string()]);
    let mut state = op_editor_core::EditorState::new();
    state.doc.children = vec![serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "group",
            "id": "layout-shell-0",
            "children": [{
                "type": "group",
                "id": "layout-shell-1",
                "children": [{
                    "type": "group",
                    "id": "layout-shell-2",
                    "children": [{
                        "type": "group",
                        "id": "layout-shell-3",
                        "children": [{
                            "type": "group",
                            "id": "layout-shell-4",
                            "children": [{
                                "type": "text",
                                "id": "headline",
                                "content": "Hello"
                            }]
                        }]
                    }]
                }]
            }]
        }]
    }))
    .expect("fixture parses")];

    register_new_node_reveals(&ids_before, &state, Some(epoch), 1_000);

    let snapshot = op_editor_core::agent_indicators::snapshot_at(1_000);
    let headline = *snapshot.reveals.get("headline").expect("headline reveal");
    assert!(
        headline <= 1_120,
        "structure-only wrappers should not delay the first visible generated node"
    );
    assert!(
        snapshot
            .reveals
            .keys()
            .all(|id| !id.starts_with("layout-shell")),
        "layout-only wrappers should not get their own reveal slots"
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

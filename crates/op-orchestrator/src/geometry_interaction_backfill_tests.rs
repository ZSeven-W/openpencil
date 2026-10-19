use super::*;

use crate::test_support::VecDocSink;
use jian_ops_schema::PenDocument;
use op_editor_core::EditorState;
use serde_json::{json, Value};

fn sink_from_value(value: Value) -> VecDocSink {
    let document: PenDocument = serde_json::from_value(value).expect("valid document");
    VecDocSink {
        state: EditorState::from_document(document),
        applied: Vec::new(),
        batch_depth: 0,
    }
}

fn icon_back(id: &str, x: f64, y: f64) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "x": x,
        "y": y,
        "width": 44,
        "height": 44,
        "layout": "none",
        "children": [{
            "type": "icon_font",
            "id": format!("{id}-icon"),
            "x": 12,
            "y": 12,
            "width": 20,
            "height": 20,
            "iconFontName": "chevron-left"
        }]
    })
}

fn path_back(id: &str, icon_id: Option<&str>, name: Option<&str>) -> Value {
    let mut path = json!({
        "type": "path",
        "id": format!("{id}-path"),
        "x": 12,
        "y": 12,
        "width": 20,
        "height": 20,
        "d": "M15 18l-6-6 6-6"
    });
    if let Some(icon_id) = icon_id {
        path["iconId"] = json!(icon_id);
    }
    if let Some(name) = name {
        path["name"] = json!(name);
    }
    json!({
        "type": "frame",
        "id": id,
        "x": 24,
        "y": 80,
        "width": 44,
        "height": 44,
        "layout": "none",
        "children": [path]
    })
}

fn card(id: &str) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "width": 100,
        "height": 150,
        "layout": "vertical",
        "children": [
            {
                "type": "image",
                "id": format!("{id}-image"),
                "width": 100,
                "height": 90,
                "src": "https://example.invalid/poster.png"
            },
            {
                "type": "text",
                "id": format!("{id}-title"),
                "width": 100,
                "height": 20,
                "content": "Movie"
            }
        ]
    })
}

fn entry_screen(cards: Vec<Value>) -> Value {
    json!({
        "type": "frame",
        "id": "entry",
        "name": "Discover",
        "screen": "/",
        "x": 0,
        "y": 0,
        "width": 390,
        "height": 844,
        "layout": "none",
        "children": [{
            "type": "frame",
            "id": "card-row",
            "x": 20,
            "y": 180,
            "width": 350,
            "height": 170,
            "layout": "horizontal",
            "gap": 12,
            "children": cards
        }]
    })
}

fn detail_screen(id: &str, route: &str, back: Value) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "name": "Movie Detail",
        "screen": route,
        "x": 450,
        "y": 0,
        "width": 390,
        "height": 844,
        "layout": "none",
        "children": [
            back,
            {
                "type": "frame",
                "id": format!("{id}-content"),
                "x": 0,
                "y": 160,
                "width": 390,
                "height": 600,
                "children": []
            }
        ]
    })
}

fn document(children: Vec<Value>) -> Value {
    json!({ "version": "1.0", "children": children })
}

fn node_value(state: &EditorState, id: &str) -> Value {
    let node =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id.to_string()))
            .unwrap_or_else(|| panic!("missing node {id}"));
    serde_json::to_value(node).expect("serialize node")
}

fn on_tap(state: &EditorState, id: &str) -> Option<Value> {
    node_value(state, id)
        .get("events")
        .and_then(|events| events.get("onTap"))
        .cloned()
}

#[test]
fn strict_back_is_wired_but_card_shapes_do_not_prove_entity_identity() {
    let mut sink = sink_from_value(document(vec![
        entry_screen(vec![card("movie-a"), card("movie-b")]),
        detail_screen("detail", "/detail", icon_back("back", 24.0, 80.0)),
    ]));

    let before = crate::geometry_validation::geometry_diagnostics(&sink.state);
    assert!(before
        .iter()
        .any(|line| line.starts_with("interaction-unwired-back: 1")));
    assert!(before
        .iter()
        .any(|line| line.starts_with("interaction-unwired-cards: 2")));

    wire_interaction_backfill(&mut sink);

    assert_eq!(on_tap(&sink.state, "back"), Some(json!([{"pop": null}])));
    assert!(
        on_tap(&sink.state, "back-icon").is_none(),
        "the square frame, not its icon child, owns the tap target"
    );
    for card_id in ["movie-a", "movie-b"] {
        assert_eq!(
            on_tap(&sink.state, card_id),
            None,
            "shape alone cannot prove which entity this detail represents"
        );
    }
    let after = crate::geometry_validation::geometry_diagnostics(&sink.state);
    assert!(after
        .iter()
        .any(|line| line.starts_with("interaction-unwired-cards: 2")
            && line.contains("selected entity")));
    assert!(
        after
            .iter()
            .all(|line| !line.starts_with("interaction-unwired-back:")),
        "{after:?}"
    );
}

#[test]
fn root_scoped_echo_reports_only_targets_inside_the_inserted_subtree() {
    let sink = sink_from_value(document(vec![
        entry_screen(vec![card("movie-a"), card("movie-b")]),
        detail_screen("detail", "/detail", icon_back("back", 24.0, 80.0)),
    ]));

    let mut cards_only = Vec::new();
    push_interaction_backfill_diagnostics(
        &sink.state,
        Some(&["card-row".to_string()]),
        &mut cards_only,
    );
    assert_eq!(cards_only.len(), 1);
    assert!(cards_only[0].starts_with("interaction-unwired-cards: 2"));

    let mut detail_only = Vec::new();
    push_interaction_backfill_diagnostics(
        &sink.state,
        Some(&["detail".to_string()]),
        &mut detail_only,
    );
    assert_eq!(detail_only.len(), 1);
    assert!(detail_only[0].starts_with("interaction-unwired-back: 1"));
}

#[test]
fn entry_screen_back_shape_is_never_wired() {
    let mut entry = entry_screen(vec![card("movie-a"), card("movie-b")]);
    entry["children"]
        .as_array_mut()
        .unwrap()
        .insert(0, icon_back("entry-menu", 24.0, 80.0));
    let mut sink = sink_from_value(document(vec![
        entry,
        detail_screen("detail", "/detail", icon_back("detail-back", 24.0, 80.0)),
    ]));

    wire_interaction_backfill(&mut sink);

    assert!(on_tap(&sink.state, "entry-menu").is_none());
    assert_eq!(
        on_tap(&sink.state, "detail-back"),
        Some(json!([{"pop": null}]))
    );
}

#[test]
fn two_detail_candidates_abandon_all_card_pushes_without_guessing() {
    let mut second = detail_screen(
        "detail-b",
        "/detail-b",
        icon_back("detail-b-back", 24.0, 80.0),
    );
    second["x"] = json!(900);
    let mut sink = sink_from_value(document(vec![
        entry_screen(vec![card("movie-a"), card("movie-b")]),
        detail_screen(
            "detail-a",
            "/detail-a",
            icon_back("detail-a-back", 24.0, 80.0),
        ),
        second,
    ]));

    wire_interaction_backfill(&mut sink);

    assert!(on_tap(&sink.state, "movie-a").is_none());
    assert!(on_tap(&sink.state, "movie-b").is_none());
    assert_eq!(
        on_tap(&sink.state, "detail-a-back"),
        Some(json!([{"pop": null}]))
    );
    assert_eq!(
        on_tap(&sink.state, "detail-b-back"),
        Some(json!([{"pop": null}]))
    );
}

#[test]
fn authored_events_are_byte_stable_and_unwired_siblings_stay_unwired() {
    let mut authored_card = card("movie-authored");
    authored_card["events"] = json!({
        "onTap": [{"custom_action": null}],
        "onHover": [{"set": {"selected": true}}]
    });
    let mut authored_back = icon_back("back", 24.0, 80.0);
    authored_back["events"] = json!({"onTap": [{"pop": null}]});
    let mut sink = sink_from_value(document(vec![
        entry_screen(vec![authored_card, card("movie-a"), card("movie-b")]),
        detail_screen("detail", "/detail", authored_back),
    ]));
    let card_events_before = node_value(&sink.state, "movie-authored")["events"].clone();
    let back_events_before = node_value(&sink.state, "back")["events"].clone();

    wire_interaction_backfill(&mut sink);

    assert_eq!(
        node_value(&sink.state, "movie-authored")["events"],
        card_events_before
    );
    assert_eq!(
        node_value(&sink.state, "back")["events"],
        back_events_before
    );
    assert_eq!(on_tap(&sink.state, "movie-a"), None);
    assert_eq!(on_tap(&sink.state, "movie-b"), None);
}

#[test]
fn conflicting_back_action_is_not_a_detail_fact_and_never_routes_cards() {
    let mut conflicting_back = icon_back("back", 24.0, 80.0);
    conflicting_back["events"] = json!({
        "onTap": [{"pop": null}, {"custom_action": null}]
    });
    let mut sink = sink_from_value(document(vec![
        entry_screen(vec![card("movie-a"), card("movie-b")]),
        detail_screen("detail", "/detail", conflicting_back),
    ]));
    let before = node_value(&sink.state, "back")["events"].clone();

    wire_interaction_backfill(&mut sink);

    assert_eq!(node_value(&sink.state, "back")["events"], before);
    assert!(on_tap(&sink.state, "movie-a").is_none());
    assert!(on_tap(&sink.state, "movie-b").is_none());
}

#[test]
fn exact_pop_detail_stays_navless_without_inventing_card_handoffs() {
    let mut entry = entry_screen(vec![card("movie-a"), card("movie-b")]);
    let entry_tabs = ["Home", "Search", "Library"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            json!({
                "type": "frame",
                "id": format!("tab-{index}"),
                "width": 130,
                "height": 64,
                "children": [{
                    "type": "text",
                    "id": format!("tab-{index}-label"),
                    "content": label,
                    "fontSize": 12
                }]
            })
        })
        .collect::<Vec<_>>();
    entry["children"].as_array_mut().unwrap().push(json!({
        "type": "frame",
        "id": "entry-nav",
        "role": "bottom-tab-bar",
        "x": 0,
        "y": 780,
        "width": 390,
        "height": 64,
        "layout": "horizontal",
        "children": entry_tabs
    }));
    let mut back = icon_back("back", 24.0, 80.0);
    back["events"] = json!({"onTap": [{"pop": null}]});
    let mut detail = detail_screen("detail", "/detail", back);
    detail["name"] = json!("Library");
    let mut sink = sink_from_value(document(vec![entry, detail]));

    crate::unify_shared_nav::unify_shared_nav(&mut sink);
    wire_interaction_backfill(&mut sink);

    assert_eq!(
        node_value(&sink.state, "detail")["children"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(on_tap(&sink.state, "movie-a"), None);
    assert_eq!(on_tap(&sink.state, "movie-b"), None);
}

#[test]
fn invalid_detail_route_is_not_a_screen_fact() {
    let mut detail = detail_screen("detail", "movie-detail", icon_back("back", 24.0, 80.0));
    detail["x"] = json!(450);
    let mut sink = sink_from_value(document(vec![
        entry_screen(vec![card("movie-a"), card("movie-b")]),
        detail,
    ]));

    wire_interaction_backfill(&mut sink);

    assert!(on_tap(&sink.state, "back").is_none());
    assert!(on_tap(&sink.state, "movie-a").is_none());
    assert!(on_tap(&sink.state, "movie-b").is_none());
}

#[test]
fn ancestor_on_tap_blocks_card_backfill_for_the_whole_chain() {
    let mut entry = entry_screen(vec![card("movie-a"), card("movie-b")]);
    entry["children"][0]["events"] = json!({"onTap": [{"custom_action": null}]});
    let mut sink = sink_from_value(document(vec![
        entry,
        detail_screen("detail", "/detail", icon_back("back", 24.0, 80.0)),
    ]));

    wire_interaction_backfill(&mut sink);

    assert!(on_tap(&sink.state, "movie-a").is_none());
    assert!(on_tap(&sink.state, "movie-b").is_none());
}

#[test]
fn path_uses_icon_id_only_and_never_falls_back_to_node_name() {
    let mut detail = detail_screen(
        "detail",
        "/detail",
        path_back("real-back", Some("lucide:arrow-left"), Some("Anything")),
    );
    detail["children"]
        .as_array_mut()
        .unwrap()
        .insert(1, path_back("name-only", None, Some("arrow-left")));
    let mut sink = sink_from_value(document(vec![
        entry_screen(vec![card("movie-a"), card("movie-b")]),
        detail,
    ]));

    wire_interaction_backfill(&mut sink);

    assert_eq!(
        on_tap(&sink.state, "real-back"),
        Some(json!([{"pop": null}]))
    );
    assert!(on_tap(&sink.state, "name-only").is_none());
}

#[test]
fn geometry_rejects_late_large_non_square_and_multi_child_controls() {
    let mut detail = detail_screen("detail", "/detail", icon_back("valid-back", 24.0, 80.0));
    let late = icon_back("late", 24.0, 140.0);
    let mut large = icon_back("large", 24.0, 80.0);
    large["width"] = json!(64);
    large["height"] = json!(64);
    let mut non_square = icon_back("non-square", 24.0, 80.0);
    non_square["width"] = json!(48);
    non_square["height"] = json!(36);
    let mut multi = icon_back("multi", 24.0, 80.0);
    multi["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"text","id":"extra","content":"x","width":10,"height":10}));
    detail["children"]
        .as_array_mut()
        .unwrap()
        .splice(1..1, [late, large, non_square, multi]);
    let mut sink = sink_from_value(document(vec![
        entry_screen(vec![card("movie-a"), card("movie-b")]),
        detail,
    ]));

    wire_interaction_backfill(&mut sink);

    assert_eq!(
        on_tap(&sink.state, "valid-back"),
        Some(json!([{"pop": null}]))
    );
    for id in ["late", "large", "non-square", "multi"] {
        assert!(
            on_tap(&sink.state, id).is_none(),
            "{id} must stay untouched"
        );
    }
}

#[test]
fn trailing_bottom_nav_uses_geometry_bottom_gap_predicate_for_detail_gate() {
    let nav_tabs = [0, 1, 2]
        .into_iter()
        .map(|index| {
            json!({
                "type": "frame",
                "id": format!("unnamed-tab-{index}"),
                "width": 130,
                "height": 64,
                "children": []
            })
        })
        .collect::<Vec<_>>();
    let mut tab_screen = detail_screen(
        "tab-screen",
        "/tab-screen",
        icon_back("tab-back", 24.0, 80.0),
    );
    tab_screen["children"].as_array_mut().unwrap().push(json!({
        "type": "frame",
        "id": "unnamed-nav",
        "x": 0,
        "y": 780,
        "width": 390,
        "height": 64,
        "layout": "horizontal",
        "children": nav_tabs
    }));
    let mut actual_detail = detail_screen(
        "actual-detail",
        "/actual-detail",
        icon_back("actual-back", 24.0, 80.0),
    );
    actual_detail["x"] = json!(900);
    let mut sink = sink_from_value(document(vec![
        entry_screen(vec![card("movie-a"), card("movie-b")]),
        tab_screen,
        actual_detail,
    ]));

    wire_interaction_backfill(&mut sink);

    assert_eq!(on_tap(&sink.state, "movie-a"), None);
    assert_eq!(on_tap(&sink.state, "movie-b"), None);
}

#[test]
fn single_page_without_screen_is_a_byte_exact_noop_and_pass_is_idempotent() {
    let mut single = sink_from_value(document(vec![json!({
        "type": "frame",
        "id": "poster",
        "name": "Single-page poster",
        "width": 390,
        "height": 844,
        "children": [icon_back("decorative-arrow", 24.0, 80.0)]
    })]));
    let before = serde_json::to_value(single.state.active_children()).unwrap();
    wire_interaction_backfill(&mut single);
    assert_eq!(
        serde_json::to_value(single.state.active_children()).unwrap(),
        before
    );
    assert!(single.applied.is_empty());

    let mut multi = sink_from_value(document(vec![
        entry_screen(vec![card("movie-a"), card("movie-b")]),
        detail_screen("detail", "/detail", icon_back("back", 24.0, 80.0)),
    ]));
    wire_interaction_backfill(&mut multi);
    let once = serde_json::to_value(multi.state.active_children()).unwrap();
    wire_interaction_backfill(&mut multi);
    assert_eq!(
        serde_json::to_value(multi.state.active_children()).unwrap(),
        once
    );
}

#[test]
fn distinct_merchants_never_inherit_the_first_merchants_static_detail() {
    let mut first = card("first");
    first["children"][1]["content"] = json!("老王家常菜");
    first["events"] = json!({"onTap":[{"push":"\"/detail\""}]});
    let mut second = card("second");
    second["children"][1]["content"] = json!("川香居麻辣香锅");
    let mut third = card("third");
    third["children"][1]["content"] = json!("状元粥铺");
    let mut sink = sink_from_value(document(vec![
        entry_screen(vec![first, second, third]),
        detail_screen("detail", "/detail", icon_back("back", 24.0, 80.0)),
    ]));
    let before = node_value(&sink.state, "card-row");
    wire_interaction_backfill(&mut sink);
    assert_eq!(node_value(&sink.state, "card-row"), before);
    assert!(on_tap(&sink.state, "second").is_none());
    assert!(on_tap(&sink.state, "third").is_none());
    assert_eq!(on_tap(&sink.state, "back"), Some(json!([{"pop":null}])));
}

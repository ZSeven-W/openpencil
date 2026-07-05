use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn insert_tree(sink: &mut VecDocSink, json: &str) {
    let tree: PenNode = serde_json::from_str(json).expect("test tree json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
}

fn find_node<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, id))
}

fn find_active_node<'a>(sink: &'a VecDocSink, id: &str) -> &'a PenNode {
    sink.state
        .active_children()
        .iter()
        .find_map(|node| find_node(node, id))
        .expect("node exists")
}

fn node_json(sink: &VecDocSink, id: &str) -> Value {
    serde_json::to_value(find_active_node(sink, id)).expect("serialize node")
}

#[test]
fn bunched_bottom_nav_tabs_distributed() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "bottom-nav",
                    "name": "Bottom Navigation",
                    "width": "fill_container",
                    "height": 72,
                    "layout": "horizontal",
                    "justifyContent": "end",
                    "children": [
                        {
                            "type": "frame",
                            "id": "home-tab",
                            "name": "Home Tab",
                            "width": 68,
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                { "type": "icon_font", "id": "home-icon", "name": "Home Icon", "iconFontName": "home", "width": 22, "height": 22 },
                                { "type": "text", "id": "home-label", "name": "Home Label", "content": "Home", "width": "fit_content", "height": 16 }
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "search-tab",
                            "name": "Search Tab",
                            "width": 68,
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                { "type": "icon_font", "id": "search-icon", "name": "Search Icon", "iconFontName": "search", "width": 22, "height": 22 },
                                { "type": "text", "id": "search-label", "name": "Search Label", "content": "Search", "width": "fit_content", "height": 16 }
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "cart-tab",
                            "name": "Cart Tab",
                            "width": 68,
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                { "type": "icon_font", "id": "cart-icon", "name": "Cart Icon", "iconFontName": "shopping-cart", "width": 22, "height": 22 },
                                { "type": "text", "id": "cart-label", "name": "Cart Label", "content": "Cart", "width": "fit_content", "height": 16 }
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "profile-tab",
                            "name": "Profile Tab",
                            "width": 68,
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                { "type": "icon_font", "id": "profile-icon", "name": "Profile Icon", "iconFontName": "user", "width": 22, "height": 22 },
                                { "type": "text", "id": "profile-label", "name": "Profile Label", "content": "Profile", "width": "fit_content", "height": 16 }
                            ]
                        }
                    ]
                }
            ]
        }"##,
    );

    distribute_bottom_nav_tabs(&mut sink, "root");

    assert_eq!(
        node_json(&sink, "bottom-nav")["justifyContent"],
        json!("space_between")
    );
    for tab_id in ["home-tab", "search-tab", "cart-tab", "profile-tab"] {
        assert_eq!(node_json(&sink, tab_id)["width"], json!("fill_container"));
    }
}

#[test]
fn date_chip_row_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "date-row",
                    "name": "Date Chips",
                    "width": "fill_container",
                    "height": 40,
                    "layout": "horizontal",
                    "justifyContent": "start",
                    "children": [
                        {
                            "type": "frame",
                            "id": "today-chip",
                            "name": "Today Chip",
                            "width": 96,
                            "height": 36,
                            "layout": "horizontal",
                            "children": [
                                { "type": "icon_font", "id": "today-icon", "iconFontName": "calendar", "width": 16, "height": 16 },
                                { "type": "text", "id": "today-label", "content": "Today", "width": "fit_content", "height": 16 }
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "tomorrow-chip",
                            "name": "Tomorrow Chip",
                            "width": 116,
                            "height": 36,
                            "layout": "horizontal",
                            "children": [
                                { "type": "icon_font", "id": "tomorrow-icon", "iconFontName": "calendar", "width": 16, "height": 16 },
                                { "type": "text", "id": "tomorrow-label", "content": "Tomorrow", "width": "fit_content", "height": 16 }
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "weekend-chip",
                            "name": "Weekend Chip",
                            "width": 112,
                            "height": 36,
                            "layout": "horizontal",
                            "children": [
                                { "type": "icon_font", "id": "weekend-icon", "iconFontName": "calendar", "width": 16, "height": 16 },
                                { "type": "text", "id": "weekend-label", "content": "Weekend", "width": "fit_content", "height": 16 }
                            ]
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "date-row");

    distribute_bottom_nav_tabs(&mut sink, "root");

    assert_eq!(node_json(&sink, "date-row"), before);
}

#[test]
fn two_tab_row_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "two-tab-row",
                    "name": "Two Tabs",
                    "width": "fill_container",
                    "height": 72,
                    "layout": "horizontal",
                    "justifyContent": "end",
                    "children": [
                        {
                            "type": "frame",
                            "id": "left-tab",
                            "width": 96,
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                { "type": "icon_font", "id": "left-icon", "iconFontName": "home", "width": 22, "height": 22 },
                                { "type": "text", "id": "left-label", "content": "Home", "width": "fit_content", "height": 16 }
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "right-tab",
                            "width": 96,
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                { "type": "icon_font", "id": "right-icon", "iconFontName": "user", "width": 22, "height": 22 },
                                { "type": "text", "id": "right-label", "content": "Profile", "width": "fit_content", "height": 16 }
                            ]
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "two-tab-row");

    distribute_bottom_nav_tabs(&mut sink, "root");

    assert_eq!(node_json(&sink, "two-tab-row"), before);
}

#[test]
fn already_distributed_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "bottom-nav",
                    "name": "Bottom Navigation",
                    "width": "fill_container",
                    "height": 72,
                    "layout": "horizontal",
                    "justifyContent": "space_between",
                    "children": [
                        {
                            "type": "frame",
                            "id": "home-tab",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                { "type": "icon_font", "id": "home-icon", "iconFontName": "home", "width": 22, "height": 22 },
                                { "type": "text", "id": "home-label", "content": "Home", "width": "fit_content", "height": 16 }
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "search-tab",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                { "type": "icon_font", "id": "search-icon", "iconFontName": "search", "width": 22, "height": 22 },
                                { "type": "text", "id": "search-label", "content": "Search", "width": "fit_content", "height": 16 }
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "profile-tab",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                { "type": "icon_font", "id": "profile-icon", "iconFontName": "user", "width": 22, "height": 22 },
                                { "type": "text", "id": "profile-label", "content": "Profile", "width": "fit_content", "height": 16 }
                            ]
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "bottom-nav");

    distribute_bottom_nav_tabs(&mut sink, "root");

    assert_eq!(node_json(&sink, "bottom-nav"), before);
    assert!(sink.applied.is_empty());
}

#[test]
fn non_tab_horizontal_row_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "plain-row",
                    "name": "Plain Horizontal Row",
                    "width": "fill_container",
                    "height": 72,
                    "layout": "horizontal",
                    "justifyContent": "end",
                    "children": [
                        { "type": "frame", "id": "plain-a", "width": 68, "height": 44, "layout": "vertical", "children": [] },
                        { "type": "frame", "id": "plain-b", "width": 68, "height": 44, "layout": "vertical", "children": [] },
                        { "type": "frame", "id": "plain-c", "width": 68, "height": 44, "layout": "vertical", "children": [] }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "plain-row");

    distribute_bottom_nav_tabs(&mut sink, "root");

    assert_eq!(node_json(&sink, "plain-row"), before);
}

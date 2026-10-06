use super::repair_category_grid_density;
use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};

fn icon(id: &str, size: f64) -> Value {
    json!({
        "type": "icon_font",
        "id": id,
        "name": format!("{id} icon"),
        "iconFontName": "utensils-crossed",
        "width": size,
        "height": size
    })
}

fn label(id: &str, content: &str, font_size: f64) -> Value {
    json!({
        "type": "text",
        "id": id,
        "name": format!("{id} label"),
        "content": content,
        "fontSize": font_size
    })
}

fn category_tile(id: &str, content: &str, holder_size: f64, icon_size: f64) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "name": format!("分类瓷片-{id}-{content}"),
        "layout": "vertical",
        "gap": 20,
        "padding": [12, 12],
        "fill": [{"type": "solid", "color": "#FFFFFF"}],
        "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]},
        "children": [
            {
                "type": "frame",
                "id": format!("{id}-holder"),
                "name": format!("{id}-holder-{content}-图标底"),
                "width": holder_size,
                "height": holder_size,
                "layout": "horizontal",
                "fill": [{"type": "solid", "color": "#F1F5F9"}],
                "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#CBD5E1"}]},
                "cornerRadius": 8,
                "children": [icon(&format!("{id}-icon"), icon_size)]
            },
            label(&format!("{id}-label"), content, 12.0)
        ]
    })
}

fn category_row(id: &str, labels: &[&str]) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "name": format!("分类行 {id}"),
        "layout": "horizontal",
        "gap": 20,
        "children": labels.iter().enumerate().map(|(index, content)| {
            category_tile(&format!("{id}-tile-{index}"), content, 56.0, 20.0)
        }).collect::<Vec<_>>()
    })
}

fn category_grid(row_count: usize) -> Value {
    let rows = (0..row_count)
        .map(|index| {
            category_row(
                &format!("row-{index}"),
                &["美食外卖", "超市便利", "水果生鲜", "药品健康"],
            )
        })
        .collect::<Vec<_>>();
    json!({
        "type": "frame",
        "id": "category-grid",
        "name": "分类九宫格",
        "layout": "vertical",
        "gap": 20,
        "children": rows
    })
}

fn sink_with_root(mut root: Value) -> VecDocSink {
    root["type"] = json!("frame");
    let mut sink = VecDocSink::new();
    let root: PenNode = serde_json::from_value(root).expect("root fixture");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

fn mobile_root(children: Value) -> Value {
    json!({
        "type": "frame",
        "id": "root",
        "name": "外卖 App 首页",
        "width": 375,
        "height": 812,
        "layout": "vertical",
        "children": children
    })
}

fn root_json(sink: &VecDocSink) -> Value {
    serde_json::to_value(&sink.state.active_children()[0]).expect("serialized root")
}

fn root_id(sink: &VecDocSink) -> String {
    sink.state.active_children()[0].id_str().to_string()
}

fn node_json(sink: &VecDocSink, id: &str) -> Value {
    fn find(node: &Value, id: &str) -> Option<Value> {
        if node["id"].as_str() == Some(id) {
            return Some(node.clone());
        }
        node["children"]
            .as_array()?
            .iter()
            .find_map(|child| find(child, id))
    }
    fn find_named(node: &Value, id: &str) -> Option<Value> {
        if node["name"].as_str().is_some_and(|name| name.contains(id)) {
            return Some(node.clone());
        }
        node["children"]
            .as_array()?
            .iter()
            .find_map(|child| find_named(child, id))
    }
    let root = root_json(sink);
    find(&root, id)
        .or_else(|| {
            (id == "category-grid")
                .then(|| find_named(&root, "分类九宫格"))
                .flatten()
        })
        .or_else(|| find_named(&root, id))
        .unwrap_or_else(|| {
            eprintln!("lookup failed for {id}: {root}");
            panic!("node fixture")
        })
}

#[test]
fn app01_grid_shrinks_density_without_changing_four_columns() {
    let mut sink = sink_with_root(mobile_root(json!([category_grid(2)])));

    let root_id = root_id(&sink);
    let changed = repair_category_grid_density(&mut sink, &root_id);

    assert_eq!(
        changed, 19,
        "two rows and sixteen tile/holder edits are itemized"
    );
    let grid = node_json(&sink, "category-grid");
    assert_eq!(grid["children"].as_array().unwrap().len(), 2);
    assert_eq!(grid["gap"], 12.0);
    for row_index in 0..2 {
        let row = node_json(&sink, &format!("row-{row_index}"));
        assert_eq!(row["gap"], 12.0);
        assert_eq!(row["children"].as_array().unwrap().len(), 4);
        for tile_index in 0..4 {
            let tile = node_json(&sink, &format!("row-{row_index}-tile-{tile_index}"));
            assert!(tile["padding"].is_null());
            assert!(tile["stroke"].is_null());
            assert!(tile["fill"].is_null());
            assert_eq!(tile["gap"], 8.0);
            let holder = node_json(&sink, &format!("row-{row_index}-tile-{tile_index}-holder"));
            assert_eq!(holder["width"], 44.0);
            assert_eq!(holder["height"], 44.0);
            assert!(holder["stroke"].is_null());
            assert_eq!(holder["cornerRadius"], 8.0);
            assert!(!holder["fill"].is_null());
            assert_eq!(holder["children"][0]["width"], 20.0);
            assert_eq!(
                node_json(&sink, &format!("row-{row_index}-tile-{tile_index}-label"))["fontSize"],
                12.0
            );
        }
    }
}

#[test]
fn bounded_category_grid_is_untouched() {
    let mut root = mobile_root(json!([category_grid(2)]));
    for row_index in 0..2 {
        let row = &mut root["children"][0]["children"][row_index];
        row["gap"] = json!(12);
        for tile_index in 0..4 {
            let tile = &mut row["children"][tile_index];
            tile["gap"] = json!(8);
            tile["padding"] = Value::Null;
            tile["fill"] = Value::Null;
            tile["stroke"] = Value::Null;
            tile["children"][0]["width"] = json!(44);
            tile["children"][0]["height"] = json!(44);
            tile["children"][0]["stroke"] = Value::Null;
            tile["children"][0]["children"][0]["width"] = json!(24);
            tile["children"][0]["children"][0]["height"] = json!(24);
            tile["children"][1]["fontSize"] = json!(12);
        }
    }
    root["children"][0]["gap"] = json!(12);
    let mut sink = sink_with_root(root);
    let before = root_json(&sink);

    let root_id = root_id(&sink);
    assert_eq!(repair_category_grid_density(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn bottom_tab_bar_is_untouched() {
    let row = category_row("bottom-row", &["Home", "Search", "Orders", "Profile"]);
    let root = mobile_root(json!([
        {"type": "frame", "id": "content", "layout": "vertical", "children": []},
        {"type": "frame", "id": "tab-shell", "name": "Tab Bar", "role": "tab-bar",
         "layout": "vertical", "children": [row]}
    ]));
    let mut sink = sink_with_root(root);
    let before = root_json(&sink);

    let root_id = root_id(&sink);
    assert_eq!(repair_category_grid_density(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn content_grid_is_not_excluded_by_a_bottom_nav_sibling() {
    let root = mobile_root(json!([
        category_grid(1),
        {"type": "frame", "id": "bottom", "name": "Bottom Navigation",
         "role": "bottom-tab-bar", "layout": "horizontal", "children": []}
    ]));
    let mut sink = sink_with_root(root);

    let root_id = root_id(&sink);
    assert!(repair_category_grid_density(&mut sink, &root_id) > 0);
    assert_eq!(node_json(&sink, "row-0-tile-0-holder")["width"], 44.0);
}

#[test]
fn numeric_keypad_row_is_untouched() {
    let root = mobile_root(json!([{
        "type": "frame", "id": "actions", "layout": "vertical", "children": [
            category_row("keypad-row", &["1", "2", "3", "4"])
        ]
    }]));
    let mut sink = sink_with_root(root);
    let before = root_json(&sink);

    let root_id = root_id(&sink);
    assert_eq!(repair_category_grid_density(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn product_grid_with_images_is_untouched() {
    let tiles = (0..4)
        .map(|index| json!({
            "type": "frame", "id": format!("product-{index}"), "layout": "vertical",
            "children": [
                {"type": "image", "id": format!("product-image-{index}"), "width": 56, "height": 56, "src": "product.png"},
                {"type": "text", "id": format!("product-label-{index}"), "content": "商品", "fontSize": 14}
            ]
        }))
        .collect::<Vec<_>>();
    let root = mobile_root(json!([{
        "type": "frame", "id": "products", "layout": "horizontal", "gap": 20,
        "children": tiles
    }]));
    let mut sink = sink_with_root(root);
    let before = root_json(&sink);

    let root_id = root_id(&sink);
    assert_eq!(repair_category_grid_density(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn map_canvas_row_is_untouched() {
    let root = mobile_root(json!([{
        "type": "frame", "id": "map-canvas", "name": "map-canvas", "layout": "vertical",
        "children": [category_row("map-row", &["餐馆", "商店", "药房", "咖啡"])]
    }]));
    let mut sink = sink_with_root(root);
    let before = root_json(&sink);

    let root_id = root_id(&sink);
    assert_eq!(repair_category_grid_density(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn status_bar_subtree_is_untouched() {
    let root = mobile_root(json!([{
        "type": "frame", "id": "status", "role": "status-bar", "layout": "vertical",
        "children": [category_row("status-row", &["餐馆", "商店", "药房", "咖啡"])]
    }]));
    let mut sink = sink_with_root(root);
    let before = root_json(&sink);

    let root_id = root_id(&sink);
    assert_eq!(repair_category_grid_density(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn root_itself_is_not_repaired() {
    let root = mobile_root(json!([
        category_tile("root-tile-0", "餐馆", 56.0, 20.0),
        category_tile("root-tile-1", "商店", 56.0, 20.0),
        category_tile("root-tile-2", "药房", 56.0, 20.0),
        category_tile("root-tile-3", "咖啡", 56.0, 20.0)
    ]));
    let mut root = root;
    root["layout"] = json!("horizontal");
    let mut sink = sink_with_root(root);
    let before = root_json(&sink);

    let root_id = root_id(&sink);
    assert_eq!(repair_category_grid_density(&mut sink, &root_id), 0);
    assert_eq!(root_json(&sink), before);
}

#[test]
fn oversized_label_shrinks_without_changing_text_growth() {
    let mut root = mobile_root(json!([category_grid(1)]));
    root["children"][0]["children"][0]["children"][0]["children"][1]["fontSize"] = json!(16);
    root["children"][0]["children"][0]["children"][0]["children"][1]["textGrowth"] =
        json!("fixed-width");
    let mut sink = sink_with_root(root);

    let root_id = root_id(&sink);
    repair_category_grid_density(&mut sink, &root_id);

    let label = node_json(&sink, "row-0-tile-0-label");
    assert_eq!(label["fontSize"], 12.0);
    assert_eq!(label["textGrowth"], "fixed-width");
}

#[test]
fn direct_icon_tile_is_repaired_without_a_container_patch() {
    let mut tiles = Vec::new();
    for index in 0..4 {
        let mut tile = category_tile(&format!("direct-tile-{index}"), "餐馆", 56.0, 32.0);
        let direct_icon = tile["children"][0]["children"][0].clone();
        tile["children"][0] = direct_icon;
        tiles.push(tile);
    }
    let root = mobile_root(json!([{
        "type": "frame", "id": "direct-row", "layout": "horizontal", "gap": 20,
        "children": tiles
    }]));
    let mut sink = sink_with_root(root);

    let root_id = root_id(&sink);
    repair_category_grid_density(&mut sink, &root_id);

    let icon = node_json(&sink, "direct-tile-0-icon");
    assert_eq!(icon["width"], 24.0);
    assert_eq!(icon["height"], 24.0);
}

#[test]
fn empty_unrounded_container_keeps_its_stroke() {
    let mut root = mobile_root(json!([category_grid(1)]));
    root["children"][0]["children"][0]["children"][0]["children"][0]["fill"] = Value::Null;
    root["children"][0]["children"][0]["children"][0]["children"][0]["cornerRadius"] = Value::Null;
    let mut sink = sink_with_root(root);

    let root_id = root_id(&sink);
    assert!(repair_category_grid_density(&mut sink, &root_id) > 0);
    let holder = node_json(&sink, "row-0-tile-0-holder");
    assert!(!holder["stroke"].is_null());
    assert_eq!(holder["width"], 44.0);
    assert!(holder["cornerRadius"].is_null());
}

#[test]
fn small_icons_are_not_grown() {
    let mut root = mobile_root(json!([category_grid(1)]));
    root["children"][0]["children"][0]["children"][0]["children"][0]["children"][0]["width"] =
        json!(16);
    root["children"][0]["children"][0]["children"][0]["children"][0]["children"][0]["height"] =
        json!(16);
    let mut sink = sink_with_root(root);

    let root_id = root_id(&sink);
    repair_category_grid_density(&mut sink, &root_id);

    assert_eq!(
        node_json(&sink, "row-0-tile-0-holder")["children"][0]["width"],
        16.0
    );
    assert_eq!(
        node_json(&sink, "row-0-tile-0-holder")["children"][0]["height"],
        16.0
    );
}

#[test]
fn three_rows_stay_three_rows_when_estimate_is_over_150() {
    let mut sink = sink_with_root(mobile_root(json!([category_grid(3)])));

    repair_category_grid_density(&mut sink, "root");

    let grid = node_json(&sink, "category-grid");
    assert_eq!(grid["children"].as_array().unwrap().len(), 3);
    assert!(sink
        .applied
        .iter()
        .all(|command| !matches!(command, EditorCommand::DeleteNode { .. })));
}

#[test]
fn repairs_are_itemized_with_tile_and_row_names() {
    let mut sink = sink_with_root(mobile_root(json!([category_grid(1)])));
    let root_id = root_id(&sink);
    let mut counter = RepairCounter::new();
    let mut counting = counter.wrap(&mut sink);
    repair_category_grid_density(&mut counting, &root_id);
    let mut summary = RepairSummary::default();
    counter.checkpoint(
        &mut summary,
        CheckCategory::Structure,
        "category-grid-density",
    );

    assert_eq!(summary.records()[0].pass, "category-grid-density");
    assert!(summary.records().iter().any(|record| {
        record
            .node_name
            .as_deref()
            .is_some_and(|name| name.starts_with("分类瓷片-"))
    }));
    assert!(summary.records().iter().any(|record| {
        record
            .node_name
            .as_deref()
            .is_some_and(|name| name.contains("分类行"))
    }));
}

#[test]
fn cleanup_driver_checkpoints_category_grid_density() {
    let mut sink = sink_with_root(mobile_root(json!([category_grid(1)])));
    let root_id = root_id(&sink);
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: root_id.clone(),
            name: "外卖 App 首页".into(),
            width: 375.0,
            height: 812.0,
            layout: Some("vertical".into()),
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
    };
    let mut summary = RepairSummary::default();

    run_cleanup_passes_with_summary(&mut sink, &plan, &[&root_id], &mut summary);

    assert!(summary.checked().contains(&CheckCategory::Structure));
    assert!(summary
        .records()
        .iter()
        .any(|record| record.pass == "category-grid-density"));
}

#[test]
fn literal_three_by_three_grid_reflows_after_density() {
    let rows = (0..3)
        .map(|index| category_row(&format!("row-{index}"), &["美食", "甜品饮品", "生鲜果蔬"]))
        .collect::<Vec<_>>();
    let grid = json!({
        "type": "frame", "id": "category-grid", "name": "分类九宫格",
        "layout": "vertical", "gap": 20, "children": rows
    });
    let mut sink = sink_with_root(mobile_root(json!([grid])));
    let root_id = root_id(&sink);
    assert!(repair_category_grid_density(&mut sink, &root_id) > 0);
    let grid = node_json(&sink, "category-grid");
    let rows = grid["children"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["children"].as_array().unwrap().len(), 5);
    assert_eq!(rows[1]["children"].as_array().unwrap().len(), 4);
    for row in rows {
        assert_eq!(row["gap"], 12.0);
        assert_eq!(row["justifyContent"], "start");
        for tile in row["children"].as_array().unwrap() {
            assert_eq!(tile["width"], "fill_container");
            assert_eq!(tile["children"][0]["width"], 44.0);
            assert_eq!(tile["children"][0]["height"], 44.0);
        }
    }
}

#[test]
fn a_lone_row_of_three_is_not_a_category_strip() {
    let row = category_row("row-0", &["美食", "甜品饮品", "生鲜果蔬"]);
    let mut sink = sink_with_root(mobile_root(json!([row])));
    let root_id = root_id(&sink);
    assert_eq!(repair_category_grid_density(&mut sink, &root_id), 0);
}

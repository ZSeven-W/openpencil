use super::super::repair_category_grid_density;
use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::{CheckCategory, RepairSummary};
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

fn label(id: &str, content: &str) -> Value {
    json!({
        "type": "text",
        "id": id,
        "name": format!("{id} label"),
        "content": content,
        "fontSize": 12
    })
}

fn tile(id: &str, content: &str) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "name": format!("分类瓷片-{id}-{content}"),
        "layout": "vertical",
        "width": 72,
        "gap": 20,
        "padding": [12, 12],
        "fill": [{"type": "solid", "color": "#FFFFFF"}],
        "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]},
        "children": [{
            "type": "frame",
            "id": format!("{id}-holder"),
            "name": format!("{id}-holder"),
            "width": 56,
            "height": 56,
            "layout": "horizontal",
            "fill": [{"type": "solid", "color": "#F1F5F9"}],
            "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#CBD5E1"}]},
            "cornerRadius": 8,
            "children": [icon(&format!("{id}-icon"), 20.0)]
        }, label(&format!("{id}-label"), content)]
    })
}

fn row(id: &str, labels: &[&str]) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "name": format!("分类行 {id}"),
        "layout": "horizontal",
        "gap": 20,
        "children": labels.iter().enumerate().map(|(index, content)| {
            tile(&format!("{id}-tile-{index}"), content)
        }).collect::<Vec<_>>()
    })
}

fn grid(rows: Vec<Value>) -> Value {
    json!({
        "type": "frame",
        "id": "category-grid",
        "name": "分类九宫格",
        "layout": "vertical",
        "gap": 20,
        "children": rows
    })
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

fn sink_with_root(root: Value) -> VecDocSink {
    let root: PenNode = serde_json::from_value(root).expect("root fixture");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

fn root_json(sink: &VecDocSink) -> Value {
    serde_json::to_value(&sink.state.active_children()[0]).expect("serialized root")
}

fn node_json(sink: &VecDocSink, id: &str) -> Value {
    fn find(node: &Value, id: &str) -> Option<Value> {
        if node.get("id").and_then(Value::as_str) == Some(id) {
            return Some(node.clone());
        }
        node.get("children")
            .and_then(Value::as_array)?
            .iter()
            .find_map(|child| find(child, id))
    }
    fn find_named(node: &Value, name: &str) -> Option<Value> {
        if node
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|value| value.contains(name))
        {
            return Some(node.clone());
        }
        node.get("children")
            .and_then(Value::as_array)?
            .iter()
            .find_map(|child| find_named(child, name))
    }
    let root = root_json(sink);
    find(&root, id)
        .or_else(|| {
            (id == "category-grid")
                .then(|| find_named(&root, "分类九宫格"))
                .flatten()
        })
        .unwrap_or_else(|| panic!("missing node {id}: {root}"))
}

fn run_density(sink: &mut VecDocSink) {
    let root_id = sink.state.active_children()[0].id_str().to_string();
    repair_category_grid_density(sink, &root_id);
}

#[test]
fn app01_three_by_three_reflows_to_five_plus_four_after_density() {
    let rows = (0..3)
        .map(|index| row(&format!("row-{index}"), &["美食", "超市", "水果"]))
        .collect();
    let mut test_grid = grid(rows);
    test_grid["gap"] = json!(12);
    let mut sink = sink_with_root(mobile_root(json!([test_grid])));
    let before_grid = root_json(&sink)["children"][0].clone();
    let before_rows = before_grid["children"].as_array().unwrap();
    let before_row_ids = before_rows
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    let before_tile_ids = before_rows
        .iter()
        .flat_map(|row| row["children"].as_array().unwrap())
        .map(|tile| tile["id"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();

    run_density(&mut sink);

    let reflowed = node_json(&sink, "category-grid");
    let rows = reflowed["children"].as_array().expect("rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], before_row_ids[0]);
    assert_eq!(rows[1]["id"], before_row_ids[1]);
    assert_eq!(rows[0]["children"].as_array().unwrap().len(), 5);
    assert_eq!(rows[1]["children"].as_array().unwrap().len(), 4);
    assert_eq!(rows[0]["gap"], 12.0);
    assert_eq!(rows[0]["justifyContent"], "start");
    assert_eq!(rows[0]["width"], "fill_container");
    assert_eq!(rows[0]["height"], "fit_content");
    for row in rows {
        assert_eq!(row["gap"], 12.0);
        assert_eq!(row["justifyContent"], "start");
        for tile in row["children"].as_array().unwrap() {
            assert_eq!(tile["width"], "fill_container");
            assert_eq!(tile["children"][0]["width"], 44.0);
            assert_eq!(tile["children"][0]["height"], 44.0);
        }
    }
    let ids = rows
        .iter()
        .flat_map(|row| row["children"].as_array().unwrap())
        .map(|tile| tile["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids[0], before_tile_ids[0]);
    assert_eq!(ids[4], before_tile_ids[4]);
    assert_eq!(ids[8], before_tile_ids[8]);
}

#[test]
fn two_by_four_is_a_legitimate_four_column_grid() {
    let rows = (0..2)
        .map(|index| row(&format!("row-{index}"), &["一", "二", "三", "四"]))
        .collect();
    let mut sink = sink_with_root(mobile_root(json!([grid(rows)])));

    run_density(&mut sink);

    let grid = node_json(&sink, "category-grid");
    assert_eq!(grid["children"].as_array().unwrap().len(), 2);
    assert!(grid["children"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["children"].as_array().unwrap().len() == 4));
}

#[test]
fn labels_over_four_cjk_characters_stay_untouched() {
    let rows = (0..3)
        .map(|index| row(&format!("row-{index}"), &["五个中文字符", "超市", "水果"]))
        .collect();
    let mut sink = sink_with_root(mobile_root(json!([grid(rows)])));
    let before = root_json(&sink);

    run_density(&mut sink);

    assert_eq!(root_json(&sink), before);
}

#[test]
fn six_tiles_use_balanced_three_plus_three_rows() {
    let rows = (0..2)
        .map(|index| row(&format!("row-{index}"), &["一", "二", "三"]))
        .collect();
    let mut sink = sink_with_root(mobile_root(json!([grid(rows)])));
    let before_grid = root_json(&sink)["children"][0].clone();
    let before_rows = before_grid["children"].as_array().unwrap();
    let before_tile_ids = before_rows
        .iter()
        .flat_map(|row| row["children"].as_array().unwrap())
        .map(|tile| tile["id"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();

    run_density(&mut sink);

    let rows = node_json(&sink, "category-grid")["children"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        rows.iter()
            .map(|row| row["children"].as_array().unwrap().len())
            .collect::<Vec<_>>(),
        [3, 3]
    );
    assert_eq!(rows[0]["children"][0]["id"], before_tile_ids[0]);
    assert_eq!(rows[1]["children"][0]["id"], before_tile_ids[3]);
}

#[test]
fn product_grid_with_images_stays_untouched() {
    let rows = (0..2)
        .map(|index| json!({
            "type": "frame", "id": format!("row-{index}"), "layout": "horizontal",
            "children": (0..3).map(|tile_index| json!({
                "type": "frame", "id": format!("product-{index}-{tile_index}"),
                "layout": "vertical", "children": [
                    {"type": "image", "id": format!("image-{index}-{tile_index}"), "width": 56, "height": 56, "src": "product.png"},
                    {"type": "text", "id": format!("label-{index}-{tile_index}"), "content": "商品", "fontSize": 12}
                ]
            })).collect::<Vec<_>>()
        }))
        .collect::<Vec<_>>();
    let mut sink = sink_with_root(mobile_root(json!([{
        "type": "frame", "id": "products", "name": "商品网格", "layout": "vertical", "children": rows
    }])));
    let before = root_json(&sink);

    run_density(&mut sink);

    assert_eq!(root_json(&sink), before);
}

#[test]
fn bottom_tab_bar_stays_untouched() {
    let rows = (0..2)
        .map(|index| row(&format!("tab-row-{index}"), &["Home", "Search", "Orders"]))
        .collect::<Vec<_>>();
    let root = mobile_root(json!([
        {"type": "frame", "id": "content", "layout": "vertical", "children": []},
        {"type": "frame", "id": "tab-shell", "name": "Tab Bar", "role": "tab-bar",
         "layout": "vertical", "children": rows}
    ]));
    let mut sink = sink_with_root(root);
    let before = root_json(&sink);

    run_density(&mut sink);

    assert_eq!(root_json(&sink), before);
}

#[test]
fn cleanup_driver_reports_density_checkpoint_once() {
    let rows = (0..3)
        .map(|index| row(&format!("row-{index}"), &["美食", "超市", "水果"]))
        .collect();
    let mut test_grid = grid(rows);
    test_grid["gap"] = json!(12);
    let mut sink = sink_with_root(mobile_root(json!([test_grid])));
    let grid_id = root_json(&sink)["children"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let root_id = sink.state.active_children()[0].id_str().to_string();
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

    assert_eq!(
        summary
            .checked()
            .iter()
            .filter(|category| **category == CheckCategory::Structure)
            .count(),
        1,
        "the density checkpoint remains in the single structure category"
    );
    assert!(summary
        .records()
        .iter()
        .any(|record| record.pass == "category-grid-density"));
    assert!(summary
        .records()
        .iter()
        .all(|record| record.pass != "category-grid-reflow"));
    assert!(summary.records().iter().any(|record| {
        record
            .detail
            .starts_with("category-grid-reflow · 分类九宫格 · 3×3 → 5+4")
            && record.detail.contains("numeric tile width(s) dropped")
    }));
    assert_eq!(
        summary
            .records()
            .iter()
            .filter(|record| record.node_id == grid_id)
            .count(),
        1,
        "the reflow is one grid-level repair entry"
    );
}

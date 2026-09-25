//! Grow-to-fit for a clipped fixed-height tile whose label is cut by the
//! clip edge (arena-m01, space-bunny-alpha: the 3x3 category grid's
//! "米饭套餐" label was half-hidden under a 72px `clipContent` tile).

use super::*;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;

fn with_ids(v: &mut serde_json::Value, next: &mut usize) {
    if let Some(obj) = v.as_object_mut() {
        obj.insert("id".into(), json!(format!("t{next}")));
        *next += 1;
        if let Some(children) = obj.get_mut("children").and_then(|c| c.as_array_mut()) {
            for child in children {
                with_ids(child, next);
            }
        }
    }
}

fn run_geometry(mut root: serde_json::Value) -> serde_json::Value {
    with_ids(&mut root, &mut 0);
    let root: PenNode = serde_json::from_value(root).expect("valid root");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state().active_children()[0].id_str().to_string();
    geometry_validate_and_fix(&mut sink, &root_id);
    serde_json::to_value(sink.state().active_children()[0].clone()).unwrap()
}

fn find<'a>(v: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    if v.get("name").and_then(|x| x.as_str()) == Some(name) {
        return Some(v);
    }
    v.get("children")
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
        .find_map(|c| find(c, name))
}

/// The real m01 tile, minimized: 72px tall, `[8, 4]` padding, a 48px dish
/// image + 5px gap + a 13px label = 79px of content in a 56px content box.
fn category_tile(name: &str, label: &str) -> serde_json::Value {
    json!({
        "type": "frame", "name": name, "width": "fill_container", "height": 72,
        "layout": "vertical", "gap": 5, "padding": [8, 4],
        "justifyContent": "center", "alignItems": "center", "clipContent": true,
        "cornerRadius": 12,
        "fill": [{"type": "solid", "color": "#FFFFFF"}],
        "children": [
            {"type": "image", "name": format!("{name} 图"), "width": 48, "height": 48,
             "src": "", "cornerRadius": 8},
            {"type": "text", "name": format!("{name} 标签"), "content": label,
             "fontSize": 13, "fontWeight": 600, "textGrowth": "auto",
             "width": "fit_content", "height": "fit_content"}
        ]
    })
}

fn category_grid() -> serde_json::Value {
    json!({
        "type": "frame", "name": "Home", "width": 375, "height": "fit_content",
        "layout": "vertical", "gap": 12, "padding": [0, 24],
        "children": [{
            "type": "frame", "name": "分类第1行", "width": "fill_container",
            "height": "fit_content", "layout": "horizontal", "gap": 8, "alignItems": "center",
            "children": [
                category_tile("米饭套餐入口", "米饭套餐"),
                category_tile("面食入口", "面食"),
                category_tile("轻食沙拉入口", "轻食沙拉")
            ]
        }]
    })
}

#[test]
fn clipped_tile_grows_until_its_label_clears_the_clip_edge() {
    let v = run_geometry(category_grid());
    let tile = find(&v, "米饭套餐入口").expect("tile survives");
    let height = tile.get("height").and_then(|h| h.as_f64()).unwrap_or(0.0);
    // 8 top + 48 image + 5 gap + label line (~18) + 8 bottom padding.
    assert!(
        height >= 86.0,
        "the clipped tile must grow so its label (and bottom padding) fit, got {height}"
    );
    assert!(
        height <= 90.0,
        "grow only to the content, not beyond it, got {height}"
    );
    assert_eq!(
        tile.get("clipContent").and_then(|c| c.as_bool()),
        Some(true),
        "the authored clip stays — only the cut label is repaired"
    );
}

#[test]
fn clipped_frame_cropping_only_an_image_is_left_alone() {
    // A fixed-height clipped cover whose IMAGE runs past the edge is an
    // intentional crop, not a defect — nothing textual is cut.
    let v = run_geometry(json!({
        "type": "frame", "name": "Home", "width": 375, "height": "fit_content",
        "layout": "vertical",
        "children": [{
            "type": "frame", "name": "Cover", "width": "fill_container", "height": 72,
            "layout": "vertical", "clipContent": true,
            "children": [
                {"type": "image", "name": "Cover Art", "width": "fill_container", "height": 88, "src": ""}
            ]
        }]
    }));
    let cover = find(&v, "Cover").expect("cover survives");
    assert_eq!(cover.get("height").and_then(|h| h.as_f64()), Some(72.0));
}

//! Retained carousel responses must survive the complete shared cleanup.

use crate::cleanup::run_cleanup_passes;
use crate::plan::OrchestratorPlan;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, LayoutPropValue, NodeId, PenNodeExt};
use serde_json::{json, Value};

const SCRIPTS: [&str; 2] = [
    include_str!("../tests/fixtures/carousel-1018/product-hero-attempt1.js"),
    include_str!("../tests/fixtures/carousel-1018/product-hero-attempt2.js"),
];

fn plan() -> OrchestratorPlan {
    serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"商品详情","width":375,"height":812},
        "subtasks":[]
    }))
    .unwrap()
}

fn inserted(script: &str) -> VecDocSink {
    let (nodes, state) = crate::script_gen::parse_script(script).unwrap();
    assert!(state.is_empty());
    let mut root = json!({
        "type":"frame","id":"root","name":"商品详情","width":375,"height":812,
        "layout":"vertical","clipContent":true,"children":[
            {"type":"frame","id":"status","name":"Status Bar","role":"status-bar",
             "width":"fill_container","height":62,"layout":"horizontal","children":[
                {"type":"text","id":"clock","content":"9:41","fontSize":14}
             ]}
        ]
    });
    root["children"]
        .as_array_mut()
        .unwrap()
        .extend(nodes.iter().map(|node| serde_json::to_value(node).unwrap()));
    let mut sink = VecDocSink::new();
    assert!(sink.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![serde_json::from_value(root).unwrap()],
        parent_id: NodeId::NONE,
        page_id: None,
    }));
    sink
}

fn named<'a>(nodes: &'a [PenNode], name: &str) -> &'a PenNode {
    fn find<'a>(node: &'a PenNode, name: &str) -> Option<&'a PenNode> {
        if node.base().name.as_deref() == Some(name) {
            return Some(node);
        }
        node.children()?.iter().find_map(|child| find(child, name))
    }
    nodes.iter().find_map(|node| find(node, name)).unwrap()
}

fn node_json(sink: &VecDocSink, name: &str) -> Value {
    serde_json::to_value(named(sink.state.active_children(), name)).unwrap()
}

fn export_replay(sink: &VecDocSink, file: &str) {
    if let Some(directory) = std::env::var_os("OPENPENCIL_CAROUSEL_REPLAY_DIR") {
        let directory = std::path::PathBuf::from(directory);
        assert!(
            directory.is_absolute(),
            "use an explicit artifact directory"
        );
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(file),
            serde_json::to_vec_pretty(&sink.state.doc).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn retained_responses_keep_full_size_pages_through_whole_cleanup_and_replay() {
    for (attempt, script) in SCRIPTS.iter().enumerate().rev() {
        let mut sink = inserted(script);
        for replay in 0..2 {
            run_cleanup_passes(&mut sink, &plan(), &["root"]);
            let section = node_json(&sink, "商品大图轮播区");
            assert!(
                section.get("padding").is_none(),
                "attempt {attempt}, replay {replay}: {section}"
            );
            let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
            let page = scene.active_page().unwrap();
            let viewport = named(sink.state.active_children(), "轮播视口");
            assert_eq!(page.find(viewport.id_str()).unwrap().bounds.size.x, 375.0);
            let image_names: Vec<String> = if attempt == 0 {
                vec!["轮播图-1-马克杯主图".into()]
            } else {
                (1..=5).map(|index| format!("轮播图-{index}")).collect()
            };
            for name in image_names {
                let image = named(sink.state.active_children(), &name);
                let value = serde_json::to_value(image).unwrap();
                assert_eq!(value["width"], 375.0, "{name}");
                assert_eq!(value["height"], 340.0, "{name}");
                let size = page.find(image.id_str()).unwrap().bounds.size;
                assert_eq!((size.x, size.y), (375.0, 340.0), "{name}");
            }
            if attempt == 1 {
                assert_eq!(node_json(&sink, "轮播条")["width"], 1875.0);
            }
        }
        export_replay(
            &sink,
            &format!("product-hero-response-{}-cleanup.op", attempt + 1),
        );
    }
}

#[test]
fn retained_responses_keep_page_geometry_after_real_image_fallback_and_settle() {
    for (attempt, script) in SCRIPTS.iter().enumerate() {
        for mixed in [false, true] {
            let mut sink = inserted(script);
            run_cleanup_passes(&mut sink, &plan(), &["root"]);
            let names: Vec<String> = if attempt == 0 {
                vec!["轮播图-1-马克杯主图".into()]
            } else {
                (1..=5).map(|index| format!("轮播图-{index}")).collect()
            };
            let ids: Vec<NodeId> = names
                .iter()
                .map(|name| NodeId::new(named(sink.state.active_children(), name).id_str()))
                .collect();
            // The retained final document had two successful images and three
            // real policy tiles. Exercise that mix and complete image failure.
            if mixed && attempt == 1 {
                for index in [0, 2] {
                    assert!(op_image_enrich::apply_result(
                        &mut sink.state,
                        &ids[index],
                        "assets/retained-local-image.webp"
                    ));
                }
            }
            let expected_fallbacks = if mixed && attempt == 1 { 3 } else { ids.len() };
            assert_eq!(
                crate::apply_image_fallback_policy_to_state(&mut sink.state, true),
                expected_fallbacks
            );
            for replay in 0..2 {
                crate::cleanup::settle_mobile_image_geometry(&mut sink, "root");
                run_cleanup_passes(&mut sink, &plan(), &["root"]);
                let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
                let page = scene.active_page().unwrap();
                let viewport = named(sink.state.active_children(), "轮播视口");
                assert_eq!(page.find(viewport.id_str()).unwrap().bounds.size.x, 375.0);
                assert!(node_json(&sink, "商品大图轮播区").get("padding").is_none());
                let mut fallback_count = 0;
                for id in &ids {
                    let node = op_editor_core::walkers::find_node(sink.state.active_children(), id)
                        .unwrap();
                    if matches!(node, PenNode::Frame(_)) {
                        assert!(op_image_enrich::is_unchanged_image_fallback(node));
                        fallback_count += 1;
                    }
                    assert_eq!(
                        (node.width_px(), node.height_px()),
                        (Some(375.0), Some(340.0))
                    );
                    let size = page.find(node.id_str()).unwrap().bounds.size;
                    assert_eq!(
                        (size.x, size.y),
                        (375.0, 340.0),
                        "attempt {attempt}, mixed {mixed}, replay {replay}, {id:?}"
                    );
                }
                assert_eq!(fallback_count, expected_fallbacks);
                if attempt == 1 {
                    assert_eq!(node_json(&sink, "轮播条")["width"], 1875.0);
                }
            }
            export_replay(
                &sink,
                &format!(
                    "product-hero-response-{}-fallback-{}.op",
                    attempt + 1,
                    if mixed { "mixed" } else { "all" }
                ),
            );
        }
    }
}

#[test]
fn authored_section_insets_keep_the_matching_smaller_media_pages() {
    let script = SCRIPTS[1]
        .replace("width:1875", "width:1635")
        .replace("width:375", "width:327");
    let mut sink = inserted(&script);
    let section = NodeId::new(named(sink.state.active_children(), "商品大图轮播区").id_str());
    assert!(sink.apply(EditorCommand::SetNodeLayoutProp {
        node_id: section,
        property: "padding".into(),
        value: LayoutPropValue::NumberArray(vec![0.0, 24.0, 0.0, 24.0]),
    }));
    run_cleanup_passes(&mut sink, &plan(), &["root"]);
    assert_eq!(
        node_json(&sink, "商品大图轮播区")["padding"],
        json!([0.0, 24.0, 0.0, 24.0])
    );
    assert_eq!(node_json(&sink, "轮播条")["width"], 1635.0);
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
    let page = scene.active_page().unwrap();
    for index in 1..=5 {
        let image = named(sink.state.active_children(), &format!("轮播图-{index}"));
        assert_eq!(page.find(image.id_str()).unwrap().bounds.size.x, 327.0);
    }
}

#[test]
fn ordinary_caption_content_still_receives_the_mobile_rail() {
    let mut sink = inserted(SCRIPTS[0]);
    let viewport = NodeId::new(named(sink.state.active_children(), "轮播视口").id_str());
    let caption = serde_json::from_value(json!({
        "type":"text","id":"caption","content":"Ordinary product description",
        "width":240,"height":24,"fontSize":16
    }))
    .unwrap();
    assert!(sink
        .insert_subtree_returning_root_ids(vec![caption], &viewport)
        .is_some());
    crate::mobile_content_rail::repair_mobile_content_rails(&mut sink, "root");
    assert_eq!(
        node_json(&sink, "商品大图轮播区")["padding"],
        json!([0.0, 24.0, 0.0, 24.0])
    );
}

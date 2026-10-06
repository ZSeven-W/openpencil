use super::*;
use crate::cleanup::{run_cleanup_passes_with_summary_and_policy_for_tests, CleanupPolicy};
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::RepairSummary;
use crate::test_support::VecDocSink;
use serde_json::json;

fn work() -> Value {
    let col = |id: &str| {
        json!({"type":"frame","id":id,"layout":"vertical","width":"fill_container","height":"fit_content","gap":16,
        "children":[{"type":"text","id":format!("{id}-body"),"name":"paragraph-body","role":"body-text","content":"普通用户先把资料整理完整，再检查每一页的内容，最后导出确认。保留全部文字。","fontFamily":"Archivo, Noto Serif SC","fontSize":36,"lineHeight":1.7,"textGrowth":"fixed-width","width":"fill_container","height":"fit_content"}]})
    };
    json!({"type":"frame","id":"board","name":"知识卡片","width":1080,"height":1440,"layout":"vertical","padding":80,
        "children":[{"type":"frame","id":"row","layout":"horizontal","width":"fill_container","height":"fit_content","gap":0,
            "children":[col("left"),{"type":"rectangle","id":"rule","width":1,"height":"fill_container","fill":[{"type":"solid","color":"#999999"}]},col("right")]}]})
}

fn sink(tree: Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        jian_ops_schema::load_str(&json!({"version":"1.0.0","children":[tree]}).to_string())
            .unwrap()
            .value,
    );
    sink.state.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("board"),
        patch_json: r#"{"name":"fixture-seed"}"#.into(),
        page_id: None,
    });
    sink.state.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("board"),
        patch_json: r#"{"name":"知识卡片"}"#.into(),
        page_id: None,
    });
    sink
}

fn value(sink: &VecDocSink, id: &str) -> Value {
    serde_json::to_value(
        op_editor_core::walkers::find_node(sink.state.active_children(), &NodeId::new(id)).unwrap(),
    )
    .unwrap()
}

#[test]
fn near_flush_prose_gets_one_em_on_each_side_without_rewriting_the_work() {
    let mut sink = sink(work());
    let before = sink.state.doc.clone();
    assert_eq!(repair_divided_prose_gutters(&mut sink, "board"), 2);
    assert_eq!(
        value(&sink, "left")["padding"],
        json!([0.0, 36.0, 0.0, 0.0])
    );
    assert_eq!(
        value(&sink, "right")["padding"],
        json!([0.0, 0.0, 0.0, 36.0])
    );
    let rects = resolved_rects(&sink.state);
    assert!(rects["rule"].x - (rects["left-body"].x + rects["left-body"].w) >= 35.0);
    assert!(rects["right-body"].x - (rects["rule"].x + rects["rule"].w) >= 35.0);
    let mut restored = sink.state.doc.clone();
    for id in ["left", "right"] {
        let jian_ops_schema::node::PenNode::Frame(frame) =
            op_editor_core::walkers::find_node_mut(&mut restored.children, &NodeId::new(id))
                .unwrap()
        else {
            panic!("column frame")
        };
        frame.container.padding = None;
    }
    // The command bridge canonicalizes absent primitive children to empty arrays.
    // Both representations mean no children; all other properties must match.
    fn canonical(v: &mut Value) {
        if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
            if kids.is_empty() {
                v.as_object_mut().unwrap().remove("children");
            } else {
                for child in kids {
                    canonical(child);
                }
            }
        }
    }
    let mut old = serde_json::to_value(&before).unwrap();
    let mut new = serde_json::to_value(&restored).unwrap();
    canonical(&mut old);
    canonical(&mut new);
    assert_eq!(
        new, old,
        "only padding and empty-children normalization may change"
    );
    let fixed = sink.state.doc.clone();
    assert_eq!(repair_divided_prose_gutters(&mut sink, "board"), 0);
    assert_eq!(sink.state.doc, fixed);
}

#[test]
fn healthy_gaps_existing_insets_and_short_labels_keep_their_composition() {
    for mode in 0..3 {
        let mut tree = work();
        let row = &mut tree["children"][0];
        match mode {
            0 => row["gap"] = json!(24),
            1 => {
                row["children"][0]["padding"] = json!([0, 40, 0, 0]);
                row["children"][2]["padding"] = json!([0, 0, 0, 48]);
            }
            _ => {
                for i in [0, 2] {
                    row["children"][i]["children"][0]["content"] = json!("简短标签");
                }
            }
        }
        let mut sink = sink(tree);
        let before = sink.state.doc.clone();
        assert_eq!(repair_divided_prose_gutters(&mut sink, "board"), 0);
        assert_eq!(sink.state.doc, before);
    }
}

#[test]
fn fixed_painted_rotated_positioned_and_explicit_break_columns_are_protected() {
    for mode in 0..9 {
        let mut tree = work();
        let row = &mut tree["children"][0];
        match mode {
            0 => {
                for i in [0, 2] {
                    row["children"][i]["width"] = json!(400);
                }
            }
            1 => row["rotation"] = json!(12),
            2 => {
                for i in [0, 2] {
                    row["children"][i]["x"] = json!(20);
                }
            }
            3 => {
                for i in [0, 2] {
                    row["children"][i]["fill"] = json!([{"type":"solid","color":"#eeeeee"}]);
                }
            }
            4 => {
                for i in [0, 2] {
                    row["children"][i]["children"][0]["content"] =
                        json!("作者选择的明确分行，不改变布局\n保留这段完整文字和诗意构图。");
                }
            }
            5 => {
                for i in [0, 2] {
                    row["children"][i]["children"][0]["textGrowth"] = json!("auto");
                }
            }
            6 => row["children"][1]["width"] = json!(12),
            7 => {
                for i in [0, 2] {
                    row["children"][i]["maskType"] = json!("alpha");
                }
            }
            _ => {
                for i in [0, 2] {
                    row["children"][i]["opacity"] = json!(0);
                }
            }
        }
        let mut sink = sink(tree);
        let before = sink.state.doc.clone();
        assert_eq!(
            repair_divided_prose_gutters(&mut sink, "board"),
            0,
            "mode {mode}"
        );
        assert_eq!(sink.state.doc, before);
    }
}

#[test]
fn imported_geometry_non_board_and_starved_columns_are_untouched() {
    for mode in 0..5 {
        let mut tree = work();
        if mode == 2 {
            tree["width"] = json!(390);
            tree["height"] = json!(844);
        }
        if mode == 3 {
            tree["padding"] = json!([80, 280]);
        }
        if mode == 4 {
            tree["rotation"] = json!(12);
        }
        let mut sink = sink(tree);
        if mode == 0 {
            sink.state.editor_ui.preserve_authored_geometry = true;
        }
        if mode == 1 {
            sink.state.editor_ui.home.imported_from = Some("https://example.test".into());
        }
        let before = sink.state.doc.clone();
        assert_eq!(
            repair_divided_prose_gutters(&mut sink, "board"),
            0,
            "mode {mode}"
        );
        assert_eq!(sink.state.doc, before);
    }
}

#[test]
fn cleanup_requires_run_output_provenance_and_records_only_accepted_padding_edits() {
    for fresh in [false, true] {
        let mut sink = sink(work());
        let mut summary = RepairSummary::default();
        run_cleanup_passes_with_summary_and_policy_for_tests(
            &mut sink,
            &plan("board"),
            &["board"],
            &mut summary,
            CleanupPolicy {
                roots_are_run_output: fresh,
                ..Default::default()
            },
        );
        let gutters: Vec<_> = summary
            .records()
            .iter()
            .filter(|r| r.pass == "prose-column-gutters")
            .collect();
        assert_eq!(gutters.len(), if fresh { 2 } else { 0 });
        assert!(gutters.iter().all(|r| r.detail.contains("padding")));
    }
}

fn plan(id: &str) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: id.into(),
            name: "图文卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn isolated(test: &str) -> bool {
    const KEY: &str = "OPENPENCIL_PROSE_FONT_TEST";
    if std::env::var(KEY).as_deref() == Ok(test) {
        let fonts =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/shared/fonts");
        jian_skia::register_bundled_fonts(
            [
                "Archivo-VF.ttf",
                "LibreCaslonText-VF.ttf",
                "NotoSerifSC-VF.ttf",
            ]
            .iter()
            .map(|name| std::fs::read(fonts.join(name)).unwrap())
            .collect(),
        );
        return false;
    }
    let full = format!("{}::{test}", module_path!())
        .split_once("::")
        .unwrap()
        .1
        .to_string();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &full, "--nocapture"])
        .env(KEY, test)
        .output()
        .unwrap();
    assert!(
        result.status.success() && String::from_utf8_lossy(&result.stdout).contains("1 passed"),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    true
}

#[test]
fn real_generated_three_page_fixture_reflows_inside_the_same_fixed_boards() {
    if isolated("real_generated_three_page_fixture_reflows_inside_the_same_fixed_boards") {
        return;
    }
    let source = include_str!("test_fixtures/editorial-column-gutters.op");
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(jian_ops_schema::load_str(source).unwrap().value);
    let before = sink.state.doc.clone();
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "n199".into(),
            name: "图文卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    };
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary_and_policy_for_tests(
        &mut sink,
        &plan,
        &["n199", "n3"],
        &mut summary,
        CleanupPolicy {
            preserve_requested_root_height: true,
            roots_are_run_output: true,
            ..Default::default()
        },
    );
    assert!(summary
        .records()
        .iter()
        .any(|r| r.pass == "prose-column-gutters"));
    let rects = resolved_rects(&sink.state);
    for (body, rule) in [("n219", "n220"), ("n182", "n183")] {
        assert!(rects[rule].x - (rects[body].x + rects[body].w) >= 35.0);
    }
    assert_eq!(sink.state.doc.children[0], before.children[0]);
    for id in ["n199", "n3"] {
        assert_eq!(value(&sink, id)["height"], json!(1440.0));
    }
    // Body type and complete copy stay intact. The existing headline-tail pass
    // may make a bounded heading adjustment against the new column measure.
    fn text_nodes(v: &Value, out: &mut Vec<Value>) {
        if v["type"] == "text" {
            out.push(v.clone());
        }
        for child in children(v) {
            text_nodes(child, out);
        }
    }
    let mut old = Vec::new();
    let mut new = Vec::new();
    text_nodes(&serde_json::to_value(&before).unwrap(), &mut old);
    text_nodes(&serde_json::to_value(&sink.state.doc).unwrap(), &mut new);
    assert_eq!(old.len(), new.len());
    for (old, mut new) in old.into_iter().zip(new) {
        let id = old["id"].as_str().unwrap();
        if old["fontSize"] != new["fontSize"] {
            assert!(summary
                .records()
                .iter()
                .any(|r| r.pass == "headline-tail-fit" && r.node_id == id));
            let name = old["name"].as_str().unwrap_or("");
            assert!(!name.contains("body") && old["fontSize"].as_f64().unwrap() >= 36.0);
            assert!(new["fontSize"].as_f64().unwrap() >= old["fontSize"].as_f64().unwrap() * 0.88);
            new["fontSize"] = old["fontSize"].clone();
        }
        assert_eq!(old, new, "text properties for {id}");
    }
    assert!(fixed_board_content_diagnostics(&sink.state).is_empty());
    if let Ok(path) = std::env::var("OPENPENCIL_PROSE_QA_OUT") {
        let output = std::path::Path::new(&path);
        std::fs::create_dir_all(output).unwrap();
        let mut doc = serde_json::to_value(&sink.state.doc).unwrap();
        doc["editorMeta"] = json!({"workFamily":"knowledge"});
        std::fs::write(
            output.join("reading-gutters.op"),
            serde_json::to_string_pretty(&doc).unwrap(),
        )
        .unwrap();
        std::fs::write(
            output.join("repairs.json"),
            serde_json::to_string_pretty(
                &summary
                    .records()
                    .iter()
                    .map(|r| json!({"pass":r.pass,"nodeId":r.node_id,"detail":r.detail}))
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        )
        .unwrap();
    }
}

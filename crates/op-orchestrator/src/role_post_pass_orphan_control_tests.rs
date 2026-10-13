use super::*;

fn toolbar() -> Value {
    json!({"type":"frame","id":"filter","name":"Filter Bar","layout":"vertical","children":[
        {"type":"frame","id":"row","name":"Filter Controls Row","layout":"horizontal","gap":12,"children":[
            {"type":"select","id":"select","name":"Role Select","width":150,"height":40,
             "options":[{"value":"all","label":"All Roles"}],"fill":[{"type":"solid","color":"$--card"}]},
            {"type":"frame","id":"add","name":"Add User Button","width":"fit_content","height":40,
             "layout":"horizontal","gap":8,"padding":[0,16],"cornerRadius":6,
             "fill":[{"type":"solid","color":"$--primary"}],"children":[]},
            {"type":"icon_font","id":"plus","name":"Add User Icon","iconFontFamily":"lucide","iconFontName":"plus","width":16,"height":16},
            {"type":"text","id":"label","name":"Add User Label","content":"Add User","fill":[{"type":"solid","color":"#FFFFFF"}]},
            {"type":"frame","id":"export","name":"Export Icon Button","width":40,"height":40,"cornerRadius":6,
             "fill":[{"type":"solid","color":"$--card"}],"children":[]},
            {"type":"icon_font","id":"download","name":"Export Icon","iconFontFamily":"lucide","iconFontName":"download","width":18,"height":18}
        ]}
    ]})
}

#[test]
fn named_button_parts_are_adopted_before_contrast_and_stub_cleanup() {
    use crate::role_defaults::Theme;
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use op_editor_core::{EditorState, NodeId};

    let mut nodes: Vec<PenNode> = vec![serde_json::from_value(toolbar()).unwrap()];
    crate::role_infer::resolve_forest_roles(&mut nodes, 1440.0, Theme::Light);
    post_pass_forest(&mut nodes, 1440.0);
    crate::tree_heuristics::apply_tree_heuristics(
        &mut nodes,
        Some("#F7F9FC"),
        Theme::Light,
        Some("$--primary"),
    );
    let value = serde_json::to_value(&nodes[0]).unwrap();
    assert!(value.get("fill").is_none_or(|fill| fill == &json!([])));
    let row = &value["children"][0]["children"];
    assert_eq!(row.as_array().unwrap().len(), 3);
    assert_eq!(row[1]["id"], "add");
    assert_eq!(row[1]["children"][0]["id"], "plus");
    assert_eq!(row[1]["children"][1]["id"], "label");
    assert_eq!(row[2]["children"][0]["id"], "download");
    assert_eq!(row[1]["fill"][0]["color"], "$--primary");

    let doc = serde_json::from_value(json!({"version":"1.0.0","children":nodes})).unwrap();
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(doc);
    assert!(!crate::stub_repair::remove_empty_decorated_stubs(
        &mut sink, "filter"
    ));
    assert!(op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new("add")
    )
    .is_some());

    post_pass_forest(&mut nodes, 1440.0);
    assert_eq!(
        serde_json::to_value(&nodes[0]).unwrap(),
        value,
        "second pass must be stable"
    );
}

#[test]
fn orphan_control_adoption_preserves_authored_boundaries() {
    for case in [
        "populated",
        "mismatched",
        "positioned",
        "leaf-positioned",
        "leaf-event",
        "unnamed",
        "vertical",
        "non-button",
    ] {
        let mut node = toolbar()["children"][0].clone();
        node["children"].as_array_mut().unwrap().truncate(4);
        node["children"][1]["role"] = json!("button");
        match case {
            "populated" => {
                node["children"][1]["children"] = json!([{"type":"text","content":"Existing"}])
            }
            "mismatched" => node["children"][2]["name"] = json!("Other Icon"),
            "positioned" => node["children"][1]["x"] = json!(0),
            "leaf-positioned" => node["children"][2]["x"] = json!(0),
            "leaf-event" => node["children"][2]["events"] = json!({"onClick":[]}),
            "unnamed" => node["children"][1]["name"] = json!("Button"),
            "vertical" => node["layout"] = json!("vertical"),
            "non-button" => node["children"][1]["role"] = json!("card"),
            _ => unreachable!(),
        }
        let before = node.clone();
        adopt_orphan_button_parts(&mut node, &[]);
        assert_eq!(node, before, "{case}");
    }
}

fn subscription() -> Value {
    json!({"type":"frame","id":"band","name":"Subscription Band","width":1120,
        "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],"children":[
        {"type":"frame","id":"form","name":"Subscribe Form","width":520,"height":48,
         "layout":"horizontal","gap":0,"alignItems":"center",
         "fill":[{"type":"solid","color":"#F5F4F0"}],"children":[
            {"type":"text_input","id":"input","name":"Email Input","width":"fill_container",
             "height":46,"placeholder":"请输入您的邮箱地址"},
            {"type":"frame","id":"subscribe","name":"Subscribe Button","width":120,"height":46,
             "layout":"horizontal","alignItems":"center","justifyContent":"center",
             "fill":[{"type":"solid","color":"#1A1A1A"}],"cornerRadius":0,"children":[]},
            {"type":"text","id":"label","name":"Button Label","content":"订阅","fontSize":14,
             "fill":[{"type":"solid","color":"#FFFFFF"}]}
        ]}
    ]})
}

#[test]
fn generic_button_label_is_reparented_before_ancestor_contrast() {
    let mut nodes: Vec<PenNode> = vec![serde_json::from_value(subscription()).unwrap()];
    crate::role_infer::resolve_forest_roles(&mut nodes, 1440.0, crate::role_defaults::Theme::Light);
    post_pass_forest(&mut nodes, 1440.0);
    let value = serde_json::to_value(&nodes[0]).unwrap();
    let row = &value["children"][0]["children"];
    assert_eq!(row.as_array().unwrap().len(), 2);
    assert_eq!(row[1]["id"], "subscribe");
    assert_eq!(row[1]["children"][0]["id"], "label");
    assert_eq!(row[1]["children"][0]["fill"][0]["color"], "#FFFFFF");
    post_pass_forest(&mut nodes, 1440.0);
    assert_eq!(serde_json::to_value(&nodes[0]).unwrap(), value);
}

#[test]
fn generic_button_label_adoption_requires_unambiguous_compact_control() {
    for case in [
        "note",
        "unnamed",
        "wide",
        "tall",
        "positioned",
        "interactive",
        "long-label",
        "multiline",
        "padded",
    ] {
        let mut node = subscription()["children"][0].clone();
        node["children"][1]["role"] = json!("button");
        match case {
            "note" => node["children"][2]["name"] = json!("Privacy Note"),
            "unnamed" => node["children"][1]["name"] = json!("Button"),
            "wide" => node["children"][1]["width"] = json!(600),
            "tall" => node["children"][1]["height"] = json!(120),
            "positioned" => node["children"][2]["x"] = json!(0),
            "interactive" => node["children"][2]["events"] = json!({"onClick":[]}),
            "long-label" => {
                node["children"][2]["content"] = json!("这是一个并不属于按钮的很长说明文字")
            }
            "multiline" => node["children"][2]["content"] = json!("订阅\n说明"),
            "padded" => node["children"][1]["padding"] = json!([0, 50]),
            _ => unreachable!(),
        }
        let before = node.clone();
        repair_orphan_controls(&mut node);
        assert_eq!(node, before, "{case}");
    }
}

fn duplicated_cta() -> Value {
    let mut empty = json!({"type":"frame","id":"shell","name":"cta-button","width":180,"height":44,
        "layout":"horizontal","alignItems":"center","justifyContent":"center","gap":8,
        "cornerRadius":0,"fill":[{"type":"solid","color":"#FFFFFF00"}],
        "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#FFFFFF"}]},"children":[]});
    let mut inner = empty.clone();
    inner["id"] = json!("inner");
    inner["name"] = json!("cta-inner");
    inner["children"] = json!([
        {"type":"text","id":"label","name":"CTA Label","content":"探索系列","fontSize":14,
         "fill":[{"type":"solid","color":"#FFFFFF"}]},
        {"type":"icon_font","id":"arrow","name":"CTA Arrow","iconFontFamily":"lucide",
         "iconFontName":"arrow-right","width":16,"height":16,"fill":[{"type":"solid","color":"#FFFFFF"}]}
    ]);
    empty["role"] = json!("button");
    inner["role"] = json!("button");
    json!({"type":"frame","id":"wrap","name":"CTA Wrap","width":1120,"height":44,
        "layout":"horizontal","justifyContent":"center","alignItems":"center","children":[empty,inner]})
}

#[test]
fn duplicated_empty_cta_shell_leaves_one_centered_populated_button() {
    let mut nodes: Vec<PenNode> = vec![serde_json::from_value(duplicated_cta()).unwrap()];
    crate::role_infer::resolve_forest_roles(&mut nodes, 1440.0, crate::role_defaults::Theme::Light);
    post_pass_forest(&mut nodes, 1440.0);
    let value = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(value["children"].as_array().unwrap().len(), 1);
    assert_eq!(value["children"][0]["id"], "shell");
    assert_eq!(
        value["children"][0]["children"].as_array().unwrap().len(),
        2
    );
    assert_eq!(value["justifyContent"], "center");
    assert_eq!(value["children"][0]["width"], 180.0);
    post_pass_forest(&mut nodes, 1440.0);
    assert_eq!(serde_json::to_value(&nodes[0]).unwrap(), value);
}

#[test]
fn duplicate_shell_repair_keeps_referenced_inner_identity() {
    for reference in [
        json!({"events":{"onClick":[{"type":"scrollTo","target":"inner"}]}}),
        json!({"bindings":{"opacity":"$node.inner.opacity"}}),
        json!({"type":"ref","ref":"inner"}),
    ] {
        let mut node =
            json!({"type":"frame","layout":"vertical","children":[duplicated_cta(),reference]});
        let before = node.clone();
        repair_orphan_controls(&mut node);
        assert_eq!(node, before);
    }
}

#[test]
fn duplicate_shell_repair_preserves_intentional_siblings() {
    for case in [
        "populated",
        "same-name",
        "different-fill",
        "different-border",
        "different-size",
        "different-role",
        "different-rotation",
        "different-name",
        "image-fill",
        "positioned",
        "interactive",
        "bindings",
        "hidden",
        "vertical",
    ] {
        let mut node = duplicated_cta();
        match case {
            "populated" => {
                node["children"][0]["children"] = json!([{"type":"text","content":"了解品牌"}])
            }
            "same-name" => node["children"][1]["name"] = json!("cta-button"),
            "different-fill" => node["children"][0]["fill"][0]["color"] = json!("#000000"),
            "different-border" => node["children"][0]["stroke"]["thickness"] = json!(2),
            "different-size" => node["children"][0]["width"] = json!(240),
            "different-role" => node["children"][0]["role"] = json!("card"),
            "different-rotation" => node["children"][0]["rotation"] = json!(30),
            "different-name" => node["children"][1]["name"] = json!("other-inner"),
            "image-fill" => {
                for child in node["children"].as_array_mut().unwrap() {
                    child["fill"] =
                        json!([{"type":"image","url":"https://example.com/button.png"}]);
                }
            }
            "positioned" => node["children"][0]["x"] = json!(0),
            "interactive" => node["children"][0]["events"] = json!({"onClick":[]}),
            "bindings" => node["children"][0]["bindings"] = json!({"opacity":"$scroll.progress"}),
            "hidden" => node["children"][0]["visible"] = json!(false),
            "vertical" => node["layout"] = json!("vertical"),
            _ => unreachable!(),
        }
        let before = node.clone();
        repair_orphan_controls(&mut node);
        assert_eq!(node, before, "{case}");
    }
}

#[test]
#[ignore = "requires a retained document with restored empty control shells and QA paths"]
fn replay_retained_toolbar_parts() {
    use crate::role_defaults::Theme;
    use op_editor_core::PenNodeExt;

    let input = std::env::var("OPENPENCIL_QA_TOOLBAR_INPUT").unwrap();
    let output = std::env::var("OPENPENCIL_QA_TOOLBAR_OUTPUT").unwrap();
    let id = std::env::var("OPENPENCIL_QA_TOOLBAR_ID").unwrap();
    let source = std::fs::read_to_string(input).unwrap();
    let mut doc = op_pen_loader::payload::load_canonical(&source)
        .unwrap()
        .value;
    fn repair(nodes: &mut [PenNode], id: &str) -> bool {
        for node in nodes {
            if node.id_str() == id {
                let nodes = std::slice::from_mut(node);
                crate::role_infer::resolve_forest_roles(nodes, 1440.0, Theme::Light);
                post_pass_forest(nodes, 1440.0);
                crate::tree_heuristics::apply_tree_heuristics(
                    nodes,
                    Some("#F7F9FC"),
                    Theme::Light,
                    Some("$--primary"),
                );
                return true;
            }
            if node
                .children_mut()
                .is_some_and(|children| repair(children, id))
            {
                return true;
            }
        }
        false
    }
    assert!(repair(&mut doc.children, &id));
    std::fs::write(output, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
}

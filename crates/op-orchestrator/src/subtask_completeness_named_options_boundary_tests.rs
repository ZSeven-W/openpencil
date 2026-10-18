use super::named_options_tests::{outcome, pill, section, sink, task, ELEMENTS};
use super::*;
use serde_json::{json, Value};

fn choices() -> Vec<Value> {
    ["月白", "燕麦", "苔绿"]
        .iter()
        .enumerate()
        .map(|(i, label)| pill(&format!("pill-{i}"), label))
        .collect()
}
fn delivered(roots: Vec<Value>, elements: &str) -> usize {
    match named_options::inspect(&sink(roots), &task(elements), &outcome(&["section"])) {
        Ok(n) => {
            assert_eq!(n, 3, "static boundary fixture must not silently defer");
            n
        }
        Err(failure) => {
            assert_eq!(failure.expected, 3);
            failure.delivered
        }
    }
}

#[test]
fn static_labeled_pills_require_no_events_or_state() {
    assert_eq!(delivered(vec![section(choices())], ELEMENTS), 3);
    assert_eq!(
        delivered(vec![section(choices())], "3 个规格胶囊（月白/燕麦/苔绿）"),
        3
    );
    assert_eq!(
        delivered(vec![section(choices())], "3 color options (月白/燕麦/苔绿)"),
        3
    );
    assert_eq!(
        delivered(
            vec![section(choices())],
            "3 个规格胶囊（「月白」/「燕麦」/「苔绿」）"
        ),
        3
    );
}

#[test]
fn unnamed_foreign_quoted_example_and_graphic_only_plans_do_not_create_contracts() {
    for elements in [
        "3 个规格胶囊",
        "颜色和尺码选择",
        "3 个标签（月白/燕麦/苔绿）",
        "3 个规格胶囊（月白/燕麦）",
        "3 个规格胶囊（月白/月白/苔绿）",
        "3 个规格胶囊（例如月白/燕麦/苔绿）",
        "标题文案「3 个规格胶囊（月白/燕麦/苔绿）」",
        "其他区域包含 3 个规格胶囊（月白/燕麦/苔绿）",
        "3 个规格胶囊（月白/燕麦/苔绿），纯色块无文字标签",
        "3 color options (white/oat/green), no labels",
        "3 color options (for example Red / Blue / Green)",
        "3 color options (Such As Red / Blue / Green)",
        "3 个颜色选项（红/白/黑），不带文字",
        "3 个颜色选项（红/白/黑），由其他区块负责；本区展示3个数量按钮",
        "3 个颜色选项（红、白、黑），由其他区块负责；本区展示3个数量按钮",
        "3 color options (Red / Blue / Green), owned by another section; show 3 items",
    ] {
        assert_eq!(
            named_options::inspect(
                &sink(vec![section(vec![])]),
                &task(elements),
                &outcome(&["section"])
            )
            .unwrap(),
            0,
            "{elements}"
        );
    }
    let mut label_only = task("保持现有内容");
    label_only.label = "3 个规格胶囊（月白/燕麦/苔绿）".into();
    assert_eq!(
        named_options::inspect(
            &sink(vec![section(vec![])]),
            &label_only,
            &outcome(&["section"])
        )
        .unwrap(),
        0
    );
}

#[test]
fn unrelated_steps_empty_frames_node_names_and_action_strings_do_not_deliver_labels() {
    let mut options = choices();
    options.pop();
    options.extend([
        json!({"type":"frame","id":"empty","name":"苔绿","events":{"onTap":[{"set":{"$app.color":"'苔绿'"}}]}}),
        pill("minus", "−"), pill("qty", "3"), pill("plus", "+"),
    ]);
    assert_eq!(delivered(vec![section(options)], ELEMENTS), 2);
}

#[test]
fn duplicated_combined_and_nested_metadata_labels_cannot_make_three_options() {
    assert_eq!(
        delivered(
            vec![section(vec![
                pill("a", "月白"),
                pill("b", "月白"),
                pill("c", "月白")
            ])],
            ELEMENTS
        ),
        1
    );
    assert_eq!(
        delivered(vec![section(vec![pill("a", "月白/燕麦/苔绿")])], ELEMENTS),
        0
    );
    let metadata = json!({"type":"frame","id":"card","events":{"onTap":[{"set":{"$app.selected":"true"}}]},"children":choices()});
    assert_eq!(delivered(vec![section(vec![metadata])], ELEMENTS), 0);
    let mut multi_text = pill("one", "月白");
    multi_text["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"text","id":"business","content":"热销"}));
    assert_eq!(
        delivered(
            vec![section(vec![
                multi_text,
                pill("b", "燕麦"),
                pill("c", "苔绿")
            ])],
            ELEMENTS
        ),
        2
    );
}

#[test]
fn independent_parents_and_distant_headings_cannot_pool_delivery() {
    let nodes = choices();
    let split = vec![
        json!({"type":"frame","id":"left","children":nodes[..2]}),
        json!({"type":"frame","id":"right","children":nodes[2..]}),
    ];
    // Only the right wrapper has one visible label; the left has two and is
    // not one capsule. They cannot pool three labels across actual parents.
    assert_eq!(delivered(vec![section(split)], ELEMENTS), 1);
    assert_eq!(
        delivered(
            vec![section(split_choices())],
            "3 个规格胶囊（月白/燕麦/苔绿）"
        ),
        2
    );
    let mut distant = section(vec![]);
    distant["children"].as_array_mut().unwrap().push(json!({"type":"frame","id":"unrelated","children":[{"type":"text","id":"other-heading","content":"营销标签"},{"type":"frame","id":"tags","children":choices()}]}));
    assert_eq!(delivered(vec![distant], ELEMENTS), 0);
}
fn split_choices() -> Vec<Value> {
    let nodes = choices();
    vec![
        json!({"type":"frame","id":"left","children":nodes[..2]}),
        json!({"type":"frame","id":"right","children":nodes[2..]}),
    ]
}

#[test]
fn hidden_labels_and_hidden_owned_ancestors_are_not_visible_delivery() {
    for property in ["visible", "enabled", "opacity"] {
        let mut options = choices();
        options[2]["children"][0][property] = if property == "opacity" {
            json!(0)
        } else {
            json!(false)
        };
        assert_eq!(delivered(vec![section(options)], ELEMENTS), 2, "{property}");
    }
    let hidden_ancestor =
        json!({"type":"frame","id":"outer","visible":false,"children":[section(choices())]});
    assert_eq!(delivered(vec![hidden_ancestor], ELEMENTS), 0);
    let mut external = section(choices());
    external["id"] = json!("foreign");
    assert_eq!(delivered(vec![section(vec![]), external], ELEMENTS), 0);
}

#[test]
fn styled_text_and_resolved_design_variables_keep_exact_literal_names() {
    let mut options = choices();
    options[0]["children"][0]["content"] = json!([{"text":"月"},{"text":"白"}]);
    options[1]["children"][0]["content"] = json!("$oat");
    let mut sink = sink(vec![section(options)]);
    sink.state.doc.variables =
        Some(serde_json::from_value(json!({"oat":{"type":"string","value":"燕麦"}})).unwrap());
    assert_eq!(
        named_options::inspect(&sink, &task(ELEMENTS), &outcome(&["section"])).unwrap(),
        3
    );
}

#[test]
fn native_option_widgets_preserve_explicit_names() {
    for kind in ["radio_group", "select", "tabs"] {
        let property = if kind == "tabs" { "tabs" } else { "options" };
        let widget = json!({"type":kind,"id":"native",property:[
            {"value":"white","label":"月白"},{"value":"oat","label":"燕麦"},{"value":"green","label":"苔绿"}
        ]});
        assert_eq!(
            delivered(vec![section(vec![widget])], ELEMENTS),
            3,
            "{kind}"
        );
    }
}

#[test]
fn reference_overrides_are_resolved_and_unused_raw_slots_never_count() {
    let master = json!({"type":"frame","id":"master","reusable":true,"children":[{"type":"text","id":"master-label","content":"月白"}]});
    let refs = ["月白","燕麦","苔绿"].iter().enumerate().map(|(i,label)| json!({"type":"ref","id":format!("ref-{i}"),"ref":"master","descendants":{"master-label":{"content":label}}})).collect();
    let mut roots = vec![master, section(refs)];
    assert_eq!(delivered(roots.clone(), ELEMENTS), 3);
    roots[1]["children"][0]["children"][1]["children"][2]["descendants"]["master-label"]
        ["visible"] = json!(false);
    assert_eq!(delivered(roots, ELEMENTS), 2);
    let full_master = json!({"type":"frame","id":"master","reusable":true,"children":[{"type":"text","id":"existing","content":"existing"}]});
    let slots = json!({"type":"ref","id":"fake","ref":"master","children":choices()});
    assert_eq!(
        delivered(vec![full_master, section(vec![slots])], ELEMENTS),
        0
    );
}

#[test]
fn a_shared_parent_tap_handler_does_not_hide_independent_static_choices() {
    let mut root = section(choices());
    root["events"] = json!({"onTap":[{"set":{"$app.visited":"true"}}]});
    root["children"][0]["children"][1]["events"] = root["events"].clone();
    assert_eq!(delivered(vec![root], ELEMENTS), 3);
}

#[test]
fn dynamic_content_and_visibility_defer_without_claiming_static_name_delivery() {
    for empty in [false, true] {
        let mut options = choices();
        options[2]["children"][0]["bindings"] = json!({"content":"$app.optionLabel"});
        if empty {
            options[2]["children"][0]["content"] = json!("");
        }
        assert_eq!(
            named_options::inspect(
                &sink(vec![section(options)]),
                &task(ELEMENTS),
                &outcome(&["section"])
            )
            .unwrap(),
            0
        );
    }
    let mut root = section(choices());
    root["bindings"] = json!({"visible":"$app.showColors"});
    assert_eq!(
        named_options::inspect(&sink(vec![root]), &task(ELEMENTS), &outcome(&["section"])).unwrap(),
        0
    );
}

#[test]
fn adjacent_media_cards_cannot_promote_nested_metadata_into_options() {
    let mut metadata = choices();
    metadata.insert(0, json!({"type":"image","id":"photo","src":"product.png"}));
    let root = json!({"type":"frame","id":"section","children":[
        {"type":"text","id":"heading","content":"颜色"},
        {"type":"frame","id":"product-card","children":metadata}
    ]});
    assert_eq!(delivered(vec![root], ELEMENTS), 0);
}

#[test]
fn a_bound_group_heading_defers_when_adjacent_named_options_are_complete() {
    let mut root = section(choices());
    root["children"][0]["children"][0]["content"] = json!("");
    root["children"][0]["children"][0]["bindings"] = json!({"content":"$app.groupLabel"});
    let mut sink = sink(vec![root]);
    sink.state.doc.state = Some(
        serde_json::from_value(json!({"groupLabel":{"type":"string","default":"颜色"}})).unwrap(),
    );
    assert_eq!(
        named_options::inspect(&sink, &task(ELEMENTS), &outcome(&["section"])).unwrap(),
        0
    );
}

#[test]
fn unknown_tab_panel_visibility_defers_instead_of_rejecting_nested_options() {
    let tabs = json!({"type":"tabs","id":"tabs","tabs":[{"value":"style","label":"款式"},{"value":"detail","label":"详情"}],"value":"style","children":[section(choices())]});
    for roots in [vec!["tabs"], vec!["section"]] {
        assert_eq!(
            named_options::inspect(&sink(vec![tabs.clone()]), &task(ELEMENTS), &outcome(&roots))
                .unwrap(),
            0
        );
    }
}

#[test]
fn unrelated_tab_panels_cannot_disable_a_missing_named_option_contract() {
    let mut root = section(vec![]);
    root["children"].as_array_mut().unwrap().push(json!({"type":"tabs","id":"tabs","tabs":[{"value":"description","label":"详情"}],"children":[{"type":"text","id":"intro","content":"商品介绍"}]}));
    assert_eq!(delivered(vec![root], ELEMENTS), 0);
}

#[test]
fn explicit_local_postposed_ownership_keeps_the_named_contract() {
    let elements = "3 个颜色选项（月白、燕麦、苔绿），由商品规格选择负责";
    assert_eq!(delivered(vec![section(choices())], elements), 3);
    assert_eq!(delivered(vec![section(vec![])], elements), 0);
}

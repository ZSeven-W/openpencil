use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

const LOCAL_LIST: &str = "商家详情(头图+菜品分类横滑+菜品列表六条+购物车条) — build exactly what this names, with every count it states; do not add content that belongs to the brief's other sections";

#[derive(Clone, Copy, Debug)]
enum Shell {
    WrappedMediaInlineControl,
    DirectMediaTrailingControl,
}

const SHELLS: [Shell; 2] = [
    Shell::WrappedMediaInlineControl,
    Shell::DirectMediaTrailingControl,
];

fn task(elements: &str) -> Subtask {
    serde_json::from_value(json!({
        "id":"section", "label":"Detail", "elements":elements,
        "region":{"width":375,"height":812}
    }))
    .unwrap()
}

fn outcome(ids: &[&str]) -> SubtaskOutcome {
    SubtaskOutcome {
        id: "section".into(),
        node_count: 1,
        inserted_root_ids: ids.iter().map(|id| (*id).into()).collect(),
        error: None,
        headline: None,
        subtask: None,
    }
}

fn roots(children: Vec<Value>) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0", "children":children})).unwrap(),
    );
    sink
}

fn frame(id: &str, layout: &str, children: Vec<Value>) -> Value {
    json!({"type":"frame", "id":id, "layout":layout, "children":children})
}

fn section(children: Vec<Value>) -> VecDocSink {
    roots(vec![frame("section", "vertical", children)])
}

fn text(id: &str, content: &str) -> Value {
    json!({"type":"text", "id":id, "content":content})
}

fn image(id: &str) -> Value {
    json!({"type":"image", "id":id, "src":"dish.png", "width":88,"height":88})
}

fn row(index: usize, old_price: bool, shell: Shell) -> Value {
    let mut prices = vec![text(&format!("price-{index}"), "¥26")];
    if old_price {
        prices.push(text(&format!("old-price-{index}"), "¥32"));
    }
    let price = frame(&format!("prices-{index}"), "horizontal", prices);
    let control = frame(
        &format!("control-{index}"),
        "horizontal",
        vec![json!({"type":"icon_font", "id":format!("plus-{index}"),
            "iconFontFamily":"lucide", "iconFontName":"plus", "width":20,"height":20})],
    );
    let mut info = vec![
        text(&format!("title-{index}"), &format!("Dish {index}")),
        text(&format!("description-{index}"), "Freshly prepared"),
        text(&format!("sales-{index}"), "Monthly orders 128"),
    ];
    let children = match shell {
        Shell::WrappedMediaInlineControl => {
            info.push(frame(
                &format!("purchase-{index}"),
                "horizontal",
                vec![price, control],
            ));
            vec![
                frame(
                    &format!("media-{index}"),
                    "horizontal",
                    vec![image(&format!("image-{index}"))],
                ),
                frame(&format!("info-{index}"), "vertical", info),
            ]
        }
        Shell::DirectMediaTrailingControl => {
            info.push(price);
            vec![
                image(&format!("image-{index}")),
                frame(&format!("info-{index}"), "vertical", info),
                frame(&format!("alignment-{index}"), "vertical", vec![control]),
            ]
        }
    };
    frame(&format!("row-{index}"), "horizontal", children)
}

fn rows(count: usize, shell: Shell) -> Vec<Value> {
    (0..count)
        .map(|index| row(index, index == 0, shell))
        .collect()
}

fn find_mut<'a>(node: &'a mut Value, id: &str) -> Option<&'a mut Value> {
    if node["id"].as_str() == Some(id) {
        return Some(node);
    }
    for child in node.get_mut("children")?.as_array_mut()? {
        if let Some(found) = find_mut(child, id) {
            return Some(found);
        }
    }
    None
}

fn node_mut<'a>(node: &'a mut Value, id: &str) -> &'a mut Value {
    find_mut(node, id).expect("fixture node exists")
}

fn proven(sink: &VecDocSink, ids: &[&str]) -> usize {
    list_rows::delivered_list_row_count(sink, &outcome(ids).inserted_root_ids)
}

fn assert_short(sink: &VecDocSink, count: usize) {
    assert_eq!(proven(sink, &["section"]), count);
    let failure = incomplete_attempt(sink, &task(LOCAL_LIST), &outcome(&["section"]))
        .expect("the local six-row list is still incomplete");
    assert_eq!((failure.expected, failure.delivered), (6, count));
}

#[test]
fn local_postfix_list_counts_require_the_same_expected_count() {
    for elements in [LOCAL_LIST, "菜品列表6条", "商品清单共六项", "list of six"] {
        let task = task(elements);
        assert_eq!(expected_item_count(&task), Some(6), "{elements}");
        let local = local_item_text(&task);
        assert!(list_rows::has_local_list_contract(&local, 6), "{elements}");
        assert!(!list_rows::has_local_list_contract(&local, 5), "{elements}");
    }
    assert!(!media_items::has_local_media_card_contract(
        &local_item_text(&task(LOCAL_LIST)),
        6
    ));
}

#[test]
fn quoted_nonlocal_and_metadata_counts_do_not_enable_list_evidence() {
    let sink = section(rows(6, Shell::DirectMediaTrailingControl));
    assert_eq!(proven(&sink, &["section"]), 6);
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 5);
    for elements in [
        "标题『菜品列表六条』；本区6个控件",
        "Title 'list of six'; show 6 items",
        "Another section owns a list of six; show 6 items",
        "菜品列表六条由其他页面负责；本区6个控件",
        "metadata (6 条); show 6 items",
        "元信息（六条）；本区6个控件",
        "6 items; list of three",
    ] {
        let task = task(elements);
        assert_eq!(expected_item_count(&task), Some(6), "{elements}");
        assert!(
            !list_rows::has_local_list_contract(&local_item_text(&task), 6),
            "{elements}"
        );
        let failure = incomplete_attempt(&sink, &task, &outcome(&["section"]))
            .expect("foreign or display-copy lists cannot enable the additive count");
        assert_eq!((failure.expected, failure.delivered), (6, 5), "{elements}");
    }
}

#[test]
fn both_retained_shell_shapes_merge_only_the_optional_old_price() {
    for shell in SHELLS {
        let sink = section(rows(6, shell));
        assert_eq!(
            delivered_item_count(&sink, &["section".into()]),
            5,
            "{shell:?}"
        );
        assert_eq!(proven(&sink, &["section"]), 6, "{shell:?}");
        assert!(incomplete_attempt(&sink, &task(LOCAL_LIST), &outcome(&["section"])).is_none());
        assert_short(&section(rows(5, shell)), 5);
    }
}

#[test]
fn expected_count_does_not_select_or_resize_the_proven_family() {
    for shell in SHELLS {
        let sink = section(rows(6, shell));
        for expected in [5, 6, 7] {
            let task = task(&format!("菜品列表{expected}条"));
            assert_eq!(expected_item_count(&task), Some(expected));
            assert!(list_rows::has_local_list_contract(
                &local_item_text(&task),
                expected
            ));
            assert_eq!(proven(&sink, &["section"]), 6);
            let result = incomplete_attempt(&sink, &task, &outcome(&["section"]));
            if expected == 7 {
                let failure = result.expect("six actual rows do not satisfy seven");
                assert_eq!((failure.expected, failure.delivered), (7, 6));
            } else {
                assert!(result.is_none());
            }
        }
    }
}

#[test]
fn five_rows_plus_empty_chips_promos_and_gallery_stay_five() {
    for shell in SHELLS {
        let mut children = rows(5, shell);
        let mut promo = row(20, false, shell);
        node_mut(&mut promo, "plus-20")["iconFontName"] = json!("tag");
        children.extend([
            promo,
            frame("empty", "horizontal", vec![]),
            frame(
                "category",
                "horizontal",
                vec![text("category-label", "Popular")],
            ),
            frame(
                "gallery",
                "horizontal",
                vec![image("gallery-a"), image("gallery-b")],
            ),
        ]);
        assert_short(&section(children), 5);
    }
}

#[test]
fn nested_nonpriced_cards_and_gallery_do_not_supply_additional_row_evidence() {
    for gallery in [false, true] {
        let nested = (10..16)
            .map(|index| {
                if gallery {
                    image(&format!("gallery-{index}"))
                } else {
                    frame(
                        &format!("promo-{index}"),
                        "horizontal",
                        vec![
                            image(&format!("promo-image-{index}")),
                            text(&format!("promo-text-{index}"), "Offer"),
                        ],
                    )
                }
            })
            .collect();
        let mut children = rows(5, Shell::DirectMediaTrailingControl);
        children.push(frame(
            "nested-card",
            "vertical",
            vec![frame("nested-list", "horizontal", nested)],
        ));
        let sink = section(children);
        assert_eq!(proven(&sink, &["section"]), 5);
        // Six nested generic siblings already passed the old counter. The new
        // evidence must not be mistaken for a repair of that separate limitation.
        assert_eq!(delivered_item_count(&sink, &["section".into()]), 6);
        assert!(incomplete_attempt(&sink, &task(LOCAL_LIST), &outcome(&["section"])).is_none());
    }
}

#[test]
fn a_media_card_cannot_lend_its_nested_complete_rows_to_the_outer_list() {
    for wrapped in [false, true] {
        let media = if wrapped {
            frame("outer-media", "horizontal", vec![image("outer-image")])
        } else {
            image("outer-image")
        };
        let nested = (10..16).map(|index| row(index, false, SHELLS[1])).collect();
        let outer = frame(
            "outer-card",
            "horizontal",
            vec![media, frame("inner-list", "vertical", nested)],
        );
        assert_eq!(proven(&section(vec![outer.clone()]), &["section"]), 0);
        let mut children = rows(5, SHELLS[1]);
        children.push(outer);
        let sink = section(children);
        assert_eq!(proven(&sink, &["section"]), 5);
        assert_eq!(delivered_item_count(&sink, &["section".into()]), 6);
        assert!(incomplete_attempt(&sink, &task(LOCAL_LIST), &outcome(&["section"])).is_none());
    }
}

#[test]
fn six_category_chips_or_tag_promos_keep_the_legacy_generic_false_positive() {
    for promos in [false, true] {
        let mut children = vec![row(0, true, Shell::DirectMediaTrailingControl)];
        children.extend((10..16).map(|index| {
            if promos {
                let mut promo = row(index, false, Shell::DirectMediaTrailingControl);
                node_mut(&mut promo, &format!("plus-{index}"))["iconFontName"] = json!("tag");
                promo
            } else {
                frame(
                    &format!("chip-{index}"),
                    "horizontal",
                    vec![text(&format!("chip-text-{index}"), "Category")],
                )
            }
        }));
        let sink = section(children);
        assert_eq!(proven(&sink, &["section"]), 1);
        assert_eq!(delivered_item_count(&sink, &["section".into()]), 6);
        assert!(incomplete_attempt(&sink, &task(LOCAL_LIST), &outcome(&["section"])).is_none());
    }
}

#[test]
fn different_actual_parents_never_merge_even_when_all_row_ids_are_inserted_roots() {
    let sink = roots(vec![
        frame(
            "list-a",
            "vertical",
            (0..3).map(|index| row(index, false, SHELLS[1])).collect(),
        ),
        frame(
            "list-b",
            "vertical",
            (3..6).map(|index| row(index, false, SHELLS[1])).collect(),
        ),
    ]);
    assert_eq!(proven(&sink, &["list-a", "list-b"]), 3);
    let ids = ["row-0", "row-1", "row-2", "row-3", "row-4", "row-5"];
    assert_eq!(proven(&sink, &ids), 3);
    // The generic root-list behavior is intentionally not changed by this patch.
    assert_eq!(
        delivered_item_count(&sink, &outcome(&ids).inserted_root_ids),
        6
    );
    assert!(incomplete_attempt(&sink, &task(LOCAL_LIST), &outcome(&ids)).is_none());
}

#[test]
fn distinct_shells_slot_order_and_metadata_do_not_merge_into_six() {
    let mut children = (0..3)
        .map(|index| row(index, false, SHELLS[0]))
        .collect::<Vec<_>>();
    children.extend((3..6).map(|index| row(index, false, SHELLS[1])));
    assert_short(&section(children), 3);

    for metadata in [false, true] {
        let mut children = rows(5, SHELLS[1]);
        let mut different = row(10, false, SHELLS[1]);
        if metadata {
            node_mut(&mut different, "prices-10")["children"]
                .as_array_mut()
                .unwrap()
                .push(text("price-note", "Member price"));
        } else {
            different["children"].as_array_mut().unwrap().swap(0, 1);
        }
        children.push(different);
        assert_short(&section(children), 5);
    }
}

#[test]
fn media_wrappers_with_extra_hidden_or_recursive_children_are_not_image_slots() {
    for invalid in 0..4 {
        let mut value = row(0, false, SHELLS[0]);
        let media = node_mut(&mut value, "media-0");
        match invalid {
            0 => media["children"]
                .as_array_mut()
                .unwrap()
                .push(image("second-image")),
            1 => {
                media["children"] = json!([frame(
                    "extra-wrapper",
                    "horizontal",
                    vec![image("wrapped-image")]
                )])
            }
            2 => media["children"]
                .as_array_mut()
                .unwrap()
                .push(text("caption", "Photo")),
            _ => {
                let mut hidden = image("hidden-image");
                hidden["visible"] = json!(false);
                media["children"].as_array_mut().unwrap().push(hidden);
            }
        }
        assert_eq!(
            proven(&section(vec![value]), &["section"]),
            0,
            "case {invalid}"
        );
    }
}

#[test]
fn extra_outer_slots_and_wrong_layouts_do_not_prove_rows() {
    for shell in SHELLS {
        for invalid in 0..3 {
            let mut value = row(0, false, shell);
            match invalid {
                0 => value["children"]
                    .as_array_mut()
                    .unwrap()
                    .push(text("outer-metadata", "Featured")),
                1 => value["layout"] = json!("vertical"),
                _ => node_mut(&mut value, "info-0")["layout"] = json!("horizontal"),
            }
            assert_eq!(
                proven(&section(vec![value]), &["section"]),
                0,
                "{shell:?} case {invalid}"
            );
        }
    }
}

#[test]
fn hidden_or_dynamic_price_and_copy_do_not_add_evidence() {
    for shell in SHELLS {
        for patch in [
            json!({"visible":false}),
            json!({"enabled":false}),
            json!({"opacity":0}),
            json!({"content":"$app.price"}),
            json!({"bindings":{"content":"'¥' + to_str($app.price)"}}),
            json!({"content":" "}),
        ] {
            let mut value = row(0, false, shell);
            node_mut(&mut value, "price-0")
                .as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            assert_eq!(
                proven(&section(vec![value]), &["section"]),
                0,
                "{shell:?} {patch}"
            );
        }
        let mut value = row(0, false, shell);
        node_mut(&mut value, "title-0")["bindings"] = json!({"content":"$app.title"});
        assert_eq!(proven(&section(vec![value]), &["section"]), 0);
    }
}

#[test]
fn hidden_ancestors_and_unresolved_visibility_block_even_explicit_descendant_roots() {
    for patch in [
        json!({"visible":false}),
        json!({"enabled":false}),
        json!({"opacity":0}),
        json!({"enabled":"$app.visible"}),
        json!({"opacity":"$app.opacity"}),
        json!({"bindings":{"visible":"$app.visible"}}),
        json!({"bindings":{"enabled":"$app.enabled"}}),
        json!({"bindings":{"opacity":"$app.opacity"}}),
    ] {
        let mut ancestor = frame("ancestor", "vertical", rows(6, SHELLS[1]));
        ancestor
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let sink = roots(vec![ancestor]);
        assert_eq!(proven(&sink, &["ancestor"]), 0, "{patch}");
        assert_eq!(
            proven(
                &sink,
                &["row-0", "row-1", "row-2", "row-3", "row-4", "row-5"]
            ),
            0,
            "{patch}"
        );
    }
}

#[test]
fn refs_and_their_raw_slot_children_do_not_add_unmaterialized_rows() {
    for raw_children in [false, true] {
        let instances = (0..6)
            .map(|index| {
                let mut instance =
                    json!({"type":"ref", "id":format!("instance-{index}"), "ref":"master"});
                if raw_children {
                    instance["children"] = json!([row(100 + index, false, SHELLS[1])]);
                }
                instance
            })
            .collect();
        let mut sink = section(instances);
        let mut master = row(99, false, SHELLS[1]);
        master["id"] = json!("master");
        master["reusable"] = json!(true);
        sink.state
            .doc
            .children
            .push(serde_json::from_value(master).unwrap());
        assert_eq!(proven(&sink, &["section"]), 0);
        assert_eq!(proven(&sink, &["instance-0", "row-100"]), 0);
        assert_eq!(delivered_item_count(&sink, &["section".into()]), 6);
        assert!(incomplete_attempt(&sink, &task(LOCAL_LIST), &outcome(&["section"])).is_none());
    }
}

#[test]
fn names_and_roles_cannot_supply_a_missing_price_or_plus_control() {
    for missing_price in [false, true] {
        let mut value = row(0, false, SHELLS[1]);
        value["name"] = json!("菜品列表六条 / product row ¥26 plus");
        value["role"] = json!("product-card");
        if missing_price {
            let price = node_mut(&mut value, "price-0");
            price["content"] = json!("Special offer");
            price["name"] = json!("¥26");
            price["role"] = json!("price");
        } else {
            let control = node_mut(&mut value, "plus-0");
            control["iconFontName"] = json!("tag");
            control["name"] = json!("plus");
            control["role"] = json!("button");
        }
        assert_eq!(proven(&section(vec![value]), &["section"]), 0);
    }
    let mut value = row(0, false, SHELLS[1]);
    value["name"] = json!("Unrelated label");
    value["role"] = json!("presentation");
    assert_eq!(proven(&section(vec![value]), &["section"]), 1);
}

#[test]
fn additional_evidence_leaves_the_shared_style_signature_strict() {
    for shell in SHELLS {
        let one: PenNode = serde_json::from_value(row(0, false, shell)).unwrap();
        let two: PenNode = serde_json::from_value(row(1, true, shell)).unwrap();
        assert_ne!(
            crate::cleanup::structural_signature(&one),
            crate::cleanup::structural_signature(&two)
        );
        assert_eq!(
            proven(
                &section(vec![row(0, false, shell), row(1, true, shell)]),
                &["section"]
            ),
            2
        );
    }
}

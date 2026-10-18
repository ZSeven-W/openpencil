use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

const LOCAL_MEDIA_PROMISE: &str = "5 条商家行卡片：左88px图片，右侧名称";

fn task(elements: &str) -> Subtask {
    serde_json::from_value(json!({
        "id":"section", "label":"Merchant list", "elements":elements,
        "region":{"width":375,"height":600}
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

fn section(children: Vec<Value>) -> VecDocSink {
    roots(vec![json!({
        "type":"frame", "id":"section", "layout":"vertical", "children":children
    })])
}

fn image(id: &str) -> Value {
    json!({"type":"image", "id":id, "src":"merchant.png", "width":88,"height":88})
}

fn text(id: &str, content: &str) -> Value {
    json!({"type":"text", "id":id, "content":content})
}

fn card(index: usize, badges: usize) -> Value {
    json!({
        "type":"frame", "id":format!("card-{index}"), "layout":"horizontal",
        "children":[
            image(&format!("image-{index}")),
            {"type":"frame", "id":format!("details-{index}"), "layout":"vertical",
                "children":[
                    text(&format!("title-{index}"), &format!("Merchant {index}")),
                    {"type":"frame", "id":format!("badges-{index}"), "layout":"horizontal",
                        "children":(0..badges).map(|badge| json!({
                            "type":"frame", "id":format!("badge-{index}-{badge}"),
                            "layout":"horizontal", "children":[
                                text(&format!("badge-text-{index}-{badge}"), "Delivery")
                            ]
                        })).collect::<Vec<_>>()}
                ]}
        ]
    })
}

fn simple_card(index: usize, images: usize) -> Value {
    let mut children = (0..images)
        .map(|slot| image(&format!("image-{index}-{slot}")))
        .collect::<Vec<_>>();
    children.push(text(&format!("title-{index}"), "Merchant"));
    json!({
        "type":"frame", "id":format!("card-{index}"),
        "layout":"horizontal", "children":children
    })
}

fn assert_media_count(sink: &VecDocSink, ids: &[&str], count: usize) {
    let outcome = outcome(ids);
    assert_eq!(
        media_items::delivered_media_card_count(sink, &outcome.inserted_root_ids),
        Some(count)
    );
    let delivered = delivered_item_count(sink, &outcome.inserted_root_ids).max(count);
    let failure = incomplete_attempt(sink, &task(LOCAL_MEDIA_PROMISE), &outcome);
    if delivered < 5 {
        let failure = failure.expect("the local media-card quota remains incomplete");
        assert_eq!((failure.expected, failure.delivered), (5, delivered));
    } else {
        assert!(
            failure.is_none(),
            "additive evidence cannot lower generic delivery"
        );
    }
}

#[test]
fn optional_badges_do_not_split_the_promised_media_card_family() {
    let sink = section(
        [2, 2, 1, 1, 2]
            .into_iter()
            .enumerate()
            .map(|(index, badges)| card(index, badges))
            .collect(),
    );
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 3);
    assert_media_count(&sink, &["section"], 5);
    assert!(incomplete_attempt(
        &sink,
        &task("5 cards each with image"),
        &outcome(&["section"])
    )
    .is_none());
}

#[test]
fn media_evidence_excludes_badges_but_keeps_the_legacy_generic_overcount() {
    // This additive fix deliberately leaves the old nested-badge false positive
    // unchanged. The media helper proves only the actual card-family count.
    for count in [2, 3] {
        for badges in [5, 6, 9] {
            let sink = section((0..count).map(|index| card(index, badges)).collect());
            assert_eq!(delivered_item_count(&sink, &["section".into()]), badges);
            assert_media_count(&sink, &["section"], count);
            assert!(
                incomplete_attempt(&sink, &task(LOCAL_MEDIA_PROMISE), &outcome(&["section"]))
                    .is_none()
            );
        }
    }
}

#[test]
fn empty_frames_and_a_separate_gallery_cannot_pad_a_proven_family() {
    let real = || (0..3).map(|index| card(index, 1)).collect::<Vec<_>>();
    let mut empty = real();
    empty.extend((0..2).map(|index| {
        json!({
            "type":"frame", "id":format!("empty-{index}"), "children":[]
        })
    }));
    assert_media_count(&section(empty), &["section"], 3);

    let mut gallery = real();
    gallery.push(json!({
        "type":"frame", "id":"gallery", "layout":"horizontal",
        "children":[image("gallery-image-0"), image("gallery-image-1")]
    }));
    assert_media_count(&section(gallery), &["section"], 3);
}

#[test]
fn inserted_root_ids_do_not_merge_cards_from_different_actual_parents() {
    let sink = roots(vec![
        json!({"type":"frame", "id":"list-a", "layout":"vertical",
            "children":(0..2).map(|index|card(index,1)).collect::<Vec<_>>()}),
        json!({"type":"frame", "id":"list-b", "layout":"vertical",
            "children":(2..5).map(|index|card(index,1)).collect::<Vec<_>>()}),
    ]);
    let ids = ["card-0", "card-1", "card-2", "card-3", "card-4"];
    assert_eq!(
        delivered_item_count(&sink, &outcome(&ids).inserted_root_ids),
        5
    );
    assert_media_count(&sink, &ids, 3);
    assert_media_count(&sink, &["list-a", "list-b"], 3);
}

#[test]
fn immediate_media_slot_multiplicity_remains_structural() {
    let mut children = (0..3)
        .map(|index| simple_card(index, 1))
        .collect::<Vec<_>>();
    children.extend((3..5).map(|index| simple_card(index, 2)));
    assert_media_count(&section(children), &["section"], 3);
}

#[test]
fn slot_order_and_card_layout_remain_separate_families() {
    for change_order in [true, false] {
        let mut children = (0..3)
            .map(|index| simple_card(index, 1))
            .collect::<Vec<_>>();
        children.extend((3..5).map(|index| {
            let mut value = simple_card(index, 1);
            if change_order {
                value["children"].as_array_mut().unwrap().reverse();
            } else {
                value["layout"] = json!("vertical");
            }
            value
        }));
        assert_media_count(&section(children), &["section"], 3);
    }
}

#[test]
fn hidden_and_blank_text_cards_do_not_pad_a_proven_family() {
    for hidden_property in ["visible", "enabled"] {
        let mut children = (0..3)
            .map(|index| simple_card(index, 1))
            .collect::<Vec<_>>();
        let mut hidden = simple_card(3, 1);
        hidden[hidden_property] = json!(false);
        children.push(hidden);
        let mut blank = simple_card(4, 1);
        blank["children"][1]["content"] = json!(" \t\n");
        children.push(blank);
        assert_media_count(&section(children), &["section"], 3);
    }

    let mut children = (0..3)
        .map(|index| simple_card(index, 1))
        .collect::<Vec<_>>();
    let mut hidden_text = simple_card(3, 1);
    hidden_text["children"][1]["visible"] = json!(false);
    children.push(hidden_text);
    children.push(
        json!({"type":"frame", "id":"hidden-parent", "visible":false,
        "children":[simple_card(4,1),simple_card(5,1)]}),
    );
    assert_media_count(&section(children), &["section"], 3);
}

#[test]
fn ref_cards_keep_generic_delivery_without_adding_media_evidence() {
    let mut children = vec![simple_card(100, 1)];
    children.extend((0..5).map(|index| {
        json!({
            "type":"ref", "id":format!("instance-{index}"), "ref":"master"
        })
    }));
    let mut sink = section(children);
    let mut master = card(99, 2);
    master["id"] = json!("master");
    master["reusable"] = json!(true);
    sink.state
        .doc
        .children
        .push(serde_json::from_value(master).unwrap());
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 5);
    assert_media_count(&sink, &["section"], 1);
    let ids: Vec<String> = (0..5).map(|index| format!("instance-{index}")).collect();
    assert_eq!(media_items::delivered_media_card_count(&sink, &ids), None);
    assert_eq!(delivered_item_count(&sink, &ids), 5);
}

#[test]
fn unsupported_media_keeps_generic_delivery_when_no_media_family_is_proven() {
    let children = (0..5)
        .map(|index| {
            json!({
                "type":"frame", "id":format!("card-{index}"), "layout":"horizontal",
                "children":[
                    {"type":"icon_font", "id":format!("icon-{index}"),
                        "iconFontFamily":"lucide", "iconFontName":"image", "width":88,"height":88},
                    text(&format!("title-{index}"), "Merchant")
                ]
            })
        })
        .collect();
    let sink = section(children);
    let outcome = outcome(&["section"]);
    assert_eq!(
        media_items::delivered_media_card_count(&sink, &outcome.inserted_root_ids),
        None
    );
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 5);
    assert!(incomplete_attempt(&sink, &task(LOCAL_MEDIA_PROMISE), &outcome).is_none());
}

#[test]
fn five_empty_frames_keep_the_legacy_generic_result() {
    let sink = section(
        (0..5)
            .map(|index| {
                json!({
                    "type":"frame", "id":format!("empty-{index}"), "children":[]
                })
            })
            .collect(),
    );
    let outcome = outcome(&["section"]);
    assert_eq!(
        media_items::delivered_media_card_count(&sink, &outcome.inserted_root_ids),
        None
    );
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 5);
    assert!(incomplete_attempt(&sink, &task(LOCAL_MEDIA_PROMISE), &outcome).is_none());
}

#[test]
fn a_proven_hero_does_not_lower_generic_wrapped_or_image_fill_card_delivery() {
    for image_fill in [false, true] {
        // The hero is itself a recognizable frame[image,text] family of one.
        // Unsupported media representations below must retain generic delivery.
        let mut children = vec![simple_card(100, 1)];
        children.extend((0..5).map(|index| {
            let mut value = card(index, 1);
            value["children"][0] = if image_fill {
                json!({"type":"rectangle", "id":format!("image-fill-{index}"),
                    "width":88,"height":88,
                    "fill":[{"type":"image","url":"merchant.png","mode":"fill"}]})
            } else {
                json!({"type":"frame", "id":format!("image-wrapper-{index}"),
                    "children":[image(&format!("wrapped-image-{index}"))]})
            };
            value
        }));
        let sink = section(children);
        assert_eq!(delivered_item_count(&sink, &["section".into()]), 5);
        assert_media_count(&sink, &["section"], 1);
    }
}

#[test]
fn media_contract_requires_a_matching_local_count_and_per_item_image_promise() {
    for elements in [
        LOCAL_MEDIA_PROMISE,
        "5 cards each with image",
        "5 cards, each with image",
    ] {
        let task = task(elements);
        assert_eq!(expected_item_count(&task), Some(5), "{elements}");
        assert!(
            media_items::has_local_media_card_contract(&local_item_text(&task), 5),
            "{elements}"
        );
        assert!(
            !media_items::has_local_media_card_contract(&local_item_text(&task), 3),
            "{elements}"
        );
    }
    for elements in [
        "5 cards; a hero image above the list",
        "A hero image; 5 items of customer reviews",
        "5 cards; each image belongs to the hero",
        "5 条评论；顶部展示图片",
        "5 cards; another clause has 5 images",
        "5 条商家行卡片；主图区域有5张图片",
        "3 cards each with image; show 5 items",
    ] {
        let task = task(elements);
        assert_eq!(expected_item_count(&task), Some(5), "{elements}");
        assert!(
            !media_items::has_local_media_card_contract(&local_item_text(&task), 5),
            "{elements}"
        );
    }
}

#[test]
fn quoted_metadata_and_foreign_media_promises_do_not_change_local_generic_delivery() {
    let sink = section(
        [2, 2, 1, 1, 2]
            .into_iter()
            .enumerate()
            .map(|(index, badges)| card(index, badges))
            .collect(),
    );
    for elements in [
        "标题『5 条商家行卡片：左88px图片，右侧名称』；本区展示5条评论",
        "Title '5 cards each with image'; show 5 items",
        "metadata (5 cards); hero image; show 5 items",
        "Another section displays 5 cards each with image; show 5 items",
        "5 cards each with image in another section; show 5 items",
        "仅供参考：5 条商家行卡片：左88px图片，右侧名称；本区展示5条评论",
        "5 cards; another section owns the images",
    ] {
        let task = task(elements);
        assert_eq!(expected_item_count(&task), Some(5), "{elements}");
        assert!(
            !media_items::has_local_media_card_contract(&local_item_text(&task), 5),
            "{elements}"
        );
        let failure = incomplete_attempt(&sink, &task, &outcome(&["section"])).unwrap();
        assert_eq!((failure.expected, failure.delivered), (5, 3), "{elements}");
    }
}

#[test]
fn ordinary_generic_promises_keep_the_existing_structural_family_result() {
    for elements in [
        "5 cards",
        "5 条商家行卡片",
        "A hero image; 5 items of customer reviews",
    ] {
        let sink = section(
            [2, 2, 1, 1, 2]
                .into_iter()
                .enumerate()
                .map(|(index, badges)| card(index, badges))
                .collect(),
        );
        assert_eq!(
            media_items::delivered_media_card_count(&sink, &["section".into()]),
            Some(5)
        );
        let failure = incomplete_attempt(&sink, &task(elements), &outcome(&["section"])).unwrap();
        assert_eq!((failure.expected, failure.delivered), (5, 3), "{elements}");
    }
}

#[test]
fn grid_delivery_stays_on_the_existing_grid_path_even_with_a_media_promise() {
    let children = (0..2)
        .map(|row| {
            json!({
                "type":"frame", "id":format!("row-{row}"), "layout":"horizontal",
                "children":(0..3).map(|column|simple_card(row*3+column,1)).collect::<Vec<_>>()
            })
        })
        .collect();
    let sink = section(children);
    let task = task("2x3 product grid; 6 cards each with image");
    assert_eq!(expected_item_count(&task), Some(6));
    assert!(media_items::has_local_media_card_contract(
        &local_item_text(&task),
        6
    ));
    assert_eq!(
        media_items::delivered_media_card_count(&sink, &["section".into()]),
        Some(3)
    );
    assert!(incomplete_attempt(&sink, &task, &outcome(&["section"])).is_none());
}

#[test]
fn shared_style_cleanup_signature_still_distinguishes_optional_badge_counts() {
    let one: PenNode = serde_json::from_value(card(0, 1)).unwrap();
    let two: PenNode = serde_json::from_value(card(1, 2)).unwrap();
    assert_ne!(
        crate::cleanup::structural_signature(&one),
        crate::cleanup::structural_signature(&two)
    );
    assert_media_count(
        &section(vec![
            card(0, 1),
            card(1, 2),
            card(2, 1),
            card(3, 2),
            card(4, 1),
        ]),
        &["section"],
        5,
    );
}

#[test]
fn overridden_ref_content_does_not_add_unproven_media_cards() {
    for nested in [false, true] {
        let mut children = (0..3).map(|index| card(index, 1)).collect::<Vec<_>>();
        children.extend((0..2).map(|index| {
            json!({
                "type":"ref", "id":format!("instance-{index}"),
                "ref":if nested { "alias" } else { "master" },
                "descendants":if nested { json!({}) } else { json!({"master":{"visible":false}}) }
            })
        }));
        let mut sink = section(children);
        let mut master = card(99, 2);
        master["id"] = json!("master");
        master["reusable"] = json!(true);
        sink.state
            .doc
            .children
            .push(serde_json::from_value(master).unwrap());
        if nested {
            sink.state.doc.children.push(
                serde_json::from_value(json!({
                    "type":"ref", "id":"alias", "ref":"master",
                    "descendants":{"master":{"visible":false}}
                }))
                .unwrap(),
            );
        }
        assert_eq!(delivered_item_count(&sink, &["section".into()]), 3);
        assert_media_count(&sink, &["section"], 3);
        let failure =
            incomplete_attempt(&sink, &task(LOCAL_MEDIA_PROMISE), &outcome(&["section"])).unwrap();
        assert_eq!((failure.expected, failure.delivered), (5, 3));
    }
}

#[test]
fn statically_transparent_cards_do_not_add_new_quota_evidence() {
    let mut children = (0..3).map(|index| card(index, 1)).collect::<Vec<_>>();
    children.extend((3..5).map(|index| {
        let mut transparent = card(index, 2);
        transparent["opacity"] = json!(0);
        transparent
    }));
    let sink = section(children);
    // Distinct original badge structures keep the old generic family at three.
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 3);
    assert_media_count(&sink, &["section"], 3);
    assert_eq!(
        incomplete_attempt(&sink, &task(LOCAL_MEDIA_PROMISE), &outcome(&["section"]))
            .unwrap()
            .delivered,
        3
    );
}

#[test]
fn shadowed_ref_slot_children_cannot_add_a_nonrendered_card_family() {
    let mut children = (0..3).map(|index| card(index, 1)).collect::<Vec<_>>();
    children.push(json!({
        "type":"ref", "id":"instance", "ref":"master",
        "children":(10..15).map(|index|card(index,if index < 13 {1} else {2})).collect::<Vec<_>>()
    }));
    let mut sink = section(children);
    sink.state.doc.children.push(
        serde_json::from_value(json!({
            "type":"frame", "id":"master", "reusable":true,
            "children":[text("master-title", "Existing component content")]
        }))
        .unwrap(),
    );
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 3);
    assert_media_count(&sink, &["section"], 3);
    assert_eq!(
        incomplete_attempt(&sink, &task(LOCAL_MEDIA_PROMISE), &outcome(&["section"]))
            .unwrap()
            .delivered,
        3
    );
}

#[test]
fn invisible_ancestors_do_not_expose_a_normalized_descendant_family() {
    for (property, value) in [
        ("visible", json!(false)),
        ("enabled", json!(false)),
        ("opacity", json!(0)),
    ] {
        let mut children = (0..3).map(|index| card(index, 1)).collect::<Vec<_>>();
        let mut ancestor = json!({
            "type":"frame", "id":"hidden-list", "layout":"vertical",
            "children":[card(10,1),card(11,1),card(12,1),card(13,2),card(14,2)]
        });
        ancestor[property] = value;
        children.push(ancestor);
        let sink = section(children);
        assert_eq!(delivered_item_count(&sink, &["section".into()]), 3);
        assert_media_count(&sink, &["section"], 3);
    }
}

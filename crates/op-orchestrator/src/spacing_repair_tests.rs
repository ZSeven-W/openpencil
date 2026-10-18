use super::*;
use serde_json::json;

fn node(v: Value) -> PenNode {
    serde_json::from_value(v).expect("valid PenNode")
}

fn card(id: &str) -> Value {
    json!({"type":"frame","id":id,"layout":"vertical","padding":24,
           "fill":[{"type":"solid","color":"#141414"}],
           "children":[{"type":"text","id":format!("{id}t"),"content":"x"}]})
}

#[test]
fn transparent_metrics_strip_loses_both_paddings() {
    // test0703.op shape: "Key Metrics" (transparent, padded [24,32]) inside a
    // [32,40]-padded gap-20 main column — its padding starved the cards.
    let mut root = node(json!({
        "type":"frame","id":"main","layout":"vertical","gap":20,"padding":[32,40],
        "children":[
            {"type":"frame","id":"metrics","layout":"horizontal","gap":16,"padding":[24,32],
             "children":[card("c1"), card("c2")]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    assert!(
        v["children"][0].get("padding").is_none(),
        "wrapper padding fully stripped: {}",
        v["children"][0]
    );
    // The CARDS keep their own padding — they paint a surface.
    assert_eq!(
        v["children"][0]["children"][0]["padding"].as_f64(),
        Some(24.0)
    );
}

#[test]
fn landing_sections_in_unpadded_root_keep_self_padding() {
    // Landing shape: root column has NO padding and NO gap — each section's
    // [0,24] self-padding IS the page inset. Must stay.
    let mut root = node(json!({
        "type":"frame","id":"page","layout":"vertical",
        "children":[
            {"type":"frame","id":"hero","layout":"vertical","padding":[64,24],
             "children":[{"type":"text","id":"h","content":"Hi"}]}
        ]
    }));
    assert!(!strip_wrapper_double_inset(&mut root));
}

#[test]
fn gapped_but_unpadded_column_strips_only_vertical() {
    // Parent gaps 24 but pads 0 horizontally: the wrapper's horizontal
    // padding is its only inset (keep); vertical padding double-spaces (strip).
    let mut root = node(json!({
        "type":"frame","id":"col","layout":"vertical","gap":24,
        "children":[
            {"type":"frame","id":"wrap","layout":"horizontal","padding":[16,24],
             "children":[card("c1")]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    assert_eq!(
        v["children"][0]["padding"],
        json!([0.0, 24.0, 0.0, 24.0]),
        "vertical stripped, horizontal kept"
    );
}

#[test]
fn canonical_bottom_nav_keeps_internal_vertical_clearance_separate_from_parent_gap() {
    for horizontal_parent_padding in [0, 24] {
        let mut root = node(json!({
            "type":"frame","id":"phone","width":390,"height":844,
            "layout":"vertical","gap":16,"padding":[0,horizontal_parent_padding,0,horizontal_parent_padding],
            "children":[{
                "type":"frame","id":"nav","role":"bottom-tab-bar","layout":"horizontal",
                "width":"fill_container","height":72,"padding":[8,16,8,16],"children":[
                    {"type":"text","id":"home","content":"Home"},
                    {"type":"text","id":"orders","content":"Orders"},
                    {"type":"text","id":"me","content":"Me"}
                ]
            }]
        }));
        assert_eq!(
            strip_wrapper_double_inset(&mut root),
            horizontal_parent_padding != 0
        );
        let v = serde_json::to_value(&root).unwrap();
        let inset = if horizontal_parent_padding == 0 {
            16.0
        } else {
            0.0
        };
        assert_eq!(v["children"][0]["padding"], json!([8.0, inset, 8.0, inset]));
        let before = root.clone();
        assert!(!strip_wrapper_double_inset(&mut root));
        assert_eq!(root, before);
    }
}

#[test]
fn nav_names_and_ordinary_rows_do_not_bypass_vertical_inset_deduplication() {
    for role in [None, Some("toolbar"), Some("content")] {
        let mut row = json!({
            "type":"frame","id":"row","name":"Bottom Tab Bar","layout":"horizontal",
            "padding":[8,16,8,16],"children":[{"type":"text","id":"label","content":"Content"}]
        });
        if let Some(role) = role {
            row["role"] = json!(role);
        }
        let mut root = node(json!({
            "type":"frame","id":"phone","width":390,"height":844,
            "layout":"vertical","gap":16,"children":[row]
        }));
        assert!(strip_wrapper_double_inset(&mut root));
        let v = serde_json::to_value(root).unwrap();
        assert_eq!(v["children"][0]["padding"], json!([0.0, 16.0, 0.0, 16.0]));
    }
}

#[test]
fn landing_page_sections_keep_their_own_vertical_rhythm() {
    // 0808-gm-1.op: a marketing page root (no padding, gap 20) whose eight
    // sections each carry an 80px vertical inset of their own. A 20px gap
    // cannot stand in for an inset four times its size — this is section
    // rhythm, not a duplicate of the gap, and zeroing it collapsed the whole
    // page into a 20px stack.
    let section = |id: &str| {
        json!({"type":"frame","id":id,"layout":"vertical","padding":[80, 80],
               "children":[card(&format!("{id}-c"))]})
    };
    let mut root = node(json!({
        "type":"frame","id":"page","layout":"vertical","gap":20,
        "children":[section("hero"), section("features"), section("cta")]
    }));
    assert!(!strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    for child in v["children"].as_array().unwrap() {
        assert_eq!(
            child["padding"],
            json!([80.0, 80.0]),
            "section rhythm survives verbatim: {child}"
        );
    }
}

#[test]
fn a_web_page_keeps_section_padding_a_phone_screen_loses_it() {
    // Same tree, two artboards — the one design-type test the repair layer
    // owes: an APP screen's rules must not reshape a WEB page. The 24px
    // section inset sits INSIDE the gap-20 column's duplicate window
    // (`gap_absorbs_vertical_padding` says "duplicate" for both cases), so only
    // the artboard can tell them apart. This isolates the form signal.
    let tree = |width: f64, height: f64| {
        json!({
            "type":"frame","id":"page","layout":"vertical","gap":20,
            "width": width, "height": height,
            "children":[
                {"type":"frame","id":"s1","layout":"vertical","padding":[24, 24],
                 "children":[card("s1-c")]},
                {"type":"frame","id":"s2","layout":"vertical","padding":[24, 24],
                 "children":[card("s2-c")]}
            ]
        })
    };

    // 1200 x 2977 — 0808-gm-1.op's marketing page. Sections own the rhythm.
    let mut web = node(tree(1200.0, 2977.0));
    assert!(
        !strip_wrapper_double_inset(&mut web),
        "a web page's section rhythm is not a duplicate of the page gap"
    );
    let web = serde_json::to_value(&web).unwrap();
    assert_eq!(web["children"][0]["padding"], json!([24.0, 24.0]));

    // 375 x 812 — a phone screen. The original de-duplication still applies.
    let mut phone = node(tree(375.0, 812.0));
    assert!(strip_wrapper_double_inset(&mut phone));
    let phone = serde_json::to_value(&phone).unwrap();
    assert_eq!(
        phone["children"][0]["padding"],
        json!([0.0, 24.0, 0.0, 24.0]),
        "phone chrome keeps the tight stack the pass was built for"
    );
}

#[test]
fn section_padding_deeper_than_the_gap_survives_on_every_artboard() {
    // The magnitude rule and the artboard rule are independent guards, and
    // this pins the magnitude one on the form the artboard rule does NOT
    // cover: a 375-wide phone screen whose sections are inset far deeper than
    // the column's gap. 48 dwarfs 20 — no gap that size is being duplicated.
    let mut phone = node(json!({
        "type":"frame","id":"screen","layout":"vertical","gap":20,
        "width": 375.0, "height": 812.0,
        "children":[
            {"type":"frame","id":"s1","layout":"vertical","padding":[48, 16],
             "children":[card("s1-c")]}
        ]
    }));
    assert!(!strip_wrapper_double_inset(&mut phone));
    let phone = serde_json::to_value(&phone).unwrap();
    assert_eq!(phone["children"][0]["padding"], json!([48.0, 16.0]));
}

#[test]
fn a_padded_desktop_main_column_still_de_duplicates() {
    // The shape the pass was BUILT for, now carrying a desktop width: the
    // column pads itself, so it owns the frame and an interior strip's inset
    // really is a duplicate. Width alone must not grant immunity.
    let mut root = node(json!({
        "type":"frame","id":"main","layout":"vertical","gap":20,"padding":[32, 40],
        "width": 1200.0, "height": 900.0,
        "children":[
            {"type":"frame","id":"metrics","layout":"horizontal","gap":16,"padding":[24, 32],
             "children":[card("c1"), card("c2")]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    assert!(v["children"][0].get("padding").is_none());
}

#[test]
fn vertical_padding_of_the_gap_s_own_order_still_strips() {
    // The narrowing must not disarm the pass: a wrapper padded 16 inside a
    // gap-24 column is still double-spacing (the gap already supplies more
    // separation than the padding does), so it goes.
    let mut root = node(json!({
        "type":"frame","id":"col","layout":"vertical","gap":24,
        "children":[
            {"type":"frame","id":"wrap","layout":"vertical","padding":[16, 0],
             "children":[card("c1")]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    assert!(v["children"][0].get("padding").is_none());
}

#[test]
fn painted_wrappers_are_not_touched() {
    // A filled panel / stroked card is a SURFACE — its padding is design.
    let mut root = node(json!({
        "type":"frame","id":"main","layout":"vertical","gap":20,"padding":[32,40],
        "children":[
            {"type":"frame","id":"panel","layout":"vertical","padding":24,
             "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#2A2A2A"}]},
             "children":[{"type":"text","id":"t","content":"x"}]}
        ]
    }));
    assert!(!strip_wrapper_double_inset(&mut root));
}

#[test]
fn divider_bar_header_strips_horizontal_keeps_vertical() {
    // A top bar whose only paint is its bottom hairline is a DIVIDER bar —
    // its [20,32] padding inside the [32,40]-padded column misaligns the
    // title 32px deeper than the sections below; horizontal strips, the
    // vertical breathing against its own rule stays.
    let mut root = node(json!({
        "type":"frame","id":"main","layout":"vertical","gap":20,"padding":[32,40],
        "children":[
            {"type":"frame","id":"bar","layout":"horizontal","padding":[20,32],
             "stroke":{"thickness":{"bottom":1},"fill":[{"type":"solid","color":"#2A2A2A"}]},
             "children":[{"type":"text","id":"t","content":"Title"}]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    assert_eq!(
        v["children"][0]["padding"],
        json!([20.0, 0.0, 20.0, 0.0]),
        "horizontal stripped, vertical kept: {}",
        v["children"][0]
    );
}

#[test]
fn side_stroked_panel_keeps_all_padding() {
    // A LEFT-accent stroke is a surface treatment, not a divider — padding
    // is design, untouched.
    let mut root = node(json!({
        "type":"frame","id":"main","layout":"vertical","gap":20,"padding":[32,40],
        "children":[
            {"type":"frame","id":"panel","layout":"vertical","padding":[16,20],
             "stroke":{"thickness":{"left":2},"fill":[{"type":"solid","color":"#C9A962"}]},
             "children":[{"type":"text","id":"t","content":"Note"}]}
        ]
    }));
    assert!(!strip_wrapper_double_inset(&mut root));
}

#[test]
fn nested_transparent_chain_converges_on_one_gutter_owner() {
    // rail(24) > sheet(24) > inner(24): every structural layer re-adding the
    // rail's gutter. Only the rail may keep it — otherwise the card lands
    // 48px inside its siblings instead of 24.
    let mut root = node(json!({
        "type":"frame","id":"rail","layout":"vertical","padding":[0,24],
        "children":[
            {"type":"frame","id":"sheet","layout":"vertical","padding":[16,24],
             "children":[
                 {"type":"frame","id":"inner","layout":"vertical","padding":[8,24],
                  "children":[card("c")]}
             ]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    let sheet = &v["children"][0];
    let inner = &sheet["children"][0];
    assert_eq!(sheet["padding"], json!([16.0, 0.0, 16.0, 0.0]));
    assert_eq!(inner["padding"], json!([8.0, 0.0, 8.0, 0.0]));
    assert_eq!(
        inner["children"][0]["padding"],
        json!(24.0),
        "the opaque card's own inset survives the whole chain"
    );
}

#[test]
fn nested_chain_strip_is_a_fixed_point() {
    let mut root = node(json!({
        "type":"frame","id":"rail","layout":"vertical","padding":[0,24],
        "children":[
            {"type":"frame","id":"sheet","layout":"vertical","padding":[16,24],
             "children":[
                 {"type":"frame","id":"inner","layout":"vertical","padding":[8,24],
                  "children":[card("c")]}
             ]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let once = serde_json::to_value(&root).unwrap();
    assert!(
        !strip_wrapper_double_inset(&mut root),
        "a second run must find nothing left to strip"
    );
    assert_eq!(once, serde_json::to_value(&root).unwrap());
}

#[test]
fn gutter_claim_does_not_cross_a_card_surface() {
    // rail(24) > card(fill, padding 8) > lane(padding 12): the lane sits
    // inside a real surface whose own inset is too small to claim a gutter,
    // so nothing authorizes stripping it. Were the rail's claim to leak
    // through the card, the lane would lose its 12px.
    let mut root = node(json!({
        "type":"frame","id":"rail","layout":"vertical","padding":[0,24],
        "children":[
            {"type":"frame","id":"surface","layout":"vertical","padding":8,
             "fill":[{"type":"solid","color":"#141414"}],
             "children":[
                 {"type":"frame","id":"lane","layout":"vertical","padding":[8,12],
                  "children":[card("c")]}
             ]}
        ]
    }));
    let v = serde_json::to_value(&root).unwrap();
    let before = v["children"][0].clone();
    strip_wrapper_double_inset(&mut root);
    assert_eq!(
        serde_json::to_value(&root).unwrap()["children"][0],
        before,
        "nothing under a painted card is a rail-gutter duplicate"
    );
}

#[test]
fn uniformly_wrapped_siblings_stay_equal_width() {
    // All three sections inset through an identical transparent wrapper. The
    // gutter moves to the single owner (the rail), so the sections stay equal
    // to each other — the pass never introduces the asymmetry it removes.
    let wrapper = |id: &str| {
        json!({"type":"frame","id":id,"layout":"vertical","padding":[16,24],
               "children":[card(&format!("{id}-c"))]})
    };
    let mut root = node(json!({
        "type":"frame","id":"rail","layout":"vertical","padding":[0,24],"gap":20,
        "children":[wrapper("a"), wrapper("b"), wrapper("c")]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    let pads: Vec<_> = v["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["padding"].clone())
        .collect();
    assert_eq!(
        pads,
        vec![Value::Null; 3],
        "every sibling loses the same padding, so their widths stay identical"
    );
}

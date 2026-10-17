use super::*;

fn retained_action_bar() -> Value {
    // Retained n172 subtree from the full-pipeline workout 05 output, before
    // its erroneous content-rail parent; no authored style is changed here.
    serde_json::from_str(include_str!("test_fixtures/workout_pinned_action_bar.json"))
        .expect("retained pinned action bar")
}

fn screen_with_bar(bar: Value) -> Value {
    json!({"type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
    "children":[
        {"type":"frame","id":"status","role":"status-bar","height":44},
        {"type":"frame","id":"body","width":"fill_container","layout":"vertical",
         "padding":[0,24],"children":[text("body-t","Exercises")]},
        bar
    ]})
}

fn root_child_ids(sink: &VecDocSink) -> Vec<&str> {
    sink.state.active_children()[0]
        .children()
        .expect("screen children")
        .iter()
        .map(PenNodeExt::id_str)
        .collect()
}

#[test]
fn retained_pinned_action_owns_its_gutter_without_becoming_an_inset_card() {
    let mut sink = insert(screen_with_bar(retained_action_bar()));
    let before = node_json(&sink, "n172");

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(root_child_ids(&sink), ["status", "body", "n172"]);
    assert_eq!(node_json(&sink, "n172"), before);
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let button = scene.active_page().unwrap().find("n174").unwrap().bounds;
    assert_eq!(button.origin.x, 24.0);
    assert_eq!(
        button.size.x, 327.0,
        "one gutter keeps the authored action width"
    );
    assert!(
        sink.applied.is_empty(),
        "the existing 24px gutter is sufficient"
    );
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "the action bar is stable on a second pass"
    );
}

#[test]
fn action_bar_can_own_the_same_gutter_directly_and_use_a_bottom_rule() {
    let mut bar = retained_action_bar();
    let inner = bar["children"][0].take();
    bar["padding"] = inner["padding"].clone();
    bar["children"] = inner["children"].clone();
    bar["stroke"]["thickness"] = json!([0, 0, 1, 0]);
    let mut sink = insert(screen_with_bar(bar));
    let before = node_json(&sink, "n172");

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(root_child_ids(&sink), ["status", "body", "n172"]);
    assert_eq!(node_json(&sink, "n172"), before);
    assert!(sink.applied.is_empty());
}

#[test]
fn cards_and_actions_without_an_owned_gutter_still_receive_an_outer_rail() {
    for case in [
        "rounded-checkout",
        "rounded-corners",
        "side-border",
        "uniform-border",
        "floating-shadow",
        "ordinary-trailing-card",
        "missing-inner-gutter",
        "button-label-padding-only",
        "small-inner-gutter",
        "asymmetric-inner-gutter",
        "positioned-inner-owner",
        "fixed-height-inner-owner",
        "non-full-width-inner-owner",
        "clipped-rounded-inner-owner",
        "shadowed-inner-owner",
        "blurred-inner-owner",
        "filled-inner-owner",
        "stroked-inner-owner",
    ] {
        let mut bar = retained_action_bar();
        match case {
            "rounded-checkout" => {
                bar["role"] = json!("checkout-bar");
                bar["cornerRadius"] = json!(16);
            }
            "rounded-corners" => bar["cornerRadius"] = json!([12, 12, 0, 0]),
            "side-border" => bar["stroke"]["thickness"]["right"] = json!(1),
            "uniform-border" => bar["stroke"]["thickness"] = json!(1),
            "floating-shadow" => {
                bar["effects"] = json!([{"type":"shadow","offsetX":0,"offsetY":-4,
                    "blur":12,"spread":0,"color":"#00000033"}]);
            }
            "ordinary-trailing-card" => {
                bar["name"] = json!("Checkout Summary");
                bar["role"] = json!("card");
            }
            "missing-inner-gutter" => {
                bar["children"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("padding");
            }
            "button-label-padding-only" => {
                bar["children"] = bar["children"][0]["children"].clone();
            }
            "small-inner-gutter" => bar["children"][0]["padding"] = json!([12, 8, 24, 8]),
            "asymmetric-inner-gutter" => bar["children"][0]["padding"] = json!([12, 16, 24, 24]),
            "positioned-inner-owner" => bar["children"][0]["x"] = json!(0),
            "fixed-height-inner-owner" => bar["children"][0]["height"] = json!(92),
            "non-full-width-inner-owner" => bar["children"][0]["width"] = json!(200),
            "clipped-rounded-inner-owner" => {
                bar["children"][0]["cornerRadius"] = json!([12, 12, 12, 12]);
                bar["children"][0]["clipContent"] = json!(true);
            }
            "shadowed-inner-owner" => {
                bar["children"][0]["effects"] = json!([{"type":"shadow","offsetX":0,
                    "offsetY":4,"blur":12,"spread":0,"color":"#00000033"}]);
            }
            "blurred-inner-owner" => {
                bar["children"][0]["effects"] = json!([{"type":"blur","radius":4}]);
            }
            "filled-inner-owner" => {
                bar["children"][0]["fill"] = json!([{"type":"solid","color":"#FFFFFF"}]);
            }
            "stroked-inner-owner" => {
                bar["children"][0]["stroke"] = json!({"thickness":1,
                    "fill":[{"type":"solid","color":"#000000"}]});
            }
            _ => unreachable!(),
        }
        let mut sink = insert(screen_with_bar(bar));
        let before = node_json(&sink, "n172");

        repair_mobile_content_rails_for_all_roots(&mut sink);

        assert_eq!(
            root_child_ids(&sink),
            ["status", "body", "n172__content_rail"],
            "{case}"
        );
        assert_eq!(
            node_json(&sink, "n172"),
            before,
            "{case}: authored card is preserved"
        );
        sink.applied.clear();
        repair_mobile_content_rails_for_all_roots(&mut sink);
        assert!(
            sink.applied.is_empty(),
            "{case}: wrapping remains idempotent"
        );
    }
}

#[test]
fn a_pinned_named_section_in_the_middle_is_not_assumed_to_be_persistent_chrome() {
    let mut screen = screen_with_bar(retained_action_bar());
    screen["children"].as_array_mut().unwrap().push(json!({
        "type":"frame","id":"related","width":"fill_container","layout":"vertical",
        "padding":[0,24],"children":[text("related-t","Related workouts")]
    }));
    let mut sink = insert(screen);

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        root_child_ids(&sink),
        ["status", "body", "n172__content_rail", "related"]
    );
}

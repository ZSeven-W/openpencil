use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn task(elements: &str) -> Subtask {
    serde_json::from_value(
        json!({"id":"section", "label":"Detail", "elements":elements,
        "region":{"width":375,"height":812}}),
    )
    .unwrap()
}

fn text(id: &str, content: &str) -> Value {
    json!({"type":"text","id":id,"content":content,"fontSize":14,"width":"fit_content","height":20})
}

fn control() -> Value {
    json!({"type":"frame","id":"checkout","width":112,"height":44,"layout":"horizontal",
        "fill":[{"type":"solid","color":"#e5482b"}],"children":[text("label","去结算")]})
}

fn cart() -> Value {
    json!({"type":"frame","id":"cart","width":327,"height":56,"layout":"horizontal",
        "children":[{"type":"icon_font","id":"icon","iconFontFamily":"lucide",
            "iconFontName":"shopping-cart","width":24,"height":24},control()]})
}

fn page(children: Vec<Value>) -> Value {
    json!({"type":"frame","id":"section","width":375,"height":"fit_content",
        "layout":"vertical","children":children})
}

fn sink(roots: Vec<Value>) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0","children":roots})).unwrap(),
    );
    sink
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

fn missing(root: Value) {
    let sink = sink(vec![root]);
    let failure = bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["section"])).unwrap();
    assert_eq!((failure.expected, failure.delivered), (1, 0));
}

#[test]
fn business_presence_does_not_require_events_state_amount_or_full_width_styling() {
    for width in [180, 327] {
        let mut bar = cart();
        bar["width"] = json!(width);
        let sink = sink(vec![page(vec![bar])]);
        assert!(bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["section"])).is_none());
    }
    let mut cta = control();
    cta["children"][0]["content"] = json!("加入购物车");
    let wrapper = json!({"type":"frame","id":"rail","width":375,"height":72,"layout":"horizontal","children":[cta.clone()]});
    let sink = sink(vec![page(vec![text("title", "商品介绍"), wrapper])]);
    assert!(bottom_action::inspect(&sink, &task("底部加购栏"), &outcome(&["section"])).is_none());
    cta["id"] = json!("section");
    cta["width"] = json!(375);
    let sink = self::sink(vec![cta]);
    assert!(bottom_action::inspect(&sink, &task("底部加购栏"), &outcome(&["section"])).is_none());
}

#[test]
fn only_an_explicit_local_mobile_business_bar_promise_is_enforced() {
    let sink = sink(vec![page(vec![])]);
    for elements in [
        "菜品列表六条",
        "每张商品卡提供加入购物车按钮",
        "底部导航栏",
        "普通页脚",
        "文案：\"购物车条\"",
        "不包含购物车条",
        "不要购物车条",
        "购物车条由其他区块负责",
        "购物车条，由其他区块负责；本区展示菜品",
        "cart bar, owned by another section; show dishes",
    ] {
        assert!(
            bottom_action::inspect(&sink, &task(elements), &outcome(&["section"])).is_none(),
            "{elements}"
        );
    }
    for width in [0.0, 1440.0] {
        let mut task = task("responsive web cart bar");
        task.region.width = width;
        assert!(bottom_action::inspect(&sink, &task, &outcome(&["section"])).is_none());
    }
    for elements in [
        "购物车条",
        "购物车条由section负责",
        "checkout footer",
        "底部加购栏",
    ] {
        assert!(
            bottom_action::inspect(&sink, &task(elements), &outcome(&["section"])).is_some(),
            "{elements}"
        );
    }
}

#[test]
fn spacer_names_roles_dish_buttons_and_bottom_copy_do_not_supply_a_bar() {
    for children in [
        vec![json!({"type":"frame","id":"spacer","name":"购物车条","role":"cart-bar","width":375,"height":64})],
        vec![text("bottom","购物车 去结算")],
        vec![control(), text("after","后续详情")],
        vec![text("title","商品介绍"), control()],
        (0..6).map(|i| json!({"type":"frame","id":format!("dish-{i}"),"height":100,"width":327,
            "layout":"horizontal","children":[text(&format!("dish-title-{i}"),"菜品"),
            {"type":"frame","id":format!("add-{i}"),"name":"加入购物车","width":32,"height":32,
                "fill":[{"type":"solid","color":"#f00"}],"children":[
                    {"type":"icon_font","id":format!("plus-{i}"),"iconFontName":"plus","width":20,"height":20}]}]})).collect(),
    ] { missing(page(children)); }
    let product = json!({"type":"frame","id":"product","width":327,"height":300,"layout":"vertical",
        "children":[{"type":"image","id":"photo","src":"product.png","width":100,"height":100},
            text("name","商品名称"),cart()]});
    missing(page(vec![product]));
}

#[test]
fn unrelated_roots_do_not_supply_the_current_tasks_bar_and_shared_ownership_stays_local() {
    let sink = sink(vec![page(vec![]), cart()]);
    // A body task that delegates chrome does not own that promise. A distinct
    // chrome task checks its own output; no generated comment can exempt a miss.
    assert!(bottom_action::inspect(
        &sink,
        &task("菜品内容；购物车条由其他区块负责"),
        &outcome(&["section"])
    )
    .is_none());
    assert!(bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["cart"])).is_none());
    assert!(bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["section"])).is_some());
}

#[test]
fn hidden_opacity_zero_and_hidden_ancestors_cannot_supply_presence() {
    for (key, value) in [("visible", json!(false)), ("opacity", json!(0))] {
        let mut bar = cart();
        bar[key] = value.clone();
        missing(page(vec![bar]));
        let mut root = page(vec![cart()]);
        root[key] = value;
        let sink = sink(vec![root]);
        assert!(bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["cart"])).is_some());
    }
    let mut bar = cart();
    bar["children"][1]["children"][0]["visible"] = json!(false);
    missing(page(vec![bar]));
}

#[test]
fn runtime_uncertainty_defers_only_a_structurally_complete_potential_bar() {
    let mut bar = cart();
    bar["children"][1]["children"][0]["content"] = json!("");
    bar["children"][1]["children"][0]["bindings"] = json!({"content":"$app.checkoutLabel"});
    let sink = sink(vec![page(vec![bar])]);
    assert!(bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["section"])).is_none());
    let mut unrelated = text("dynamic", "anything");
    unrelated["bindings"] = json!({"content":"$app.label"});
    missing(page(vec![unrelated]));
}

#[test]
fn reference_overrides_use_canvas_materialization_instead_of_raw_master_or_slot_nodes() {
    let mut master = cart();
    master["id"] = json!("master");
    master["reusable"] = json!(true);
    let instance = json!({"type":"ref","id":"instance","ref":"master",
        "descendants":{"master":{"visible":false}}});
    let sink = sink(vec![master, page(vec![instance])]);
    assert!(bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["section"])).is_some());
}

#[test]
fn business_controls_allow_count_copy_badges_and_transparent_real_affordances() {
    for label in ["Checkout (2)", "去结算（3）", "结算并支付"] {
        let mut bar = cart();
        bar["children"][1]["children"][0]["content"] = json!(label);
        let sink = sink(vec![page(vec![bar])]);
        assert!(
            bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["section"])).is_none(),
            "{label}"
        );
    }
    for role in [true, false] {
        let mut bar = cart();
        let cta = &mut bar["children"][1];
        cta.as_object_mut().unwrap().remove("fill");
        if role {
            cta["role"] = json!("button");
        } else {
            cta["events"] = json!({"onTap":[{"toast":"'Checkout'"}]});
        }
        cta["children"]
            .as_array_mut()
            .unwrap()
            .push(text("badge", "2"));
        let sink = sink(vec![page(vec![bar])]);
        assert!(bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["section"])).is_none());
    }
}

#[test]
fn bound_numeric_badge_is_supporting_copy_not_a_second_business_label() {
    let mut badge = text("badge", "2");
    badge["bindings"] = json!({"content":"to_str($app.cartCount)"});
    let mut bar = cart();
    bar["children"][1]["children"]
        .as_array_mut()
        .unwrap()
        .push(badge.clone());
    let sink = sink(vec![page(vec![bar])]);
    assert!(bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["section"])).is_none());

    // The same numeric badge cannot supply a missing business action itself.
    let mut badge_only = cart();
    badge_only["children"][1]["children"] = json!([badge]);
    missing(page(vec![badge_only]));
}

#[test]
fn proven_business_control_keeps_price_specification_and_arbitrary_supporting_copy() {
    for events in [false, true] {
        let mut bar = cart();
        let cta = &mut bar["children"][1];
        if events {
            cta["events"] = json!({"onTap":[{"push":"\"/cart\""}]});
        }
        cta["children"].as_array_mut().unwrap().extend([
            text("price", "¥129"),
            text("spec", "白色 · M"),
            text("note", "明日送达 · 可退换"),
        ]);
        let sink = sink(vec![page(vec![bar])]);
        assert!(bottom_action::inspect(&sink, &task("购物车条"), &outcome(&["section"])).is_none());
    }
    let summary = json!({"type":"frame","id":"summary","layout":"vertical","width":150,"height":56,
        "children":[text("total", "合计 ¥129"), text("freight", "不含运费"), text("note", "到货后可申请售后")]});
    let mut bar = cart();
    bar["children"][0] = summary;
    let sink = sink(vec![page(vec![bar])]);
    assert!(bottom_action::inspect(&sink, &task("结算条"), &outcome(&["section"])).is_none());
}

#[test]
fn supporting_copy_cannot_create_an_action_or_swallow_unrelated_content() {
    let mut no_action = cart();
    no_action["children"][1]["events"] = json!({"onTap":[{"push":"\"/cart\""}]});
    no_action["children"][1]["children"] = json!([text("price", "¥129"), text("spec", "白色 · M")]);
    missing(page(vec![no_action]));

    let mut two_labels = cart();
    two_labels["children"][1]["children"]
        .as_array_mut()
        .unwrap()
        .push(text("second", "立即购买"));
    missing(page(vec![two_labels]));

    let mut outside = cart();
    outside["children"]
        .as_array_mut()
        .unwrap()
        .push(text("unowned-spec", "白色 · M"));
    missing(page(vec![outside]));

    let info = json!({"type":"frame","id":"info","layout":"vertical","children":[
        text("product-title", "云感棉衫"), text("price", "¥129"), text("spec", "白色 · M")]});
    let ordinary_card = json!({"type":"frame","id":"card","width":327,"height":160,"layout":"vertical",
        "fill":[{"type":"solid","color":"#fff"}],"children":[info,control()]});
    missing(page(vec![ordinary_card]));
    missing(page(vec![
        text("action-as-heading", "加入购物车"),
        text("subcopy", "¥129 · 白色 · M"),
    ]));
}

#[test]
fn compact_title_first_product_rows_are_not_compound_business_controls() {
    for interactive in [false, true] {
        let mut row = json!({"type":"frame","id":"product-row","width":327,"height":88,
            "layout":"horizontal","fill":[{"type":"solid","color":"#fff"}],"children":[
                text("product-title", "云感棉衫"), text("price", "¥129"), text("action-copy", "加入购物车")]});
        if interactive {
            row["events"] = json!({"onTap":[{"push":"\"/product\""}]});
        }
        missing(page(vec![row]));
    }
}

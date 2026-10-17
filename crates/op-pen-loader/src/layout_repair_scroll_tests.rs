use super::*;
use op_editor_core::EditorState;
use serde_json::{json, Value};

const FIXTURE: &str = include_str!("test_fixtures/mobile_pinned_cta_absolute_wrapper.json");
type LayoutMutationCase = (&'static str, fn(&mut Value));

fn root_value() -> Value {
    serde_json::from_str::<Value>(FIXTURE).unwrap()["children"][0].clone()
}

#[test]
fn pinned_absolute_cta_wrapper_reserves_height_before_scroll_allocation() {
    // The complete-generated 2026-10-18 workout had this exact CTA subtree.
    // Its inner absolute wrapper was first measured as zero, then repaired to
    // 76px while the scroll area incorrectly retained its original 722px.
    let state = EditorState::from_document(serde_json::from_str(FIXTURE).unwrap());
    let stored_document = serde_json::to_value(&state.doc).unwrap();
    let scene = crate::editor_state_to_layout_scene(&state);
    let page = scene.active_page().unwrap();
    let bounds = |id| page.find(id).unwrap().bounds;
    let root = bounds("n175");
    let viewport = bounds("n257");
    let footer = bounds("n251");
    let wrapper = bounds("n252");
    let button = bounds("n254");
    let fade = bounds("n253");

    assert_eq!(root.size.y, 812.0);
    assert_eq!(viewport.size.y, 646.0);
    assert_eq!(footer.size.y, 104.0);
    assert_eq!(wrapper.size.y, 76.0);
    assert_eq!(button.size.y, 56.0);
    assert_eq!(button.origin.y - wrapper.origin.y, 20.0);
    assert_eq!(fade.origin.y, wrapper.origin.y);
    assert_eq!(footer.origin.y, viewport.origin.y + viewport.size.y);
    assert_eq!(footer.origin.y + footer.size.y, root.origin.y + root.size.y);
    assert_eq!(
        button.origin.y + button.size.y + 28.0,
        root.origin.y + root.size.y
    );
    assert_eq!(bounds("long-content").size.y, 1100.0);
    assert!(viewport.size.y < bounds("long-content").size.y);
    assert!(page.find("n257").unwrap().clip_content);
    // Layout solving does not rewrite the stored dimensions or inner offsets.
    assert_eq!(serde_json::to_value(&state.doc).unwrap(), stored_document);
}

fn repaired_rects(value: Value, is_root: bool) -> BTreeMap<String, [f32; 4]> {
    let node: PenNode = serde_json::from_value(value).unwrap();
    let mut rects = initial_rects();
    let props = container_props(&node).unwrap();
    reserve_intrinsic_footer(&node, props, children(&node).unwrap(), &mut rects, is_root);
    rects
}

fn initial_rects() -> BTreeMap<String, [f32; 4]> {
    [
        ("n175", [80.0, 40.0, 375.0, 812.0]),
        ("n176", [80.0, 40.0, 375.0, 62.0]),
        ("n257", [80.0, 102.0, 375.0, 722.0]),
        ("n251", [80.0, 824.0, 375.0, 104.0]),
        ("n252", [104.0, 824.0, 327.0, 76.0]),
        ("n254", [104.0, 844.0, 327.0, 56.0]),
    ]
    .into_iter()
    .map(|(id, rect)| (id.to_owned(), rect))
    .collect()
}

#[test]
fn pinned_scroll_reservation_changes_only_the_flexible_viewport_height() {
    let mut root = root_value();
    root["clipContent"] = json!(true);
    let mut expected = initial_rects();
    expected.get_mut("n257").unwrap()[3] = 646.0;
    assert_eq!(repaired_rects(root, true), expected);
}

#[test]
fn pinned_scroll_reservation_preserves_ambiguous_or_authored_layouts() {
    let cases: &[LayoutMutationCase] = &[
        ("no explicit scroll", |root| {
            root["children"][1]["role"] = json!("content")
        }),
        ("multiple fill areas", |root| {
            root["children"][0]["height"] = json!("fill_container")
        }),
        ("nonfixed root", |root| {
            root["height"] = json!("fit_content")
        }),
        ("fixed scroll height", |root| {
            root["children"][1]["height"] = json!(722)
        }),
        ("minimum scroll height", |root| {
            root["children"][1]["minHeight"] = json!(700)
        }),
        ("maximum scroll height", |root| {
            root["children"][1]["maxHeight"] = json!(700)
        }),
        ("minimum scroll width", |root| {
            root["children"][1]["minWidth"] = json!(320)
        }),
        ("maximum scroll width", |root| {
            root["children"][1]["maxWidth"] = json!(400)
        }),
        ("centered scroll", |root| {
            root["children"][1]["justifyContent"] = json!("center")
        }),
        ("end-aligned scroll", |root| {
            root["children"][1]["justifyContent"] = json!("end")
        }),
        ("distributed scroll", |root| {
            root["children"][1]["justifyContent"] = json!("space_between")
        }),
        ("fixed footer height", |root| {
            root["children"][2]["height"] = json!(28)
        }),
        ("positioned scroll", |root| {
            root["children"][1]["y"] = json!(0)
        }),
        ("positioned footer", |root| {
            root["children"][2]["x"] = json!(0)
        }),
        ("unclipped board", |root| root["clipContent"] = json!(false)),
        ("unclipped viewport", |root| {
            root["children"][1]["clipContent"] = json!(false)
        }),
        ("absolute root layout", |root| {
            root["layout"] = json!("none")
        }),
        ("centered root", |root| {
            root["justifyContent"] = json!("center")
        }),
        ("desktop board", |root| root["width"] = json!(1440)),
    ];
    for (name, mutate) in cases {
        let mut root = root_value();
        root["clipContent"] = json!(true);
        mutate(&mut root);
        assert_eq!(repaired_rects(root, true), initial_rects(), "{name}");
    }
    let mut root = root_value();
    root["clipContent"] = json!(true);
    assert_eq!(
        repaired_rects(root, false),
        initial_rects(),
        "nested container"
    );
}

#[test]
fn pinned_scroll_keeps_native_minimum_height_and_height_dependent_alignment() {
    let mut root = root_value();
    root["children"][1]["minHeight"] = json!(700);
    let scene_for = |root: Value| {
        let state = EditorState::from_document(
            serde_json::from_value(json!({"version":"1.0.0", "children":[root]})).unwrap(),
        );
        crate::editor_state_to_layout_scene(&state)
    };
    let scene = scene_for(root);
    assert!(
        scene
            .active_page()
            .unwrap()
            .find("n257")
            .unwrap()
            .bounds
            .size
            .y
            >= 700.0
    );

    for justify in ["center", "end", "space_between", "space_around"] {
        let mut root = root_value();
        let scroll = &mut root["children"][1];
        scroll["justifyContent"] = json!(justify);
        scroll["children"][0]["height"] = json!(100);
        let scene = scene_for(root);
        let page = scene.active_page().unwrap();
        let viewport = page.find("n257").unwrap().bounds;
        let content = page.find("long-content").unwrap().bounds;
        assert_eq!(viewport.size.y, 722.0, "{justify}");
        assert_eq!(content.size.y, 100.0, "{justify}");
        let offset = content.origin.y - viewport.origin.y;
        let expected = match justify {
            "end" => 622.0,
            "center" | "space_around" => 311.0,
            _ => 0.0,
        };
        assert!(
            (offset - expected).abs() < 0.5,
            "{justify}: offset={offset}"
        );
        assert!(
            content.origin.y + content.size.y <= viewport.origin.y + viewport.size.y + 0.5,
            "{justify}: {content:?} clipped inside {viewport:?}"
        );
    }
}

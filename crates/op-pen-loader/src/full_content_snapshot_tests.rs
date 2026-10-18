use crate::{
    editor_state_to_active_page_export_layout_scene,
    editor_state_to_active_page_full_content_export_layout_scene,
    editor_state_to_active_page_layout_scene, pen_document_to_layout_scene,
};
use op_editor_core::EditorState;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn bottom(rect: op_editor_core::Rect) -> f32 {
    rect.origin.y + rect.size.y
}

fn fixture() -> Value {
    let reviews = json!({"type":"frame","id":"reviews","layout":"vertical","gap":12,
    "width":"fill_container","height":"fit_content","children":[
        {"type":"rectangle","id":"r1","width":"fill_container","height":119},
        {"type":"rectangle","id":"r2","width":"fill_container","height":119},
        {"type":"rectangle","id":"r3","width":"fill_container","height":119}
    ]});
    let scroll = json!({"type":"frame","id":"scroll","role":"scroll-area",
    "clipContent":true,"layout":"vertical","width":"fill_container",
    "height":"fill_container","gap":16,"padding":[0,0,24,0],
    "events":{"onScroll":[{"set":{"$state.scrolled":"true"}}]},
    "children":[
        {"type":"frame","id":"hero","layout":"none",
            "width":"fill_container","height":300,"clipContent":true,
            "children":[{"type":"rectangle","id":"next-slide",
                "x":375,"y":0,"width":375,"height":300}]},
        {"type":"rectangle","id":"info","width":"fill_container","height":162},
        {"type":"rectangle","id":"specs","width":"fill_container","height":189},
        reviews
    ]});
    json!({"version":"1.0.0", "children":[{
        "type":"frame","id":"phone","screen":"/","x":80,"y":40,
        "width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","width":"fill_container","height":62},
            scroll,
            {"type":"frame","id":"footer","width":"fill_container","height":114,
                "layout":"vertical","padding":[0,24,28,24],"children":[
                    {"type":"rectangle","id":"buy","width":"fill_container","height":48}
                ]}
        ]
    }, {"type":"frame","id":"cart","screen":"/cart","x":535,"y":40,
        "width":375,"height":812,"children":[]} ]})
}

fn state(value: &Value) -> EditorState {
    EditorState::from_document(jian_ops_schema::load_str(&value.to_string()).unwrap().value)
}

#[test]
fn full_content_contains_all_reviews_then_footer_without_changing_document_or_preview() {
    let state = state(&fixture());
    let original = serde_json::to_string(&state.doc).unwrap();
    let viewport = editor_state_to_active_page_export_layout_scene(&state);
    let full = editor_state_to_active_page_full_content_export_layout_scene(&state);
    let page = full.active_page().unwrap();
    let phone = page.find("phone").unwrap();
    let scroll = page.find("scroll").unwrap();
    let footer = page.find("footer").unwrap();
    let last = page.find("r3").unwrap();
    let viewport_page = viewport.active_page().unwrap();
    assert!(
        bottom(viewport_page.find("r3").unwrap().bounds)
            > bottom(viewport_page.find("scroll").unwrap().bounds),
        "the original viewport clips the last review"
    );
    assert!(phone.bounds.size.y > 1200.0);
    assert_eq!(phone.bounds.size.x, 375.0);
    assert_eq!(phone.bounds.origin.x, 80.0);
    assert_eq!(phone.bounds.origin.y, 40.0);
    assert_eq!(bottom(last.bounds) + 24.0, bottom(scroll.bounds));
    assert_eq!(footer.bounds.origin.y, bottom(scroll.bounds));
    assert_eq!(bottom(footer.bounds), bottom(phone.bounds));
    assert_eq!(footer.bounds.size.y, 114.0);
    assert_eq!(page.find("hero").unwrap().bounds.size.y, 300.0);
    assert!(page.find("hero").unwrap().clip_content);
    assert_eq!(
        page.find("cart"),
        viewport.active_page().unwrap().find("cart")
    );
    assert_eq!(serde_json::to_string(&state.doc).unwrap(), original);
    assert_eq!(
        editor_state_to_active_page_export_layout_scene(&state),
        viewport
    );
    assert_eq!(editor_state_to_active_page_layout_scene(&state), viewport);
    let preview = pen_document_to_layout_scene(&state.doc, &BTreeMap::new(), 0);
    assert_eq!(preview, viewport);
    assert_eq!(
        preview
            .active_page()
            .unwrap()
            .find("phone")
            .unwrap()
            .bounds
            .size
            .y,
        812.0
    );
}

#[test]
fn horizontal_carousels_clipped_cards_and_full_screen_maps_keep_their_viewports() {
    for role in [json!(null), json!("carousel"), json!("map")] {
        let mut doc = fixture();
        doc["children"][0]["children"][1]["role"] = role;
        let state = state(&doc);
        assert_eq!(
            editor_state_to_active_page_full_content_export_layout_scene(&state),
            editor_state_to_active_page_export_layout_scene(&state)
        );
    }
    let mut doc = fixture();
    doc["children"][0]["children"][1]["layout"] = json!("horizontal");
    let state = state(&doc);
    assert_eq!(
        editor_state_to_active_page_full_content_export_layout_scene(&state),
        editor_state_to_active_page_export_layout_scene(&state)
    );
}

#[test]
fn hidden_and_absolute_decorations_do_not_inflate_static_content() {
    let mut doc = fixture();
    let expected = editor_state_to_active_page_full_content_export_layout_scene(&state(&doc));
    let children = doc["children"][0]["children"][1]["children"]
        .as_array_mut()
        .unwrap();
    children.push(json!({"type":"rectangle","id":"hidden","visible":false,
        "width":10,"height":9000}));
    children.push(json!({"type":"rectangle","id":"absolute","x":0,"y":8000,
        "width":10,"height":100}));
    let full = editor_state_to_active_page_full_content_export_layout_scene(&state(&doc));
    assert_eq!(
        full.active_page().unwrap().find("phone").unwrap().bounds,
        expected
            .active_page()
            .unwrap()
            .find("phone")
            .unwrap()
            .bounds
    );
}

#[test]
fn preserving_import_geometry_and_no_overflow_do_not_relayout() {
    let mut imported = state(&fixture());
    imported.editor_ui.preserve_authored_geometry = true;
    assert_eq!(
        editor_state_to_active_page_full_content_export_layout_scene(&imported),
        editor_state_to_active_page_export_layout_scene(&imported)
    );
    let mut doc = fixture();
    doc["children"][0]["children"][1]["children"] = json!([
        {"type":"rectangle","id":"short","width":375,"height":100}
    ]);
    let state = state(&doc);
    assert_eq!(
        editor_state_to_active_page_full_content_export_layout_scene(&state),
        editor_state_to_active_page_export_layout_scene(&state)
    );
}

#[test]
fn desktop_columns_grow_together_and_short_side_panels_do_not_add_unneeded_height() {
    let mut doc = fixture();
    let phone = doc["children"][0].clone();
    doc["children"] = json!([{"type":"frame","id":"desktop","width":635,"height":812,
    "layout":"horizontal","children":[
        {"type":"frame","id":"sidebar","width":260,"height":"fill_container"},
        phone
    ]}]);
    doc["children"][0]["children"][1]
        .as_object_mut()
        .unwrap()
        .remove("x");
    doc["children"][0]["children"][1]
        .as_object_mut()
        .unwrap()
        .remove("y");
    let full = editor_state_to_active_page_full_content_export_layout_scene(&state(&doc));
    let page = full.active_page().unwrap();
    assert_eq!(
        page.find("desktop").unwrap().bounds.size.y,
        page.find("phone").unwrap().bounds.size.y
    );
    assert_eq!(
        page.find("sidebar").unwrap().bounds.size.y,
        page.find("desktop").unwrap().bounds.size.y
    );

    doc["children"][0]["height"] = json!(3000);
    let full = editor_state_to_active_page_full_content_export_layout_scene(&state(&doc));
    assert_eq!(
        full.active_page()
            .unwrap()
            .find("desktop")
            .unwrap()
            .bounds
            .size
            .y,
        3000.0
    );
    assert!(
        full.active_page()
            .unwrap()
            .find("phone")
            .unwrap()
            .bounds
            .size
            .y
            > 1200.0,
        "a descendant expansion survives even when its ancestor has enough room"
    );
}

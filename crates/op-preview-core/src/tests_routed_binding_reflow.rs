//! Routed binding relayout must keep paint pages paired with their own geometry.

use super::{test_measure, PreviewSession};
use serde_json::json;

fn fixture() -> jian_ops_schema::PenDocument {
    let children: Vec<_> = ["/", "/cart", "/receipt"]
        .iter()
        .enumerate()
        .map(|(i, route)| {
            json!({"type":"frame","id":format!("screen-{i}"),"screen":route,
            "x":i * 500,"width":375 + i * 15,"height":812 + i * 20,
            "layout":"vertical","padding":16,"gap":12,
            "children":[
                {"type":"text","id":format!("label-{i}"),"content":"Draft",
                    "width":"fit_content","height":"fit_content","fontSize":16,
                    "bindings":{"content":"$app.expanded ? 'Ready to check out' : 'Draft'"}},
                {"type":"frame","id":format!("toggle-{i}"),"width":120,"height":44,
                    "events":{"onTap":[{"toggle":"$app.expanded"}]},"children":[]}
            ]})
        })
        .collect();
    serde_json::from_value(json!({"version":"1.1",
        "state":{"expanded":{"type":"bool","default":false}},"children":children}))
    .unwrap()
}

fn enter(doc: &jian_ops_schema::PenDocument, preserve: bool) -> PreviewSession {
    enter_with_measure(doc, preserve, test_measure())
}

fn enter_with_measure(
    doc: &jian_ops_schema::PenDocument,
    preserve: bool,
    measure: std::rc::Rc<dyn jian_core::layout::measure::MeasureBackend>,
) -> PreviewSession {
    PreviewSession::enter(
        doc,
        (1200.0, 900.0),
        &Default::default(),
        0,
        preserve,
        false,
        measure,
        0,
    )
    .unwrap()
}

fn assert_mounted(session: &PreviewSession, index: usize, expanded: bool) {
    let (id, rect) = session.framed_root().unwrap();
    assert_eq!(id, format!("screen-{index}"));
    assert_eq!(rect.size.x, (375 + index * 15) as f32);
    assert_eq!(rect.size.y, (812 + index * 20) as f32);
    let scene = session.preview_scene_for_test();
    assert_eq!(scene.pages.len(), 3);
    assert_eq!(scene.active_page_index, index);
    assert_eq!(
        session
            .runtime()
            .document
            .as_ref()
            .unwrap()
            .schema
            .pages
            .as_ref()
            .unwrap()[0]
            .id,
        format!("screen-{index}"),
        "scene ordering must not reorder the mounted runtime document",
    );
    for (i, page) in scene.pages.iter().enumerate() {
        let root = &page.children[0];
        assert_eq!(root.id, format!("screen-{i}"));
        assert_eq!(root.bounds.size.x, (375 + i * 15) as f32);
        assert_eq!(root.bounds.size.y, (812 + i * 20) as f32);
    }
    let active = scene.active_page().unwrap();
    let label = active.find(&format!("label-{index}")).unwrap();
    assert_eq!(
        label.text.as_deref(),
        Some(if expanded {
            "Ready to check out"
        } else {
            "Draft"
        })
    );
    let runtime = session.current_screen_scene_rect().unwrap();
    assert_eq!(runtime.size, rect.size);
}

fn assert_text_geometry(session: &PreviewSession, index: usize, backend: &str) {
    let id = format!("label-{index}");
    let scene = session.preview_scene_for_test();
    let painted = scene.active_page().unwrap().find(&id).unwrap().bounds;
    let (x, y, w, h) = session.node_rect(&id).unwrap();
    assert!(
        (painted.origin.x - x).abs() <= 1.0
            && (painted.origin.y - y).abs() <= 1.0
            && (painted.size.x - w).abs() <= 1.0
            && (painted.size.y - h).abs() <= 1.0,
        "{backend} {id}: paint={painted:?}, runtime=({x},{y},{w},{h})"
    );
}

#[test]
fn routed_bound_text_keeps_all_page_geometry_after_switch_and_updates() {
    let doc = fixture();
    let original = serde_json::to_value(&doc).unwrap();
    for (backend, measure) in crate::tests::measure_backends() {
        let mut session = enter_with_measure(&doc, false, measure);
        assert_mounted(&session, 0, false);
        assert_text_geometry(&session, 0, backend);
        for (step, index) in [1, 2, 0, 2, 1].into_iter().enumerate() {
            let path = ["/", "/cart", "/receipt"][index];
            session.navigate_to_screen(path);
            assert!(session.reconcile(1000 * (step as u64 + 1)).switched);
            assert_mounted(&session, index, false);
            assert_text_geometry(&session, index, backend);
            session.set_state("expanded", json!(true));
            assert_mounted(&session, index, true);
            assert_text_geometry(&session, index, backend);
            session.set_state("expanded", json!(false));
            assert_mounted(&session, index, false);
            assert_text_geometry(&session, index, backend);
        }
    }
    assert_eq!(serde_json::to_value(&doc).unwrap(), original);
}

#[test]
fn routed_bound_text_retains_real_tap_hit_after_relayout() {
    let mut session = enter(&fixture(), false);
    session.navigate_to_screen("/cart");
    assert!(session.reconcile(100).switched);
    assert_mounted(&session, 1, false);
    for (now, expanded) in [(1000, true), (2000, false)] {
        session.set_now_ms(now);
        let scene = session.preview_scene_for_test();
        let rect = scene
            .active_page()
            .unwrap()
            .find("toggle-1")
            .unwrap()
            .bounds;
        assert!(session.dispatch_tap(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        ));
        assert_mounted(&session, 1, expanded);
    }
}

#[test]
fn routed_content_reflow_preserves_authored_geometry_page_pairing() {
    let mut session = enter(&fixture(), true);
    session.navigate_to_screen("/receipt");
    assert!(session.reconcile(100).switched);
    assert_mounted(&session, 2, false);
    session.set_state("expanded", json!(true));
    assert_mounted(&session, 2, true);
}

#[test]
fn routed_table_filter_reflow_keeps_the_target_page_mounted() {
    use op_editor_core::table_filter_contract::{ROW_MARKER, TABLE_MARKER};
    let mut value = serde_json::to_value(fixture()).unwrap();
    let cart = &mut value["children"][1]["children"];
    cart[0].as_object_mut().unwrap().remove("bindings");
    cart.as_array_mut().unwrap().push(json!({
        "type":"frame","id":"table","explain":TABLE_MARKER,
        "width":"fill_container","height":"fit_content","layout":"vertical",
        "children":[
            {"type":"frame","id":"row-a","explain":ROW_MARKER,"width":120,"height":40},
            {"type":"frame","id":"row-b","explain":ROW_MARKER,"width":120,"height":40,
             "bindings":{"visible":"$app.expanded"}}
        ]
    }));
    let doc = serde_json::from_value(value).unwrap();
    let mut session = enter(&doc, false);
    session.navigate_to_screen("/cart");
    assert!(session.reconcile(100).switched);
    for expanded in [false, true, false] {
        session.set_state("expanded", json!(expanded));
        assert_mounted(&session, 1, false);
        let scene = session.preview_scene_for_test();
        let table = scene.active_page().unwrap().find("table").unwrap();
        assert_eq!(table.children.len(), if expanded { 2 } else { 1 });
    }
}

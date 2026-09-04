use super::{test_measure, PreviewSession};
use jian_core::gesture::pointer::Modifiers;
use op_editor_ui::widgets::canvas_viewport_paint::tabs_active_index;

fn tabs_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
      "version":"1.1","formatVersion":"1.1","id":"x",
      "app":{"name":"x","version":"1","id":"x"},
      "children":[{
        "type":"tabs","id":"tabs","width":240,"height":180,"value":"overview",
        "tabs":[
          {"value":"overview","label":"Overview"},
          {"value":"details","label":"Details"}
        ],
        "children":[
          {"type":"frame","id":"overview-panel","width":240,"height":148},
          {"type":"frame","id":"details-panel","width":240,"height":148}
        ]
      }]
    }"##;
    jian_ops_schema::load_str(src)
        .expect("parse tabs doc")
        .value
}

#[test]
fn runtime_tab_switch_keeps_visual_and_hit_mapping_on_the_same_panel() {
    let mut session = PreviewSession::enter(
        &tabs_doc(),
        (800.0, 600.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter tabs preview");

    assert_eq!(
        session.mapped_child_ids_for_test("tabs"),
        vec!["overview-panel".to_owned()]
    );
    assert!(session.focus_node_for_test("tabs"));
    // Widget-action keys update state without emitting an authored semantic
    // event, so the host's bool may be false even though the tab switched.
    let _ = session.dispatch_key("ArrowRight", Modifiers::default());

    let scene = session.preview_scene_for_test();
    let tabs = scene.active_page().unwrap().find("tabs").unwrap();
    let active = tabs_active_index(tabs.widget.as_ref().unwrap());
    assert_eq!(tabs.children[active].id, "details-panel");
    assert_eq!(
        session.mapped_child_ids_for_test("tabs"),
        vec!["details-panel".to_owned()]
    );
}

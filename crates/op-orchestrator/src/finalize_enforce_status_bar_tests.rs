//! Tests for finalize-time status-bar contract enforcement.

use op_editor_core::{EditorState, PenNodeExt};

use crate::loop_finalize::StateDocSink;

/// Helper to build an EditorState from JSON and pass it through finalize enforcement.
fn state_from_json_and_enforce(json: &str) -> EditorState {
    let parsed = jian_ops_schema::load_str(json)
        .expect("fixture parses")
        .value;
    let mut state = EditorState::from_document(parsed);
    let mut sink = StateDocSink { state: &mut state };
    super::finalize_enforce_status_bar_contract(&mut sink);
    state
}

/// DeepSeek shape: 390-wide mobile root, first child is a plain header, no status bar.
/// Expected: canonical bar inserted at index 0, header becomes index 1.
#[test]
fn deepseek_shape_inserts_missing_status_bar() {
    let json = r##"{"version":"1.0","children":[
      {"type":"frame","id":"root","name":"Home","width":390,"height":844,
       "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
       "children":[
         {"type":"frame","id":"header","name":"Header","width":"fill_container","height":80,
          "layout":"none","fill":[{"type":"solid","color":"#F0F0F0"}],
          "children":[{"type":"text","id":"t","name":"Title","content":"Home",
            "width":100,"height":40,"fill":[{"type":"solid","color":"#111111ff"}]}]}
       ]}]}"##;

    let state = state_from_json_and_enforce(json);
    let root = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("root".to_string()),
    )
    .expect("root exists");

    let children = root.children().expect("root has children");
    assert!(
        children.len() >= 2,
        "root should have at least 2 children (status bar + header)"
    );

    let first_child = &children[0];
    assert_eq!(
        first_child.base().role.as_deref(),
        Some("status-bar"),
        "first child should be status bar with role"
    );
    assert!(
        first_child
            .children()
            .and_then(|c| c
                .iter()
                .find(|ch| ch.base().name.as_deref() == Some("Levels")))
            .is_some(),
        "status bar should have Levels child"
    );
    assert!(
        op_design_lint::detect_text_explicit_heights(first_child).is_empty(),
        "finalize-injected canonical status bar must not introduce text-explicit-height"
    );

    let second_child = &children[1];
    assert_eq!(
        second_child.id_str(),
        "header",
        "header should now be at index 1"
    );
}

/// Kimi K3 shape: status bar exists with children `time` + `status-icons` (non-canonical).
/// Expected: replaced by canonical bar (with Levels child, no status-icons).
#[test]
fn kimi_k3_shape_replaces_non_canonical_status_bar() {
    let json = r##"{"version":"1.0","children":[
      {"type":"frame","id":"root","name":"Home","width":390,"height":844,
       "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
       "children":[
         {"type":"frame","id":"sb","name":"status","role":"status-bar",
          "width":"fill_container","height":62,"layout":"none",
          "children":[
            {"type":"text","id":"time","name":"time","content":"9:41","width":54,"height":22,
             "fill":[{"type":"solid","color":"#111111ff"}]},
            {"type":"frame","id":"si","name":"status-icons","width":78,"height":14,"layout":"none",
             "children":[{"type":"rectangle","id":"icon1","width":10,"height":10,
               "fill":[{"type":"solid","color":"#111111ff"}]}]}
          ]},
         {"type":"frame","id":"body","name":"Body","width":"fill_container","height":600,
          "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
          "children":[{"type":"text","id":"t","name":"T","content":"Hello","width":100,
            "height":24,"fill":[{"type":"solid","color":"#111111ff"}]}]}
       ]}]}"##;

    let state = state_from_json_and_enforce(json);
    let root = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("root".to_string()),
    )
    .expect("root exists");

    let children = root.children().expect("root has children");
    assert!(
        !children.is_empty(),
        "root should have at least 1 child (status bar)"
    );

    let first_child = &children[0];
    assert_eq!(
        first_child.base().role.as_deref(),
        Some("status-bar"),
        "first child should have status-bar role"
    );

    let bar_children = first_child.children().expect("status bar has children");
    let has_levels = bar_children
        .iter()
        .any(|c| c.base().name.as_deref() == Some("Levels"));
    assert!(has_levels, "canonical bar should have Levels child");

    let has_status_icons = bar_children
        .iter()
        .any(|c| c.base().name.as_deref() == Some("status-icons"));
    assert!(
        !has_status_icons,
        "canonical bar should not have status-icons child"
    );
}

/// GLM-5.3 shape: already canonical (role="status-bar", child named "Levels").
/// Expected: untouched, zero commands emitted.
#[test]
fn glm_shape_leaves_canonical_bar_untouched() {
    let json = r##"{"version":"1.0","children":[
      {"type":"frame","id":"root","name":"Home","width":390,"height":844,
       "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
       "children":[
         {"type":"frame","id":"sb","name":"Status Bar","role":"status-bar",
          "width":"fill_container","height":62,"layout":"none",
          "children":[
            {"type":"frame","id":"time","name":"Time","x":34,"y":21,"width":54,"height":22,
             "layout":"none","children":[{"type":"text","id":"tl","name":"Time",
               "content":"9:41","width":54,"height":22,
               "fill":[{"type":"solid","color":"#ffffffff"}],"fontSize":17}]},
            {"type":"frame","id":"levels","name":"Levels","x":286,"y":24,"width":78,"height":14,
             "layout":"none","children":[{"type":"rectangle","id":"b","width":20,"height":10,
               "fill":[{"type":"solid","color":"#ffffffff"}]}]}
          ]},
         {"type":"frame","id":"body","name":"Body","width":"fill_container","height":600,
          "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
          "children":[{"type":"text","id":"t","name":"T","content":"Hello","width":100,
            "height":24,"fill":[{"type":"solid","color":"#111111ff"}]}]}
       ]}]}"##;

    let state = state_from_json_and_enforce(json);
    let root = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("root".to_string()),
    )
    .expect("root exists");

    let children = root.children().expect("root has children");
    let first_child = &children[0];
    assert_eq!(
        first_child.id_str(),
        "sb",
        "canonical bar should stay at index 0 with same id"
    );
    assert_eq!(
        first_child.base().role.as_deref(),
        Some("status-bar"),
        "status bar should keep role"
    );
}

/// Deck board (1920×1080) should not get a status bar.
/// Expected: untouched, zero commands emitted.
#[test]
fn deck_board_does_not_get_status_bar() {
    let json = r##"{"version":"1.0","children":[
      {"type":"frame","id":"board","name":"Slide 1","width":1920,"height":1080,
       "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
       "children":[
         {"type":"frame","id":"content","name":"Content","width":400,"height":300,
          "layout":"none","fill":[{"type":"solid","color":"#F0F0F0"}],
          "children":[{"type":"text","id":"t","name":"Title","content":"Slide",
            "width":200,"height":60,"fill":[{"type":"solid","color":"#111111ff"}]}]}
       ]}]}"##;

    let state = state_from_json_and_enforce(json);
    let root = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("board".to_string()),
    )
    .expect("root exists");

    let children = root.children().expect("root has children");
    let first_child = &children[0];
    assert_eq!(
        first_child.id_str(),
        "content",
        "deck board should keep its original first child, no status bar added"
    );
}

/// Web page (1440-wide, 900-tall) should not get a status bar.
/// Expected: untouched, zero commands emitted.
#[test]
fn web_page_does_not_get_status_bar() {
    let json = r##"{"version":"1.0","children":[
      {"type":"frame","id":"web","name":"Dashboard","width":1440,"height":900,
       "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
       "children":[
         {"type":"frame","id":"header","name":"Header","width":"fill_container","height":60,
          "layout":"none","fill":[{"type":"solid","color":"#F0F0F0"}],
          "children":[{"type":"text","id":"h","name":"Title","content":"Dashboard",
            "width":200,"height":30,"fill":[{"type":"solid","color":"#111111ff"}]}]}
       ]}]}"##;

    let state = state_from_json_and_enforce(json);
    let root = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("web".to_string()),
    )
    .expect("root exists");

    let children = root.children().expect("root has children");
    let first_child = &children[0];
    assert_eq!(
        first_child.id_str(),
        "header",
        "web page should keep its original first child, no status bar added"
    );
}

/// Non-mobile 1200×800 root (desktop/web) must NOT get a status bar.
/// Expected: untouched, zero commands emitted.
#[test]
fn non_mobile_1200x800_root_does_not_get_status_bar() {
    let json = r##"{"version":"1.0","children":[
      {"type":"frame","id":"desktop","name":"Dashboard","width":1200,"height":800,
       "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
       "children":[
         {"type":"frame","id":"header","name":"Header","width":"fill_container","height":60,
          "layout":"none","fill":[{"type":"solid","color":"#F0F0F0"}],
          "children":[{"type":"text","id":"h","name":"Title","content":"Dashboard",
            "width":200,"height":30,"fill":[{"type":"solid","color":"#111111ff"}]}]}
       ]}]}"##;

    let state = state_from_json_and_enforce(json);
    let root = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("desktop".to_string()),
    )
    .expect("root exists");

    let children = root.children().expect("root has children");
    let first_child = &children[0];
    assert_eq!(
        first_child.id_str(),
        "header",
        "1200px-wide root should keep original first child, no status bar"
    );
}

/// The existing test that must stay green: a generated iOS screen with a canonical bar.
#[test]
fn finalize_never_paints_the_status_bar_with_the_accent() {
    let json = r##"{"version":"1.0","children":[{"type":"frame","id":"root","name":"Home",
      "width":390,"height":844,"layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
      "children":[
        {"type":"frame","id":"sb","name":"Status Bar","role":"status-bar",
         "width":"fill_container","height":62,"layout":"none","children":[
          {"type":"frame","id":"tw","name":"Time","x":34,"y":21,"width":54,"height":22,
           "layout":"none","children":[{"type":"text","id":"tl","name":"Time","content":"9:41",
             "width":54,"height":22,"fill":[{"type":"solid","color":"#ffffffff"}],"fontSize":17}]},
          {"type":"frame","id":"lv","name":"Levels","x":286,"y":24,"width":78,"height":14,
           "layout":"none","children":[{"type":"rectangle","id":"b1","x":0,"y":0,
             "width":20,"height":10,"fill":[{"type":"solid","color":"#ffffffff"}]}]}]},
        {"type":"frame","id":"body","name":"Body","width":"fill_container","height":600,
         "layout":"vertical","fill":[{"type":"solid","color":"#FFFFFF"}],
         "children":[{"type":"text","id":"t","name":"T","content":"Hello","width":100,
           "height":24,"fill":[{"type":"solid","color":"#111111ff"}]}]}
      ]}]}"##;

    let state = state_from_json_and_enforce(json);
    let bar = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("sb".to_string()),
    )
    .cloned()
    .expect("status bar survives finalize");

    assert_eq!(
        op_editor_core::first_solid_fill_hex(&bar),
        None,
        "OS chrome must stay fill-less so it shows the screen behind it"
    );
}

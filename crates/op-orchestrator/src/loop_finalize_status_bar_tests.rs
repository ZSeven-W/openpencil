//! OS status-bar chrome must reach the end of finalize unpainted.
//!
//! Split from `loop_finalize_tests.rs`, which is at the 800-line ceiling.

use op_editor_core::EditorState;

/// A generated iOS screen: light page, fill-less status bar carrying white
/// glyphs, one body section below it.
fn light_screen_with_status_bar() -> EditorState {
    let doc = r##"{"version":"1.0","children":[{"type":"frame","id":"root","name":"Home",
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
    let parsed = jian_ops_schema::load_str(doc)
        .expect("fixture parses")
        .value;
    EditorState::from_document(parsed)
}

/// The invisible-text-band heuristic paints a fill-less frame whose text is all
/// light with the design accent. The injected status bar is fill-less by
/// contract and carries white glyphs, so on a light page it matched that shape
/// and shipped as an accent-coloured band across the top of generated iOS
/// screens. Asserted through the whole of finalize because the defect lives in
/// reachability: the bar reaches that heuristic only when a single root's
/// children become the section forest.
#[test]
fn finalize_never_paints_the_status_bar_with_the_accent() {
    let mut state = light_screen_with_status_bar();
    crate::apply_loop_finalize(&mut state);
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

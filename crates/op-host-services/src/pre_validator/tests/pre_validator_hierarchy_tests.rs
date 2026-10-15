use super::{LintPreValidator, TestSink};
use op_orchestrator::{DocSink, PreValidator};
use serde_json::json;

#[test]
fn pre_validation_keeps_display_type_and_fixed_avatar_geometry() {
    let doc = serde_json::from_value(json!({"version":"1.0","children":[
        {"type":"frame","id":"screen","width":375,"height":812,"layout":"vertical","children":[
            {"type":"frame","id":"eta","layout":"horizontal","children":[
                {"type":"text","id":"prefix","content":"预计","fontSize":15,"fontFamily":"Inter","fontWeight":400},
                {"type":"text","id":"value","content":"12","fontSize":34,"fontFamily":"JetBrains Mono","fontWeight":700},
                {"type":"text","id":"suffix","content":"分钟送达","fontSize":15,"fontFamily":"Inter","fontWeight":400}
            ]},
            {"type":"frame","id":"rider","layout":"horizontal","children":[
                {"type":"frame","id":"avatar","width":44,"height":44,"cornerRadius":22,
                 "layout":"horizontal","children":[{"type":"icon_font","id":"icon","iconFontName":"user","width":20,"height":20}]},
                {"type":"frame","id":"info","height":"fit_content","layout":"vertical"},
                {"type":"frame","id":"actions","height":"fit_content","layout":"horizontal"}
            ]}
        ]}
    ]})).unwrap();
    let mut sink = TestSink::from_doc(doc);
    LintPreValidator.run_pre_validation_fixes(&mut sink);
    for (id, property, expected) in [("value", "fontSize", 34.0), ("avatar", "height", 44.0)] {
        let node = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &op_editor_core::NodeId::new(id),
        )
        .expect("node survives pre-validation");
        assert_eq!(
            serde_json::to_value(node).unwrap()[property].as_f64(),
            Some(expected)
        );
    }
}

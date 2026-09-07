use super::{LintPreValidator, TestSink};
use op_orchestrator::{DocSink, PreValidator};

#[test]
fn pre_validation_preserves_coloured_bleed_section_padding() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{
            "version":"1.0",
            "children":[{
                "type":"frame","id":"root","width":375,"height":812,
                "layout":"vertical",
                "children":[
                    {"type":"frame","id":"status","role":"status-bar","height":44},
                    {
                        "type":"frame","id":"map","name":"Map (bleed)",
                        "width":"fill_container","layout":"vertical","padding":[0,0],
                        "children":[
                            {
                                "type":"frame","id":"map-placeholder",
                                "width":"fill_container",
                                "fill":[{"type":"solid","color":"#223344"}]
                            },
                            {
                                "type":"frame","id":"map-inset","padding":[0,24],
                                "children":[{"type":"text","id":"map-title","content":"Map"}]
                            }
                        ]
                    },
                    {"type":"frame","id":"section-a","padding":[0,24,0,24],
                     "children":[{"type":"text","id":"a-title","content":"A"}]},
                    {"type":"frame","id":"section-b","padding":[0,24,0,24],
                     "children":[{"type":"text","id":"b-title","content":"B"}]},
                    {"type":"frame","id":"section-c","padding":[0,24,0,24],
                     "children":[{"type":"text","id":"c-title","content":"C"}]},
                    {"type":"frame","id":"tabs","role":"bottom-tab-bar","height":72}
                ]
            }]
        }"##,
    )
    .expect("bleed pre-validation fixture");
    let mut sink = TestSink::from_doc(doc);

    LintPreValidator.run_pre_validation_fixes(&mut sink);

    let map = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &op_editor_core::NodeId::new("map"),
    )
    .expect("Map (bleed) exists");
    assert_eq!(
        serde_json::to_value(map).expect("map serializes")["padding"],
        serde_json::json!([0.0, 0.0])
    );
}

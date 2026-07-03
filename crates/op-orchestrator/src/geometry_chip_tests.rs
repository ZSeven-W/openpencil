use crate::geometry_validation::geometry_diagnostics;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::{json, Value};

fn insert_root(value: Value) -> VecDocSink {
    let root: PenNode = serde_json::from_value(value).expect("valid root");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink
}

#[test]
fn diagnostics_skip_jam_for_all_fill_tab_cells_but_keep_mixed_jams() {
    let fill_tabs = insert_root(json!({
        "type":"frame","id":"root","name":"Root","width":300,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"tabs","name":"Tabs","width":300,"height":64,"layout":"horizontal","gap":0,"children":[
                {"type":"frame","id":"home","name":"Home Tab","width":"fill_container","height":54,"children":[{"type":"text","id":"ht","content":"Home"}]},
                {"type":"frame","id":"search","name":"Search Tab","width":"fill_container","height":54,"children":[{"type":"text","id":"st","content":"Search"}]},
                {"type":"frame","id":"profile","name":"Profile Tab","width":"fill_container","height":54,"children":[{"type":"text","id":"pt","content":"Profile"}]}
            ]}
        ]
    }));
    let fill_out = geometry_diagnostics(fill_tabs.state());
    assert!(
        fill_out
            .iter()
            .all(|line| !line.contains("text columns touch")),
        "all-fill tab cells should not report jam: {fill_out:?}"
    );

    let mixed = insert_root(json!({
        "type":"frame","id":"root","name":"Root","width":300,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"row","name":"Data Row","width":300,"height":48,"layout":"horizontal","gap":0,"children":[
                {"type":"frame","id":"date","name":"Date","width":120,"height":40,"children":[{"type":"text","id":"dt","content":"Oct 24"}]},
                {"type":"frame","id":"count","name":"Count","width":"fill_container","height":40,"children":[{"type":"text","id":"ct","content":"42"}]}
            ]}
        ]
    }));
    let mixed_out = geometry_diagnostics(mixed.state());
    assert!(
        mixed_out
            .iter()
            .any(|line| line.contains("Date") && line.contains("text columns touch")),
        "mixed jam must still report: {mixed_out:?}"
    );
}

#[test]
fn diagnostics_skip_single_line_pill_text_overflow() {
    let chip = insert_root(json!({
        "type":"frame","id":"root","name":"Root","width":180,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"chip","name":"Guest Chip","width":60,"height":40,"layout":"horizontal","cornerRadius":9999,"children":[
                {"type":"text","id":"guest-text","name":"Guest Text","content":"2 Guests, 1 Room","width":"fit_content","textGrowth":"auto"}
            ]}
        ]
    }));

    let out = geometry_diagnostics(chip.state());

    assert!(
        out.iter().all(|line| !line.contains("Guest Text")),
        "single-line pill text overflow is intentional clipping, not a reportable wrap issue: {out:?}"
    );
}

//! End-to-end replays of measured weak-model programs through the real
//! program-gen path (`run_program_to_forest`).

use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;

use super::run_program_to_forest;

/// Replay of the GLM-5.3-Flash w01 sidebar (design-arena-v1
/// glm-5-3-flash-0926d, arena-w01 run-1): the model parked its leaves in a
/// staging `sidebar-spacer`, then created each container listing them by
/// handle. The 11 container lines the executor dropped are reproduced from
/// the logged previews (their truncated tails reduced to minimal
/// equivalents); the leaf and shell lines, which the log does not show, are
/// reconstructed from the shipped document, where every one of these leaves
/// sat loose in the spacer.
const W01_SIDEBAR: &str = r##"b1=I(null, {"type":"frame","name":"Sidebar Project List","layout":"vertical","width":260,"height":900})
bs=I(b1, {"type":"frame","name":"sidebar-spacer","layout":"vertical","width":"fill_container","height":"fill_container"})
b2=I(bs, {"type":"icon_font","iconFontName":"layers","width":16,"height":16})
b9=I(bs, {"type":"text","content":"●","fontSize":8})
b11=I(bs, {"type":"text","content":"●","fontSize":8})
b13=I(bs, {"type":"text","content":"●","fontSize":8})
b24=I(bs, {"type":"text","content":"18","fontSize":11})
b29=I(bs, {"type":"text","content":"12","fontSize":11})
b34=I(bs, {"type":"text","content":"7","fontSize":11})
b39=I(bs, {"type":"text","content":"23","fontSize":11})
b44=I(bs, {"type":"text","content":"4","fontSize":11})
b52=I(bs, {"type":"text","content":"林","fontSize":14})
b54=I(bs, {"type":"text","content":"林晓雨","fontSize":13})
b55=I(bs, {"type":"text","content":"产品负责人","fontSize":11})
bb=I(b1, {"type":"frame","name":"brand-row","layout":"horizontal","width":"fill_container","height":64})
b3=I(bb, {"type":"frame","name":"brand-mark","width":28,"height":28,"cornerRadius":8,"layout":"horizontal","alignItems":"center","justifyContent":"center","fill":[{"type":"solid","color":"#4F46E5"}],"children":["b2"]})
b8=I(b1, {"type":"frame","name":"workspace-members","layout":"horizontal"})
b10=I(b8, {"type":"frame","width":20,"height":20,"cornerRadius":10,"layout":"horizontal","alignItems":"center","justifyContent":"center","stroke":{"thickness":2,"fill":[{"type":"solid","color":"#FFFFFF"}]},"children":["b9"]})
b12=I(b8, {"type":"frame","width":20,"height":20,"cornerRadius":10,"layout":"horizontal","alignItems":"center","justifyContent":"center","stroke":{"thickness":2,"fill":[{"type":"solid","color":"#FFFFFF"}]},"children":["b11"]})
b14=I(b8, {"type":"frame","width":20,"height":20,"cornerRadius":10,"layout":"horizontal","alignItems":"center","justifyContent":"center","stroke":{"thickness":2,"fill":[{"type":"solid","color":"#FFFFFF"}]},"children":["b13"]})
bl=I(b1, {"type":"frame","name":"project-list","layout":"vertical","width":"fill_container"})
b21=I(bl, {"type":"frame","name":"project-item-a","layout":"horizontal","width":"fill_container","height":40})
b26=I(bl, {"type":"frame","name":"project-item-b","layout":"horizontal","width":"fill_container","height":40})
b31=I(bl, {"type":"frame","name":"project-item-c","layout":"horizontal","width":"fill_container","height":40})
b36=I(bl, {"type":"frame","name":"project-item-d","layout":"horizontal","width":"fill_container","height":40})
b41=I(bl, {"type":"frame","name":"project-item-e","layout":"horizontal","width":"fill_container","height":40})
b25=I(b21, {"type":"frame","width":"fit_content","height":20,"cornerRadius":10,"layout":"horizontal","alignItems":"center","justifyContent":"center","padding":[0,8],"fill":[{"type":"solid","color":"#4F46E5"}],"children":["b24"]})
b30=I(b26, {"type":"frame","width":"fit_content","height":20,"cornerRadius":10,"layout":"horizontal","alignItems":"center","justifyContent":"center","padding":[0,8],"fill":[{"type":"solid","color":"#F1F5F9"}],"children":["b29"]})
b35=I(b31, {"type":"frame","width":"fit_content","height":20,"cornerRadius":10,"layout":"horizontal","alignItems":"center","justifyContent":"center","padding":[0,8],"fill":[{"type":"solid","color":"#F1F5F9"}],"children":["b34"]})
b40=I(b36, {"type":"frame","width":"fit_content","height":20,"cornerRadius":10,"layout":"horizontal","alignItems":"center","justifyContent":"center","padding":[0,8],"fill":[{"type":"solid","color":"#F1F5F9"}],"children":["b39"]})
b45=I(b41, {"type":"frame","width":"fit_content","height":20,"cornerRadius":10,"layout":"horizontal","alignItems":"center","justifyContent":"center","padding":[0,8],"fill":[{"type":"solid","color":"#F1F5F9"}],"children":["b44"]})
b51=I(b1, {"type":"frame","name":"user-profile","layout":"horizontal","width":"fill_container"})
b53=I(b51, {"type":"frame","name":"user-avatar","width":36,"height":36,"cornerRadius":18,"layout":"horizontal","alignItems":"center","justifyContent":"center","fill":[{"type":"solid","color":"#6366F1"}],"children":["b52"]})
b56=I(b51, {"type":"frame","name":"user-meta","layout":"vertical","width":"fill_container","gap":2,"children":["b54","b55"]})"##;

fn find<'a>(node: &'a PenNode, name: &str) -> Option<&'a PenNode> {
    if node.base().name.as_deref() == Some(name) {
        return Some(node);
    }
    node.children()?.iter().find_map(|c| find(c, name))
}

fn texts(node: &PenNode) -> Vec<String> {
    node.children()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter(|c| matches!(c, PenNode::Text(_)))
        .filter_map(|c| {
            serde_json::to_value(c)
                .ok()
                .and_then(|v| v["content"].as_str().map(str::to_string))
        })
        .collect()
}

#[test]
fn w01_sidebar_handle_children_leave_no_loose_leaves() {
    let (nodes, _state) = run_program_to_forest(W01_SIDEBAR).expect("sidebar builds");
    let sidebar = &nodes[0];
    let named = |name: &str| find(sidebar, name).unwrap_or_else(|| panic!("{name} missing"));

    assert!(
        named("sidebar-spacer")
            .children()
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .is_empty(),
        "loose leaves left behind in the staging frame"
    );
    assert_eq!(named("brand-mark").children().map(Vec::len), Some(1));
    assert_eq!(texts(named("user-avatar")), vec!["林"]);
    assert_eq!(texts(named("user-meta")), vec!["林晓雨", "产品负责人"]);
    let rings = named("workspace-members").children().expect("rings");
    assert_eq!(rings.len(), 3);
    for ring in rings {
        assert_eq!(texts(ring), vec!["●"]);
    }
    let counts: Vec<Vec<String>> = named("project-list")
        .children()
        .expect("rows")
        .iter()
        .map(|row| texts(&row.children().expect("row holds its pill")[0]))
        .collect();
    assert_eq!(
        counts,
        vec![vec!["18"], vec!["12"], vec!["7"], vec!["23"], vec!["4"]]
    );
}

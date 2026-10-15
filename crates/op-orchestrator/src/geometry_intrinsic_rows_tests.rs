use super::*;
use crate::test_support::VecDocSink;
use op_editor_core::PenNodeExt;
use serde_json::json;

fn eta_fixture(width: f64) -> Value {
    let mut steps = Vec::new();
    for (i, label) in ["已接单", "取餐中", "配送中"].iter().enumerate() {
        if i > 0 {
            steps.push(json!({"type":"rectangle","id":format!("line{i}"),"width":16,"height":1}));
        }
        steps.push(json!({"type":"frame","id":format!("step{i}"),"name":format!("step{i}"),
            "layout":"horizontal","width":"fit_content","height":"fit_content","gap":6,
            "padding":[5,10],"cornerRadius":3,"children":[
                {"type":"icon_font","id":format!("icon{i}"),"width":12,"height":12,"iconFontName":"check"},
                {"type":"text","id":format!("label{i}"),"name":format!("label{i}"),"content":label,
                 "fontSize":11,"lineHeight":1.2,"textGrowth":"auto"}]}));
    }
    json!({"type":"frame","id":"root","name":"Root","width":width,"height":"fit_content","layout":"vertical",
        "children":[{"type":"frame","id":"eta","name":"ETA Row","width":"fill_container","height":"fit_content",
        "layout":"horizontal","alignItems":"end","justifyContent":"space_between","children":[
            {"type":"frame","id":"value","name":"ETA Value Group","width":"fit_content","height":"fit_content","layout":"horizontal","gap":8,"children":[
                {"type":"text","id":"eta-value","content":"12分钟","fontSize":34,"fontWeight":700,"textGrowth":"auto"},
                {"type":"text","id":"eta-label","content":"预计送达","fontSize":13,"textGrowth":"auto"}]},
            {"type":"frame","id":"steps","name":"Progress Steps","width":"fit_content","height":"fit_content","layout":"horizontal","children":steps}]}]})
}

fn sink(root: Value) -> (VecDocSink, String) {
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![serde_json::from_value(root).unwrap()],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let id = sink.state.active_children()[0].id_str().to_string();
    (sink, id)
}
fn by_name<'a>(v: &'a Value, name: &str) -> &'a Value {
    fn find<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
        if v.get("name").and_then(Value::as_str) == Some(name) {
            return Some(v);
        }
        children(v).iter().find_map(|c| find(c, name))
    }
    find(v, name).unwrap()
}
fn doc(sink: &VecDocSink) -> Value {
    serde_json::to_value(&sink.state.active_children()[0]).unwrap()
}
fn bounds(v: &Value, rects: &HashMap<String, Rect>) -> Rect {
    rects[v["id"].as_str().unwrap()]
}

#[test]
fn intrinsic_eta_groups_stack_before_labels_can_be_shrunk_or_clipped() {
    let (mut sink, id) = sink(eta_fixture(327.0));
    geometry_validate_and_fix(&mut sink, &id);
    let v = doc(&sink);
    let rects = resolved_rects(sink.state());
    let row = by_name(&v, "ETA Row");
    assert_eq!(layout_str(row), Some("vertical"));
    let root = bounds(&v, &rects);
    for name in ["label0", "label1", "label2"] {
        let label = by_name(&v, name);
        let r = bounds(label, &rects);
        assert!(r.h < 20.0, "one line label {name}: {r:?}");
        assert!(
            r.x + r.w <= root.x + root.w + 2.0,
            "label stays inside root: {r:?}"
        );
        assert_ne!(
            label.get("textGrowth").and_then(Value::as_str),
            Some("fixed-width")
        );
    }
    let before = v;
    geometry_validate_and_fix(&mut sink, &id);
    assert_eq!(
        doc(&sink),
        before,
        "repair converges without oscillating axes"
    );
}

#[test]
fn intrinsic_groups_preserve_fitting_wide_rows_scrollers_and_tables() {
    let (mut wide, id) = sink(eta_fixture(600.0));
    geometry_validate_and_fix(&mut wide, &id);
    assert_eq!(
        layout_str(by_name(&doc(&wide), "ETA Row")),
        Some("horizontal")
    );
    for kind in ["scroll", "absolute", "table", "fixed-height", "interactive"] {
        let mut v = eta_fixture(327.0);
        match kind {
            "scroll" => v["name"] = json!("Horizontal carousel"),
            "absolute" => v["layout"] = json!("none"),
            "table" => {
                let content = v["children"][0].clone();
                let row = json!({"type":"frame","layout":"horizontal","children":[
                    {"type":"frame","width":327,"layout":"vertical","children":[content]},
                    {"type":"text","width":80,"content":"Owner"},
                    {"type":"text","width":80,"content":"Active"}]});
                v["children"] = json!([row.clone(), row]);
                assert!(is_table_shape(&v));
            }
            "fixed-height" => v["children"][0]["height"] = json!(44),
            "interactive" => v["children"][0]["children"][1]["events"] = json!([]),
            _ => unreachable!(),
        }
        // Collector scope exclusions use the same measured geometry; duplicate
        // row ids in this table-only predicate fixture are never inserted.
        let (s, _) = sink(eta_fixture(327.0));
        let measured = doc(&s);
        let mut rects = resolved_rects(s.state());
        fn copy_rects(a: &Value, b: &Value, rects: &mut HashMap<String, Rect>) {
            if let (Some(a), Some(b)) = (
                a.get("id").and_then(Value::as_str),
                b.get("id").and_then(Value::as_str),
            ) {
                if let Some(rect) = rects.get(a).copied() {
                    rects.insert(b.into(), rect);
                }
            }
            for (a, b) in children(a).iter().zip(children(b)) {
                copy_rects(a, b, rects)
            }
        }
        copy_rects(&measured, &v, &mut rects);
        let mut cmds = Vec::new();
        collect_reflows(&v, &rects, &mut cmds);
        assert!(cmds.is_empty(), "{kind}: {cmds:?}");
    }
}

#[test]
fn direct_name_and_status_reflow_before_badge_clip_with_actions_preserved() {
    let v = json!({"type":"frame","id":"root","width":303,"height":"fit_content","layout":"horizontal","gap":12,
      "children":[
        {"type":"frame","id":"avatar","name":"avatar","width":48,"height":48,"layout":"horizontal","alignItems":"center","justifyContent":"center","cornerRadius":24,"fill":[{"type":"solid","color":"#2A3B55"}],"children":[{"type":"icon_font","id":"avatar-icon","width":22,"height":22,"iconFontName":"user"}]},
        {"type":"frame","id":"info","width":"fill_container","height":"fit_content","layout":"vertical","children":[
          {"type":"frame","id":"name-line","name":"name-line","width":"fill_container","height":"fit_content","layout":"horizontal","gap":8,"children":[
            {"type":"text","id":"name","name":"name","content":"李师傅","fontSize":16,"width":"fit_content"},
            {"type":"frame","id":"badge","name":"badge","width":"fit_content","height":"fit_content","layout":"horizontal","gap":4,"padding":[3,8],"cornerRadius":999,"fill":[{"type":"solid","color":"#F9731615"}],"children":[
              {"type":"ellipse","id":"dot","width":6,"height":6},
              {"type":"text","id":"status","name":"status","content":"正在为您配送","fontSize":11,"width":"fit_content"}]}]}]},
        {"type":"frame","id":"actions","name":"actions","width":"fit_content","height":"fit_content","layout":"horizontal","gap":10,"children":[
          {"type":"frame","id":"call","name":"call","width":44,"height":44,"layout":"horizontal","alignItems":"center","justifyContent":"center","cornerRadius":22,"fill":[{"type":"solid","color":"#F97316"}],"children":[{"type":"icon_font","id":"phone","width":18,"height":18,"iconFontName":"phone"}]},
          {"type":"frame","id":"message","name":"message","width":44,"height":44,"layout":"horizontal","alignItems":"center","justifyContent":"center","cornerRadius":22,"fill":[{"type":"solid","color":"#1A2A42"}],"children":[{"type":"icon_font","id":"message-icon","width":18,"height":18,"iconFontName":"message-circle"}]}]}]});
    let (mut sink, id) = sink(v);
    repair_card_inner_padding(&mut sink, &id);
    geometry_validate_and_fix(&mut sink, &id);
    let v = doc(&sink);
    let rects = resolved_rects(sink.state());
    assert_eq!(layout_str(by_name(&v, "name-line")), Some("vertical"));
    let badge = by_name(&v, "badge");
    let b = bounds(badge, &rects);
    let t = bounds(by_name(&v, "status"), &rects);
    assert!(
        t.x + t.w <= b.x + b.w + 1.0 && t.h < 20.0,
        "status fully readable: {t:?} {b:?}"
    );
    assert_ne!(badge.get("clipContent"), Some(&json!(true)));
    for name in ["avatar", "call", "message"] {
        assert_eq!(
            horizontal_padding(by_name(&v, name)),
            0.0,
            "no accidental card padding"
        );
    }
    assert_eq!(bounds(by_name(&v, "call"), &rects).w, 44.0);
    assert_eq!(bounds(by_name(&v, "actions"), &rects).w, 98.0);
}

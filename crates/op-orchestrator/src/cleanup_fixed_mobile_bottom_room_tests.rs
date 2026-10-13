//! Fixed phone screens reserve a closing inset before spending interior slack.
use super::*;

fn phone(gap: f64, body_height: f64) -> EditorState {
    state(json!({
        "type":"frame","id":"root","x":80,"y":40,"width":375,"height":812,"layout":"vertical","gap":gap,
        "children":[
            {"type":"frame","id":"status","role":"status-bar","width":"fill_container","height":62},
            {"type":"frame","id":"body","width":"fill_container","height":body_height,"clipContent":true,
             "children":[{"type":"text","id":"title","content":"Now playing","fontSize":24}]},
            {"type":"frame","id":"actions","width":"fill_container","height":60,"layout":"horizontal",
             "children":[{"type":"frame","id":"play","role":"button","width":48,"height":48,
                "children":[{"type":"text","id":"label","content":"Play","fontSize":16}]}]}
        ]
    }))
}

#[test]
fn no_nav_phone_reserves_bottom_room_without_scaling_its_content() {
    for gap in [20.0, 36.0] {
        let mut draft = phone(gap, 648.0);
        let before = draft.doc.clone();
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
        repair(&mut sink, "root");
        let gap =
            crate::geometry_validation::fixed_mobile_bottom_gap(sink.state(), "root").unwrap();
        assert!(gap >= 27.0, "closing gap = {gap}");
        for id in ["status", "body", "actions"] {
            assert_eq!(
                op_editor_core::walkers::find_node(&before.children, &NodeId::new(id)),
                op_editor_core::walkers::find_node(
                    sink.state().active_children(),
                    &NodeId::new(id)
                ),
                "business content and controls must not be scaled"
            );
        }
        assert_eq!(
            find_root(sink.state(), "root").unwrap().height_px(),
            Some(812.0)
        );
        let settled = sink.state().doc.clone();
        repair(&mut sink, "root");
        assert_eq!(sink.state().doc, settled, "second pass is idempotent");
    }
}

#[test]
fn no_recoverable_space_does_not_add_padding_or_clip_business_content() {
    let mut draft = phone(4.0, 680.0);
    let before = draft.doc.clone();
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    repair(&mut sink, "root");
    assert_eq!(sink.state().doc, before);
}

#[test]
fn descendant_token_padding_is_preserved_while_root_gap_supplies_room() {
    let mut draft = phone(20.0, 648.0);
    draft.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("body"),
        patch_json: json!({"padding":"$--inset","layout":"vertical"}).to_string(),
        page_id: None,
    });
    let before = draft.doc.clone();
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    repair(&mut sink, "root");
    assert!(
        crate::geometry_validation::fixed_mobile_bottom_gap(sink.state(), "root").unwrap() >= 27.0
    );
    assert_eq!(
        op_editor_core::walkers::find_node(&before.children, &NodeId::new("body")),
        op_editor_core::walkers::find_node(sink.state().active_children(), &NodeId::new("body"))
    );
}

#[test]
fn fixed_control_row_yields_blank_height_without_resizing_buttons() {
    let mut draft = phone(4.0, 598.0);
    draft.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("actions"),
        patch_json: json!({"height":110,"alignItems":"center","children":[
            {"type":"frame","id":"back","role":"button","width":48,"height":48},
            {"type":"frame","id":"play","role":"button","width":64,"height":64},
            {"type":"frame","id":"next","role":"button","width":48,"height":48}
        ]})
        .to_string(),
        page_id: None,
    });
    let mut root = serde_json::to_value(&draft.active_children()[0]).unwrap();
    root["children"].as_array_mut().unwrap().push(json!({
        "type":"text","id":"footer","content":"Download · Share","height":24,"fontSize":12
    }));
    draft = state(root);
    let before = draft.doc.clone();
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    repair(&mut sink, "root");
    assert!(
        crate::geometry_validation::fixed_mobile_bottom_gap(sink.state(), "root").unwrap() >= 27.0
    );
    for id in ["back", "play", "next"] {
        assert_eq!(
            op_editor_core::walkers::find_node(&before.children, &NodeId::new(id)),
            op_editor_core::walkers::find_node(sink.state().active_children(), &NodeId::new(id))
        );
    }
    let settled = sink.state().doc.clone();
    repair(&mut sink, "root");
    assert_eq!(settled, sink.state().doc);
}

#[test]
fn painted_or_clipped_control_rows_are_not_treated_as_blank_height() {
    for extra in [
        json!({"fill":[{"type":"solid","color":"#222222"}]}),
        json!({"clipContent":true}),
        json!({"minHeight":110}),
        json!({"position":"absolute"}),
        json!({"rotation":5}),
        json!({"reusable":true}),
    ] {
        let mut row = json!({"type":"frame","id":"actions","height":110,"layout":"horizontal","alignItems":"center","children":[
            {"type":"frame","id":"a","role":"button","width":64,"height":64},
            {"type":"frame","id":"b","role":"button","width":48,"height":48}
        ]});
        row.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let draft = state(row.clone());
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&draft);
        let root = scene.active_page().unwrap().find("actions").unwrap();
        assert!(!super::super::control_rows::spend_blank_row_height(
            &mut row, root, 28.0
        ));
    }
}

#[test]
fn internal_control_rows_do_not_supply_external_bottom_room() {
    let controls = |id: &str| {
        json!({
            "type":"frame","id":id,"height":110,"layout":"horizontal","alignItems":"center",
            "children":[
                {"type":"frame","id":format!("{id}-a"),"role":"button","width":64,"height":64},
                {"type":"frame","id":format!("{id}-b"),"role":"button","width":48,"height":48}
            ]
        })
    };
    for boundary in [
        json!({"height":600,"layout":"vertical","clipContent":true}),
        json!({"height":600,"layout":"vertical"}),
        json!({"height":"fill_container","layout":"vertical"}),
        json!({"height":"fit_content","layout":"horizontal"}),
    ] {
        let mut body = json!({"type":"frame","id":"body","children":[controls("internal")]});
        body.as_object_mut()
            .unwrap()
            .extend(boundary.as_object().unwrap().clone());
        let mut root = json!({"type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
            "children":[body.clone(),controls("external")]});
        let draft = state(root.clone());
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&draft);
        let board = scene.active_page().unwrap().find("root").unwrap();
        assert!(super::super::control_rows::spend_blank_row_height(
            &mut root, board, 26.0
        ));
        assert_eq!(
            root["children"][0], body,
            "internal row must remain unchanged"
        );
        assert_eq!(root["children"][1]["height"], 84.0);
    }
}

#[test]
fn asymmetric_padding_keeps_eight_pixels_below_centered_controls() {
    let mut row = json!({"type":"frame","id":"row","width":375,"height":140,"layout":"horizontal",
    "alignItems":"center","padding":[24,0,0,0],"children":[
        {"type":"frame","id":"a","role":"button","width":64,"height":64},
        {"type":"frame","id":"b","role":"button","width":48,"height":48}
    ]});
    let draft = state(row.clone());
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&draft);
    let board = scene.active_page().unwrap().find("row").unwrap();
    assert!(super::super::control_rows::spend_blank_row_height(
        &mut row, board, 100.0
    ));
    assert_eq!(row["height"], 104.0);
    let settled = state(row);
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&settled);
    let board = scene.active_page().unwrap().find("row").unwrap();
    let button = board.find("a").unwrap().bounds;
    assert_eq!(
        board.bounds.origin.y + board.bounds.size.y - button.origin.y - button.size.y,
        8.0
    );
}

#[test]
fn compliant_phone_and_trailing_navigation_keep_their_authored_spacing() {
    let mut comfortable = phone(20.0, 600.0);
    let mut nav = phone(20.0, 648.0);
    nav.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("actions"),
        patch_json: json!({"role":"bottom-tab-bar"}).to_string(),
        page_id: None,
    });
    for mut draft in [comfortable.clone(), nav] {
        let before = draft.doc.clone();
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
        repair(&mut sink, "root");
        assert_eq!(sink.state().doc, before);
    }
    comfortable.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("root"),
        patch_json: json!({"padding":"$--inset"}).to_string(),
        page_id: None,
    });
    let before = comfortable.doc.clone();
    let mut sink = crate::loop_finalize::StateDocSink {
        state: &mut comfortable,
    };
    assert!(!repair_bottom_room(&mut sink, "root"));
    assert_eq!(sink.state().doc, before);
}

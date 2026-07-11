//! Tests for `mcp::batch_design::BatchDesign`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`. Tool-layer parsing / validation + a few end-to-end
//! `EditorState::apply` checks; the apply-path correctness is covered
//! by `op-editor-core`'s `command_tests.rs`.

use super::test_fixtures::sample;
use super::{BatchInsertItem, EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
use crate::batch_design_snapshot;
use op_editor_core::PenNodeExt;
use std::collections::BTreeMap;

#[test]
fn batch_design_requires_nodes_json() {
    let tool = batch_design_snapshot(&sample());
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("nodes_json"));
        }
        _ => panic!(),
    }
}

#[test]
fn batch_design_rejects_empty_array() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("nodes_json".into(), "[]".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn batch_design_parses_minimal_two_node_array() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "nodes_json".into(),
        r##"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":20},{"kind":"ellipse","name":"B","x":40,"y":50,"width":30,"height":30,"fill_hex":"#ff0000"}]"##
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(result, EditorCommand::BatchInsert { items, page_id }) => {
            assert_eq!(result.get("count"), Some(&"2".to_string()));
            assert!(page_id.is_none());
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].kind, "rect");
            assert_eq!(items[0].name, "A");
            assert_eq!(items[0].width, 10);
            assert_eq!(items[0].height, 20);
            assert!(items[0].fill_hex.is_none());
            assert_eq!(items[1].kind, "ellipse");
            assert_eq!(items[1].fill_hex.as_deref(), Some("#ff0000"));
        }
        other => panic!("expected BatchInsert, got {other:?}"),
    }
}

#[test]
fn batch_design_fill_passthrough_carries_mesh_and_radial() {
    use jian_ops_schema::style::PenFill;
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "nodes_json".into(),
        r##"[{"kind":"rect","name":"Mesh","x":0,"y":0,"width":100,"height":100,"fill":[{"type":"mesh_gradient","rows":2,"cols":2,"stops":[{"row":0,"col":0,"color":"#ff0000"},{"row":0,"col":1,"color":"#00ff00"},{"row":1,"col":0,"color":"#0000ff"},{"row":1,"col":1,"color":"#ffff00"}]}]},{"kind":"ellipse","name":"Radial","x":0,"y":0,"width":80,"height":80,"fill":{"type":"radial_gradient","stops":[{"offset":0,"color":"#fff"},{"offset":1,"color":"#000"}]}}]"##
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::BatchInsert { items, .. }) => {
            assert_eq!(items.len(), 2);
            // Array form → full mesh fill stack.
            let mesh = items[0].fill.as_ref().expect("mesh fill stack");
            assert_eq!(mesh.len(), 1);
            match &mesh[0] {
                PenFill::MeshGradient(b) => {
                    assert_eq!(b.rows, 2);
                    assert_eq!(b.cols, 2);
                    assert_eq!(b.stops.len(), 4);
                }
                other => panic!("expected mesh_gradient, got {other:?}"),
            }
            // Single-object form → wrapped into a 1-entry radial stack.
            let radial = items[1].fill.as_ref().expect("radial fill stack");
            assert_eq!(radial.len(), 1);
            assert!(matches!(radial[0], PenFill::RadialGradient(_)));
        }
        other => panic!("expected BatchInsert, got {other:?}"),
    }
}

#[test]
fn batch_design_fill_passthrough_carries_shader() {
    // The generic `fill` passthrough must carry a `{type:"shader",...}`
    // entry straight through to `PenFill::Shader` with no new tool — same
    // path mesh / radial already ride.
    use jian_ops_schema::style::{PenFill, ShaderUniformValue};
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "nodes_json".into(),
        r##"[{"kind":"rect","name":"Shader","x":0,"y":0,"width":240,"height":240,"fill":[{"type":"shader","sksl":"half4 main(float2 p){ return half4(p.x/240.0, p.y/240.0, 0.5, 1.0); }","uniforms":{"glow":0.5,"tint":"#ff00aa"}}]}]"##
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::BatchInsert { items, .. }) => {
            assert_eq!(items.len(), 1);
            let stack = items[0].fill.as_ref().expect("shader fill stack");
            assert_eq!(stack.len(), 1);
            match &stack[0] {
                PenFill::Shader(b) => {
                    assert!(b.sksl.contains("half4 main(float2 p)"));
                    let u = b.uniforms.as_ref().expect("uniforms map");
                    assert_eq!(u.get("glow"), Some(&ShaderUniformValue::Float(0.5)));
                    assert_eq!(
                        u.get("tint"),
                        Some(&ShaderUniformValue::Color("#ff00aa".into()))
                    );
                }
                other => panic!("expected shader, got {other:?}"),
            }
        }
        other => panic!("expected BatchInsert, got {other:?}"),
    }
}

#[test]
fn batch_design_nodes_json_accepts_outer_page_id() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), "page-2".into());
    args.insert(
        "nodes_json".into(),
        r##"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":20}]"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::BatchInsert { items, page_id }) => {
            assert_eq!(items.len(), 1);
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected BatchInsert with page id, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_ts_insert_operations_tree() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Page","width":320,"height":240})
label=I(root, {"type":"text","name":"Greeting","content":"Hello","width":120,"height":24})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(
            json,
            EditorCommand::InsertAuthoredSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            // TS result shape: { results:[{binding,nodeId}], nodeCount }.
            let v: serde_json::Value = serde_json::from_str(&json).expect("batch_design json");
            let results = v["results"].as_array().expect("results array");
            assert_eq!(results.len(), 2);
            let bindings: Vec<&str> = results
                .iter()
                .map(|r| r["binding"].as_str().expect("binding"))
                .collect();
            assert!(
                bindings.contains(&"root") && bindings.contains(&"label"),
                "{v}"
            );
            assert!(
                results.iter().all(|r| r["nodeId"].as_str().is_some()),
                "{v}"
            );
            assert!(v["nodeCount"].as_u64().is_some_and(|n| n >= 2), "{v}");
            // The emitted command is unchanged.
            assert!(!parent_id.is_real());
            assert!(page_id.is_none());
            assert_eq!(nodes.len(), 1);
            let root = &nodes[0];
            assert!(root.is_container());
            assert_eq!(root.children().expect("children").len(), 1);
            assert_eq!(
                root.children().unwrap()[0].base().name.as_deref(),
                Some("Greeting")
            );
        }
        other => panic!("expected OkJsonWithCommand InsertAuthoredSubtree, got {other:?}"),
    }
}

#[test]
fn batch_design_promotes_legacy_role_input_frame_to_text_input() {
    // Phase E3 — an old-style `frame role="input"` (with a two-way value
    // binding + a muted-grey placeholder text child) the AI emits must land a
    // real `text_input` widget node, not a frame. The promotion runs over the
    // parsed forest before it becomes the inserted command.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"field=I(null, {"type":"frame","name":"Email","role":"input","width":240,"height":40,
          "fill":[{"type":"solid","color":"#1E1E1E"}],
          "bindings":{"bind:value":"$state.email"},
          "children":[{"type":"text","name":"ph","content":"Enter email","width":120,"height":20,
                       "fill":[{"type":"solid","color":"#9A9A9A"}]}]})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(
            json,
            EditorCommand::InsertAuthoredSubtree { nodes, .. },
        ) => {
            // The marked frame became a first-class text_input widget node.
            assert_eq!(nodes.len(), 1);
            let jian_ops_schema::node::PenNode::TextInput(ti) = &nodes[0] else {
                panic!(
                    "expected promotion to PenNode::TextInput, got {:?}",
                    nodes[0]
                );
            };
            // Muted-grey text child → placeholder; role marker is dropped.
            assert_eq!(ti.placeholder.as_deref(), Some("Enter email"));
            assert!(ti.base.role.is_none());
            // The two-way value binding carried over from the frame.
            assert!(ti
                .bindings
                .as_ref()
                .is_some_and(|b| b.contains_key("bind:value")));
            // The JSON result reports the promotion (Phase E3 surface).
            let v: serde_json::Value = serde_json::from_str(&json).expect("batch_design json");
            let promoted = v["promoted"].as_array().expect("promoted array");
            assert_eq!(promoted.len(), 1);
            assert_eq!(promoted[0]["to"], "text_input");
            assert_eq!(promoted[0]["fromRole"], "input");
        }
        other => panic!("expected InsertAuthoredSubtree, got {other:?}"),
    }
}

#[test]
fn batch_design_ellipse_preserves_arc_fields() {
    // A native ring/arc/donut: an ellipse authored with startAngle /
    // sweepAngle / innerRadius (camelCase, as the DSL uses) must keep
    // those fields on the constructed EllipseNode instead of dropping
    // them — so an LLM can author a ring in one batch_design call.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"ring=I(null, {"type":"ellipse","name":"Gauge","width":120,"height":120,"innerRadius":0.6,"startAngle":0,"sweepAngle":270})"##
            .into(),
    );

    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertAuthoredSubtree command");
    };
    assert_eq!(nodes.len(), 1);
    let jian_ops_schema::node::PenNode::Ellipse(ell) = &nodes[0] else {
        panic!("expected PenNode::Ellipse, got {:?}", nodes[0]);
    };
    assert_eq!(ell.inner_radius, Some(0.6));
    assert_eq!(ell.start_angle, Some(0.0));
    assert_eq!(ell.sweep_angle, Some(270.0));
}

#[test]
fn batch_design_normalizes_ts_layout_keywords() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Toolbar","layout":"horizontal","alignItems":"flex-start","justifyContent":"space-between","padding":[2],"children":[{"type":"frame","name":"Button","layout":"horizontal","alignItems":"flex-end","justifyContent":"flex-end"}]})"##
            .into(),
    );

    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertSubtree command");
    };

    let value = serde_json::to_value(&nodes[0]).expect("node json");
    assert_eq!(value["alignItems"], "start");
    assert_eq!(value["justifyContent"], "space_between");
    assert_eq!(value["padding"].as_f64(), Some(2.0));
    assert_eq!(value["children"][0]["alignItems"], "end");
    assert_eq!(value["children"][0]["justifyContent"], "end");
}

#[test]
fn batch_design_normalizes_underscore_flex_keywords() {
    // A CSS-fluent model writes the snake_case `flex_start`/`flex_end` (by
    // analogy to our `space_between`). The schema only has `start`/`end`, so an
    // un-normalized flex_* fails the WHOLE node's deserialize and it is silently
    // dropped — measured: glm's right-aligned amount cells + left-aligned header
    // labels vanished, leaving only the `center` ones. Both children below MUST
    // survive with normalized alignment.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Row","layout":"horizontal","children":[{"type":"frame","name":"Left","justifyContent":"flex_start"},{"type":"frame","name":"Amount","justifyContent":"flex_end","alignItems":"flex_start"}]})"##
            .into(),
    );

    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertSubtree command");
    };
    let value = serde_json::to_value(&nodes[0]).expect("node json");
    let children = value["children"].as_array().expect("children survive");
    assert_eq!(children.len(), 2, "BOTH flex_* children must survive");
    assert_eq!(children[0]["justifyContent"], "start");
    assert_eq!(children[1]["justifyContent"], "end");
    assert_eq!(children[1]["alignItems"], "start");
}

#[test]
fn batch_design_defaults_missing_stroke_thickness() {
    // DeepSeek V4 writes `stroke:{"color":…}` with no `thickness` — the
    // schema requires it, one rejected node cascaded into "parent not
    // found" for 60+ descendant lines and the design shipped as one empty
    // section (measured 2026-07-12). The normalize layer must default it.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Card","stroke":{"color":"#333333"},"children":[{"type":"text","content":"inside","width":100,"height":20}]})"##
            .to_string(),
    );
    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertSubtree command");
    };
    let value = serde_json::to_value(&nodes[0]).expect("node json");
    assert_eq!(
        value["stroke"]["thickness"],
        serde_json::json!(1.0),
        "missing thickness defaults to hairline: {:?}",
        value["stroke"]
    );
    assert_eq!(
        value["children"].as_array().map(Vec::len),
        Some(1),
        "child landed with its parent"
    );
}

#[test]
fn batch_design_maps_pencil_autolayout_dialect() {
    // MiniMax-M3 is trained on Pencil's schema: it emits `layoutMode` /
    // `itemSpacing` / `strokeWeight` / `primaryAxisAlignItems` /
    // `counterAxisAlignItems`. serde drops those unknown keys, so the frame
    // loses its layout entirely and 229 nodes render as one horizontal strip
    // (measured on the barbershop loop run). The dialect map MUST rename them
    // onto our schema so the model's layout survives.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Card","layoutMode":"VERTICAL","itemSpacing":16,"strokeWeight":2,"stroke":"#E7E5E4","primaryAxisAlignItems":"SPACE_BETWEEN","counterAxisAlignItems":"CENTER"})"##
            .into(),
    );

    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertSubtree command");
    };
    let value = serde_json::to_value(&nodes[0]).expect("node json");
    assert_eq!(value["layout"], "vertical", "layoutMode → layout");
    assert_eq!(value["gap"].as_f64(), Some(16.0), "itemSpacing → gap");
    assert_eq!(
        value["justifyContent"], "space_between",
        "primaryAxisAlignItems → justifyContent"
    );
    assert_eq!(
        value["alignItems"], "center",
        "counterAxisAlignItems → alignItems"
    );
    // strokeWeight folds into the stroke as its thickness (schema uses per-side
    // thickness; a scalar lands on `.thickness`).
    assert!(
        value["stroke"].is_object() || value["stroke"].is_array(),
        "stroke survives as a structured value, got {:?}",
        value["stroke"]
    );
}

#[test]
fn batch_design_flattens_structured_layout_object() {
    // glm-5.2 in the agentic loop writes `layout` as a Figma/flex OBJECT —
    // `{"type":"horizontal","gap":0,"padding":[…]}` and the externally-tagged
    // `{"Vertical":{"gap":12}}`. serde rejects both against our string-typed
    // `layout` field, so every `U(n1,{layout:{…}})` failed and glm's (correctly
    // id-tracked!) tree never got its layout. Both shapes MUST flatten.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Shell","layout":{"type":"horizontal","gap":24,"padding":[8,8,8,8]},"children":[{"type":"frame","name":"Col","layout":{"Vertical":{"gap":12}}}]})"##
            .into(),
    );
    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertSubtree command");
    };
    let value = serde_json::to_value(&nodes[0]).expect("node json");
    assert_eq!(
        value["layout"], "horizontal",
        "type-keyed object → layout string"
    );
    assert_eq!(value["gap"].as_f64(), Some(24.0), "hoisted gap");
    assert_eq!(
        value["padding"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_f64()),
        Some(8.0),
        "hoisted per-side padding"
    );
    assert_eq!(
        value["children"][0]["layout"], "vertical",
        "externally-tagged {{Vertical:{{…}}}} → layout string"
    );
    assert_eq!(
        value["children"][0]["gap"].as_f64(),
        Some(12.0),
        "variant-keyed inner gap hoisted"
    );
}

#[test]
fn batch_design_maps_direction_alias_to_layout() {
    // glm-5.2 in the loop reaches for the flex/CSS `direction` alias instead of
    // our `layout` (measured: `{…,"direction":"horizontal",…}` on every frame),
    // so serde drops it and 0/62 frames got a layout. Map it.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Row","direction":"horizontal","children":[{"type":"frame","name":"Col","direction":"column"}]})"##
            .into(),
    );
    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertSubtree command");
    };
    let value = serde_json::to_value(&nodes[0]).expect("node json");
    assert_eq!(value["layout"], "horizontal", "direction → layout");
    assert_eq!(
        value["children"][0]["layout"], "vertical",
        "direction:column → vertical"
    );
}

#[test]
fn batch_design_insert_operations_accept_outer_page_id() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), "page-2".into());
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Page","width":320,"height":240})"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(
            _,
            EditorCommand::InsertAuthoredSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            assert_eq!(nodes.len(), 1);
            assert!(!parent_id.is_real());
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected InsertAuthoredSubtree with page id, got {other:?}"),
    }
}

#[test]
fn batch_design_insert_operations_apply_as_one_nested_subtree() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"card=I(null, {"type":"frame","name":"Card","width":200,"height":120})
title=I(card, {"type":"text","name":"Title","content":"Ready","width":100,"height":24})"##
            .into(),
    );
    let cmd = match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(_, cmd) => cmd,
        other => panic!("expected command, got {other:?}"),
    };

    let mut s = sample();
    let before = s.active_children().len();
    assert!(s.apply(cmd));
    assert_eq!(s.active_children().len(), before + 1);
    let inserted = s.active_children().last().expect("inserted root");
    assert_eq!(inserted.base().name.as_deref(), Some("Card"));
    assert_eq!(inserted.children().expect("nested children").len(), 1);
}

#[test]
fn batch_design_results_predict_the_applied_node_ids() {
    // The whole point: the binding->nodeId map the tool REPORTS must equal the
    // ids the host actually ASSIGNS at apply (single-user localhost: the tool
    // predicts off the same doc the apply mutates, running the same allocation).
    let mut state = sample();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"card=I(null, {"type":"frame","name":"Card","width":200,"height":120})
title=I(card, {"type":"text","name":"Title","content":"Ready","width":100,"height":24})"##
            .into(),
    );

    let (json, cmd) = match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(json, cmd) => (json, cmd),
        other => panic!("expected OkJsonWithCommand, got {other:?}"),
    };
    let v: serde_json::Value = serde_json::from_str(&json).expect("json");
    let mut predicted = std::collections::BTreeMap::new();
    for r in v["results"].as_array().expect("results") {
        predicted.insert(
            r["binding"].as_str().unwrap().to_string(),
            r["nodeId"].as_str().unwrap().to_string(),
        );
    }

    // Apply against the SAME doc the snapshot was taken from.
    assert!(state.apply(cmd));
    let root = state.active_children().last().expect("inserted card");
    assert_eq!(root.base().name.as_deref(), Some("Card"));
    assert_eq!(
        root.base().id,
        predicted["card"],
        "predicted card id must equal the applied id"
    );
    let child = &root.children().expect("children")[0];
    assert_eq!(child.base().name.as_deref(), Some("Title"));
    assert_eq!(
        child.base().id,
        predicted["title"],
        "predicted title id must equal the applied id"
    );
}

#[test]
fn batch_design_authored_ids_reject_on_concurrent_collision() {
    // The robustness guarantee: if the doc changes between the tool's snapshot
    // and the apply (e.g. a concurrent edit on the live desktop canvas) so an
    // assigned authored id now collides, InsertAuthoredSubtree REJECTS (the
    // applier demotes to an error) — it NEVER silently lands a different id.
    let mut state = sample();
    let tool = batch_design_snapshot(&state); // snapshot at the current id seed
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"card=I(null, {"type":"frame","name":"Card","width":200,"height":120})"##.into(),
    );
    let cmd = match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(_, cmd) => cmd,
        other => panic!("expected OkJsonWithCommand, got {other:?}"),
    };

    // Simulate a concurrent edit that grabs the very id the tool assigned
    // (the next minted id == the tool's first authored id).
    assert!(state.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "Concurrent".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
    }));

    // The authored id now collides → the command must be rejected, not remapped.
    assert!(
        !state.apply(cmd),
        "an authored-id collision must reject, never silently land a different id"
    );
}

#[test]
fn batch_design_direct_operation_accepts_outer_page_id() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), "page-2".into());
    args.insert("operations".into(), r##"U("n11", {"x":80})"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::UpdateNode {
                node_id, page_id, ..
            },
        ) => {
            assert_eq!(node_id.as_str(), "n11");
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected UpdateNode with page id, got {other:?}"),
    }
}

#[test]
fn batch_design_direct_update_preserves_rich_ts_patch_fields() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), "page-2".into());
    args.insert(
        "operations".into(),
        r##"U("n11", {"content":"Updated","fontSize":24})"##.into(),
    );

    let ToolOutcome::OkWithCommand(
        _,
        EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id,
        },
    ) = tool.call(&args)
    else {
        panic!("expected PatchNodeData command from rich U() patch");
    };
    let patch: serde_json::Value = serde_json::from_str(&patch_json).expect("patch json");
    assert_eq!(node_id.as_str(), "n11");
    assert_eq!(patch["content"], "Updated");
    assert_eq!(patch["fontSize"], 24);
    assert_eq!(page_id.as_deref(), Some("page-2"));
}

#[test]
fn batch_design_accepts_single_update_operation() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"U("n11", {"x":80,"y":90,"width":260,"height":32,"name":"Updated title","fill_hex":"#112233"})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::UpdateNode {
                node_id,
                x,
                y,
                width,
                height,
                name,
                fill_hex,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n11");
            assert_eq!(x, Some(80));
            assert_eq!(y, Some(90));
            assert_eq!(width, Some(260));
            assert_eq!(height, Some(32));
            assert_eq!(name.as_deref(), Some("Updated title"));
            assert_eq!(fill_hex.as_deref(), Some("#112233"));
            assert_eq!(page_id, None);
        }
        other => panic!("expected UpdateNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_delete_operation() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"D("n14")"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(result, EditorCommand::DeleteNode { node_id, page_id }) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n14");
            assert_eq!(page_id, None);
        }
        other => panic!("expected DeleteNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_move_operation_without_index() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"M("n14", null)"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::MoveNode {
                node_id,
                target_parent,
                page_id,
                index,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n14");
            assert!(!target_parent.is_real());
            assert!(page_id.is_none());
            assert!(index.is_none());
        }
        other => panic!("expected MoveNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_copy_operation_with_overrides() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"C("n12", "n10", {"name":"Copied","x":24,"id":"ignored"})"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::CopyNode {
                node_id,
                target_parent,
                overrides_json,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n12");
            assert_eq!(target_parent.as_str(), "n10");
            assert!(page_id.is_none());

            let overrides: serde_json::Value =
                serde_json::from_str(overrides_json.as_deref().expect("overrides")).unwrap();
            assert_eq!(
                overrides.get("name").and_then(|v| v.as_str()),
                Some("Copied")
            );
            assert_eq!(overrides.get("x").and_then(|v| v.as_i64()), Some(24));
            assert_eq!(
                overrides.get("id").and_then(|v| v.as_str()),
                Some("ignored")
            );
        }
        other => panic!("expected CopyNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_bound_single_copy_operation() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"copied=C("n12", null)"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::CopyNode {
                node_id,
                target_parent,
                overrides_json,
                ..
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n12");
            assert!(!target_parent.is_real());
            assert!(overrides_json.is_none());
        }
        other => panic!("expected bound CopyNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_replace_operation() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"R("n12", {"type":"rectangle","name":"Replacement","x":5,"y":6,"width":70,"height":80,"fill":"#abcdef"})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::ReplaceNode {
                node_id,
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
                drop_children,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n12");
            assert_eq!(kind, "rect");
            assert_eq!(name, "Replacement");
            assert_eq!(x, 5);
            assert_eq!(y, 6);
            assert_eq!(width, 70);
            assert_eq!(height, 80);
            assert_eq!(fill_hex.as_deref(), Some("#abcdef"));
            assert!(!drop_children);
            assert!(page_id.is_none());
        }
        other => panic!("expected ReplaceNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_bound_single_replace_operation() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"replacement=R("n12", {"type":"text","content":"Renamed","width":120,"height":24})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::ReplaceNode {
                node_id,
                kind,
                name,
                width,
                height,
                ..
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n12");
            assert_eq!(kind, "text");
            assert_eq!(name, "Renamed");
            assert_eq!(width, 120);
            assert_eq!(height, 24);
        }
        other => panic!("expected bound ReplaceNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_image_operation_without_fetcher() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"G("n10", "search", "hero product photo")"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InsertSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(parent_id.as_str(), "n10");
            assert!(page_id.is_none());
            assert_eq!(nodes.len(), 1);
            assert!(matches!(nodes[0], jian_ops_schema::node::PenNode::Image(_)));
            assert_eq!(nodes[0].base().name.as_deref(), Some("hero product photo"));
        }
        other => panic!("expected image InsertSubtree command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_bound_single_image_operation_without_fetcher() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"hero=G(null, "generate", "dashboard background")"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InsertSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert!(!parent_id.is_real());
            assert!(page_id.is_none());
            assert_eq!(nodes.len(), 1);
            assert!(matches!(nodes[0], jian_ops_schema::node::PenNode::Image(_)));
            assert_eq!(
                nodes[0].base().name.as_deref(),
                Some("dashboard background")
            );
        }
        other => panic!("expected bound image InsertSubtree command, got {other:?}"),
    }
}

#[test]
fn batch_design_rejects_unknown_kind_in_any_item() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "nodes_json".into(),
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":10},{"kind":"blob","name":"B","x":0,"y":0,"width":10,"height":10}]"#
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!("a single bad entry must reject the whole batch"),
    }
}

#[test]
fn batch_design_rejects_negative_geometry() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "nodes_json".into(),
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":-1,"height":10}]"#.into(),
    );
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn batch_design_rejects_malformed_json() {
    let tool = batch_design_snapshot(&sample());
    for bad in [
        "not json",
        "{}",
        "[{}]",
        r#"[{"kind":"rect"}]"#,
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":10"#,
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":10},]"#,
    ] {
        let mut args = BTreeMap::new();
        args.insert("nodes_json".into(), bad.into());
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument, "{bad}")
            }
            _ => panic!("expected reject on {bad}"),
        }
    }
}

#[test]
fn batch_design_accepts_single_move_operation_with_index() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"M("n14", "n10", 2)"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::MoveNode {
                target_parent,
                index,
                ..
            },
        ) => {
            assert_eq!(target_parent.as_str(), "n10");
            assert_eq!(index, Some(2));
        }
        other => panic!("expected indexed MoveNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_bound_single_move_operation() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"moved=M("n14", "n10", 1)"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::MoveNode {
                node_id,
                target_parent,
                index,
                ..
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n14");
            assert_eq!(target_parent.as_str(), "n10");
            assert_eq!(index, Some(1));
        }
        other => panic!("expected bound MoveNode command, got {other:?}"),
    }
}

#[test]
fn batch_insert_command_adds_all_nodes() {
    let mut s = sample();
    let pre_root_len = s.active_children().len();
    assert!(s.apply(EditorCommand::BatchInsert {
        items: vec![
            BatchInsertItem {
                kind: "rect".into(),
                name: "A".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 20,
                fill_hex: None,
                fill: None,
            },
            BatchInsertItem {
                kind: "ellipse".into(),
                name: "B".into(),
                x: 40,
                y: 50,
                width: 30,
                height: 30,
                fill_hex: Some("#00ff00".into()),
                fill: None,
            },
        ],
        page_id: None,
    }));
    assert_eq!(s.active_children().len(), pre_root_len + 2);
}

#[test]
fn batch_insert_command_atomic_on_bad_descriptor() {
    let mut s = sample();
    let pre_root_len = s.active_children().len();
    assert!(!s.apply(EditorCommand::BatchInsert {
        items: vec![
            BatchInsertItem {
                kind: "rect".into(),
                name: "A".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
                fill: None,
            },
            BatchInsertItem {
                kind: "blob".into(),
                name: "B".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
                fill: None,
            },
        ],
        page_id: None,
    }));
    assert_eq!(
        s.active_children().len(),
        pre_root_len,
        "no partial insertion"
    );
}

#[test]
fn batch_insert_command_rejects_empty_items() {
    let mut s = sample();
    assert!(!s.apply(EditorCommand::BatchInsert {
        items: vec![],
        page_id: None,
    }));
}

#[test]
fn batch_design_drops_ambiguous_auto_sizing_and_recovers_text_growth_words() {
    // `width:"auto"` is ambiguous in CSS (fill for a block's width, hug for its
    // height) — forcing either direction inverts intent half the time, so the
    // key must be DROPPED (schema default wins) while the node itself survives.
    // A misspelled `textGrowth` carrying clear words ("fixed_width_and_height")
    // must recover its meaning instead of silently reverting to the default.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Block","width":"auto","height":"fit_content","children":[{"type":"text","name":"T","content":"hello","textGrowth":"fixed_width_and_height"}]})"##
            .into(),
    );
    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertSubtree command");
    };
    let value = serde_json::to_value(&nodes[0]).expect("node json");
    assert!(
        value.get("width").map(|w| !w.is_string()).unwrap_or(true),
        "ambiguous auto width dropped, got {:?}",
        value.get("width")
    );
    assert_eq!(value["height"], "fit_content", "valid keyword untouched");
    let text = &value["children"][0];
    assert_eq!(
        text["textGrowth"], "fixed-width-height",
        "word-based textGrowth spelling recovered"
    );
}

#[test]
fn batch_design_falls_back_to_root_for_phantom_parent_binding() {
    // A weak model copies the `sec` example binding as its FIRST line's
    // parent. The phantom parent used to ride into `InsertAuthoredSubtree`
    // unvalidated and the host rejected the WHOLE otherwise-valid program
    // (an orchestrator stats subtask retried its complete 4-card section
    // away). The tool must fall back to a root insert and surface a warning.
    let state = op_editor_core::EditorState::new();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        "row=I(sec, {\"type\":\"frame\",\"name\":\"Stat Row\",\"layout\":\"horizontal\",\"gap\":24})\nc1=I(row, {\"type\":\"frame\",\"name\":\"Card\",\"layout\":\"vertical\"})".to_string(),
    );
    let ToolOutcome::OkJsonWithCommand(json, cmd) = tool.call(&args) else {
        panic!("expected a command outcome");
    };
    assert!(json.contains("warnings"), "phantom parent surfaced: {json}");
    let mut s2 = op_editor_core::EditorState::new();
    assert!(s2.apply(cmd), "root-fallback insert must apply cleanly");
    assert_eq!(s2.active_children().len(), 1, "one root landed");
    use op_editor_core::PenNodeExt;
    let root = &s2.active_children()[0];
    assert_eq!(
        root.children().map(|c| c.len()),
        Some(1),
        "card nested in row"
    );
}

#[test]
fn batch_design_operations_hoists_node_state() {
    // An I()-program insert whose root frame declares node-level
    // `state` must yield TWO sibling commands — MergeAppState(unplanned)
    // then the insert — batched by the program finisher's existing
    // 0/1/many wrap, with the node's `state` stripped.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Card","width":320,"height":240,"state":{"count":{"type":"int","default":1}}})"##
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(_, EditorCommand::Batch { commands }) => {
            assert_eq!(commands.len(), 2);
            match &commands[0] {
                EditorCommand::MergeAppState { plan_idx, state } => {
                    assert_eq!(*plan_idx, usize::MAX);
                    assert!(state.contains_key("count"));
                }
                other => panic!("expected MergeAppState first, got {other:?}"),
            }
            match &commands[1] {
                EditorCommand::InsertAuthoredSubtree { nodes, .. } => {
                    let v = serde_json::to_value(&nodes[0]).expect("json");
                    assert!(v.get("state").is_none(), "node state must be stripped");
                }
                other => panic!("expected InsertAuthoredSubtree second, got {other:?}"),
            }
        }
        other => panic!("expected Batch command, got {other:?}"),
    }
}

#[test]
fn batch_design_without_node_state_keeps_plain_command() {
    // No node-level state → the command shape is unchanged (no Batch).
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Plain","width":320,"height":240})"##.into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { .. }) => {}
        other => panic!("expected plain InsertAuthoredSubtree, got {other:?}"),
    }
}

#[test]
fn batch_design_noop_state_merge_still_lands_the_insert() {
    // Regression for the codex BLOCKER: regenerating a section into a
    // document whose root state ALREADY carries the declared key is a
    // completely normal flow (the merge is a legitimate additive
    // no-op), not a failure. Before the fix, `merge_app_state` returned
    // `false` for the fully-skipped-keys case, so the sim-validated
    // `ctx.emit` in `batch_program.rs` treated the merge as a failed
    // line — misreporting an `errors[]` entry for a line whose insert
    // had already landed — and the SAME `merge_app_state` bug would
    // sink the whole `Batch` at HOST apply time on the five other
    // `with_hoisted_state` producers (insert_node / replace_node /
    // design_content / design_skeleton / batch_design), since none of
    // them sim-validate before batching.
    use jian_ops_schema::state::{PrimitiveType, StateEntry, StateType};
    let mut state = sample();
    let mut existing: BTreeMap<String, StateEntry> = BTreeMap::new();
    existing.insert(
        "count".into(),
        StateEntry {
            kind: StateType::Primitive(PrimitiveType::Int),
            default: Some(serde_json::json!(1)),
            description: None,
            persist: None,
        },
    );
    state.doc.state = Some(existing);

    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Card","width":320,"height":240,"state":{"count":{"type":"int","default":1}}})"##
            .into(),
    );
    let (json, cmd) = match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(json, cmd) => (json, cmd),
        other => panic!("expected OkJsonWithCommand, got {other:?}"),
    };
    // The line must not be misreported as errored — the merge is a
    // designed no-op, not a failure.
    assert!(
        !json.contains("\"errors\""),
        "a no-op state merge must not surface as a line error: {json}"
    );

    let before = state.active_children().len();
    assert!(
        state.apply(cmd),
        "the outcome command must apply cleanly despite the pre-existing state key"
    );
    assert_eq!(
        state.active_children().len(),
        before + 1,
        "the insert must land even though its declared state key was a no-op"
    );
    assert_eq!(
        state
            .doc
            .state
            .as_ref()
            .unwrap()
            .get("count")
            .unwrap()
            .default,
        Some(serde_json::json!(1)),
        "the doc-owned state entry is untouched"
    );
}

#[test]
fn batch_design_promotes_radio_group_role() {
    // Task D2: jian's promote table grew a `radio-group` role (D1) — a
    // legacy frame marked `role:"radio-group"` must collapse into a real
    // `radio_group` node through the same operations/I() path, with each
    // visible text child becoming an option.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"rg=I(null, {"type":"frame","name":"Plan","role":"radio-group","width":200,"height":80,"children":[{"type":"text","content":"Monthly"},{"type":"text","content":"Yearly"}]})"##
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) => {
            let v = serde_json::to_value(&nodes[0]).expect("json");
            assert_eq!(v["type"], "radio_group", "role frame must promote, got {v}");
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
fn insert_node_data_hoists_node_state() {
    use crate::write_tools::insert_node_snapshot;
    let tool = insert_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "data".into(),
        r##"{"type":"frame","name":"Widgetful","width":200,"height":100,"state":{"on":{"type":"bool","default":false}},"children":[{"type":"text","content":"hi"}]}"##
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::Batch { commands }) => {
            assert!(
                matches!(&commands[0], EditorCommand::MergeAppState { plan_idx, state }
                if *plan_idx == usize::MAX && state.contains_key("on"))
            );
            assert!(matches!(&commands[1], EditorCommand::InsertSubtree { .. }));
        }
        other => panic!("expected Batch command, got {other:?}"),
    }
}

#[test]
fn design_content_hoists_node_state() {
    use crate::batch_layered::dispatch_design_content;
    let mut args = BTreeMap::new();
    args.insert("sectionId".into(), "sec1".into());
    args.insert(
        "children".into(),
        r##"[{"type":"frame","name":"Counter","width":200,"height":100,"state":{"n":{"type":"int","default":0}}}]"##
            .into(),
    );
    match dispatch_design_content(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::Batch { commands }) => {
            assert!(
                matches!(&commands[0], EditorCommand::MergeAppState { plan_idx, state }
                if *plan_idx == usize::MAX && state.contains_key("n"))
            );
            assert!(matches!(&commands[1], EditorCommand::InsertSubtree { .. }));
        }
        other => panic!("expected Batch command, got {other:?}"),
    }
}

#[cfg(feature = "script")]
#[test]
fn batch_design_script_input_builds_nodes() {
    let state = op_editor_core::EditorState::new();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "script".to_string(),
        r#"const root = I(null, {type: "frame", name: "S"});
for (let i = 0; i < 3; i++) { I(root, {type: "text", content: "t" + i}); }"#
            .to_string(),
    );
    match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(json, _cmd) => {
            assert!(json.contains("\"nodeCount\""), "envelope: {json}");
        }
        other => panic!("expected OkJsonWithCommand, got {other:?}"),
    }
}

#[cfg(feature = "script")]
#[test]
fn batch_design_rejects_script_plus_operations() {
    let state = op_editor_core::EditorState::new();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "script".to_string(),
        "I(null, {type: \"frame\"});".to_string(),
    );
    args.insert(
        "operations".to_string(),
        "r=I(null, {\"type\":\"frame\"})".to_string(),
    );
    match tool.call(&args) {
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg) => {
            assert!(msg.contains("only one of"), "msg: {msg}");
        }
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[cfg(not(feature = "script"))]
#[test]
fn batch_design_script_unavailable_without_feature() {
    let state = op_editor_core::EditorState::new();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("script".to_string(), "I(null, {});".to_string());
    match tool.call(&args) {
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg) => {
            assert!(msg.contains("script-enabled"), "msg: {msg}");
        }
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

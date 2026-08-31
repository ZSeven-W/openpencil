//! Tests for `mcp::batch_design::BatchDesign`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`. Tool-layer parsing / validation + a few end-to-end
//! `EditorState::apply` checks; the apply-path correctness is covered
//! by `op-editor-core`'s `command_tests.rs`.

use super::test_fixtures::sample;
use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
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
                    assert!(b
                        .sksl
                        .as_deref()
                        .is_some_and(|sksl| sksl.contains("half4 main(float2 p)")));
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
fn batch_design_normalizes_hyphenated_fill_type_to_snake_case() {
    // GLM-5.3 occasionally emits CSS-style hyphenated type names instead of
    // snake_case (measured: `"linear-gradient"` dropped, leaving no fill).
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"rectangle","name":"Gradient","width":100,"height":100,"fill":[{"type":"linear-gradient","angle":45,"stops":[{"offset":0,"color":"#ff0000"},{"offset":1,"color":"#0000ff"}]}]})"##
            .into(),
    );

    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertAuthoredSubtree");
    };
    let value = serde_json::to_value(&nodes[0]).expect("node json");
    assert_eq!(value["fill"][0]["type"], "linear_gradient");
}

#[test]
fn batch_design_normalizes_camelcase_fill_type_to_snake_case() {
    // Also normalize camelCase variants.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"rectangle","name":"Radial","width":80,"height":80,"fill":[{"type":"radialGradient","stops":[{"offset":0,"color":"#fff"},{"offset":1,"color":"#000"}]}]})"##
            .into(),
    );

    let ToolOutcome::OkJsonWithCommand(_, EditorCommand::InsertAuthoredSubtree { nodes, .. }) =
        tool.call(&args)
    else {
        panic!("expected InsertAuthoredSubtree");
    };
    let value = serde_json::to_value(&nodes[0]).expect("node json");
    assert_eq!(value["fill"][0]["type"], "radial_gradient");
}

#[test]
fn batch_design_rejects_unknown_fill_type() {
    // Unrecognized type names are left untouched and rejected by serde.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"rectangle","name":"Unknown","width":100,"height":100,"fill":[{"type":"conic-gradient","stops":[]}]})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkJson(msg) if msg.contains("applied\":false") => {
            // Transaction rolled back due to invalid fill type.
            assert!(msg.contains("conic-gradient") || msg.contains("unknown variant"));
        }
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::InvalidArgument);
            assert!(msg.contains("conic-gradient") || msg.contains("expected one of"));
        }
        other => panic!("expected rolled-back transaction or error, got {other:?}"),
    }
}

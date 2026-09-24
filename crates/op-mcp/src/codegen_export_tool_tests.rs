use std::collections::BTreeMap;

use op_editor_core::{EditorState, NodeId};
use serde_json::Value;

use super::codegen_export_tool::{codegen_export_snapshot, codegen_export_targets};
use super::{McpTool, ToolErrorCode, ToolOutcome};

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        r##"{"version":"1.0.0",
            "themes":{"Mode":["Light","Dark"]},
            "variables":{"--card":{"type":"color","value":[
                {"value":"#ffffff","theme":{"Mode":"Light"}},
                {"value":"#18181b","theme":{"Mode":"Dark"}}]}},
            "pages":[
              {"id":"p1","name":"Page","children":[
                {"type":"frame","id":"card","name":"Promo card","width":320,"layout":"vertical","gap":8,
                 "fill":[{"type":"solid","color":"$--card"}],
                 "children":[{"type":"ref","id":"cta","ref":"btn","descendants":{"btn-label":{"content":"Buy"}}}]},
                {"type":"frame","id":"other","name":"Other","width":100,"height":40}
              ]},
              {"id":"p2","name":"Components","children":[
                {"type":"frame","id":"btn","name":"Secondary Button","reusable":true,
                 "children":[{"type":"text","id":"btn-label","content":"Button"}]}
              ]}
            ]}"##,
    )
    .expect("fixture")
    .value;
    EditorState::from_document(doc)
}

fn call(state: &EditorState, pairs: &[(&str, &str)]) -> ToolOutcome {
    let args: BTreeMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    codegen_export_snapshot(state).call(&args)
}

fn json(outcome: ToolOutcome) -> Value {
    match outcome {
        ToolOutcome::OkJson(text) => serde_json::from_str(&text).expect("json result"),
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn defaults_to_react_tailwind_with_theme_files_and_cross_page_masters() {
    let out = json(call(&state(), &[("nodeIds", "card")]));
    assert_eq!(out["framework"], "react-tailwind");
    assert_eq!(out["nodeCount"], 1);
    let component = out["files"]["component.tsx"].as_str().expect("component");
    assert!(component.contains("export default function PromoCard()"));
    assert!(component.contains("bg-card"), "{component}");
    // The master lives on another page and was not selected.
    assert!(
        component.contains("<Button variant=\"secondary\">Buy</Button>"),
        "{component}"
    );
    let css = out["files"]["app/globals.css"].as_str().expect("css");
    let dark = &css[css.find(".dark {").expect("dark block")..];
    assert!(
        dark[..dark.find('}').unwrap()].contains("  --card: #18181b;"),
        "{css}"
    );
    assert!(out["files"]["tailwind.config.ts"].is_string());
    assert!(out["code"].as_str().unwrap().starts_with(component));
}

#[test]
fn output_is_byte_stable_and_other_targets_are_selectable() {
    let first = json(call(&state(), &[("framework", "react-tailwind")]));
    let second = json(call(&state(), &[("framework", "react-tailwind")]));
    assert_eq!(first, second);
    let vue = json(call(&state(), &[("framework", "vue"), ("pageId", "p1")]));
    assert_eq!(vue["nodeCount"], 2);
    assert!(vue["files"]["component.vue"]
        .as_str()
        .unwrap()
        .contains("<template>"));
    let css = json(call(&state(), &[("framework", "css-variables")]));
    assert!(css["files"]["component.css"]
        .as_str()
        .unwrap()
        .contains("--card"));
    assert!(codegen_export_targets().contains(&"react-tailwind"));
    assert!(codegen_export_targets().contains(&"css-variables"));
}

#[test]
fn selection_scopes_the_export_when_no_ids_are_given() {
    let mut state = state();
    state.set_single_selection(NodeId::new("other"));
    let out = json(call(&state, &[]));
    assert_eq!(out["nodeCount"], 1);
    assert!(out["code"].as_str().unwrap().contains("function Other()"));
}

#[test]
fn bad_arguments_return_typed_errors() {
    match call(&state(), &[("framework", "angular")]) {
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, message) => {
            assert!(message.contains("react-tailwind"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    match call(&state(), &[("nodeIds", "missing")]) {
        ToolOutcome::Err(ToolErrorCode::ToolFailed, message) => {
            assert!(message.contains("node not found: missing"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

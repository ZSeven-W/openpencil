//! Brand kit → generation binding, end to end.
//!
//! An extracted kit is applied to a document; then the two generation-side
//! paths that decide what nodes reference are run exactly as a design turn
//! runs them:
//! - the palette seed (`variables::seed_commands`) must NOT overwrite the
//!   kit (existing variables win), and
//! - `variable_binding` must bind a generated literal in the brand colour
//!   to `$--primary`, so the node follows the variable afterwards.
//!
//! Swapping the brand is then applying another kit: the node's `$--primary`
//! reference stays, the value it resolves to changes.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::variable::{VariableScalar, VariableValue};
use op_editor_core::{BrandKitPayload, EditorCommand, EditorState};
use serde_json::json;

use crate::test_support::VecDocSink;
use crate::types::DocSink;

fn payload(kit: &op_brand::BrandKit) -> BrandKitPayload {
    BrandKitPayload {
        label: kit.name.clone(),
        swatches: kit.swatches(),
        variables: kit.variables(),
        themes: kit.themes(),
        design_md: Some(kit.design_md()),
    }
}

fn kit_from_css(css: &str, name: &str) -> op_brand::BrandKit {
    let html =
        format!("<html><head><style>{css}</style></head><body><a class=cta>Go</a></body></html>");
    op_brand::extract_from_html(&html, &[], name)
}

fn light_value(state: &EditorState, name: &str) -> String {
    let VariableValue::Themed(values) = &state.doc.variables.as_ref().unwrap()[name].value else {
        panic!("{name} themed");
    };
    let VariableScalar::Str(s) = &values[0].value else {
        panic!("colour string");
    };
    s.clone()
}

fn plan() -> crate::plan::OrchestratorPlan {
    crate::plan::build_fallback_plan(&crate::types::DesignRequest {
        prompt: "a landing page".into(),
        ..Default::default()
    })
}

#[test]
fn an_applied_kit_survives_the_seed_and_generated_nodes_bind_to_it() {
    let kit = kit_from_css(
        "body{background:#FFFDF8;color:#1C1917} .cta{background:#0E7C66;color:#fff;border-radius:10px}",
        "verdant",
    );
    let mut sink = VecDocSink::new();
    assert!(sink.state.apply_brand_kit(&payload(&kit)));
    assert_eq!(light_value(&sink.state, "--primary"), "#0E7C66");

    // The seed only adds what is missing — the brand stays.
    let plan = plan();
    let snap = crate::variables::snapshot_plan_vars(&sink, &plan);
    assert!(!snap.created.contains(&"--primary".to_string()));
    for cmd in crate::variables::seed_commands(&plan, &snap) {
        sink.apply(cmd);
    }
    assert_eq!(light_value(&sink.state, "--primary"), "#0E7C66");
    assert_eq!(light_value(&sink.state, "--background"), "#FFFDF8");

    // A generated CTA written with the brand literal binds to the token.
    let mut nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"cta","layout":"horizontal",
        "fill":[{"type":"solid","color":"#0E7C66"}],
        "children":[]
    }))
    .unwrap()];
    crate::variable_binding::bind_generated_color_variables(&mut nodes, &sink.state);
    let bound = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(bound["fill"][0]["color"], "$--primary");

    // Swap the brand: same reference, new value.
    let next = kit_from_css(
        "body{background:#FFFFFF;color:#111827} .cta{background:#C2410C;color:#fff}",
        "harbor",
    );
    assert!(sink.state.apply_brand_kit(&payload(&next)));
    assert_eq!(light_value(&sink.state, "--primary"), "#C2410C");
    assert_eq!(bound["fill"][0]["color"], "$--primary");
}

#[test]
fn the_kit_design_md_reaches_the_planner_request() {
    // The host builds `DesignRequest.design_md` from `state.doc.design_md`,
    // so the kit's brief rides along with every later turn.
    let kit = kit_from_css(
        "body{background:#fff;color:#111} .cta{background:#7C3AED}",
        "violet",
    );
    let mut state = EditorState::new();
    assert!(state.apply_brand_kit(&payload(&kit)));
    let md = state.doc.design_md.as_ref().expect("kit design.md written");
    assert!(md.raw.contains("$--primary"));
    assert!(md
        .color_palette
        .as_ref()
        .unwrap()
        .iter()
        .any(|c| c.hex == "#7C3AED"));
    // And the apply is one batch command a host applier can run verbatim.
    let EditorCommand::Batch { commands } = payload(&kit).to_command(None) else {
        panic!("batch");
    };
    assert_eq!(commands.len(), 2);
}

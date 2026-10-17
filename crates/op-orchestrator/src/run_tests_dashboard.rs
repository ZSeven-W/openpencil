//! Dashboard-shell end-to-end tests and the planning retry-before-fallback
//! case.

use super::*;

const DASHBOARD_PLAN_JSON: &str = r##"{
  "sharedContext": {}, "rootFrame": { "id": "root", "name": "Barbershop Dashboard", "width": 1200, "height": 0,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#0A0A0A" }] },
  "subtasks": [
    { "id": "sidebar", "label": "Sidebar Navigation", "region": { "width": 260, "height": 900 } },
    { "id": "kpi", "label": "KPI Stat Cards", "region": { "width": 940, "height": 200 } },
    { "id": "clients", "label": "Client Table", "region": { "width": 940, "height": 500 } }
  ]
}"##;

/// GOLDEN end-to-end (no LLM): dashboard plan → two-column scaffold →
/// scripted subtasks → finalize. The scaffold's Sidebar shell is authored
/// `height: fill_container` and MUST still be fill_container when the run
/// finishes — the user-visible symptom of losing it is the sidebar footer
/// floating mid-page (reported three times).
#[test]
fn run_dashboard_shell_keeps_sidebar_fill_height_end_to_end() {
    let sidebar_nodes = r#"[{"type":"frame","id":"sb-1","name":"Sidebar Navigation","layout":"vertical","width":"fill_container","height":"fill_container","justifyContent":"space_between","children":[{"type":"frame","id":"sb-top","name":"Top","layout":"vertical","children":[{"type":"text","id":"sb-logo","content":"MAISON","fontSize":20}]},{"type":"frame","id":"sb-bottom","name":"Owner Card","layout":"vertical","children":[{"type":"text","id":"sb-owner","content":"Marcus Reed","fontSize":14}]}]}]"#;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(DASHBOARD_PLAN_JSON.into()),
        ScriptResponse::Text(sidebar_nodes.into()),
        ScriptResponse::Text(node_json("kpi")),
        ScriptResponse::Text(node_json("clients")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let mut request = req();
    request.prompt = "barbershop client-management dashboard with a left sidebar".into();
    request.validation_enabled = false;

    futures::executor::block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("dashboard run ok");

    let root = sink.state.active_children().first().expect("root");
    let v = serde_json::to_value(root).unwrap();
    let kids = v["children"].as_array().expect("root children");
    let sidebar = kids
        .iter()
        .find(|k| {
            k["name"]
                .as_str()
                .map(|n| n.to_lowercase().contains("sidebar"))
                .unwrap_or(false)
        })
        .unwrap_or_else(|| {
            panic!(
                "sidebar shell present, got children: {:?}",
                kids.iter().map(|k| k["name"].clone()).collect::<Vec<_>>()
            )
        });
    assert_eq!(
        sidebar["height"],
        serde_json::json!("fill_container"),
        "sidebar shell keeps fill height; sidebar = {}",
        serde_json::to_string_pretty(&sidebar)
            .unwrap()
            .chars()
            .take(600)
            .collect::<String>()
    );
}

#[test]
fn planning_retries_once_before_the_fallback_plan() {
    // A truncated planning response fails the parse; the SECOND attempt
    // returns a valid plan and must be used (no skeleton fallback).
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(r##"{"palette":{"background":"#0B0C0E","surface":"#1A1B"##.into()),
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("run ok after planning retry");

    // The REAL plan (2 subtasks) landed — not the single-subtask fallback.
    assert_eq!(summary.subtasks.len(), 2, "retried plan used, not fallback");
}

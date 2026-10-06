use super::*;
use crate::test_support::VecDocSink;
use op_design_lint::node_util::Variables;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::json;

fn palette() -> Variables {
    [
        ("--foreground", "#0F172A"),
        ("--muted-foreground", "#64748B"),
        ("--card", "#FFFFFF"),
        ("--background", "#F8FAFC"),
    ]
    .into_iter()
    .map(|(name, color)| {
        (
            name.to_string(),
            serde_json::from_value(json!({
                "type": "color",
                "value": [{"value": color, "theme": {"Mode": "Light"}}],
            }))
            .expect("variable"),
        )
    })
    .collect()
}

fn light_theme() -> op_design_lint::node_util::Theme {
    let themes = [("Mode".to_string(), vec!["Light".to_string()])]
        .into_iter()
        .collect();
    op_design_lint::node_util::default_theme(Some(&themes))
}

fn gradient_card(text_x: f64, text_y: f64, text_fill: &str) -> jian_ops_schema::node::PenNode {
    serde_json::from_value(json!({
        "type": "frame",
        "id": "card",
        "width": 320,
        "height": 200,
        "layout": "none",
        "fill": [{
            "type": "linear_gradient",
            "angle": 135,
            "stops": [
                {"offset": 0.0, "color": "#0B1F2A"},
                {"offset": 1.0, "color": "#14B8A6"}
            ]
        }],
        "children": [{
            "type": "text",
            "id": "hero-number",
            "x": text_x,
            "y": text_y,
            "width": 120,
            "height": 40,
            "content": "1,286,430.52",
            "fontSize": 34,
            "fill": [{"type": "solid", "color": text_fill}]
        }]
    }))
    .expect("gradient card")
}

fn repair_gradient_card(text_x: f64, text_y: f64, text_fill: &str) -> (usize, String) {
    let mut sink = VecDocSink::new();
    sink.state.doc.variables = Some(palette());
    sink.state.doc.themes = Some(
        [("Mode".to_string(), vec!["Light".to_string()])]
            .into_iter()
            .collect(),
    );
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![gradient_card(text_x, text_y, text_fill)],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let repaired = repair_text_contrast(&mut sink, "card");
    let fill = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize")
        ["children"][0]["fill"][0]["color"]
        .as_str()
        .expect("hero fill")
        .to_string();
    (repaired, fill)
}

#[test]
fn app_19_gradient_text_at_navy_end_is_repaired_to_a_light_token() {
    let (repaired, fill) = repair_gradient_card(0.0, 0.0, "$--foreground");
    assert_eq!(repaired, 1);
    assert_eq!(fill, "$--card");
}

#[test]
fn dark_text_at_the_teal_end_is_not_an_offender() {
    let (repaired, fill) = repair_gradient_card(240.0, 140.0, "$--foreground");
    assert_eq!(repaired, 0);
    assert_eq!(fill, "$--foreground");
}

#[test]
fn white_text_at_the_navy_end_is_not_an_offender() {
    let (repaired, fill) = repair_gradient_card(0.0, 0.0, "#FFFFFF");
    assert_eq!(repaired, 0);
    assert_eq!(fill, "#FFFFFF");
}

#[test]
fn missing_resolved_rects_use_the_best_stop_fallback() {
    let source = GradientSource::linear(
        135.0,
        vec![(0.0, "#000000".to_string()), (1.0, "#FFFFFF".to_string())],
    );
    let gradient =
        gradient::resolve_gradient(source, &palette(), &light_theme()).expect("gradient");
    let background = LocatedBackground {
        colors: gradient.colors(),
        gradient: Some(gradient),
        source_node_id: Some("owner".to_string()),
        source_index: Some(0),
    };

    // With no owner/text rects, the old best-stop rule sees the readable black
    // stop and does not flag the text, even though a positional sample could.
    assert!(
        below_contrast_threshold("text", "#777777", background, TARGET_RATIO, &HashMap::new(),)
            .is_none()
    );
}

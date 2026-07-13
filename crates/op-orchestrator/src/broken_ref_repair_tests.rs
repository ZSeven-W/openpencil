use super::repair_broken_variable_refs;
use crate::test_support::VecDocSink;
use op_editor_core::{EditorState, NodeId};

/// A document with the shadcn-shaped table the design systems install.
fn sink_from(json: &str) -> VecDocSink {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(json).expect("parse");
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(doc);
    sink
}

fn fill_color(state: &EditorState, id: &str) -> Option<String> {
    let node =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id.to_string()))?;
    let value = serde_json::to_value(node).ok()?;
    value
        .get("fill")?
        .as_array()?
        .first()?
        .get("color")?
        .as_str()
        .map(str::to_string)
}

/// The measured filter button: a soft tint of the brand orange whose glyph was
/// painted `$--white` — a token in no table. It rendered as bare white on a
/// near-white tint and the button read as empty.
#[test]
fn a_glyph_on_a_soft_tint_takes_the_tints_own_colour() {
    let mut sink = sink_from(
        r##"{ "version": "1.0",
          "variables": { "--primary": { "type": "color", "value": "#EA580C" },
                         "--foreground": { "type": "color", "value": "#111111" } },
          "children": [{
            "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "filter", "width": 32, "height": 32,
                "layout": "horizontal",
                "fill": [{ "type": "solid", "color": "#EA580C15" }],
                "children": [
                    { "type": "icon_font", "id": "glyph", "width": 16, "height": 16,
                      "iconFontName": "sliders-horizontal",
                      "fill": [{ "type": "solid", "color": "$--white" }] }
                ]
            }]
          }] }"##,
    );
    repair_broken_variable_refs(&mut sink);
    assert_eq!(
        fill_color(&sink.state, "glyph").as_deref(),
        Some("#EA580C"),
        "the glyph takes the tint's own colour at full strength"
    );
}

#[test]
fn a_glyph_on_a_token_surface_takes_that_tokens_foreground_partner() {
    let mut sink = sink_from(
        r##"{ "version": "1.0",
          "variables": { "--primary": { "type": "color", "value": "#EA580C" },
                         "--primary-foreground": { "type": "color", "value": "#FFF7ED" },
                         "--foreground": { "type": "color", "value": "#111111" } },
          "children": [{
            "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "cta", "width": 200, "height": 48,
                "layout": "horizontal",
                "fill": [{ "type": "solid", "color": "$--primary" }],
                "children": [
                    { "type": "text", "id": "label", "content": "Search",
                      "fill": [{ "type": "solid", "color": "$--white" }] }
                ]
            }]
          }] }"##,
    );
    repair_broken_variable_refs(&mut sink);
    assert_eq!(
        fill_color(&sink.state, "label").as_deref(),
        Some("$--primary-foreground"),
        "white-on-brand is exactly what the -foreground partner means"
    );
}

#[test]
fn a_broken_container_fill_is_dropped_rather_than_guessed() {
    let mut sink = sink_from(
        r##"{ "version": "1.0",
          "variables": { "--foreground": { "type": "color", "value": "#111111" } },
          "children": [{
            "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "panel", "width": 200, "height": 48,
                  "fill": [{ "type": "solid", "color": "$--surface-raised" }] }
            ]
          }] }"##,
    );
    repair_broken_variable_refs(&mut sink);
    assert_eq!(
        fill_color(&sink.state, "panel"),
        None,
        "an unknown surface colour is not worth guessing — show what is behind it"
    );
}

#[test]
fn a_reference_the_table_defines_is_left_alone() {
    let mut sink = sink_from(
        r##"{ "version": "1.0",
          "variables": { "--muted": { "type": "color", "value": "#F5F5F5" },
                         "--muted-foreground": { "type": "color", "value": "#737373" } },
          "children": [{
            "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "bar", "width": 340, "height": 52,
                "layout": "horizontal",
                "fill": [{ "type": "solid", "color": "$--muted" }],
                "children": [
                    { "type": "text", "id": "placeholder", "content": "Where to?",
                      "fill": [{ "type": "solid", "color": "$--muted-foreground" }] }
                ]
            }]
          }] }"##,
    );
    repair_broken_variable_refs(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "a well-formed token design is untouched: {:?}",
        sink.applied
    );
}

#[test]
fn a_document_with_no_variable_table_is_left_alone() {
    // Nothing to reason from — every `$ref` is equally unresolvable and the
    // design was never token-based.
    let mut sink = sink_from(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "width": 390, "height": 844,
            "children": [
                { "type": "text", "id": "t", "content": "Hi",
                  "fill": [{ "type": "solid", "color": "$--white" }] }
            ]
        }] }"##,
    );
    repair_broken_variable_refs(&mut sink);
    assert!(sink.applied.is_empty());
}

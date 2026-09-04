//! Theme-polarity repair of the BORDER family, and the interlock that keeps
//! it from dragging text down with it.
//!
//! Split from `loop_finalize_tests.rs`, which is at the 800-line ceiling.

use op_editor_core::EditorState;
use serde_json::json;

/// A dark design whose ACTIVE (Light) slot carries stock-light border values —
/// `0808-gm-1.op`'s verbatim palette. `--foreground` is correctly light
/// in both slots and must stay untouched.
fn dusk_state() -> EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "themes": {"Mode": ["Light", "Dark"]},
        "variables": {
            "--border": {"type":"color","value":[
                {"value":"#E2E8F0","theme":{"Mode":"Light"}},
                {"value":"#334155","theme":{"Mode":"Dark"}}]},
            "--input": {"type":"color","value":[
                {"value":"#CBD5E1","theme":{"Mode":"Light"}},
                {"value":"#475569","theme":{"Mode":"Dark"}}]},
            "--foreground": {"type":"color","value":[
                {"value":"#F1F5F9","theme":{"Mode":"Light"}},
                {"value":"#F1F5F9","theme":{"Mode":"Dark"}}]}
        },
        "children": [
            {"type":"frame","id":"root","name":"Page","width":1200,"height":2977,
             "layout":"vertical",
             "fill":[{"type":"solid","color":"#0A0A0A"}],
             "children":[]}
        ]
    }))
    .expect("valid doc");
    EditorState::from_document(doc)
}

fn heal(state: &mut EditorState) {
    let mut sink = crate::loop_finalize::StateDocSink { state };
    crate::loop_finalize::fix_theme_variable_polarity(&mut sink);
}

fn resolved(state: &EditorState, name: &str) -> String {
    state
        .resolve_color_variable_hex(name)
        .unwrap_or_else(|| panic!("{name} resolves"))
}

#[test]
fn a_dark_design_pulls_its_border_tokens_to_the_dark_slot() {
    // `border` was absent from the polarity families, so a near-WHITE hairline
    // survived on a #0A0A0A page — and the widget renderer paints a tabs bar
    // with the node's STROKE colour, turning that hairline into a white slab.
    let mut state = dusk_state();
    assert!(resolved(&state, "--border").eq_ignore_ascii_case("#E2E8F0"));

    heal(&mut state);

    assert!(
        resolved(&state, "--border").eq_ignore_ascii_case("#334155"),
        "got {}",
        resolved(&state, "--border")
    );
    assert!(
        resolved(&state, "--input").eq_ignore_ascii_case("#475569"),
        "got {}",
        resolved(&state, "--input")
    );
    assert!(
        resolved(&state, "--foreground").eq_ignore_ascii_case("#F1F5F9"),
        "a correctly-light text token on a dark page is untouched"
    );
}

#[test]
fn a_light_design_keeps_its_light_borders() {
    // The repair is polarity-driven, not "always prefer dark": on a white page
    // the same border token is already correct and must not flip.
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "themes": {"Mode": ["Light", "Dark"]},
        "variables": {
            "--border": {"type":"color","value":[
                {"value":"#E2E8F0","theme":{"Mode":"Light"}},
                {"value":"#334155","theme":{"Mode":"Dark"}}]}
        },
        "children": [
            {"type":"frame","id":"root","name":"Page","width":1200,"height":900,
             "layout":"vertical",
             "fill":[{"type":"solid","color":"#FFFFFF"}],
             "children":[]}
        ]
    }))
    .expect("valid doc");
    let mut state = EditorState::from_document(doc);
    heal(&mut state);
    assert!(resolved(&state, "--border").eq_ignore_ascii_case("#E2E8F0"));
}

#[test]
fn healing_the_border_token_does_not_darken_the_headlines() {
    // THE INTERLOCK. These two repairs are only safe together.
    //
    // Before the binding pass became slot-aware, a headline authored
    // `#E2E8F0` bound to `$--border` purely because the hexes matched.
    // Pulling `--border` to its dark slot (the test above) would then have
    // repainted every headline #334155 — hairline grey on a #0A0A0A page.
    //
    // This asserts the defence actually holds when both run: bind first, heal
    // second, headline still near-white.
    let mut state = dusk_state();

    let mut nodes: Vec<jian_ops_schema::node::PenNode> = serde_json::from_value(json!([
        {"type":"text","id":"h","content":"Multimodal Neural Synthesis",
         "fontSize":36,"fontWeight":500,
         "fill":[{"type":"solid","color":"#E2E8F0"}]}
    ]))
    .expect("valid PenNode forest");
    crate::variable_binding::bind_generated_color_variables(&mut nodes, &state);

    let bound = serde_json::to_value(&nodes[0]).expect("serializes");
    let authored = bound["fill"][0]["color"]
        .as_str()
        .expect("a solid fill colour")
        .to_string();
    assert_ne!(
        authored, "$--border",
        "the slot filter must refuse a border token on a glyph"
    );

    heal(&mut state);

    // Resolve the headline the way the renderer would, AFTER the palette moved.
    let painted = match authored.strip_prefix('$') {
        Some(name) => resolved(&state, name),
        None => authored.clone(),
    };
    let luminance =
        crate::loop_finalize::hex_luminance(&painted).unwrap_or_else(|| panic!("{painted} parses"));
    assert!(
        luminance > 0.5,
        "headline stayed light after the border repair — painted {painted} (luminance {luminance})"
    );
    // And the repair really did move the border, so this is not a vacuous pass.
    assert!(resolved(&state, "--border").eq_ignore_ascii_case("#334155"));
}

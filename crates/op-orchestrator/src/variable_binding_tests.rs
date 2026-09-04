//! Slot-awareness of `bind_generated_color_variables`.
//!
//! The fixture is `0808-gm-1.op`'s real palette: a dark design whose
//! ACTIVE (Light-slot) `--border` is the near-white `#E2E8F0`, close
//! enough to a heading's authored white that colour distance alone bound
//! every title in the document to the border token.

use super::*;
use serde_json::json;

fn state_with_dusk_palette() -> EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "themes": {"Mode": ["Light", "Dark"]},
        "variables": {
            "--border": {"type":"color","value":[
                {"value":"#E2E8F0","theme":{"Mode":"Light"}},
                {"value":"#334155","theme":{"Mode":"Dark"}}]},
            "--foreground": {"type":"color","value":[
                {"value":"#F1F5F9","theme":{"Mode":"Light"}},
                {"value":"#F1F5F9","theme":{"Mode":"Dark"}}]},
            "--card": {"type":"color","value":[
                {"value":"#1E293B","theme":{"Mode":"Light"}},
                {"value":"#1E293B","theme":{"Mode":"Dark"}}]},
            "--primary": {"type":"color","value":[
                {"value":"#6B62F2","theme":{"Mode":"Light"}},
                {"value":"#6B62F2","theme":{"Mode":"Dark"}}]}
        },
        "children": []
    }))
    .expect("valid doc");
    EditorState::from_document(doc)
}

fn bind_one(value: serde_json::Value) -> serde_json::Value {
    let state = state_with_dusk_palette();
    let mut nodes: Vec<PenNode> = vec![serde_json::from_value(value).expect("valid PenNode")];
    bind_generated_color_variables(&mut nodes, &state);
    serde_json::to_value(&nodes[0]).expect("serializes")
}

#[test]
fn a_heading_is_not_bound_to_the_border_token() {
    // The measured defect: `#E2E8F0` IS `$--border`'s active value, so
    // distance-only matching bound a 36px headline to the hairline token.
    // Nothing rendered differently — until the theme flipped, or the palette
    // repair pulled `--border` to its dark slot, and the headline went
    // with it.
    let bound = bind_one(json!({
        "type":"text","id":"h","content":"Multimodal Neural Synthesis",
        "fontSize":36,"fontWeight":500,
        "fill":[{"type":"solid","color":"#E2E8F0"}]
    }));
    assert_eq!(
        bound["fill"][0]["color"], "#E2E8F0",
        "a glyph colour keeps its literal rather than binding to a border token"
    );
}

#[test]
fn the_same_hex_on_a_hairline_still_binds() {
    // Slot-awareness must not disarm binding: on the slot the token is FOR,
    // the very same colour still resolves to `$--border`.
    let bound = bind_one(json!({
        "type":"frame","id":"card","layout":"vertical",
        "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#E2E8F0"}]},
        "children":[]
    }));
    assert_eq!(bound["stroke"]["fill"][0]["color"], "$--border");
}

#[test]
fn a_surface_is_not_bound_to_a_text_token() {
    // The mirror category error — a container filled with a text token. The
    // surface-discipline pass repairs this downstream by REWRITING the fill;
    // refusing the bind keeps the author's actual colour instead.
    let bound = bind_one(json!({
        "type":"frame","id":"button","layout":"horizontal",
        "fill":[{"type":"solid","color":"#F1F5F9"}],
        "children":[]
    }));
    assert_eq!(bound["fill"][0]["color"], "#F1F5F9");
}

#[test]
fn a_semantic_colour_still_themes_every_slot() {
    // Accent / destructive / chart colours carry meaning, not a slot, so the
    // denylist must let them through everywhere. Without this the change
    // would strip theming from every accent-coloured label and icon.
    let text = bind_one(json!({
        "type":"text","id":"t","content":"Live",
        "fill":[{"type":"solid","color":"#6B62F2"}]
    }));
    assert_eq!(text["fill"][0]["color"], "$--primary");

    let icon = bind_one(json!({
        "type":"icon_font","id":"i","iconFontName":"zap","width":14,"height":14,
        "fill":[{"type":"solid","color":"#6B62F2"}]
    }));
    assert_eq!(icon["fill"][0]["color"], "$--primary");

    let surface = bind_one(json!({
        "type":"frame","id":"pill","layout":"horizontal",
        "fill":[{"type":"solid","color":"#6B62F2"}],
        "children":[]
    }));
    assert_eq!(surface["fill"][0]["color"], "$--primary");
}

#[test]
fn a_glyph_still_binds_to_the_text_family() {
    // The positive case for text: an exact `$--foreground` hex on a
    // text node binds, so the slot filter is a category guard and not a
    // blanket refusal to theme text.
    let bound = bind_one(json!({
        "type":"text","id":"t","content":"4K Render",
        "fill":[{"type":"solid","color":"#F1F5F9"}]
    }));
    assert_eq!(bound["fill"][0]["color"], "$--foreground");
}

#[test]
fn family_of_reads_the_naming_convention() {
    assert_eq!(family_of("--foreground"), ColorFamily::Text);
    // A state TEXT token, not a state background — "text" must win.
    assert_eq!(family_of("--color-error-foreground"), ColorFamily::Text);
    assert_eq!(family_of("--color-error"), ColorFamily::Surface);
    assert_eq!(family_of("--input"), ColorFamily::Border);
    assert_eq!(family_of("--muted"), ColorFamily::Surface);
    assert_eq!(family_of("--primary"), ColorFamily::Semantic);
    assert_eq!(family_of("--chart-1"), ColorFamily::Semantic);
}

//! Tests for [`super::template_provenance`].

use super::*;
use op_editor_core::scene_template_catalog::scene_template_catalogue;
use op_editor_core::EditorState;

fn state_with_variables(names: &[&str]) -> EditorState {
    let mut state = EditorState::new();
    let mut table = std::collections::BTreeMap::new();
    for name in names {
        table.insert(
            (*name).to_string(),
            serde_json::from_value(serde_json::json!({"type": "color", "value": "#123456"}))
                .expect("variable fixture"),
        );
    }
    state.doc.variables = Some(table);
    state
}

/// The append path is what produces the namespaced names this matches, so the
/// contract is asserted against that path rather than against a literal —
/// a change to the separator there fails here instead of silently turning the
/// signal off.
#[test]
fn namespaced_variable_names_are_the_contract_this_matches() {
    let deck = op_editor_core::scene_template_catalog::scene_template_by_id("slide-deck")
        .expect("the deck template ships");
    let boards = op_editor_core::scene_template_append::template_boards(deck.document(), &deck.id)
        .expect("the shipped template parses");

    let names: Vec<&str> = boards.variables.keys().map(String::as_str).collect();
    assert!(!names.is_empty(), "the deck template declares variables");

    let state = state_with_variables(&names);
    let provenance = template_provenance(&state).expect("appended boards are template provenance");
    assert_eq!(provenance.template_id, "slide-deck");
    assert_eq!(provenance.evidence, TemplateEvidence::NamespacedVariables);
}

#[test]
fn an_ordinary_generated_palette_is_not_provenance() {
    // The names a generation actually produces, plus a user-authored name that
    // happens to carry the separator. Neither resolves to a shipped template.
    let state = state_with_variables(&[
        "--background",
        "--foreground",
        "--primary",
        "c-bg",
        "my-thing--tone",
    ]);
    assert_eq!(template_provenance(&state), None);
}

#[test]
fn an_empty_or_absent_variable_table_is_not_provenance() {
    assert_eq!(template_provenance(&EditorState::new()), None);
    assert_eq!(template_provenance(&state_with_variables(&[])), None);
}

#[test]
fn the_generate_row_basis_is_provenance_with_nothing_on_the_tree() {
    // The "generate from this" door resets the canvas to a blank starter, so
    // the tree carries no evidence at all — the pin is the only signal, and it
    // has to be enough.
    let mut state = EditorState::starter();
    state.editor_ui.scene_template_center.generate_basis = Some("minimal-keynote".to_string());

    let provenance = template_provenance(&state).expect("a pinned basis is provenance");
    assert_eq!(provenance.template_id, "minimal-keynote");
    assert_eq!(provenance.evidence, TemplateEvidence::GenerateBasis);
}

#[test]
fn a_basis_the_catalogue_does_not_know_is_not_provenance() {
    let mut state = EditorState::starter();
    state.editor_ui.scene_template_center.generate_basis = Some("not-a-template".to_string());
    assert_eq!(template_provenance(&state), None);
}

/// Every shipped template must be detectable through the append door, or the
/// gate silently protects only some of the catalogue.
#[test]
fn every_shipped_template_is_detectable_from_its_appended_variables() {
    for template in scene_template_catalogue() {
        let boards = op_editor_core::scene_template_append::template_boards(
            template.document(),
            &template.id,
        )
        .unwrap_or_else(|| panic!("{} parses", template.id));
        if boards.variables.is_empty() {
            // A template with no palette leaves no trace to match; the
            // generate-basis door still covers it.
            continue;
        }
        let names: Vec<&str> = boards.variables.keys().map(String::as_str).collect();
        let state = state_with_variables(&names);
        assert_eq!(
            template_provenance(&state).map(|p| p.template_id),
            Some(template.id.clone()),
            "{} is not detectable from its appended variables",
            template.id
        );
    }
}

#[test]
fn the_basis_wins_when_both_facts_hold() {
    let mut state = state_with_variables(&["slide-deck--c-bg"]);
    state.editor_ui.scene_template_center.generate_basis = Some("minimal-keynote".to_string());

    let provenance = template_provenance(&state).expect("provenance");
    assert_eq!(provenance.template_id, "minimal-keynote");
    assert_eq!(provenance.evidence, TemplateEvidence::GenerateBasis);
}

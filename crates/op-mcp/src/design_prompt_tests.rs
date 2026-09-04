//! `get_design_prompt` MCP parity tests.

use std::collections::BTreeMap;

use super::{get_design_prompt_snapshot, McpTool, ToolOutcome};

#[test]
fn get_design_prompt_defaults_to_all_and_lists_sections() {
    let state = op_editor_core::EditorState::new();

    match get_design_prompt_snapshot(&state).call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("section"), Some(&"all".to_string()));
            assert!(out
                .get("availableSections")
                .is_some_and(|sections| sections.contains("\"layout\"")));
            assert!(out
                .get("designPrompt")
                .is_some_and(|prompt| prompt.contains("OpenPencil")));
        }
        other => panic!("expected prompt ok, got {other:?}"),
    }
}

#[test]
fn get_design_prompt_full_uses_rust_mcp_element_compatibility_note() {
    let state = op_editor_core::EditorState::new();

    match get_design_prompt_snapshot(&state).call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            let prompt = out.get("designPrompt").expect("prompt");
            assert!(prompt.contains("RUST MCP ELEMENT TOOL COMPATIBILITY"));
            assert!(prompt.contains("operations"));
            assert!(!prompt.contains("add_section_header_v1"));
        }
        other => panic!("expected prompt ok, got {other:?}"),
    }
}

#[test]
fn get_design_prompt_full_includes_mobile_quality_guardrails() {
    let state = op_editor_core::EditorState::new();

    match get_design_prompt_snapshot(&state).call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            let prompt = out.get("designPrompt").expect("prompt");
            assert!(prompt.contains("AESTHETIC QUALITY BAR"));
            assert!(prompt.contains("MOBILE APP"));
            assert!(prompt.contains("App Content wrapper"));
            assert!(prompt.contains("Avoid crowded output"));
            assert!(prompt.contains("Mobile top rhythm"));
            assert!(prompt.contains("favorite/heart"));
            assert!(prompt.contains("must stay fully inside its card"));
            assert!(prompt.contains("signature moment"));
            assert!(prompt.contains("Do not repeat the same predictable mobile stack"));
        }
        other => panic!("expected prompt ok, got {other:?}"),
    }
}

#[test]
fn get_design_prompt_elements_section_is_script_first_and_edits_with_operations() {
    let state = op_editor_core::EditorState::new();
    let mut args = BTreeMap::new();
    args.insert("section".into(), "elements".into());

    match get_design_prompt_snapshot(&state).call(&args) {
        ToolOutcome::Ok(out) => {
            let prompt = out.get("designPrompt").expect("prompt");
            assert!(prompt.contains("batch_design"));
            assert!(prompt.contains("batch_design.script"));
            assert!(prompt.contains("script-first"));
            assert!(prompt.contains("I(parent, nodeObject)"));
            assert!(prompt.contains("K(\"starter/<id>\""));
            assert!(prompt.contains("batch_design.operations"));
            assert!(prompt.contains("ONLY to edit existing nodes"));
            assert!(prompt.contains("Do not create fresh UI trees with `I()` in operations"));
            assert!(prompt.contains("U(nodeId, patchJson)"));
            assert!(prompt.contains("C(sourceId, parent"));
            assert!(prompt.contains("R(nodeId, nodeJson)"));
            assert!(prompt.contains("G(slotIdOrBinding"));
            assert!(prompt.contains("target must exist and have zero children"));
            assert!(prompt.contains("accepted only on a parent that declares layout"));
            assert!(prompt.contains("never overlays"));
            assert!(prompt.contains("Size the appended binding"));
            assert!(!prompt.contains("add_card_row_v1"));
        }
        other => panic!("expected prompt ok, got {other:?}"),
    }
}

#[test]
fn get_design_prompt_uses_document_design_md_for_style_section() {
    let mut state = op_editor_core::EditorState::new();
    state.doc.design_md = Some(op_editor_core::parse_design_md(
        "# Design System: Aurora\n\n\
         ## Visual Theme\nCalm minimal.\n\n\
         ## Color Palette\n- **Primary** (#3366FF) - accent buttons",
    ));
    let mut args = BTreeMap::new();
    args.insert("section".into(), "style".into());

    match get_design_prompt_snapshot(&state).call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("section"), Some(&"style".to_string()));
            let prompt = out.get("designPrompt").expect("prompt");
            assert!(prompt.contains("DESIGN SYSTEM (from design.md):"));
            assert!(prompt.contains("VISUAL THEME: Calm minimal."));
            assert!(prompt.contains("Primary (#3366FF)"));
        }
        other => panic!("expected prompt ok, got {other:?}"),
    }
}

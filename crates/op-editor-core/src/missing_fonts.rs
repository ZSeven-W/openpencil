//! Missing-font detection for the post-load prompt.
//!
//! This module is platform-free: the diff runs against the family snapshots
//! both hosts maintain on [`EditorUiState`] (system fonts plus imported fonts),
//! so native and web share the same detection logic.

use crate::font_catalog::{
    is_generic_or_system_font_alias, is_same_font_family, primary_concrete_font_family,
    split_font_family_stack,
};
use crate::{EditorState, EditorUiState};
use jian_ops_schema::font_plan::FontPlan;
use jian_ops_schema::node::PenNode;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct MissingFontEntry {
    pub family: String,
    pub run_count: u32,
    /// Set after a supply attempt whose file declared another family.
    pub mismatch_note: Option<String>,
    pub resolved: bool,
}

/// Hoverable controls of the missing-fonts modal and the shared row
/// component — cursor-move hover wash state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingFontsHover {
    ChooseFile(usize),
    RemoveImported(usize),
    Dismiss,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MissingFontsPrompt {
    pub entries: Vec<MissingFontEntry>,
}

fn family_available(family: &str, ui: &EditorUiState) -> bool {
    let family = family.trim();
    if is_generic_or_system_font_alias(family) {
        return true;
    }
    let exact_match = |candidate: &String| candidate.trim().eq_ignore_ascii_case(family);
    ui.imported_font_families.iter().any(exact_match)
        || ui.bundled_font_families.iter().any(exact_match)
        || ui
            .system_font_families
            .iter()
            .any(|candidate| is_same_font_family(family, candidate.trim()))
}

fn family_stack_available(stack: &str, ui: &EditorUiState) -> bool {
    split_font_family_stack(stack)
        .iter()
        .any(|family| family_available(family, ui))
}

fn add_missing_stack(
    missing: &mut BTreeMap<String, MissingFontEntry>,
    stack: &str,
    run_count: u32,
    ui: &EditorUiState,
) {
    if stack.trim().is_empty() || family_stack_available(stack, ui) {
        return;
    }
    let Some(family) = primary_concrete_font_family(stack) else {
        return;
    };
    let key = family.to_ascii_lowercase();
    missing
        .entry(key)
        .and_modify(|entry| entry.run_count += run_count)
        .or_insert(MissingFontEntry {
            family,
            run_count,
            mismatch_note: None,
            resolved: false,
        });
}

/// Font-family stacks authored inside component-instance descendant overrides.
///
/// These values are not concrete `Text` nodes in the document tree, so hosts
/// that register font bytes must include this list in addition to walking
/// normal text nodes. Keeping the scanner shared with missing-font detection
/// prevents an override from being declared available without being loaded.
pub fn component_override_font_family_stacks(state: &EditorState) -> Vec<String> {
    fn scan_json(value: &serde_json::Value, stacks: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(object) => {
                for (key, value) in object {
                    if key == "fontFamily" {
                        if let Some(family) = value.as_str() {
                            stacks.push(family.to_string());
                        }
                    } else {
                        scan_json(value, stacks);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    scan_json(value, stacks);
                }
            }
            _ => {}
        }
    }

    fn scan_nodes(nodes: &[PenNode], stacks: &mut Vec<String>) {
        for node in nodes {
            if let PenNode::Ref(reference) = node {
                if let Some(descendants) = &reference.descendants {
                    for value in descendants.values() {
                        scan_json(value, stacks);
                    }
                }
            }
            if let Some(children) = crate::pen_node_ext::PenNodeExt::children(node) {
                scan_nodes(children, stacks);
            }
        }
    }

    let mut stacks = Vec::new();
    scan_nodes(&state.doc.children, &mut stacks);
    if let Some(pages) = &state.doc.pages {
        for page in pages {
            scan_nodes(&page.children, &mut stacks);
        }
    }
    stacks
}

/// Diff the document's families against the current available-family snapshots.
///
/// Returns `None` when every family is available or system enumeration has not
/// finished yet. Hosts distinguish those cases through
/// `missing_fonts_pending_detect` and retry after enumeration completes.
pub fn detect_missing_fonts(state: &EditorState) -> Option<MissingFontsPrompt> {
    if !state.editor_ui.system_fonts_loaded {
        return None;
    }

    let plan = FontPlan::scan(&state.doc);
    let mut missing = BTreeMap::<String, MissingFontEntry>::new();
    for (stack, usage) in plan.families().filter(|(family, _)| !family.is_empty()) {
        add_missing_stack(&mut missing, stack, usage.run_count, &state.editor_ui);
    }
    for stack in component_override_font_family_stacks(state) {
        add_missing_stack(&mut missing, &stack, 1, &state.editor_ui);
    }
    let entries = missing.into_values().collect::<Vec<_>>();

    (!entries.is_empty()).then_some(MissingFontsPrompt { entries })
}

/// Re-check prompt entries against the current available-family snapshots.
///
/// Returns `true` when every row is resolved so the host can close the modal.
pub fn refresh_prompt(prompt: &mut MissingFontsPrompt, ui: &EditorUiState) -> bool {
    for entry in &mut prompt.entries {
        if !entry.resolved && family_available(&entry.family, ui) {
            entry.resolved = true;
            entry.mismatch_note = None;
        }
    }
    prompt.entries.iter().all(|entry| entry.resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_text(family: &str) -> EditorState {
        let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
            "version": "0.8.0",
            "children": [{
                "type": "text",
                "id": "t1",
                "name": "t",
                "x": 0,
                "y": 0,
                "width": 10,
                "height": 10,
                "content": "hi",
                "fontFamily": family
            }]
        }))
        .expect("document");
        EditorState::from_document(doc)
    }

    #[test]
    fn missing_family_is_detected_with_run_count() {
        let mut state = state_with_text("Katibeh");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families = std::sync::Arc::new(vec!["Arial".into()]);

        let prompt = detect_missing_fonts(&state).expect("prompt");
        assert_eq!(prompt.entries.len(), 1);
        assert_eq!(prompt.entries[0].family, "Katibeh");
        assert!(prompt.entries[0].run_count >= 1);
    }

    #[test]
    fn windows_ui_family_split_does_not_report_the_authored_family_missing() {
        // Issue #211: Windows enumerates `Microsoft YaHei UI` for msyh.ttc;
        // a document authored with `Microsoft YaHei` must not prompt. This
        // is the documented same-file alias, not a general `… UI` fold.
        let mut state = state_with_text("Microsoft YaHei");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families =
            std::sync::Arc::new(vec!["Microsoft YaHei UI".into()]);

        assert!(detect_missing_fonts(&state).is_none());
    }

    #[test]
    fn windows_ui_family_split_matches_in_both_directions() {
        // A document authored on Windows with the UI spelling must also
        // resolve against a machine enumerating the plain family.
        let mut state = state_with_text("Microsoft YaHei UI");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families =
            std::sync::Arc::new(vec!["Arial".into(), "Microsoft YaHei".into()]);

        assert!(detect_missing_fonts(&state).is_none());
    }

    #[test]
    fn imported_ui_family_does_not_satisfy_a_system_alias_row() {
        // The documented alias is a system-font enumeration fact, not a
        // license to treat an unrelated imported file as the same face.
        let mut state = state_with_text("Microsoft YaHei");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.imported_font_families =
            std::sync::Arc::new(vec!["Microsoft YaHei UI".into()]);

        let prompt = detect_missing_fonts(&state).expect("imported alias stays distinct");
        assert_eq!(prompt.entries[0].family, "Microsoft YaHei");
    }

    #[test]
    fn unrelated_ui_family_stays_missing() {
        let mut state = state_with_text("Adventure Works Sans");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families =
            std::sync::Arc::new(vec!["Arial".into(), "Contoso UI".into()]);

        let prompt = detect_missing_fonts(&state).expect("distinct family stays missing");
        assert_eq!(prompt.entries[0].family, "Adventure Works Sans");
    }

    #[test]
    fn yu_gothic_ui_does_not_satisfy_yu_gothic() {
        // The over-broad `Name UI ≡ Name` fold treated these as one family
        // and silently dropped the missing-font prompt. They are distinct
        // faces with different vertical metrics.
        let mut state = state_with_text("Yu Gothic");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families = std::sync::Arc::new(vec!["Yu Gothic UI".into()]);

        let prompt = detect_missing_fonts(&state).expect("Yu Gothic stays missing");
        assert_eq!(prompt.entries[0].family, "Yu Gothic");
    }

    #[test]
    fn available_families_produce_no_prompt() {
        let mut state = state_with_text("Arial");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families = std::sync::Arc::new(vec!["Arial".into()]);

        assert!(detect_missing_fonts(&state).is_none());
    }

    #[test]
    fn imported_families_count_as_available() {
        let mut state = state_with_text("Katibeh");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.imported_font_families = std::sync::Arc::new(vec!["Katibeh".into()]);

        assert!(detect_missing_fonts(&state).is_none());
    }

    #[test]
    fn family_matching_is_ascii_case_insensitive() {
        let mut state = state_with_text("kAtIbEh");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families = std::sync::Arc::new(vec!["KATIBEH".into()]);

        assert!(detect_missing_fonts(&state).is_none());
    }

    #[test]
    fn css_stack_matches_each_quoted_candidate_case_insensitively() {
        let mut state = state_with_text(
            r#"Missing, ui-sans-serif, system-ui, -apple-system, "pInGfAnG sC", sans-serif"#,
        );
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families = std::sync::Arc::new(vec!["PingFang SC".into()]);

        assert!(detect_missing_fonts(&state).is_none());
    }

    #[test]
    fn generic_and_platform_aliases_never_prompt_for_font_files() {
        for family in [
            "SANS-SERIF",
            "system-UI",
            "UI-SANS-SERIF",
            "-APPLE-SYSTEM",
            "BlinkMacSystemFont",
        ] {
            let mut state = state_with_text(family);
            state.editor_ui.system_fonts_loaded = true;
            assert!(detect_missing_fonts(&state).is_none(), "{family}");
        }
    }

    #[test]
    fn bundled_families_are_available_without_system_enumeration_entries() {
        let mut state = state_with_text("iNtEr");
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.bundled_font_families =
            std::sync::Arc::new(vec!["INTER".to_string(), "Space Grotesk".to_string()]);
        assert!(state.editor_ui.system_font_families.is_empty());
        assert!(detect_missing_fonts(&state).is_none());
    }

    #[test]
    fn catalog_name_is_not_available_until_the_renderer_registers_it() {
        let mut state = state_with_text("Poppins");
        state.editor_ui.system_fonts_loaded = true;

        let prompt = detect_missing_fonts(&state).expect("not registered");
        assert_eq!(prompt.entries[0].family, "Poppins");
    }

    #[test]
    fn unavailable_stack_reports_only_its_primary_family() {
        let mut state = state_with_text(r#""Missing Display", "Missing Text""#);
        state.editor_ui.system_fonts_loaded = true;

        let prompt = detect_missing_fonts(&state).expect("prompt");
        assert_eq!(prompt.entries.len(), 1);
        assert_eq!(prompt.entries[0].family, "Missing Display");
    }

    #[test]
    fn ref_descendant_override_families_are_detected_case_insensitively() {
        let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
            "version": "1.0.0",
            "children": [{
                "type": "ref",
                "id": "instance",
                "ref": "component",
                "descendants": {
                    "label": {
                        "fontFamily": "Missing Override",
                        "content": [{"text": "Hi", "fontFamily": "AVAILABLE FONT"}]
                    }
                }
            }]
        }))
        .expect("document");
        let mut state = EditorState::from_document(doc);
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.system_font_families = std::sync::Arc::new(vec!["available font".into()]);

        let prompt = detect_missing_fonts(&state).expect("missing override prompt");
        assert_eq!(prompt.entries.len(), 1);
        assert_eq!(prompt.entries[0].family, "Missing Override");
    }

    #[test]
    fn unloaded_system_fonts_defer_detection() {
        let state = state_with_text("Katibeh");
        assert!(
            detect_missing_fonts(&state).is_none(),
            "defer instead of reporting a false positive"
        );
    }

    #[test]
    fn refresh_marks_resolved_and_reports_all_done() {
        let mut prompt = MissingFontsPrompt {
            entries: vec![MissingFontEntry {
                family: "Katibeh".into(),
                run_count: 3,
                mismatch_note: Some("wrong family".into()),
                resolved: false,
            }],
        };
        let ui = EditorUiState {
            imported_font_families: std::sync::Arc::new(vec!["Katibeh".into()]),
            ..EditorUiState::default()
        };

        assert!(refresh_prompt(&mut prompt, &ui), "all rows resolved");
        assert!(prompt.entries[0].resolved);
        assert_eq!(prompt.entries[0].mismatch_note, None);
    }

    #[test]
    fn refresh_keeps_unavailable_rows_unresolved() {
        let mut prompt = MissingFontsPrompt {
            entries: vec![MissingFontEntry {
                family: "Katibeh".into(),
                run_count: 1,
                mismatch_note: None,
                resolved: false,
            }],
        };

        assert!(!refresh_prompt(&mut prompt, &EditorUiState::default()));
        assert!(!prompt.entries[0].resolved);
    }
}

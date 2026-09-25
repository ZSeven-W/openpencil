//! Seed the palette tokens a finalized document references but never defined.
//!
//! The orchestrator seeds the full semantic palette before a run
//! (`variables::seed_commands`), so the role passes may paint `$--accent`,
//! `$--card`, `$--border` … and trust them to resolve. The agentic-loop path
//! (MCP file sessions, the built-in loop) has no such seed: a document the
//! model wrote with literal colours ends up with dangling refs the canvas
//! cannot resolve. After the loop's passes run, every referenced token the
//! palette knows and the document lacks is merged in with its default value;
//! tokens the document already defines are never touched.

use std::collections::BTreeSet;

use op_editor_core::{EditorCommand, EditorState};
use serde_json::Value;

use crate::semantic_palette;

/// Merge the palette defaults for referenced-but-undefined tokens. Returns
/// the applied command (for the recording finalizer), `None` when nothing
/// was missing.
pub(crate) fn seed_referenced_palette_tokens(state: &mut EditorState) -> Option<EditorCommand> {
    let command = palette_seed_command(state)?;
    state.apply(command.clone()).then_some(command)
}

fn palette_seed_command(state: &EditorState) -> Option<EditorCommand> {
    let mut referenced = BTreeSet::new();
    for node in state.active_children() {
        if let Ok(value) = serde_json::to_value(node) {
            collect_token_refs(&value, &mut referenced);
        }
    }
    let defined = state.doc.variables.as_ref();
    let palette = semantic_palette::palette_variables();
    let variables: std::collections::BTreeMap<_, _> = referenced
        .into_iter()
        .filter(|name| !defined.is_some_and(|vars| vars.contains_key(name)))
        .filter_map(|name| palette.get(&name).map(|def| (name, def.clone())))
        .collect();
    if variables.is_empty() {
        return None;
    }
    let themes = semantic_palette::palette_themes()
        .into_iter()
        .filter(|(axis, _)| {
            !state
                .doc
                .themes
                .as_ref()
                .is_some_and(|axes| axes.contains_key(axis))
        })
        .collect();
    Some(EditorCommand::MergeThemePreset { variables, themes })
}

fn collect_token_refs(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            if let Some(name) = text.strip_prefix('$') {
                if name.starts_with("--") {
                    out.insert(name.to_string());
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect_token_refs(item, out)),
        Value::Object(map) => map.values().for_each(|item| collect_token_refs(item, out)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state_with(doc: Value) -> EditorState {
        EditorState::from_document(serde_json::from_value(doc).expect("valid doc"))
    }

    #[test]
    fn referenced_palette_tokens_are_seeded_and_defined_ones_kept() {
        let mut state = state_with(json!({
            "version": "1.0",
            "variables": {"--primary": {"type": "color", "value": "#123456"}},
            "children": [{
                "type": "frame", "id": "tile",
                "fill": [{"type": "solid", "color": "$--accent"}],
                "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "$--border"}]},
                "children": [
                    {"type": "text", "id": "t", "content": "Go",
                     "fill": [{"type": "solid", "color": "$--primary"}]},
                    {"type": "text", "id": "u", "content": "?",
                     "fill": [{"type": "solid", "color": "$--not-a-palette-token"}]}
                ]
            }]
        }));
        assert!(seed_referenced_palette_tokens(&mut state).is_some());
        let vars = state.doc.variables.as_ref().expect("variables");
        assert!(vars.contains_key("--accent") && vars.contains_key("--border"));
        assert!(!vars.contains_key("--not-a-palette-token"));
        assert_eq!(
            state.resolve_color_variable_hex("--primary").as_deref(),
            Some("#123456"),
            "a token the document defines is never overwritten"
        );
        assert!(state.resolve_color_variable_hex("--accent").is_some());
    }

    #[test]
    fn a_document_without_refs_is_left_alone() {
        let mut state = state_with(json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "f",
                "fill": [{"type": "solid", "color": "#FFFFFF"}], "children": []}]
        }));
        assert!(seed_referenced_palette_tokens(&mut state).is_none());
        assert!(state
            .doc
            .variables
            .as_ref()
            .is_none_or(|vars| vars.is_empty()));
    }
}

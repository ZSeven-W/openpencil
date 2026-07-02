//! Component-library loading + merge.
//!
//! A *component library* is an ordinary `.op` / `.lib.op` document whose
//! top-level (or per-page) frames carry `reusable: true`. Loading one into
//! a working document makes those masters addressable by `ref` nodes during
//! AI generation — the "design-kit composition" path.
//!
//! [`merge_library_into_state`] reads such a file via the canonical loader,
//! harvests its reusable masters with
//! [`op_editor_core::ComponentLibrary::from_document`], and merges them into
//! a live [`EditorState`]:
//!
//! 1. the master frames are appended to `state.doc.children` (deduped by id),
//! 2. the library's `variables` + `themes` are merged into the doc (so each
//!    master's `$--token` fills resolve), and
//! 3. `state.components` is rebuilt so the runtime registry + the generator's
//!    available-components manifest see the new masters immediately.
//!
//! This is intentionally additive and gated: nothing calls it on the default
//! path. The smoke runner wires it behind `OPENPENCIL_SMOKE_LIBRARY`, and the
//! desktop host can call it when a user imports a kit.

use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::{ComponentLibrary, EditorState};

use crate::payload::load_canonical;

/// Outcome of a successful library merge — counts for logging / tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryMergeReport {
    /// Reusable master frames appended to `doc.children` (post-dedup).
    pub masters_added: usize,
    /// Total reusable masters now registered in `state.components`.
    pub component_count: usize,
    /// Variable definitions merged in from the library (newly added only).
    pub variables_added: usize,
    /// Theme axes merged in from the library (newly added only).
    pub themes_added: usize,
}

/// Load the `.lib.op` at `path`, harvest its reusable masters, and merge them
/// (plus its variables + themes) into `state`. Existing document content is
/// preserved; masters/variables/themes whose ids already exist are skipped so
/// re-importing the same library is idempotent.
///
/// Returns the merge report on success, or a human-readable error string when
/// the file can't be read or parsed. On error `state` is left unchanged.
pub fn merge_library_into_state(
    state: &mut EditorState,
    path: &str,
) -> Result<LibraryMergeReport, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read library {path}: {e}"))?;
    merge_library_src_into_state(state, &src).map_err(|e| format!("load library {path}: {e}"))
}

/// Source-string variant of [`merge_library_into_state`] — parses canonical
/// `.op` JSON already in memory. Shared core so tests can drive it without a
/// temp file.
pub fn merge_library_src_into_state(
    state: &mut EditorState,
    src: &str,
) -> Result<LibraryMergeReport, String> {
    let loaded = load_canonical(src).map_err(|e| e.to_string())?;
    let lib_doc = loaded.value;

    // Harvest reusable masters from the library document (top-level + pages).
    let library = ComponentLibrary::from_document(&lib_doc);

    let mut report = LibraryMergeReport::default();

    // 1. Append master frames to the working doc, deduped by id.
    let existing_ids: std::collections::HashSet<String> = state
        .doc
        .children
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();
    let mut added_ids = existing_ids.clone();
    for component in &library.components {
        let id = component.root.id_str().to_string();
        if added_ids.contains(&id) {
            continue;
        }
        added_ids.insert(id);
        state.doc.children.push(component.root.clone());
        report.masters_added += 1;
    }

    // 2. Merge the library's variables (only new names) so master `$--token`
    //    fills resolve against concrete values.
    if let Some(lib_vars) = lib_doc.variables.as_ref() {
        let dst = state.doc.variables.get_or_insert_with(Default::default);
        for (name, def) in lib_vars {
            if !dst.contains_key(name) {
                dst.insert(name.clone(), def.clone());
                report.variables_added += 1;
            }
        }
    }

    // 3. Merge the library's theme axes (only new axis names).
    if let Some(lib_themes) = lib_doc.themes.as_ref() {
        let dst = state.doc.themes.get_or_insert_with(Default::default);
        for (axis, values) in lib_themes {
            if !dst.contains_key(axis) {
                dst.insert(axis.clone(), values.clone());
                report.themes_added += 1;
            }
        }
    }

    // 4. Rebuild the runtime component registry off the merged document so the
    //    generator's available-components manifest sees the new masters now.
    state.components = ComponentLibrary::from_document(&state.doc);
    report.component_count = state.components.len();

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One reusable master frame as canonical `.op` JSON (a button-like
    /// frame whose fill references a `$--primary` token so the merge's
    /// variable handling matters). Built with `serde_json` so color
    /// literals like `#18181B` don't trip the raw-string `r#` prefix rules.
    fn reusable_frame_json(id: &str, name: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "type": "frame",
            "name": name,
            "reusable": true,
            "width": 120,
            "height": 40,
            "layout": "horizontal",
            "cornerRadius": 8,
            "fill": [{ "type": "solid", "color": "$--primary" }],
            "children": [
                { "id": format!("{id}-label"), "type": "text", "name": "Label",
                  "content": name, "fontSize": 14 }
            ],
        })
    }

    /// A library document (JSON) with `n` reusable masters + variables + a
    /// theme axis. Serialized to a canonical `.op` JSON string so it goes
    /// through the exact same `load_canonical` path a real `.lib.op` file
    /// would.
    fn library_src(n: usize) -> String {
        let children: Vec<serde_json::Value> = (0..n)
            .map(|i| reusable_frame_json(&format!("lib-comp-{i}"), &format!("Component {i}")))
            .collect();
        let doc = serde_json::json!({
            "version": "1.0",
            "name": "Test Library",
            "themes": { "mode": ["light", "dark"] },
            "variables": {
                "--primary": { "type": "color", "value": "#18181B" },
                "--surface": { "type": "color", "value": "#FAFAFA" },
            },
            "children": children,
        });
        serde_json::to_string(&doc).unwrap()
    }

    #[test]
    fn merges_masters_variables_and_themes_into_empty_state() {
        let mut state = EditorState::new();
        let src = library_src(120);
        let report = merge_library_src_into_state(&mut state, &src).expect("merge");

        assert_eq!(report.masters_added, 120);
        assert_eq!(report.component_count, 120);
        assert!(
            report.component_count > 100,
            "component count must exceed 100"
        );
        assert_eq!(report.variables_added, 2);
        assert_eq!(report.themes_added, 1);
        // The masters landed in the working doc.
        assert_eq!(state.doc.children.len(), 120);
        // Variables + themes are present so master `$--token` fills resolve.
        assert!(state
            .doc
            .variables
            .as_ref()
            .unwrap()
            .contains_key("--primary"));
        assert!(state.doc.themes.as_ref().unwrap().contains_key("mode"));
        // The runtime registry sees them too.
        assert_eq!(state.components.len(), 120);
        assert!(state.components.find_by_name("Component 7").is_some());
    }

    #[test]
    fn dedup_keeps_existing_and_is_idempotent() {
        let mut state = EditorState::new();
        let src = library_src(5);
        let first = merge_library_src_into_state(&mut state, &src).expect("first");
        assert_eq!(first.masters_added, 5);
        // Re-importing the same library adds nothing new.
        let second = merge_library_src_into_state(&mut state, &src).expect("second");
        assert_eq!(second.masters_added, 0);
        assert_eq!(second.variables_added, 0);
        assert_eq!(second.themes_added, 0);
        // Component count is stable.
        assert_eq!(second.component_count, 5);
        assert_eq!(state.doc.children.len(), 5);
    }

    #[test]
    fn preserves_existing_document_content() {
        // A working document with one ordinary (non-reusable) user frame.
        let user_doc_src = r#"{"version":"1.0","children":[
            {"id":"user-frame","type":"frame","name":"User Frame","width":200,"height":100}
        ]}"#;
        let loaded = load_canonical(user_doc_src).expect("user doc");
        let mut state = EditorState::from_document(loaded.value);
        let src = library_src(3);
        let report = merge_library_src_into_state(&mut state, &src).expect("merge");
        assert_eq!(report.masters_added, 3);
        // User content survived + masters appended after it.
        assert_eq!(state.doc.children.len(), 4);
        assert_eq!(state.doc.children[0].id_str(), "user-frame");
    }

    #[test]
    fn bad_source_leaves_state_unchanged() {
        let mut state = EditorState::new();
        let err = merge_library_src_into_state(&mut state, "not json").unwrap_err();
        assert!(!err.is_empty());
        assert!(state.doc.children.is_empty());
        assert!(state.components.is_empty());
    }
}

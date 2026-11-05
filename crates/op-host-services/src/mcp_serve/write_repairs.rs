//! Repairs shared by every bare-MCP document write, after the command has
//! applied and before the document saves.

use std::collections::HashSet;

use op_editor_core::{EditorState, PenNodeExt};

/// Which top-level screens a write may bring to the canonical mobile shape
/// (status bar, viewport height).
///
/// Only roots a design-generation tool creates qualify. Screens that
/// already exist are the author's: re-inserting a status bar after every
/// write made deleting, moving or hiding it a silent no-op, and a save
/// stamped one onto every untouched phone-sized board (issue #274).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MobileRepairScope {
    /// Leave every screen as authored.
    None,
    /// Repair only roots absent from this pre-write id set.
    NewRoots(HashSet<String>),
}

/// The repair scope for a write by `tool`, captured before it applies.
pub(crate) fn mobile_repair_scope(tool: &str, state: &EditorState) -> MobileRepairScope {
    if !super::auto_finalize::arms_auto_finalize(tool) {
        return MobileRepairScope::None;
    }
    MobileRepairScope::NewRoots(
        state
            .active_children()
            .iter()
            .map(|root| root.id_str().to_string())
            .collect(),
    )
}

fn normalize_mobile_screens_after_apply(
    state: &mut EditorState,
    existing_roots: &HashSet<String>,
) -> op_chat_agent::MobileNormalizeReport {
    let report = op_chat_agent::normalize_new_mobile_screens(state, existing_roots);
    let mut changes = Vec::new();
    if report.status_bars_inserted > 0 {
        changes.push(format!(
            "status_bars_inserted={}",
            report.status_bars_inserted
        ));
    }
    if report.status_bars_replaced > 0 {
        changes.push(format!(
            "status_bars_replaced={}",
            report.status_bars_replaced
        ));
    }
    if report.duplicate_status_bars_removed > 0 {
        changes.push(format!(
            "duplicate_status_bars_removed={}",
            report.duplicate_status_bars_removed
        ));
    }
    if report.viewport_heights_fixed > 0 {
        changes.push(format!(
            "viewport_heights_fixed={}",
            report.viewport_heights_fixed
        ));
    }
    if !changes.is_empty() {
        eprintln!(
            "openpencil-desktop mcp: mobile normalization: {}",
            changes.join(", ")
        );
    }
    report
}

fn normalize_icon_paths_after_apply(
    state: &mut EditorState,
) -> op_editor_core::icon_path_normalize::IconPathNormalizeReport {
    let report = op_editor_core::icon_path_normalize::normalize_icon_paths(
        state,
        op_editor_ui::widgets::icons::lucide_name_for_path_d,
    );
    if report != Default::default() {
        eprintln!(
            "openpencil-desktop mcp: icon path normalization: converted_to_icon_font={}, refit_uniform={}",
            report.converted_to_icon_font, report.refit_uniform
        );
    }
    report
}

/// Run the two cheap, idempotent repair passes shared by every bare-MCP
/// document write after the command itself has applied and before it saves.
/// `scope` comes from [`mobile_repair_scope`], taken BEFORE the apply.
pub(crate) fn run_write_repairs_after_apply(
    state: &mut EditorState,
    scope: &MobileRepairScope,
) -> bool {
    if let MobileRepairScope::NewRoots(existing_roots) = scope {
        normalize_mobile_screens_after_apply(state, existing_roots);
    }
    normalize_icon_paths_after_apply(state);
    let mobile_nav_reflowed = op_orchestrator::repair_mobile_trailing_nav_reflow(state);
    let image_slots_materialized =
        op_orchestrator::cleanup_image_slots::materialize_empty_image_fill_slots(state);
    mobile_nav_reflowed || image_slots_materialized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone_doc() -> EditorState {
        let doc = jian_ops_schema::load_str(
            r##"{"version":"1.0.0","children":[
              {"type":"frame","id":"wx","name":"Mini Program","x":0,"y":0,"width":375,"height":812,
               "layout":"vertical","children":[{"type":"frame","id":"nav","name":"Nav","width":"fill_container","height":44}]}]}"##,
        )
        .expect("fixture")
        .value;
        EditorState::from_document(doc)
    }

    fn first_child_name(state: &EditorState) -> Option<String> {
        state.active_children()[0]
            .children()
            .and_then(|c| c.first())
            .and_then(|c| c.base().name.clone())
    }

    /// Issue #274: an existing phone-sized screen keeps the chrome its
    /// author gave it, whatever tool writes to the document.
    #[test]
    fn writes_never_add_a_status_bar_to_existing_screens() {
        for tool in ["update_node", "save_document", "batch_design"] {
            let mut state = phone_doc();
            let scope = mobile_repair_scope(tool, &state);
            run_write_repairs_after_apply(&mut state, &scope);
            assert_eq!(first_child_name(&state).as_deref(), Some("Nav"), "{tool}");
        }
    }

    #[test]
    fn generation_tools_still_seed_the_screens_they_create() {
        let mut state = EditorState::from_document(
            jian_ops_schema::load_str(r#"{"version":"1.0.0","children":[]}"#)
                .expect("fixture")
                .value,
        );
        let scope = mobile_repair_scope("batch_design", &state);
        *state.active_children_mut() = phone_doc().active_children().to_vec();
        run_write_repairs_after_apply(&mut state, &scope);
        assert_ne!(first_child_name(&state).as_deref(), Some("Nav"));
    }
}

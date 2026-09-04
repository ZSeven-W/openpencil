//! Tests for the unresolved-blocker completion-gate scan — see
//! `loop_blocker_ledger.rs` module doc for scope (structure / empty-shell /
//! nav are blockers; layoutIssues stays advisory and out of scope here).

use super::*;
use jian_ops_schema::PenDocument;
use op_editor_core::pen_node_ext::PenNodeExt;

fn state_from_json(json: &str) -> EditorState {
    let doc: PenDocument = serde_json::from_str(json).expect("valid PenDocument");
    EditorState::from_document(doc)
}

#[test]
fn clean_document_has_no_blockers() {
    let state = state_from_json(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "home", "name": "Home", "width": 390, "height": 844,
              "children": [ { "type": "text", "id": "t1", "content": "Hello" } ] }
        ] }"##,
    );
    let report = detect_blockers(&state);
    assert!(!report.has_blockers(), "{report:?}");
}

#[test]
fn duplicate_top_level_frame_is_a_structure_blocker() {
    // Same test0711-1-m3.op shape `duplicate_root_tests` covers in
    // design_agent_tools.rs: model abandoned the first `Explore` and
    // rebuilt everything in a second one of the same name.
    let state = state_from_json(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "r1", "name": "Explore", "width": 390, "height": 844,
              "children": [ { "type": "frame", "id": "empty", "name": "AppContent",
                               "width": "fill_container", "height": "fit_content" } ] },
            { "type": "frame", "id": "r2", "name": "Explore", "width": 390,
              "height": "fit_content",
              "children": [ { "type": "frame", "id": "rich", "name": "AppContent",
                               "width": "fill_container", "height": "fit_content" } ] }
        ] }"##,
    );
    let report = detect_blockers(&state);
    assert!(report.has_blockers());
    let hit = report
        .blockers
        .iter()
        .find(|b| b.category == "structure")
        .expect("a structure blocker");
    assert!(
        hit.detail.contains("Explore") && hit.detail.contains("r1") && hit.detail.contains("r2")
    );
}

#[test]
fn empty_named_shell_is_an_empty_shell_blocker() {
    let state = state_from_json(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "home", "name": "Home", "width": 390, "height": 844,
              "children": [
                { "type": "frame", "id": "section", "name": "RecentActivity",
                  "width": "fill_container", "height": "fit_content", "children": [] }
              ] }
        ] }"##,
    );
    let report = detect_blockers(&state);
    assert!(report.has_blockers());
    let hit = report
        .blockers
        .iter()
        .find(|b| b.category == "empty-shell")
        .expect("an empty-shell blocker");
    // Must carry a concrete, locatable node id alongside the name — a
    // corrective nudge built from name alone could be ambiguous.
    assert!(hit.detail.contains("RecentActivity"), "{hit:?}");
    assert!(hit.detail.contains("section"), "{hit:?}");
}

#[test]
fn unbound_nav_tab_is_a_nav_blocker() {
    // Mirrors op_orchestrator::nav_issues_tests's
    // TWO_SCREENS_UNBOUND_PROFILE_TAB fixture: two screen-marked frames, the
    // "Profile" tab in Home's bottom nav has no events bound yet.
    let state = state_from_json(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "home", "name": "Home", "screen": "/",
              "width": 390, "height": 844, "layout": "vertical",
              "children": [
                { "type": "frame", "id": "nav", "name": "Bottom Nav", "role": "bottom-tab-bar",
                  "layout": "horizontal", "width": "fill_container",
                  "children": [
                    { "type": "frame", "id": "tab-home", "layout": "vertical",
                      "events": { "onTap": [ { "replace": "\"/\"" } ] },
                      "children": [ { "type": "text", "id": "t1", "content": "Home" } ] },
                    { "type": "frame", "id": "tab-profile", "layout": "vertical",
                      "children": [ { "type": "text", "id": "t2", "content": "Profile" } ] }
                  ] }
              ] },
            { "type": "frame", "id": "profile", "name": "Profile", "screen": "/profile",
              "width": 390, "height": 844 }
        ] }"##,
    );
    let report = detect_blockers(&state);
    assert!(report.has_blockers());
    let hit = report
        .blockers
        .iter()
        .find(|b| b.category == "nav")
        .expect("a nav blocker");
    assert!(hit.detail.contains("tab-profile"), "{hit:?}");
}

#[test]
fn fixing_the_document_makes_the_blocker_disappear() {
    // The "no accumulating ledger" contract: re-running the SAME scan
    // against a document where the issue was fixed must report nothing —
    // there is no stale entry to prune because nothing was ever stored.
    let mut state = state_from_json(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "home", "name": "Home", "width": 390, "height": 844,
              "children": [
                { "type": "frame", "id": "section", "name": "RecentActivity",
                  "width": "fill_container", "height": "fit_content", "children": [] }
              ] }
        ] }"##,
    );
    assert!(detect_blockers(&state).has_blockers());

    // Fill the shell with real content.
    let home = &mut state.active_children_mut()[0];
    let section = &mut home.children_mut().unwrap()[0];
    *section.children_mut().unwrap() = vec![serde_json::from_value(serde_json::json!(
        { "type": "text", "id": "t1", "content": "No recent activity" }
    ))
    .expect("text node")];

    assert!(
        !detect_blockers(&state).has_blockers(),
        "fixed shell must not still be reported"
    );
}

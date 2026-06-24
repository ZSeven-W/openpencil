//! Task F5 — backward-compat regression: appended generation must leave a
//! pre-existing styled node byte-identical.
//!
//! Wired as `#[path = "run_tests_f5.rs"] mod tests_f5;` inside `run.rs`.
//! Stays a child module of `run`, so `use super::*` resolves to `run`.
//!
//! Guards the end-to-end contract delivered by F2-F4: `run_cleanup_passes`
//! in append mode is scoped to the newly inserted roots only and never
//! emits `SetNodeFillHex` / `PatchNodeData` / `UpdateNode` against a
//! pre-existing styled node.

use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use crate::types::{AppendContext, DesignRequest};
use op_editor_core::{EditorCommand, PenNodeExt};

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

/// Find the live (post-remap) id of the first descendant whose `name` field
/// matches `name_needle`.  Returns `None` when not found.
fn find_live_id_by_name(sink: &VecDocSink, name_needle: &str) -> Option<String> {
    fn recurse(nodes: &[jian_ops_schema::node::PenNode], needle: &str) -> Option<String> {
        for n in nodes {
            if n.base().name.as_deref().unwrap_or("") == needle {
                return Some(n.id_str().to_string());
            }
            if let Some(kids) = n.children() {
                if let Some(hit) = recurse(kids, needle) {
                    return Some(hit);
                }
            }
        }
        None
    }
    recurse(sink.state().active_children(), name_needle)
}

// ── Task F5 regression test ───────────────────────────────────────────────────

/// Appended generation must NOT mutate a pre-existing styled node.
///
/// Setup:
///   - Target frame (mobile 390x844) containing one "Status Bar" child:
///     explicit fill #123456, width 390, height 44, plus a text child
///     carrying content "9:41".  These attributes would be candidates for
///     cleanup passes (fill normalisation, size-repair) if cleanup were
///     applied to the wrong scope.
///
/// Run:
///   - Append-mode generation inserts one NEW section ("Content Area").
///
/// Assertions:
///   (a) The pre-existing "Status Bar" node serialises byte-identically
///       before and after the run (fill / width / height / content unchanged).
///   (b) No `SetNodeFillHex`, `PatchNodeData`, or `UpdateNode` command in
///       `VecDocSink.applied` targets the pre-existing node's live id.
///   (c) The new node WAS inserted (descendant count grew), proving the run
///       did real work and did not trivially no-op.
#[test]
fn append_does_not_mutate_preexisting_styled_node() {
    use op_editor_core::walkers::find_node as find_deep;

    // ── seed the target frame with a fully-styled pre-existing child ──────────
    // The "Status Bar" child has:
    //   - explicit fill #123456  (cleanup fill-normalization would overwrite it)
    //   - explicit width 390 / height 44  (size-repair UpdateNode would touch it)
    //   - a text child with content "9:41"  (PatchNodeData content patch target)
    let target_node: jian_ops_schema::node::PenNode = serde_json::from_str(
        r##"{
            "type": "frame",
            "id": "hint-app-root",
            "name": "App Root",
            "x": 0, "y": 0,
            "width": 390, "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [
                {
                    "type": "frame",
                    "id": "hint-status-bar",
                    "name": "Status Bar",
                    "x": 0, "y": 0,
                    "width": 390, "height": 44,
                    "fill": [{ "type": "solid", "color": "#123456" }],
                    "children": [
                        {
                            "type": "text",
                            "id": "hint-clock",
                            "name": "clock",
                            "x": 8, "y": 14,
                            "width": 40, "height": 16,
                            "content": "9:41"
                        }
                    ]
                }
            ]
        }"##,
    )
    .expect("parse target node with pre-existing status bar");

    let mut sink = VecDocSink::new();
    let root_count_before = sink.state().active_children().len();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![target_node],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });

    // Resolve the live (remapped) id of the target frame.
    let live_target_id = sink
        .state()
        .active_children()
        .get(root_count_before)
        .map(|n| n.id_str().to_string())
        .expect("target frame inserted at page root");

    // Resolve the live id of the pre-existing "Status Bar" child.
    let live_keep_id = find_live_id_by_name(&sink, "Status Bar")
        .expect("Status Bar child must be present after InsertSubtree");

    // Snapshot the pre-existing node as a byte string before the run.
    let keep_before: String = {
        let children = sink.state().active_children();
        let node = find_deep(children, &op_editor_core::NodeId::new(live_keep_id.clone()))
            .expect("Status Bar must be findable by live id");
        serde_json::to_string(node).expect("serialize pre-existing node")
    };

    let descendant_count_before = descendant_count(sink.state(), &live_target_id);

    // ── append-generate one new section under the target frame ────────────────
    const PLAN_JSON: &str = r##"{
      "rootFrame": {
        "id": "frame-placeholder",
        "name": "App Root",
        "width": 390, "height": 844,
        "layout": "vertical", "gap": 0,
        "fill": [{ "type": "solid", "color": "#FFFFFF" }]
      },
      "subtasks": [
        { "id": "content", "label": "Content Area",
          "region": { "width": 390, "height": 600 } }
      ]
    }"##;

    // The new section is a plain blue frame — deliberately different from the
    // Status Bar so any accidental bleed from the pre-existing child is visible.
    const NEW_SECTION_JSON: &str = r##"[{
        "type": "frame",
        "id": "new-content-1",
        "name": "Content Area",
        "x": 0, "y": 44,
        "width": 390, "height": 600,
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": "#F0F8FF" }],
        "children": [{
            "type": "text",
            "id": "new-body-text",
            "name": "body",
            "x": 16, "y": 16,
            "width": 358, "height": 24,
            "content": "Hello, world"
        }]
    }]"##;

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(NEW_SECTION_JSON.into()),
    ]);
    let abort = AbortFlag::new();

    let req = DesignRequest {
        prompt: "add a content area section".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: Some(AppendContext {
            target_parent_id: live_target_id.clone(),
            target_width: 390.0,
            existing_section_labels: vec!["Status Bar".into()],
            is_mobile: true,
        }),
        validation_enabled: true,
        visual_ref_enabled: false,
    };

    futures::executor::block_on(Orchestrator::new().run(
        req,
        &mut sink,
        &llm,
        &mut |_| {},
        &abort,
        &stub_providers(),
    ))
    .expect("append run must succeed");

    // ── (a) byte-identical JSON for the pre-existing node ─────────────────────
    let keep_after: String = {
        let children = sink.state().active_children();
        let node = find_deep(children, &op_editor_core::NodeId::new(live_keep_id.clone()))
            .expect("Status Bar must still be findable after append run");
        serde_json::to_string(node).expect("serialize pre-existing node after run")
    };

    assert_eq!(
        keep_before, keep_after,
        "pre-existing styled node (Status Bar) must be byte-identical before vs after \
         append run — fill / width / height / content must not change"
    );

    // ── (b) no mutating command targeted the pre-existing node's live id ──────
    let bad_cmd = sink.applied.iter().find(|c| match c {
        EditorCommand::SetNodeFillHex { node_id, .. } => node_id.as_str() == live_keep_id,
        EditorCommand::PatchNodeData { node_id, .. } => node_id.as_str() == live_keep_id,
        EditorCommand::UpdateNode { node_id, .. } => node_id.as_str() == live_keep_id,
        _ => false,
    });
    assert!(
        bad_cmd.is_none(),
        "no cleanup command (SetNodeFillHex / PatchNodeData / UpdateNode) may target \
         the pre-existing Status Bar node (live id {live_keep_id:?}); \
         found: {bad_cmd:?}"
    );

    // ── (c) the new section WAS inserted (run did real work) ──────────────────
    let descendant_count_after = descendant_count(sink.state(), &live_target_id);
    assert!(
        descendant_count_after > descendant_count_before,
        "append run must insert new content into the target frame: \
         before={descendant_count_before} after={descendant_count_after}"
    );

    // Additionally confirm the live id of the newly inserted "Content Area"
    // exists, so the test fails loudly if the scripted LLM response was ignored.
    let new_section_live_id = find_live_id_by_name(&sink, "Content Area");
    assert!(
        new_section_live_id.is_some(),
        "newly appended 'Content Area' node must exist in the doc tree after the run"
    );
}

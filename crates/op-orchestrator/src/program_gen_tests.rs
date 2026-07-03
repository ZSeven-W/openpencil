//! Tests for the shared program→forest executor (`program_gen.rs`).
//!
//! `parse_program` / `extract_program` / the env-gated protocol-selection
//! functions were retired when script-gen became the default generation
//! protocol for every model (the reduced/minimal retry rungs use flat JSONL
//! instead of the program-DSL text format). `run_program_to_forest` survives
//! as the shared executor `script_gen` runs its recorded `I(...)` program
//! through — these tests exercise it directly.

use super::run_program_to_forest;
use op_editor_core::PenNodeExt;

#[test]
fn run_program_to_forest_nests_cells_under_rows_via_bindings() {
    // The whole point: a cell's content lands UNDER the row, not as a sibling of
    // the table — purely because the parent is a captured binding.
    let program = concat!(
        "sec=I(null, {\"type\":\"frame\",\"name\":\"Sec\",\"layout\":\"vertical\",\"width\":\"fill_container\"})\n",
        "tbl=I(sec, {\"type\":\"frame\",\"name\":\"Table\",\"layout\":\"vertical\",\"width\":\"fill_container\"})\n",
        "r1=I(tbl, {\"type\":\"frame\",\"name\":\"Row\",\"layout\":\"horizontal\"})\n",
        "c1=I(r1, {\"type\":\"frame\",\"name\":\"Cell\"})\n",
        "I(c1, {\"type\":\"text\",\"content\":\"Alice\"})"
    );
    let (nodes, state) = run_program_to_forest(program).expect("program builds a forest");
    assert_eq!(nodes.len(), 1, "exactly one section root");
    assert!(state.is_empty(), "program declared no state");
    let sec = &nodes[0];
    let tbl = &sec.children().expect("sec children")[0];
    let row = &tbl.children().expect("table children")[0];
    let cell = &row.children().expect("row children")[0];
    let content = cell.children().expect("cell children");
    assert_eq!(
        content.len(),
        1,
        "text is nested under the cell, not a sibling"
    );
}

#[test]
fn run_program_to_forest_empty_is_error() {
    assert!(run_program_to_forest("   ").is_err());
}

#[test]
fn image_without_src_and_bad_textgrowth_survive() {
    // Weak-model typos that used to DROP the whole node: an `image` with no
    // `src` (REQUIRED field) and a `text` with `textGrowth:"fit_content"` (not a
    // valid variant). The executor's normalize now recovers both so the avatar
    // and the label survive instead of vanishing (and collapsing their column).
    let program = concat!(
        "sec=I(null, {\"type\":\"frame\",\"name\":\"Sec\",\"layout\":\"vertical\",\"width\":\"fill_container\"})\n",
        "I(sec, {\"type\":\"image\",\"name\":\"Avatar\",\"width\":40,\"height\":40})\n",
        "I(sec, {\"type\":\"text\",\"content\":\"Hi\",\"textGrowth\":\"fit_content\"})"
    );
    let (nodes, _state) = run_program_to_forest(program).expect("program builds a forest");
    let sec = &nodes[0];
    let kids = sec.children().expect("sec children");
    assert_eq!(
        kids.len(),
        2,
        "both the src-less image and the bad-textGrowth text survived"
    );
}

#[test]
fn bare_identifier_sizing_value_survives() {
    // A weak model wrote `"width":fill_container_str` — a bare (unquoted) leaked
    // variable name that fails strict JSON. `parse_json_arg` quotes it and the
    // sizing normalize maps `fill_container_str` → `fill_container`, so the node
    // lands instead of being dropped (measured: it dropped a table column header).
    let program = concat!(
        "sec=I(null, {\"type\":\"frame\",\"name\":\"Sec\",\"layout\":\"vertical\",\"width\":\"fill_container\"})\n",
        "I(sec, {\"type\":\"text\",\"name\":\"Col Service\",\"content\":\"SERVICE\",\"width\":fill_container_str})"
    );
    let (nodes, _state) = run_program_to_forest(program).expect("program builds a forest");
    let sec = &nodes[0];
    let kids = sec.children().expect("sec children");
    assert_eq!(kids.len(), 1, "the bare-identifier-sized text survived");
}

#[test]
fn run_program_to_forest_drains_hoisted_state_off_the_scratch_document() {
    // A program whose `I()` node carries a `state` block must come back with
    // that state DRAINED from the returned schema — and the returned nodes
    // must have their own `state` field stripped (op-mcp's generation-hoist
    // already stripped it before this executor ever saw the forest; this is
    // a regression guard against that hoist landing on the scratch doc and
    // getting silently discarded instead of returned to the caller).
    let program = concat!(
        "I(null, {\"type\":\"frame\",\"name\":\"Sec\",\"width\":\"fill_container\",",
        "\"state\":{\"n\":{\"type\":\"int\",\"default\":0}},",
        "\"children\":[{\"type\":\"text\",\"content\":\"Hi\"}]})"
    );
    let (nodes, state) = run_program_to_forest(program).expect("program builds a forest");
    assert!(
        state.contains_key("n"),
        "hoisted state key must be returned to the caller, got {state:?}"
    );
    assert_eq!(nodes.len(), 1);
    let jian_ops_schema::node::PenNode::Frame(frame) = &nodes[0] else {
        panic!("expected a frame root");
    };
    assert!(
        frame.state.is_none(),
        "returned node must have its state drained, not just the scratch schema"
    );
}

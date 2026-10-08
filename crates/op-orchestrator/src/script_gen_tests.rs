//! Tests for the script-gen subagent parse path (`script_gen.rs`).
//!
//! `script_gen::parse_script` is a thin wrapper: `op_mcp::script_runner`
//! (feature `script`) owns fence-stripping, sandboxing, resource limits, and
//! truncation repair — that surface is covered end-to-end by op-mcp's
//! `script_runner_tests.rs`, including the sandbox-stub / standard-ES /
//! mid-run-salvage cases retired from here in the protocol-collapse (Task 4).
//! What stays here is bridge-specific: does the recorded `batch_design`
//! program actually assemble into a correct `PenNode` section forest via
//! `program_gen::run_program_to_forest`.

use super::parse_script;
use op_editor_core::PenNodeExt;

#[test]
#[ignore = "requires the explicitly retained GLM response; no provider calls"]
fn replay_retained_glm_corrected_program_without_a_model_retry() {
    let source =
        std::path::PathBuf::from(std::env::var("OPENPENCIL_CORRECTION_REPLAY_SOURCE").unwrap());
    let raw = std::fs::read_to_string(&source).unwrap();
    let (nodes, _) = parse_script(&raw).expect("the complete correction is directly runnable");
    assert_eq!(nodes.len(), 1, "exactly the final program's one design");
    let json = serde_json::to_string(&nodes).unwrap();
    for fact in [
        "晴日咖啡",
        "咖啡小聚",
        "2026年10月17日",
        "14:30",
        "拿铁",
        "28元",
        "美式",
        "22元",
        "燕麦拿铁",
        "32元",
        "到店自取，不含配送。",
    ] {
        assert!(json.contains(fact), "missing material: {fact}");
    }
    let mut state = op_editor_core::EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().extend(nodes);
    std::fs::write(
        source.with_file_name("decoded-correction.op"),
        serde_json::to_string_pretty(&state.doc).unwrap(),
    )
    .unwrap();
}

#[test]
fn js_loop_builds_repeated_rows_nested_under_the_table() {
    // The whole point: a JS `for` loop generates N rows; each row's cells nest
    // under the row purely via the binding returned by `I`.
    let script = r#"
        const sec = I(null, {type:"frame", name:"Sec", layout:"vertical", width:"fill_container"});
        const tbl = I(sec, {type:"frame", name:"Table", layout:"vertical", width:"fill_container"});
        const rows = [{name:"Alice"},{name:"Bob"},{name:"Cara"}];
        for (const r of rows) {
            const row = I(tbl, {type:"frame", layout:"horizontal", width:"fill_container"});
            const cell = I(row, {type:"frame", width:"fill_container"});
            I(cell, {type:"text", content:r.name});
        }
    "#;
    let (nodes, _state) = parse_script(script).expect("script builds a forest");
    assert_eq!(nodes.len(), 1, "one section root");
    let sec = &nodes[0];
    let tbl = &sec.children().expect("sec children")[0];
    let rows = tbl.children().expect("table children");
    assert_eq!(rows.len(), 3, "loop produced all 3 rows");
    // each row -> cell -> text
    let cell = &rows[0].children().expect("row children")[0];
    assert_eq!(
        cell.children().expect("cell children").len(),
        1,
        "text nested under the cell via the binding"
    );
}

#[test]
fn empty_script_is_error() {
    assert!(parse_script("   ").is_err());
}

/// A CSS-fluent model writes `justifyContent:"flex_end"` (snake_case CSS, by
/// analogy to our `space_between`). Such cells must survive end-to-end through
/// script-gen → program → forest. Before the executor normalized the underscore
/// flex_* forms, every such cell failed to deserialize and was SILENTLY dropped
/// — a 5-column table lost the right-aligned amount column + the left-aligned
/// header labels, keeping only the `center` ones. This guards the whole path.
#[test]
fn flex_aligned_cells_survive_the_forest() {
    let script = r#"
        const tbl = I(null, {type:"frame", name:"Table", layout:"vertical", width:"fill_container"});
        const data = [{name:"Alice", amount:"$1,240"}, {name:"Bob", amount:"$860"}];
        for (const d of data) {
            const row = I(tbl, {type:"frame", layout:"horizontal", width:"fill_container"});
            const nameCell = I(row, {type:"frame", width:"fill_container", justifyContent:"flex_start"});
            I(nameCell, {type:"text", content:d.name});
            const amtCell = I(row, {type:"frame", width:110, justifyContent:"flex_end", alignItems:"flex_end"});
            I(amtCell, {type:"text", content:d.amount});
        }
    "#;
    let (nodes, _state) = parse_script(script).expect("script builds a forest");
    let json = serde_json::to_string(&nodes).unwrap();
    // The flex_end amount cells must NOT be dropped — both amounts present.
    assert!(
        json.contains("$1,240"),
        "flex_end amount cell must survive: {json}"
    );
    assert!(
        json.contains("$860"),
        "every flex_end amount cell must survive"
    );
    // And each row keeps BOTH cells (name + amount), not just the non-flex one.
    let tbl = &nodes[0];
    let rows = tbl.children().expect("table rows");
    assert_eq!(rows.len(), 2, "both rows present");
    for row in rows {
        assert_eq!(
            row.children().map(|c| c.len()).unwrap_or(0),
            2,
            "each row keeps the flex_start name cell AND the flex_end amount cell"
        );
    }
}

/// Truncation repair (`op_mcp::script_runner::repair_truncated_script`) is
/// exercised end-to-end through `parse_script`: a script cut mid-token after
/// several complete statements must still build a forest from the surviving
/// prefix instead of losing the whole section to a trailing SyntaxError (the
/// JS parser fails before ANY `I(...)` call runs, so without repair the
/// salvage-on-throw path in op-mcp's `eval_to_program` has nothing recorded
/// to save).
#[test]
fn truncated_script_repair_still_builds_a_forest_via_parse_script() {
    let cut = r#"
        const sec = I(null, {type:"frame", name:"Sec", layout:"vertical", width:"fill_container"});
        const a = I(sec, {type:"text", content:"kept-1"});
        const b = I(sec, {type:"text", content:"kept-2"});
        I(sec, {type:"text", content:"dangl
    "#;
    let (nodes, _state) =
        parse_script(cut).expect("truncation repair must salvage a runnable forest");
    let sec = &nodes[0];
    assert_eq!(
        sec.children().map(|c| c.len()).unwrap_or(0),
        2,
        "the two complete statements before the truncation must survive"
    );
}

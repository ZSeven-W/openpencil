//! Tests for the program-DSL generation path (`program_gen.rs`).

use super::{extract_program, parse_program};
use op_editor_core::PenNodeExt;

#[test]
fn parse_program_nests_cells_under_rows_via_bindings() {
    // The whole point: a cell's content lands UNDER the row, not as a sibling of
    // the table — purely because the parent is a captured binding.
    let program = concat!(
        "sec=I(null, {\"type\":\"frame\",\"name\":\"Sec\",\"layout\":\"vertical\",\"width\":\"fill_container\"})\n",
        "tbl=I(sec, {\"type\":\"frame\",\"name\":\"Table\",\"layout\":\"vertical\",\"width\":\"fill_container\"})\n",
        "r1=I(tbl, {\"type\":\"frame\",\"name\":\"Row\",\"layout\":\"horizontal\"})\n",
        "c1=I(r1, {\"type\":\"frame\",\"name\":\"Cell\"})\n",
        "I(c1, {\"type\":\"text\",\"content\":\"Alice\"})"
    );
    let nodes = parse_program(program).expect("program builds a forest");
    assert_eq!(nodes.len(), 1, "exactly one section root");
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
fn extract_program_strips_markdown_fences() {
    let fenced = "```js\nsec=I(null, {\"type\":\"frame\"})\n```";
    assert_eq!(extract_program(fenced), "sec=I(null, {\"type\":\"frame\"})");
    let plain = "sec=I(null, {\"type\":\"frame\"})";
    assert_eq!(extract_program(plain), plain);
}

#[test]
fn parse_program_empty_is_error() {
    assert!(parse_program("   ").is_err());
}

#[test]
fn gate_is_off_by_default() {
    // No env set in the test process → off (the production default).
    assert!(
        !super::program_gen_enabled_for_model("glm-5.2")
            || std::env::var("OPENPENCIL_PROGRAM_GEN").is_ok()
    );
}

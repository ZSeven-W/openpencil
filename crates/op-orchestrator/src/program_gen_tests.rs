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
fn gate_defaults_on_for_weak_models_off_for_strong() {
    // Env override decides everything when set, so this invariant only holds on
    // the no-override path.
    if std::env::var("OPENPENCIL_PROGRAM_GEN").is_ok()
        || std::env::var("OPENPENCIL_SCRIPT_GEN").is_ok()
        || std::env::var("OPENPENCIL_MANIFEST").is_ok()
    {
        return;
    }
    // Open / Chinese reasoning models default to program-DSL: flat JSONL is
    // fragile for them, and executable JS is all-or-nothing on truncation.
    for m in [
        "glm-5.2",
        "minimax-m3",
        "deepseek-v4-pro",
        "qwen-max",
        "MiniMax-M3",
    ] {
        assert!(
            super::program_gen_enabled_for_model(m),
            "{m} should default to program-DSL"
        );
    }
    // Claude / GPT / Gemini / o-series emit flat JSONL natively → program OFF.
    for m in ["claude-opus-4-8", "gpt-4o", "gemini-3-pro", "o3-mini"] {
        assert!(
            !super::program_gen_enabled_for_model(m),
            "{m} should keep flat-JSONL by default"
        );
    }
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
    let nodes = parse_program(program).expect("program builds a forest");
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
    let nodes = parse_program(program).expect("program builds a forest");
    let sec = &nodes[0];
    let kids = sec.children().expect("sec children");
    assert_eq!(kids.len(), 1, "the bare-identifier-sized text survived");
}

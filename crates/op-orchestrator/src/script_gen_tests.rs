//! Tests for the JS-script generation path (`script_gen.rs`).

use super::parse_script;
use op_editor_core::PenNodeExt;

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
    let nodes = parse_script(script).expect("script builds a forest");
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
fn script_gate_is_opt_in_only() {
    // Script-gen (executable JS) is now env opt-in — OFF by default for EVERY
    // model (program-DSL is the weak-model default). The env override decides
    // when set, so this invariant only holds on the no-override path.
    if std::env::var("OPENPENCIL_SCRIPT_GEN").is_ok() {
        return;
    }
    for m in [
        "glm-5.2",
        "minimax-m3",
        "deepseek-v4-pro",
        "qwen-max",
        "MiniMax-M3",
        "claude-opus-4-8",
        "gpt-4o",
        "gemini-3-pro",
        "o3-mini",
    ] {
        assert!(
            !super::script_gen_enabled_for_model(m),
            "{m} should not use script-gen unless OPENPENCIL_SCRIPT_GEN is set"
        );
    }
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
    let nodes = parse_script(script).expect("script builds a forest");
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

/// A Pencil-trained model (Pencil's own free backend is pencil-minimax-m3)
/// habitually calls `console.log` and Pencil's other batch_design ops
/// (`C/U/D/M/R/G`). The sandbox stubs them as no-ops so a stray call can't
/// `ReferenceError` and abort the whole section — the section's `I(...)` nodes
/// must still come through. (Pre-fix, each of these aborted the entire script.)
#[test]
fn pencil_ops_and_console_do_not_abort_the_script() {
    let base = r#"const s = I(null, {type:"frame", name:"S", layout:"vertical", width:"fill_container"});"#;
    let cases: &[(&str, String)] = &[
        (
            "console.log",
            format!("console.log('building section');\n{base}"),
        ),
        (
            "console mid-script",
            format!("{base}\nconsole.warn('done');"),
        ),
        (
            "Pencil G() image-gen",
            format!("{base}\nG(s, 'ai', 'a hero photo');"),
        ),
        (
            "Pencil C() copy",
            format!("{base}\nconst c = C(s, s, {{}});"),
        ),
        (
            "Pencil U/D/M/R",
            format!("{base}\nU('s',{{}}); D('x'); M('x','s',0); R('p',{{}});"),
        ),
    ];
    for (label, script) in cases {
        let nodes = parse_script(script)
            .unwrap_or_else(|e| panic!("{label} must not abort the script: {e}"));
        assert_eq!(nodes.len(), 1, "{label}: the section root must survive");
    }
}

/// Real JS the models DO write must keep working (full ES via QuickJS).
#[test]
fn standard_es_constructs_work() {
    let base = r#"const s = I(null, {type:"frame", name:"S", layout:"vertical", width:"fill_container"});"#;
    for (label, extra) in [
        ("Math", "const n = Math.round(3.7);"),
        ("Date", "const d = new Date(2024,0,1);"),
        ("template literal", "const t = `row ${1+1}`;"),
        (
            "map",
            "[1,2,3].map(x => I(s, {type:'text', content:String(x)}));",
        ),
    ] {
        let script = format!("{base}\n{extra}");
        assert!(
            parse_script(&script).is_ok(),
            "{label} is standard ES and must run"
        );
    }
}

/// Partial-success recovery: a throw PART-WAY through the script keeps every
/// node recorded before it. A typo'd binding reference (genuine ReferenceError
/// we can't stub) on a late line must not erase the good nodes that preceded it.
#[test]
fn throw_midway_salvages_nodes_recorded_before_it() {
    let script = r#"
        const s = I(null, {type:"frame", name:"S", layout:"vertical", width:"fill_container"});
        const a = I(s, {type:"text", content:"kept-1"});
        const b = I(s, {type:"text", content:"kept-2"});
        I(typoBindingNeverDefined, {type:"text", content:"lost"});
        I(s, {type:"text", content:"after-throw"});
    "#;
    let nodes = parse_script(script).expect("partial program must be salvaged, not discarded");
    let s = &nodes[0];
    // The two texts recorded before the throw survive; the post-throw line never
    // ran. (Section root + 2 kept children.)
    assert_eq!(
        s.children().map(|c| c.len()).unwrap_or(0),
        2,
        "the two nodes built before the throw must be kept"
    );
}

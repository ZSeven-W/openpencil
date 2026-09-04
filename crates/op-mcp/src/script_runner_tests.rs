//! Sandbox, transactional recovery, and resource-limit tests for the shared script runner.

use super::*;

#[test]
fn loop_records_one_line_per_iteration() {
    let program = run_script_to_program(
        r#"const row = I(null, {type: "frame", name: "Row"});
for (const label of ["A", "B", "C"]) { I(row, {type: "text", content: label}); }"#,
    )
    .expect("script runs");
    let lines: Vec<&str> = program.lines().collect();
    assert_eq!(lines.len(), 4);
    assert!(lines[0].starts_with("b0=I(null, "));
    assert!(lines[1].starts_with("b1=I(b0, "));
    assert!(lines[3].contains(r#""content":"C""#));
}

#[test]
fn text_insert_defaults_missing_typography_without_overwriting_explicit_values() {
    let program = run_script_to_program(
        r#"const root = I(null, {type: "frame", name: "Root"});
I(root, {type: "text", content: "Default"});
I(root, {type: "text", content: "Brand", fontFamily: "IBM Plex Mono", fontSize: 19, lineHeight: 1.1});"#,
    )
    .expect("text inserts run");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 3, "{program}");
    assert!(
        !lines[0].contains("fontFamily"),
        "non-text remains unchanged: {program}"
    );
    assert!(
        lines[1].contains(r#""fontFamily":"Inter""#),
        "missing font gets the deterministic default: {program}"
    );
    assert!(lines[1].contains(r#""fontSize":16"#), "{program}");
    assert!(lines[1].contains(r#""lineHeight":1.5"#), "{program}");
    assert!(
        lines[2].contains(r#""fontFamily":"IBM Plex Mono""#),
        "explicit font must be preserved: {program}"
    );
    assert!(lines[2].contains(r#""fontSize":19"#), "{program}");
    assert!(lines[2].contains(r#""lineHeight":1.1"#), "{program}");
    assert!(!lines[2].contains(r#""fontFamily":"Inter""#), "{program}");
}

#[test]
fn cjk_text_raises_only_an_explicit_low_line_height() {
    let program = run_script_to_program(
        r#"I(null, {type: "text", content: "中文标题", fontFamily: "Inter", fontSize: 24, lineHeight: 1.2});
I(null, {type: "text", content: "日本語", fontFamily: "Inter", fontSize: 20, lineHeight: 1.3});
I(null, {type: "text", content: "English", fontFamily: "Inter", fontSize: 18, lineHeight: 1.1});"#,
    )
    .expect("CJK typography normalization runs");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 3, "{program}");
    assert!(lines[0].contains(r#""fontSize":24"#), "{program}");
    assert!(lines[0].contains(r#""lineHeight":1.5"#), "{program}");
    assert!(lines[1].contains(r#""lineHeight":1.3"#), "{program}");
    assert!(lines[2].contains(r#""fontSize":18"#), "{program}");
    assert!(lines[2].contains(r#""lineHeight":1.1"#), "{program}");
}

#[test]
fn icon_font_insert_normalizes_only_the_measured_exact_aliases() {
    let program = run_script_to_program(
        r##"I(null, {type: "icon_font", name: "Search", iconFontFamily: "lucide", iconFontName: "magnifying-glass", width: 20, height: 22, fill: [{type: "solid", color: "#123456"}]});
I(null, {type: "icon_font", iconFontName: "snow"});
I(null, {type: "icon_font", iconFontName: "drop"});
I(null, {type: "icon_font", iconFontName: "cup"});
I(null, {type: "icon_font", iconFontName: "table-lamp"});
I(null, {type: "icon_font", iconFontName: "search"});
I(null, {type: "icon_font", iconFontName: "Snow"});
I(null, {type: "icon_font", iconFontName: "drop "});"##,
    )
    .expect("icon aliases normalize");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 8, "{program}");
    for (line, expected) in
        lines[..5]
            .iter()
            .zip(["search", "snowflake", "droplet", "coffee", "lamp-desk"])
    {
        assert!(
            line.contains(&format!(r#""iconFontName":"{expected}""#)),
            "{line}"
        );
    }
    assert!(lines[5].contains(r#""iconFontName":"search""#), "{program}");
    assert!(lines[6].contains(r#""iconFontName":"Snow""#), "{program}");
    assert!(lines[7].contains(r#""iconFontName":"drop ""#), "{program}");
    assert!(lines[0].contains(r#""name":"Search""#), "{program}");
    assert!(
        lines[0].contains(r#""iconFontFamily":"lucide""#),
        "{program}"
    );
    assert!(lines[0].contains(r#""width":20"#), "{program}");
    assert!(lines[0].contains(r#""height":22"#), "{program}");
    assert!(lines[0].contains(r##""color":"#123456""##), "{program}");
}

#[test]
fn frame_center_layout_normalizes_only_the_exact_compatibility_shape() {
    let program = run_script_to_program(
        r#"I(null, {type: "frame", name: "Default centered", layout: "center"});
I(null, {type: "frame", name: "Explicit alignment", layout: "center", alignItems: "end", justifyContent: "space_between"});
I(null, {type: "rectangle", name: "Other type", layout: "center"});
I(null, {type: "frame", name: "Other case", layout: "Center"});
I(null, {type: "frame", name: "Padded value", layout: " center "});"#,
    )
    .expect("exact frame center layout normalizes");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 5, "{program}");
    assert!(lines[0].contains(r#""layout":"vertical""#), "{program}");
    assert!(lines[0].contains(r#""alignItems":"center""#), "{program}");
    assert!(
        lines[0].contains(r#""justifyContent":"center""#),
        "{program}"
    );
    assert!(lines[1].contains(r#""layout":"vertical""#), "{program}");
    assert!(lines[1].contains(r#""alignItems":"end""#), "{program}");
    assert!(
        lines[1].contains(r#""justifyContent":"space_between""#),
        "{program}"
    );
    assert!(lines[2].contains(r#""layout":"center""#), "{program}");
    assert!(!lines[2].contains("alignItems"), "{program}");
    assert!(lines[3].contains(r#""layout":"Center""#), "{program}");
    assert!(!lines[3].contains("alignItems"), "{program}");
    assert!(lines[4].contains(r#""layout":" center ""#), "{program}");
    assert!(!lines[4].contains("alignItems"), "{program}");
}

#[test]
fn text_insert_normalizes_numeric_height_unless_growth_is_explicitly_fixed() {
    let program = run_script_to_program(
        r#"I(null, {type: "text", content: "Auto", height: 40});
I(null, {type: "text", content: "Fixed", height: 40, textGrowth: "fixed-width-height"});
I(null, {type: "text", content: "Already safe", height: "fit_content"});"#,
    )
    .expect("text height normalization runs");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 3, "{program}");
    assert!(lines[0].contains(r#""height":"fit_content""#), "{program}");
    assert!(lines[1].contains(r#""height":40"#), "{program}");
    assert!(
        lines[1].contains(r#""textGrowth":"fixed-width-height""#),
        "{program}"
    );
    assert!(lines[2].contains(r#""height":"fit_content""#), "{program}");
}

#[test]
fn insert_repairs_only_low_contrast_text_and_icons_on_known_solid_backgrounds() {
    let program = run_script_to_program(
        r##"const light = I(null, {type: "frame", name: "Light", fill: [{type: "solid", color: "#FFFFFF"}]});
I(light, {type: "text", content: "Faint", fill: [{type: "solid", color: "#F5F5F5"}]});
const dark = I(null, {type: "frame", name: "Dark", fill: [{type: "solid", color: "#17191D"}]});
I(dark, {type: "icon_font", name: "Faint icon", fill: [{type: "solid", color: "#303238"}]});"##,
    )
    .expect("low-contrast foregrounds are normalized");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 4, "{program}");
    assert!(lines[1].contains(r##""color":"#17191D""##), "{program}");
    assert!(lines[3].contains(r##""color":"#FAF8F3""##), "{program}");
    assert!(!lines[1].contains("#F5F5F5"), "{program}");
    assert!(!lines[3].contains("#303238"), "{program}");
}

#[test]
fn insert_preserves_qualified_foregrounds_and_non_foreground_nodes() {
    let program = run_script_to_program(
        r##"const root = I(null, {type: "frame", fill: [{type: "solid", color: "#FFFFFF"}]});
I(root, {type: "text", content: "Readable", fill: [{type: "solid", color: "#555555"}]});
I(root, {type: "icon_font", name: "Readable icon", fill: [{type: "solid", color: "#777777"}]});
I(root, {type: "rectangle", name: "Quiet decoration", fill: [{type: "solid", color: "#F5F5F5"}]});"##,
    )
    .expect("already-safe and non-foreground fills remain unchanged");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 4, "{program}");
    assert!(lines[1].contains(r##""color":"#555555""##), "{program}");
    assert!(lines[2].contains(r##""color":"#777777""##), "{program}");
    assert!(lines[3].contains(r##""color":"#F5F5F5""##), "{program}");
}

#[test]
fn insert_preserves_foregrounds_when_background_or_fill_is_not_known_opaque_solid() {
    let program = run_script_to_program(
        r##"const unknown = I(null, {type: "frame", name: "No background"});
I(unknown, {type: "text", content: "Unknown bg", fill: [{type: "solid", color: "#F5F5F5"}]});
const gradient = I(null, {type: "frame", fill: [{type: "linear_gradient", stops: [{color: "#FFFFFF", offset: 0}, {color: "#000000", offset: 1}]}]});
I(gradient, {type: "text", content: "Gradient bg", fill: [{type: "solid", color: "#F5F5F5"}]});
const light = I(null, {type: "frame", fill: [{type: "solid", color: "#FFFFFF"}]});
I(light, {type: "text", content: "Gradient fg", fill: [{type: "linear_gradient", stops: [{color: "#FFFFFF", offset: 0}, {color: "#EEEEEE", offset: 1}]}]});
I(light, {type: "text", content: "Alpha hex", fill: [{type: "solid", color: "#FFFFFF80"}]});
I(light, {type: "icon_font", name: "Paint opacity", fill: [{type: "solid", color: "#FFFFFF", opacity: 0.5}]});"##,
    )
    .expect("unknown and transparent colors pass through");

    assert_eq!(program.lines().count(), 8, "{program}");
    assert_eq!(program.matches("#F5F5F5").count(), 2, "{program}");
    assert!(program.contains("linear_gradient"), "{program}");
    assert!(program.contains("#FFFFFF80"), "{program}");
    assert!(program.contains(r#""opacity":0.5"#), "{program}");
}

#[test]
fn insert_inherits_known_background_through_unfilled_bindings() {
    let program = run_script_to_program(
        r##"const root = I(null, {type: "frame", fill: [{type: "solid", color: "#FFFFFF"}]});
const section = I(root, {type: "frame", name: "Transparent section"});
const row = I(section, {type: "frame", name: "Row", layout: "horizontal"});
I(row, {type: "text", content: "Inherited", fill: [{type: "solid", color: "#FAFAFA"}]});"##,
    )
    .expect("background metadata follows the binding chain");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 4, "{program}");
    assert!(lines[3].contains(r##""color":"#17191D""##), "{program}");
    assert!(!lines[3].contains("#FAFAFA"), "{program}");
}

#[test]
fn thin_fill_width_divider_is_promoted_out_of_horizontal_parent() {
    let program = run_script_to_program(
        r#"const root = I(null, {type: "frame", name: "Page", layout: "vertical"});
const header = I(root, {type: "frame", name: "Header", layout: "horizontal"});
I(header, {type: "text", name: "Label", content: "Store"});
I(header, {type: "rectangle", name: "HeaderDivider", width: "fill_container", height: 1});
I(header, {type: "rectangle", name: "Tall Divider", width: "fill_container", height: 3});
I(header, {type: "rectangle", name: "Rule", width: 120, height: 1});"#,
    )
    .expect("only the provable horizontal-row divider is promoted");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 6, "{program}");
    assert!(lines[3].starts_with("b3=I(b0, "), "{program}");
    assert!(lines[4].starts_with("b4=I(b1, "), "{program}");
    assert!(lines[5].starts_with("b5=I(b1, "), "{program}");
}

#[test]
fn raw_newline_inside_a_quoted_text_value_is_escaped_and_retried() {
    let program =
        run_script_to_program("I(null, {type: \"text\", content: \"first line\nsecond line\"});")
            .expect("outer template newline is repaired");

    assert!(
        program.contains(r#""content":"first line\nsecond line""#),
        "{program}"
    );
}

#[test]
fn kit_stub_records_a_k_operation_and_returns_its_binding() {
    let program = run_script_to_program(
        r#"const root = I(null, {type: "frame", name: "Root"});
const button = K("shadcn/btn-primary", root, {descendants: {"shadcn-btn-primary-label": {content: "Book now"}}});
I(button, {type: "text", content: "Nested"});"#,
    )
    .expect("script runs");
    let lines: Vec<&str> = program.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("b0=I(null, "));
    assert_eq!(
        lines[1],
        r#"b1=K("shadcn/btn-primary", b0, {"descendants":{"shadcn-btn-primary-label":{"content":"Book now"}}})"#
    );
    assert!(lines[2].starts_with("b2=I(b1, "));
    assert!(lines[2].contains(r#""content":"Nested""#));
}

#[test]
fn fence_wrapped_script_is_unwrapped() {
    let program =
        run_script_to_program("```js\nI(null, {type: \"frame\"});\n```").expect("fenced ok");
    assert_eq!(program.lines().count(), 1);
}

#[test]
fn mid_run_throw_rejects_the_recorded_prefix() {
    let error = run_script_to_program(
        r#"I(null, {type: "frame"});
I(b0_missing_binding_reference.oops, {});"#,
    )
    .expect_err("a runtime throw must discard the recorded prefix")
    .to_string();
    assert!(error.contains("is not defined"), "{error}");
}

#[test]
fn mutating_an_insert_binding_rejects_the_recorded_prefix() {
    let error = run_script_to_program(
        r#"const card = I(null, {type: "frame", name: "Product"});
card.x = undefined;
I(card, {type: "text", content: "Must not be partially applied"});"#,
    )
    .expect_err("I() returns an opaque string binding, not a mutable node")
    .to_string();
    assert!(error.contains("not an object"), "{error}");
}

#[test]
fn reasoning_block_before_fenced_script_is_stripped() {
    // A reasoning model (MiniMax-M3 rides Adaptive) emits <think>…</think>
    // full of draft JS, then the real fenced script. Feeding the think block
    // to QuickJS was a guaranteed syntax error that dropped the model onto the
    // fragile flat-JSONL rung (measured: a travel page collapsed to 44 flat
    // siblings). The think body must be stripped, the fenced script harvested.
    let program = run_script_to_program(
        "<think>\nLet me plan. Maybe I(null,{type:\"frame\"}) then rows...\nActually reconsider.\n</think>\n\nHere is the design:\n\n```js\nconst root = I(null, {type: \"frame\", name: \"Root\"});\nI(root, {type: \"text\", content: \"Hi\"});\n```",
    )
    .expect("think + fence must parse");
    assert_eq!(program.lines().count(), 2);
    assert!(program.lines().next().unwrap().starts_with("b0=I(null, "));
}

#[test]
fn prose_preamble_before_fence_is_ignored() {
    // Models add a sentence before the fence; a start-anchored strip passed
    // the prose to the runtime as source.
    let program = run_script_to_program(
        "Sure — here's the section:\n```javascript\nI(null, {type: \"frame\"});\n```",
    )
    .expect("prose + fence must parse");
    assert_eq!(program.lines().count(), 1);
}

#[test]
fn reasoning_block_before_bare_script_is_stripped() {
    // Think block, then a bare (unfenced) program.
    let program = run_script_to_program("<think>draft: I(x)</think>\nI(null, {type: \"frame\"});")
        .expect("think + bare script must parse");
    assert_eq!(program.lines().count(), 1);
}

#[test]
fn empty_script_is_an_error() {
    assert!(run_script_to_program("   \n").is_err());
    assert!(run_script_to_program("```\n```").is_err());
    // A think block with no script after it is empty, not a syntax error.
    assert!(run_script_to_program("<think>only reasoning, no answer</think>").is_err());
}

#[test]
fn script_with_no_inserts_is_an_error() {
    assert!(run_script_to_program("const x = 1 + 1;").is_err());
}

#[test]
fn oversized_source_is_rejected_before_eval() {
    let big = format!("// {}\nI(null, {{}});", "x".repeat(MAX_SCRIPT_BYTES));
    let err = run_script_to_program(&big).unwrap_err().to_string();
    assert!(err.contains("script too large"), "got: {err}");
}

#[test]
fn infinite_loop_is_interrupted_not_hung() {
    let start = std::time::Instant::now();
    let err = run_script_to_program("while (true) {}")
        .unwrap_err()
        .to_string();
    assert!(
        start.elapsed() < std::time::Duration::from_secs(10),
        "interrupt fired"
    );
    assert!(!err.is_empty());
}

#[test]
fn memory_bomb_is_rejected() {
    let err = run_script_to_program("let s = 'x'; while (true) { s += s; }")
        .unwrap_err()
        .to_string();
    assert!(!err.is_empty());
}

#[test]
fn recorded_lines_are_capped() {
    let program = run_script_to_program(
        "for (let i = 0; i < 10000; i++) { I(null, {type: \"frame\", n: i}); }",
    )
    .expect("capped run still returns the recorded prefix");
    assert_eq!(program.lines().count(), MAX_RECORDED_LINES);
}

#[test]
fn recorded_bytes_are_capped() {
    // Few lines, huge content: 8 iterations each embed a ~2 MiB string in
    // the recorded JSON, well under MAX_RECORDED_LINES (4096) but well over
    // MAX_RECORDED_BYTES (8 MiB) once a handful accumulate. The byte cap —
    // not the line-count cap — must be what stops accumulation here.
    let program = run_script_to_program(
        r#"for (let i = 0; i < 8; i++) {
  const big = "x".repeat(2 * 1024 * 1024);
  I(null, {type: "text", content: big});
}"#,
    )
    .expect("byte-capped run still returns the recorded prefix");
    assert!(
        program.lines().count() < 8,
        "byte cap must stop accumulation before all 8 iterations record"
    );
    // The cap is HARD: the whole line must fit before it is pushed, so the
    // recorded program can never exceed the advertised limit (newline joins
    // between accepted lines are the only unaccounted bytes).
    assert!(
        program.len() <= MAX_RECORDED_BYTES + program.lines().count(),
        "program bytes {} exceeded the advertised byte cap",
        program.len()
    );
}

#[test]
fn oversized_first_line_is_refused_and_recording_latches() {
    // A single line that alone exceeds MAX_RECORDED_BYTES is refused
    // outright, and the refusal LATCHES: the later small I(...) call must
    // not sneak into the remaining budget (the program stays a clean
    // prefix — here an empty one, which surfaces as the typed
    // no-operations error instead of a partially-holed program).
    let err = run_script_to_program(
        r#"const big = "x".repeat(9 * 1024 * 1024);
I(null, {type: "text", content: big});
I(null, {type: "frame"});"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        err.contains("no I(...), K(...), or U(...) operations"),
        "expected the empty-program error, got: {err}"
    );
}

const COMPLETE_PREFIX: &str = r#"const root = I(null, {type: "frame", name: "Card"});
I(root, {type: "text", content: "Title"});
I(root, {type: "text", content: "Body"});"#;

#[test]
fn truncated_mid_string_salvages_complete_prefix() {
    let cut = format!("{COMPLETE_PREFIX}\nI(root, {{type: \"text\", content: \"dangl");
    let program = run_script_to_program(&cut).expect("repair salvages prefix");
    assert_eq!(program.lines().count(), 3);
}

#[test]
fn truncated_mid_object_salvages_complete_prefix() {
    let cut = format!("{COMPLETE_PREFIX}\nI(root, {{type: \"frame\", fill: [{{color:");
    let program = run_script_to_program(&cut).expect("repair salvages prefix");
    assert_eq!(program.lines().count(), 3);
}

#[test]
fn truncated_mid_loop_body_closes_and_runs() {
    // Loop braces are open at the cut; repair appends closers so the
    // complete statements before the loop still run.
    let cut = r#"const root = I(null, {type: "frame"});
for (const x of ["a", "b"]) {
  I(root, {type: "text", content: x});
  I(root, {type: "text", cont"#;
    let program = run_script_to_program(cut).expect("repair runs prefix");
    assert!(program.lines().count() >= 1, "at least the root survives");
}

#[test]
fn truncated_mid_token_salvages_prefix() {
    let cut = format!("{COMPLETE_PREFIX}\nconst extra = I(roo");
    let program = run_script_to_program(&cut).expect("repair salvages prefix");
    assert_eq!(program.lines().count(), 3);
}

#[test]
fn unrepairable_garbage_still_errors() {
    assert!(run_script_to_program("{{{{ not js at all").is_err());
}

#[test]
fn update_records_an_operations_compatible_program_line() {
    let program = run_script_to_program(
        r#"U("node-42", {x: 10, name: "Updated", width: "fill_container"});"#,
    )
    .expect("U() is a supported repair script operation");

    assert_eq!(
        program,
        r#"U("node-42", {"x":10,"name":"Updated","width":"fill_container"})"#
    );
}

#[test]
fn update_script_reaches_the_existing_batch_update_executor() {
    use crate::{EditorCommand, McpTool, ToolOutcome};
    use std::collections::BTreeMap;

    let state = crate::test_fixtures::sample();
    let tool = crate::batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "script".to_string(),
        r#"U("n11", {x: 80, name: "Updated title"});"#.to_string(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::UpdateNode {
                node_id, x, name, ..
            },
        ) => {
            assert_eq!(node_id.as_str(), "n11");
            assert_eq!(x, Some(80));
            assert_eq!(name.as_deref(), Some("Updated title"));
        }
        other => panic!("expected the existing U() executor command, got {other:?}"),
    }
}

#[test]
fn update_accepts_an_insert_binding_and_returns_the_target() {
    let program = run_script_to_program(
        r#"const root = I(null, {type: "frame", name: "Root"});
const sameRoot = U(root, {x: 10});
I(sameRoot, {type: "text", content: "Child"});"#,
    )
    .expect("U() records and remains chainable");
    let lines: Vec<&str> = program.lines().collect();

    assert_eq!(lines.len(), 3, "{program}");
    assert!(lines[0].starts_with("b0=I(null, "), "{program}");
    assert_eq!(lines[1], r#"U("b0", {"x":10})"#);
    assert!(lines[2].starts_with("b1=I(b0, "), "{program}");
}

#[test]
fn still_unsupported_mutations_are_rejected_instead_of_silently_dropped() {
    let error = run_script_to_program(
        r#"const root = I(null, {type: "frame", name: "Root"});
D(root);"#,
    )
    .expect_err("D() must not look successful while doing nothing")
    .to_string();
    assert!(error.contains("OP_SCRIPT_MODE_UNSUPPORTED"), "{error}");
    assert!(error.contains("direct QuickJS"), "{error}");
    assert!(error.contains("I(), K(), and authorized U()"), "{error}");
}

#[test]
fn console_remains_a_noop_while_insert_records() {
    let program = run_script_to_program(
        r#"console.log("building card");
console.warn("heads up");
I(null, {type: "frame", name: "Root"});"#,
    )
    .expect("console does not affect the design program");
    assert_eq!(program.lines().count(), 1);
    assert!(program.contains(r#""name":"Root""#));
}

#[test]
fn balances_glm_missing_outer_brace() {
    let broken = r##"const btnProfile = I(null, {type:"frame", name:"Profile Button", width:44, height:44, cornerRadius:12, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#EAD8C8"}]});"##;

    let repaired = balance_brackets(broken);

    assert!(repaired.ends_with(r##""#EAD8C8"}]}});"##));
    assert_eq!(repaired.matches('{').count(), repaired.matches('}').count());
    let program = eval_to_program(&repaired).expect("balanced GLM script evals");
    assert!(program.contains(r#""name":"Profile Button""#));
}

#[test]
fn wellformed_script_unchanged() {
    let script = r##"const root = I(null, {type:"frame", stroke:{thickness:1, fill:[{type:"solid", color:"#EAD8C8"}]}});
I(root, {type:"text", content:"Ready"});"##;

    assert_eq!(balance_brackets(script), script);
}

#[test]
fn brackets_inside_strings_not_counted() {
    let script = r##"I(null, {type:"text", content:"a) b}", name:'keep ] here', note:`literal { bracket`});"##;

    assert_eq!(balance_brackets(script), script);
}

#[test]
fn glm_missing_outer_brace_repairs_on_eval_failure() {
    let broken = r##"const btnProfile = I(null, {type:"frame", name:"Profile Button", width:44, height:44, cornerRadius:12, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#EAD8C8"}]});"##;

    let program = run_script_to_program(broken).expect("runner retries with bracket repair");

    let lines: Vec<&str> = program.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains(r#""name":"Profile Button""#));
}

/// gemini-3.6-flash measured shape (2026-08-05): three `justify.content:`
/// keys in an otherwise valid slide script. A bare dotted key is a
/// SyntaxError, so the whole board used to be lost to a retry.
#[test]
fn dotted_property_keys_repair_on_eval_failure() {
    let broken = r##"const slide = I(null, {type:"frame", name:"01 Cover", width:1920, height:1080, layout:"vertical", justify.content:"space_between", align.items:"start"});
I(slide, {type:"text", content:"Q3 Review", font.size:104});"##;

    let program = run_script_to_program(broken).expect("runner retries with dotted-key repair");

    let lines: Vec<&str> = program.lines().collect();
    assert_eq!(lines.len(), 2, "both inserts survive: {program}");
    assert!(
        lines[0].contains(r#""justifyContent":"space_between""#),
        "{program}"
    );
    assert!(lines[0].contains(r#""alignItems":"start""#), "{program}");
    assert!(lines[1].contains(r#""fontSize":104"#), "{program}");
}

/// The repair must compose with the ladder below it — a script can be both
/// mis-keyed AND missing its closing brace, which is exactly what a model
/// that got the property names wrong tends to also get wrong.
#[test]
fn dotted_keys_and_a_missing_brace_repair_together() {
    let broken = r##"const card = I(null, {type:"frame", name:"KPI", corner.radius:24, stroke:{thickness:1, fill:[{type:"solid", color:"#EAD8C8"}]});"##;

    let program = run_script_to_program(broken).expect("both repairs apply");

    assert_eq!(program.lines().count(), 1, "{program}");
    assert!(program.contains(r#""cornerRadius":24"#), "{program}");
}

/// The duplicate-echo needle is a byte slice of the first declaration line;
/// byte 60 landing inside a multi-byte char (CJK node names are routine)
/// must clamp to a boundary instead of panicking. Regression: glm-5.2
/// emitted exactly this shape and the whole generation process aborted.
#[test]
fn a_cjk_declaration_line_does_not_panic_the_duplicate_scan() {
    let script = r##"const sec = I(null, {type:"frame", name:"折射规律（一）：三线共面", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,0,0,0], fill:[{type:"solid",color:"#FFFBF0"}]});"##;

    let program = run_script_to_program(script).expect("a clean script still runs");
    assert!(program.contains("折射规律"), "{program}");
}

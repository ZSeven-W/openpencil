//! Sandbox, salvage, and resource-limit tests for the shared script runner.

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
fn mid_run_throw_salvages_recorded_prefix() {
    let program = run_script_to_program(
        r#"I(null, {type: "frame"});
I(b0_missing_binding_reference.oops, {});"#,
    )
    .expect("salvage keeps the first line");
    assert_eq!(program.lines().count(), 1);
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
    let err = run_script_to_program(&big).unwrap_err();
    assert!(err.contains("script too large"), "got: {err}");
}

#[test]
fn infinite_loop_is_interrupted_not_hung() {
    let start = std::time::Instant::now();
    let err = run_script_to_program("while (true) {}").unwrap_err();
    assert!(
        start.elapsed() < std::time::Duration::from_secs(10),
        "interrupt fired"
    );
    assert!(!err.is_empty());
}

#[test]
fn memory_bomb_is_rejected() {
    let err = run_script_to_program("let s = 'x'; while (true) { s += s; }").unwrap_err();
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
    .unwrap_err();
    assert!(
        err.contains("no I(...) operations"),
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
fn unsupported_mutations_are_rejected_instead_of_silently_dropped() {
    let error = run_script_to_program(
        r#"const root = I(null, {type: "frame", name: "Root"});
U(root, {x: 10});"#,
    )
    .expect_err("U() must not look successful while doing nothing");
    assert!(error.contains("OP_SCRIPT_MODE_UNSUPPORTED"), "{error}");
    assert!(error.contains("operations mode"), "{error}");
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

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
fn pencil_ops_and_console_are_noop_stubs() {
    // The PRELUDE advertises C/U/D/M/R/G plus console.* as no-op stubs so a
    // script generated against the batch_design DSL vocabulary (and any
    // stray console logging) never aborts the sandbox — only I(...) may
    // cause an effect. Calling every stub plus a couple of real inserts must
    // still return Ok with exactly the I() lines recorded.
    let program = run_script_to_program(
        r#"console.log("building card");
console.warn("heads up");
console.error("nope");
console.info("fyi");
console.debug("trace");
G("root", "search", "prompt");
C(null, {type: "frame", name: "Copy"});
U("n1", {x: 10});
D("n2");
M("n3", null);
R("n4", "n5");
I(null, {type: "frame", name: "Root"});
I(null, {type: "text", content: "Hi"});"#,
    )
    .expect("pencil-op + console stubs must not abort the script");
    let lines: Vec<&str> = program.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "only the two I() calls record a line; stub calls are no-ops"
    );
    assert!(lines[0].starts_with("b0=I(null, "));
    assert!(lines[0].contains(r#""name":"Root""#));
    assert!(lines[1].starts_with("b1=I(null, "));
    assert!(lines[1].contains(r#""content":"Hi""#));
}

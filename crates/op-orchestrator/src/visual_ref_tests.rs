//! Tests for `visual_ref.rs` — S4 B2 + C1.

use crate::design_system::default_design_system;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use crate::types::{
    AbortFlag, CallRequest, DesignRequest, LlmChunk, LlmClient, LlmError, OrchestratorError,
    Progress, ValidationProviders, VisualRefProvider,
};
use crate::visual_ref::{
    build_enhanced_prompt, execute_visual_ref_orchestration, extract_html_from_response,
    extract_structure_summary, generate_design_code,
};
use futures::stream::BoxStream;
use std::sync::Mutex;

// ── generate_design_code ──────────────────────────────────────────────────

#[tokio::test]
async fn generate_design_code_returns_llm_output_verbatim() {
    let expected = "<html><body>Hello</body></html>";
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(expected.to_string())]);
    let abort = AbortFlag::new();
    let ds = default_design_system();

    let result =
        generate_design_code("a login page", ds, 1440.0, 900.0, &llm, None, None, &abort).await;

    assert_eq!(result, expected);
}

#[tokio::test]
async fn generate_design_code_returns_empty_on_llm_error() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
        message: "timeout".into(),
        aborted: false,
    })]);
    let abort = AbortFlag::new();
    let ds = default_design_system();

    let result =
        generate_design_code("a dashboard", ds, 1440.0, 900.0, &llm, None, None, &abort).await;

    assert_eq!(result, "");
}

#[tokio::test]
async fn generate_design_code_respects_model_and_provider() {
    // This test just checks the call doesn't panic with model/provider set.
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text("<!DOCTYPE html>".to_string())]);
    let abort = AbortFlag::new();
    let ds = default_design_system();

    let result = generate_design_code(
        "a settings page",
        ds,
        375.0,
        812.0,
        &llm,
        Some("claude-3-5-sonnet"),
        Some("anthropic"),
        &abort,
    )
    .await;

    assert!(!result.is_empty());
}

// ── extract_structure_summary ─────────────────────────────────────────────

#[test]
fn extract_structure_summary_header_line() {
    let result = extract_structure_summary("<div></div>");
    assert_eq!(
        result.lines().next().unwrap(),
        "DESIGN REFERENCE STRUCTURE:"
    );
}

#[test]
fn extract_structure_summary_section_with_class() {
    let html = r#"<section class="hero"><h1>Welcome</h1></section>"#;
    let result = extract_structure_summary(html);
    assert!(
        result.contains("- Section: hero"),
        "expected Section: hero in:\n{result}"
    );
    assert!(
        result.contains("- H1: \"Welcome\""),
        "expected H1: Welcome in:\n{result}"
    );
}

#[test]
fn extract_structure_summary_section_h1_cta() {
    // This is the byte-exact test from the plan:
    // `extract_structure_summary("<section><h1>Hero</h1><a>CTA</a></section>")` → exact format
    // Note: <section> has no class/id → no Section line.
    // <h1> → H1 line.
    // <a> has no class → no CTA line.
    let html = "<section><h1>Hero</h1><a>CTA</a></section>";
    let result = extract_structure_summary(html);
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "DESIGN REFERENCE STRUCTURE:");
    assert_eq!(lines[1], "- H1: \"Hero\"");
    // Only 2 lines (header + H1) — no section/CTA since no class attrs
    assert_eq!(lines.len(), 2);
}

#[test]
fn extract_structure_summary_skips_bem_modifier_classes() {
    // classOrId containing `__` must be skipped
    let html = r#"<div class="hero__inner">content</div>"#;
    let result = extract_structure_summary(html);
    assert!(
        !result.contains("hero__inner"),
        "BEM modifier should be skipped"
    );
}

#[test]
fn extract_structure_summary_cta_with_btn_class() {
    let html = r#"<button class="btn-primary">Get Started</button>"#;
    let result = extract_structure_summary(html);
    assert!(
        result.contains("- CTA: \"Get Started\""),
        "expected CTA in:\n{result}"
    );
}

#[test]
fn extract_structure_summary_cta_with_cta_class() {
    let html = "<a class=\"cta-link\" href=\"#\">Learn More</a>";
    let result = extract_structure_summary(html);
    assert!(
        result.contains("- CTA: \"Learn More\""),
        "expected CTA in:\n{result}"
    );
}

#[test]
fn extract_structure_summary_cta_with_button_class() {
    let html = "<a class=\"button-primary\" href=\"#\">Sign Up</a>";
    let result = extract_structure_summary(html);
    assert!(
        result.contains("- CTA: \"Sign Up\""),
        "expected CTA in:\n{result}"
    );
}

#[test]
fn extract_structure_summary_headings_truncated_to_60_chars() {
    let long_title = "A".repeat(80);
    let html = format!("<h2>{long_title}</h2>");
    let result = extract_structure_summary(&html);
    // Should be truncated to 60 chars
    let expected_content: String = "A".repeat(60);
    assert!(
        result.contains(&format!("- H2: \"{expected_content}\"")),
        "should truncate to 60 chars:\n{result}"
    );
}

#[test]
fn extract_structure_summary_cta_text_truncated_to_30_chars() {
    let long_text = "B".repeat(50);
    let html = format!(r#"<button class="btn">{long_text}</button>"#);
    let result = extract_structure_summary(&html);
    let expected_text: String = "B".repeat(30);
    assert!(
        result.contains(&format!("- CTA: \"{expected_text}\"")),
        "should truncate to 30 chars:\n{result}"
    );
}

#[test]
fn extract_structure_summary_fallback_when_no_structure() {
    let html = "<div><span>plain</span></div>";
    let result = extract_structure_summary(html);
    assert!(
        result.contains("(HTML structure extracted"),
        "expected fallback line:\n{result}"
    );
}

#[test]
fn extract_structure_summary_section_with_id_attribute() {
    let html = r#"<section id="about">content</section>"#;
    let result = extract_structure_summary(html);
    assert!(
        result.contains("- Section: about"),
        "expected Section: about in:\n{result}"
    );
}

#[test]
fn extract_structure_summary_all_heading_levels() {
    let html = "<h1>One</h1><h2>Two</h2><h3>Three</h3><h4>Four</h4><h5>Five</h5><h6>Six</h6>";
    let result = extract_structure_summary(html);
    for (level, text) in [
        (1, "One"),
        (2, "Two"),
        (3, "Three"),
        (4, "Four"),
        (5, "Five"),
        (6, "Six"),
    ] {
        assert!(
            result.contains(&format!("- H{level}: \"{text}\"")),
            "missing H{level} in:\n{result}"
        );
    }
}

#[test]
fn extract_structure_summary_strips_tags_from_headings() {
    let html = "<h1><span>Hello</span> <em>World</em></h1>";
    let result = extract_structure_summary(html);
    assert!(
        result.contains("- H1: \"Hello World\""),
        "should strip tags:\n{result}"
    );
}

// ── build_enhanced_prompt ─────────────────────────────────────────────────

#[test]
fn build_enhanced_prompt_exact_template() {
    let original = "Design a login screen";
    let structure = "DESIGN REFERENCE STRUCTURE:\n- H1: \"Login\"";
    let ds_context = "DESIGN SYSTEM (use these values consistently):\nColors: bg #F8FAFC";

    let result = build_enhanced_prompt(original, structure, ds_context);

    let expected = format!(
        "{original}\n\n{structure}\n\n{ds_context}\n\nIMPORTANT: Follow the design reference structure closely. The design system colors, fonts, and spacing have already been determined — use them consistently. The reference structure shows the intended layout — match its section order and composition."
    );
    assert_eq!(result, expected);
}

#[test]
fn build_enhanced_prompt_contains_important_instruction() {
    let result = build_enhanced_prompt("p", "s", "d");
    assert!(result.contains("IMPORTANT: Follow the design reference structure closely."));
    assert!(result.contains("use them consistently"));
    assert!(result.contains("match its section order and composition."));
}

#[test]
fn build_enhanced_prompt_double_newline_separators() {
    let result = build_enhanced_prompt("p", "s", "d");
    // Check the exact separators: p\n\ns\n\nd\n\nIMPORTANT...
    assert!(result.starts_with("p\n\ns\n\nd\n\nIMPORTANT:"));
}

#[test]
fn build_enhanced_prompt_empty_inputs() {
    let result = build_enhanced_prompt("", "", "");
    // Should still produce the IMPORTANT line
    assert!(result.contains("IMPORTANT:"));
    assert_eq!(result, "\n\n\n\n\n\nIMPORTANT: Follow the design reference structure closely. The design system colors, fonts, and spacing have already been determined — use them consistently. The reference structure shows the intended layout — match its section order and composition.");
}

// ── execute_visual_ref_orchestration — C1 tests ───────────────────────────

// Shared test fixtures ---------------------------------------------------

fn make_request() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: true,
    }
}

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

/// Minimal valid plan JSON for the Orchestrator.
const PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
             "layout": "vertical", "gap": 0,
             "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
{ "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } }
  ]
}"##;

fn node_json(prefix: &str) -> String {
    format!(
        r#"[{{"type":"frame","id":"{prefix}-1","name":"Sec","x":0,"y":0,"width":1200,"height":300,"children":[]}}]"#
    )
}

fn default_ds_json() -> String {
    let ds = default_design_system();
    serde_json::to_string(ds).expect("serialize default DS")
}

/// An `LlmClient` that records every `CallRequest` it sees while still
/// returning scripted responses in order.  Used to verify the enhanced
/// prompt reaches the underlying orchestrator.
struct RecordingLlm {
    responses: Mutex<std::collections::VecDeque<ScriptResponse>>,
    recorded: Mutex<Vec<CallRequest>>,
}

impl RecordingLlm {
    fn new(responses: Vec<ScriptResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            recorded: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<CallRequest> {
        self.recorded.lock().unwrap().clone()
    }
}

impl LlmClient for RecordingLlm {
    fn call(&self, req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        self.recorded.lock().unwrap().push(req);
        let next = self.responses.lock().unwrap().pop_front();
        let items: Vec<Result<LlmChunk, LlmError>> = match next {
            Some(ScriptResponse::Text(t)) => vec![Ok(LlmChunk::Text(t))],
            Some(ScriptResponse::Fail(e)) => vec![Err(e)],
            None => vec![Err(LlmError {
                message: "RecordingLlm exhausted".into(),
                aborted: false,
            })],
        };
        Box::pin(futures::stream::iter(items))
    }
}

// A `VisualRefProvider` that returns Some(base64) for any call.
struct MockVisualRefProvider;
impl VisualRefProvider for MockVisualRefProvider {
    fn render_html_to_screenshot(&self, _html: &str, _w: f64, _h: f64) -> Option<String> {
        Some("base64screenshot==".to_string())
    }
}

// A `VisualRefProvider` that always returns None.
struct NoneVisualRefProvider;
impl VisualRefProvider for NoneVisualRefProvider {
    fn render_html_to_screenshot(&self, _html: &str, _w: f64, _h: f64) -> Option<String> {
        None
    }
}

// ── Test 1: None screenshot → still runs enhanced orchestration ──────────

/// When `VisualRefProvider` returns `None`, the pipeline does NOT fall back
/// to plain orchestration — it emits
/// `Progress::VisualRefScreenshotReady { skipped: true }` and continues
/// to the enhanced `Orchestrator::run`.  No `VisualRefFallback` event is
/// emitted. Matches TS `visual-ref-orchestrator.ts:109-122` semantics.
#[tokio::test]
async fn execute_visual_ref_none_screenshot_still_runs_enhanced_orchestration() {
    // LLM call order:
    //  [0] generate_design_system  → DS JSON
    //  [1] generate_design_code    → HTML (contains <h1>Hero</h1>)
    //  [2] Orchestrator planning   → PLAN_JSON
    //  [3] Orchestrator subtask    → node_json
    let llm = RecordingLlm::new(vec![
        ScriptResponse::Text(default_ds_json()),
        ScriptResponse::Text("<html><body><h1>Hero</h1></body></html>".into()),
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let providers = stub_providers();

    let result = execute_visual_ref_orchestration(
        &mut sink,
        &llm,
        &providers,
        &NoneVisualRefProvider,
        make_request(),
        &mut |p| events.push(p),
        &AbortFlag::new(),
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());

    // All four pre-orchestrator events
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::VisualRefStarted)),
        "missing VisualRefStarted in {:?}",
        events
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::VisualRefDesignSystem { .. })),
        "missing VisualRefDesignSystem in {:?}",
        events
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::VisualRefHtmlGenerated { .. })),
        "missing VisualRefHtmlGenerated in {:?}",
        events
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::VisualRefScreenshotReady { skipped: true })),
        "expected VisualRefScreenshotReady{{skipped:true}} in {:?}",
        events
    );

    // CRITICAL: no VisualRefFallback should be emitted when screenshot is None.
    let no_fallback = events
        .iter()
        .all(|e| !matches!(e, Progress::VisualRefFallback { .. }));
    assert!(
        no_fallback,
        "stage 3 None → must NOT emit VisualRefFallback; got {:?}",
        events
    );

    // Orchestrator emits Planning when it runs.
    assert!(
        events.iter().any(|e| matches!(e, Progress::Planning)),
        "expected Planning event from enhanced Orchestrator::run, got {:?}",
        events
    );

    // CRITICAL: the planning call (call index 2, after DS + codegen) must
    // have received the ENHANCED prompt (not the original "a landing page").
    // The enhanced prompt contains the IMPORTANT-instruction tail.
    let calls = llm.calls();
    assert!(
        calls.len() >= 3,
        "expected ≥3 LLM calls (DS + codegen + planning), got {}",
        calls.len()
    );
    let planning_user_prompt = &calls[2].user_prompt;
    assert!(
        planning_user_prompt.contains("IMPORTANT: Follow the design reference structure closely."),
        "planning user prompt should carry the enhanced-prompt instruction tail; got:\n{}",
        planning_user_prompt
    );
    // It should also carry the structure summary marker (since HTML had <h1>Hero</h1>).
    assert!(
        planning_user_prompt.contains("DESIGN REFERENCE STRUCTURE:"),
        "planning user prompt should carry the structure summary header; got:\n{}",
        planning_user_prompt
    );
}

// ── Test 2: MockVisualRefProvider → all 5 stages + Orchestrator::run ──────

/// When `VisualRefProvider` returns `Some(base64)`, all 5 stages run and
/// `Orchestrator::run` is called with the enhanced prompt.
#[tokio::test]
async fn execute_visual_ref_with_screenshot_runs_all_stages() {
    // LLM call order:
    //  [0] generate_design_system  → DS JSON
    //  [1] generate_design_code    → HTML
    //  [2] Orchestrator planning   → PLAN_JSON
    //  [3] Orchestrator subtask    → node_json
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(default_ds_json()),
        ScriptResponse::Text("<html><body><h1>Hero</h1></body></html>".into()),
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let providers = stub_providers();

    let result = execute_visual_ref_orchestration(
        &mut sink,
        &llm,
        &providers,
        &MockVisualRefProvider,
        make_request(),
        &mut |p| events.push(p),
        &AbortFlag::new(),
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());

    // VisualRefStarted
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::VisualRefStarted)),
        "missing VisualRefStarted"
    );
    // VisualRefDesignSystem
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::VisualRefDesignSystem { .. })),
        "missing VisualRefDesignSystem"
    );
    // VisualRefHtmlGenerated
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::VisualRefHtmlGenerated { .. })),
        "missing VisualRefHtmlGenerated"
    );
    // VisualRefScreenshotReady { skipped: false }
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::VisualRefScreenshotReady { skipped: false })),
        "missing VisualRefScreenshotReady{{skipped:false}}"
    );
    // No fallback
    let no_fallback = events
        .iter()
        .all(|e| !matches!(e, Progress::VisualRefFallback { .. }));
    assert!(no_fallback, "unexpected VisualRefFallback in {:?}", events);

    // Orchestrator events present (Planning at minimum)
    let has_planning = events.iter().any(|e| matches!(e, Progress::Planning));
    assert!(has_planning, "expected Planning in {:?}", events);
}

// ── Test 3: generate_design_code returns empty → fallback ─────────────────

/// When `generate_design_code` returns an empty string (LLM fails or returns
/// nothing), the function emits `VisualRefFallback` and falls back.
#[tokio::test]
async fn execute_visual_ref_empty_html_falls_back() {
    // LLM call order:
    //  [0] generate_design_system  → DS JSON
    //  [1] generate_design_code    → empty (simulated via LLM error)
    //  [2] Orchestrator planning   → PLAN_JSON (fallback runs Orchestrator)
    //  [3] Orchestrator subtask    → node_json
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(default_ds_json()),
        ScriptResponse::Fail(LlmError {
            message: "codegen timeout".into(),
            aborted: false,
        }),
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let providers = stub_providers();

    let result = execute_visual_ref_orchestration(
        &mut sink,
        &llm,
        &providers,
        &MockVisualRefProvider,
        make_request(),
        &mut |p| events.push(p),
        &AbortFlag::new(),
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());

    let has_fallback = events
        .iter()
        .any(|e| matches!(e, Progress::VisualRefFallback { .. }));
    assert!(
        has_fallback,
        "expected VisualRefFallback for empty HTML in {:?}",
        events
    );
    // No VisualRefHtmlGenerated (never got past code gen)
    let no_html_event = events
        .iter()
        .all(|e| !matches!(e, Progress::VisualRefHtmlGenerated { .. }));
    assert!(
        no_html_event,
        "unexpected VisualRefHtmlGenerated when HTML empty"
    );
}

// ── Test 4: abort before stage 1 → Err(Aborted) ──────────────────────────

/// When the abort flag is set before the call, the function returns
/// `Err(OrchestratorError::Aborted)`.
#[tokio::test]
async fn execute_visual_ref_abort_before_stage_1_returns_aborted() {
    let llm = ScriptedLlm::new(vec![]);
    let mut sink = VecDocSink::new();
    let providers = stub_providers();
    let abort = AbortFlag::new();
    abort.set(); // fire before any call

    let result = execute_visual_ref_orchestration(
        &mut sink,
        &llm,
        &providers,
        &NoneVisualRefProvider,
        make_request(),
        &mut |_| {},
        &abort,
    )
    .await;

    assert!(
        matches!(result, Err(OrchestratorError::Aborted)),
        "expected Aborted, got {:?}",
        result
    );
}

// ── Test 5: DesignSystem var_count matches seeded commands ─────────────────

/// `VisualRefDesignSystem.var_count` equals the number of commands emitted
/// by `design_system_to_seed_commands(default)` = 17.
#[tokio::test]
async fn execute_visual_ref_ds_progress_reports_correct_var_count() {
    // Use NoneVisualRefProvider to keep test simple (fallback after screenshot)
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(default_ds_json()),
        ScriptResponse::Text("<html><body>page</body></html>".into()),
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let providers = stub_providers();

    let _ = execute_visual_ref_orchestration(
        &mut sink,
        &llm,
        &providers,
        &NoneVisualRefProvider,
        make_request(),
        &mut |p| events.push(p),
        &AbortFlag::new(),
    )
    .await;

    // Find the VisualRefDesignSystem event and check var_count
    let ds_event = events.iter().find_map(|e| {
        if let Progress::VisualRefDesignSystem { var_count } = e {
            Some(*var_count)
        } else {
            None
        }
    });
    assert!(
        ds_event.is_some(),
        "missing VisualRefDesignSystem event in {:?}",
        events
    );
    // DEFAULT_DESIGN_SYSTEM: 8 palette + 6 spacing + 3 radius = 17
    assert_eq!(
        ds_event.unwrap(),
        17,
        "expected 17 vars from DEFAULT_DESIGN_SYSTEM"
    );
}

// ── extract_html_from_response ────────────────────────────────────────────

#[test]
fn extract_html_fenced_with_doctype_returns_inner() {
    // Stage 1: ```html ... ``` wraps a full HTML document → return inner (trimmed).
    let resp =
        "Sure here's the HTML:\n```html\n<!DOCTYPE html>\n<html><body>Hi</body></html>\n```\nDone.";
    let html = extract_html_from_response(resp);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.ends_with("</html>"));
    assert!(!html.contains("```"));
    assert!(!html.contains("Done."));
}

#[test]
fn extract_html_fenced_no_doctype_marker_falls_through_to_wrap() {
    // Stage 1 falls through (inner has no <!DOCTYPE/<html); stage 2-3 don't match;
    // stage 4 wraps the ORIGINAL trimmed response (fence chars and all).
    let resp = "```\n<div>not a full doc</div>\n```";
    let html = extract_html_from_response(resp);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<body>```"));
}

#[test]
fn extract_html_starts_with_doctype_returns_trimmed_verbatim() {
    // Stage 2: trimmed starts with <!DOCTYPE → return as-is.
    let resp = "  <!DOCTYPE html>\n<html><body>X</body></html>  ";
    let html = extract_html_from_response(resp);
    assert_eq!(html, "<!DOCTYPE html>\n<html><body>X</body></html>");
}

#[test]
fn extract_html_starts_with_html_returns_verbatim() {
    // Stage 2: trimmed starts with <html → return as-is.
    let resp = "<html><body>Y</body></html>";
    let html = extract_html_from_response(resp);
    assert_eq!(html, "<html><body>Y</body></html>");
}

#[test]
fn extract_html_embedded_doctype_to_close_extracted() {
    // Stage 3: chatty preamble + trailing text → slice out <!DOCTYPE…</html>.
    let resp = "Here's the design:\n<!DOCTYPE html>\n<html><body>Hi</body></html>\nLet me know.";
    let html = extract_html_from_response(resp);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.ends_with("</html>"));
    assert!(!html.contains("Let me know"));
    assert!(!html.contains("Here's the design"));
}

#[test]
fn extract_html_case_insensitive_doctype_via_stage3() {
    // Lowercase <!doctype doesn't pass case-sensitive stage 2 startsWith;
    // stage 3 case-insensitive find catches it. Original case preserved.
    let resp = "<!doctype html>\n<HTML><body>Z</body></HTML>";
    let html = extract_html_from_response(resp);
    assert_eq!(html, "<!doctype html>\n<HTML><body>Z</body></HTML>");
}

#[test]
fn extract_html_bare_text_wrapped_in_default_scaffold() {
    // Stage 4: no fence, no <!DOCTYPE/<html → wrap in default scaffold.
    let resp = "Hello world.";
    let html = extract_html_from_response(resp);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<body>Hello world.</body>"));
}

#[test]
fn extract_html_empty_response_wrapped_as_empty_body() {
    // Stage 4 with empty trimmed content.
    let html = extract_html_from_response("");
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<body></body>"));
}

#[test]
fn extract_html_utf8_preamble_does_not_panic_stage3() {
    // Regression guard: `text.to_lowercase()` would change byte length for
    // some non-ASCII chars (e.g. Turkish İ → i\u{0307}, German ẞ in some
    // locales), making indices derived from the lowercased string unsafe
    // on the original UTF-8 text. The ASCII-byte-scan implementation lets
    // any UTF-8 preamble pass through cleanly. Asserts no panic + correct
    // slice extraction.
    let resp = "解释一下:İ\n<!DOCTYPE html>\n<html><body>UTF-8 OK</body></html>\n注释";
    let html = extract_html_from_response(resp);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.ends_with("</html>"));
    assert!(!html.contains("解释一下"));
    assert!(!html.contains("注释"));
    assert!(!html.contains("İ"));
}

#[test]
fn extract_html_utf8_inside_body_preserved() {
    // Multi-byte chars INSIDE the extracted HTML must round-trip intact.
    let resp = "Here:\n<!DOCTYPE html>\n<html><body>你好 İstanbul</body></html>\n done";
    let html = extract_html_from_response(resp);
    assert!(html.contains("你好"));
    assert!(html.contains("İstanbul"));
    assert!(!html.contains("Here:"));
    assert!(!html.contains("done"));
}

#[test]
fn extract_html_mixed_case_doctype_with_utf8_preamble() {
    // Combine case-insensitivity + UTF-8 preamble. ASCII-byte scan handles
    // both without to_lowercase() byte-shift.
    let resp = "Note: ß and İ are tricky.\n<!DocType html>\n<HTML><body>OK</body></HTML>";
    let html = extract_html_from_response(resp);
    assert_eq!(html, "<!DocType html>\n<HTML><body>OK</body></HTML>");
}

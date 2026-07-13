//! C1 tests — `parse_validation_response` + `validate_design_screenshot`.

use crate::types::{VisionCallRequest, VisionLlmClient, VisionResponse};
use crate::validation::{
    build_vision_request, parse_validation_response, validate_design_screenshot, ValidationResult,
};
use crate::validation_config::VALIDATION_TIMEOUT_MS;
use std::sync::Mutex;
use std::time::Duration;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// A `VisionLlmClient` that records the last request and returns a canned response.
struct CapturingVisionClient {
    response: VisionResponse,
    captured: Mutex<Option<VisionCallRequest>>,
}

impl CapturingVisionClient {
    fn with_text(text: &str) -> Self {
        Self {
            response: VisionResponse::Text(text.to_string()),
            captured: Mutex::new(None),
        }
    }
    fn with_skipped(reason: Option<String>) -> Self {
        Self {
            response: VisionResponse::Skipped { reason },
            captured: Mutex::new(None),
        }
    }
    fn last_req(&self) -> Option<VisionCallRequest> {
        self.captured.lock().unwrap().clone()
    }
}

impl VisionLlmClient for CapturingVisionClient {
    fn validate(&self, req: VisionCallRequest) -> VisionResponse {
        *self.captured.lock().unwrap() = Some(req);
        self.response.clone()
    }
}

// ── parse_validation_response tests ──────────────────────────────────────────

/// Well-formed JSON with all 4 fields extracts correctly.
#[test]
fn parse_extracts_four_fields_from_plain_json() {
    // Use escaped string to avoid # inside raw-string delimiters.
    let json = "{\"issues\": [\"Text too small\", \"Button misaligned\"],\
        \"fixes\": [\
            {\"nodeId\": \"node-1\", \"property\": \"fontSize\", \"value\": 14},\
            {\"nodeId\": \"node-2\", \"property\": \"fillColor\", \"value\": \"#FF0000\"}\
        ],\
        \"structuralFixes\": [],\
        \"qualityScore\": 7}";
    let resp = parse_validation_response(json);
    assert_eq!(resp.issues.len(), 2);
    assert_eq!(resp.fixes.len(), 2);
    assert_eq!(resp.structural_fixes.len(), 0);
    assert_eq!(resp.quality_score, 7);
}

/// JSON wrapped in markdown code fences is cleaned and parsed correctly.
#[test]
fn parse_strips_markdown_fences() {
    let text =
        "```json\n{\"issues\":[],\"fixes\":[],\"structuralFixes\":[],\"qualityScore\":9}\n```";
    // The JSON extractor picks up the inner object.
    let resp = parse_validation_response(text);
    assert_eq!(resp.quality_score, 9);
    assert!(resp.fixes.is_empty());
}

/// JSON embedded in prose is extracted via the `{…}` greedy scan.
#[test]
fn parse_extracts_json_from_prose() {
    let text = r#"Here is my analysis: {"issues":["spacing off"],"fixes":[],"structuralFixes":[],"qualityScore":5} I hope that helps."#;
    let resp = parse_validation_response(text);
    assert_eq!(resp.quality_score, 5);
    assert_eq!(resp.issues, vec!["spacing off"]);
}

/// Malformed JSON returns an empty default response (no panic).
#[test]
fn parse_malformed_json_returns_default() {
    let resp = parse_validation_response("this is not json at all");
    assert!(resp.issues.is_empty());
    assert!(resp.fixes.is_empty());
    assert!(resp.structural_fixes.is_empty());
    assert_eq!(resp.quality_score, 0);
}

/// Empty string returns default (no panic).
#[test]
fn parse_empty_string_returns_default() {
    let resp = parse_validation_response("");
    assert_eq!(resp.quality_score, 0);
}

/// `qualityScore` is clamped to [1, 10] when > 0; strings are coerced.
#[test]
fn parse_quality_score_clamped_and_coerced() {
    // Above 10 → 10
    let resp = parse_validation_response(
        r#"{"issues":[],"fixes":[],"structuralFixes":[],"qualityScore":15}"#,
    );
    assert_eq!(resp.quality_score, 10);

    // Below 1 but > 0 → 1
    let resp = parse_validation_response(
        r#"{"issues":[],"fixes":[],"structuralFixes":[],"qualityScore":0.3}"#,
    );
    assert_eq!(resp.quality_score, 1);

    // Exactly 0 → 0 (parse-failure marker)
    let resp = parse_validation_response(
        r#"{"issues":[],"fixes":[],"structuralFixes":[],"qualityScore":0}"#,
    );
    assert_eq!(resp.quality_score, 0);

    // String "8" → 8
    let resp = parse_validation_response(
        r#"{"issues":[],"fixes":[],"structuralFixes":[],"qualityScore":"8"}"#,
    );
    assert_eq!(resp.quality_score, 8);
}

/// Fixes with invalid properties or values are filtered out.
#[test]
fn parse_filters_invalid_fixes() {
    let json = r#"{
        "issues": [],
        "fixes": [
            {"nodeId": "n1", "property": "dangerousProp", "value": 42},
            {"nodeId": "n2", "property": "fontSize", "value": "not_a_number"},
            {"nodeId": "n3", "property": "fontSize", "value": 16}
        ],
        "structuralFixes": [],
        "qualityScore": 6
    }"#;
    let resp = parse_validation_response(json);
    // Only the valid fontSize:16 fix should survive.
    assert_eq!(resp.fixes.len(), 1);
    assert_eq!(resp.fixes[0].node_id, "n3");
    assert_eq!(resp.fixes[0].property, "fontSize");
}

/// Valid structural fixes (addChild + removeNode) are parsed.
#[test]
fn parse_structural_fixes_valid() {
    let json = r#"{
        "issues": [],
        "fixes": [],
        "structuralFixes": [
            {"action": "removeNode", "nodeId": "old-node"},
            {"action": "addChild", "parentId": "parent-1", "node": {"type": "text", "name": "Label"}}
        ],
        "qualityScore": 7
    }"#;
    let resp = parse_validation_response(json);
    assert_eq!(resp.structural_fixes.len(), 2);
}

/// Invalid structural fixes are filtered out.
#[test]
fn parse_structural_fixes_invalid_are_filtered() {
    let json = r#"{
        "issues": [],
        "fixes": [],
        "structuralFixes": [
            {"action": "removeNode"},
            {"action": "addChild", "node": {"type": "text"}},
            {"action": "removeNode", "nodeId": "valid-id"}
        ],
        "qualityScore": 5
    }"#;
    let resp = parse_validation_response(json);
    // Only the valid removeNode survives.
    assert_eq!(resp.structural_fixes.len(), 1);
}

/// `<tool_use>…</tool_use>` blocks are stripped before parsing.
#[test]
fn parse_strips_tool_use_blocks() {
    let text = r#"<tool_use><name>some_tool</name><input>{"junk":true}</input></tool_use>{"issues":[],"fixes":[],"structuralFixes":[],"qualityScore":4}"#;
    let resp = parse_validation_response(text);
    assert_eq!(resp.quality_score, 4);
}

/// Missing `fixes` array → default (TS: `if (!Array.isArray(parsed.fixes)) return null`).
#[test]
fn parse_missing_fixes_array_returns_default() {
    let json = r#"{"issues":[],"structuralFixes":[],"qualityScore":7}"#;
    let resp = parse_validation_response(json);
    // Falls through to default.
    assert_eq!(resp.quality_score, 0);
    assert!(resp.fixes.is_empty());
}

// ── build_vision_request tests (message + timeout construction) ──────────────

/// Round 1, no reference: standard timeout, no round-N or REFERENCE blurb.
#[test]
fn build_vision_request_round1_no_reference() {
    let req = build_vision_request(
        "system prompt",
        "base64img==",
        "node-tree-dump",
        None,
        None,
        None,
        1,
    );
    assert_eq!(req.system, "system prompt");
    assert_eq!(req.image_base64, "base64img==");
    assert_eq!(req.timeout, Duration::from_millis(VALIDATION_TIMEOUT_MS));
    assert!(req.message.contains("node-tree-dump"));
    assert!(req
        .message
        .contains("Image review is limited to rendering integrity"));
    assert!(req
        .message
        .contains("Do not judge or replace image content"));
    assert!(!req.message.contains("validation round"));
    assert!(!req.message.contains("REFERENCE DESIGN"));
}

/// Reference screenshot present: timeout doubles + REFERENCE blurb injected.
#[test]
fn build_vision_request_timeout_doubled_with_reference() {
    let req = build_vision_request(
        "sys",
        "img",
        "tree",
        None,
        None,
        Some("reference-img-b64"),
        1,
    );
    assert_eq!(
        req.timeout,
        Duration::from_millis(VALIDATION_TIMEOUT_MS * 2)
    );
    assert!(req.message.contains("REFERENCE DESIGN"));
    assert!(req
        .message
        .contains("Ignore differences in photographic or generated image content"));
}

/// Round > 1: "This is validation round N" instruction injected.
#[test]
fn build_vision_request_round2_instruction_injected() {
    let req = build_vision_request("sys", "img", "tree", None, None, None, 2);
    assert!(req.message.contains("validation round 2"));
}

/// `model` and `provider` are forwarded into `VisionCallRequest`.
#[test]
fn build_vision_request_forwards_model_and_provider() {
    let req = build_vision_request("sys", "img", "tree", Some("gpt-5"), Some("openai"), None, 1);
    assert_eq!(req.model.as_deref(), Some("gpt-5"));
    assert_eq!(req.provider.as_deref(), Some("openai"));
}

// ── validate_design_screenshot tests (full dispatch + parse) ─────────────────

/// Round 1 happy-path: client receives the request, response is parsed.
#[test]
fn validate_screenshot_happy_path_parses_response() {
    let json_resp =
        r#"{"issues":["padding uneven"],"fixes":[],"structuralFixes":[],"qualityScore":6}"#;
    let client = CapturingVisionClient::with_text(json_resp);

    let result = validate_design_screenshot(
        &client,
        "system prompt",
        "base64img==",
        "node-tree-dump",
        None,
        None,
        None,
        1,
    );

    assert!(!result.skipped);
    let resp = result.response.unwrap();
    assert_eq!(resp.quality_score, 6);
    assert_eq!(resp.issues, vec!["padding uneven"]);

    // The same VisionCallRequest the helper would build flows through to the client.
    let req = client.last_req().unwrap();
    assert_eq!(req.system, "system prompt");
    assert_eq!(req.image_base64, "base64img==");
}

/// When the client returns `Skipped`, `ValidationResult.skipped` is `true`
/// and the reason propagates into `error`.
#[test]
fn validate_screenshot_skipped_response_propagated() {
    let client = CapturingVisionClient::with_skipped(Some("no vision provider".to_string()));

    let result = validate_design_screenshot(&client, "sys", "img", "tree", None, None, None, 1);

    assert!(result.skipped);
    assert!(result.response.is_none());
    assert_eq!(result.error.as_deref(), Some("no vision provider"));
}

/// `Skipped` with no reason → `error` is `None`.
#[test]
fn validate_screenshot_skipped_no_reason() {
    let client = CapturingVisionClient::with_skipped(None);
    let result = validate_design_screenshot(&client, "sys", "img", "tree", None, None, None, 1);
    assert!(result.skipped);
    assert!(result.error.is_none());
}

/// Default `ValidationResult` has `skipped: false` and no response.
#[test]
fn validation_result_default() {
    let r: ValidationResult = ValidationResult::default();
    assert!(!r.skipped);
    assert!(r.response.is_none());
}

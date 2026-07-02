//! C1 + C2: Vision-validation pipeline — per-round call + full loop.
//!
//! C1 port: `apps/web/src/services/ai/design-validation.ts:137-274`
//! (`validateDesignScreenshot` + `parseValidationResponse`).
//!
//! C2 port: `apps/web/src/services/ai/design-validation.ts:280-524`
//! (`runPostGenerationValidation` — the full loop).

#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;

use crate::types::{
    AbortFlag, DesignRequest, DocSink, OrchestratorError, PreValidator, Progress,
    ScreenshotProvider, VisionCallRequest, VisionLlmClient, VisionResponse,
};
use crate::validation_config::{
    MAX_VALIDATION_ROUNDS, VALIDATION_NODE_COUNT_THRESHOLD, VALIDATION_QUALITY_THRESHOLD,
    VALIDATION_TIMEOUT_MS,
};
use crate::validation_dump::{build_node_tree_dump, count_nodes_in_active_page};
use crate::validation_fixes::apply::{apply_validation_fixes, StructuralFix, ValidationFix};
use crate::validation_fixes::{is_valid_fix_value, is_valid_structural_fix, SAFE_FIX_PROPERTIES};

// ── Response types ────────────────────────────────────────────────────────────

/// The 4-field payload extracted from a vision LLM's JSON response.
///
/// Faithful port of the `ValidationResult` shape in
/// `design-validation-fixes.ts:50-62` / `design-validation.ts:230-274`.
#[derive(Debug, Clone, Default)]
pub(crate) struct ValidationResponse {
    /// Natural-language descriptions of visual issues.
    pub issues: Vec<String>,
    /// Property fixes to apply to existing nodes.
    pub fixes: Vec<ValidationFix>,
    /// Structural fixes (addChild / removeNode).
    pub structural_fixes: Vec<StructuralFix>,
    /// Quality score 1–10 (0 = parse failure / not set).
    pub quality_score: u8,
}

/// Result of `validate_design_screenshot` — either a parsed response or skipped.
#[derive(Debug, Clone, Default)]
pub(crate) struct ValidationResult {
    /// When `true`, the vision call was skipped (stub/unavailable/error).
    /// `response` is `None` in this case.
    pub skipped: bool,
    /// Parsed response when available.
    pub response: Option<ValidationResponse>,
    /// The raw text returned by the LLM (for diagnostics).
    pub raw_text: Option<String>,
    /// Human-readable reason when skipped (e.g. "stub", provider error).
    pub error: Option<String>,
}

// ── parse_validation_response ─────────────────────────────────────────────────

/// Parse the raw text from a vision LLM into a `ValidationResponse`.
///
/// Steps (port of `parseValidationResponse` in `design-validation.ts:230-274`):
/// 1. Strip `<tool_use>…</tool_use>` blocks.
/// 2. Try direct parse on the cleaned text.
/// 3. Try extracting the first `{…}` blob with a greedy regex and re-parsing.
/// 4. On any failure return `ValidationResponse::default()` (zero everything).
///
/// Within each parse attempt the function:
/// - Filters `fixes` through the safe-property whitelist + value validator.
/// - Filters `structuralFixes` through `is_valid_structural_fix`.
/// - Clamps `qualityScore` to [1, 10]; 0 → 0 (parse / not-set marker).
pub(crate) fn parse_validation_response(text: &str) -> ValidationResponse {
    // Strip Agent SDK tool_use XML blocks (port of TS line 260).
    let cleaned = strip_tool_use_blocks(text);
    let cleaned = cleaned.trim();

    // Try direct parse.
    if let Some(resp) = try_parse_json(cleaned) {
        return resp;
    }

    // Try extracting the outermost {…} blob.
    if let Some(blob) = extract_json_object(cleaned) {
        if let Some(resp) = try_parse_json(blob) {
            return resp;
        }
    }

    ValidationResponse::default()
}

/// Attempt to parse `json` as a `ValidationResponse`.
///
/// Returns `None` when the text isn't valid JSON **or** when the `fixes` array
/// is missing (TS: `if (!Array.isArray(parsed.fixes)) return null`).
fn try_parse_json(json: &str) -> Option<ValidationResponse> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let obj = value.as_object()?;

    // `fixes` must be an array (TS: `if (!Array.isArray(parsed.fixes)) return null`).
    let fixes_raw = obj.get("fixes")?;
    if !fixes_raw.is_array() {
        return None;
    }

    // Parse and filter `fixes`.
    let fixes: Vec<ValidationFix> = fixes_raw
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| {
            let fo = f.as_object()?;
            let node_id = fo.get("nodeId")?.as_str()?.to_string();
            if node_id.is_empty() {
                return None;
            }
            let property = fo.get("property")?.as_str()?.to_string();
            // Must be in safe-fix whitelist.
            if !SAFE_FIX_PROPERTIES.iter().any(|p| p.name == property) {
                return None;
            }
            let value = fo.get("value")?.clone();
            // Value must be valid for this property.
            if !is_valid_fix_value(&property, &value) {
                return None;
            }
            Some(ValidationFix {
                node_id,
                property,
                value,
            })
        })
        .collect();

    // Parse and filter `structuralFixes`.
    let structural_fixes: Vec<StructuralFix> = obj
        .get("structuralFixes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|sf| is_valid_structural_fix(sf))
                .filter_map(structural_fix_from_value)
                .collect()
        })
        .unwrap_or_default();

    // Parse `issues` — must be an array of strings.
    let issues: Vec<String> = obj
        .get("issues")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Parse `qualityScore` — number or numeric string; clamp to [1, 10] when > 0.
    let raw_score = obj.get("qualityScore");
    let num_score: f64 = match raw_score {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    };
    let quality_score: u8 = if num_score > 0.0 {
        // Port of TS: `Math.max(1, Math.min(10, Math.round(numScore)))`.
        let rounded = num_score.round() as i64;
        rounded.clamp(1, 10) as u8
    } else {
        0
    };

    Some(ValidationResponse {
        issues,
        fixes,
        structural_fixes,
        quality_score,
    })
}

/// Convert a validated `serde_json::Value` into a `StructuralFix`.
///
/// Assumes the value has already been validated by `is_valid_structural_fix`.
fn structural_fix_from_value(value: &serde_json::Value) -> Option<StructuralFix> {
    let obj = value.as_object()?;
    match obj.get("action")?.as_str()? {
        "removeNode" => {
            let node_id = obj.get("nodeId")?.as_str()?.to_string();
            Some(StructuralFix::RemoveNode { node_id })
        }
        "addChild" => {
            let parent_id = obj.get("parentId")?.as_str()?.to_string();
            let index = obj
                .get("index")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let spec = obj.get("node")?.clone();
            Some(StructuralFix::AddChild {
                parent_id,
                index,
                spec,
            })
        }
        _ => None,
    }
}

/// Strip `<tool_use>…</tool_use>` blocks from `text`.
///
/// Port of TS line 260:
/// ```ts
/// text.replace(/<tool_use>[\s\S]*?<\/tool_use>/g, '').trim()
/// ```
fn strip_tool_use_blocks(text: &str) -> String {
    // Simple repeated scan — avoids pulling in a regex crate.
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    const OPEN: &str = "<tool_use>";
    const CLOSE: &str = "</tool_use>";

    while let Some(start) = remaining.find(OPEN) {
        result.push_str(&remaining[..start]);
        // Advance past the opening tag.
        remaining = &remaining[start + OPEN.len()..];
        // Skip until the closing tag.
        if let Some(end) = remaining.find(CLOSE) {
            remaining = &remaining[end + CLOSE.len()..];
        } else {
            // No closing tag — stop (don't emit the rest of the open block).
            remaining = "";
            break;
        }
    }
    result.push_str(remaining);
    result
}

/// Extract the first outermost `{…}` blob from `text` using a greedy scan.
///
/// Port of TS: `const match = cleaned.match(/\{[\s\S]*\}/);`
fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    // Find the matching last `}` (greedy — match all `}` chars).
    let end = text.rfind('}')?;
    if end >= start {
        Some(&text[start..=end])
    } else {
        None
    }
}

// ── validate_design_screenshot ────────────────────────────────────────────────

/// Build the `VisionCallRequest` for a single validation round.
///
/// The message template (port of TS L160-166) wraps the `node_tree_dump` in a
/// fenced block; reference-comparison + round-N instructions are appended
/// conditionally (port of TS L151-158).
///
/// **Timeout rule (port of TS L147-149):** when a `reference_screenshot` is
/// provided, the per-round timeout is doubled (`VALIDATION_TIMEOUT_MS * 2`).
///
/// Tests assert on the returned `VisionCallRequest` directly (no LLM call
/// needed to verify message/timeout/model/provider construction).
pub(crate) fn build_vision_request(
    system_prompt: &str,
    image_base64: &str,
    node_tree_dump: &str,
    model: Option<&str>,
    provider: Option<&str>,
    reference_screenshot: Option<&str>,
    round: u8,
) -> VisionCallRequest {
    // Reference-comparison instruction (port of TS L151-153).
    let reference_instruction = if reference_screenshot.is_some() {
        "\n\nA REFERENCE DESIGN screenshot was also provided. Compare the current design \
against the reference and fix any significant deviations in layout, spacing, proportions, \
or missing elements. The reference shows the intended design — the current screenshot \
should match its structure, visual balance, and element completeness. If elements visible \
in the reference are missing in the current design, use structuralFixes with addChild to \
add them."
    } else {
        ""
    };

    // Round-specific instruction (port of TS L155-158).
    let round_instruction = if round > 1 {
        format!(
            "\n\nThis is validation round {round}. Previous fixes have already been applied. \
Focus on remaining issues only — do NOT re-report issues that have already been fixed."
        )
    } else {
        String::new()
    };

    // Full user message (port of TS L160-166).
    let message = format!(
        "Analyze this UI design screenshot. Here is the node tree structure:\n\n\
```\n{node_tree_dump}\n```\n\n\
Cross-reference visual issues with the node IDs above. \
Return JSON fixes using real node IDs from the tree.\
{reference_instruction}{round_instruction}"
    );

    // Timeout doubled when reference screenshot is present (TS L147-149).
    let timeout_ms = if reference_screenshot.is_some() {
        VALIDATION_TIMEOUT_MS * 2
    } else {
        VALIDATION_TIMEOUT_MS
    };

    VisionCallRequest {
        system: system_prompt.to_string(),
        message,
        image_base64: image_base64.to_string(),
        model: model.map(|s| s.to_string()),
        provider: provider.map(|s| s.to_string()),
        timeout: Duration::from_millis(timeout_ms),
    }
}

/// Per-round vision validation call.
///
/// Builds the request via `build_vision_request`, calls
/// `vision_client.validate`, parses the response via
/// `parse_validation_response`, and returns a `ValidationResult`.
///
/// Faithful port of `validateDesignScreenshot` in
/// `design-validation.ts:137-228`.
///
/// 8 parameters is faithful to the TS call-site — suppress the lint.
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_design_screenshot(
    vision_client: &dyn VisionLlmClient,
    system_prompt: &str,
    image_base64: &str,
    node_tree_dump: &str,
    model: Option<&str>,
    provider: Option<&str>,
    reference_screenshot: Option<&str>,
    round: u8,
) -> ValidationResult {
    let req = build_vision_request(
        system_prompt,
        image_base64,
        node_tree_dump,
        model,
        provider,
        reference_screenshot,
        round,
    );

    match vision_client.validate(req) {
        VisionResponse::Text(text) => {
            let response = parse_validation_response(&text);
            ValidationResult {
                skipped: false,
                raw_text: Some(text),
                response: Some(response),
                error: None,
            }
        }
        VisionResponse::Skipped { reason } => ValidationResult {
            skipped: true,
            raw_text: None,
            response: None,
            error: reason,
        },
    }
}

// ── C2: ValidationSummary ────────────────────────────────────────────────────

/// Summary returned by `run_post_generation_validation`.
///
/// `total_applied` is pre-validation fixes + all vision-round fixes combined.
/// `rounds_run` is the number of completed vision LLM rounds (0 when the node
/// count gate fires or the screenshot is unavailable).
#[derive(Debug, Clone, Default)]
pub struct ValidationSummary {
    pub total_applied: usize,
    pub rounds_run: u8,
}

// ── C2: run_post_generation_validation ───────────────────────────────────────

/// Full post-generation validation loop.
///
/// Faithful port of `runPostGenerationValidation` in
/// `apps/web/src/services/ai/design-validation.ts:280-524`.
///
/// Steps:
/// 1. Emit `Progress::ValidationStarted`.
/// 2. Run `pre_validator.run_pre_validation_fixes(sink)`.
///    Emit `Progress::ValidationPreCheckDone`.
/// 3. If `request.validation_enabled == false` → emit `ValidationDone`, return.
/// 4. Node-count gate: if node count < 30 → emit `ValidationDone`, return.
/// 5. Loop `round in 1..=MAX_VALIDATION_ROUNDS`:
///    a. Check abort.
///    b. Emit `ValidationRoundStarted`.
///    c. `screenshot.capture_root_frame()` — `None` → break.
///    d. `build_node_tree_dump(sink.state())`.
///    e. `validate_design_screenshot(...)` — call the vision LLM.
///    f. `Skipped` → break (round 1) or break (round > 1).
///    g. `quality_score==0 && issues.is_empty()` → parse failure → break.
///    h. `quality_score >= VALIDATION_QUALITY_THRESHOLD` → break.
///    i. Track fix history; deduplicate fixes seen ≥2 times.
///    j. `fixes.is_empty() && structural_fixes.is_empty()` → break.
///    k. `apply_validation_fixes` → add to `total_applied`.
///    l. Emit `ValidationRoundDone`.
///    m. If applied == 0 → break (nothing could be applied).
/// 6. Emit `ValidationDone`.  Return `Ok(ValidationSummary)`.
///
/// Returns `Err(OrchestratorError::Aborted)` when the abort flag is set before
/// any vision round starts.
///
/// `reference_screenshot` is currently always `None` from within this function;
/// D1's host wiring will thread host-side reference images through.
#[allow(clippy::too_many_arguments)]
pub fn run_post_generation_validation(
    sink: &mut dyn DocSink,
    pre_validator: &dyn PreValidator,
    screenshot: &dyn ScreenshotProvider,
    vision: &dyn VisionLlmClient,
    system_prompt: &str,
    request: &DesignRequest,
    on_progress: &mut dyn FnMut(Progress),
    abort: &AbortFlag,
) -> Result<ValidationSummary, OrchestratorError> {
    let mut total_applied: usize = 0;
    let mut rounds_run: u8 = 0;

    // ── Step 1: signal that validation phase started ──────────────────────────
    on_progress(Progress::ValidationStarted);

    // ── Step 2: pre-validation (pure code checks, no LLM) ────────────────────
    let pre = pre_validator.run_pre_validation_fixes(sink);
    total_applied += pre.total;
    on_progress(Progress::ValidationPreCheckDone {
        applied: pre.total,
        by_category: pre.by_category,
    });

    // ── Step 3: LLM-validation gate — host can disable entirely ───────────────
    if !request.validation_enabled {
        on_progress(Progress::ValidationDone { total_applied });
        return Ok(ValidationSummary {
            total_applied,
            rounds_run,
        });
    }

    // ── Step 4: node-count gate (TS L328-342) ─────────────────────────────────
    let node_count = count_nodes_in_active_page(sink.state());
    if node_count < VALIDATION_NODE_COUNT_THRESHOLD {
        on_progress(Progress::ValidationDone { total_applied });
        return Ok(ValidationSummary {
            total_applied,
            rounds_run,
        });
    }

    // ── Step 5: vision LLM loop ───────────────────────────────────────────────

    // Fix-history map: "nodeId:property" → count of rounds where this fix appeared.
    let mut fix_history: HashMap<String, usize> = HashMap::new();

    for round in 1..=MAX_VALIDATION_ROUNDS {
        // 5a: abort check — if set before this round starts, return early.
        if abort.is_set() {
            return Err(OrchestratorError::Aborted);
        }

        // 5b: emit round-started progress.
        on_progress(Progress::ValidationRoundStarted { round });

        // 5c: capture screenshot of the LIVE (post-fix) document — None means bail.
        // `sink.state()` is the current mutated state: the host's real provider
        // renders it, the stub ignores it and returns `None` (loop short-circuits,
        // default-path behavior unchanged).
        let Some(image_base64) = screenshot.capture_root_frame(sink.state()) else {
            // TS: on first round → emit "done" with error; subsequent rounds → break.
            break;
        };

        // 5d: build node-tree dump.
        let node_tree_dump = build_node_tree_dump(sink.state());

        // 5e: call vision LLM (reference_screenshot = None for C2; D1 wires host ref).
        let result = validate_design_screenshot(
            vision,
            system_prompt,
            &image_base64,
            &node_tree_dump,
            request.model.as_deref(),
            request.provider.as_deref(),
            None, // reference_screenshot — D1 will thread host-side refs here
            round,
        );

        // 5f: skipped → break (round 1 or subsequent).
        if result.skipped {
            break;
        }

        // Unwrap the response (skipped=false guarantees Some).
        let response = result.response.unwrap_or_default();

        // 5g: quality_score==0 && issues.is_empty() → parse failure → break.
        if response.quality_score == 0 && response.issues.is_empty() {
            rounds_run = round;
            on_progress(Progress::ValidationRoundDone {
                round,
                applied: 0,
                quality_score: 0,
            });
            break;
        }

        // 5h: quality at or above threshold → acceptable; stop loop.
        if response.quality_score >= VALIDATION_QUALITY_THRESHOLD {
            rounds_run = round;
            on_progress(Progress::ValidationRoundDone {
                round,
                applied: 0,
                quality_score: response.quality_score,
            });
            break;
        }

        // 5i: track fix history for all fixes returned this round.
        for fix in &response.fixes {
            let key = format!("{}:{}", fix.node_id, fix.property);
            *fix_history.entry(key).or_insert(0) += 1;
        }

        // 5i (continued): deduplicate fixes seen 2+ times across rounds.
        // TS: filter out fixes whose history count > 1 when round > 1.
        let deduped_fixes: Vec<ValidationFix> = if round > 1 {
            response
                .fixes
                .into_iter()
                .filter(|f| {
                    let key = format!("{}:{}", f.node_id, f.property);
                    fix_history.get(&key).copied().unwrap_or(0) <= 1
                })
                .collect()
        } else {
            response.fixes
        };

        // 5j: no fixes at all → break (design is already good or nothing actionable).
        if deduped_fixes.is_empty() && response.structural_fixes.is_empty() {
            rounds_run = round;
            on_progress(Progress::ValidationRoundDone {
                round,
                applied: 0,
                quality_score: response.quality_score,
            });
            break;
        }

        // 5k: apply fixes.
        let apply_result = apply_validation_fixes(sink, &deduped_fixes, &response.structural_fixes);
        total_applied += apply_result.applied;

        // 5l: emit round-done progress.
        rounds_run = round;
        on_progress(Progress::ValidationRoundDone {
            round,
            applied: apply_result.applied,
            quality_score: response.quality_score,
        });

        // 5m: if nothing could be applied (all fixes were invalid/skipped), stop.
        if apply_result.applied == 0 {
            break;
        }
    }

    // ── Step 6: done ──────────────────────────────────────────────────────────
    on_progress(Progress::ValidationDone { total_applied });
    Ok(ValidationSummary {
        total_applied,
        rounds_run,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "validation_tests_c1.rs"]
mod tests;

#[cfg(test)]
#[path = "validation_tests_c2.rs"]
mod tests_c2;

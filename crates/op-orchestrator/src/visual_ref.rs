//! `visual_ref.rs` — S4 B2+C1: HTML helpers + main orchestration for the
//! visual-reference pipeline.
//!
//! Functions ported from the deleted TS source (`commit 0f12b6e9^`):
//!
//! - [`generate_design_code`] — LLM call using `design-code` +
//!   `design-principles` skills as the system prompt. Returns HTML verbatim.
//!   Port of `design-code-generator.ts:26-49`.
//!
//! - [`extract_structure_summary`] — hand-rolled HTML scanner. Extracts
//!   section containers (by class/id), headings (`<h1>`-`<h6>`), and CTA
//!   elements (buttons/anchors whose class contains `btn`, `button`, or `cta`).
//!   Returns an indented text summary. Port of `design-code-generator.ts:90-136`.
//!
//! - [`build_enhanced_prompt`] — string concatenation producing the exact
//!   prompt template from `visual-ref-orchestrator.ts:172-184`.
//!
//! - [`execute_visual_ref_orchestration`] — 5-stage visual-ref pipeline.
//!   Port of `executeVisualRefOrchestration` in
//!   `visual-ref-orchestrator.ts:45-166`.
//!
//! No `regex` crate dependency (not in workspace Cargo.toml). The scanner uses
//! a hand-rolled byte-level approach consistent with the S3b-4 precedent.

use futures::StreamExt;

use crate::design_system::{
    design_system_to_prompt_context, design_system_to_seed_commands, generate_design_system,
    DesignSystem,
};
use crate::run::Orchestrator;
use crate::types::{
    AbortFlag, CallRequest, DesignRequest, DocSink, LlmChunk, LlmClient, OrchestratorError,
    Progress, RunSummary, ValidationProviders, VisualRefProvider,
};

// ── generate_design_code ──────────────────────────────────────────────────────

/// Generate self-contained HTML/CSS code for a design request.
///
/// Port of `generateDesignCode` in `design-code-generator.ts:26-49`.
///
/// System prompt = `design-code` skill content + `\n\n` + `design-principles`
/// skill content (when principles are non-empty).
/// User prompt = `buildCodeGenUserPrompt(prompt, dsContext, width, height)`.
/// Returns the raw LLM output (HTML verbatim — no post-processing here;
/// the TS `extractHtmlFromResponse` is intentionally omitted to keep this
/// function a minimal faithful port of the I/O shape only).
#[allow(clippy::too_many_arguments)]
pub async fn generate_design_code(
    prompt: &str,
    ds: &DesignSystem,
    width: f64,
    height: f64,
    llm: &dyn LlmClient,
    model: Option<&str>,
    provider: Option<&str>,
    abort: &AbortFlag,
) -> String {
    let design_code = op_ai_skills::get_skill_by_name("design-code")
        .map(|e| e.content.as_str())
        .unwrap_or("")
        .to_string();
    let principles = op_ai_skills::get_skill_by_name("design-principles")
        .map(|e| e.content.as_str())
        .unwrap_or("")
        .to_string();

    // TS: `const systemPrompt = principles ? \`${designCodeSkill}\n\n${principles}\` : designCodeSkill`
    let system_prompt = if principles.is_empty() {
        design_code
    } else {
        format!("{design_code}\n\n{principles}")
    };

    let ds_context = design_system_to_prompt_context(ds);
    let user_prompt = build_code_gen_user_prompt(prompt, &ds_context, width, height);

    let req = CallRequest {
        system_prompt,
        user_prompt,
        model: model.map(|s| s.to_string()),
        provider: provider.map(|s| s.to_string()),
        timeout: std::time::Duration::from_secs(60),
        abort: abort.clone(),
        no_text_timeout: None,
        first_text_timeout: None,
    };

    let mut stream = llm.call(req);
    let mut text = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(LlmChunk::Text(t)) => text.push_str(&t),
            Ok(LlmChunk::Thinking(_)) => {}
            Err(_) => break,
        }
    }
    // Empty text = LLM error or truly-empty response. We can't distinguish
    // here (no exceptions in Rust streams). Return empty so the caller's
    // `html.is_empty()` fall-back path fires — TS catches the LLM error via
    // try/catch and falls back; this is the equivalent shortcut. Non-empty
    // text gets the full 4-stage normalization (TS extractHtmlFromResponse).
    if text.is_empty() {
        text
    } else {
        extract_html_from_response(&text)
    }
}

/// Normalize an LLM response into clean HTML.
///
/// Port of `extractHtmlFromResponse` in `design-code-generator.ts:54-87`.
/// Faithful 4-stage chain — without this, code-fence-wrapped HTML
/// (the most common LLM output shape) leaks ``` markers into the
/// downstream `extract_structure_summary` scan.
///
/// 1. ` ```(html)?\s*\n?…\n?``` ` fenced block — if the inner content
///    contains `<!DOCTYPE` or `<html`, return the inner content (trimmed).
/// 2. Trimmed response itself starts with `<!DOCTYPE` or `<html` — return as-is.
/// 3. Case-insensitive find of `<!DOCTYPE…</html>` substring — return slice.
/// 4. Wrap bare content in a default HTML document scaffold.
pub(crate) fn extract_html_from_response(response: &str) -> String {
    let trimmed = response.trim();

    // Stage 1: code fence with HTML inside
    if let Some(content) = extract_html_fenced_content(trimmed) {
        let inner = content.trim();
        if inner.contains("<!DOCTYPE") || inner.contains("<html") {
            return inner.to_string();
        }
    }

    // Stage 2: trimmed starts with DOCTYPE / html
    if trimmed.starts_with("<!DOCTYPE") || trimmed.starts_with("<html") {
        return trimmed.to_string();
    }

    // Stage 3: embedded <!DOCTYPE…</html>
    if let Some(html) = extract_doctype_to_html_close(trimmed) {
        return html;
    }

    // Stage 4: wrap bare content in default scaffold
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><title>Design</title></head>\n<body>{trimmed}</body>\n</html>"
    )
}

/// Find content inside ` ```(html)?…``` `. Returns raw inner (caller trims).
/// Matches TS regex `/```(?:html)?\s*\n?([\s\S]*?)\n?```/`.
fn extract_html_fenced_content(text: &str) -> Option<&str> {
    let start = text.find("```")?;
    let after = &text[start + 3..];

    // Optional "html" language tag
    let after = after.strip_prefix("html").unwrap_or(after);
    // \s* — strip any leading whitespace (incl. \n)
    let after = after.trim_start();

    // Closing ```
    let end = after.find("```")?;
    Some(&after[..end])
}

/// ASCII case-insensitive find of `<!DOCTYPE…</html>` in `text`. Returns
/// the slice from `text` (preserving original case) between those markers.
///
/// **UTF-8 safety**: we scan byte windows with `eq_ignore_ascii_case`
/// instead of `text.to_lowercase()`. The `to_lowercase()` approach can
/// change byte lengths for non-ASCII (e.g. Turkish `İ` 2 bytes →
/// `i\u{0307}` 3 bytes), making lower-string indices unsafe to use on
/// the original `text` — slicing on a non-char-boundary panics. ASCII
/// needles (`<!doctype` / `</html>`) always land on byte positions that
/// are valid UTF-8 char boundaries (any byte ≤ 0x7F is a char start in
/// UTF-8), so direct byte-index slicing on `text` is safe.
fn extract_doctype_to_html_close(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let doc_needle: &[u8] = b"<!doctype";
    let close_needle: &[u8] = b"</html>";

    let doc_start = bytes.windows(doc_needle.len()).position(|w| {
        w.iter()
            .zip(doc_needle)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    })?;

    let close_rel = bytes[doc_start..]
        .windows(close_needle.len())
        .position(|w| {
            w.iter()
                .zip(close_needle)
                .all(|(a, b)| a.eq_ignore_ascii_case(b))
        })?;

    let end = doc_start + close_rel + close_needle.len();
    Some(text[doc_start..end].to_string())
}

/// Build the user prompt for HTML/CSS code generation.
///
/// Port of `buildCodeGenUserPrompt` in `design-code-generator.ts:168-185`.
fn build_code_gen_user_prompt(
    user_prompt: &str,
    design_system_context: &str,
    width: f64,
    height: f64,
) -> String {
    let height_instruction = if height > 0.0 {
        format!("Height: {}px (fixed viewport).", height as i64)
    } else {
        "Height: auto (content determines height, estimate based on sections).".to_string()
    };

    format!(
        "Design request: {user_prompt}\n\nViewport: Width {width}px. {height_instruction}\n\n{design_system_context}\n\nGenerate the complete HTML file now.",
        width = width as i64,
    )
}

// ── extract_structure_summary ─────────────────────────────────────────────────

/// Extract a structural summary from HTML for use as sub-agent reference.
///
/// Port of `extractStructureSummary` in `design-code-generator.ts:90-136`.
///
/// Scans the HTML string for:
/// 1. Section-level containers (`<section|header|footer|nav|main|div>`) with a
///    `class` or `id` attribute — emits `- Section: {classOrId}` (skips
///    values containing `__` to filter BEM modifiers).
/// 2. Headings `<h1>`-`<h6>` — emits `- H{n}: "{content}"` (inner text
///    stripped of tags, truncated to 60 chars).
/// 3. CTAs: `<button|a>` whose `class` attribute contains `btn`, `button`, or
///    `cta` — emits `- CTA: "{text}"` (inner text stripped, truncated to 30 chars).
///
/// Returns a single string with `DESIGN REFERENCE STRUCTURE:` as the first
/// line.  If nothing was extracted, appends a generic fallback line.
pub fn extract_structure_summary(html: &str) -> String {
    let mut lines: Vec<String> = vec!["DESIGN REFERENCE STRUCTURE:".to_string()];

    // ── 1. Section-level containers ──────────────────────────────────────────
    // TS regex: /<(?:section|header|footer|nav|main|div)\s+[^>]*(?:class|id)="([^"]*)"[^>]*>/gi
    extract_section_containers(html, &mut lines);

    // ── 2. Headings ──────────────────────────────────────────────────────────
    // TS regex: /<h([1-6])[^>]*>([\s\S]*?)<\/h\1>/gi
    extract_headings(html, &mut lines);

    // ── 3. CTA buttons / links ───────────────────────────────────────────────
    // TS regex: /<(?:button|a)\s+[^>]*class="[^"]*(?:btn|button|cta)[^"]*"[^>]*>([\s\S]*?)<\/(?:button|a)>/gi
    extract_ctas(html, &mut lines);

    // Fallback when nothing was extracted
    if lines.len() <= 1 {
        lines.push("(HTML structure extracted — use as visual layout reference)".to_string());
    }

    lines.join("\n")
}

/// Strip HTML tags from a string, returning plain text.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Extract the value of `class` or `id` attribute from an opening tag string.
///
/// Returns the first match found (class takes precedence over id).
/// `tag_inner` is the content between `<` and `>` (exclusive).
fn extract_class_or_id(tag_inner: &str) -> Option<&str> {
    for attr in ["class", "id"] {
        // Search for `class="..."` or `id="..."`
        if let Some(pos) = find_attr(tag_inner, attr) {
            let rest = &tag_inner[pos..];
            // Skip `attr="`
            let start = attr.len() + 2; // `attr="`  length
            if rest.len() > start {
                let value_start = start;
                if let Some(end) = rest[value_start..].find('"') {
                    let val = &rest[value_start..value_start + end];
                    if !val.is_empty() {
                        return Some(val);
                    }
                }
            }
        }
    }
    None
}

/// Find a word-boundary attribute name within `tag_inner` followed by `="`.
///
/// Returns the byte offset of the start of `attr="` in `tag_inner`.
fn find_attr(tag_inner: &str, attr: &str) -> Option<usize> {
    let pattern = format!("{attr}=\"");
    let bytes = tag_inner.as_bytes();
    let pat_bytes = pattern.as_bytes();
    let pat_len = pat_bytes.len();

    let mut i = 0usize;
    while i + pat_len <= bytes.len() {
        if bytes[i..i + pat_len] == *pat_bytes {
            // Check word boundary: previous char must be whitespace or start
            if i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t' || bytes[i - 1] == b'\n' {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Scan for section-level container opening tags and extract class/id values.
fn extract_section_containers(html: &str, lines: &mut Vec<String>) {
    // Container tag names (lowercase)
    const CONTAINERS: &[&str] = &["section", "header", "footer", "nav", "main", "div"];

    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for '<'
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        // Skip '<'
        let tag_start = i + 1;
        if tag_start >= len {
            break;
        }

        // Try to match one of our container tag names (case-insensitive)
        let mut matched_tag: Option<&str> = None;
        for &tag in CONTAINERS {
            let tag_len = tag.len();
            if tag_start + tag_len <= len {
                let slice = &html[tag_start..tag_start + tag_len];
                if slice.eq_ignore_ascii_case(tag) {
                    // Must be followed by whitespace (has attributes) or '>'
                    let after = tag_start + tag_len;
                    if after < len
                        && (bytes[after] == b' '
                            || bytes[after] == b'\t'
                            || bytes[after] == b'\n'
                            || bytes[after] == b'\r')
                    {
                        matched_tag = Some(tag);
                        break;
                    }
                }
            }
        }

        if let Some(_tag) = matched_tag {
            // Find the end of this opening tag
            if let Some(end_offset) = html[i..].find('>') {
                let tag_inner = &html[tag_start..i + end_offset];
                if let Some(class_or_id) = extract_class_or_id(tag_inner) {
                    // TS: `if (classOrId && !classOrId.includes('__'))`
                    if !class_or_id.contains("__") {
                        lines.push(format!("- Section: {class_or_id}"));
                    }
                }
                i = i + end_offset + 1;
                continue;
            }
        }

        i += 1;
    }
}

/// Scan for `<h1>`-`<h6>` elements and extract their text content.
fn extract_headings(html: &str, lines: &mut Vec<String>) {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let tag_start = i + 1;
        if tag_start >= len {
            break;
        }

        // Match `h` followed by digit 1-6
        if bytes[tag_start] != b'h' && bytes[tag_start] != b'H' {
            i += 1;
            continue;
        }
        if tag_start + 1 >= len {
            i += 1;
            continue;
        }
        let digit = bytes[tag_start + 1];
        if !(b'1'..=b'6').contains(&digit) {
            i += 1;
            continue;
        }
        let level = digit - b'0';
        // Must be followed by '>' or whitespace (no other chars)
        let after = tag_start + 2;
        if after < len
            && bytes[after] != b'>'
            && bytes[after] != b' '
            && bytes[after] != b'\t'
            && bytes[after] != b'\n'
            && bytes[after] != b'\r'
        {
            i += 1;
            continue;
        }

        // Find end of opening tag
        let Some(end_offset) = html[i..].find('>') else {
            i += 1;
            continue;
        };
        let after_open_tag = i + end_offset + 1;

        // Build closing tag `</h{level}>`
        let closing = format!("</h{level}>");
        let closing_lower = format!("</h{level}>");

        // Find closing tag (case-insensitive: just check both cases since h is ASCII)
        let rest = &html[after_open_tag..];
        let inner_end = rest
            .find(&closing)
            .or_else(|| rest.find(&closing_lower))
            .or_else(|| {
                // Try uppercase H
                let closing_upper = format!("</H{level}>");
                rest.find(&closing_upper)
            });

        if let Some(inner_len) = inner_end {
            let inner_html = &rest[..inner_len];
            let content = strip_tags(inner_html).trim().to_string();
            // Truncate to 60 chars (TS: `.slice(0, 60)`)
            let content: String = content.chars().take(60).collect();
            if !content.is_empty() {
                lines.push(format!("- H{level}: \"{content}\""));
            }
            i = after_open_tag + inner_len + closing.len();
            continue;
        }

        i += 1;
    }
}

/// Scan for `<button>` and `<a>` elements whose class contains `btn`, `button`,
/// or `cta`, and extract their text content.
fn extract_ctas(html: &str, lines: &mut Vec<String>) {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let tag_start = i + 1;
        if tag_start >= len {
            break;
        }

        // Match `button` or `a` (case-insensitive)
        let matched_tag: Option<(&str, &str)> = if html[tag_start..].len() >= 6
            && html[tag_start..tag_start + 6].eq_ignore_ascii_case("button")
            && tag_start + 6 < len
            && (bytes[tag_start + 6] == b' '
                || bytes[tag_start + 6] == b'\t'
                || bytes[tag_start + 6] == b'\n'
                || bytes[tag_start + 6] == b'\r')
        {
            Some(("button", "</button>"))
        } else if !html[tag_start..].is_empty()
            && html[tag_start..tag_start + 1].eq_ignore_ascii_case("a")
            && tag_start + 1 < len
            && (bytes[tag_start + 1] == b' '
                || bytes[tag_start + 1] == b'\t'
                || bytes[tag_start + 1] == b'\n'
                || bytes[tag_start + 1] == b'\r')
        {
            Some(("a", "</a>"))
        } else {
            None
        };

        if let Some((_tag_name, closing)) = matched_tag {
            // Find end of opening tag
            let Some(end_offset) = html[i..].find('>') else {
                i += 1;
                continue;
            };
            let tag_inner = &html[tag_start..i + end_offset];

            // Check class contains btn/button/cta
            // TS regex: class="[^"]*(?:btn|button|cta)[^"]*"
            let has_cta_class = if let Some(class_pos) = find_attr(tag_inner, "class") {
                let rest = &tag_inner[class_pos + 7..]; // skip `class="`
                if let Some(end) = rest.find('"') {
                    let class_val = &rest[..end];
                    let cv_lower = class_val.to_ascii_lowercase();
                    cv_lower.contains("btn")
                        || cv_lower.contains("button")
                        || cv_lower.contains("cta")
                } else {
                    false
                }
            } else {
                false
            };

            if has_cta_class {
                let after_open_tag = i + end_offset + 1;
                let rest = &html[after_open_tag..];

                // Find closing tag (case-insensitive check: try lowercase and uppercase A)
                let inner_end = rest.find(closing).or_else(|| {
                    let closing_upper = closing.to_ascii_uppercase();
                    rest.find(&closing_upper)
                });

                if let Some(inner_len) = inner_end {
                    let inner_html = &rest[..inner_len];
                    let text = strip_tags(inner_html).trim().to_string();
                    // Truncate to 30 chars (TS: `.slice(0, 30)`)
                    let text: String = text.chars().take(30).collect();
                    if !text.is_empty() {
                        lines.push(format!("- CTA: \"{text}\""));
                    }
                    i = after_open_tag + inner_len + closing.len();
                    continue;
                }
            }

            i = i + end_offset + 1;
            continue;
        }

        i += 1;
    }
}

// ── build_enhanced_prompt ─────────────────────────────────────────────────────

/// Build the enhanced orchestration prompt that incorporates the visual reference.
///
/// Port of `buildEnhancedPrompt` in `visual-ref-orchestrator.ts:172-184`.
///
/// Template (byte-exact from TS):
/// ```text
/// {originalPrompt}
///
/// {structureSummary}
///
/// {designSystemContext}
///
/// IMPORTANT: Follow the design reference structure closely. The design system
/// colors, fonts, and spacing have already been determined — use them
/// consistently. The reference structure shows the intended layout — match its
/// section order and composition.
/// ```
pub fn build_enhanced_prompt(original: &str, structure: &str, ds_context: &str) -> String {
    format!(
        "{original}\n\n{structure}\n\n{ds_context}\n\nIMPORTANT: Follow the design reference structure closely. The design system colors, fonts, and spacing have already been determined — use them consistently. The reference structure shows the intended layout — match its section order and composition."
    )
}

// ── execute_visual_ref_orchestration ─────────────────────────────────────────

/// 5-stage visual-reference pipeline.
///
/// Port of `executeVisualRefOrchestration` in
/// `visual-ref-orchestrator.ts:45-166`.
///
/// ## Stage flow
///
/// 1. Emit `Progress::VisualRefStarted`. Check abort. Call
///    `generate_design_system`, apply `design_system_to_seed_commands` to
///    `sink`, emit `Progress::VisualRefDesignSystem { var_count }`.
///
/// 2. Check abort. Call `generate_design_code`. If the result is empty,
///    emit `Progress::VisualRefFallback` and fall back to `Orchestrator::run`
///    with the original (un-enhanced) request.
///    Otherwise emit `Progress::VisualRefHtmlGenerated { byte_len }`.
///
/// 3. Check abort. Call `visual_ref.render_html_to_screenshot`. Emit
///    `Progress::VisualRefScreenshotReady { skipped: screenshot.is_none() }`.
///    Stage 3 **does not fall back** — the pipeline continues to stages 4+5
///    regardless of whether a screenshot was returned. The screenshot is an
///    optional reference for the downstream vision-validation loop
///    (currently always `None` in Rust S3c; future plumb-through to
///    `validation.rs::reference_screenshot` is out of S4 scope).
///
/// 4. Build `enhanced_request` with prompt replaced by the output of
///    `build_enhanced_prompt(original_prompt, extract_structure_summary(&html),
///    design_system_to_prompt_context(&ds))`.
///
/// 5. Call `Orchestrator::new().run(enhanced_request, ...)` and return its
///    `Result<RunSummary, OrchestratorError>`.
///
/// ## Abort handling
///
/// `abort.is_set()` is checked before each stage. If set, returns
/// `Err(OrchestratorError::Aborted)` immediately.
///
/// ## Fallback semantics
///
/// Fallback to plain `Orchestrator::run(original_request, ...)` happens
/// ONLY on:
/// - Stage 2 failure (`generate_design_code` returns empty string).
///
/// Stage 1 (`generate_design_system`) already has internal fallback to
/// `DEFAULT_DESIGN_SYSTEM` on any LLM/parse failure, so it never
/// short-circuits.  Stage 3 (`render_html_to_screenshot` returning `None`)
/// is informational only — the pipeline continues to stages 4+5 without
/// a screenshot.
///
/// ## Canvas size
///
/// The Rust `DesignRequest` does not carry a `canvas_size` field.  The TS
/// source used `request.context?.canvasSize?.width ?? 1200` and
/// `?? 0` for height, so this function uses the same defaults:
/// `width = 1200.0, height = 0.0`.
#[allow(clippy::too_many_arguments)]
pub async fn execute_visual_ref_orchestration(
    sink: &mut dyn DocSink,
    llm: &dyn LlmClient,
    providers: &ValidationProviders<'_>,
    visual_ref: &dyn VisualRefProvider,
    request: DesignRequest,
    on_progress: &mut dyn FnMut(Progress),
    abort: &AbortFlag,
) -> Result<RunSummary, OrchestratorError> {
    // Default canvas dimensions: matches TS `request.context?.canvasSize?.{width,height} ?? default`
    const PLAN_WIDTH: f64 = 1200.0;
    const PLAN_HEIGHT: f64 = 0.0;

    // Keep a clone of the original request for fallback paths.
    let original_request = request.clone();

    // ── Stage 0: emit started, check abort ────────────────────────────────────
    on_progress(Progress::VisualRefStarted);

    if abort.is_set() {
        return Err(OrchestratorError::Aborted);
    }

    // ── Stage 1: design system ────────────────────────────────────────────────

    // generate_design_system falls back to DEFAULT on any LLM/parse failure.
    let ds = generate_design_system(
        &request.prompt,
        llm,
        request.model.as_deref(),
        request.provider.as_deref(),
        abort,
    )
    .await;

    // Seed design-system variables into the document.
    let seed_cmds = design_system_to_seed_commands(&ds);
    let var_count = seed_cmds.len();
    for cmd in seed_cmds {
        sink.apply(cmd);
    }

    on_progress(Progress::VisualRefDesignSystem { var_count });

    if abort.is_set() {
        return Err(OrchestratorError::Aborted);
    }

    // ── Stage 2: generate HTML/CSS ────────────────────────────────────────────

    let html = generate_design_code(
        &request.prompt,
        &ds,
        PLAN_WIDTH,
        PLAN_HEIGHT,
        llm,
        request.model.as_deref(),
        request.provider.as_deref(),
        abort,
    )
    .await;

    if html.is_empty() {
        on_progress(Progress::VisualRefFallback {
            reason: "design-code failed".into(),
        });
        return Orchestrator::new()
            .run(original_request, sink, llm, on_progress, abort, providers)
            .await;
    }

    on_progress(Progress::VisualRefHtmlGenerated {
        byte_len: html.len(),
    });

    if abort.is_set() {
        return Err(OrchestratorError::Aborted);
    }

    // ── Stage 3: render screenshot ─────────────────────────────────────────────
    //
    // Informational stage: a `None` screenshot is OK — we still proceed to
    // stages 4+5 with the HTML structure summary + design system context.
    // The screenshot is reserved for a future plumb-through to S3c's
    // `validation.rs::reference_screenshot` (currently always `None`).
    let screenshot = visual_ref.render_html_to_screenshot(&html, PLAN_WIDTH, PLAN_HEIGHT);
    on_progress(Progress::VisualRefScreenshotReady {
        skipped: screenshot.is_none(),
    });
    let _ = screenshot; // currently unused downstream; see doc-comment

    if abort.is_set() {
        return Err(OrchestratorError::Aborted);
    }

    // ── Stage 4 + 5: build enhanced request → run Orchestrator ───────────────

    let structure_summary = extract_structure_summary(&html);
    let ds_context = design_system_to_prompt_context(&ds);
    let enhanced_prompt = build_enhanced_prompt(&request.prompt, &structure_summary, &ds_context);

    let enhanced_request = DesignRequest {
        prompt: enhanced_prompt,
        ..request
    };

    Orchestrator::new()
        .run(enhanced_request, sink, llm, on_progress, abort, providers)
        .await
}

#[cfg(test)]
#[path = "visual_ref_tests.rs"]
mod tests;

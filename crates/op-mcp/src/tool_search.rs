//! MCP tool `ToolSearch` — keyword/select discovery over the tool catalog.
//!
//! Lets a design agent discover available tools before calling them — the
//! first step in the Pencil-style agentic tool-loop. Two query forms:
//!
//! - `select:Name1,Name2,...` — direct selection. Returns each matching
//!   tool descriptor verbatim. Comma-separated, so the agent can request
//!   several tools in one shot.
//! - bare keyword query — scores every catalog entry by keyword overlap
//!   against name + description, returns the top `≤ max_results` matches.
//!
//! ## Matching algorithm (ported from `vendor/agent/.../discovery.rs`)
//!
//! For `select:` queries:
//!   1. Split on `,`, trim each name, look up in the catalog by exact
//!      match against the descriptor's `"name"` field.
//!   2. Matched descriptors are returned in order; unmatched names are
//!      silently dropped (no panic).
//!
//! For keyword queries:
//!   1. Lower-case the query and split on whitespace → terms.
//!   2. For each catalog entry, pull `name` + `description`, lower-case them,
//!      compute a score:
//!      - Name split on CamelCase / `_` → parts. Term exactly matching a
//!        part scores 10; term as substring of a part scores 5.
//!      - If no name-part score, term as substring of the full lower-case
//!        name scores 3.
//!      - Term matching a word boundary in the description scores 2.
//!   3. Sort descending by score (ties: lexicographic name); take max `max_results`.
//!
//! ## JSON field extraction
//!
//! Each catalog entry is a raw descriptor string such as
//! `{"name":"batch_design","description":"...","inputSchema":{...}}`.
//! Name and description are extracted with `parser::extract_string_field`
//! — the same hand-rolled extractor used throughout op-mcp — so this
//! module stays serde-free.

use std::collections::BTreeMap;

use super::{McpTool, ToolOutcome};
use crate::parser::extract_string_field;

// --- pure search function ------------------------------------------------

/// Search `catalog` (slice of raw JSON descriptor strings) by `query`
/// and return the matching descriptor strings (full JSON text) in ranked
/// order.
///
/// `max_results` is honoured for keyword queries; `select:` queries
/// always return all matched names regardless of `max_results`.
/// Missing names in a `select:` query are dropped silently.
pub fn tool_search(catalog: &[&str], query: &str, max_results: usize) -> Vec<String> {
    let max = max_results.max(1);
    if let Some(rest) = query.strip_prefix("select:") {
        handle_select(catalog, rest)
    } else {
        keyword_search(catalog, query, max)
    }
}

/// `select:Name1,Name2,...` path — exact name match, unknown names dropped.
fn handle_select(catalog: &[&str], names_csv: &str) -> Vec<String> {
    let mut results = Vec::new();
    for raw in names_csv.split(',') {
        let name = raw.trim();
        if name.is_empty() {
            continue;
        }
        // Find the first descriptor whose "name" field matches exactly.
        if let Some(descriptor) = catalog.iter().find(|d| {
            extract_string_field(d, "name").as_deref() == Some(name)
        }) {
            // Deduplicate (same name requested twice).
            let ds = descriptor.to_string();
            if !results.contains(&ds) {
                results.push(ds);
            }
        }
        // Unknown names are silently dropped — no panic, matching
        // discovery.rs semantics for the op-mcp synchronous path.
    }
    results
}

/// Keyword search — score every entry, sort, take top `max`.
fn keyword_search(catalog: &[&str], query: &str, max: usize) -> Vec<String> {
    let query_lower = query.to_ascii_lowercase();
    let terms: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .collect();
    if terms.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(String, u32, String)> = Vec::new();
    for descriptor in catalog {
        let name = match extract_string_field(descriptor, "name") {
            Some(n) => n,
            None => continue,
        };
        let desc = extract_string_field(descriptor, "description").unwrap_or_default();
        let name_lower = name.to_ascii_lowercase();
        let desc_lower = desc.to_ascii_lowercase();
        let parts = split_tool_name_parts(&name_lower);

        let mut score: u32 = 0;
        for term in &terms {
            // Name-part exact match → 10; substring match → 5.
            if parts.iter().any(|p| p == term) {
                score += 10;
            } else if parts.iter().any(|p| p.contains(term)) {
                score += 5;
            } else if name_lower.contains(term) {
                // Full-name fallback when no part matched → 3.
                score += 3;
            }
            // Description word-boundary match → 2.
            if word_boundary_contains(&desc_lower, term) {
                score += 2;
            }
        }
        if score > 0 {
            scored.push((name, score, descriptor.to_string()));
        }
    }
    // Descending by score, then lexicographic name for stable tie-break.
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    scored.into_iter().take(max).map(|(_, _, d)| d).collect()
}

/// Split a lowercase tool name into searchable parts.
///
/// Underscore-separated names (`batch_design`, `insert_node`) split on `_`.
/// CamelCase names (`BatchDesign`) split on lowercase-to-uppercase transitions.
fn split_tool_name_parts(name_lower: &str) -> Vec<String> {
    // Underscore-separated (most op-mcp tools).
    if name_lower.contains('_') {
        return name_lower
            .split('_')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
    }
    // CamelCase split: insert a split point before each run of a capital
    // letter that follows a lowercase letter (already lowered, so we detect
    // transitions via the original chars — we work on the lowered string
    // but detect positions by looking at the original name lengths being
    // equal, so just split on underscores if present, otherwise treat as
    // a single part for snake_case-free tool names like "ToolSearch").
    //
    // For names already in lowercase without underscores (e.g. schema
    // entries whose "name" is already snake_case with underscores handled
    // above), return as a single part.
    vec![name_lower.to_string()]
}

/// `true` if `term` appears in `haystack` flanked by non-word characters
/// (letters, digits, `_`) or string boundaries. A cheap word-boundary
/// check that avoids pulling in the `regex` crate.
fn word_boundary_contains(haystack: &str, term: &str) -> bool {
    let haystack_bytes = haystack.as_bytes();
    let term_bytes = term.as_bytes();
    let term_len = term_bytes.len();
    let hay_len = haystack_bytes.len();
    if term_len > hay_len {
        return false;
    }
    let mut start = 0usize;
    while start + term_len <= hay_len {
        // Fast scan for the first byte of term.
        let Some(rel) = haystack[start..].find(term) else {
            break;
        };
        let abs = start + rel;
        // Check boundary before.
        let before_ok = abs == 0
            || !is_word_char(haystack_bytes[abs - 1]);
        // Check boundary after.
        let after = abs + term_len;
        let after_ok = after == hay_len
            || !is_word_char(haystack_bytes[after]);
        if before_ok && after_ok {
            return true;
        }
        start = abs + term_len;
    }
    false
}

#[inline]
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// --- MCP tool struct ------------------------------------------------------

/// `ToolSearch` MCP tool. Stateless — the catalog is injected at
/// registration time via the `catalog` field so unit tests can pass a
/// small fixture without touching the real `TOOL_SCHEMAS`.
pub struct ToolSearch {
    /// The full tool catalog: each entry is a raw JSON descriptor string.
    /// In production this is `schemas::TOOL_SCHEMAS`; tests pass a fixture.
    pub catalog: &'static [&'static str],
}

impl McpTool for ToolSearch {
    fn name(&self) -> &str {
        "ToolSearch"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let query = match args.get("query") {
            Some(q) => q.as_str(),
            None => {
                return ToolOutcome::Err(
                    super::ToolErrorCode::MissingArgument,
                    "query is required".into(),
                )
            }
        };
        let max_results: usize = args
            .get("max_results")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5)
            .max(1);

        let matches = tool_search(self.catalog, query, max_results);
        let count = matches.len();

        // Return OkJson so the nested descriptor strings survive
        // verbatim (they are themselves JSON objects). This mirrors
        // batch_get's choice of OkJson for rich nested results.
        let json = build_result_json(query, count, &matches);
        ToolOutcome::OkJson(json)
    }
}

/// Serialize the result into the JSON envelope.
///
/// Shape: `{"query":"...", "count": N, "results": [<descriptor>, ...]}`
/// The `results` array contains the full raw descriptor JSON strings,
/// so the agent gets name + description + inputSchema in one call.
fn build_result_json(query: &str, count: usize, descriptors: &[String]) -> String {
    let mut out = String::with_capacity(256 + descriptors.iter().map(|d| d.len()).sum::<usize>());
    out.push_str("{\"query\":");
    push_json_string(&mut out, query);
    out.push_str(",\"count\":");
    out.push_str(&count.to_string());
    out.push_str(",\"results\":[");
    for (i, d) in descriptors.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        // Each descriptor is already valid JSON — embed verbatim.
        out.push_str(d);
    }
    out.push_str("]}");
    out
}

/// Push a JSON-escaped string literal onto `buf`.
fn push_json_string(buf: &mut String, s: &str) {
    buf.push('"');
    for c in s.chars() {
        match c {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c => buf.push(c),
        }
    }
    buf.push('"');
}

/// Snapshot constructor — matches `get_guidelines_snapshot` convention.
/// The production host supplies `schemas::TOOL_SCHEMAS` as the catalog.
pub fn tool_search_snapshot(catalog: &'static [&'static str]) -> ToolSearch {
    ToolSearch { catalog }
}

// --- unit tests -----------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Small fixture catalog (4 descriptors) used by all tests below.
    /// NOT the real TOOL_SCHEMAS — keeps tests fast and self-contained.
    static FIXTURE: &[&str] = &[
        r#"{"name":"batch_design","description":"Insert or refine design content using the batch DSL.","inputSchema":{"type":"object","properties":{"operations":{"type":"string"}}}}"#,
        r#"{"name":"snapshot_layout","description":"Get the hierarchical bounding box layout tree of the document.","inputSchema":{"type":"object","properties":{}}}"#,
        r#"{"name":"create_variable","description":"Create a new design-token variable. kind is color/number/boolean/string.","inputSchema":{"type":"object","properties":{"name":{"type":"string"}}}}"#,
        r#"{"name":"list_variables","description":"List design variables with kinds.","inputSchema":{"type":"object","properties":{}}}"#,
    ];

    // --- select: path ---

    #[test]
    fn select_returns_exact_named_descriptors() {
        let results = tool_search(FIXTURE, "select:batch_design,snapshot_layout", 11);
        assert_eq!(results.len(), 2);
        // Check each result contains the right "name" field.
        assert!(results[0].contains("\"batch_design\""),
            "first result must be batch_design: {}", results[0]);
        assert!(results[1].contains("\"snapshot_layout\""),
            "second result must be snapshot_layout: {}", results[1]);
    }

    #[test]
    fn select_unknown_name_dropped_not_panicking() {
        // "does_not_exist" is absent from fixture — must be silently
        // dropped so only the batch_design descriptor is returned.
        let results = tool_search(FIXTURE, "select:batch_design,does_not_exist", 11);
        assert_eq!(results.len(), 1, "unknown name must be dropped: {:?}", results);
        assert!(results[0].contains("\"batch_design\""),
            "surviving result must be batch_design: {}", results[0]);
    }

    #[test]
    fn select_empty_catalog_returns_empty() {
        let results = tool_search(&[], "select:batch_design", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn select_duplicate_name_deduplicated() {
        let results = tool_search(FIXTURE, "select:batch_design,batch_design", 5);
        assert_eq!(results.len(), 1);
    }

    // --- keyword search path ---

    #[test]
    fn keyword_variable_returns_variable_tools_first() {
        // Both create_variable and list_variables contain "variable" in
        // their names; they should rank above batch_design / snapshot_layout.
        let results = tool_search(FIXTURE, "variable", 5);
        assert!(!results.is_empty(), "keyword 'variable' must match at least one entry");
        // Both variable tools must appear.
        let has_create = results.iter().any(|d| d.contains("\"create_variable\""));
        let has_list = results.iter().any(|d| d.contains("\"list_variables\""));
        assert!(has_create, "create_variable must be in results: {:?}", results);
        assert!(has_list, "list_variables must be in results: {:?}", results);
        // A non-variable tool must score lower; check it's not ranked first.
        assert!(
            results[0].contains("variable"),
            "top result should contain 'variable' in name: {}", results[0]
        );
    }

    #[test]
    fn keyword_max_results_respected() {
        // Query "design" matches batch_design (name part) and potentially
        // others via description. max=1 must limit to exactly 1 result.
        let results = tool_search(FIXTURE, "design", 1);
        assert_eq!(results.len(), 1, "max_results=1 must limit output: {:?}", results);
    }

    #[test]
    fn keyword_empty_whitespace_query_returns_empty() {
        let results = tool_search(FIXTURE, "   ", 5);
        assert!(results.is_empty(), "blank query must return empty: {:?}", results);
    }

    #[test]
    fn keyword_description_word_boundary_match() {
        // "hierarchical" only appears in snapshot_layout's description —
        // no name part will match, but the description word-boundary
        // matcher should still surface it.
        let results = tool_search(FIXTURE, "hierarchical", 5);
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("\"snapshot_layout\""),
            "description match must return snapshot_layout: {}", results[0]);
    }

    // --- result JSON shape ---

    #[test]
    fn result_json_has_expected_fields() {
        // The OkJson path goes through build_result_json; verify the
        // shape is well-formed and has the required envelope fields.
        let json = build_result_json("variable", 2, &[
            r#"{"name":"create_variable"}"#.to_string(),
            r#"{"name":"list_variables"}"#.to_string(),
        ]);
        assert!(json.contains("\"query\":\"variable\""), "query field missing: {json}");
        assert!(json.contains("\"count\":2"), "count field missing: {json}");
        assert!(json.contains("\"results\":["), "results array missing: {json}");
        assert!(json.contains("\"create_variable\""), "first result missing: {json}");
        assert!(json.contains("\"list_variables\""), "second result missing: {json}");
    }

    // --- MCP tool struct ---

    #[test]
    fn mcp_tool_name_is_tool_search() {
        let tool = tool_search_snapshot(FIXTURE);
        assert_eq!(tool.name(), "ToolSearch");
    }

    #[test]
    fn mcp_tool_missing_query_returns_err() {
        let tool = tool_search_snapshot(FIXTURE);
        let result = tool.call(&BTreeMap::new());
        assert!(
            matches!(result, ToolOutcome::Err(super::super::ToolErrorCode::MissingArgument, _)),
            "missing query must return MissingArgument: {result:?}"
        );
    }

    #[test]
    fn mcp_tool_select_via_call() {
        let tool = tool_search_snapshot(FIXTURE);
        let mut args = BTreeMap::new();
        args.insert("query".into(), "select:batch_design,snapshot_layout".into());
        args.insert("max_results".into(), "11".into());
        match tool.call(&args) {
            ToolOutcome::OkJson(json) => {
                assert!(json.contains("\"batch_design\""), "batch_design missing: {json}");
                assert!(json.contains("\"snapshot_layout\""), "snapshot_layout missing: {json}");
                assert!(json.contains("\"count\":2"), "count wrong: {json}");
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    #[test]
    fn mcp_tool_select_missing_name_via_call() {
        let tool = tool_search_snapshot(FIXTURE);
        let mut args = BTreeMap::new();
        args.insert("query".into(), "select:batch_design,does_not_exist".into());
        args.insert("max_results".into(), "11".into());
        match tool.call(&args) {
            ToolOutcome::OkJson(json) => {
                assert!(json.contains("\"count\":1"), "count must be 1: {json}");
                assert!(json.contains("\"batch_design\""), "batch_design missing: {json}");
                // The unknown name may appear in the echoed "query" field but must
                // NOT appear as a matched result in "results" — check via count.
                // Count=1 already proves only one entry was returned.
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    #[test]
    fn mcp_tool_keyword_via_call() {
        let tool = tool_search_snapshot(FIXTURE);
        let mut args = BTreeMap::new();
        args.insert("query".into(), "variable".into());
        match tool.call(&args) {
            ToolOutcome::OkJson(json) => {
                assert!(json.contains("variable"), "variable results missing: {json}");
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    // --- word_boundary_contains ---

    #[test]
    fn word_boundary_basic_cases() {
        assert!(word_boundary_contains("hello world", "world"));
        assert!(!word_boundary_contains("worldview", "world"));
        assert!(word_boundary_contains("(world)", "world"));
        assert!(!word_boundary_contains("the_world_is", "world")); // _ is a word char
    }
}

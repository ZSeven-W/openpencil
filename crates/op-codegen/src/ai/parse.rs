//! Pure, deterministic ports from code-generation-pipeline.ts (lines
//! 28-220) + sanitizeName. No AI calls — fully unit-testable.

use std::collections::HashMap;

use crate::ai::types::{ChunkContract, ChunkResult, PlannedChunk};

/// Port of computeExecutionOrder (pipeline.ts:86-111). Chunks with no deps
/// get order 0; dependents get max(dep orders)+1; cycles resolve to 0.
pub fn compute_execution_order(chunks: &[PlannedChunk]) -> HashMap<String, usize> {
    fn resolve(
        id: &str,
        chunks: &[PlannedChunk],
        orders: &mut HashMap<String, usize>,
        visiting: &mut Vec<String>,
    ) -> usize {
        if let Some(o) = orders.get(id) {
            return *o;
        }
        if visiting.iter().any(|v| v == id) {
            return 0; // cycle guard (pipeline.ts:91)
        }
        visiting.push(id.to_string());
        let chunk = chunks.iter().find(|c| c.id == id);
        let order = match chunk {
            Some(c) if !c.dependencies.is_empty() => {
                c.dependencies
                    .iter()
                    .map(|d| resolve(d, chunks, orders, visiting))
                    .max()
                    .unwrap_or(0)
                    + 1
            }
            _ => 0,
        };
        visiting.pop();
        orders.insert(id.to_string(), order);
        order
    }

    let mut orders = HashMap::new();
    for c in chunks {
        let mut visiting = Vec::new();
        resolve(&c.id, chunks, &mut orders, &mut visiting);
    }
    orders
}

/// Port of cleanCode (pipeline.ts:215-220): strip lines that are ```lang or
/// ``` fences, then trim.
pub fn clean_code(raw: &str) -> String {
    let mut out = String::new();
    for line in raw.lines() {
        if line.trim_start().starts_with("```") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

/// True PascalCase check — port of /^[A-Z][a-zA-Z0-9]*$/.
fn is_pascal_case(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

/// Port of validateContract (pipeline.ts:28-39). Returns (valid, issues).
pub fn validate_contract(result: &ChunkResult) -> (bool, Vec<String>) {
    let mut issues = Vec::new();
    let name = &result.contract.component_name;
    if !name.is_empty() && !is_pascal_case(name) {
        issues.push(format!(
            "componentName \"{name}\" is not a valid PascalCase identifier"
        ));
    }
    let is_sfc = result.code.contains("<script")
        || result.code.contains("<template")
        || result.code.contains("<style");
    if !name.is_empty() && !is_sfc && !result.code.contains(name.as_str()) {
        issues.push(format!(
            "componentName \"{name}\" not found in generated code"
        ));
    }
    (issues.is_empty(), issues)
}

/// Port of sanitizeName (pen-core) — PascalCase a free-text label: split on
/// non-alphanumerics, uppercase each word's first letter, join.
pub fn sanitize_name(input: &str) -> String {
    input
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut ch = w.chars();
            match ch.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + ch.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Port of runPlanning's JSON extraction (pipeline.ts:516): first `{` to
/// last `}` slice.
pub fn extract_plan_json(response: &str) -> Option<String> {
    let start = response.find('{')?;
    let end = response.rfind('}')?;
    if end >= start {
        Some(response[start..=end].to_string())
    } else {
        None
    }
}

/// Try-parse a contract JSON string (pipeline.ts:160-181). Strips fences,
/// requires a non-empty componentName, stamps chunk_id.
fn try_parse_contract(s: &str, chunk_id: &str) -> Option<ChunkContract> {
    let cleaned = clean_code(s);
    let mut parsed: ChunkContract = serde_json::from_str(cleaned.trim()).ok()?;
    if parsed.component_name.is_empty() {
        return None;
    }
    parsed.chunk_id = chunk_id.to_string();
    Some(parsed)
}

/// Infer a contract from code when no JSON was emitted (pipeline.ts:183-213).
fn infer_contract_from_code(code: &str, chunk_id: &str) -> ChunkContract {
    let component_name = infer_component_name(code).unwrap_or_default();
    ChunkContract {
        chunk_id: chunk_id.to_string(),
        component_name,
        css_classes: Vec::new(),
        css_variables: Vec::new(),
    }
}

/// Extract a component name from common export forms (pipeline.ts:188-195).
fn infer_component_name(code: &str) -> Option<String> {
    if let Some(n) = capture_after(code, "export default function ", |c| {
        c.is_ascii_alphanumeric() || c == '_'
    }) {
        return Some(n);
    }
    if let Some(n) = capture_after(code, "export function ", |c| {
        c.is_ascii_alphanumeric() || c == '_'
    }) {
        if n.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            return Some(n);
        }
    }
    if let Some(n) = capture_between(code, "struct ", ": View") {
        return Some(n.trim().to_string());
    }
    if let Some(n) = capture_after(code, "class ", |c| c.is_ascii_alphanumeric() || c == '_') {
        if code.contains(&format!("class {n} extends")) {
            return Some(n);
        }
    }
    None
}

fn capture_after(hay: &str, prefix: &str, pred: impl Fn(char) -> bool) -> Option<String> {
    let idx = hay.find(prefix)? + prefix.len();
    let name: String = hay[idx..].chars().take_while(|&c| pred(c)).collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn capture_between(hay: &str, start: &str, end: &str) -> Option<String> {
    let s = hay.find(start)? + start.len();
    let e = hay[s..].find(end)? + s;
    Some(hay[s..e].to_string())
}

/// Port of parseChunkResponse (pipeline.ts:117-158). Four strategies.
pub fn parse_chunk_response(response: &str, chunk_id: &str) -> ChunkResult {
    const SEP: &str = "---CONTRACT---";
    if let Some(idx) = response.find(SEP) {
        let code = clean_code(&response[..idx]);
        let contract_str = response[idx + SEP.len()..].trim();
        if let Some(contract) = try_parse_contract(contract_str, chunk_id) {
            return ChunkResult {
                chunk_id: chunk_id.into(),
                code,
                contract,
            };
        }
    }

    // Strategy 2 + 3 keep their staged structure (separator vs trailing
    // object); the nested `if let ... { if ... }` is intentional.
    #[allow(clippy::collapsible_if)]
    if let Some((block, inner)) = find_json_fence(response) {
        if inner.contains("\"componentName\"") {
            if let Some(contract) = try_parse_contract(inner, chunk_id) {
                let block_start = response.find(block).unwrap_or(0);
                let code = clean_code(&response[..block_start]);
                return ChunkResult {
                    chunk_id: chunk_id.into(),
                    code,
                    contract,
                };
            }
        }
    }

    #[allow(clippy::collapsible_if)]
    if let Some(obj) = find_trailing_component_object(response) {
        if let Some(contract) = try_parse_contract(&obj, chunk_id) {
            let json_start = response.rfind(&obj).unwrap_or(0);
            let code = clean_code(&response[..json_start]);
            return ChunkResult {
                chunk_id: chunk_id.into(),
                code,
                contract,
            };
        }
    }

    let code = clean_code(response);
    let contract = infer_contract_from_code(&code, chunk_id);
    ChunkResult {
        chunk_id: chunk_id.into(),
        code,
        contract,
    }
}

/// Find the first ```json ... ``` block; return (full_block, inner_json).
fn find_json_fence(resp: &str) -> Option<(&str, &str)> {
    let open = resp.find("```json")?;
    let after = open + "```json".len();
    let close_rel = resp[after..].find("```")?;
    let close = after + close_rel;
    let inner = resp[after..close].trim_matches(|c| c == '\n' || c == '\r' || c == ' ');
    Some((&resp[open..close + 3], inner))
}

/// Find a trailing `{...}` object containing "componentName" (no nesting).
fn find_trailing_component_object(resp: &str) -> Option<String> {
    let trimmed = resp.trim_end();
    let end = trimmed.rfind('}')?;
    let start = trimmed[..end].rfind('{')?;
    let obj = &trimmed[start..=end];
    if obj.contains("\"componentName\"") && !obj[1..obj.len() - 1].contains('{') {
        Some(obj.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_order_zero_for_no_deps_then_increments() {
        let chunks = vec![
            planned("a", &[]),
            planned("b", &["a"]),
            planned("c", &["a", "b"]),
        ];
        let orders = compute_execution_order(&chunks);
        assert_eq!(orders["a"], 0);
        assert_eq!(orders["b"], 1);
        assert_eq!(orders["c"], 2);
    }

    #[test]
    fn execution_order_breaks_cycles_at_zero() {
        // TS parity (verified against code-generation-pipeline.ts:86-111 run
        // in Node): the cycle guard returns 0 for the back-edge, so the
        // traversal terminates with finite orders instead of recursing
        // forever. Resolving `a` first yields a=2, b=1; both are bounded.
        let chunks = vec![planned("a", &["b"]), planned("b", &["a"])];
        let orders = compute_execution_order(&chunks);
        assert_eq!(orders["a"], 2);
        assert_eq!(orders["b"], 1);
    }

    #[test]
    fn parse_chunk_strategy1_contract_separator() {
        let resp = "export default function Card(){}\n---CONTRACT---\n{\"componentName\":\"Card\"}";
        let r = parse_chunk_response(resp, "c1");
        assert_eq!(r.contract.component_name, "Card");
        assert_eq!(r.contract.chunk_id, "c1");
        assert!(r.code.contains("function Card"));
        assert!(!r.code.contains("CONTRACT"));
    }

    #[test]
    fn parse_chunk_strategy2_json_fence() {
        let resp = "code here\n```json\n{\"componentName\":\"Hero\"}\n```";
        let r = parse_chunk_response(resp, "c2");
        assert_eq!(r.contract.component_name, "Hero");
        assert!(r.code.contains("code here"));
        assert!(!r.code.contains("componentName"));
    }

    #[test]
    fn parse_chunk_strategy4_infer_from_export() {
        let resp = "```tsx\nexport default function Footer() { return null }\n```";
        let r = parse_chunk_response(resp, "c4");
        assert_eq!(r.contract.component_name, "Footer");
        assert!(!r.code.contains("```"));
    }

    #[test]
    fn clean_code_strips_fences() {
        assert_eq!(clean_code("```ts\nlet x = 1;\n```"), "let x = 1;");
    }

    #[test]
    fn validate_contract_rejects_non_pascal_and_missing_name() {
        let bad = ChunkResult {
            chunk_id: "x".into(),
            code: "export default function card(){}".into(),
            contract: ChunkContract {
                component_name: "card".into(),
                ..Default::default()
            },
        };
        assert!(!validate_contract(&bad).0);
        let ok = ChunkResult {
            chunk_id: "x".into(),
            code: "export default function Card(){ return null }".into(),
            contract: ChunkContract {
                component_name: "Card".into(),
                ..Default::default()
            },
        };
        assert!(validate_contract(&ok).0);
    }

    #[test]
    fn validate_contract_allows_sfc_without_name_in_code() {
        let sfc = ChunkResult {
            chunk_id: "x".into(),
            code: "<template><div/></template><script>export default {}</script>".into(),
            contract: ChunkContract {
                component_name: "Widget".into(),
                ..Default::default()
            },
        };
        assert!(validate_contract(&sfc).0);
    }

    #[test]
    fn sanitize_name_pascal_cases() {
        assert_eq!(sanitize_name("nav bar"), "NavBar");
        assert_eq!(sanitize_name("hero-section"), "HeroSection");
    }

    #[test]
    fn extract_plan_json_finds_object() {
        let resp = "Here is the plan:\n{\"chunks\":[],\"rootLayout\":{}}\nDone.";
        let json = extract_plan_json(resp).unwrap();
        assert!(json.starts_with('{') && json.ends_with('}'));
    }

    fn planned(id: &str, deps: &[&str]) -> crate::ai::types::PlannedChunk {
        crate::ai::types::PlannedChunk {
            id: id.into(),
            name: id.into(),
            node_ids: vec![],
            role: String::new(),
            suggested_component_name: id.into(),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
        }
    }
}

//! Element Manifest v2 — programmatic facade over the semantic element
//! builders (`semantic_alias_node`) for the built-in agent's manifest
//! pipeline. Spec: openpencil-docs/superpowers/specs/2026-06-10-element-manifest-v2.md.
//!
//! Three responsibilities, all in front of the existing builders so the
//! 188 `add_*_v0/v1` MCP aliases stay byte-identical for external
//! clients:
//!
//! 1. **Kind normalization** — manifest lines say `"el":"stat_card"`;
//!    tolerate `add_` prefixes, `_vN` suffixes, hyphens and case, and
//!    resolve to the canonical alias (preferring the theme-aware `_v1`).
//! 2. **Repairing argument layer** — instead of rejecting model
//!    hallucinations (the dominant ab-v8 failure mode), repair them:
//!    invalid enums map through a synonym table, missing required
//!    params get schema-driven placeholders, and every repair emits a
//!    warning so the miss stays observable.
//! 3. **Second-chance retry** — when a builder still reports
//!    `<key> is required` / `<key> must be a JSON array` (schema and
//!    builder disagree about what's required), synthesize that one arg
//!    and retry, bounded to avoid loops.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use jian_ops_schema::node::PenNode;
use serde_json::Value;

use crate::element_alias_builders::semantic_alias_node;
use crate::element_tools::TS_ELEMENT_ALIASES;
use crate::element_ts_schema::ts_alias_schema;
use crate::ToolOutcome;

/// A successfully built element subtree.
#[derive(Debug)]
pub struct ElementBuild {
    /// Canonical alias the kind resolved to (e.g. `add_stat_card_v1`).
    pub tool: String,
    pub node: PenNode,
    /// Repair trail (`W-ENUM` / `W-PLACEHOLDER` / `W-RESCUE` entries).
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub enum ElementBuildError {
    /// The kind doesn't resolve to any known element alias.
    UnknownKind(String),
    /// The builder rejected the (already repaired) arguments.
    BuildFailed { tool: String, message: String },
}

impl std::fmt::Display for ElementBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementBuildError::UnknownKind(kind) => write!(f, "unknown element kind {kind:?}"),
            ElementBuildError::BuildFailed { tool, message } => {
                write!(f, "{tool} failed: {message}")
            }
        }
    }
}

/// Build one element subtree from a manifest line's kind + args.
pub fn build_element(
    kind: &str,
    args: &BTreeMap<String, String>,
) -> Result<ElementBuild, ElementBuildError> {
    let Some(tool) = resolve_element_kind(kind) else {
        return Err(ElementBuildError::UnknownKind(kind.to_string()));
    };
    let mut warnings = Vec::new();
    let mut repaired = repair_args(tool, args, &mut warnings);

    // Bounded second-chance loop: a builder may require args the TS
    // schema doesn't list as required (or vice versa). Its error names
    // the missing key — synthesize just that one and retry.
    let mut rescued: BTreeSet<String> = BTreeSet::new();
    loop {
        let message = match semantic_alias_node(tool, &repaired) {
            Ok(Some(node)) => {
                return Ok(ElementBuild {
                    tool: tool.to_string(),
                    node,
                    warnings,
                })
            }
            Ok(None) => "no semantic builder for alias".to_string(),
            Err(ToolOutcome::Err(_, message)) => message,
            Err(_) => "unexpected tool outcome".to_string(),
        };
        let Some(key) = rescuable_key(&message) else {
            return Err(ElementBuildError::BuildFailed {
                tool: tool.to_string(),
                message,
            });
        };
        if rescued.len() >= 8 || !rescued.insert(key.clone()) {
            return Err(ElementBuildError::BuildFailed {
                tool: tool.to_string(),
                message,
            });
        }
        let placeholder = synthesize_placeholder(tool, &key, schema_prop(tool, &key).as_ref());
        warnings.push(format!(
            "W-RESCUE {tool}.{key}: builder reported {message:?} — filled {placeholder:?}"
        ));
        repaired.insert(key, placeholder);
    }
}

/// `<key> is required` / `<key> must be a JSON array: …` → `key`.
fn rescuable_key(message: &str) -> Option<String> {
    if let Some(key) = message.strip_suffix(" is required") {
        let key = key.trim();
        if is_identifier(key) {
            return Some(key.to_string());
        }
    }
    if let Some(idx) = message.find(" must be a JSON array") {
        let key = message[..idx].trim();
        if is_identifier(key) {
            return Some(key.to_string());
        }
    }
    None
}

fn is_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Resolve a manifest kind to the canonical alias name, preferring the
/// theme-aware `_v1` and falling back to `_v0`.
pub fn resolve_element_kind(raw: &str) -> Option<&'static str> {
    let base = canonical_base(raw);
    if base.is_empty() {
        return None;
    }
    let v1 = format!("add_{base}_v1");
    if let Some(name) = TS_ELEMENT_ALIASES.iter().find(|name| **name == v1) {
        return Some(name);
    }
    let v0 = format!("add_{base}_v0");
    TS_ELEMENT_ALIASES.iter().find(|name| **name == v0).copied()
}

/// Lowercase, `-`/space → `_`, strip `add_` prefix and `_v<digits>` suffix.
fn canonical_base(raw: &str) -> String {
    let mut base = raw.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    if let Some(stripped) = base.strip_prefix("add_") {
        base = stripped.to_string();
    }
    if let Some(pos) = base.rfind("_v") {
        let suffix = &base[pos + 2..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            base.truncate(pos);
        }
    }
    base
}

/// All base kinds (version suffix stripped, deduped) — the manifest
/// prompt catalog generator iterates this.
pub fn known_element_kinds() -> Vec<String> {
    let mut kinds: Vec<String> = TS_ELEMENT_ALIASES
        .iter()
        .map(|name| canonical_base(name))
        .collect();
    kinds.sort();
    kinds.dedup();
    kinds
}

/// Compact per-kind signature catalog for the manifest prompt skill —
/// one line per kind, e.g. `- stat_card: label*, value*, trend(up|down|flat), delta, icon, width`.
/// Generated from the embedded TS schemas so the prompt can never drift
/// from the builders. Wire-level params (schemaVersion / filePath /
/// parent_id / pageId) and the universal `theme` param are omitted —
/// the skill body documents `theme` once.
pub fn manifest_catalog() -> String {
    known_element_kinds()
        .iter()
        .filter_map(|kind| resolve_element_kind(kind).map(|tool| catalog_line(kind, tool)))
        .collect::<Vec<_>>()
        .join("\n")
}

const CATALOG_SKIP_PARAMS: &[&str] = &[
    "schemaVersion",
    "filePath",
    "parent_id",
    "parentId",
    "pageId",
    "page_id",
    "theme",
];

fn catalog_line(kind: &str, tool: &str) -> String {
    let Some(schema) = ts_alias_schema(tool).and_then(|s| serde_json::from_str::<Value>(&s).ok())
    else {
        return format!("- {kind}");
    };
    let input = &schema["inputSchema"];
    let Some(props) = input.get("properties").and_then(Value::as_object) else {
        return format!("- {kind}");
    };
    let required: Vec<&str> = input
        .get("required")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut params = Vec::new();
    for key in &required {
        if CATALOG_SKIP_PARAMS.contains(key) {
            continue;
        }
        if let Some(prop) = props.get(*key) {
            params.push(render_catalog_param(key, prop, true));
        }
    }
    for (key, prop) in props {
        if CATALOG_SKIP_PARAMS.contains(&key.as_str()) || required.contains(&key.as_str()) {
            continue;
        }
        params.push(render_catalog_param(key, prop, false));
    }
    if params.is_empty() {
        format!("- {kind}")
    } else {
        format!("- {kind}: {}", params.join(", "))
    }
}

fn render_catalog_param(key: &str, prop: &Value, required: bool) -> String {
    let star = if required { "*" } else { "" };
    if let Some(values) = prop.get("enum").and_then(Value::as_array) {
        let values: Vec<&str> = values.iter().filter_map(Value::as_str).collect();
        if !values.is_empty() {
            return format!("{key}{star}({})", values.join("|"));
        }
    }
    match prop.get("type").and_then(Value::as_str) {
        Some("array") => {
            let fields: Vec<String> = prop
                .get("items")
                .and_then(|items| items.get("properties"))
                .and_then(Value::as_object)
                .map(|props| props.keys().cloned().collect())
                .unwrap_or_default();
            if fields.is_empty() {
                format!("{key}{star}:[..]")
            } else {
                format!("{key}{star}:[{{{}}}]", fields.join(","))
            }
        }
        _ => format!("{key}{star}"),
    }
}

/// Parsed inputSchema property for `tool.key`, if the TS schema knows it.
fn schema_prop(tool: &str, key: &str) -> Option<Value> {
    let schema: Value = serde_json::from_str(&ts_alias_schema(tool)?).ok()?;
    schema
        .get("inputSchema")?
        .get("properties")?
        .get(key)
        .cloned()
}

/// Schema-driven argument repair: enum synonym mapping + placeholders
/// for missing required params. Never rejects — every change is logged.
fn repair_args(
    tool: &str,
    args: &BTreeMap<String, String>,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, String> {
    let mut out = args.clone();
    // Manifest default theme: `light` (spec D2, revised by the 2026-06-10
    // smoke). `system` emits `$color-*` refs, but the Rust pipeline has no
    // semantic-palette seeding yet (`variables::seed_commands` is dormant)
    // — un-seeded refs render as grey fallbacks. Until seeding lands,
    // default to the v0-parity hex palette; `system` stays opt-in for
    // documents that carry a palette.
    if tool.ends_with("_v1") && !out.contains_key("theme") {
        out.insert("theme".to_string(), "light".to_string());
    }
    let Some(schema) = ts_alias_schema(tool).and_then(|s| serde_json::from_str::<Value>(&s).ok())
    else {
        return out;
    };
    let input = &schema["inputSchema"];
    let Some(props) = input.get("properties").and_then(Value::as_object) else {
        return out;
    };

    for (key, prop) in props {
        if let Some(allowed) = prop.get("enum").and_then(Value::as_array) {
            let allowed: Vec<&str> = allowed.iter().filter_map(Value::as_str).collect();
            if allowed.is_empty() {
                continue;
            }
            let Some(given) = out.get(key) else { continue };
            if allowed.contains(&given.as_str()) {
                continue;
            }
            let fixed = repair_enum_value(given, &allowed);
            warnings.push(format!("W-ENUM {tool}.{key}: {given:?} → {fixed:?}"));
            out.insert(key.clone(), fixed);
            continue;
        }
        // Shape coercion: the model handed a plain string where the
        // schema wants JSON (smoke-observed: `action: "View All"` on
        // section_header). Wrap objects as `{label}`, split arrays on
        // commas — preserving the model's intent beats a placeholder.
        let ty = prop.get("type").and_then(Value::as_str).unwrap_or("");
        if ty != "object" && ty != "array" {
            continue;
        }
        let Some(given) = out.get(key) else { continue };
        let trimmed = given.trim();
        if trimmed.is_empty() || serde_json::from_str::<Value>(trimmed).is_ok() {
            continue;
        }
        let coerced = if ty == "object" {
            coerce_string_to_object(trimmed, prop)
        } else {
            let items: Vec<Value> = trimmed
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| Value::String(s.to_string()))
                .collect();
            serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
        };
        warnings.push(format!("W-COERCED {tool}.{key}: {given:?} → {coerced}"));
        out.insert(key.clone(), coerced);
    }

    let required: Vec<&str> = input
        .get("required")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for key in required {
        let missing = out.get(key).map(|v| v.trim().is_empty()).unwrap_or(true);
        if !missing {
            continue;
        }
        // Wire-level args are not content — they're never placeholder
        // material. parent_id/pageId are manifest-forbidden anyway.
        match key {
            "schemaVersion" => {
                out.insert(key.to_string(), "1.0".to_string());
                continue;
            }
            "filePath" | "parent_id" | "parentId" | "pageId" | "page_id" => continue,
            _ => {}
        }
        let placeholder = synthesize_placeholder(tool, key, props.get(key));
        warnings.push(format!(
            "W-PLACEHOLDER {tool}.{key}: missing required — filled {placeholder:?}"
        ));
        out.insert(key.to_string(), placeholder);
    }
    out
}

/// Build a schema-shaped object from a plain string (smoke-observed:
/// `trailing: "Owner"` where the builder wants `{kind, value}`). Enum
/// props pick the given value when it matches, else a text-bearing
/// variant (`badge`/`value`/`text`/`label`) when available, else the
/// first allowed; the first plain string prop carries the given text.
fn coerce_string_to_object(given: &str, prop: &Value) -> String {
    let mut out = serde_json::Map::new();
    if let Some(props) = prop.get("properties").and_then(Value::as_object) {
        let mut text_used = false;
        for (key, sub) in props {
            if let Some(allowed) = sub.get("enum").and_then(Value::as_array) {
                let allowed: Vec<&str> = allowed.iter().filter_map(Value::as_str).collect();
                if allowed.is_empty() {
                    continue;
                }
                let pick = allowed
                    .iter()
                    .find(|a| a.eq_ignore_ascii_case(given))
                    .copied()
                    .or_else(|| {
                        ["badge", "value", "text", "label"]
                            .iter()
                            .find(|cand| allowed.contains(*cand))
                            .copied()
                    })
                    .unwrap_or(allowed[0]);
                out.insert(key.clone(), Value::String(pick.to_string()));
            } else if !text_used
                && sub.get("type").and_then(Value::as_str).unwrap_or("string") == "string"
            {
                out.insert(key.clone(), Value::String(given.to_string()));
                text_used = true;
            }
        }
    }
    if out.is_empty() {
        out.insert("label".to_string(), Value::String(given.to_string()));
    }
    Value::Object(out).to_string()
}

/// Map an out-of-enum value to the closest allowed one: exact
/// case-insensitive match, then a synonym table, then `"default"` when
/// allowed, then the first allowed value.
fn repair_enum_value(given: &str, allowed: &[&str]) -> String {
    let lower = given.trim().to_ascii_lowercase();
    if let Some(hit) = allowed.iter().find(|a| a.eq_ignore_ascii_case(&lower)) {
        return hit.to_string();
    }
    const SYNONYMS: &[(&str, &str)] = &[
        ("info", "default"),
        ("neutral", "default"),
        ("secondary", "default"),
        ("muted", "default"),
        ("normal", "default"),
        ("regular", "default"),
        ("danger", "error"),
        ("destructive", "error"),
        ("critical", "error"),
        ("negative", "error"),
        ("alert", "warning"),
        ("caution", "warning"),
        ("positive", "success"),
        ("ok", "success"),
        ("good", "success"),
        ("primary", "accent"),
        ("brand", "accent"),
    ];
    for (from, to) in SYNONYMS {
        if lower == *from && allowed.contains(to) {
            return to.to_string();
        }
    }
    if allowed.contains(&"default") {
        return "default".to_string();
    }
    allowed.first().copied().unwrap_or("").to_string()
}

/// Synthesize a placeholder value for `tool.key` from its schema shape.
fn synthesize_placeholder(tool: &str, key: &str, prop: Option<&Value>) -> String {
    if let Some(first) = prop
        .and_then(|p| p.get("enum"))
        .and_then(Value::as_array)
        .and_then(|allowed| allowed.iter().filter_map(Value::as_str).next())
    {
        return first.to_string();
    }
    let ty = prop
        .and_then(|p| p.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("string");
    match ty {
        "array" => placeholder_array(prop),
        "number" | "integer" => placeholder_number(key).to_string(),
        "boolean" => "false".to_string(),
        _ => placeholder_string(tool, key),
    }
}

/// Placeholder JSON array string per the item schema: numbers get demo
/// chart data; objects get 3 rows carrying every known item field (the
/// union keeps builder-side per-item validation satisfied even when the
/// TS schema under-declares); plain strings get `Item N`.
fn placeholder_array(prop: Option<&Value>) -> String {
    let items = prop.and_then(|p| p.get("items"));
    let item_ty = items
        .and_then(|i| i.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("string");
    match item_ty {
        "number" | "integer" => "[12, 18, 9, 22, 16, 25]".to_string(),
        "object" => {
            let mut fields: BTreeSet<String> =
                ["label", "value", "icon", "content", "name", "title"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            if let Some(required) = items
                .and_then(|i| i.get("required"))
                .and_then(Value::as_array)
            {
                fields.extend(required.iter().filter_map(Value::as_str).map(String::from));
            }
            if let Some(props) = items
                .and_then(|i| i.get("properties"))
                .and_then(Value::as_object)
            {
                fields.extend(props.keys().cloned());
            }
            let rows: Vec<Value> = (1..=3)
                .map(|i| {
                    Value::Object(
                        fields
                            .iter()
                            .map(|field| {
                                (
                                    field.clone(),
                                    Value::String(placeholder_item_field(field, i)),
                                )
                            })
                            .collect(),
                    )
                })
                .collect();
            serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
        }
        _ => r#"["Item 1", "Item 2", "Item 3"]"#.to_string(),
    }
}

fn placeholder_item_field(field: &str, i: usize) -> String {
    let lower = field.to_ascii_lowercase();
    if lower == "icon" {
        return "circle".to_string();
    }
    if lower == "value" || lower.contains("count") {
        return format!("{}", 10 * i + 2);
    }
    if lower.contains("subtitle") || lower.contains("description") {
        return format!("Placeholder description {i}");
    }
    format!("Item {i}")
}

fn placeholder_number(key: &str) -> i64 {
    let lower = key.to_ascii_lowercase();
    if lower.contains("width") {
        240
    } else if lower.contains("height") {
        120
    } else if lower.contains("size") {
        24
    } else if lower.contains("percent") || lower.contains("progress") || lower.contains("value") {
        60
    } else {
        3
    }
}

fn placeholder_string(tool: &str, key: &str) -> String {
    let lower = key.to_ascii_lowercase();
    if lower == "icon" {
        return "circle".to_string();
    }
    if lower.contains("avatar") || lower == "initials" {
        return "AB".to_string();
    }
    if lower.contains("price") || lower.contains("amount") {
        return "$29".to_string();
    }
    if lower == "value" {
        return "42".to_string();
    }
    if lower.contains("date") {
        return "Jun 10".to_string();
    }
    if lower.contains("time") {
        return "12:30".to_string();
    }
    if lower.contains("url") || lower.contains("href") || lower.contains("link") {
        return "#".to_string();
    }
    if lower.contains("placeholder") || lower.contains("query") {
        return "Search".to_string();
    }
    if lower.contains("label")
        || lower.contains("title")
        || lower.contains("heading")
        || lower.contains("name")
    {
        return humanize_kind(tool);
    }
    "Placeholder copy".to_string()
}

/// `add_stat_card_v1` → `Stat Card`.
fn humanize_kind(tool: &str) -> String {
    canonical_base(tool)
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn kind_normalization_resolves_aliases() {
        // Bare kind prefers the theme-aware v1.
        assert_eq!(resolve_element_kind("stat_card"), Some("add_stat_card_v1"));
        // Tolerates add_ prefix, version suffix, hyphens, case.
        assert_eq!(
            resolve_element_kind("add_stat_card_v0"),
            Some("add_stat_card_v1")
        );
        assert_eq!(resolve_element_kind("Stat-Card"), Some("add_stat_card_v1"));
        assert_eq!(resolve_element_kind("nonexistent_widget"), None);
    }

    #[test]
    fn invalid_enum_is_repaired_with_warning() {
        let built = build_element("tag", &args(&[("label", "Beta"), ("tone", "info")]))
            .expect("tag with invalid tone must repair, not reject");
        assert!(
            built.warnings.iter().any(|w| w.starts_with("W-ENUM")),
            "expected a W-ENUM warning, got {:?}",
            built.warnings
        );
    }

    #[test]
    fn missing_required_args_get_placeholders() {
        let built = build_element("stat_card", &BTreeMap::new())
            .expect("stat_card with no args must build via placeholders");
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.starts_with("W-PLACEHOLDER") || w.starts_with("W-RESCUE")),
            "expected placeholder warnings, got {:?}",
            built.warnings
        );
    }

    #[test]
    fn v1_kinds_default_to_light_theme() {
        // `system` becomes the default once semantic-palette seeding
        // lands; un-seeded `$color-*` refs render grey (2026-06-10 smoke).
        let mut warnings = Vec::new();
        let repaired = repair_args("add_stat_card_v1", &BTreeMap::new(), &mut warnings);
        assert_eq!(repaired.get("theme").map(String::as_str), Some("light"));
    }

    #[test]
    fn explicit_theme_is_preserved() {
        let mut warnings = Vec::new();
        let repaired = repair_args(
            "add_stat_card_v1",
            &args(&[("theme", "dark")]),
            &mut warnings,
        );
        assert_eq!(repaired.get("theme").map(String::as_str), Some("dark"));
    }

    #[test]
    fn plain_string_coerces_to_expected_json_shape() {
        // 冒烟实测:M2.7 把 section_header 的 `action` 写成纯字符串。
        let built = build_element(
            "section_header",
            &args(&[("title", "Revenue"), ("action", "View All")]),
        )
        .expect("plain-string action must coerce, not reject");
        assert!(
            built.warnings.iter().any(|w| w.starts_with("W-COERCED")),
            "expected W-COERCED, got {:?}",
            built.warnings
        );
    }

    #[test]
    fn plain_string_object_coerces_to_schema_shape() {
        // ab-v9 实测:member_row 的 trailing 传纯字符串 "Owner" —— 需按
        // schema 形状构造 {kind:"badge", value:"Owner"} 而非 {label}。
        let built = build_element(
            "member_row",
            &args(&[("name", "Sarah Lee"), ("trailing", "Owner")]),
        )
        .expect("plain-string trailing must coerce to schema shape");
        assert!(
            built.warnings.iter().any(|w| w.starts_with("W-COERCED")),
            "{:?}",
            built.warnings
        );
    }

    #[test]
    fn catalog_covers_every_kind_compactly() {
        let catalog = manifest_catalog();
        let kinds = known_element_kinds();
        assert_eq!(catalog.lines().count(), kinds.len());
        assert!(
            catalog.lines().any(|line| line.starts_with("- stat_card:")),
            "stat_card line present"
        );
        // Version suffixes and wire params never leak into the prompt.
        assert!(!catalog.contains("_v0"), "no version suffixes");
        assert!(!catalog.contains("_v1"), "no version suffixes");
        assert!(!catalog.contains("schemaVersion"));
        assert!(!catalog.contains("parent_id"));
        assert!(
            !catalog.contains("theme"),
            "theme documented once, not per line"
        );
        // Keep the catalog inside the skill's token budget (~2.4k tokens
        // total for the skill; the catalog must stay well under it).
        assert!(
            catalog.len() < 9000,
            "catalog grew past its prompt budget: {} chars",
            catalog.len()
        );
    }

    /// The manifest contract: every known kind must build from EMPTY
    /// args — the repair layer owns required-param synthesis. This is
    /// the "never reject a well-formed manifest line" guarantee.
    #[test]
    fn every_known_kind_builds_from_empty_args() {
        let mut failures = Vec::new();
        for kind in known_element_kinds() {
            if let Err(err) = build_element(&kind, &BTreeMap::new()) {
                failures.push(format!("{kind}: {err}"));
            }
        }
        assert!(
            failures.is_empty(),
            "{} kind(s) failed to build from empty args:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    /// 收集子树里所有布局层不解析的 expression gap/padding。Rust 布局链
    /// 把 Expression 归零(jian-core `container_to_style` 与 op-pen-loader
    /// `gap_value` 的 `_ => 0` 臂),所以任何 `$spacing-*` ref 都等于
    /// gap/padding 直接消失。
    fn collect_expression_layout_refs(node: &PenNode, kind: &str, hits: &mut Vec<String>) {
        use jian_ops_schema::node::base::NumberOrExpression;
        use jian_ops_schema::node::container::Padding;
        let (container, children) = match node {
            PenNode::Frame(f) => (Some(&f.container), f.children.as_deref()),
            PenNode::Group(g) => (Some(&g.container), g.children.as_deref()),
            PenNode::Rectangle(r) => (Some(&r.container), None),
            _ => (None, None),
        };
        if let Some(c) = container {
            if let Some(NumberOrExpression::Expression(expr)) = c.gap.as_ref() {
                hits.push(format!("{kind}: gap = {expr:?}"));
            }
            if let Some(Padding::Expression(expr)) = c.padding.as_ref() {
                hits.push(format!("{kind}: padding = {expr:?}"));
            }
        }
        for child in children.unwrap_or(&[]) {
            collect_expression_layout_refs(child, kind, hits);
        }
    }

    /// system 模式布局不变量:任何 kind 在 theme=system 下都不得发
    /// `$spacing-*` 等 expression gap/padding(2026-06-13 审计:曾有
    /// modal_shell 的 padding/gap 与 toast 的 gap 三处,system 模式下
    /// 卡片/胶囊直接坍缩)。颜色 `$color-*` ref 不在此列——scene_vars
    /// 的 VariableTable 在 paint 期解析 fill/stroke ref。
    #[test]
    fn system_mode_never_emits_expression_gap_or_padding() {
        let mut hits = Vec::new();
        for kind in known_element_kinds() {
            let built = build_element(&kind, &args(&[("theme", "system")]))
                .unwrap_or_else(|err| panic!("{kind} must build in system mode: {err:?}"));
            collect_expression_layout_refs(&built.node, &kind, &mut hits);
        }
        assert!(
            hits.is_empty(),
            "{} expression gap/padding emission(s) — the layout chain zeroes these:\n{}",
            hits.len(),
            hits.join("\n")
        );
    }
}

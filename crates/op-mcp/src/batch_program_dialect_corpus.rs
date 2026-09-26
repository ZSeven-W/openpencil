//! Opt-in measurement: replay the `[program-gen] dropped line` corpus through
//! the node-body parser before and after the dialect repairs.
//!
//! `OP_DIALECT_CORPUS=<file> cargo test -p op-mcp dialect_corpus -- --ignored
//! --nocapture`, where the file holds `grep -H "[program-gen] dropped line"`
//! output from run stderr logs. Logged lines are previews capped at 200
//! chars: a truncated body is cut back to its last complete top-level field
//! and closed, so a failure that sat in the lost tail is reported as
//! "undecidable" instead of being credited to either side.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use serde::Deserialize;

use super::batch_design::{ensure_node_ids, normalize_node_shape};
use super::batch_program_node_parse::{parse_node_body, NodeParseOptions};
use super::batch_program_parse::parse_json_arg;

struct Dropped {
    run: String,
    binding: Option<String>,
    body: Option<String>,
    truncated: bool,
    reason: String,
}

/// For a truncated preview whose failure sat in the lost tail, a minimal
/// field that reproduces the LOGGED error — so the category is still
/// measured, on a reconstruction. `None` = the error names no field.
fn reconstructed_tail(reason: &str) -> Option<String> {
    if let Some(rest) = reason.split("invalid type: string \"").nth(1) {
        let handle = rest.split('"').next()?;
        return Some(format!("\"children\":[\"{handle}\"]"));
    }
    if reason.contains("unknown variant `center`, expected one of `top`") {
        return Some("\"textAlignVertical\":\"center\"".into());
    }
    if reason.contains("unknown variant `press`") {
        return Some(
            "\"animations\":[{\"trigger\":\"press\",\"keyframes\":{\"from\":{\"scale\":1},\"to\":{\"scale\":0.97}},\"durationMs\":120}]"
                .into(),
        );
    }
    if reason.contains("sequence, expected struct EventHandlers") {
        return Some("\"events\":[{\"onTap\":[]}]".into());
    }
    if reason.contains("invalid type: null, expected internally tagged enum PenNode") {
        return Some("\"children\":[null]".into());
    }
    // The next two errors do not name their field. The probes stand in for
    // "some optional field holds a map": only the best-effort field salvage
    // can accept them, and it would accept any such non-protected field.
    if reason.ends_with("invalid type: map, expected a string") {
        return Some("\"returnKeyHint\":{\"probe\":1}".into());
    }
    if reason.ends_with("invalid type: map, expected a sequence") {
        return Some("\"slot\":{\"probe\":1}".into());
    }
    None
}

fn parse_row(row: &str) -> Option<Dropped> {
    let (run, rest) = row.split_once(":[program-gen] dropped line `")?;
    let (line, reason) = rest.rsplit_once("`: ")?;
    let (binding, call) = match line.split_once('=') {
        Some((b, c)) if b.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') => {
            (Some(b.trim().to_string()), c.trim())
        }
        _ => (None, line.trim()),
    };
    let args = call
        .strip_prefix("I(")
        .and_then(|args| args.split_once(',').map(|(_, body)| body.trim()));
    let truncated = args.is_some_and(|body| body.ends_with("..."));
    let body = args.map(|body| match body.strip_suffix("...") {
        Some(cut) => close_truncated(cut),
        None => body.strip_suffix(')').unwrap_or(body).to_string(),
    });
    Some(Dropped {
        run: run.to_string(),
        binding,
        body,
        truncated,
        reason: reason.to_string(),
    })
}

/// Cut a truncated object body back to its last complete top-level field.
fn close_truncated(cut: &str) -> String {
    let (mut depth, mut in_string, mut escape, mut last) = (0i32, false, false, None);
    for (index, ch) in cut.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' | '[' if !in_string => depth += 1,
            '}' | ']' if !in_string => depth -= 1,
            ',' if !in_string && depth == 1 => last = Some(index),
            _ => {}
        }
    }
    match last {
        Some(index) => format!("{}}}", &cut[..index]),
        None => "{}".to_string(),
    }
}

/// The node-body path as it was before the dialect repairs.
fn parses_before(body: &str) -> bool {
    let Ok(mut value) = parse_json_arg(body) else {
        return false;
    };
    if !value.is_object() {
        return false;
    }
    normalize_node_shape(&mut value);
    ensure_node_ids(&mut value, &mut 1usize);
    PenNode::deserialize(&value).is_ok()
}

fn parses_after(body: &str) -> bool {
    parse_node_body(
        body,
        NodeParseOptions {
            handle_refs: true,
            salvage_fields: true,
            ..NodeParseOptions::default()
        },
    )
    .is_ok()
}

fn category(reason: &str) -> String {
    if let Some(rest) = reason.split("unknown variant `").nth(1) {
        return format!("unknown variant {}", rest.split('`').next().unwrap_or(""));
    }
    if reason.starts_with("Insert parent not found") {
        return "cascade (parent not found)".into();
    }
    if reason.contains("invalid type: string \"") {
        return "string children".into();
    }
    reason
        .trim_start_matches("invalid PenNode payload: ")
        .split(" at line")
        .next()
        .unwrap_or(reason)
        .to_string()
}

/// `Some(Some(2))` accepted as logged, `Some(Some(1))` accepted on a
/// reconstructed tail, `Some(None)` still rejected, `None` undecidable.
fn recover(body: &str, truncated: bool, reason: &str) -> Option<Option<usize>> {
    if !parses_before(body) {
        return Some(parses_after(body).then_some(2));
    }
    if !truncated {
        // Complete body the old parser now accepts: fixed before this change.
        return None;
    }
    // The prefix parses: the logged failure was in the lost tail.
    let tail = reconstructed_tail(reason)?;
    let open = body.strip_suffix('}').unwrap_or(body);
    let sep = if open.trim_end().ends_with('{') {
        ""
    } else {
        ","
    };
    let rebuilt = format!("{open}{sep}{tail}}}");
    if parses_before(&rebuilt) {
        return None;
    }
    Some(parses_after(&rebuilt).then_some(1))
}

#[test]
#[ignore = "measurement over an external corpus; set OP_DIALECT_CORPUS"]
fn dialect_corpus_replay() {
    let Ok(path) = std::env::var("OP_DIALECT_CORPUS") else {
        return;
    };
    let text = std::fs::read_to_string(path).expect("corpus readable");
    // category -> [total, accepted on a reconstructed tail, accepted on the
    // logged body, undecidable]
    let mut table: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    // run -> binding -> column its most recent drop recovered in (1 =
    // reconstructed tail, 2 = logged body), `None` = still dropped.
    let mut dropped: BTreeMap<String, BTreeMap<String, Option<usize>>> = BTreeMap::new();
    for row in text.lines().filter_map(parse_row) {
        let cat = category(&row.reason);
        let recovered = if cat.starts_with("cascade") {
            // A cascade recovers exactly when the parent's own dropped line
            // does, and inherits how certain that recovery is; a parent that
            // was never dropped (a leaf used as a parent) stays unrecovered.
            let parent = row.reason.rsplit(": ").next().unwrap_or_default();
            dropped
                .get(&row.run)
                .and_then(|runs| runs.get(parent))
                .copied()
                .flatten()
        } else {
            row.body
                .as_deref()
                .and_then(|body| recover(body, row.truncated, &row.reason))
                .unwrap_or(Some(3))
        };
        let entry = table.entry(cat).or_default();
        entry[0] += 1;
        let recovered = match recovered {
            Some(3) => {
                entry[3] += 1;
                None
            }
            Some(column) => {
                entry[column] += 1;
                Some(column)
            }
            None => None,
        };
        if let Some(binding) = row.binding {
            dropped
                .entry(row.run)
                .or_default()
                .insert(binding, recovered);
        }
    }
    println!(
        "category | total | accepted (logged body) | accepted (reconstructed tail) | undecidable"
    );
    for (cat, [total, rebuilt, after, undecidable]) in &table {
        println!("{cat} | {total} | {after} | {rebuilt} | {undecidable}");
    }
    let sum = |i: usize| table.values().map(|v| v[i]).sum::<usize>();
    println!("TOTAL | {} | {} | {} | {}", sum(0), sum(2), sum(1), sum(3));
}

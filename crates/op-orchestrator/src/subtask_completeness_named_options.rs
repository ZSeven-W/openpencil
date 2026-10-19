//! Explicit named specification choices must exist before layout-only repair.

use super::{local_item_text, parse_count, CompletenessFailure};
use crate::plan::Subtask;
use crate::types::{DocSink, SubtaskOutcome};
use jian_ops_schema::node::{PenNode, TextContent};
use op_design_lint::node_util::{is_node_visible, opacity};
use op_editor_core::PenNodeExt;
use regex::Regex;
use std::collections::BTreeSet;
use std::sync::LazyLock;

#[derive(Debug)]
struct Contract {
    labels: Vec<String>,
    heading: Option<String>,
}

#[cfg(test)]
#[path = "subtask_completeness_prose_options_tests.rs"]
mod prose_tests;

/// Return named delivery evidence, or a precise failure before generic counts
/// can substitute unrelated steppers, repeated labels, or empty frames.
pub(super) fn inspect(
    sink: &dyn DocSink,
    subtask: &Subtask,
    outcome: &SubtaskOutcome,
) -> Result<usize, CompletenessFailure> {
    let contracts = contracts(subtask);
    if contracts.is_empty() {
        return Ok(0);
    }
    let state = sink.state();
    // Use the canvas's actual instance/variable semantics, including overrides
    // and hidden ancestors outside the inserted roots. Never inspect raw slots.
    let mut nodes = op_editor_core::ref_resolve::resolve_refs_for_canvas_roots(
        state.active_children(),
        &state.doc,
    );
    op_editor_core::variables_resolve::resolve_roots_for_canvas(
        &mut nodes,
        &state.doc,
        &state.ui.variables.active_theme,
    );
    let mut largest_complete = 0;
    for contract in contracts {
        let mut evidence = Evidence::default();
        inspect_siblings(
            &nodes,
            &outcome.inserted_root_ids,
            false,
            &contract,
            false,
            &mut evidence,
        );
        if evidence.uncertain_complete && evidence.best.len() < contract.labels.len() {
            // Runtime content/visibility belongs to the existing binding gate.
            // Unknown expressions supply neither named evidence nor a new failure.
            continue;
        }
        let best = evidence.best;
        if best.len() < contract.labels.len() {
            let missing: Vec<_> = contract
                .labels
                .iter()
                .filter(|label| !best.contains(*label))
                .collect();
            let expected = contract.labels.len();
            let delivered = best.len();
            return Err(CompletenessFailure {
                expected,
                delivered,
                feedback: format!(
                    "The plan asks for {expected} named specification options in this section; only {delivered} were delivered in one visible option group. Missing labels: {}. Regenerate this section with one distinct, visible labeled option for each planned name inside its planned group. Quantity steppers, empty frames, layer names, event strings, and unrelated labels do not satisfy these options. Preserve the other content and layout.",
                    serde_json::to_string(&missing).expect("option names serialize")
                ),
            });
        }
        largest_complete = largest_complete.max(best.len());
    }
    Ok(largest_complete)
}

fn contracts(subtask: &Subtask) -> Vec<Contract> {
    static NAMED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)
            (?:(?P<count>[0-9]+|[一二两三四五六七八九十]+)\s*(?:[个枚项种]\s*)?)?
            (?P<kind>(?:规格|颜色|尺码|尺寸)(?:选项|胶囊|按钮|选择项)
                |(?:color|colour|size|variant|specification)\s+(?:options?|chips?|pills?|buttons?))
            (?:组)?\s*(?:（(?P<cjk>[^（）\n;；]{1,240})）|\((?P<ascii>[^()\n;；]{1,240})\))",
        )
        .expect("valid explicitly named specification contract")
    });
    static HEADING: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"[「『“\"](?P<heading>[^」』”\"\n]{1,32})[」』”\"]\s*(?:分组)?标题\s*[+＋]?\s*$"#,
        )
        .expect("valid explicit option heading")
    });
    // Only an explicit quoted heading plus an exhaustive quoted list counts.
    // Prose such as “color and size selection” still promises no named set.
    static PROSE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"[「『“\"'‘](?P<heading>颜色|容量|尺码|尺寸|规格)[」』”\"'’]\s*[+＋:：]\s*[「『“\"'‘](?P<body>[^」』”\"'’\n;；]{1,240})[」』”\"'’]\s*(?P<kind>(?:胶囊|选项|按钮|选择项)(?:选项|按钮|组)?)"#)
            .expect("valid quoted specification list")
    });
    let raw = format!(
        "{}\n{}",
        subtask.label,
        subtask.elements.as_deref().unwrap_or("")
    );
    let masked = local_item_text(subtask);
    let mut out = Vec::new();
    for captures in NAMED.captures_iter(&masked) {
        let count = captures.name("count");
        let anchor = count.unwrap_or_else(|| captures.name("kind").unwrap());
        let body = captures
            .name("cjk")
            .or_else(|| captures.name("ascii"))
            .unwrap();
        let expected = match count {
            Some(count) => match parse_count(count.as_str()) {
                Some(expected) => Some(expected),
                None => continue,
            },
            None => None,
        };
        let heading = HEADING
            .captures(
                raw[..anchor.start()]
                    .rsplit([';', '；', '\n', '。'])
                    .next()
                    .unwrap_or(""),
            )
            .and_then(|capture| capture.name("heading"))
            .map(|heading| heading.as_str().trim().to_owned());
        if let Some(contract) = named_contract(
            &raw,
            &masked,
            subtask,
            anchor.range(),
            body.range(),
            expected,
            heading,
        ) {
            out.push(contract);
        }
    }
    for captures in PROSE.captures_iter(&raw) {
        if let Some(contract) = named_contract(
            &raw,
            &masked,
            subtask,
            captures.name("kind").unwrap().range(),
            captures.name("body").unwrap().range(),
            None,
            Some(captures.name("heading").unwrap().as_str().to_owned()),
        ) {
            out.push(contract);
        }
    }
    out
}

fn named_contract(
    raw: &str,
    masked: &str,
    subtask: &Subtask,
    anchor: std::ops::Range<usize>,
    body: std::ops::Range<usize>,
    expected: Option<usize>,
    heading: Option<String>,
) -> Option<Contract> {
    static GRAPHIC_ONLY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)无(?:文字|文本|标签)|不(?:显示|包含|要|带)(?:文字|文本|标签)|纯色(?:板|块)|仅(?:用|显示)?色块|\b(?:unlabelled|unlabeled|swatches?)\b|\b(?:no|without)\s+(?:text|labels?)\b")
            .expect("valid explicit graphic-only specification")
    });
    // The option noun must survive the quote/metadata/ownership masks before
    // recovering literal names at the same byte spans.
    if anchor.start < subtask.label.len() + 1
        || masked[anchor.clone()].trim().is_empty()
        || super::ignored_number_context(masked, anchor.start)
    {
        return None;
    }
    let labels: Vec<_> = raw[body.clone()]
        .split(['/', '、', ',', '，', '|'])
        .take_while(|name| !is_behavior_suffix(name))
        .map(|name| {
            name.trim()
                .trim_matches(['\'', '"', '「', '」', '『', '』', '“', '”', '‘', '’'])
                .trim()
                .to_owned()
        })
        .collect();
    let expected = expected.unwrap_or(labels.len());
    if !(2..=12).contains(&expected)
        || labels.len() != expected
        || labels.iter().any(|label| {
            label.is_empty()
                || label.chars().count() > 32
                || label.contains(['(', ')', '（', '）', ':', '：', '\n'])
                || ["例如", "比如", "如", "e.g.", "for example", "such as"]
                    .iter()
                    .any(|prefix| label.to_lowercase().starts_with(prefix))
                || label.ends_with(['等', '…'])
                || label.contains("...")
        })
        || labels.iter().collect::<BTreeSet<_>>().len() != expected
    {
        return None;
    }
    let clause_start = raw[..anchor.start]
        .rfind([';', '；', '\n', '。'])
        .map_or(0, |index| {
            index + raw[index..].chars().next().unwrap().len_utf8()
        });
    let clause_end = raw[anchor.end.max(body.end)..]
        .find([';', '；', '\n', '。'])
        .map_or(raw.len(), |index| anchor.end.max(body.end) + index);
    let clause = &raw[clause_start..clause_end];
    if GRAPHIC_ONLY.is_match(clause) {
        return None;
    }
    // A postposed owner can scope the preceding list. Enumeration separators
    // are labels, not ownership coordinators; reuse the normal scope rules.
    let mut bytes = clause.as_bytes().to_vec();
    bytes[body.start - clause_start..body.end - clause_start].fill(b' ');
    for (index, ch) in clause
        .char_indices()
        .filter(|(_, ch)| matches!(ch, ',' | '，'))
    {
        bytes[index..index + ch.len_utf8()].fill(b' ');
    }
    let clause = String::from_utf8(bytes).expect("commas replaced with ASCII spaces");
    let clause = super::mask_metadata_copy(&super::mask_quoted_copy(&clause));
    let scoped = super::scope::mask_nonlocal_references(&clause, subtask);
    if scoped[anchor.start - clause_start..anchor.end - clause_start]
        .trim()
        .is_empty()
    {
        return None;
    }
    Some(Contract { labels, heading })
}

/// Parentheses often contain a named list followed by interaction directions.
/// Those directions do not add choices; explicitly quoted names remain literal.
fn is_behavior_suffix(value: &str) -> bool {
    let value = value.trim();
    if value.starts_with(['\'', '"', '「', '『', '“', '‘']) {
        return false;
    }
    static BEHAVIOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)^(?:onTap\b|onClick\b|transition\b|pressed\b|hover\b|同交互|点击(?:切换|选中)|切换选中态)")
            .expect("valid specification behavior suffix")
    });
    BEHAVIOR.is_match(value)
}

fn visible(node: &PenNode) -> bool {
    is_node_visible(node) && opacity(node) > 0.0
}

#[derive(Default)]
struct Evidence {
    best: BTreeSet<String>,
    uncertain_complete: bool,
}

fn inspect_siblings(
    nodes: &[PenNode],
    roots: &[String],
    inside: bool,
    contract: &Contract,
    uncertain_ancestor: bool,
    evidence: &mut Evidence,
) {
    let owned: Vec<_> = nodes
        .iter()
        .filter(|node| {
            potentially_visible(node) && (inside || roots.iter().any(|id| id == node.id_str()))
        })
        .collect();
    if contract.heading.is_none() {
        inspect_group(&owned, contract, uncertain_ancestor, evidence);
    } else {
        let labels: Vec<_> = owned.iter().filter_map(|node| single_label(node)).collect();
        let headings: Vec<_> = labels
            .iter()
            .filter(|(label, _)| Some(label) == contract.heading.as_ref())
            .collect();
        let heading_uncertain = if headings.len() == 1 {
            Some(headings[0].1)
        } else if headings.is_empty() && labels.iter().filter(|(_, dynamic)| *dynamic).count() == 1
        {
            // A bound heading can have empty authored copy and a real runtime
            // label. Defer only when this neighboring group could be complete.
            Some(true)
        } else {
            None
        };
        if let Some(heading_uncertain) = heading_uncertain {
            // The explicit heading owns direct choices or one adjacent row.
            // A remote heading cannot lend ownership to arbitrary metadata.
            let uncertain = uncertain_ancestor || heading_uncertain;
            inspect_group(&owned, contract, uncertain, evidence);
            for child in &owned {
                if matches!(child, PenNode::Ref(_) | PenNode::Tabs(_)) {
                    continue;
                }
                if let Some(children) = child.children() {
                    let siblings = children
                        .iter()
                        .filter(|node| potentially_visible(node))
                        .collect::<Vec<_>>();
                    // A neighboring media/business card is not a choices row.
                    if !siblings.iter().all(|node| {
                        matches!(
                            node,
                            PenNode::Frame(_)
                                | PenNode::Rectangle(_)
                                | PenNode::Group(_)
                                | PenNode::RadioGroup(_)
                                | PenNode::Select(_)
                                | PenNode::Tabs(_)
                        )
                    }) {
                        continue;
                    }
                    inspect_group(
                        &siblings,
                        contract,
                        uncertain || dynamic_visibility(child),
                        evidence,
                    );
                }
            }
        }
    }
    for node in nodes {
        if !potentially_visible(node) || matches!(node, PenNode::Ref(_)) {
            continue;
        }
        let owned = inside || roots.iter().any(|id| id == node.id_str());
        if let PenNode::Tabs(tabs) = node {
            // Panel visibility is a runtime choice. Only a complete potential
            // named group inside these panels can defer the static check.
            if let Some(children) = &tabs.children {
                inspect_siblings(children, roots, owned, contract, true, evidence);
            }
            continue;
        }
        if let Some(children) = node.children() {
            inspect_siblings(
                children,
                roots,
                owned,
                contract,
                uncertain_ancestor || dynamic_visibility(node),
                evidence,
            );
        }
    }
}

fn inspect_group(
    nodes: &[&PenNode],
    contract: &Contract,
    uncertain: bool,
    evidence: &mut Evidence,
) {
    let mut labels = BTreeSet::new();
    let mut unknown = 0;
    for node in nodes {
        if matches!(
            node,
            PenNode::Frame(_) | PenNode::Rectangle(_) | PenNode::Group(_)
        ) {
            // A capsule may be static, but each branch must have exactly one
            // visible label. A card with several labels is not several options.
            if let Some((label, dynamic)) = single_label(node) {
                if dynamic {
                    unknown += 1;
                } else if contract.labels.contains(&label) {
                    labels.insert(label);
                }
            }
        }
        let widget = widget_labels(node, contract);
        record(
            widget,
            0,
            uncertain || dynamic_visibility(node),
            contract,
            evidence,
        );
    }
    record(labels, unknown, uncertain, contract, evidence);
}

fn record(
    labels: BTreeSet<String>,
    unknown: usize,
    uncertain: bool,
    contract: &Contract,
    evidence: &mut Evidence,
) {
    if (unknown > 0 || uncertain) && labels.len() + unknown >= contract.labels.len() {
        evidence.uncertain_complete = true;
    }
    if !uncertain && labels.len() > evidence.best.len() {
        evidence.best = labels;
    }
}

fn node_bindings(node: &PenNode) -> Option<&jian_ops_schema::events::Bindings> {
    match node {
        PenNode::Frame(node) => node.bindings.as_ref(),
        PenNode::Rectangle(node) => node.bindings.as_ref(),
        PenNode::Group(node) => node.bindings.as_ref(),
        PenNode::Text(node) => node.bindings.as_ref(),
        PenNode::RadioGroup(node) => node.bindings.as_ref(),
        PenNode::Select(node) => node.bindings.as_ref(),
        PenNode::Tabs(node) => node.bindings.as_ref(),
        _ => None,
    }
}

fn dynamic_visibility(node: &PenNode) -> bool {
    node_bindings(node).is_some_and(|bindings| {
        bindings.contains_key("visible") || bindings.contains_key("opacity")
    })
}

fn potentially_visible(node: &PenNode) -> bool {
    visible(node) || dynamic_visibility(node)
}

fn single_label(node: &PenNode) -> Option<(String, bool)> {
    fn collect(node: &PenNode, texts: &mut Vec<String>, dynamic: &mut bool, depth: usize) -> bool {
        if !potentially_visible(node) {
            return true;
        }
        if depth >= 32 || matches!(node, PenNode::Ref(_) | PenNode::Tabs(_)) || texts.len() > 1 {
            return false;
        }
        *dynamic |= dynamic_visibility(node);
        if let PenNode::Text(text) = node {
            let content_binding = text
                .bindings
                .as_ref()
                .is_some_and(|bindings| bindings.contains_key("content"));
            *dynamic |= content_binding;
            let value = match &text.content {
                TextContent::Plain(value) => value.clone(),
                TextContent::Styled(runs) => runs.iter().map(|run| run.text.as_str()).collect(),
            };
            if content_binding || !value.trim().is_empty() {
                texts.push(value.trim().to_owned());
            }
        }
        node.children()
            .into_iter()
            .flatten()
            .all(|child| collect(child, texts, dynamic, depth + 1))
    }
    let mut texts = Vec::new();
    let mut dynamic = false;
    (collect(node, &mut texts, &mut dynamic, 0) && texts.len() == 1)
        .then(|| (texts.remove(0), dynamic))
}

fn widget_labels(node: &PenNode, contract: &Contract) -> BTreeSet<String> {
    let options = match node {
        PenNode::RadioGroup(group) => group.options.as_deref(),
        PenNode::Select(select) => select.options.as_deref(),
        PenNode::Tabs(tabs) => tabs.tabs.as_deref(),
        _ => None,
    };
    options
        .into_iter()
        .flatten()
        .filter_map(|option| {
            let label = option.label.trim();
            contract
                .labels
                .iter()
                .find(|name| name.as_str() == label)
                .cloned()
        })
        .collect()
}

//! Exact counts from one explicitly requested, complete exercise list.
//!
//! The same brief-derived contract guides every section and rejects conflicting
//! visible copy before insertion. It never rewrites generated text or invents
//! a count label. Partial lists, multiple lists and source-derived work opt out.

use super::{text_content, text_only_source, visible};
use crate::types::DesignRequest;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use regex::Regex;
use std::sync::LazyLock;

// Capture the whole quantity before checking the supported range. A restricted
// token would let the unanchored CJK branch read 16 / 十六 as the suffix 6 / 六.
// Unsupported compound or financial numerals are consumed, then declined.
const NUMBER: &str = r"(?:[0-9零〇一二两三四五六七八九十百千万亿兆廿卅卌壹贰叁肆伍陆柒捌玖拾佰仟萬億兩]+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)";

#[derive(Debug, PartialEq, Eq)]
struct ExerciseList {
    count: usize,
    source: String,
}

impl ExerciseList {
    fn from_request(request: &DesignRequest) -> Option<Self> {
        if !text_only_source(request) {
            return None;
        }
        let brief = request
            .prompt
            .split_once("用户需求：")
            .map_or(request.prompt.as_str(), |(_, brief)| brief);
        static AMBIGUOUS: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?ix)\b(?:first|preview|sample|some|selected|remaining|available|partial|pagination|paginated|several|multiple|(?:two|three)\s+(?:workout\s+)?(?:lists|workouts|pages|screens)|out\s+of|of\s+the|up\s+to|at\s+least|at\s+most|between|per\s+page)\b|(?:\d|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|[一二两三四五六七八九十])\s*(?:[-–—~至到]|to\b)|预览|部分|前[一二两三四五六七八九十0-9]|分页|每页|多个列表|两[个组份页]列表|至少|最多|其中|总共|共计")
                .expect("valid exercise-list ambiguity guard")
        });
        if AMBIGUOUS.is_match(brief) {
            return None;
        }
        static LIST: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(&format!(
                r"(?ix)(?:\bexercise\s+list\s+of\s+(?P<after>{NUMBER})\b|\blist\s+(?:of|with|containing)\s+(?P<before>{NUMBER})\s+exercises?\b|(?P<cjk>{NUMBER})\s*(?:个|项|条)\s*(?:训练动作|锻炼动作|动作|练习)(?:的)?列表)"
            ))
            .expect("valid explicit exercise-list count")
        });
        let mut matches = LIST.captures_iter(brief);
        let matched = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        // An unnumbered second exercise list is ambiguous too. This is not a
        // general parser for multiple workouts or multi-page programs.
        static LIST_NOUN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)\blists?\b|列表").expect("valid exercise-list noun"));
        if LIST_NOUN.find_iter(brief).count() != 1 {
            return None;
        }
        let count = ["after", "before", "cjk"]
            .iter()
            .find_map(|name| matched.name(name))
            .and_then(|number| parse_number(number.as_str()))?;
        (2..=12).contains(&count).then(|| Self {
            count,
            source: matched.get(0).unwrap().as_str().to_owned(),
        })
    }
}

fn parse_number(value: &str) -> Option<usize> {
    if let Ok(number) = value.parse() {
        return Some(number);
    }
    match value.to_lowercase().as_str() {
        "one" | "一" => Some(1),
        "two" | "二" | "两" => Some(2),
        "three" | "三" => Some(3),
        "four" | "四" => Some(4),
        "five" | "五" => Some(5),
        "six" | "六" => Some(6),
        "seven" | "七" => Some(7),
        "eight" | "八" => Some(8),
        "nine" | "九" => Some(9),
        "ten" | "十" => Some(10),
        "eleven" | "十一" => Some(11),
        "twelve" | "十二" => Some(12),
        _ => None,
    }
}

pub(super) fn append_instruction(request: &DesignRequest, prompt: &mut String) {
    let Some(contract) = ExerciseList::from_request(request) else {
        return;
    };
    let fact = serde_json::json!({"entity":"complete exercise list","count":contract.count,"source":contract.source});
    prompt.push_str(&format!(
        "\n\nEXPLICIT EXERCISE LIST FACT — brief-derived data: {fact}. Render the requested list completely in its owning section. If this section already displays that list's count, it must match this fact even when planner metadata proposes another number. This does not request an extra count label or extra rows in a hero. Sets/reps/duration are different metrics; preserve each row's own values. An optional 'N sets each' summary is valid only if every row actually has N sets."
    ));
}

pub(super) fn rejection(request: &DesignRequest, nodes: &[PenNode]) -> Option<String> {
    let contract = ExerciseList::from_request(request)?;
    let mut copy = Vec::new();
    for node in nodes {
        collect_copy(node, &mut copy);
    }
    if let Some((name, text, actual)) = copy.iter().find_map(|(name, text)| {
        displayed_count(text)
            .filter(|actual| *actual != contract.count)
            .map(|actual| (name, text, actual))
    }) {
        let evidence = serde_json::json!({"node":name,"text":text,"expected":contract.count,"actual":actual,"source":contract.source});
        return Some(format!(
            "self-check failed: exercise-list-count-conflict: {evidence}. The ORIGINAL USER BRIEF specifies one complete exercise list. Return the complete section with a matching count if that label is retained; do not add rows to a hero or change sets/reps/duration to reconcile it. Do not return only a fragment."
        ));
    }
    let mut families = Vec::new();
    for node in nodes {
        collect_row_sets(node, contract.count, &mut families);
    }
    // A universal summary can be checked only against one complete, fully
    // parseable sibling list. Unknown rows or competing groups are left alone.
    let [sets] = families.as_slice() else {
        return None;
    };
    let (name, text, stated) = copy.iter().find_map(|(name, text)| {
        sets_each(text)
            .filter(|stated| sets.iter().any(|actual| actual != stated))
            .map(|stated| (name, text, stated))
    })?;
    let evidence =
        serde_json::json!({"node":name,"text":text,"summary_sets":stated,"row_sets":sets});
    Some(format!(
        "self-check failed: exercise-set-summary-conflict: {evidence}. Return the complete exercise list section preserving the per-row set counts. Remove or accurately rewrite the optional universal sets summary; do not force all rows to match it or return only a fragment."
    ))
}

fn collect_copy(node: &PenNode, copy: &mut Vec<(String, String)>) {
    if !visible(node) {
        return;
    }
    if let Some(text) = text_content(node) {
        copy.push((node.base().name.clone().unwrap_or_default(), text));
    }
    for child in node.children().into_iter().flatten() {
        collect_copy(child, copy);
    }
}

fn displayed_count(text: &str) -> Option<usize> {
    static SCOPE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:of|completed|available|remaining|recommended|previous|history|week|total\s+catalog)\b|累计|已完成|剩余|推荐|历史|本周|总库")
            .expect("valid distinct exercise-count scope")
    });
    if SCOPE.is_match(text) {
        return None;
    }
    static COUNT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)(?:\b(?P<before>\d{1,3})\s+exercises?\b|\A\s*exercises\s*[:：(（]?\s*(?P<after>\d{1,3})\s*[)）]?\s*\z|(?P<cjk>\d{1,3})\s*(?:个|项)\s*(?:训练动作|锻炼动作|动作|练习))")
            .expect("valid visible exercise count")
    });
    let mut matches = COUNT.captures_iter(text);
    let matched = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    ["before", "after", "cjk"]
        .iter()
        .find_map(|name| matched.name(name))
        .and_then(|number| number.as_str().parse().ok())
}

fn sets_each(text: &str) -> Option<usize> {
    static EACH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)^\s*(?:(?P<en>\d{1,2})\s+sets?\s+(?:each|per\s+exercise)|每(?:项|个动作|个练习)\s*(?P<cjk>\d{1,2})\s*组)\s*$")
            .expect("valid universal exercise sets summary")
    });
    let matched = EACH.captures(text)?;
    matched
        .name("en")
        .or_else(|| matched.name("cjk"))?
        .as_str()
        .parse()
        .ok()
}

fn row_sets(node: &PenNode) -> Option<usize> {
    let mut copy = Vec::new();
    collect_copy(node, &mut copy);
    static SETS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)^\s*(?P<count>\d{1,2})\s*(?:sets?\b|组)\s*[×x*]\s*\S+")
            .expect("valid per-row exercise sets")
    });
    let mut values = copy.iter().filter_map(|(_, text)| {
        SETS.captures(text)
            .and_then(|matched| matched["count"].parse().ok())
    });
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn collect_row_sets(node: &PenNode, count: usize, groups: &mut Vec<Vec<usize>>) {
    if !visible(node) {
        return;
    }
    let Some(children) = node.children() else {
        return;
    };
    let children: Vec<_> = children.iter().filter(|child| visible(child)).collect();
    if children.len() == count {
        if let Some(sets) = children.iter().map(|child| row_sets(child)).collect() {
            groups.push(sets);
            return;
        }
    }
    for child in children {
        collect_row_sets(child, count, groups);
    }
}

#[cfg(test)]
#[path = "source_facts_exercise_tests.rs"]
mod tests;

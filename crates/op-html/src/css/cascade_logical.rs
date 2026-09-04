//! Direction-aware presentational hints and logical margin aliases.

use std::collections::BTreeMap;

use crate::dom::DomElement;

use super::{origin_rank, Candidate, CascadeScope, Priority, StyleOrigin};

pub(super) fn push_direction_hint(
    candidates: &mut BTreeMap<String, Vec<Candidate>>,
    path: &[&DomElement],
) {
    let Some(direction) = path
        .last()
        .and_then(|element| element.attr("dir"))
        .and_then(|direction| {
            let direction = direction.trim().to_ascii_lowercase();
            matches!(direction.as_str(), "ltr" | "rtl").then_some(direction)
        })
    else {
        return;
    };
    candidates
        .entry("direction".to_string())
        .or_default()
        .push(Candidate {
            target: "direction".to_string(),
            priority: Priority {
                important: false,
                origin: origin_rank(StyleOrigin::Author, false),
                scope: CascadeScope::Unlayered,
                specificity: (0, 0, 0),
                order: 0,
                declaration_order: 0,
            },
            value: direction,
            deferred_shorthand: None,
            origin: StyleOrigin::Author,
            scope: CascadeScope::Unlayered,
        });
}

pub(super) fn remap_margin_candidates(
    candidates: &mut BTreeMap<String, Vec<Candidate>>,
    rtl: bool,
) {
    for (logical, physical) in [
        (
            "margin-inline-start",
            if rtl { "margin-right" } else { "margin-left" },
        ),
        (
            "margin-inline-end",
            if rtl { "margin-left" } else { "margin-right" },
        ),
    ] {
        let Some(mut logical_values) = candidates.remove(logical) else {
            continue;
        };
        candidates
            .entry(physical.to_string())
            .or_default()
            .append(&mut logical_values);
    }
}

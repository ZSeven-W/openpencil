//! `section-structure-drift` false-positive guards (GLM-5.3-Flash arena
//! 0926c: a kanban board and a pricing section were dropped whole) and the
//! intent-class severity contract. Font-agnostic: pure JSON structure.

use serde_json::{json, Value};

use super::*;

fn drift_issues(report: &SelfCheckReport) -> Vec<&SelfCheckIssue> {
    report
        .issues
        .iter()
        .filter(|issue| issue.code == "section-structure-drift")
        .collect()
}

fn board(children: Vec<Value>) -> Value {
    json!([{
        "type": "frame", "id": "board", "name": "Board", "layout": "horizontal",
        "children": children
    }])
}

fn task_card(id: &str, badge: bool) -> Value {
    let mut children = vec![
        json!({ "type": "text", "id": format!("{id}-t"), "content": "Write spec" }),
        json!({ "type": "text", "id": format!("{id}-d"), "content": "Due Fri" }),
    ];
    if badge {
        children.push(json!({
            "type": "frame", "id": format!("{id}-b"), "name": "Tag",
            "children": [{ "type": "text", "id": format!("{id}-bt"), "content": "Urgent" }]
        }));
    }
    json!({
        "type": "frame", "id": id, "name": "Task Card", "layout": "vertical",
        "children": children
    })
}

fn column(id: &str, cards: usize) -> Value {
    let mut children = vec![json!({
        "type": "text", "id": format!("{id}-h"), "content": "To do"
    })];
    children.extend((0..cards).map(|index| task_card(&format!("{id}-c{index}"), false)));
    json!({
        "type": "frame", "id": id, "name": "Kanban Column", "layout": "vertical",
        "children": children
    })
}

/// arena-w01: columns holding 3 / 4 / 5 identical task cards are ONE
/// structure — list length is content, not structure.
#[test]
fn kanban_columns_with_different_card_counts_are_not_drift() {
    let nodes = board(vec![
        column("todo", 3),
        column("doing", 4),
        column("done", 5),
    ]);
    let report = check_value_forest(&nodes, 1280.0);
    assert!(drift_issues(&report).is_empty(), "{report:?}");
    assert!(!report.has_fatal(), "{report:?}");
}

/// Consecutive identical children collapse, but a different child kind or
/// order is still structure.
#[test]
fn structure_signature_collapses_only_consecutive_repeats() {
    let three = column("a", 3);
    let five = column("b", 5);
    assert_eq!(
        drift::value_structure_signature(&three),
        drift::value_structure_signature(&five)
    );
    assert_ne!(
        drift::value_structure_signature(&task_card("x", false)),
        drift::value_structure_signature(&task_card("y", true))
    );
}

/// 4 task cards, 2 carrying an optional tag: that IS drift (2/4 is below
/// the 2/3 majority), so it still rejects while the ladder can retry — but
/// it is intent-class, so the final attempt only turns it into an advisory.
#[test]
fn optional_badge_cards_reject_early_but_are_advisory_on_the_final_attempt() {
    let nodes = board(vec![
        task_card("c1", false),
        task_card("c2", true),
        task_card("c3", false),
        task_card("c4", true),
    ]);
    let report = check_value_forest(&nodes, 1280.0);
    let drift = drift_issues(&report);
    assert_eq!(drift.len(), 1, "{report:?}");
    assert_eq!(drift[0].severity, SelfCheckSeverity::Intent);

    assert!(report.rejects(IntentCheckMode::Reject));
    assert!(report
        .failure_message_for(IntentCheckMode::Reject)
        .contains("section-structure-drift"));
    assert!(advisory_accepts(&report), "{report:?}");
}

fn advisory_accepts(report: &SelfCheckReport) -> bool {
    !report.rejects(IntentCheckMode::Advisory)
        && report
            .failure_message_for(IntentCheckMode::Advisory)
            .is_empty()
        && report
            .advisory_lines(IntentCheckMode::Advisory)
            .iter()
            .any(|line| line.starts_with("section-structure-drift"))
        && report.advisory_lines(IntentCheckMode::Reject).is_empty()
}

fn pricing_card(tier: &str, role: &str, extra: Vec<Value>) -> Value {
    let mut children = vec![
        json!({ "type": "text", "id": format!("{tier}-name"), "content": tier }),
        json!({ "type": "frame", "id": format!("{tier}-cta"), "name": "CTA Button",
                "children": [{ "type": "text", "id": format!("{tier}-cta-t"), "content": "Start" }] }),
    ];
    for (index, node) in extra.into_iter().enumerate() {
        children.insert(index, node);
    }
    json!({
        "type": "frame", "id": format!("pricing-card-{tier}"),
        "name": format!("pricing-card-{tier}"), "role": role, "layout": "vertical",
        "children": children
    })
}

fn pricing_row(role: &str) -> Value {
    board(vec![
        pricing_card(
            "starter",
            role,
            vec![json!({ "type": "text", "id": "starter-price", "content": "$9" })],
        ),
        pricing_card(
            "pro",
            role,
            vec![
                json!({ "type": "frame", "id": "pro-badge", "name": "Most Popular",
                        "children": [{ "type": "icon_font", "id": "pro-star", "iconFontName": "star" }] }),
                json!({ "type": "text", "id": "pro-price", "content": "$29" }),
            ],
        ),
        pricing_card(
            "enterprise",
            role,
            vec![json!({ "type": "icon_font", "id": "ent-icon", "iconFontName": "building" })],
        ),
    ])
}

/// arena-l02: starter / pro / enterprise are legitimately different. With
/// the role inference pipeline's own `card` (read off the names) they form
/// no family at all; with an AUTHORED `pricing-card` role they are a list
/// that drifts — rejected early, accepted on the final attempt.
#[test]
fn pricing_tiers_never_block_the_final_attempt() {
    let inferred = check_value_forest(&pricing_row("card"), 1280.0);
    assert!(drift_issues(&inferred).is_empty(), "{inferred:?}");

    let authored = check_value_forest(&pricing_row("pricing-card"), 1280.0);
    assert_eq!(drift_issues(&authored).len(), 1, "{authored:?}");
    assert!(authored.rejects(IntentCheckMode::Reject));
    assert!(advisory_accepts(&authored), "{authored:?}");
}

fn card_part(id: &str, role: &str, layout: &str, children: Vec<Value>) -> Value {
    json!({
        "type": "frame", "id": id, "name": id, "role": role, "layout": layout,
        "children": children
    })
}

fn pricing_parts(role: &str, layouts: [&str; 3]) -> Value {
    json!([{
        "type": "frame", "id": "pricing-card-pro", "name": "Pro", "layout": "vertical",
        "children": [
            card_part("pricing-card-pro-header", role, layouts[0], vec![
                json!({ "type": "text", "id": "h1", "content": "Pro" }),
            ]),
            card_part("pricing-card-pro-price", role, layouts[1], vec![
                json!({ "type": "text", "id": "p1", "content": "$29" }),
                json!({ "type": "icon_font", "id": "p2", "iconFontName": "info" }),
            ]),
            card_part("pricing-card-pro-cta", role, layouts[2], vec![
                json!({ "type": "rectangle", "id": "c1" }),
                json!({ "type": "text", "id": "c2", "content": "Buy" }),
            ]),
        ]
    }])
}

/// arena-l02: header / price / cta are PARTS of one card that happened to
/// share a role value — they must not form a drift group.
#[test]
fn card_parts_sharing_a_generic_role_are_not_a_family() {
    // The run's actual shape: `card` inferred from every part's name.
    let inferred = check_value_forest(
        &pricing_parts("card", ["vertical", "vertical", "vertical"]),
        1280.0,
    );
    assert!(drift_issues(&inferred).is_empty(), "{inferred:?}");

    // An authored generic role on parts that do not flow like one list.
    let authored = check_value_forest(
        &pricing_parts("section", ["vertical", "horizontal", "vertical"]),
        1280.0,
    );
    assert!(drift_issues(&authored).is_empty(), "{authored:?}");
}

/// An authored, name-independent role still groups a consecutive,
/// same-layout list whose names drifted (the 0815 role path survives).
#[test]
fn authored_role_on_a_consecutive_list_still_groups() {
    let nodes = json!([{
        "type": "frame", "id": "card", "name": "Card", "layout": "vertical",
        "children": [
            card_part("Alpha", "rule-item", "vertical", vec![
                json!({ "type": "text", "id": "a", "content": "A" }),
            ]),
            card_part("Beta", "rule-item", "vertical", vec![
                json!({ "type": "icon_font", "id": "b", "iconFontName": "x" }),
            ]),
            card_part("Gamma", "rule-item", "vertical", vec![
                json!({ "type": "rectangle", "id": "g" }),
            ]),
        ]
    }]);
    let report = check_value_forest(&nodes, 1080.0);
    assert_eq!(drift_issues(&report).len(), 1, "{report:?}");
}

/// Contract-class findings are untouched by the attempt mode: they still
/// reject on the final attempt.
#[test]
fn contract_findings_reject_in_advisory_mode_too() {
    let report = SelfCheckReport {
        issues: vec![SelfCheckIssue {
            code: "radial-stack-not-concentric",
            node_id: Some("n1".into()),
            message: "x".into(),
            severity: SelfCheckSeverity::Fatal,
        }],
    };
    assert!(report.rejects(IntentCheckMode::Advisory));
    assert!(report.advisory_lines(IntentCheckMode::Advisory).is_empty());
}

use super::*;
use serde_json::json;

#[test]
fn app11_calendar_week_rows_with_different_event_dots_do_not_drift() {
    for source in [
        include_str!("fixtures/regress_1019/month_grid_1.json"),
        include_str!("fixtures/regress_1019/month_grid_2.json"),
    ] {
        let grid: Value = serde_json::from_str(source).unwrap();
        let mut counts = BTreeMap::new();
        for week in children(&grid).unwrap() {
            assert!(calendar_week(week));
            *counts.entry(value_structure_signature(week)).or_insert(0) += 1;
        }
        // Prove the old structural-vote predicate rejected this fixture.
        assert!(counts.len() > 1);
        assert!(counts.values().max().unwrap() * 3 < children(&grid).unwrap().len() * 2);
        assert!(sibling_structure_drift(&grid).is_none());
        let report = super::super::check_value_forest(&json!([grid]), 375.0);
        assert!(
            report
                .issues
                .iter()
                .all(|issue| issue.code != "section-structure-drift"),
            "{report:?}"
        );
    }
}

#[test]
fn week_row_names_do_not_exempt_non_calendar_structure_drift() {
    let node = json!({"type":"frame", "children":[
        {"type":"frame", "name":"Week Row 1", "children":[{"type":"text", "content":"Title"}]},
        {"type":"frame", "name":"Week Row 2", "children":[{"type":"icon_font"}]},
        {"type":"frame", "name":"Week Row 3", "children":[{"type":"image"}]}
    ]});
    assert!(sibling_structure_drift(&node).is_some());
}

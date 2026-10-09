use crate::get_skill_by_name;
#[test]
fn web_app_skill_requires_truthful_local_pages_and_explicit_clock_data() {
    let skill = get_skill_by_name("web-app").unwrap();
    let content = &skill.content;
    for contract in [
        "Pagination Footer",
        "Rows Per Page Select",
        "Page Buttons",
        "op-table-clock:v1",
        "reference_ms",
        "op-table-time:v1",
        "last_active_ms",
        "Yesterday",
        "inclusive cutoff",
    ] {
        assert!(content.contains(contract), "missing {contract}");
    }
}

#[test]
fn table_contract_is_not_truncated_by_its_own_skill_budget() {
    let skill = get_skill_by_name("web-app").unwrap();
    assert!(crate::budget::estimate_tokens(&skill.content) <= skill.meta.budget);
}

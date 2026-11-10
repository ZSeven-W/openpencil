use super::*;

fn details() -> Vec<String> {
    vec![
        "Checked layout, overflow, structure".to_owned(),
        "repairs: layout 3, structure 7".to_owned(),
        "structure · hero-bleed · Home [n1] · gap 0 → 16".to_owned(),
        "structure · motion-recipes · [n28] · animations (unset) → [inView animation 320ms]"
            .to_owned(),
        "layout · stretch-root · height (unset) → 844".to_owned(),
        "… and 4 more repair(s) — full list in the log".to_owned(),
    ]
}

#[test]
fn repair_records_parse_into_who_what_and_the_change() {
    let body = detail_body(&details(), 60, false).expect("records present");
    let repairs: Vec<&Repair> = body
        .lines
        .iter()
        .filter_map(|line| match line {
            DetailLine::Repair(repair) => Some(repair),
            _ => None,
        })
        .collect();
    assert_eq!(repairs.len(), 3);
    assert_eq!(repairs[0].category, "structure");
    assert_eq!(repairs[0].pass, "hero-bleed");
    assert_eq!(
        repairs[0].node,
        Some(("Home".to_owned(), Some("n1".to_owned())))
    );
    assert_eq!(
        (repairs[0].before.as_str(), repairs[0].after.as_deref()),
        ("gap 0", Some("16"))
    );
    // An unnamed node shows its id; a document-level repair has no node.
    assert_eq!(repairs[1].node, Some(("n28".to_owned(), None)));
    assert_eq!(repairs[2].node, None);
    assert_eq!(repairs[2].before, "height (unset)");
    assert!(matches!(&body.lines[1], DetailLine::Counts(counts)
        if counts == &[("layout".to_owned(), 3), ("structure".to_owned(), 7)]));
    assert!(matches!(&body.lines[0], DetailLine::Text { .. }));
    assert!(matches!(body.lines.last(), Some(DetailLine::Text { .. })));
}

#[test]
fn an_activity_detail_splits_its_lines_and_sizes_the_body() {
    // The desktop "Polishing the layout" row carries one newline-joined detail.
    let joined = vec![details().join("\n")];
    let body = detail_body(&joined, 60, false).expect("records present");
    assert_eq!(body.lines.len(), 6);
    let expected = 3.0 * REPAIR_ROW_H + CHIP_ROW_H + 2.0 * ACTION_DETAIL_LINE_H + 4.0;
    assert!((body.height - expected).abs() < 0.01, "{}", body.height);
}

fn lines(details: &[&str], alert: bool) -> Vec<DetailLine> {
    let details: Vec<String> = details.iter().map(|line| (*line).to_owned()).collect();
    detail_body(&details, 60, alert).expect("body").lines
}

#[test]
fn progress_details_become_fields_stats_and_changes() {
    let parsed = lines(
        &[
            "3 screens · 41 elements",
            "skills: cjk-typography, mobile-app-ui",
            "dropped: hero-image",
            "retry #1: geometry echo: 3 issue(s) → retry",
            "Refining the layout",
        ],
        false,
    );
    assert_eq!(
        parsed[0],
        DetailLine::Stats(vec!["3 screens".into(), "41 elements".into()])
    );
    assert!(
        matches!(&parsed[1], DetailLine::Field { key, value: FieldValue::Chips(items), .. }
        if key == "skills" && items.len() == 2)
    );
    assert!(
        matches!(&parsed[2], DetailLine::Field { value: FieldValue::Chips(items), .. }
        if items == &["hero-image".to_owned()])
    );
    assert!(
        matches!(&parsed[3], DetailLine::Field { key, value: FieldValue::Change { before, after }, .. }
        if key == "retry #1" && before == "geometry echo: 3 issues" && after == "retry")
    );
    assert!(matches!(&parsed[4], DetailLine::Text { alert: false, .. }));
}

#[test]
fn json_values_expand_into_their_fields() {
    let parsed = lines(
        &[r#"Args: {"nodeId":"n1","scale":2,"tags":["a","b"],"style":{"type":"solid"}}"#],
        false,
    );
    let DetailLine::Field {
        key,
        value: FieldValue::Fields { fields, more },
        ..
    } = &parsed[0]
    else {
        panic!("{parsed:?}");
    };
    assert_eq!(key, "Args");
    assert_eq!(*more, 0);
    assert_eq!(
        fields,
        &[
            ("nodeId".to_owned(), "n1".to_owned()),
            ("scale".to_owned(), "2".to_owned()),
            ("tags".to_owned(), "[a, b]".to_owned()),
            ("style".to_owned(), "solid".to_owned()),
        ]
    );
    let many: String = format!(
        "Args: {{{}}}",
        (0..11)
            .map(|i| format!("\"k{i:02}\":{i}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let parsed = lines(&[&many], false);
    assert!(
        matches!(&parsed[0], DetailLine::Field { value: FieldValue::Fields { fields, more: 3 }, .. }
        if fields.len() == 8)
    );
}

#[test]
fn failures_tint_their_lines_and_prose_is_not_a_field() {
    let parsed = lines(&["Only 2 of 3 promised item(s) delivered"], true);
    assert!(matches!(&parsed[0], DetailLine::Text { rows, alert: true }
        if rows[0] == "Only 2 of 3 promised items delivered"));
    // A URL's colon or a sentence that starts with prose is not a key.
    let parsed = lines(
        &["see https://example.com/a", "Two words then: a colon later"],
        false,
    );
    assert!(matches!(&parsed[0], DetailLine::Text { .. }));
    assert!(matches!(
        &parsed[1],
        DetailLine::Field { .. } | DetailLine::Text { .. }
    ));
    assert_eq!(detail_body(&[], 60, false), None);
}

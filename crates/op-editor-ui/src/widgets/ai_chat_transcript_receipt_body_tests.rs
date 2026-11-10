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
    let body = receipt_body(&details(), 60).expect("records present");
    let repairs: Vec<&Repair> = body
        .lines
        .iter()
        .filter_map(|line| match line {
            ReceiptLine::Repair(repair) => Some(repair),
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
    assert!(matches!(&body.lines[1], ReceiptLine::Counts(counts)
        if counts == &[("layout".to_owned(), 3), ("structure".to_owned(), 7)]));
    assert!(matches!(&body.lines[0], ReceiptLine::Text(_)));
    assert!(matches!(body.lines.last(), Some(ReceiptLine::Text(_))));
}

#[test]
fn details_without_records_keep_their_plain_rows() {
    let plain = vec![
        "skills: cjk-typography".to_owned(),
        "retry #1: timeout".to_owned(),
    ];
    assert_eq!(receipt_body(&plain, 60), None);
}

#[test]
fn an_activity_detail_splits_its_lines_and_sizes_the_body() {
    // The desktop "Polishing the layout" row carries one newline-joined detail.
    let joined = vec![details().join("\n")];
    let body = receipt_body(&joined, 60).expect("records present");
    assert_eq!(body.lines.len(), 6);
    let expected = 3.0 * REPAIR_ROW_H + CHIP_ROW_H + 2.0 * ACTION_DETAIL_LINE_H + 4.0;
    assert!((body.height - expected).abs() < 0.01, "{}", body.height);
}

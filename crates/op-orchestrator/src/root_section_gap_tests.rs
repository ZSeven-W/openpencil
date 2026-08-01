use super::*;
use serde_json::json;

fn mobile_root(gap: Option<Value>, sections: usize) -> Value {
    let mut root = json!({
        "type": "frame",
        "name": "Explore",
        "layout": "vertical",
        "width": 390,
        "children": (0..sections)
            .map(|i| json!({"type": "frame", "name": format!("Section {i}")}))
            .collect::<Vec<_>>(),
    });
    if let Some(gap) = gap {
        root.as_object_mut().unwrap().insert("gap".into(), gap);
    }
    root
}

#[test]
fn a_mobile_root_with_no_gap_gets_the_planner_default() {
    // The 0731 reproduction: `gap` absent entirely, so every section touched
    // the next and only the sections' own padding kept them apart.
    let mut root = mobile_root(None, 7);
    assert!(fix_root_section_gap(&mut root));
    assert_eq!(root["gap"], json!(MOBILE_DEFAULT_ROOT_GAP));
}

#[test]
fn an_explicit_zero_is_repaired_the_same_way_the_planner_repairs_it() {
    // `plan_normalize` treats `<= 0` as absent; matching that keeps one
    // contract across both paths.
    let mut root = mobile_root(Some(json!(0)), 4);
    assert!(fix_root_section_gap(&mut root));
    assert_eq!(root["gap"], json!(MOBILE_DEFAULT_ROOT_GAP));
}

#[test]
fn a_desktop_root_uses_the_scaffold_default_instead() {
    let mut root = mobile_root(None, 5);
    root["width"] = json!(1440);
    assert!(fix_root_section_gap(&mut root));
    assert_eq!(root["gap"], json!(SECTION_STACK_GAP));
}

#[test]
fn an_authored_gap_is_never_overwritten() {
    // Including a value tighter than the default: a deliberate dense stack is
    // a design decision, not a defect.
    for authored in [4.0, 12.0, 48.0] {
        let mut root = mobile_root(Some(json!(authored)), 6);
        assert!(
            !fix_root_section_gap(&mut root),
            "gap {authored} was rewritten"
        );
        assert_eq!(root["gap"], json!(authored));
    }
}

#[test]
fn a_horizontal_root_is_left_alone() {
    // An app shell's column row is horizontal; its gap is a different
    // decision and not this pass's business.
    let mut root = mobile_root(None, 5);
    root["layout"] = json!("horizontal");
    assert!(!fix_root_section_gap(&mut root));
    assert!(root.get("gap").is_none());
}

#[test]
fn one_or_two_sections_read_as_composition_not_as_a_defect() {
    for sections in 0..MIN_SECTIONS {
        let mut root = mobile_root(None, sections);
        assert!(
            !fix_root_section_gap(&mut root),
            "{sections} sections should not be repaired"
        );
    }
    // The threshold itself is repaired.
    let mut root = mobile_root(None, MIN_SECTIONS);
    assert!(fix_root_section_gap(&mut root));
}

#[test]
fn a_root_without_a_width_is_treated_as_mobile() {
    // Loop-built roots frequently omit width or use `fill_container`; the
    // planner's mobile default is the safer of the two there.
    let mut root = mobile_root(None, 4);
    root.as_object_mut().unwrap().remove("width");
    assert!(fix_root_section_gap(&mut root));
    assert_eq!(root["gap"], json!(MOBILE_DEFAULT_ROOT_GAP));

    let mut root = mobile_root(None, 4);
    root["width"] = json!("fill_container");
    assert!(fix_root_section_gap(&mut root));
    assert_eq!(root["gap"], json!(MOBILE_DEFAULT_ROOT_GAP));
}

#[test]
fn running_twice_changes_nothing_the_second_time() {
    let mut root = mobile_root(None, 5);
    assert!(fix_root_section_gap(&mut root));
    assert!(
        !fix_root_section_gap(&mut root),
        "the pass must be idempotent"
    );
}

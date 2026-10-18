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

fn fixed_mobile_scroll_shell() -> Value {
    json!({
        "type":"frame","id":"phone","name":"Product detail",
        "width":375,"height":812,"layout":"vertical","gap":0,
        "children":[
            {"type":"frame","id":"status","name":"Status Bar","role":"status-bar",
                "width":"fill_container","height":62},
            {"type":"frame","id":"viewport","name":"Scrollable content","role":"scroll-area",
                "width":"fill_container","height":"fill_container","layout":"vertical",
                "gap":16,"clipContent":true,"children":[
                    {"type":"frame","id":"hero","name":"Product photo",
                        "width":"fill_container","height":360},
                    {"type":"frame","id":"reviews","name":"Reviews",
                        "width":"fill_container","height":700}
                ]},
            {"type":"frame","id":"actions","name":"Purchase actions","role":"bottom-action-bar",
                "width":"fill_container","height":88,"layout":"horizontal",
                "padding":[12,24,24,24],"children":[
                    {"type":"frame","id":"purchase","name":"Add to cart","role":"button",
                        "width":"fill_container","height":52,"children":[
                            {"type":"text","id":"purchase-label","content":"Add to cart","fontSize":16}
                        ]}
                ]}
        ]
    })
}

#[test]
fn fixed_mobile_scroll_shell_keeps_zero_root_gap_and_inner_rhythm_on_replay() {
    for height in [812, 1100] {
        let mut root = fixed_mobile_scroll_shell();
        root["height"] = json!(height);
        let before = root.clone();

        for _ in 0..2 {
            assert!(!fix_root_section_gap(&mut root), "height: {height}");
            assert_eq!(root["gap"], json!(0));
            assert_eq!(root["children"][1]["gap"], json!(16));
            assert_eq!(
                root, before,
                "chrome boundaries and content remain unchanged"
            );
        }
    }
}

#[test]
fn fixed_mobile_scroll_shell_keeps_an_explicit_positive_root_gap() {
    let mut root = fixed_mobile_scroll_shell();
    root["gap"] = json!(12);
    let before = root.clone();
    assert!(!fix_root_section_gap(&mut root));
    assert_eq!(root, before);
}

#[test]
fn fixed_mobile_scroll_shell_with_bottom_tabs_keeps_zero_root_gap() {
    let mut root = fixed_mobile_scroll_shell();
    let tabs: Vec<Value> = [("home", "Home"), ("search", "Search"), ("user", "Profile")]
        .into_iter()
        .map(|(icon, label)| {
            json!({
                "type":"frame","id":format!("tab-{icon}"),"name":label,"role":"tab",
                "width":"fill_container","height":52,"layout":"vertical","children":[
                    {"type":"icon_font","id":format!("icon-{icon}"),
                        "iconFontFamily":"lucide","iconFontName":icon,"width":20,"height":20},
                    {"type":"text","id":format!("label-{icon}"),"content":label,"fontSize":11}
                ]
            })
        })
        .collect();
    root["children"][2] = json!({
        "type":"frame","id":"navigation","name":"Bottom navigation","role":"bottom-tab-bar",
        "width":"fill_container","height":88,"layout":"horizontal","children":tabs
    });
    let before = root.clone();
    assert!(!fix_root_section_gap(&mut root));
    assert_eq!(root, before);
}

#[test]
fn incomplete_or_nonmobile_scroll_shells_still_receive_the_default_root_gap() {
    for case in [
        "ordinary content",
        "action role without button",
        "missing status",
        "missing scroll role",
        "nonclipping viewport",
        "unspecified viewport clipping",
        "fixed viewport height",
        "fixed viewport width",
        "horizontal viewport",
        "positioned viewport",
        "constrained viewport",
        "positioned action",
        "too tall action",
        "desktop width",
        "too short root",
        "intrinsic root height",
        "root clipping disabled",
        "extra body section",
    ] {
        let mut root = fixed_mobile_scroll_shell();
        let mut expected = MOBILE_DEFAULT_ROOT_GAP;
        match case {
            "ordinary content" => {
                let action = root["children"][2].as_object_mut().unwrap();
                action.remove("role");
                action.insert("id".into(), json!("related-products"));
                action.insert("name".into(), json!("Related products"));
            }
            "action role without button" => {
                root["children"][2]["children"] = json!([
                    {"type":"text","id":"summary","content":"Order summary","fontSize":16}
                ]);
            }
            "missing status" => {
                root["children"][0] = json!({
                    "type":"frame","id":"intro","name":"Introduction",
                    "width":"fill_container","height":62
                });
            }
            "missing scroll role" => root["children"][1]["role"] = json!("content"),
            "nonclipping viewport" => root["children"][1]["clipContent"] = json!(false),
            "unspecified viewport clipping" => {
                root["children"][1]
                    .as_object_mut()
                    .unwrap()
                    .remove("clipContent");
            }
            "fixed viewport height" => root["children"][1]["height"] = json!(662),
            "fixed viewport width" => root["children"][1]["width"] = json!(375),
            "horizontal viewport" => root["children"][1]["layout"] = json!("horizontal"),
            "positioned viewport" => root["children"][1]["y"] = json!(62),
            "constrained viewport" => {
                root["children"][1]["constraints"] = json!({"vertical":"top-bottom"});
            }
            "positioned action" => root["children"][2]["y"] = json!(724),
            "too tall action" => root["children"][2]["height"] = json!(129),
            "desktop width" => {
                root["width"] = json!(1440);
                expected = SECTION_STACK_GAP;
            }
            "too short root" => root["height"] = json!(499),
            "intrinsic root height" => root["height"] = json!("fit_content"),
            "root clipping disabled" => root["clipContent"] = json!(false),
            "extra body section" => {
                root["children"].as_array_mut().unwrap().insert(
                    2,
                    json!({
                        "type":"frame","id":"extra","name":"Extra content",
                        "width":"fill_container","height":100
                    }),
                );
            }
            _ => unreachable!(),
        }
        let children = root["children"].clone();

        assert!(fix_root_section_gap(&mut root), "{case}");
        assert_eq!(root["gap"], json!(expected), "{case}");
        assert_eq!(root["children"], children, "{case}");
        assert!(!fix_root_section_gap(&mut root), "replay: {case}");
    }
}

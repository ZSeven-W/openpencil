use super::*;

fn get<'a>(declarations: &'a [Declaration], name: &str) -> Option<&'a str> {
    declarations
        .iter()
        .rev()
        .find(|declaration| declaration.name == name)
        .map(|declaration| declaration.value.as_str())
}

#[test]
fn box_and_logical_shorthands_expand() {
    let declarations =
        parse_declarations("margin:10px 20px; padding-inline:4px 8px; inset:1px 2px 3px 4px");
    assert_eq!(get(&declarations, "margin-bottom"), Some("10px"));
    assert_eq!(get(&declarations, "padding-right"), Some("8px"));
    assert_eq!(get(&declarations, "right"), Some("2px"));
    assert_eq!(get(&declarations, "left"), Some("4px"));
}

#[test]
fn direction_rejects_invalid_literals_but_keeps_deferred_values() {
    let declarations = parse_declarations(
        "direction:rtl;direction:nonsense;direction:var(--direction);\
         direction:'var(--not-a-function)'",
    );
    let directions: Vec<_> = declarations
        .iter()
        .filter(|declaration| declaration.name == "direction")
        .map(|declaration| declaration.value.as_str())
        .collect();
    assert_eq!(directions, ["rtl", "var(--direction)"]);

    for value in [
        "LTR",
        "rtl",
        "inherit",
        "initial",
        "unset",
        "revert",
        "revert-layer",
    ] {
        assert_eq!(
            get(
                &parse_declarations(&format!("direction:{value}")),
                "direction"
            ),
            Some(value)
        );
    }
}

#[test]
fn borders_expand_per_side_reset_and_corner() {
    let declarations = parse_declarations(
        "border-left-width:3px; border:1px solid #123; border-radius:4px 8px / 2px",
    );
    assert_eq!(get(&declarations, "border-width"), Some("1px"));
    assert_eq!(get(&declarations, "border-left-width"), Some("1px"));
    assert_eq!(get(&declarations, "border-color"), Some("#123"));
    assert_eq!(get(&declarations, "border-top-right-radius"), Some("8px"));
    let reset = parse_declarations("border:1px");
    assert_eq!(get(&reset, "border-left-style"), Some("none"));
    assert_eq!(get(&reset, "border-left-color"), Some("currentcolor"));
}

#[test]
fn functions_strings_comments_and_important_are_tokenized_safely() {
    let declarations = parse_declarations(
        r#"--x:calc(100% - 2px); background:url("data:x;a:b") center/cover no-repeat /*x*/ !important; content:"a;b:c""#,
    );
    assert_eq!(get(&declarations, "--x"), Some("calc(100% - 2px)"));
    assert_eq!(
        get(&declarations, "background-image"),
        Some("url(\"data:x;a:b\")")
    );
    assert!(declarations
        .iter()
        .filter(|declaration| declaration.name.starts_with("background-"))
        .all(|declaration| declaration.important));
    assert_eq!(get(&declarations, "content"), Some("\"a;b:c\""));
    let tokens = parse_declarations(r"--block:{a:b;c:d};--escaped:one\;two;color:red");
    assert_eq!(get(&tokens, "--block"), Some("{a:b;c:d}"));
    assert_eq!(get(&tokens, "--escaped"), Some(r"one\;two"));
    assert_eq!(get(&tokens, "color"), Some("red"));
}

#[test]
fn font_flex_gap_and_place_items_expand() {
    let declarations = parse_declarations(
        "font:italic 700 18px/1.4 Inter, sans-serif;flex:2 0 30%;flex-flow:column;gap:8px 12px;place-items:center stretch",
    );
    assert_eq!(get(&declarations, "font-weight"), Some("700"));
    assert_eq!(get(&declarations, "line-height"), Some("1.4"));
    assert_eq!(get(&declarations, "font-family"), Some("Inter, sans-serif"));
    assert_eq!(get(&declarations, "flex-grow"), Some("2"));
    assert_eq!(get(&declarations, "flex-shrink"), Some("0"));
    assert_eq!(get(&declarations, "row-gap"), Some("8px"));
    assert_eq!(get(&declarations, "column-gap"), Some("12px"));
    assert_eq!(get(&declarations, "align-items"), Some("center"));
    assert_eq!(get(&declarations, "justify-items"), Some("stretch"));
    assert_eq!(get(&declarations, "flex-direction"), Some("column"));
    assert_eq!(get(&declarations, "flex-wrap"), Some("nowrap"));
}

#[test]
fn font_shorthand_rejects_mixed_css_wide_keywords() {
    for value in [
        "700 15px inherit",
        "15px/INHERIT Arial",
        "15px Arial, revert-layer",
    ] {
        let declarations = parse_declarations(&format!("color:red;font:{value};opacity:.5"));
        assert!(
            declarations
                .iter()
                .all(|declaration| !declaration.name.starts_with("font-")),
            "mixed CSS-wide keyword should invalidate the whole font shorthand: {value}"
        );
        assert_eq!(get(&declarations, "color"), Some("red"));
        assert_eq!(get(&declarations, "opacity"), Some(".5"));
    }

    let whole_keyword = parse_declarations("font:inherit");
    assert_eq!(get(&whole_keyword, "font-family"), Some("inherit"));
    assert_eq!(get(&whole_keyword, "font-size"), Some("inherit"));

    let quoted_family = parse_declarations(r#"font:15px "inherit""#);
    assert_eq!(get(&quoted_family, "font-family"), Some(r#""inherit""#));
}

#[test]
fn text_decoration_none_resets_ua_underline() {
    let declarations = parse_declarations("text-decoration:none");
    assert_eq!(get(&declarations, "text-decoration-line"), Some("none"));
    assert_eq!(get(&declarations, "text-decoration-style"), Some("solid"));
    assert_eq!(
        get(&declarations, "text-decoration-color"),
        Some("currentcolor")
    );
    assert_eq!(
        get(&declarations, "text-decoration-thickness"),
        Some("auto")
    );
}

#[test]
fn background_collects_gradient_and_defers_variable_until_computed_value_time() {
    let declarations = parse_declarations(
        "background:linear-gradient(90deg,#000,#fff) center/cover no-repeat #123456",
    );
    assert_eq!(
        get(&declarations, "background-image"),
        Some("linear-gradient(90deg,#000,#fff)")
    );
    assert_eq!(get(&declarations, "background-color"), Some("#123456"));
    let variable = parse_declarations("background:var(--blue)");
    assert_eq!(get(&variable, "background-color"), Some("var(--blue)"));
    assert_eq!(get(&variable, "background-image"), Some("var(--blue)"));
    assert!(variable.iter().all(|declaration| {
        declaration.value == "var(--blue)"
            && declaration.deferred_shorthand.as_deref() == Some("background")
    }));
}

#[test]
fn webkit_gradient_text_properties_normalize_to_internal_longhands() {
    let declarations =
        parse_declarations("-webkit-background-clip:text;-webkit-text-fill-color:transparent");
    assert_eq!(get(&declarations, "background-clip"), Some("text"));
    assert_eq!(get(&declarations, "text-fill-color"), Some("transparent"));
    assert!(get(&declarations, "-webkit-text-fill-color").is_none());
}

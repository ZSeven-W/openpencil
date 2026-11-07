use super::*;

fn doc(json: &str) -> PenDocument {
    jian_ops_schema::load_str(json).expect("parse").value
}

fn kinds(doc: &PenDocument) -> Vec<InteractionIssueKind> {
    check_interactions(doc)
        .into_iter()
        .map(|i| i.kind)
        .collect()
}

/// The QA's generated product page, reduced: no state block, `$app`
/// bindings, a route and a layer target.
const PRODUCT: &str = r#"{"version":"1.0","children":[
  {"type":"frame","id":"home","screen":"/","children":[
    {"type":"frame","id":"minus","events":{"onTap":[{"set":{"$app.qty":"$app.qty > 1 ? $app.qty - 1 : 1"}}]}},
    {"type":"text","id":"value","content":"-","bindings":{"content":"to_str($app.qty)"}},
    {"type":"text","id":"label","content":"-","bindings":{"content":"$app.finish == 'cream' ? '奶油 $app.fake' : '绿色'"}},
    {"type":"text_input","id":"name","bindings":{"bind:value":"$app.name"}},
    {"type":"frame","id":"open","events":{"onTap":[{"push":"\"/detail\""},{"show":"missing-sheet"}]}}
  ]},
  {"type":"frame","id":"detail","screen":"/detail","children":[
    {"type":"frame","id":"back","events":{"onTap":[{"pop":null},{"push":"'/nowhere'"}]}}
  ]}
]}"#;

#[test]
fn reports_undeclared_state_routes_and_targets() {
    let issues = kinds(&doc(PRODUCT));
    for key in ["qty", "finish", "name"] {
        assert!(
            issues.contains(&InteractionIssueKind::UndeclaredState { key: key.into() }),
            "{key}: {issues:?}"
        );
    }
    assert!(
        !issues.contains(&InteractionIssueKind::UndeclaredState { key: "fake".into() }),
        "text inside a string literal is not a state read"
    );
    assert!(issues.contains(&InteractionIssueKind::UnknownTarget {
        target: "missing-sheet".into()
    }));
    assert!(issues.contains(&InteractionIssueKind::UnknownRoute {
        path: "/nowhere".into()
    }));
    assert!(!issues.contains(&InteractionIssueKind::UnknownRoute {
        path: "/detail".into()
    }));
}

#[test]
fn declared_state_and_valid_wiring_are_clean() {
    let src = PRODUCT.replace(
        r#"{"version":"1.0","#,
        r#"{"version":"1.0","state":{"qty":{"type":"int","default":1},"finish":{"type":"string","default":"cream"},"name":{"type":"string","default":""}},"#,
    )
    .replace(r#",{"show":"missing-sheet"}"#, "")
    .replace(r#",{"push":"'/nowhere'"}"#, "");
    assert_eq!(check_interactions(&doc(&src)), Vec::new());
}

#[test]
fn unsupported_bind_targets_are_reported() {
    let src = r#"{"version":"1.0","state":{"a":{"type":"string","default":""}},"children":[
      {"type":"text_input","id":"i","bindings":{"bind:value":"$route.q"}},
      {"type":"text_input","id":"j","bindings":{"bind:value":"$app.a.b"}}]}"#;
    let issues = kinds(&doc(src));
    assert_eq!(
        issues
            .iter()
            .filter(|k| matches!(k, InteractionIssueKind::UnsupportedBindTarget { .. }))
            .count(),
        2,
        "{issues:?}"
    );
}

#[test]
fn nested_if_branches_are_checked() {
    let src = r#"{"version":"1.0","children":[
      {"type":"frame","id":"f","events":{"onTap":[
        {"if":{"expr":"true","then":[{"hide":"ghost"}]}}]}}]}"#;
    assert!(
        kinds(&doc(src)).contains(&InteractionIssueKind::UnknownTarget {
            target: "ghost".into()
        })
    );
}

#[test]
fn audit_parallel_actions_report_missing_target() {
    let src = r#"{"version":"1.1","children":[{"type":"frame","id":"f","events":{"onTap":[{"parallel":[{"hide":"ghost"}]}]}}]}"#;
    assert!(
        kinds(&doc(src)).contains(&InteractionIssueKind::UnknownTarget {
            target: "ghost".into()
        })
    );
}

#[test]
fn audit_navigation_without_screens_is_not_valid() {
    let src = r#"{"version":"1.1","children":[{"type":"frame","id":"f","events":{"onTap":[{"push":"'/missing'"}]}}]}"#;
    assert!(
        kinds(&doc(src)).contains(&InteractionIssueKind::UnknownRoute {
            path: "/missing".into()
        })
    );
}

#[test]
fn audit_unsupported_binding_property_is_reported() {
    let src = r#"{"version":"1.1","state":{"count":{"type":"int","default":1}},"children":[{"type":"frame","id":"f","bindings":{"selected":"$app.count > 0"}}]}"#;
    assert!(
        !check_interactions(&doc(src)).is_empty(),
        "selected is not a supported binding property"
    );
}

#[test]
fn audit_literal_template_text_does_not_read_app_state() {
    let runtime = jian_core::Runtime::new();
    let expression = jian_core::expression::Expression::compile("`literal $app.fake`").unwrap();
    let (value, diagnostics) = expression.eval(&runtime.state, None, None);
    assert!(diagnostics.is_empty());
    assert_eq!(value.0, serde_json::json!("literal $app.fake"));
    let src = r#"{"version":"1.1","children":[{"type":"text","id":"t","content":"literal $app.fake","bindings":{"content":"`literal $app.fake`"}}]}"#;
    assert!(
        check_interactions(&doc(src)).is_empty(),
        "template text without interpolation is literal"
    );
}

#[test]
fn race_branches_and_their_state_reads_are_checked_once() {
    let src = r#"{"version":"1.1","children":[{"type":"frame","id":"f","events":{"onTap":[
      {"race":[[{"hide":"ghost"}],[{"set":{"$app.n":"$app.n + 1"}}]]}]}}]}"#;
    let issues = check_interactions(&doc(src));
    assert!(issues.iter().any(|i| i.kind
        == InteractionIssueKind::UnknownTarget {
            target: "ghost".into()
        }));
    let undeclared: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == InteractionIssueKind::UndeclaredState { key: "n".into() })
        .collect();
    assert_eq!(
        undeclared.len(),
        1,
        "nested reads are reported at their own site only: {issues:?}"
    );
}

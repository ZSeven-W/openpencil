use super::{initial_value, is_pure_app_expression, resolved_visibility, unguarded_app_key};
use jian_core::signal::scheduler::Scheduler;
use jian_core::state::StateGraph;
use serde_json::json;
use std::rc::Rc;

fn state() -> StateGraph {
    StateGraph::new(Rc::new(Scheduler::new()))
}

#[test]
fn selection_comparisons_report_the_unique_app_key() {
    for source in [
        "$app.tab == 'home' ? 'selected' : 'normal'",
        "$app.tab === 'home' ? 'selected' : 'normal'",
        "'home' != $app.tab ? 'normal' : 'selected'",
        "$app.tab !== 'home' ? 'normal' : 'selected'",
        "$app.tab == 'home' ? $app.tab : 'normal'",
    ] {
        assert_eq!(
            unguarded_app_key(source).as_deref(),
            Some("tab"),
            "{source}"
        );
    }
}

#[test]
fn arithmetic_and_template_reads_report_the_unique_app_key() {
    for source in [
        "$app.count + 1",
        "-$app.count * 2",
        "$app.count + $app.count",
        "`Count: ${$app.count}`",
        "`$app.fake: ${$app.count + 1}`",
    ] {
        assert_eq!(
            unguarded_app_key(source).as_deref(),
            Some("count"),
            "{source}"
        );
    }
}

#[test]
fn fallback_operators_suppress_the_entire_finding() {
    for source in [
        "$app.tab ?? 'home'",
        "$app.tab || 'home'",
        "$app.tab && 'selected'",
        "($app.tab ?? 'home') == 'home' ? 'selected' : 'normal'",
        "$app.tab == 'home' ? ($app.tab || 'home') : 'normal'",
        "`${$app.tab} ${$app.tab ?? 'home'}`",
        "[$app.tab, $app.tab && 'selected']",
    ] {
        assert_eq!(unguarded_app_key(source), None, "{source}");
        assert!(is_pure_app_expression(source), "{source}");
    }
}

#[test]
fn truthy_ternary_guards_include_repeated_negation() {
    for source in [
        "$app.tab ? 'selected' : 'normal'",
        "!$app.tab ? 'normal' : 'selected'",
        "!!$app.tab ? 'selected' : 'normal'",
        "!!!$app.tab ? 'normal' : 'selected'",
        "$app.tab == 'home' ? (!$app.tab ? 'normal' : 'selected') : 'normal'",
    ] {
        assert_eq!(unguarded_app_key(source), None, "{source}");
        assert!(is_pure_app_expression(source), "{source}");
    }
}

#[test]
fn null_comparison_guards_cover_both_sides_and_all_equalities() {
    for source in [
        "$app.tab == null",
        "$app.tab != null",
        "$app.tab === null",
        "$app.tab !== null",
        "null == $app.tab",
        "null != $app.tab",
        "null === $app.tab",
        "null !== $app.tab",
        "!!($app.tab != null) ? $app.tab : 'home'",
        "!($app.tab == null) ? $app.tab : 'home'",
        "$app.tab == 'home' ? 'selected' : ($app.tab == null ? 'normal' : $app.tab)",
    ] {
        assert_eq!(unguarded_app_key(source), None, "{source}");
        assert!(is_pure_app_expression(source), "{source}");
    }
}

#[test]
fn foreign_scopes_and_dynamic_access_invalidate_the_whole_expression() {
    for source in [
        "$viewport.width",
        "$page.tab",
        "$self.open",
        "$route.id",
        "$storage.token",
        "$vars.theme",
        "$app",
        "selected",
        "$app.tab + selected",
        "$app.tab + $viewport.width",
        "unknownFn($app.tab)",
        "len($app.items)",
        "$app['tab']",
        "$app.items[0]",
        "$app.user.name",
        "({value: $app.tab}).value",
        "[$app.tab, $page.tab]",
        "{value: $app.tab, host: $viewport.width}",
    ] {
        assert_eq!(unguarded_app_key(source), None, "{source}");
        assert!(!is_pure_app_expression(source), "{source}");
    }
}

#[test]
fn multiple_app_keys_are_pure_but_not_a_unique_missing_key() {
    for source in [
        "$app.tab == $app.selected",
        "$app.count + $app.total",
        "`${$app.first} ${$app.last}`",
        "{first: $app.first, last: $app.last}",
    ] {
        assert!(is_pure_app_expression(source), "{source}");
        assert_eq!(unguarded_app_key(source), None, "{source}");
    }
}

#[test]
fn literals_containing_app_text_do_not_create_reads() {
    for source in [
        "'$app.fake'",
        "`$app.fake`",
        "{label: '$app.fake'}",
        "['$app.fake', true, null]",
        "true",
        "null",
        "42",
    ] {
        assert!(is_pure_app_expression(source), "{source}");
        assert_eq!(unguarded_app_key(source), None, "{source}");
    }
    assert_eq!(
        unguarded_app_key("'$app.fake' + $app.real").as_deref(),
        Some("real")
    );
}

#[test]
fn pure_array_and_object_literals_inspect_their_values() {
    for source in [
        "[$app.tab, 'home']",
        "{active: $app.tab == 'home', nested: [$app.tab]}",
    ] {
        assert!(is_pure_app_expression(source), "{source}");
        assert_eq!(
            unguarded_app_key(source).as_deref(),
            Some("tab"),
            "{source}"
        );
    }
    assert_eq!(
        unguarded_app_key("{tab: $app.tab, fallback: $app.tab ?? 'home'}"),
        None
    );
}

#[test]
fn missing_selection_returns_the_normal_value_and_missing_state_warning() {
    let state = state();
    assert_eq!(
        initial_value("$app.tab == 'home' ? 'selected' : 'normal'", &state),
        Some((json!("normal"), true))
    );
    assert_eq!(initial_value("$app.tab", &state), Some((json!(null), true)));
}

#[test]
fn declared_selection_and_declared_null_have_no_missing_state_warning() {
    let state = state();
    state.app_set("tab", json!("home"));
    assert_eq!(
        initial_value("$app.tab == 'home' ? 'selected' : 'normal'", &state),
        Some((json!("selected"), false))
    );
    state.app_set("tab", json!(null));
    assert_eq!(
        initial_value("$app.tab == 'home' ? 'selected' : 'normal'", &state),
        Some((json!("normal"), false))
    );
}

#[test]
fn initial_value_rejects_other_diagnostics_and_parse_failures() {
    let state = state();
    for source in ["unknownFn()", "typeof()", "$app.tab + unknownFn()", "(true"] {
        assert_eq!(initial_value(source, &state), None, "{source}");
    }
}

#[test]
fn resolved_visibility_accepts_declared_guards_and_multiple_app_keys() {
    let state = state();
    state.app_set("tab", json!("home"));
    state.app_set("selected", json!("home"));
    state.app_set("enabled", json!(true));
    state.app_set("optional", json!(null));
    for source in [
        "$app.tab == $app.selected",
        "$app.enabled && $app.tab == 'home'",
        "$app.enabled || false",
        "$app.optional ?? true",
        "$app.optional == null",
        "$app.enabled ? true : false",
        "!!$app.enabled ? true : false",
        "true",
    ] {
        assert_eq!(
            resolved_visibility(source, &state),
            Some(json!(true)),
            "{source}"
        );
    }
    assert_eq!(resolved_visibility("false", &state), Some(json!(false)));
}

#[test]
fn resolved_visibility_rejects_undeclared_keys_even_in_unexecuted_branches() {
    let state = state();
    state.app_set("enabled", json!(true));
    for source in [
        "$app.missing",
        "$app.missing ?? true",
        "true || $app.missing",
        "false && $app.missing",
        "true ? true : $app.missing",
        "false ? $app.missing : true",
        "$app.enabled ? true : $app.missing",
    ] {
        assert_eq!(resolved_visibility(source, &state), None, "{source}");
    }
}

#[test]
fn resolved_visibility_rejects_foreign_scopes_and_dynamic_expressions() {
    let state = state();
    state.app_set("enabled", json!(true));
    state.app_set("user", json!({"visible": true}));
    state.app_set("items", json!([true]));
    for source in [
        "$viewport.width > 0",
        "$page.visible",
        "visible",
        "unknownFn()",
        "to_bool($app.enabled)",
        "$app['enabled']",
        "$app.user.visible",
        "$app.items[0]",
        "true || $viewport.width",
        "(true",
    ] {
        assert_eq!(resolved_visibility(source, &state), None, "{source}");
    }
}

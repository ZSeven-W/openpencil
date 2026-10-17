use super::*;
use serde_json::json;

fn state(document: Value) -> EditorState {
    EditorState::from_document(serde_json::from_value(document).unwrap())
}

fn single_binding(source: &str) -> Value {
    json!({"version":"1.0.0","children":[{"type":"frame","id":"section","children":[
        {"type":"text","id":"value","content":"1","bindings":{"content":source}},
        {"type":"frame","id":"increase","events":{"onTap":[{"set":{"$app.qty":"$app.qty + 1"}}]}}
    ]}]})
}

fn findings(document: Value) -> Vec<InitialBindingFinding> {
    missing_initial_bindings(&state(document), &["section".into()])
}

#[test]
fn selection_and_quantity_patterns_report_actual_initial_values_without_mutation() {
    let document = json!({"version":"1.0.0","state":{"scrolled":{"type":"bool","default":false}},
    "children":[{"type":"frame","id":"section","children":[
        {"type":"frame","id":"choice","bindings":{
            "fill":"$app.specColor == '云白' ? '#2563EB15' : '#FFFFFF'",
            "stroke":"$app.specColor == '云白' ? '#2563EB' : '#E5E7EB'"},
            "events":{"onTap":[{"set":{"$app.specColor":"\"云白\""}}]}},
        {"type":"text","id":"choice-label","content":"云白",
            "bindings":{"fontWeight":"$app.specColor == '云白' ? 600 : 500"}},
        {"type":"text","id":"quantity","content":"1","bindings":{"content":"\"\" + $app.qty"}},
        {"type":"frame","id":"minus","events":{"onTap":[{"if":{"expr":"$app.qty > 1",
            "then":[{"set":{"$app.qty":"$app.qty - 1"}}]}}]}},
        {"type":"frame","id":"plus","events":{"onTap":[{"set":{"$app.qty":"$app.qty + 1"}}]}}
    ]}]});
    let state = state(document);
    let before = serde_json::to_value(&state.doc).unwrap();
    let found = missing_initial_bindings(&state, &["section".into()]);
    assert_eq!(found.len(), 3, "{found:?}");
    let quantity = found
        .iter()
        .find(|finding| finding.node_id == "quantity")
        .unwrap();
    assert_eq!(quantity.state_key, "qty");
    assert_eq!(quantity.initial_value, json!(""));
    assert_eq!(quantity.authored_value, Some(json!("1")));
    let paint = found
        .iter()
        .find(|finding| finding.property == "fill")
        .unwrap();
    assert_eq!(paint.initial_value, json!("#FFFFFF"));
    assert!(found.iter().all(|finding| finding.property != "fontWeight"));
    assert_eq!(serde_json::to_value(&state.doc).unwrap(), before);
}

#[test]
fn unsupported_font_weight_binding_does_not_override_static_text_weight() {
    // Preview skips unknown targets before evaluating their expression.
    assert!(jian_core::binding::BindingTarget::parse("fontWeight").is_none());
    for property in ["content", "fill", "stroke"] {
        assert!(jian_core::binding::BindingTarget::parse(property).is_some());
    }
    let document = json!({"version":"1.0.0","children":[{
        "type":"frame","id":"section","children":[{
            "type":"text","id":"choice-label","content":"云白","fontWeight":600,
            "bindings":{"fontWeight":"$app.specColor == '云白' ? 600 : 500"},
            "events":{"onTap":[{"set":{"$app.specColor":"'云白'"}}]}
        }]
    }]});
    let state = state(document);
    let before = serde_json::to_value(&state.doc).unwrap();
    assert!(missing_initial_bindings(&state, &["section".into()]).is_empty());
    assert_eq!(serde_json::to_value(&state.doc).unwrap(), before);
}

#[test]
fn arbitrary_keys_and_target_value_set_shape_are_supported() {
    let mut doc = single_binding("'' + $app.seatCount");
    doc["children"][0]["children"][1]["events"]["onTap"] =
        json!([{"set":{"target":"$app.seatCount","value":"$app.seatCount + 1"}}]);
    assert_eq!(findings(doc)[0].state_key, "seatCount");
    let mut doc = single_binding("'' + $app.qty");
    doc["children"][0]["children"][1]["events"]["onTap"] = json!([{"set":{"$state.qty":"2"}}]);
    assert_eq!(
        findings(doc).len(),
        1,
        "$state write aliases App in the runtime"
    );
}

#[test]
fn declared_null_and_omitted_defaults_are_not_missing_declarations() {
    for entry in [
        json!({"type":"int","default":null}),
        json!({"type":"int"}),
        json!({"type":"int","default":1}),
    ] {
        let mut doc = single_binding("'' + $app.qty");
        doc["state"] = json!({"qty":entry});
        assert!(findings(doc).is_empty());
    }
}

#[test]
fn assembled_cross_section_and_append_state_resolve_the_same_read() {
    for owned in [
        vec!["section".into()],
        vec!["section".into(), "other".into()],
    ] {
        let mut doc = single_binding("'' + $app.qty");
        doc["state"] = json!({"qty":{"type":"int","default":7}});
        doc["children"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type":"frame","id":"other"}));
        let state = state(doc);
        let before = state.doc.state.clone();
        assert!(missing_initial_bindings(&state, &owned).is_empty());
        assert_eq!(state.doc.state, before);
    }
    let mut doc = single_binding("'' + $app.qty");
    // A self-scope declaration does not initialize an explicit app-scope read.
    doc["children"][0]["state"] = json!({"qty":{"type":"int","default":1}});
    assert_eq!(findings(doc).len(), 1);
}

#[test]
fn explicit_fallbacks_and_uncertain_scopes_are_deferred() {
    for source in [
        "$app.qty ?? 1",
        "$app.qty || 1",
        "$app.qty && 'Ready'",
        "$app.qty ? $app.qty : 'Select quantity'",
        "$app.qty == null ? '' : $app.qty",
        "null !== $app.qty ? $app.qty : 'Loading'",
        "!$app.qty ? 'Empty' : $app.qty",
        "$state.qty",
        "$self.qty",
        "$page.qty",
        "$app.qty + $route.price",
        "$app.qty + $event.value",
        "$app.qty + $app.price",
        "$app['qty']",
        "$app.cart.qty",
        "String($app.qty)",
        "$app.qty == selected ? 'yes' : 'no'",
    ] {
        assert!(findings(single_binding(source)).is_empty(), "{source}");
    }
    assert_eq!(
        findings(single_binding("$app.qty == 1 ? 'Selected' : 'Normal'")).len(),
        1
    );
}

#[test]
fn reads_need_a_valid_local_user_writer_and_only_owned_nodes_are_reported() {
    let mut doc = single_binding("'' + $app.qty");
    doc["children"][0]["children"][1]["events"] = json!({});
    assert!(findings(doc).is_empty());
    let mut doc = single_binding("'' + $app.qty");
    doc["children"][0]["children"][1]["events"]["onTap"] = json!([{"set":{"$app.qty":1}}]);
    assert!(
        findings(doc).is_empty(),
        "invalid set RHS is not an executable local writer"
    );
    let state = state(single_binding("'' + $app.qty"));
    assert!(missing_initial_bindings(&state, &["missing".into()]).is_empty());
    assert!(missing_initial_bindings(&state, &["increase".into()]).is_empty());
    assert_eq!(missing_initial_bindings(&state, &["value".into()]).len(), 1);
}

#[test]
fn lifecycle_initializers_defer_but_disabled_and_later_hooks_do_not() {
    for hook in ["onLaunch", "onMount"] {
        let mut doc = single_binding("'' + $app.qty");
        let target = if hook == "onLaunch" {
            &mut doc
        } else {
            &mut doc["children"][0]
        };
        target["lifecycle"] = json!({hook:[{"set":{"$app.qty":"1"}}]});
        assert!(findings(doc).is_empty(), "{hook}");
    }
    let mut doc = single_binding("'' + $app.qty");
    let children = doc["children"].take();
    doc["pages"] = json!([{"id":"page","name":"Page","children":children,
        "lifecycle":{"onEnter":[{"set":{"$app.qty":"1"}}]}}]);
    doc["children"] = json!([]);
    assert!(findings(doc).is_empty());
    for hooks in [
        json!({"onMount":[{"set":{"$app.qty":"1"}}],"disabledEvents":["onMount"]}),
        json!({"onUnmount":[{"set":{"$app.qty":"1"}}]}),
        json!({"onResume":[{"set":{"$app.qty":"1"}}]}),
    ] {
        let mut doc = single_binding("'' + $app.qty");
        doc["children"][0]["lifecycle"] = hooks;
        assert_eq!(findings(doc).len(), 1);
    }
}

#[test]
fn asynchronous_conditional_and_host_initialization_are_deferred() {
    for list in [
        json!([{"delay":{"ms":100}},{"set":{"$app.qty":"1"}}]),
        json!([{"if":{"expr":"$route.ready","then":[{"set":{"$app.qty":"1"}}]}}]),
        json!([{"fetch":{"url":"'/quantity'","into":"$app.qty"}}]),
        json!([{"call":{"module":"host","method":"seed"}}]),
    ] {
        let mut doc = single_binding("'' + $app.qty");
        doc["lifecycle"] = json!({"onLaunch":list});
        assert!(findings(doc).is_empty());
    }
    let mut doc = single_binding("'' + $app.qty");
    doc["children"][0]["children"][1]["events"]["onTap"] =
        json!([{"set":{"$app.qty":"$event.value"}}]);
    assert!(findings(doc).is_empty());
}

#[test]
fn initially_hidden_or_unknown_visibility_sites_are_not_rejected() {
    for visibility in [
        json!({"visible":false}),
        json!({"opacity":0}),
        json!({"bindings":{"visible":"$route.ready"}}),
        json!({"bindings":{"visible":"$app.ready"}}),
        json!({"opacity":"$app.alpha"}),
    ] {
        let mut doc = single_binding("'' + $app.qty");
        for (key, value) in visibility.as_object().unwrap() {
            doc["children"][0][key] = value.clone();
        }
        assert!(findings(doc).is_empty());
    }
    let mut doc = single_binding("'' + $app.qty");
    doc["state"] = json!({"ready":{"type":"bool","default":true}});
    doc["children"][0]["bindings"] = json!({"visible":"$app.ready"});
    assert_eq!(findings(doc).len(), 1);
}

#[test]
fn duplicate_ids_and_unknown_binding_properties_are_not_fatal() {
    let mut doc = single_binding("'' + $app.qty");
    doc["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"frame","id":"section"}));
    assert!(findings(doc).is_empty());
    for property in ["visible", "width", "value", "bind:value"] {
        let mut doc = single_binding("'' + $app.qty");
        doc["children"][0]["children"][0]["bindings"] = json!({property:"$app.qty"});
        assert!(findings(doc).is_empty(), "{property}");
    }
}

#[test]
fn invalid_event_chain_and_host_originated_or_async_writers_are_deferred() {
    for list in [
        json!([{"set":{"$app.qty":"1"}},{"if":{"else":[]}}]),
        json!([{"fetch":{"url":"'/other'","into":"$app.other"}},{"set":{"$app.qty":"1"}}]),
    ] {
        let mut doc = single_binding("'' + $app.qty");
        doc["children"][0]["children"][1]["events"]["onTap"] = list;
        assert!(findings(doc).is_empty());
    }
    for hook in ["onFocus", "onScroll"] {
        let mut doc = single_binding("'' + $app.qty");
        doc["children"][0]["children"][1]["events"][hook] = json!([{"set":{"$app.qty":"1"}}]);
        assert!(findings(doc).is_empty(), "{hook}");
    }
}

#[test]
fn lifecycle_visibility_changes_only_defer_the_affected_subtree() {
    for action in [
        json!({"hide":"section"}),
        json!({"toggle_visibility":{"target":"value"}}),
        json!({"animate":{"target":"value","from":{"opacity":0},"to":{"opacity":1},"duration":200}}),
    ] {
        let mut doc = single_binding("'' + $app.qty");
        doc["lifecycle"] = json!({"onLaunch":[action]});
        assert!(findings(doc).is_empty());
    }
    let mut doc = single_binding("'' + $app.qty");
    doc["lifecycle"] = json!({"onLaunch":[{"hide":"unrelated"}]});
    assert_eq!(findings(doc).len(), 1);
    let mut doc = single_binding("'' + $app.qty");
    doc["lifecycle"] = json!({"onLaunch":[{"hide":"section"}],"disabledEvents":["onLaunch"]});
    assert_eq!(findings(doc).len(), 1);
}

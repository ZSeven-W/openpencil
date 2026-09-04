//! Regressions for the turn-scoped root contract: which turns may inherit a
//! live screen's artboard, and what an inherited profile is allowed to rewrite.

use super::*;

#[test]
fn continuation_guard_survives_existing_only_batch_and_normalizes_later_roots() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(
        serde_json::from_value(serde_json::json!({
            "type": "frame", "id": "home", "name": "Nocturne 今夜",
            "width": 390, "height": 844,
            "fill": [{ "type": "solid", "color": "#050508" }],
            "children": [{ "type": "text", "id": "home-title", "content": "今夜天空" }]
        }))
        .expect("existing mobile screen"),
    );
    // The prompt is what makes this a continuation: it names the sibling
    // screens it promises. A turn that merely runs on a non-empty canvas must
    // not inherit anything.
    let mut guard = RootSeedGuard::from_prompt("手机上继续生成 星图、观测计划 两个界面");

    // DeepSeek may spend its first batch editing the old screen. That must
    // neither mutate its chrome nor consume the contract needed by siblings.
    let (first, first_mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"note=I(\"home\",{type:'text',name:'Existing screen note',content:'keep going',width:120,height:20})"}"#,
        None,
        Some(&mut guard),
    );
    assert!(!first.is_error, "first batch failed: {}", first.content);
    assert!(first_mutated);
    assert_eq!(state.active_children().len(), 1);
    assert!(state.active_children()[0]
        .children()
        .into_iter()
        .flatten()
        .all(|child| child.base().role.as_deref() != Some("status-bar")));

    let (second, second_mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r##"{"operations":"star=I(null,{type:'frame',name:'星图',width:1512,height:982,fill:[{type:'solid',color:'#16002E'}]})"}"##,
        None,
        Some(&mut guard),
    );
    assert!(!second.is_error, "second batch failed: {}", second.content);
    assert!(second_mutated);

    let (third, third_mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"plan=I(null,{type:'frame',name:'观测计划',width:375,height:812})"}"#,
        None,
        Some(&mut guard),
    );
    assert!(!third.is_error, "third batch failed: {}", third.content);
    assert!(third_mutated);

    for name in ["星图", "观测计划"] {
        let root = state
            .active_children()
            .iter()
            .find(|node| node.base().name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("missing generated screen {name}"));
        assert_eq!(
            (root.width_px(), root.height_px()),
            (Some(390.0), Some(844.0)),
            "{name} must inherit the live artboard"
        );
        // 星图 chose its own background and KEEPS it — an inherited artboard
        // fixes the frame size, not the palette. 观测计划 authored none, so it
        // inherits the newest live screen's, which by then is 星图's.
        assert_eq!(
            op_editor_core::first_solid_fill_hex(root),
            Some("#16002E"),
            "{name} background"
        );
        assert_eq!(
            root.children()
                .and_then(|children| children.first())
                .and_then(|child| child.base().role.as_deref()),
            Some("status-bar"),
            "{name} must receive canonical mobile chrome"
        );
    }
}

/// The empty-canvas twin of this test cannot fail: with nothing to match
/// against, the seed profile is the default one, which already leaves authored
/// numbers alone. The regression only exists once a matching screen is on the
/// canvas — an ORDINARY turn there used to inherit its artboard and stamp the
/// model's explicit width, height AND fill.
#[test]
fn ordinary_turn_on_a_populated_canvas_keeps_every_authored_root_value() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(
        serde_json::from_value(serde_json::json!({
            "type": "frame", "id": "home", "name": "Home",
            "width": 390, "height": 844,
            "fill": [{ "type": "solid", "color": "#050508" }],
            "children": [{ "type": "text", "id": "home-title", "content": "Home" }]
        }))
        .expect("existing mobile screen"),
    );
    // No promised sibling screens -> not a continuation.
    let mut guard = RootSeedGuard::from_prompt("mobile app");
    let (result, mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r##"{"operations":"wide=I(null,{type:'frame',name:'Wide',width:1200,height:1600,fill:[{type:'solid',color:'#FFFFFF'}]})"}"##,
        None,
        Some(&mut guard),
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let wide = state
        .active_children()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("Wide"))
        .expect("authored root");
    assert_eq!(
        (wide.width_px(), wide.height_px()),
        (Some(1200.0), Some(1600.0)),
        "authored dimensions must survive an ordinary turn"
    );
    assert_eq!(
        op_editor_core::first_solid_fill_hex(wide),
        Some("#FFFFFF"),
        "authored background must survive an ordinary turn"
    );
}

/// The guard stays pending across a continuation turn, so without this the
/// stamping applied to every root for the rest of the turn. A turn with no
/// continuation signal must consume it after the first batch, exactly as a
/// fresh-design turn always did.
#[test]
fn ordinary_turn_on_a_populated_canvas_consumes_the_guard_after_one_batch() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(
        serde_json::from_value(serde_json::json!({
            "type": "frame", "id": "home", "name": "Home",
            "width": 390, "height": 844,
            "children": [{ "type": "text", "id": "home-title", "content": "Home" }]
        }))
        .expect("existing mobile screen"),
    );
    let mut guard = RootSeedGuard::from_prompt("mobile app");
    for operations in [
        r#"{"operations":"first=I(null,{type:'frame',name:'First',width:390,height:844})"}"#,
        r#"{"operations":"second=I(null,{type:'frame',name:'Second',width:'fit_content',height:'fit_content'})"}"#,
    ] {
        let (result, mutated) = execute_design_tool_with_root_seed_guard(
            &mut state,
            "batch_design",
            operations,
            None,
            Some(&mut guard),
        );
        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
    }
    let second = state
        .active_children()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("Second"))
        .expect("second top-level frame");
    assert_eq!(
        (second.width_px(), second.height_px()),
        (None, None),
        "the guard must not survive an ordinary turn's first batch"
    );
}

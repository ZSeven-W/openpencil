//! Root-seed + mobile status-bar chrome tests for the design-agent
//! `batch_design` surface — split out of `design_agent_tools_tests.rs`
//! at the 800-line cap. Still a child module of `design_agent_tools`,
//! so `super::` paths resolve unchanged.

use super::*;

#[test]
fn execute_design_first_batch_seeds_mobile_sizeless_root() {
    let mut state = EditorState::new();
    let mut guard = RootSeedGuard::from_prompt("travel itinerary app");
    let (result, mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',name:'Mobile Page'})"}"#,
        None,
        Some(&mut guard),
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let root = only_root_frame(&state);
    assert_eq!(root.width_px(), Some(390.0));
    assert_eq!(root.height_px(), Some(844.0));
    assert!(root_frame_layout_is_vertical(root));
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(
        v["layoutHint"]
            .as_str()
            .unwrap_or("")
            .contains("root seeded to 390x844"),
        "seed hint must be visible to the next batch: {}",
        result.content
    );
}

#[test]
fn execute_design_first_batch_seeds_desktop_sizeless_root() {
    let mut state = EditorState::new();
    let mut guard = RootSeedGuard::from_prompt("build a SaaS analytics dashboard");
    let (result, mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',name:'Dashboard'})"}"#,
        None,
        Some(&mut guard),
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let root = only_root_frame(&state);
    assert_eq!(root.width_px(), Some(1440.0));
    assert_eq!(root.height_px(), Some(900.0));
    assert!(root_frame_layout_is_vertical(root));
}

#[test]
fn execute_design_root_seed_preserves_authored_numeric_width() {
    let mut state = EditorState::new();
    let mut guard = RootSeedGuard::from_prompt("mobile hotel booking");
    let (result, mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',name:'Phone',width:320,height:'fit_content'})"}"#,
        None,
        Some(&mut guard),
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let root = only_root_frame(&state);
    assert_eq!(
        root.width_px(),
        Some(320.0),
        "authored numeric width must stay untouched"
    );
    assert_eq!(root.height_px(), Some(844.0));
}

#[test]
fn continuation_seed_inherits_every_mobile_screen_and_repairs_wrong_numeric_sizes() {
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
    // The artboard is inherited because the REQUEST promises sibling screens,
    // not because the canvas happens to hold a frame.
    let mut guard = RootSeedGuard::from_prompt("手机上继续生成 星图、观测计划、我的3个界面");
    let (result, mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"a=I(null,{type:'frame',name:'星图',width:1512,height:982,fill:[{type:'solid',color:'#16002E'}]})\nb=I(null,{type:'frame',name:'观测计划'})\nc=I(null,{type:'frame',name:'我的',width:375,height:812})"}"#,
        None,
        Some(&mut guard),
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let generated = &state.active_children()[1..];
    assert_eq!(
        generated.len(),
        3,
        "top-level roots: {:?}; result: {}",
        state
            .active_children()
            .iter()
            .map(|node| node.base().name.as_deref())
            .collect::<Vec<_>>(),
        result.content
    );
    for root in generated {
        assert_eq!(
            (root.width_px(), root.height_px()),
            (Some(390.0), Some(844.0))
        );
        assert_eq!(
            root.children()
                .and_then(|children| children.first())
                .and_then(|child| child.base().role.as_deref()),
            Some("status-bar")
        );
        // The artboard is a contract; the background is authored intent. Only
        // the roots the model left unfilled inherit the live screen's colour.
        let expected_fill = match root.base().name.as_deref() {
            Some("星图") => "#16002E",
            _ => "#050508",
        };
        assert_eq!(
            op_editor_core::first_solid_fill_hex(root),
            Some(expected_fill),
            "{:?}",
            root.base().name
        );
    }
    let value: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(value["layoutHint"]
        .as_str()
        .unwrap_or("")
        .contains("390x844"));
}

#[test]
fn execute_design_mobile_first_batch_injects_status_bar_chrome() {
    // Chrome parity with the orchestrator scaffold: even when the model
    // authored explicit root dimensions (so size seeding is skipped),
    // the mobile root still gets the pre-inserted status bar as its
    // FIRST child, and the hint tells the model not to build another.
    let mut state = EditorState::new();
    let mut guard = RootSeedGuard::from_prompt("mobile fitness tracker home");
    let (result, mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',name:'Home',width:390,height:844,layout:'vertical'})"}"#,
        None,
        Some(&mut guard),
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let root = only_root_frame(&state);
    let first = &root.children().expect("root children")[0];
    assert_eq!(
        first.base().role.as_deref(),
        Some("status-bar"),
        "status bar must be the root's first child"
    );
    assert_eq!(first.base().name.as_deref(), Some("Status Bar"));
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(
        v["layoutHint"]
            .as_str()
            .unwrap_or("")
            .contains("do NOT create another status bar"),
        "chrome hint must be visible to the next batch: {}",
        result.content
    );
}

#[test]
fn execute_design_mobile_batch_canonicalizes_model_authored_status_bar() {
    // The model built its own status bar in the first batch — the guard
    // must NOT stack a second one, and the hand-rolled variant is
    // replaced in place with the canonical chrome (every measured
    // hand-built bar deviated visibly from the iOS reference).
    let mut state = EditorState::new();
    let mut guard = RootSeedGuard::from_prompt("iphone travel app");
    let (result, mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',name:'Screen',width:390,height:844,layout:'vertical',children:[{type:'frame',name:'Status Bar',width:'fill_container',height:62}]})"}"#,
        None,
        Some(&mut guard),
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let root = only_root_frame(&state);
    let bars = root
        .children()
        .expect("root children")
        .iter()
        .filter(|c| {
            c.base()
                .name
                .as_deref()
                .is_some_and(|n| n.to_ascii_lowercase().contains("status bar"))
        })
        .count();
    assert_eq!(bars, 1, "must not stack a second status bar");
    let bar = root
        .children()
        .expect("root children")
        .iter()
        .find(|c| c.base().role.as_deref() == Some("status-bar"))
        .expect("model-built bar replaced with the canonical status bar");
    assert!(
        bar.children().is_some_and(|children| children
            .iter()
            .any(|c| c.base().name.as_deref() == Some("Levels"))),
        "canonical bar carries the Time/Levels structure"
    );
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(
        v["layoutHint"]
            .as_str()
            .unwrap_or("")
            .contains("replaced with the standard iOS status bar"),
        "replacement must be echoed so the model stops restyling it: {}",
        result.content
    );
}

#[test]
fn execute_design_desktop_first_batch_gets_no_status_bar() {
    let mut state = EditorState::new();
    let mut guard = RootSeedGuard::from_prompt("SaaS analytics web app dashboard");
    let (result, mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',name:'Dashboard'})"}"#,
        None,
        Some(&mut guard),
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let root = only_root_frame(&state);
    let has_bar = root
        .children()
        .into_iter()
        .flatten()
        .any(|c| c.base().role.as_deref() == Some("status-bar"));
    assert!(!has_bar, "desktop roots must not get mobile chrome");
}

#[test]
fn execute_design_root_seed_guard_consumes_after_first_successful_batch() {
    let mut state = EditorState::new();
    let mut guard = RootSeedGuard::from_prompt("phone onboarding flow");
    let (first, first_mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',name:'Root',width:390,height:844})"}"#,
        None,
        Some(&mut guard),
    );
    assert!(!first.is_error, "first batch failed: {}", first.content);
    assert!(first_mutated);

    let (second, second_mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"second=I(null,{type:'frame',name:'Second',width:'fit_content',height:'fit_content'})"}"#,
        None,
        Some(&mut guard),
    );

    assert!(!second.is_error, "second batch failed: {}", second.content);
    assert!(second_mutated);
    let second_root = state
        .active_children()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("Second"))
        .expect("second top-level frame exists");
    assert_eq!(
        second_root.width_px(),
        None,
        "second batch must not be seeded after the first success"
    );
    assert_eq!(
        second_root.height_px(),
        None,
        "second batch must not be seeded after the first success"
    );
    let v: serde_json::Value = serde_json::from_str(&second.content).unwrap();
    assert!(
        v.get("layoutHint")
            .and_then(|h| h.as_str())
            .is_none_or(|hint| !hint.contains("root seeded")),
        "second batch must not get another root seed hint: {}",
        second.content
    );
}

#[test]
fn execute_design_tool_without_loop_root_seed_guard_does_not_seed() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_design_tool(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',name:'Plain',width:'fit_content',height:'fit_content'})"}"#,
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let root = only_root_frame(&state);
    assert_eq!(root.width_px(), None);
    assert_eq!(root.height_px(), None);
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(
        v.get("layoutHint")
            .and_then(|h| h.as_str())
            .is_none_or(|hint| !hint.contains("root seeded")),
        "non-loop path must not inject root seed feedback: {}",
        result.content
    );
}

fn only_root_frame(state: &EditorState) -> &PenNode {
    let children = state.active_children();
    assert_eq!(children.len(), 1, "expected a single root frame");
    let root = &children[0];
    assert!(matches!(root, PenNode::Frame(_)), "expected frame root");
    root
}

fn root_frame_layout_is_vertical(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    matches!(
        frame.container.layout,
        Some(jian_ops_schema::node::container::LayoutMode::Vertical)
    )
}

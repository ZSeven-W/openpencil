//! Design-agent tool tests. Split out of `design_agent_tools.rs` (inline
//! `mod tests`) to keep that file under the 800-line cap; still a child
//! module of `design_agent_tools`, so `super::` paths resolve unchanged.

#[test]
fn web_app_prompts_are_not_mobile_seeded() {
    // A dashboard "web app" is a desktop product — it must not be
    // seeded 390x844 (regression: the bare "app" word-match did).
    assert!(!super::root_seed_prompt_is_mobile(
        "Technical dashboard web app for a utilities company"
    ));
    assert!(!super::root_seed_prompt_is_mobile(
        "Luxury webapp for managing barbershop clients"
    ));
}

#[test]
fn app_prompts_are_mobile_seeded() {
    assert!(super::root_seed_prompt_is_mobile(
        "Technical dashboard app for a utilities company"
    ));
    // "web app" phrasing is covered (negatively) by
    // web_app_prompts_are_not_mobile_seeded below.
    assert!(super::root_seed_prompt_is_mobile(
        "mobile companion for our web app"
    ));
    assert!(super::root_seed_prompt_is_mobile(
        "phone booking flow for a travel brand"
    ));
    assert!(super::root_seed_prompt_is_mobile(
        "Design a travel booking mobile app explore page"
    ));
    assert!(super::root_seed_prompt_is_mobile("设计一个手机端首页"));
}

use super::*;

#[test]
fn design_tool_defs_cover_all_14_tools_with_schema_parity() {
    let defs = design_tool_defs();

    // All 15 tools are present, every one MCP-sourced.
    assert_eq!(defs.len(), 15, "expected 15 design tool defs");
    for (name, _) in DESIGN_TOOLS {
        assert!(
            defs.iter().any(|d| d.name == *name),
            "missing design tool def for {name}"
        );
    }

    // PARITY: for each tool, the input_schema_json in the def must equal
    // the inputSchema value from TOOL_SCHEMAS (as parsed JSON), so
    // in-process defs stay byte-equal to the MCP server.
    for def in defs.iter() {
        // Find the matching TOOL_SCHEMAS entry.
        let schema_entry = schemas::TOOL_SCHEMAS
            .iter()
            .find(|entry| {
                let v: serde_json::Value = serde_json::from_str(entry).unwrap();
                v.get("name").and_then(|n| n.as_str()) == Some(def.name.as_str())
            })
            .unwrap_or_else(|| panic!("design tool {} not found in TOOL_SCHEMAS", def.name));

        // Extract the canonical inputSchema from TOOL_SCHEMAS.
        let canonical: serde_json::Value = serde_json::from_str(schema_entry).unwrap();
        let canonical_schema = canonical
            .get("inputSchema")
            .unwrap_or_else(|| panic!("TOOL_SCHEMAS entry for {} missing inputSchema", def.name));

        // Parse the def's input_schema_json and compare as Value.
        let def_schema: serde_json::Value = serde_json::from_str(&def.input_schema_json)
            .unwrap_or_else(|e| panic!("def.input_schema_json for {} unparseable: {e}", def.name));

        assert_eq!(
            def_schema, *canonical_schema,
            "inputSchema mismatch for {}: in-process def != TOOL_SCHEMAS",
            def.name
        );
    }

    // Every DESIGN_TOOLS entry must exist in TOOL_SCHEMAS (no orphans).
    for (name, _) in DESIGN_TOOLS {
        let found = schemas::TOOL_SCHEMAS.iter().any(|entry| {
            let v: serde_json::Value = serde_json::from_str(entry).unwrap();
            v.get("name").and_then(|n| n.as_str()) == Some(*name)
        });
        assert!(found, "design tool {name} is not in TOOL_SCHEMAS — orphan!");
    }
}

#[test]
fn execute_design_rejects_tools_outside_the_design_set() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_design_tool(&mut state, "delete_page", "{}");
    assert!(result.is_error);
    assert!(!mutated);
    assert!(result.content.contains("not available in design agent"));
}

#[test]
fn execute_design_read_tool_returns_success_envelope() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_design_tool(&mut state, "get_editor_state", "{}");
    assert!(!result.is_error, "got {}", result.content);
    assert!(!mutated, "read tools never mutate");
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert_eq!(v["success"], serde_json::Value::Bool(true));
}

#[test]
fn execute_design_batch_design_inserts_frame_and_mutates() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_design_tool(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',width:120,height:80})"}"#,
    );
    assert!(!result.is_error, "batch_design failed: {}", result.content);
    assert!(mutated, "batch_design must mutate the document");

    // The active page must now have at least one child (the inserted frame).
    assert!(
        !state.active_children().is_empty(),
        "doc must have a frame after batch_design"
    );
}

#[test]
fn execute_design_batch_design_registers_reveals_when_epoch_is_set() {
    use op_editor_core::agent_indicators;

    let _guard = agent_indicators::test_guard();
    agent_indicators::clear();
    let epoch = agent_indicators::begin();
    let mut state = EditorState::new();
    let (result, mutated) = execute_design_tool_with_reveals(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',name:'Root',width:120,height:80})\nbox=I(root,{type:'rectangle',name:'Box',width:80,height:20})"}"#,
        Some(epoch),
    );
    assert!(!result.is_error, "batch_design failed: {}", result.content);
    assert!(mutated, "batch_design must mutate the document");

    let ids: Vec<String> = collect_active_node_ids(&state).into_iter().collect();
    assert!(ids.len() >= 2, "batch inserted a subtree, got {ids:?}");
    let snapshot = agent_indicators::snapshot();
    for id in ids {
        assert!(
            snapshot.reveals.contains_key(&id),
            "newly inserted node {id} should have a reveal: {:?}",
            snapshot.reveals
        );
    }
    agent_indicators::end_if_epoch(epoch);
    agent_indicators::clear();
}

#[test]
fn execute_design_batch_design_attaches_per_batch_layout_feedback() {
    // A batch that lands an OVERFLOWING table (5×240 fixed columns in a
    // 600px root) must come back with `layoutIssues` — the per-batch
    // geometry feedback the model repairs in-process.
    let mut state = EditorState::new();
    let ops = r#"{"operations":"root=I(null,{\"type\":\"frame\",\"name\":\"Page\",\"width\":600,\"height\":\"fit_content\",\"layout\":\"vertical\",\"children\":[{\"type\":\"frame\",\"name\":\"Client Table\",\"layout\":\"vertical\",\"width\":\"fill_container\",\"children\":[{\"type\":\"frame\",\"name\":\"Row\",\"layout\":\"horizontal\",\"gap\":16,\"width\":\"fill_container\",\"height\":24,\"children\":[{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20}]},{\"type\":\"frame\",\"name\":\"Row\",\"layout\":\"horizontal\",\"gap\":16,\"width\":\"fill_container\",\"height\":24,\"children\":[{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20}]}]}]})"}"#;
    let (result, mutated) = execute_design_tool(&mut state, "batch_design", ops);
    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    let issues = v["layoutIssues"].as_array().expect("layoutIssues attached");
    assert!(
        issues
            .iter()
            .any(|i| i.as_str().unwrap_or("").contains("column widths")),
        "table overflow reported, got {issues:?}"
    );
    assert!(v["layoutHint"].is_string(), "actionable hint attached");
}

#[test]
fn execute_design_clean_batch_attaches_no_layout_feedback() {
    // A geometrically clean batch must NOT carry layoutIssues noise.
    let mut state = EditorState::new();
    let (result, mutated) = execute_design_tool(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',width:400,height:300})"}"#,
    );
    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(
        v.get("layoutIssues").is_none(),
        "clean layout must not attach issues: {}",
        result.content
    );
}

#[test]
fn flat_script_reports_missing_layout_after_the_forest_is_assembled() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_design_tool(
        &mut state,
        "batch_design",
        r#"{"script":"const section=I(null,{type:'frame',name:'Popular',width:360,height:240}); I(section,{type:'text',name:'Title',content:'Popular'}); I(section,{type:'frame',name:'Rail',layout:'horizontal',width:'fill_container',height:180});"}"#,
    );
    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);

    let value: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    let questions = value["intentQuestions"]
        .as_array()
        .expect("missing layout is reported after flat I(parent,obj) assembly");
    assert!(questions.iter().any(|question| question
        .as_str()
        .is_some_and(|line| line.contains("Popular") && line.contains("no layout"))));

    let root = state.active_children().first().expect("section inserted");
    let PenNode::Frame(frame) = root else {
        panic!("expected frame")
    };
    assert_eq!(
        frame.container.layout, None,
        "reporting ambiguity must not silently write vertical"
    );
}

#[test]
fn contrast_scanner_flags_bad_pair() {
    let bad_root: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Card",
        "fill": [{ "type": "solid", "color": "#888888" }],
        "children": [{
            "type": "text",
            "id": "title",
            "name": "Title",
            "content": "Low contrast",
            "fill": [{ "type": "solid", "color": "#777777" }]
        }]
    }))
    .unwrap();
    let issues = scan_contrast_issues(&[bad_root], None, &std::collections::BTreeMap::new());
    assert_eq!(issues.len(), 1, "exactly one bad text/background pair");
    assert_eq!(issues[0].node_id, "title");
    assert_eq!(issues[0].node_name.as_deref(), Some("Title"));
    assert_eq!(issues[0].fg, "#777777");
    assert_eq!(issues[0].bg, "#888888");
    assert_eq!(issues[0].target, 4.5);
    assert!(issues[0].ratio < issues[0].target);

    let passing_root: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Card",
        "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        "children": [{
            "type": "text",
            "id": "title",
            "name": "Title",
            "content": "Readable",
            "fill": [{ "type": "solid", "color": "#111111" }]
        }]
    }))
    .unwrap();
    assert!(
        scan_contrast_issues(&[passing_root], None, &std::collections::BTreeMap::new()).is_empty()
    );
}

#[test]
fn contrast_scanner_resolves_tokens_and_alpha_for_icons() {
    let root: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame", "id": "root", "name": "Search", "layout": "horizontal",
        "fill": [{"type":"solid","color":"$--accent"}], "children": [{
            "type": "frame", "id": "filter", "name": "Filter", "layout": "horizontal",
            "fill": [{"type":"solid","color":"#EA580C15"}], "children": [{
                "type": "icon_font", "id": "icon", "name": "Filter Icon",
                "iconFontName": "sliders-horizontal", "width": 20, "height": 20,
                "fill": [{"type":"solid","color":"$--white"}]
            }]
        }]
    }))
    .unwrap();
    let variables: std::collections::BTreeMap<
        String,
        jian_ops_schema::variable::VariableDefinition,
    > = serde_json::from_value(serde_json::json!({
        "--accent": {"type":"color","value":"#F5F5F5"},
        "--white": {"type":"color","value":"#FFFFFF"}
    }))
    .unwrap();

    let issues = scan_contrast_issues(
        &[root],
        Some(&variables),
        &std::collections::BTreeMap::new(),
    );

    let issue = issues
        .iter()
        .find(|issue| issue.node_id == "icon")
        .expect("white icon on a translucent orange tint is reported");
    assert_eq!(issue.fg, "#FFFFFF");
    assert_eq!(issue.target, CONTRAST_ICON_TARGET);
    assert!(issue.ratio < issue.target);
}

#[test]
fn contrast_scanner_accounts_for_fill_and_node_opacity() {
    let root: PenNode = serde_json::from_value(serde_json::json!({
        "type":"frame", "id":"root", "fill":[{"type":"solid","color":"#000000"}],
        "children":[
            {"type":"text", "id":"fill-opacity", "content":"Dimmed",
             "fill":[{"type":"solid","color":"#FFFFFF","opacity":0.2}]},
            {"type":"text", "id":"node-opacity", "content":"Also dimmed", "opacity":0.2,
             "fill":[{"type":"solid","color":"#FFFFFF"}]}
        ]
    }))
    .unwrap();

    let issues = scan_contrast_issues(&[root], None, &std::collections::BTreeMap::new());

    assert!(issues.iter().any(|issue| issue.node_id == "fill-opacity"));
    assert!(issues.iter().any(|issue| issue.node_id == "node-opacity"));
}

#[test]
fn design_quality_returns_more_than_twelve_contrast_issues_but_stays_bounded() {
    let labels = (0..70)
        .map(|index| {
            serde_json::json!({
                "type": "text",
                "id": format!("label-{index}"),
                "name": format!("Label {index}"),
                "content": "Low contrast",
                "fill": [{"type": "solid", "color": "#AAAAAA"}]
            })
        })
        .collect::<Vec<_>>();
    let document: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "width": 390, "height": 844,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": labels
        }]
    }))
    .expect("document");
    let state = EditorState::from_document(document);

    let report = collect_design_quality(&state);
    assert_eq!(report.contrast_issues.len(), MAX_CONTRAST_ISSUES);
    assert_eq!(report.contrast_issues[0].node_id, "label-0");
    assert_eq!(report.contrast_issues[63].node_id, "label-63");
}

#[test]
fn batch_design_image_slot_feedback_requires_the_exact_slot_id() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_design_tool(
        &mut state,
        "batch_design",
        r##"{"operations":"root=I(null,{type:'frame',name:'Playlist',layout:'vertical',width:320,height:200,fill:[{type:'solid',color:'#111111'}],children:[{type:'frame',name:'Cover',layout:'none',width:56,height:56,fill:[{type:'solid',color:'#222222'}]}]})"}"##,
    );
    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);

    let value: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(
        value["imageSlots"].as_array().is_some_and(|slots| slots
            .iter()
            .any(|slot| slot.as_str().is_some_and(|line| line.contains("Cover")))),
        "explicit empty cover slot is surfaced: {}",
        result.content
    );
    let hint = value["layoutHint"].as_str().unwrap_or("");
    assert!(hint.contains("exact EMPTY slot id"), "{hint}");
    assert!(hint.contains("never its row/card container"), "{hint}");
}

#[test]
#[cfg_attr(
    target_os = "windows",
    ignore = "Windows CI aborts in native text geometry while attaching batch contrast feedback"
)]
fn batch_design_result_carries_contrast_issues() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_design_tool(
        &mut state,
        "batch_design",
        r##"{"operations":"root=I(null,{type:'frame',name:'Card',width:320,height:120,fill:[{type:'solid',color:'#888888'}],children:[{type:'text',name:'Title',content:'Low contrast',width:180,height:24,fill:[{type:'solid',color:'#777777'}]}]})"}"##,
    );
    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);

    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    let issues = v["contrastIssues"]
        .as_array()
        .expect("contrastIssues attached");
    assert!(!issues.is_empty(), "bad contrast pair reported");
    assert_eq!(issues[0]["nodeName"], "Title");
    assert_eq!(issues[0]["fg"], "#777777");
    assert_eq!(issues[0]["bg"], "#888888");
    assert!(issues[0]["ratio"].as_f64().unwrap() < issues[0]["target"].as_f64().unwrap());
    assert!(
        v["contrastHint"]
            .as_str()
            .unwrap_or("")
            .contains("text: 1 below 4.5:1"),
        "actionable contrast hint attached: {}",
        result.content
    );
}

// --- execute_agent_tool tests ---

#[test]
fn execute_agent_tool_routes_design_tool_to_design_surface() {
    // batch_design is a design-only tool — it must execute and mutate
    // via the design surface, not the CRUD surface.
    let mut state = EditorState::new();
    let (result, mutated) = execute_agent_tool(
        &mut state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',width:80,height:60})"}"#,
    );
    assert!(
        !result.is_error,
        "batch_design via agent router failed: {}",
        result.content
    );
    assert!(mutated, "batch_design must mutate via the design surface");
    assert!(
        !state.active_children().is_empty(),
        "a frame must exist after batch_design via execute_agent_tool"
    );
}

#[test]
fn execute_agent_tool_routes_crud_tool_to_chat_surface() {
    // delete_node is a CRUD-only tool — it must route to execute_chat_tool.
    // With an unknown nodeId the chat surface returns an error (node not found),
    // which proves the CRUD path was taken rather than the design path that
    // would have returned "not available in design agent".
    let mut state = EditorState::new();
    let (result, mutated) = execute_agent_tool(&mut state, "delete_node", r#"{"nodeId":"nope"}"#);
    // The CRUD surface returns an error for an unknown node — NOT "not available in design agent".
    assert!(result.is_error, "unknown node delete must error");
    assert!(!mutated);
    assert!(
        !result.content.contains("not available in design agent"),
        "must have taken the CRUD path, not the design path"
    );
}

#[test]
fn execute_agent_tool_unknown_name_returns_not_available_error() {
    // A name outside both sets falls through to execute_chat_tool
    // which returns "not available in chat".
    let mut state = EditorState::new();
    let (result, mutated) = execute_agent_tool(&mut state, "delete_page", "{}");
    assert!(result.is_error);
    assert!(!mutated);
    assert!(
        result.content.contains("not available in chat"),
        "unknown tools should report the CRUD surface's 'not available in chat' error, got: {}",
        result.content
    );
}

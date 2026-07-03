use super::*;

#[test]
fn parse_args_maps_ts_design_dsl_flags_to_operations_arg() {
    let dsl = r#"root=I(null, {"type":"frame","name":"Page","width":320,"height":240})"#;
    let p = parse_args(&[
        "design".to_string(),
        dsl.to_string(),
        "--canvas-width".to_string(),
        "375".to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse design dsl");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "batch_design".to_string(),
            args: vec![
                ("operations".to_string(), dsl.to_string()),
                ("postProcess".to_string(), "true".to_string()),
                ("canvasWidth".to_string(), "375".to_string()),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_keeps_json_array_design_as_nodes_json() {
    let nodes = r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":10}]"#;
    let p = parse_args(&["design".to_string(), nodes.to_string()]).expect("parse design array");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "batch_design".to_string(),
            args: vec![
                ("nodes_json".to_string(), nodes.to_string()),
                ("postProcess".to_string(), "true".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_design_content_to_section_children_args() {
    let payload = r#"{"children":[{"type":"text","name":"Title","content":"Hello","width":120,"height":24}]}"#;
    let p = parse_args(&[
        "design:content".to_string(),
        "section-1".to_string(),
        payload.to_string(),
        "--canvas-width".to_string(),
        "375".to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse design content");

    match p.command {
        Command::ToolCallJson { tool, args_json } => {
            assert_eq!(tool, "design_content");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&args_json).expect("args json"),
                serde_json::json!({
                    "sectionId": "section-1",
                    "children": [
                        {
                            "type": "text",
                            "name": "Title",
                            "content": "Hello",
                            "width": 120,
                            "height": 24,
                        },
                    ],
                    "canvasWidth": 375,
                    "pageId": "page-2",
                })
            );
        }
        other => panic!("expected structured design_content call, got {other:?}"),
    }
}

#[test]
fn parse_args_maps_design_skeleton_to_root_frame_sections_args() {
    let payload = r##"{"rootFrame":{"name":"Login","width":375,"height":812,"fill":[{"type":"solid","color":"#FFFFFF"}]},"sections":[{"name":"Hero","height":240,"padding":[24,24],"role":"hero"}]}"##;
    let p = parse_args(&[
        "design:skeleton".to_string(),
        payload.to_string(),
        "--canvas-width".to_string(),
        "375".to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse design skeleton");

    match p.command {
        Command::ToolCallJson { tool, args_json } => {
            assert_eq!(tool, "design_skeleton");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&args_json).expect("args json"),
                serde_json::json!({
                    "rootFrame": {
                        "name": "Login",
                        "width": 375,
                        "height": 812,
                        "fill": [{"type": "solid", "color": "#FFFFFF"}],
                    },
                    "sections": [
                        {
                            "name": "Hero",
                            "height": 240,
                            "padding": [24, 24],
                            "role": "hero",
                        },
                    ],
                    "canvasWidth": 375,
                    "pageId": "page-2",
                })
            );
        }
        other => panic!("expected structured design_skeleton call, got {other:?}"),
    }
}

#[test]
fn parse_args_maps_design_script_flag_to_script_arg() {
    let script = r#"I(null, {"type":"frame","name":"Page","width":320,"height":240});"#;
    // `--script` must trail the payload: like other boolean flags
    // (`--post-process`), a leading `--script` would otherwise swallow the
    // next non-flag token as its own inline value instead of leaving it as
    // the design positional.
    let p = parse_args(&[
        "design".to_string(),
        script.to_string(),
        "--script".to_string(),
    ])
    .expect("parse design script flag");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "batch_design".to_string(),
            args: vec![
                ("script".to_string(), script.to_string()),
                ("postProcess".to_string(), "true".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_design_js_file_payload_to_script_arg() {
    let dir = std::env::temp_dir();
    let file_path = dir.join(format!(
        "op-cli-design-script-test-{}.js",
        std::process::id()
    ));
    let script = r#"I(null, {"type":"frame","name":"Page","width":320,"height":240});"#;
    std::fs::write(&file_path, script).expect("write temp script file");

    let payload = format!("@{}", file_path.display());
    let p = parse_args(&["design".to_string(), payload]).expect("parse design js file payload");

    std::fs::remove_file(&file_path).ok();

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "batch_design".to_string(),
            args: vec![
                ("script".to_string(), script.to_string()),
                ("postProcess".to_string(), "true".to_string()),
            ],
        }
    );
}

#[test]
fn design_js_file_payload_detection_predicate_matches_without_flag() {
    // Mirrors the `script_requested` predicate in `map_design_like` for
    // payloads we don't want to round-trip through the filesystem here.
    let payload = "@/tmp/does-not-matter.js".to_string();
    let is_script =
        payload.starts_with('@') && (payload.ends_with(".js") || payload.ends_with(".mjs"));
    assert!(is_script);

    let mjs_payload = "@/tmp/does-not-matter.mjs".to_string();
    let is_script = mjs_payload.starts_with('@')
        && (mjs_payload.ends_with(".js") || mjs_payload.ends_with(".mjs"));
    assert!(is_script);
}

// `parse_args_maps_ts_design_dsl_flags_to_operations_arg` (above, top of
// file) and `parse_args_keeps_json_array_design_as_nodes_json` already pin
// the non-script regression cases: a bare DSL payload still maps to
// `operations`, and a `[`-prefixed payload still maps to `nodes_json`, when
// `--script` is absent and the payload isn't an `@*.js` / `@*.mjs` file.

#[test]
fn parse_args_maps_design_refine_root_id_args() {
    let p = parse_args(&[
        "design:refine".to_string(),
        "--root-id".to_string(),
        "root-1".to_string(),
        "--canvas-width".to_string(),
        "375".to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse design refine");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "design_refine".to_string(),
            args: vec![
                ("rootId".to_string(), "root-1".to_string()),
                ("canvasWidth".to_string(), "375".to_string()),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}

use super::*;
use std::path::PathBuf;

fn temp_json(name: &str, contents: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "op-cli-conversion-{name}-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, contents).expect("write temp json");
    path.to_string_lossy().into_owned()
}

fn temp_out_dir(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("op-cli-skill-export-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&path).ok();
    path
}

#[test]
fn design_upsert_vars_maps_to_tool_call() {
    let vars = temp_json(
        "vars",
        r##"{"color/primary":{"type":"color","value":"#3366ff"}}"##,
    );
    let parsed = parse_args(&[
        "design:upsert-vars".into(),
        "--key".into(),
        "tokens:theme.css".into(),
        "--file".into(),
        vars.clone(),
        "--source".into(),
        "src/theme.css".into(),
        "--hash".into(),
        "h1".into(),
    ])
    .expect("parse design:upsert-vars");
    std::fs::remove_file(vars).ok();

    match parsed.command {
        Command::ToolCall { tool, args } => {
            assert_eq!(tool, "upsert_variables");
            assert!(args.contains(&("key".into(), "tokens:theme.css".into())));
            assert!(args
                .iter()
                .any(|(key, value)| key == "variables" && value.contains("color/primary")));
            assert!(args.contains(&("sourcePath".into(), "src/theme.css".into())));
            assert!(args.contains(&("sourceHash".into(), "h1".into())));
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
}

#[test]
fn design_upsert_component_requires_name() {
    let node = temp_json("component", r#"{"type":"frame","id":"button"}"#);
    let parsed = parse_args(&[
        "design:upsert-component".into(),
        "--key".into(),
        "src/Button.tsx#Button".into(),
        "--file".into(),
        node.clone(),
    ]);
    std::fs::remove_file(node).ok();

    assert!(parsed.is_err());
}

#[test]
fn design_upsert_screen_maps_payload_file() {
    let node = temp_json("screen", r#"{"type":"frame","id":"home"}"#);
    let parsed = parse_args(&[
        "design:upsert-screen".into(),
        "--key".into(),
        "route:/".into(),
        "--file".into(),
        node.clone(),
    ])
    .expect("parse design:upsert-screen");
    std::fs::remove_file(node).ok();

    assert_eq!(
        parsed.command,
        Command::ToolCall {
            tool: "upsert_screen".into(),
            args: vec![
                ("key".into(), "route:/".into()),
                ("node_json".into(), r#"{"type":"frame","id":"home"}"#.into()),
            ],
        }
    );
}

#[test]
fn design_status_and_lint_map_to_read_tools() {
    let status = parse_args(&["design:status".into(), "--kind".into(), "component".into()])
        .expect("parse design:status");
    assert_eq!(
        status.command,
        Command::ToolCall {
            tool: "conversion_status".into(),
            args: vec![("kind".into(), "component".into())],
        }
    );

    let lint = parse_args(&["design:lint".into(), "--node".into(), "n1".into()])
        .expect("parse design:lint");
    assert_eq!(
        lint.command,
        Command::ToolCall {
            tool: "lint_document".into(),
            args: vec![("nodeId".into(), "n1".into())],
        }
    );
}

#[test]
fn skill_export_maps_to_cli_command() {
    let parsed = parse_args(&[
        "skill:export".into(),
        "code-to-design".into(),
        "--out".into(),
        ".tmp-skills".into(),
    ])
    .expect("parse skill:export");

    assert_eq!(
        parsed.command,
        Command::SkillExport {
            name: "code-to-design".into(),
            out_dir: Some(".tmp-skills".into()),
        }
    );
}

#[test]
fn skill_export_writes_claude_skill_md() {
    let out_dir = temp_out_dir("code-to-design");
    let output = crate::skill_export_cli::run_export(
        "code-to-design",
        Some(out_dir.to_string_lossy().as_ref()),
    )
    .expect("export code-to-design skill");
    let skill_path = out_dir.join("code-to-design").join("SKILL.md");
    let contents = std::fs::read_to_string(&skill_path).expect("read exported skill");
    std::fs::remove_dir_all(&out_dir).ok();

    assert!(output.contains("SKILL.md"));
    assert!(contents.starts_with("---\nname: code-to-design\n"));
    assert!(contents.contains("Five-step"));
    assert!(contents.contains("upsert_variables"));
}

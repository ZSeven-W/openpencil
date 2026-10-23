use super::*;

// Lines captured from `kimi -p … --output-format stream-json` (2.3.1).
const VERSION: &str = r#"{"role":"meta","type":"system.version","version":"2.3.1"}"#;
const ANSWER: &str = r#"{"role":"assistant","content":"1+1 等于 2；可调用的工具名：无。"}"#;
const RESUME: &str = r#"{"role":"meta","type":"session.resume_hint","session_id":"session_7cb09134","command":"kimi -r session_7cb09134","content":"To resume this session: kimi -r session_7cb09134"}"#;

#[test]
fn only_assistant_messages_carry_text() {
    assert_eq!(parse_kimi_line(VERSION), None);
    assert_eq!(
        parse_kimi_line(ANSWER),
        Some(ChatDelta::TextDelta(
            "1+1 等于 2；可调用的工具名：无。".into()
        ))
    );
    // The resume hint has a `content` field too; it is not the answer.
    assert_eq!(parse_kimi_line(RESUME), None);
    assert_eq!(
        parse_kimi_line(r#"{"role":"tool","content":"result"}"#),
        None
    );
    assert_eq!(parse_kimi_line("Loading MCP servers…"), None);
    assert_eq!(
        parse_kimi_line(r#"{"role":"assistant","content":""}"#),
        None
    );
}

#[test]
fn text_parts_are_joined_and_other_parts_dropped() {
    let line = r#"{"role":"assistant","content":[{"type":"text","text":"{\"a\":"},{"type":"image_url","image_url":{}},{"type":"text","text":"1}"}]}"#;
    assert_eq!(
        parse_kimi_line(line),
        Some(ChatDelta::TextDelta("{\"a\":1}".into()))
    );
}

#[test]
fn bare_model_ids_get_the_cli_alias_prefix() {
    assert_eq!(model_alias("k3.1-pro"), "kimi-code/k3.1-pro");
    assert_eq!(model_alias("kimi-code/k3.1-pro"), "kimi-code/k3.1-pro");
}

#[test]
fn generation_turns_select_a_tool_free_agent_in_an_empty_workspace() {
    let root = std::env::temp_dir().join(format!("op-kimi-agent-test-{}", std::process::id()));
    let agent = root.join("generation-agent.md");
    let workspace = root.join("workspace");
    write_generation_files(&agent, &workspace).unwrap();
    let written = std::fs::read_to_string(&agent).unwrap();
    assert!(written.starts_with("---\nname: openpencil-generation\n"));
    assert!(written.contains("\ntools: []\n"));
    assert!(std::fs::read_dir(&workspace).unwrap().next().is_none());
    assert_eq!(
        args_for(&agent),
        [
            "--output-format",
            "stream-json",
            "--agent-file",
            agent.to_string_lossy().as_ref()
        ]
    );
    let _ = std::fs::remove_dir_all(root);
}

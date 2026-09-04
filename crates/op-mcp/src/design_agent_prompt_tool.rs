//! Read-only MCP access to the complete, task-matched design-agent prompt.
//!
//! `get_design_prompt` exposes individual reference sections. This tool
//! exposes the actual host-aware system prompt used by OpenPencil's design
//! loop, including the generation skills resolved from the user's request.

use std::collections::BTreeMap;

use op_ai_skills::{design_agent_system_prompt_with_skills_for, DesignVerifyProtocol};

use super::{McpTool, ToolErrorCode, ToolOutcome};

pub struct GetDesignAgentPrompt;

impl McpTool for GetDesignAgentPrompt {
    fn name(&self) -> &str {
        "get_design_agent_prompt"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let user_message = match args.get("userMessage").map(String::as_str).map(str::trim) {
            Some(message) if !message.is_empty() => message,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "userMessage is required".into(),
                )
            }
        };
        let (verify, verify_name) = match args
            .get("verifyProtocol")
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("screenshot")
        {
            "screenshot" => (DesignVerifyProtocol::Screenshot, "screenshot"),
            "layout" => (DesignVerifyProtocol::LayoutSnapshot, "layout"),
            other => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("verifyProtocol must be \"screenshot\" or \"layout\", got {other:?}"),
                )
            }
        };
        let result = serde_json::json!({
            "prompt": design_agent_system_prompt_with_skills_for(user_message, verify),
            "verifyProtocol": verify_name,
        });
        ToolOutcome::OkJson(result.to_string())
    }
}

pub fn get_design_agent_prompt_snapshot() -> GetDesignAgentPrompt {
    GetDesignAgentPrompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_message_resolves_domain_depth_and_defaults_to_screenshot() {
        let args = BTreeMap::from([(
            "userMessage".to_string(),
            "Design an analytics dashboard with a client table".to_string(),
        )]);
        let ToolOutcome::OkJson(json) = get_design_agent_prompt_snapshot().call(&args) else {
            panic!("expected nested JSON prompt result");
        };
        let value: serde_json::Value = serde_json::from_str(&json).expect("prompt JSON");
        assert_eq!(value["verifyProtocol"], "screenshot");
        assert!(value["prompt"]
            .as_str()
            .is_some_and(|prompt| prompt.contains("DASHBOARD / ADMIN / DATA-TABLE DEPTH")));
    }

    #[test]
    fn layout_protocol_removes_screenshot_requirement() {
        let args = BTreeMap::from([
            (
                "userMessage".to_string(),
                "Design a mobile form".to_string(),
            ),
            ("verifyProtocol".to_string(), "layout".to_string()),
        ]);
        let ToolOutcome::OkJson(json) = get_design_agent_prompt_snapshot().call(&args) else {
            panic!("expected nested JSON prompt result");
        };
        let value: serde_json::Value = serde_json::from_str(&json).expect("prompt JSON");
        assert_eq!(value["verifyProtocol"], "layout");
        let prompt = value["prompt"].as_str().expect("prompt string");
        assert!(prompt.contains("This host cannot render screenshots"));
        assert!(prompt.contains("THREE-SECTION ARCHITECTURE"));
    }

    #[test]
    fn rejects_missing_message_and_unknown_protocol() {
        assert!(matches!(
            get_design_agent_prompt_snapshot().call(&BTreeMap::new()),
            ToolOutcome::Err(ToolErrorCode::MissingArgument, _)
        ));
        let args = BTreeMap::from([
            ("userMessage".to_string(), "Design a page".to_string()),
            ("verifyProtocol".to_string(), "video".to_string()),
        ]);
        assert!(matches!(
            get_design_agent_prompt_snapshot().call(&args),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
        ));
    }
}

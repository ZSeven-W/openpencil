//! Read-only MCP exposure for the design loop's detect-only quality scans.

use std::collections::BTreeMap;

use op_editor_core::EditorState;
use op_mcp::{McpTool, ToolOutcome};

pub struct GetDesignQuality {
    state: EditorState,
}

pub fn get_design_quality_snapshot(state: &EditorState) -> GetDesignQuality {
    GetDesignQuality {
        state: state.clone(),
    }
}

impl McpTool for GetDesignQuality {
    fn name(&self) -> &str {
        "get_design_quality"
    }

    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let report = op_chat_agent::design_agent_tools::collect_design_quality(&self.state);
        match serde_json::to_string(&report) {
            Ok(json) => ToolOutcome::OkJson(json),
            Err(error) => ToolOutcome::Err(
                op_mcp::ToolErrorCode::Internal,
                format!("serialize design quality failed: {error}"),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_result_is_nested_and_non_destructive() {
        let doc: jian_ops_schema::PenDocument = serde_json::from_str(
            r#"{"version":"1.0","children":[{"type":"frame","id":"root","name":"Root","width":390,"height":844,"children":[{"type":"frame","id":"shell","name":"Content Shell","width":390,"height":120,"children":[]}]}]}"#,
        )
        .expect("document");
        let state = EditorState::from_document(doc);
        let before = serde_json::to_string(&state.doc).expect("before");
        let ToolOutcome::OkJson(json) = get_design_quality_snapshot(&state).call(&BTreeMap::new())
        else {
            panic!("expected nested JSON quality report");
        };
        let value: serde_json::Value = serde_json::from_str(&json).expect("quality JSON");
        for key in [
            "geometryIssues",
            "layoutIssues",
            "contrastIssues",
            "iconIssues",
            "structureIssues",
            "emptyShells",
            "intentQuestions",
            "variableIssues",
            "imageSlots",
            "navIssues",
        ] {
            assert!(value[key].is_array(), "missing array field {key}: {value}");
        }
        assert!(value["emptyShells"].as_array().is_some_and(|items| {
            items.iter().any(|item| {
                item.as_str()
                    .is_some_and(|shell| shell.contains("Content Shell"))
            })
        }));
        assert_eq!(
            serde_json::to_string(&state.doc).expect("after"),
            before,
            "quality read must not mutate state"
        );
    }
}

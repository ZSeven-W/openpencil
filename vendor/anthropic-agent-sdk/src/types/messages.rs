//! Message types for conversations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::identifiers::SessionId;
use super::introspection::{ModelUsage, SDKPermissionDenial};

// ============================================================================
// AskUserQuestion Tool Types
// ============================================================================

/// Option for a question in `AskUserQuestion`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Display text for this option
    pub label: String,
    /// Explanation of what this option means
    pub description: String,
}

/// Question specification for `AskUserQuestion`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionSpec {
    /// The complete question to ask the user
    pub question: String,
    /// Short label displayed as a chip/tag (max 12 chars)
    pub header: String,
    /// Available choices (2-4 options)
    pub options: Vec<QuestionOption>,
    /// Whether multiple options can be selected
    pub multi_select: bool,
}

/// Input for the `AskUserQuestion` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserQuestionInput {
    /// Questions to ask the user (1-4 questions)
    pub questions: Vec<QuestionSpec>,
    /// User answers collected (populated when tool result returns)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answers: Option<HashMap<String, String>>,
}

/// Output from the `AskUserQuestion` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserQuestionOutput {
    /// User's answers keyed by question header
    pub answers: HashMap<String, String>,
}

// ============================================================================
// Message Types
// ============================================================================

/// Content value for tool results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentValue {
    /// String content
    String(String),
    /// Structured content blocks
    Blocks(Vec<serde_json::Value>),
}

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content block
    Text {
        /// Text content
        text: String,
    },
    /// Thinking content block (extended thinking)
    Thinking {
        /// Thinking content
        thinking: String,
        /// Signature for verification
        signature: String,
    },
    /// Tool use request
    ToolUse {
        /// Tool use ID
        id: String,
        /// Tool name
        name: String,
        /// Tool input parameters
        input: serde_json::Value,
    },
    /// Tool execution result
    ToolResult {
        /// ID of the tool use this is a result for
        tool_use_id: String,
        /// Result content
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ContentValue>,
        /// Whether this is an error result
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

/// User message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageContent {
    /// Message role (always "user")
    pub role: String,
    /// Message content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<UserContent>,
}

/// User content can be string or blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserContent {
    /// Plain string content
    String(String),
    /// Structured content blocks
    Blocks(Vec<ContentBlock>),
}

/// Assistant message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageContent {
    /// Model that generated the message
    pub model: String,
    /// Message content blocks
    pub content: Vec<ContentBlock>,
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// User message
    User {
        /// Parent tool use ID for nested conversations
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
        /// Message content
        message: UserMessageContent,
        /// Session ID
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<SessionId>,
        /// Checkpoint UUID for file rewind (requires `--replay-user-messages`)
        #[serde(skip_serializing_if = "Option::is_none")]
        uuid: Option<String>,
    },
    /// Assistant message
    Assistant {
        /// Parent tool use ID for nested conversations
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
        /// Message content
        message: AssistantMessageContent,
        /// Session ID
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<SessionId>,
    },
    /// System message
    System {
        /// System message subtype
        subtype: String,
        /// Additional system message data
        #[serde(flatten)]
        data: serde_json::Value,
    },
    /// Result message with metrics
    Result {
        /// Result subtype (success, `error_max_turns`, `error_during_execution`, etc.)
        subtype: String,
        /// Total duration in milliseconds
        duration_ms: u64,
        /// API call duration in milliseconds
        duration_api_ms: u64,
        /// Whether this is an error result
        is_error: bool,
        /// Number of conversation turns
        num_turns: u32,
        /// Session ID
        session_id: SessionId,
        /// Total cost in USD
        #[serde(skip_serializing_if = "Option::is_none")]
        total_cost_usd: Option<f64>,
        /// Token usage statistics (aggregate)
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<serde_json::Value>,
        /// Result message (for success subtype)
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<String>,
        /// Per-model usage statistics
        #[serde(
            rename = "modelUsage",
            default,
            skip_serializing_if = "HashMap::is_empty"
        )]
        model_usage: HashMap<String, ModelUsage>,
        /// List of denied tool uses
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        permission_denials: Vec<SDKPermissionDenial>,
        /// Structured output (when outputFormat is specified)
        #[serde(skip_serializing_if = "Option::is_none")]
        structured_output: Option<serde_json::Value>,
        /// Error messages (for error subtypes)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        errors: Vec<String>,
    },
    /// Stream event for partial messages
    StreamEvent {
        /// Event UUID
        uuid: String,
        /// Session ID
        session_id: SessionId,
        /// Raw stream event data
        event: serde_json::Value,
        /// Parent tool use ID
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_option_serde() {
        let opt = QuestionOption {
            label: "Option A".to_string(),
            description: "First option".to_string(),
        };

        let json = serde_json::to_string(&opt).unwrap();
        assert!(json.contains("Option A"));
        assert!(json.contains("First option"));

        let parsed: QuestionOption = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.label, "Option A");
        assert_eq!(parsed.description, "First option");
    }

    #[test]
    fn test_question_spec_serde() {
        let spec = QuestionSpec {
            question: "Which approach?".to_string(),
            header: "Approach".to_string(),
            options: vec![
                QuestionOption {
                    label: "A".to_string(),
                    description: "Option A".to_string(),
                },
                QuestionOption {
                    label: "B".to_string(),
                    description: "Option B".to_string(),
                },
            ],
            multi_select: false,
        };

        let json = serde_json::to_string(&spec).unwrap();
        // camelCase for multiSelect
        assert!(json.contains("multiSelect"));
        assert!(!json.contains("multi_select"));

        let parsed: QuestionSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.question, "Which approach?");
        assert_eq!(parsed.header, "Approach");
        assert_eq!(parsed.options.len(), 2);
        assert!(!parsed.multi_select);
    }

    #[test]
    fn test_question_spec_multi_select() {
        let spec = QuestionSpec {
            question: "Select features".to_string(),
            header: "Features".to_string(),
            options: vec![
                QuestionOption {
                    label: "Feature 1".to_string(),
                    description: "First feature".to_string(),
                },
                QuestionOption {
                    label: "Feature 2".to_string(),
                    description: "Second feature".to_string(),
                },
            ],
            multi_select: true,
        };

        let json = serde_json::to_string(&spec).unwrap();
        let parsed: QuestionSpec = serde_json::from_str(&json).unwrap();
        assert!(parsed.multi_select);
    }

    #[test]
    fn test_ask_user_question_input_serde() {
        let input = AskUserQuestionInput {
            questions: vec![QuestionSpec {
                question: "Which library?".to_string(),
                header: "Library".to_string(),
                options: vec![
                    QuestionOption {
                        label: "tokio".to_string(),
                        description: "Async runtime".to_string(),
                    },
                    QuestionOption {
                        label: "async-std".to_string(),
                        description: "Alternative runtime".to_string(),
                    },
                ],
                multi_select: false,
            }],
            answers: None,
        };

        let json = serde_json::to_string(&input).unwrap();
        // answers should be omitted when None
        assert!(!json.contains("answers"));

        let parsed: AskUserQuestionInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.questions.len(), 1);
        assert!(parsed.answers.is_none());
    }

    #[test]
    fn test_ask_user_question_input_with_answers() {
        let mut answers = HashMap::new();
        answers.insert("Library".to_string(), "tokio".to_string());

        let input = AskUserQuestionInput {
            questions: vec![QuestionSpec {
                question: "Which library?".to_string(),
                header: "Library".to_string(),
                options: vec![QuestionOption {
                    label: "tokio".to_string(),
                    description: "Async runtime".to_string(),
                }],
                multi_select: false,
            }],
            answers: Some(answers),
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("answers"));
        assert!(json.contains("tokio"));

        let parsed: AskUserQuestionInput = serde_json::from_str(&json).unwrap();
        assert!(parsed.answers.is_some());
        assert_eq!(
            parsed.answers.as_ref().unwrap().get("Library"),
            Some(&"tokio".to_string())
        );
    }

    #[test]
    fn test_ask_user_question_output_serde() {
        let mut answers = HashMap::new();
        answers.insert("Library".to_string(), "tokio".to_string());
        answers.insert("Framework".to_string(), "axum".to_string());

        let output = AskUserQuestionOutput { answers };

        let json = serde_json::to_string(&output).unwrap();
        let parsed: AskUserQuestionOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.answers.len(), 2);
        assert_eq!(parsed.answers.get("Library"), Some(&"tokio".to_string()));
        assert_eq!(parsed.answers.get("Framework"), Some(&"axum".to_string()));
    }

    #[test]
    fn test_ask_user_question_from_json_value() {
        // Simulate parsing from a ToolUse input
        let json_value = serde_json::json!({
            "questions": [{
                "question": "Which database?",
                "header": "Database",
                "options": [
                    {"label": "PostgreSQL", "description": "Relational DB"},
                    {"label": "MongoDB", "description": "Document DB"}
                ],
                "multiSelect": false
            }]
        });

        let input: AskUserQuestionInput = serde_json::from_value(json_value).unwrap();
        assert_eq!(input.questions.len(), 1);
        assert_eq!(input.questions[0].header, "Database");
        assert_eq!(input.questions[0].options.len(), 2);
    }
}

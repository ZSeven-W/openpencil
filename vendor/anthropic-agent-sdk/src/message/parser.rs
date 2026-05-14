//! Message parser for Claude Code SDK responses

use crate::error::{ClaudeError, Result};
use crate::types::Message;

/// Parse a JSON value into a typed Message
///
/// # Arguments
/// * `data` - Raw JSON value from CLI output
///
/// # Returns
/// Parsed Message object or error
///
/// # Errors
/// Returns `ClaudeError::MessageParse` if the JSON cannot be parsed into a valid Message
///
/// # Security Note
/// For additional security against deeply nested JSON attacks, consider using
/// `serde_json::Deserializer::with_recursion_limit()` to set explicit depth limits.
/// This is not currently implemented to avoid additional complexity, but the current
/// timeout mechanism on read operations provides some protection against excessive
/// parsing time.
pub fn parse_message(data: serde_json::Value) -> Result<Message> {
    // Debug log for result messages with structured_output
    if data.get("type") == Some(&serde_json::json!("result")) {
        tracing::debug!(
            "Parsing result message, structured_output in JSON: {:?}",
            data.get("structured_output")
        );
    }

    serde_json::from_value(data.clone()).map_err(|e| {
        ClaudeError::message_parse(format!("Failed to parse message: {e}"), Some(data))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_user_message() {
        let data = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": "Hello, Claude!"
            }
        });

        let result = parse_message(data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_message() {
        let data = json!({
            "type": "invalid_type",
            "data": "some data"
        });

        let result = parse_message(data);
        assert!(result.is_err());
    }
}

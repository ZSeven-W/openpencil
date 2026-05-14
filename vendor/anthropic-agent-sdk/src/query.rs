//! Simple query function for one-shot interactions

use futures::Stream;

use crate::Transport;
use crate::error::Result;
use crate::message::parse_message;
use crate::transport::{PromptInput, SubprocessTransport};
use crate::types::{ClaudeAgentOptions, Message};

/// One-shot query function.
///
/// For stateless queries without bidirectional communication.
/// For multi-turn conversations, use [`crate::client::ClaudeSDKClient`] instead.
///
/// # Key differences from `ClaudeSDKClient`
/// - **Unidirectional**: Send all messages upfront, receive all responses
/// - **Stateless**: Each query is independent, no conversation state
/// - **Simple**: Fire-and-forget style, no connection management
/// - **No interrupts**: Cannot interrupt or send follow-up messages
///
/// # When to use `query()`
/// - Simple one-off questions ("What is 2+2?")
/// - Batch processing of independent prompts
/// - Code generation or analysis tasks
/// - Automated scripts and CI/CD pipelines
/// - When you know all inputs upfront
///
/// # When to use `ClaudeSDKClient`
/// - Interactive conversations with follow-ups
/// - Chat applications or REPL-like interfaces
/// - When you need to send messages based on responses
/// - When you need interrupt capabilities
/// - Long-running sessions with state
///
/// # Arguments
/// * `prompt` - The prompt to send to Claude (string)
/// * `options` - Optional configuration (defaults to `ClaudeAgentOptions::default()` if None)
///
/// # Returns
/// A stream of Messages from the conversation
///
/// # Errors
/// Returns error if:
/// - Claude Code CLI is not found
/// - Connection fails
/// - Process fails
/// - JSON parsing fails
///
/// # Examples
///
/// Simple query:
/// ```no_run
/// use anthropic_agent_sdk::query;
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let stream = query("What is the capital of France?", None).await?;
///     let mut stream = Box::pin(stream);
///
///     while let Some(message) = stream.next().await {
///         println!("{:?}", message?);
///     }
///     Ok(())
/// }
/// ```
///
/// With options:
/// ```no_run
/// use anthropic_agent_sdk::{query, ClaudeAgentOptions};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::builder()
///         .system_prompt("You are a helpful coding assistant")
///         .max_turns(1)
///         .build();
///
///     let stream = query("Write a hello world in Python", Some(options)).await?;
///     let mut stream = Box::pin(stream);
///
///     while let Some(message) = stream.next().await {
///         println!("{:?}", message?);
///     }
///     Ok(())
/// }
/// ```
#[must_use = "query returns a stream that must be consumed to receive messages"]
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeAgentOptions>,
) -> Result<impl Stream<Item = Result<Message>>> {
    let options = options.unwrap_or_default();
    let prompt_input = PromptInput::from(prompt.into());

    let mut transport = SubprocessTransport::new(prompt_input, options, None)?;
    transport.connect().await?;

    // Get message receiver from transport
    let mut msg_receiver = transport.read_messages();

    // Create stream that parses messages
    // We need to move transport into the stream to keep it alive
    let message_stream = async_stream::stream! {
        while let Some(result) = msg_receiver.recv().await {
            match result {
                Ok(value) => {
                    match parse_message(value) {
                        Ok(msg) => yield Ok(msg),
                        Err(e) => yield Err(e),
                    }
                }
                Err(e) => yield Err(e),
            }
        }
        // Keep transport alive until stream is done
        drop(transport);
    };

    Ok(message_stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_simple_query() {
        let stream = query("What is 2+2?", None).await.unwrap();
        let mut stream = Box::pin(stream);

        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => println!("Message: {msg:?}"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }
}

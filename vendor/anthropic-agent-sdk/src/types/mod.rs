//! Type definitions for Claude Agent SDK
//!
//! This module contains all the type definitions used throughout the SDK,
//! including identifiers for type safety, message types, option types, and more.

// Module declarations
pub mod hooks;
pub mod identifiers;
pub mod introspection;
pub mod mcp;
pub mod messages;
pub mod options;
pub mod permissions;
pub mod usage;

// Re-export all public types for backward compatibility
pub use hooks::{
    // Base input
    BaseHookInput,
    // Event-specific inputs
    CompactTrigger,
    // Core hook types
    HookContext,
    HookDecision,
    HookEvent,
    HookInput,
    HookMatcher,
    HookOutput,
    NotificationHookInput,
    PermissionRequestHookInput,
    PostToolUseFailureHookInput,
    PostToolUseHookInput,
    PreCompactHookInput,
    PreToolUseHookInput,
    SessionEndHookInput,
    SessionEndReason,
    SessionStartHookInput,
    SessionStartSource,
    StopHookInput,
    SubagentStartHookInput,
    SubagentStopHookInput,
    UserPromptSubmitHookInput,
};
pub use identifiers::{RequestId, SessionId, ToolName};
pub use introspection::{
    AccountInfo, McpServerStatus, ModelInfo, ModelUsage, SDKPermissionDenial, SessionInfo,
    SlashCommand, ToolInfo,
};
pub use mcp::{
    McpHttpServerConfig, McpServerConfig, McpServers, McpSseServerConfig, McpStdioServerConfig,
    SdkMcpServerConfig,
};
pub use messages::{
    AskUserQuestionInput, AskUserQuestionOutput, AssistantMessageContent, ContentBlock,
    ContentValue, Message, QuestionOption, QuestionSpec, UserContent, UserMessageContent,
};
pub use options::{
    AgentDefinition, ClaudeAgentOptions, ClaudeAgentOptionsBuilder, NetworkSandboxSettings,
    OutputFormat, SandboxIgnoreViolations, SandboxSettings, SdkBeta, SdkPluginConfig,
    StderrCallback, SystemPrompt, SystemPromptPreset, ToolsConfig, ToolsPreset,
};
pub use permissions::{
    CanUseToolCallback, PermissionBehavior, PermissionMode, PermissionRequest, PermissionResult,
    PermissionResultAllow, PermissionResultDeny, PermissionRuleValue, PermissionUpdate,
    PermissionUpdateDestination, SettingSource, ToolPermissionContext,
};
pub use usage::{UsageData, UsageLimit};

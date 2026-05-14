//! Trait-based callback definitions for hooks and permissions.
//!
//! This module provides idiomatic Rust traits for implementing callbacks.
//! Users can implement these traits on their own types, or use closures
//! via the provided blanket implementations.
//!
//! # Example: Implementing `HookCallback`
//!
//! ```no_run
//! use anthropic_agent_sdk::callbacks::HookCallback;
//! use anthropic_agent_sdk::types::{HookOutput, HookContext};
//! use anthropic_agent_sdk::Result;
//! use async_trait::async_trait;
//!
//! struct LoggingHook;
//!
//! #[async_trait]
//! impl HookCallback for LoggingHook {
//!     async fn call(
//!         &self,
//!         input: serde_json::Value,
//!         tool_use_id: Option<String>,
//!         _context: HookContext,
//!     ) -> Result<HookOutput> {
//!         println!("Tool called: {:?}", tool_use_id);
//!         Ok(HookOutput::default())
//!     }
//! }
//! ```
//!
//! # Example: Implementing `PermissionCallback`
//!
//! ```no_run
//! use anthropic_agent_sdk::callbacks::PermissionCallback;
//! use anthropic_agent_sdk::types::{PermissionResult, PermissionResultAllow, ToolPermissionContext};
//! use anthropic_agent_sdk::Result;
//! use async_trait::async_trait;
//!
//! struct AllowReadOnly;
//!
//! #[async_trait]
//! impl PermissionCallback for AllowReadOnly {
//!     async fn call(
//!         &self,
//!         tool_name: String,
//!         _input: serde_json::Value,
//!         _context: ToolPermissionContext,
//!     ) -> Result<PermissionResult> {
//!         if tool_name == "Read" || tool_name == "Glob" {
//!             Ok(PermissionResult::Allow(PermissionResultAllow {
//!                 updated_input: None,
//!                 updated_permissions: None,
//!             }))
//!         } else {
//!             Ok(PermissionResult::Deny(anthropic_agent_sdk::types::PermissionResultDeny {
//!                 message: "Only read operations allowed".to_string(),
//!                 interrupt: false,
//!             }))
//!         }
//!     }
//! }
//! ```

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{HookContext, HookOutput, PermissionResult, ToolPermissionContext};

// ============================================================================
// Hook Callback Trait
// ============================================================================

/// Trait for hook callbacks.
///
/// Implement this trait to create custom hook handlers that can intercept
/// tool usage and other events in the Claude agent loop.
///
/// # Examples
///
/// ## Using a struct
///
/// ```no_run
/// use anthropic_agent_sdk::callbacks::HookCallback;
/// use anthropic_agent_sdk::types::{HookOutput, HookContext};
/// use anthropic_agent_sdk::Result;
/// use async_trait::async_trait;
///
/// struct MyHook {
///     log_prefix: String,
/// }
///
/// #[async_trait]
/// impl HookCallback for MyHook {
///     async fn call(
///         &self,
///         input: serde_json::Value,
///         tool_use_id: Option<String>,
///         _context: HookContext,
///     ) -> Result<HookOutput> {
///         println!("{}: {:?}", self.log_prefix, tool_use_id);
///         Ok(HookOutput::default())
///     }
/// }
/// ```
#[async_trait]
pub trait HookCallback: Send + Sync {
    /// Called when a hook event occurs.
    ///
    /// # Arguments
    ///
    /// * `input` - The hook input data (tool input for `PreToolUse`, result for `PostToolUse`, etc.)
    /// * `tool_use_id` - Optional tool use ID for tool-related hooks
    /// * `context` - Hook execution context
    ///
    /// # Returns
    ///
    /// A `HookOutput` that can optionally block the action or add system messages.
    async fn call(
        &self,
        input: serde_json::Value,
        tool_use_id: Option<String>,
        context: HookContext,
    ) -> Result<HookOutput>;
}

// Blanket implementation for boxed trait objects
#[async_trait]
impl HookCallback for Box<dyn HookCallback> {
    async fn call(
        &self,
        input: serde_json::Value,
        tool_use_id: Option<String>,
        context: HookContext,
    ) -> Result<HookOutput> {
        (**self).call(input, tool_use_id, context).await
    }
}

// ============================================================================
// Permission Callback Trait
// ============================================================================

/// Trait for permission callbacks.
///
/// Implement this trait to create custom permission handlers that control
/// which tools Claude is allowed to use.
///
/// # Examples
///
/// ## Using a struct
///
/// ```no_run
/// use anthropic_agent_sdk::callbacks::PermissionCallback;
/// use anthropic_agent_sdk::types::{PermissionResult, PermissionResultAllow, PermissionResultDeny, ToolPermissionContext};
/// use anthropic_agent_sdk::Result;
/// use async_trait::async_trait;
///
/// struct SafeToolsOnly {
///     allowed_tools: Vec<String>,
/// }
///
/// #[async_trait]
/// impl PermissionCallback for SafeToolsOnly {
///     async fn call(
///         &self,
///         tool_name: String,
///         _input: serde_json::Value,
///         _context: ToolPermissionContext,
///     ) -> Result<PermissionResult> {
///         if self.allowed_tools.contains(&tool_name) {
///             Ok(PermissionResult::Allow(PermissionResultAllow {
///                 updated_input: None,
///                 updated_permissions: None,
///             }))
///         } else {
///             Ok(PermissionResult::Deny(PermissionResultDeny {
///                 message: format!("Tool '{}' not allowed", tool_name),
///                 interrupt: false,
///             }))
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait PermissionCallback: Send + Sync {
    /// Called when Claude requests permission to use a tool.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool being requested
    /// * `input` - The tool input parameters
    /// * `context` - Permission context with suggestions from CLI
    ///
    /// # Returns
    ///
    /// A `PermissionResult` that either allows or denies the tool use.
    async fn call(
        &self,
        tool_name: String,
        input: serde_json::Value,
        context: ToolPermissionContext,
    ) -> Result<PermissionResult>;
}

// Blanket implementation for boxed trait objects
#[async_trait]
impl PermissionCallback for Box<dyn PermissionCallback> {
    async fn call(
        &self,
        tool_name: String,
        input: serde_json::Value,
        context: ToolPermissionContext,
    ) -> Result<PermissionResult> {
        (**self).call(tool_name, input, context).await
    }
}

// ============================================================================
// Arc wrapper implementations for shared callbacks
// ============================================================================

use std::sync::Arc;

#[async_trait]
impl<T: HookCallback + ?Sized> HookCallback for Arc<T> {
    async fn call(
        &self,
        input: serde_json::Value,
        tool_use_id: Option<String>,
        context: HookContext,
    ) -> Result<HookOutput> {
        (**self).call(input, tool_use_id, context).await
    }
}

#[async_trait]
impl<T: PermissionCallback + ?Sized> PermissionCallback for Arc<T> {
    async fn call(
        &self,
        tool_name: String,
        input: serde_json::Value,
        context: ToolPermissionContext,
    ) -> Result<PermissionResult> {
        (**self).call(tool_name, input, context).await
    }
}

// ============================================================================
// Type aliases for backward compatibility and convenience
// ============================================================================

/// Type alias for a shared hook callback.
pub type SharedHookCallback = Arc<dyn HookCallback>;

/// Type alias for a shared permission callback.
pub type SharedPermissionCallback = Arc<dyn PermissionCallback>;

// ============================================================================
// Closure-based callback wrappers
// ============================================================================

/// Wrapper to convert a closure into a `HookCallback`.
///
/// This allows using closures where trait objects are expected.
///
/// # Example
///
/// ```no_run
/// use anthropic_agent_sdk::callbacks::{FnHookCallback, HookCallback};
/// use anthropic_agent_sdk::types::{HookOutput, HookContext};
/// use std::sync::Arc;
///
/// let callback = FnHookCallback::new(|input, tool_id, ctx| {
///     Box::pin(async move {
///         // ctx provides session_id, cwd, and cancellation_token
///         println!("Hook for tool: {:?}, session: {:?}", tool_id, ctx.session_id);
///         Ok(HookOutput::default())
///     })
/// });
///
/// let shared: Arc<dyn HookCallback> = Arc::new(callback);
/// ```
pub struct FnHookCallback<F>
where
    F: Fn(
            serde_json::Value,
            Option<String>,
            HookContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<HookOutput>> + Send>>
        + Send
        + Sync,
{
    func: F,
}

impl<F> FnHookCallback<F>
where
    F: Fn(
            serde_json::Value,
            Option<String>,
            HookContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<HookOutput>> + Send>>
        + Send
        + Sync,
{
    /// Create a new function-based hook callback.
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

#[async_trait]
impl<F> HookCallback for FnHookCallback<F>
where
    F: Fn(
            serde_json::Value,
            Option<String>,
            HookContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<HookOutput>> + Send>>
        + Send
        + Sync,
{
    async fn call(
        &self,
        input: serde_json::Value,
        tool_use_id: Option<String>,
        context: HookContext,
    ) -> Result<HookOutput> {
        (self.func)(input, tool_use_id, context).await
    }
}

/// Wrapper to convert a closure into a `PermissionCallback`.
///
/// # Example
///
/// ```no_run
/// use anthropic_agent_sdk::callbacks::{FnPermissionCallback, PermissionCallback};
/// use anthropic_agent_sdk::types::{PermissionResult, PermissionResultAllow, ToolPermissionContext};
/// use std::sync::Arc;
///
/// let callback = FnPermissionCallback::new(|tool_name, input, ctx| {
///     Box::pin(async move {
///         // ctx.suggestions has CLI permission suggestions
///         println!("Permission for: {}, suggestions: {:?}", tool_name, ctx.suggestions);
///         Ok(PermissionResult::Allow(PermissionResultAllow {
///             updated_input: None,
///             updated_permissions: None,
///         }))
///     })
/// });
///
/// let shared: Arc<dyn PermissionCallback> = Arc::new(callback);
/// ```
pub struct FnPermissionCallback<F>
where
    F: Fn(
            String,
            serde_json::Value,
            ToolPermissionContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PermissionResult>> + Send>>
        + Send
        + Sync,
{
    func: F,
}

impl<F> FnPermissionCallback<F>
where
    F: Fn(
            String,
            serde_json::Value,
            ToolPermissionContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PermissionResult>> + Send>>
        + Send
        + Sync,
{
    /// Create a new function-based permission callback.
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

#[async_trait]
impl<F> PermissionCallback for FnPermissionCallback<F>
where
    F: Fn(
            String,
            serde_json::Value,
            ToolPermissionContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PermissionResult>> + Send>>
        + Send
        + Sync,
{
    async fn call(
        &self,
        tool_name: String,
        input: serde_json::Value,
        context: ToolPermissionContext,
    ) -> Result<PermissionResult> {
        (self.func)(tool_name, input, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PermissionResultAllow, PermissionResultDeny};

    struct TestHook;

    #[async_trait]
    impl HookCallback for TestHook {
        async fn call(
            &self,
            _input: serde_json::Value,
            _tool_use_id: Option<String>,
            _context: HookContext,
        ) -> Result<HookOutput> {
            Ok(HookOutput::default())
        }
    }

    struct TestPermission {
        allow_all: bool,
    }

    #[async_trait]
    impl PermissionCallback for TestPermission {
        async fn call(
            &self,
            tool_name: String,
            _input: serde_json::Value,
            _context: ToolPermissionContext,
        ) -> Result<PermissionResult> {
            if self.allow_all {
                Ok(PermissionResult::Allow(PermissionResultAllow {
                    updated_input: None,
                    updated_permissions: None,
                }))
            } else {
                Ok(PermissionResult::Deny(PermissionResultDeny {
                    message: format!("Denied: {tool_name}"),
                    interrupt: false,
                }))
            }
        }
    }

    #[tokio::test]
    async fn test_hook_callback_trait() {
        let hook = TestHook;
        let result = hook
            .call(serde_json::json!({}), None, HookContext::default())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_permission_callback_trait() {
        let perm = TestPermission { allow_all: true };
        let result = perm
            .call(
                "Read".to_string(),
                serde_json::json!({}),
                ToolPermissionContext::new(vec![]),
            )
            .await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PermissionResult::Allow(_)));
    }

    #[tokio::test]
    async fn test_arc_wrapped_callback() {
        let hook: Arc<dyn HookCallback> = Arc::new(TestHook);
        let result = hook
            .call(serde_json::json!({}), None, HookContext::default())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fn_hook_callback() {
        let callback = FnHookCallback::new(|_input, _tool_id, ctx| {
            Box::pin(async move {
                // Verify context fields are accessible
                let _ = ctx.session_id;
                let _ = ctx.cwd;
                let _ = ctx.is_cancelled();
                Ok(HookOutput::default())
            })
        });

        let result = callback
            .call(serde_json::json!({}), None, HookContext::default())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fn_permission_callback() {
        let callback = FnPermissionCallback::new(|_tool, _input, ctx| {
            Box::pin(async move {
                // Verify context has suggestions and cancellation fields
                let _ = ctx.suggestions;
                let _ = ctx.is_cancelled();
                Ok(PermissionResult::Allow(PermissionResultAllow {
                    updated_input: None,
                    updated_permissions: None,
                }))
            })
        });

        let result = callback
            .call(
                "Test".to_string(),
                serde_json::json!({}),
                ToolPermissionContext::new(vec![]),
            )
            .await;
        assert!(result.is_ok());
    }
}

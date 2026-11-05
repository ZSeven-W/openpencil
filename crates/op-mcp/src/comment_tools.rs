//! Canvas comment tools — how an agent finds review feedback, answers
//! it, and moves threads through `pending → addressed → archived`.
//!
//! Read tools skip archived threads unless history is asked for
//! (`include_archived` or an explicit `status`), so an agent polling for
//! work only sees what still needs attention. Write tools pre-validate
//! against a detached [`CommentSnapshot`] and hand the host an
//! [`EditorCommand::Comment`], the same command the GUI applies.

use std::collections::BTreeMap;

use op_editor_core::comments::{
    CanvasComment, CommentAnchor, CommentAuthor, CommentAuthorKind, CommentCommand, CommentError,
    CommentLookup, CommentSnapshot, CommentStatus,
};
use op_editor_core::{EditorCommand, EditorState};
use serde_json::{json, Value};

use super::{McpTool, ToolErrorCode, ToolOutcome};

/// Author name used when an agent does not introduce itself.
pub const DEFAULT_AGENT_AUTHOR: &str = "Agent";

/// `list_comments` / `get_comment` / `add_comment` / `reply_comment` /
/// `set_comment_status` share one snapshot shape; `kind` selects the tool.
pub struct CommentTool {
    kind: CommentToolKind,
    snapshot: CommentSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommentToolKind {
    List,
    Get,
    Add,
    Reply,
    SetStatus,
}

pub fn list_comments_snapshot(state: &EditorState) -> CommentTool {
    CommentTool::new(CommentToolKind::List, state)
}

pub fn get_comment_snapshot(state: &EditorState) -> CommentTool {
    CommentTool::new(CommentToolKind::Get, state)
}

pub fn add_comment_snapshot(state: &EditorState) -> CommentTool {
    CommentTool::new(CommentToolKind::Add, state)
}

pub fn reply_comment_snapshot(state: &EditorState) -> CommentTool {
    CommentTool::new(CommentToolKind::Reply, state)
}

pub fn set_comment_status_snapshot(state: &EditorState) -> CommentTool {
    CommentTool::new(CommentToolKind::SetStatus, state)
}

impl CommentTool {
    fn new(kind: CommentToolKind, state: &EditorState) -> Self {
        Self {
            kind,
            snapshot: CommentSnapshot::capture(state),
        }
    }
}

impl McpTool for CommentTool {
    fn name(&self) -> &str {
        match self.kind {
            CommentToolKind::List => "list_comments",
            CommentToolKind::Get => "get_comment",
            CommentToolKind::Add => "add_comment",
            CommentToolKind::Reply => "reply_comment",
            CommentToolKind::SetStatus => "set_comment_status",
        }
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let result = match self.kind {
            CommentToolKind::List => self.list(args),
            CommentToolKind::Get => self.get(args),
            CommentToolKind::Add => self.add(args),
            CommentToolKind::Reply => self.reply(args),
            CommentToolKind::SetStatus => self.set_status(args),
        };
        result.unwrap_or_else(|e| e.into_outcome())
    }
}

/// A comment-tool failure, mapped onto the MCP error codes.
#[derive(Debug, Clone, PartialEq)]
pub enum CommentToolError {
    Missing(&'static str),
    Invalid { key: &'static str, raw: String },
    Rejected(CommentError),
}

impl std::fmt::Display for CommentToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(key) => write!(f, "{key} is required"),
            Self::Invalid { key, raw } => write!(f, "invalid {key}: {raw:?}"),
            Self::Rejected(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CommentToolError {}

impl From<CommentError> for CommentToolError {
    fn from(e: CommentError) -> Self {
        Self::Rejected(e)
    }
}

impl CommentToolError {
    fn into_outcome(self) -> ToolOutcome {
        let code = match &self {
            Self::Missing(_) => ToolErrorCode::MissingArgument,
            Self::Invalid { .. } => ToolErrorCode::InvalidArgument,
            Self::Rejected(_) => ToolErrorCode::ToolFailed,
        };
        ToolOutcome::Err(code, self.to_string())
    }
}

type ToolResult = Result<ToolOutcome, CommentToolError>;

impl CommentTool {
    fn list(&self, args: &BTreeMap<String, String>) -> ToolResult {
        let statuses = status_filter(args)?;
        let node_id = arg(args, &["node_id", "nodeId"]);
        let page_id = arg(args, &["page_id", "pageId"]);
        let page_index = match page_id {
            Some(page_id) => Some(self.snapshot.comment_page_by_id(page_id).ok_or(
                CommentToolError::Rejected(CommentError::UnknownPage(page_id.to_owned())),
            )?),
            None => None,
        };
        let mut hidden_archived = 0usize;
        let comments: Vec<Value> = self
            .snapshot
            .comments
            .iter()
            .filter(|c| node_id.is_none_or(|id| c.anchor.node_id.as_deref() == Some(id)))
            .filter(|c| page_index.is_none_or(|p| self.snapshot.comment_page_index(c) == Some(p)))
            .filter(|c| {
                let keep = statuses.contains(&c.status);
                if !keep && c.status == CommentStatus::Archived {
                    hidden_archived += 1;
                }
                keep
            })
            .map(|c| self.comment_json(c))
            .collect();
        Ok(ToolOutcome::OkJson(
            json!({
                "count": comments.len(),
                "hiddenArchivedCount": hidden_archived,
                "comments": comments,
            })
            .to_string(),
        ))
    }

    fn get(&self, args: &BTreeMap<String, String>) -> ToolResult {
        let id = required(args, &["comment_id", "commentId", "id"], "comment_id")?;
        let comment = self
            .snapshot
            .find_comment(id)
            .ok_or_else(|| CommentError::NotFound(id.to_owned()))?;
        Ok(ToolOutcome::OkJson(
            json!({ "comment": self.comment_json(comment) }).to_string(),
        ))
    }

    fn add(&self, args: &BTreeMap<String, String>) -> ToolResult {
        let text = required(args, &["text", "message"], "text")?.to_owned();
        let anchor = CommentAnchor {
            page_id: arg(args, &["page_id", "pageId"]).map(str::to_owned),
            node_id: arg(args, &["node_id", "nodeId"]).map(str::to_owned),
            x: number(args, "x")?,
            y: number(args, "y")?,
        };
        let at_ms = now_ms();
        let id = self.snapshot.next_comment_id(at_ms);
        let cmd = CommentCommand::Add {
            id: id.clone(),
            anchor: anchor.clone(),
            author: agent_author(args),
            text: text.clone(),
            at_ms,
        };
        self.snapshot.check_comment_command(&cmd)?;
        let preview = CanvasComment {
            id,
            status: CommentStatus::Pending,
            anchor,
            messages: vec![op_editor_core::comments::CommentMessage {
                author: agent_author(args),
                text: text.trim().to_owned(),
                created_at: at_ms,
            }],
            updated_at: Some(at_ms),
        };
        Ok(self.write(cmd, &preview))
    }

    fn reply(&self, args: &BTreeMap<String, String>) -> ToolResult {
        let id = required(args, &["comment_id", "commentId", "id"], "comment_id")?;
        let text = required(args, &["text", "message"], "text")?;
        let at_ms = now_ms();
        let cmd = CommentCommand::Reply {
            comment_id: id.to_owned(),
            author: agent_author(args),
            text: text.to_owned(),
            at_ms,
        };
        self.snapshot.check_comment_command(&cmd)?;
        let mut preview = self
            .snapshot
            .find_comment(id)
            .cloned()
            .ok_or_else(|| CommentError::NotFound(id.to_owned()))?;
        preview
            .messages
            .push(op_editor_core::comments::CommentMessage {
                author: agent_author(args),
                text: text.trim().to_owned(),
                created_at: at_ms,
            });
        preview.updated_at = Some(at_ms);
        Ok(self.write(cmd, &preview))
    }

    fn set_status(&self, args: &BTreeMap<String, String>) -> ToolResult {
        let id = required(args, &["comment_id", "commentId", "id"], "comment_id")?;
        let raw = required(args, &["status"], "status")?;
        let status = parse_status(raw)?;
        let at_ms = now_ms();
        let cmd = CommentCommand::SetStatus {
            comment_id: id.to_owned(),
            status,
            at_ms,
        };
        self.snapshot.check_comment_command(&cmd)?;
        let mut preview = self
            .snapshot
            .find_comment(id)
            .cloned()
            .ok_or_else(|| CommentError::NotFound(id.to_owned()))?;
        preview.status = status;
        preview.updated_at = Some(at_ms);
        Ok(self.write(cmd, &preview))
    }

    fn write(&self, cmd: CommentCommand, preview: &CanvasComment) -> ToolOutcome {
        ToolOutcome::OkJsonWithCommand(
            json!({ "wrote": true, "comment": self.comment_json(preview) }).to_string(),
            EditorCommand::Comment(cmd),
        )
    }

    /// The stored thread plus the page it resolves to, so an agent can go
    /// straight to the right page.
    fn comment_json(&self, comment: &CanvasComment) -> Value {
        let mut value = serde_json::to_value(comment).unwrap_or(Value::Null);
        if let Value::Object(map) = &mut value {
            let page_id = self
                .snapshot
                .comment_page_index(comment)
                .and_then(|i| self.snapshot.page_ids.get(i).cloned());
            map.insert("resolvedPageId".into(), json!(page_id));
            let orphaned = comment
                .anchor
                .node_id
                .as_deref()
                .is_some_and(|id| self.snapshot.comment_page_of_node(id).is_none());
            map.insert("orphaned".into(), json!(orphaned));
        }
        value
    }
}

fn arg<'a>(args: &'a BTreeMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .filter_map(|k| args.get(*k))
        .map(|v| v.trim())
        .find(|v| !v.is_empty())
}

fn required<'a>(
    args: &'a BTreeMap<String, String>,
    keys: &[&str],
    name: &'static str,
) -> Result<&'a str, CommentToolError> {
    arg(args, keys).ok_or(CommentToolError::Missing(name))
}

fn number(
    args: &BTreeMap<String, String>,
    key: &'static str,
) -> Result<Option<f64>, CommentToolError> {
    match arg(args, &[key]) {
        None => Ok(None),
        Some(raw) => raw
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(Some)
            .ok_or_else(|| CommentToolError::Invalid {
                key,
                raw: raw.to_owned(),
            }),
    }
}

fn parse_status(raw: &str) -> Result<CommentStatus, CommentToolError> {
    CommentStatus::parse(raw).ok_or_else(|| CommentToolError::Invalid {
        key: "status",
        raw: raw.to_owned(),
    })
}

/// Which statuses `list_comments` returns: an explicit `status` list
/// (comma-separated, or `all`) wins; otherwise pending + addressed, plus
/// archived when `include_archived` is true.
fn status_filter(args: &BTreeMap<String, String>) -> Result<Vec<CommentStatus>, CommentToolError> {
    use CommentStatus::*;
    if let Some(raw) = arg(args, &["status"]) {
        if raw.eq_ignore_ascii_case("all") {
            return Ok(vec![Pending, Addressed, Archived]);
        }
        return raw.split(',').map(parse_status).collect();
    }
    let include_archived = match arg(args, &["include_archived", "includeArchived"]) {
        None => false,
        Some("true") => true,
        Some("false") => false,
        Some(raw) => {
            return Err(CommentToolError::Invalid {
                key: "include_archived",
                raw: raw.to_owned(),
            })
        }
    };
    Ok(if include_archived {
        vec![Pending, Addressed, Archived]
    } else {
        vec![Pending, Addressed]
    })
}

fn agent_author(args: &BTreeMap<String, String>) -> CommentAuthor {
    CommentAuthor {
        kind: CommentAuthorKind::Agent,
        name: arg(args, &["author", "author_name", "authorName"])
            .unwrap_or(DEFAULT_AGENT_AUTHOR)
            .to_owned(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The browser bundle has no `SystemTime`; MCP is served by the native
/// daemon, so this path only exists to keep the wasm build compiling.
#[cfg(target_arch = "wasm32")]
fn now_ms() -> u64 {
    0
}

#[cfg(test)]
#[path = "comment_tools_tests.rs"]
mod tests;

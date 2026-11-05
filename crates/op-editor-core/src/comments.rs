//! Canvas comment threads — the editor side of `PenDocument.comments`.
//!
//! GUI hosts and MCP tools both mutate threads through
//! [`EditorCommand::Comment`](crate::EditorCommand::Comment), so the
//! validation rules (unique ids, non-blank text, lifecycle transitions)
//! live in one place. Producers supply the thread id and the timestamp:
//! the wasm32 bundle has no wall clock, and an MCP tool must report the
//! new id before the host applies the command.
//!
//! Comment edits ride the normal undo history like any other document
//! edit. Excluding them would let undoing an older node edit restore a
//! snapshot that predates a newer comment and silently drop it.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers::find_node;
pub use jian_ops_schema::comment::{
    CanvasComment, CommentAnchor, CommentAuthor, CommentAuthorKind, CommentMessage, CommentStatus,
};
use jian_ops_schema::node::PenNode;
use std::collections::HashMap;

/// Longest accepted message, in characters. Keeps a runaway agent from
/// writing megabytes into the document.
pub const MAX_COMMENT_TEXT_CHARS: usize = 10_000;

/// One comment-thread mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum CommentCommand {
    /// Open a new thread with its first message.
    Add {
        id: String,
        anchor: CommentAnchor,
        author: CommentAuthor,
        text: String,
        at_ms: u64,
    },
    /// Append a reply to an existing thread.
    Reply {
        comment_id: String,
        author: CommentAuthor,
        text: String,
        at_ms: u64,
    },
    /// Move a thread along its lifecycle (see
    /// [`CommentStatus::can_transition_to`]).
    SetStatus {
        comment_id: String,
        status: CommentStatus,
        at_ms: u64,
    },
    /// Remove a thread and its whole history.
    Delete { comment_id: String },
}

/// Why a comment command was refused. The MCP tools pre-validate with
/// [`EditorState::check_comment_command`] so the caller gets this reason
/// instead of a bare "not applied".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentError {
    EmptyId,
    DuplicateId(String),
    NotFound(String),
    EmptyText,
    TextTooLong {
        chars: usize,
    },
    /// The anchor names neither a node nor a point.
    EmptyAnchor,
    UnknownNode(String),
    UnknownPage(String),
    /// A node anchor whose node is not on the anchor's page.
    NodeNotOnPage {
        node_id: String,
        page_id: String,
    },
    InvalidTransition {
        from: CommentStatus,
        to: CommentStatus,
    },
}

impl std::fmt::Display for CommentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => write!(f, "comment id must not be empty"),
            Self::DuplicateId(id) => write!(f, "comment id already exists: {id}"),
            Self::NotFound(id) => write!(f, "comment not found: {id}"),
            Self::EmptyText => write!(f, "comment text must not be empty"),
            Self::TextTooLong { chars } => write!(
                f,
                "comment text is {chars} characters; the limit is {MAX_COMMENT_TEXT_CHARS}"
            ),
            Self::EmptyAnchor => write!(f, "a comment needs a nodeId or both x and y"),
            Self::UnknownNode(id) => write!(f, "node not found: {id}"),
            Self::UnknownPage(id) => write!(f, "page not found: {id}"),
            Self::NodeNotOnPage { node_id, page_id } => {
                write!(f, "node {node_id} is not on page {page_id}")
            }
            Self::InvalidTransition { from, to } => write!(
                f,
                "cannot change a comment from {} to {}",
                from.as_str(),
                to.as_str()
            ),
        }
    }
}

impl std::error::Error for CommentError {}

/// The read-side a comment command is validated against. Implemented by
/// the live [`EditorState`] and by the detached [`CommentSnapshot`] that
/// MCP tools capture, so both run exactly the same rules.
pub trait CommentLookup {
    /// Every thread in document order (oldest first).
    fn comment_list(&self) -> &[CanvasComment];
    /// Index of the page with this id.
    fn comment_page_by_id(&self, page_id: &str) -> Option<usize>;
    /// Index of the page holding this node.
    fn comment_page_of_node(&self, node_id: &str) -> Option<usize>;

    /// Look up one thread by id.
    fn find_comment(&self, id: &str) -> Option<&CanvasComment> {
        self.comment_list().iter().find(|c| c.id == id)
    }

    /// A fresh thread id derived from `now_ms`, suffixed when two threads
    /// are opened within the same millisecond.
    fn next_comment_id(&self, now_ms: u64) -> String {
        let base = format!("cm_{now_ms:x}");
        if self.find_comment(&base).is_none() {
            return base;
        }
        (2u32..)
            .map(|n| format!("{base}_{n}"))
            .find(|id| self.find_comment(id).is_none())
            .unwrap_or(base)
    }

    /// Index of the page a thread is pinned to: the page holding its
    /// node, else its `pageId`, else the first page. `None` when the
    /// anchor names a page or node that no longer exists.
    fn comment_page_index(&self, comment: &CanvasComment) -> Option<usize> {
        if let Some(node_id) = comment.anchor.node_id.as_deref() {
            return self.comment_page_of_node(node_id);
        }
        match comment.anchor.page_id.as_deref() {
            Some(page_id) => self.comment_page_by_id(page_id),
            None => Some(0),
        }
    }

    /// Validate a comment command without mutating anything.
    fn check_comment_command(&self, cmd: &CommentCommand) -> Result<(), CommentError> {
        let existing = |id: &str| {
            self.find_comment(id)
                .ok_or_else(|| CommentError::NotFound(id.to_owned()))
        };
        match cmd {
            CommentCommand::Add {
                id, anchor, text, ..
            } => {
                if id.trim().is_empty() {
                    return Err(CommentError::EmptyId);
                }
                if self.find_comment(id).is_some() {
                    return Err(CommentError::DuplicateId(id.clone()));
                }
                check_text(text)?;
                check_anchor(self, anchor)
            }
            CommentCommand::Reply {
                comment_id, text, ..
            } => {
                existing(comment_id)?;
                check_text(text)
            }
            CommentCommand::SetStatus {
                comment_id, status, ..
            } => {
                let from = existing(comment_id)?.status;
                if from.can_transition_to(*status) {
                    Ok(())
                } else {
                    Err(CommentError::InvalidTransition { from, to: *status })
                }
            }
            CommentCommand::Delete { comment_id } => existing(comment_id).map(|_| ()),
        }
    }
}

impl CommentLookup for EditorState {
    fn comment_list(&self) -> &[CanvasComment] {
        self.comments()
    }

    fn comment_page_by_id(&self, page_id: &str) -> Option<usize> {
        match self.doc.pages.as_ref() {
            Some(pages) if !pages.is_empty() => pages.iter().position(|p| p.id == page_id),
            _ => None,
        }
    }

    fn comment_page_of_node(&self, node_id: &str) -> Option<usize> {
        let id = NodeId::new_opt(node_id)?;
        match self.doc.pages.as_ref() {
            Some(pages) if !pages.is_empty() => pages
                .iter()
                .position(|page| find_node(&page.children, &id).is_some()),
            _ => find_node(&self.doc.children, &id).map(|_| 0),
        }
    }
}

/// A detached copy of what comment validation needs — threads, page ids,
/// and a node → page index — so an MCP tool can validate and answer
/// without borrowing the editor state.
#[derive(Debug, Clone, Default)]
pub struct CommentSnapshot {
    pub comments: Vec<CanvasComment>,
    pub page_ids: Vec<String>,
    node_pages: HashMap<String, usize>,
}

impl CommentSnapshot {
    pub fn capture(state: &EditorState) -> Self {
        let mut node_pages = HashMap::new();
        let mut page_ids = Vec::new();
        match state.doc.pages.as_ref() {
            Some(pages) if !pages.is_empty() => {
                for (index, page) in pages.iter().enumerate() {
                    page_ids.push(page.id.clone());
                    index_nodes(&page.children, index, &mut node_pages);
                }
            }
            _ => index_nodes(&state.doc.children, 0, &mut node_pages),
        }
        Self {
            comments: state.comments().to_vec(),
            page_ids,
            node_pages,
        }
    }
}

impl CommentLookup for CommentSnapshot {
    fn comment_list(&self) -> &[CanvasComment] {
        &self.comments
    }

    fn comment_page_by_id(&self, page_id: &str) -> Option<usize> {
        self.page_ids.iter().position(|id| id == page_id)
    }

    fn comment_page_of_node(&self, node_id: &str) -> Option<usize> {
        self.node_pages.get(node_id).copied()
    }
}

fn index_nodes(nodes: &[PenNode], page: usize, out: &mut HashMap<String, usize>) {
    let mut stack: Vec<&PenNode> = nodes.iter().collect();
    while let Some(node) = stack.pop() {
        out.insert(node.id_str().to_owned(), page);
        if let Some(children) = node.children() {
            stack.extend(children.iter());
        }
    }
}

impl EditorState {
    /// Every thread in document order (oldest first).
    pub fn comments(&self) -> &[CanvasComment] {
        self.doc.comments.as_deref().unwrap_or(&[])
    }

    /// Look up one thread by id.
    pub fn comment(&self, id: &str) -> Option<&CanvasComment> {
        self.find_comment(id)
    }

    /// Apply a comment command; `false` (and no change) when it is invalid.
    pub(crate) fn apply_comment_command(&mut self, cmd: CommentCommand) -> bool {
        if self.check_comment_command(&cmd).is_err() {
            return false;
        }
        let comments = self.doc.comments.get_or_insert_with(Vec::new);
        match cmd {
            CommentCommand::Add {
                id,
                anchor,
                author,
                text,
                at_ms,
            } => comments.push(CanvasComment {
                id,
                status: CommentStatus::Pending,
                anchor,
                messages: vec![CommentMessage {
                    author,
                    text: text.trim().to_owned(),
                    created_at: at_ms,
                }],
                updated_at: Some(at_ms),
            }),
            CommentCommand::Reply {
                comment_id,
                author,
                text,
                at_ms,
            } => {
                if let Some(c) = comments.iter_mut().find(|c| c.id == comment_id) {
                    c.messages.push(CommentMessage {
                        author,
                        text: text.trim().to_owned(),
                        created_at: at_ms,
                    });
                    c.updated_at = Some(at_ms);
                }
            }
            CommentCommand::SetStatus {
                comment_id,
                status,
                at_ms,
            } => {
                if let Some(c) = comments.iter_mut().find(|c| c.id == comment_id) {
                    c.status = status;
                    c.updated_at = Some(at_ms);
                }
            }
            CommentCommand::Delete { comment_id } => comments.retain(|c| c.id != comment_id),
        }
        if comments.is_empty() {
            self.doc.comments = None;
        }
        true
    }
}

fn check_anchor<L: CommentLookup + ?Sized>(
    lookup: &L,
    anchor: &CommentAnchor,
) -> Result<(), CommentError> {
    let has_point = anchor.x.is_some_and(f64::is_finite) && anchor.y.is_some_and(f64::is_finite);
    let page_index = match anchor.page_id.as_deref() {
        Some(page_id) => Some(
            lookup
                .comment_page_by_id(page_id)
                .ok_or_else(|| CommentError::UnknownPage(page_id.to_owned()))?,
        ),
        None => None,
    };
    match anchor.node_id.as_deref() {
        Some(node_id) => {
            let node_page = lookup
                .comment_page_of_node(node_id)
                .ok_or_else(|| CommentError::UnknownNode(node_id.to_owned()))?;
            if let (Some(page), Some(page_id)) = (page_index, anchor.page_id.as_deref()) {
                if page != node_page {
                    return Err(CommentError::NodeNotOnPage {
                        node_id: node_id.to_owned(),
                        page_id: page_id.to_owned(),
                    });
                }
            }
            Ok(())
        }
        None if has_point => Ok(()),
        None => Err(CommentError::EmptyAnchor),
    }
}

fn check_text(text: &str) -> Result<(), CommentError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(CommentError::EmptyText);
    }
    let chars = trimmed.chars().count();
    if chars > MAX_COMMENT_TEXT_CHARS {
        return Err(CommentError::TextTooLong { chars });
    }
    Ok(())
}

#[cfg(test)]
#[path = "comments_tests.rs"]
mod tests;

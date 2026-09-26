//! Node body -> `PenNode` for the `batch_design` DSL executor: lenient JSON,
//! weak-model dialect repair (`batch_program_dialect`), handle-reference
//! lifting (`batch_program_handle_refs`), shape normalisation, typed
//! deserialisation, and the optional post-process refine.

use jian_ops_schema::node::PenNode;
use serde::Deserialize;
use serde_json::Value;

use super::batch_design::{ensure_node_ids, normalize_node_shape};
use super::batch_program::Result;
use super::batch_program_dialect::repair_node_dialect;
use super::batch_program_error::ProgramError;
use super::batch_program_handle_refs::{extract_handle_children, HandleChildRef};
use super::batch_program_parse::parse_json_arg;

/// How a node body is parsed.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NodeParseOptions {
    /// Run the TS postProcess refine on the parsed subtree.
    pub(crate) post_process: bool,
    /// The body replaces a document root (enables the root-only refine).
    pub(crate) document_root: bool,
    /// Lift string `children` entries out as handle references (`I()`).
    pub(crate) handle_refs: bool,
    /// Last resort for the best-effort (scratch-document) policy: when the
    /// payload still fails to deserialise, drop the ONE top-level field
    /// whose removal lets it parse, rather than the whole node.
    pub(crate) salvage_fields: bool,
}

/// A parsed node body.
pub(crate) struct ParsedNode {
    pub(crate) node: PenNode,
    /// String `children` entries lifted out (empty unless requested).
    pub(crate) handle_refs: Vec<HandleChildRef>,
    /// Every dialect rewrite / salvage applied, for the envelope's
    /// `warnings[]`.
    pub(crate) notes: Vec<String>,
}

/// Parse + normalize an I()/R() node body into a `PenNode` with authored ids
/// filled in (the caller remaps them to final ids). `document_root` controls
/// the one refine pass that is valid only for a real document root; child
/// insertion payloads still receive every subtree-safe post-process fix.
/// (The program executor itself calls [`parse_node_body`]; this default-option
/// form serves the `script` generator runtime.)
#[cfg(feature = "script")]
pub(crate) fn parse_node_json(
    raw: &str,
    post_process: bool,
    document_root: bool,
) -> Result<PenNode> {
    parse_node_body(
        raw,
        NodeParseOptions {
            post_process,
            document_root,
            ..NodeParseOptions::default()
        },
    )
    .map(|parsed| parsed.node)
}

pub(crate) fn parse_node_body(raw: &str, options: NodeParseOptions) -> Result<ParsedNode> {
    let mut value = parse_json_arg(raw)?;
    if !value.is_object() {
        return Err(ProgramError::Json("node data must be a JSON object".into()));
    }
    let mut notes = Vec::new();
    repair_node_dialect(&mut value, &mut notes);
    let handle_refs = if options.handle_refs {
        extract_handle_children(&mut value)
    } else {
        Vec::new()
    };
    normalize_node_shape(&mut value);
    let mut tmp = 1usize;
    ensure_node_ids(&mut value, &mut tmp);
    let mut node = match PenNode::deserialize(&value) {
        Ok(node) => node,
        Err(error) => {
            let salvaged = if options.salvage_fields {
                salvage_one_field(&value, &mut notes)
            } else {
                None
            };
            salvaged.ok_or_else(|| {
                ProgramError::InvalidNode(format!("invalid PenNode payload: {error}"))
            })?
        }
    };
    if options.post_process {
        // TS postProcess hooks (emoji strip, unique ids, layout-child
        // position sanitize, screen-bounds clamp) — the deterministic
        // subset shipped in `command_refine.rs`.
        if options.document_root {
            let _ = op_editor_core::command_refine::refine_subtree(&mut node);
        } else {
            let _ = op_editor_core::command_refine::refine_child_subtree(&mut node);
        }
    }
    Ok(ParsedNode {
        node,
        handle_refs,
        notes,
    })
}

/// Fields a salvage may never remove: identity, structure, and the required
/// content of each leaf type — dropping one of those changes WHAT the node
/// is, not just how it looks.
const SALVAGE_PROTECTED: [&str; 7] = [
    "type",
    "id",
    "children",
    "content",
    "iconFontName",
    "src",
    "name",
];

/// Try removing each non-protected top-level field in key order; the first
/// removal that deserialises wins. A node whose failure is not confined to
/// one such field stays dropped.
fn salvage_one_field(value: &Value, notes: &mut Vec<String>) -> Option<PenNode> {
    let object = value.as_object()?;
    for key in object.keys() {
        if SALVAGE_PROTECTED.contains(&key.as_str()) {
            continue;
        }
        let mut candidate = object.clone();
        candidate.remove(key);
        let candidate = Value::Object(candidate);
        if let Ok(node) = PenNode::deserialize(&candidate) {
            notes.push(format!(
                "dropped field `{key}`: the schema rejects its value"
            ));
            return Some(node);
        }
    }
    None
}

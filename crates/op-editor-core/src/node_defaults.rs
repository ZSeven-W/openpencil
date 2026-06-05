//! Shared default geometry for nodes created without explicit bounds.

/// Generic flat-leaf fallback used for non-text TS-compatible inserts.
pub const DEFAULT_LEAF_NODE_SIZE: i32 = 100;

/// Compact default for the Text tool's "Text" placeholder.
pub const DEFAULT_TEXT_NODE_WIDTH: i32 = 48;
pub const DEFAULT_TEXT_NODE_HEIGHT: i32 = 20;

/// Default `(width, height)` for a freshly-created flat leaf of `kind`.
pub fn default_leaf_node_size(kind: &str) -> (i32, i32) {
    match kind {
        "text" => (DEFAULT_TEXT_NODE_WIDTH, DEFAULT_TEXT_NODE_HEIGHT),
        _ => (DEFAULT_LEAF_NODE_SIZE, DEFAULT_LEAF_NODE_SIZE),
    }
}

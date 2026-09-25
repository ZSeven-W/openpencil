//! Turn a runtime's raw output into the children a generator owns:
//! enforce the output limits, derive deterministic path-based ids, and
//! strip nested generator markers.

use jian_ops_schema::node::PenNode;

use super::{GeneratorError, GENERATOR_EXPLAIN_PREFIX, MAX_GENERATED_DEPTH, MAX_GENERATED_NODES};
use crate::pen_node_ext::PenNodeExt;

/// Rewrite `raw` into owned children of generator `generator_id`.
///
/// Ids are a pure function of the tree path — `{gid}_g{i}` for the i-th
/// top-level child, `{parent}_{j}` for the j-th child below — so the same
/// output always lands with the same ids (determinism, stable selection
/// across regenerates) and never consumes the document's `n{N}` space.
/// Nested generator markers are stripped: generated children are plain
/// nodes, a generator does not spawn generators.
pub fn materialize_children(
    generator_id: &str,
    raw: Vec<PenNode>,
) -> Result<Vec<PenNode>, GeneratorError> {
    let mut count = 0usize;
    let mut depth = 0usize;
    for node in &raw {
        measure(node, 1, &mut count, &mut depth);
    }
    if count > MAX_GENERATED_NODES {
        return Err(GeneratorError::TooManyNodes {
            count,
            max: MAX_GENERATED_NODES,
        });
    }
    if depth > MAX_GENERATED_DEPTH {
        return Err(GeneratorError::TooDeep {
            depth,
            max: MAX_GENERATED_DEPTH,
        });
    }
    let mut out = raw;
    for (index, node) in out.iter_mut().enumerate() {
        assign_ids(node, format!("{generator_id}_g{index}"));
    }
    Ok(out)
}

/// A short stable fingerprint of a child list — FNV-1a 64 over the
/// canonical JSON. Stored in the spec after every write so the panel can
/// tell when the owned children were edited by hand.
pub fn output_hash(children: &[PenNode]) -> String {
    let json = serde_json::to_string(children).unwrap_or_default();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in json.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn measure(node: &PenNode, level: usize, count: &mut usize, depth: &mut usize) {
    *count += 1;
    *depth = (*depth).max(level);
    if let Some(children) = node.children() {
        for child in children {
            measure(child, level + 1, count, depth);
        }
    }
}

fn assign_ids(node: &mut PenNode, id: String) {
    let base = node.base_mut();
    if base
        .explain
        .as_deref()
        .is_some_and(|explain| explain.starts_with(GENERATOR_EXPLAIN_PREFIX))
    {
        base.explain = None;
    }
    base.id = id.clone();
    if let Some(children) = node.children_mut() {
        for (index, child) in children.iter_mut().enumerate() {
            assign_ids(child, format!("{id}_{index}"));
        }
    }
}

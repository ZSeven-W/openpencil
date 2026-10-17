//! Recover authored image intent from the policy-owned fallback envelope.
//! This is read-only: normal frames never gain intent from arbitrary explain
//! text, and edits to the live search query supersede the saved contract.

use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt as _;
use serde_json::Value;

use super::ImageRequestMode;
use crate::fallback_policy::{
    fallback_patch_for_branch, image_fallback_policy, ImageFallbackBranch,
};

pub(super) struct FallbackIntent {
    pub restored: PenNode,
    pub mode: ImageRequestMode,
    branch: ImageFallbackBranch,
}

pub(super) fn for_node(node: &PenNode) -> Option<FallbackIntent> {
    let PenNode::Frame(frame) = node else {
        return None;
    };
    let envelope = node
        .base()
        .explain
        .as_deref()?
        .strip_prefix("image fallback: ")?;
    let (branch, payload) = envelope.split_once(' ')?;
    let branch = match branch {
        "thumb" => ImageFallbackBranch::Thumb,
        "media" => ImageFallbackBranch::Media,
        "covered" => ImageFallbackBranch::Covered,
        _ => return None,
    };
    let metadata: Value = serde_json::from_str(payload).ok()?;
    let metadata = metadata.as_object()?;
    let query = optional_text(metadata.get("imageSearchQuery")?)?;
    let prompt = optional_text(metadata.get("imagePrompt")?)?;
    optional_text(metadata.get("originalExplain")?)?;
    let mode = match metadata.get("imageRequestMode")?.as_str()? {
        "auto" => ImageRequestMode::Auto,
        "search" => ImageRequestMode::Search,
        "generate" => ImageRequestMode::Generate,
        _ => return None,
    };
    if (mode == ImageRequestMode::Search && normalized(query).is_none())
        || (mode == ImageRequestMode::Generate && normalized(prompt).is_none())
    {
        return None;
    }
    if normalized(frame.image_search_query.as_deref()) != normalized(query) {
        return None;
    }
    Some(FallbackIntent {
        restored: crate::fallback_policy::restore_image_fallback_node(node)?,
        mode,
        branch,
    })
}

/// Only add automatic write-back eligibility for untouched policy tiles.
/// Existing explicit-query frames retain their previous rules, but a newly
/// recognized prompt-only fallback must not swallow authored descendants.
pub(super) fn is_unchanged_tile(node: &PenNode) -> bool {
    let Some(intent) = for_node(node) else {
        return false;
    };
    if intent.branch == ImageFallbackBranch::Covered {
        return node.children().is_none_or(|children| children.is_empty());
    }
    // Reconstruct decoration ownership from its saved branch. Resizing the
    // slot or reopening a resolved fill-container thumb must not switch it.
    let expected = fallback_patch_for_branch(&intent.restored, intent.branch);
    let Ok(expected) = serde_json::from_str::<Value>(&expected.patch_json) else {
        return false;
    };
    let Ok(expected) = serde_json::from_value::<Vec<PenNode>>(expected["children"].clone()) else {
        return false;
    };
    if node
        .children()
        .is_some_and(|children| children == &expected)
    {
        return true;
    }
    // Reuse the conservative legacy-caption upgrade instead of treating any
    // matching id or arbitrary fixed width as policy-owned decoration.
    image_fallback_policy(node, false).into_iter().any(|patch| {
        patch.branch == ImageFallbackBranch::Media
            && serde_json::from_str::<Value>(&patch.patch_json)
                .ok()
                .and_then(|patch| {
                    serde_json::from_value::<Vec<PenNode>>(patch["children"].clone()).ok()
                })
                .is_some_and(|children| children == expected)
    })
}

fn optional_text(value: &Value) -> Option<Option<&str>> {
    match value {
        Value::Null => Some(None),
        Value::String(value) => Some(Some(value)),
        _ => None,
    }
}

fn normalized(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

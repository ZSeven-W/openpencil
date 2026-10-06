//! Unit tests for [`editor_state_to_layout_scene`].
//!
//! This file is the spine: the shared fixture helpers plus the module
//! declarations. The cases themselves live in sibling files under
//! `layout_scene_tests/` (per the 800-line-per-file ceiling). Each is
//! wired with an explicit `#[path]` because this module is itself
//! `#[path]`-loaded under the name `tests`.

use super::*;
use jian_ops_schema::node::MaskType;

#[path = "layout_scene_tests/fills.rs"]
mod fills;
#[path = "layout_scene_tests/flex_pages.rs"]
mod flex_pages;
#[path = "layout_scene_tests/masks_blend.rs"]
mod masks_blend;
#[path = "layout_scene_tests/opacity.rs"]
mod opacity;
#[path = "layout_scene_tests/shader_preset.rs"]
mod shader_preset;
#[path = "layout_scene_tests/text.rs"]
mod text;
#[path = "layout_scene_tests/tokens_refs.rs"]
mod tokens_refs;
#[path = "layout_scene_tests/widget_stroke.rs"]
mod widget_stroke;

use op_editor_core::EditorState;

/// Build an `EditorState` from a `.op` JSON source — mirrors how the
/// `adapter_tests` fixtures parse a `PenDocument`.
fn state_from(src: &str) -> EditorState {
    let parsed = jian_ops_schema::load_str(src).expect("parse .op fixture");
    EditorState::from_document(parsed.value)
}

#[path = "layout_repair_tests.rs"]
mod layout_repair_tests;

fn max_descendant_bottom(node: &SceneNode) -> f32 {
    node.children.iter().fold(
        node.bounds.origin.y + node.bounds.size.y,
        |bottom, child| bottom.max(max_descendant_bottom(child)),
    )
}

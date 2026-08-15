//! Shared fixtures for the `image_search_session` test suite. The cases
//! themselves live in the sibling modules under
//! `image_search_session_tests/`; this file only holds the node builders
//! they all share.

use std::time::Duration;

// The cases live in the sibling modules below; names they reach for that
// sit in the source module's private submodules are pulled in here once so
// every case module picks them up through its own `use super::*`.
use super::fetch::{
    claim_openverse_result, claim_unused_image_src, fetch_first_image_url,
    first_unused_renderable_image_src, image_bytes_to_data_url, openverse_search_url,
    select_openverse_result, settle_provider_identity, wikimedia_image_candidates,
    wikimedia_page_identity, ImageCandidateClaim,
};

use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::{
    ContainerProps, FrameNode, ImageNode, PenNode, RectangleNode, TextContent, TextNode,
};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{ImageFillBody, ImageFillMode, PenFill, SolidFillBody};

fn image_node(id: &str, src: &str, query: Option<&str>) -> PenNode {
    PenNode::Image(ImageNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some("Menu photo".into()),
            ..Default::default()
        },
        src: src.into(),
        object_fit: None,
        width: Some(SizingBehavior::Number(240.0)),
        height: Some(SizingBehavior::Number(160.0)),
        corner_radius: None,
        effects: None,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
        image_prompt: None,
        image_search_query: query.map(str::to_string),
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: Default::default(),
    })
}

fn text_label(id: &str, role: Option<&str>, content: &str) -> PenNode {
    PenNode::Text(TextNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some("Label".into()),
            role: role.map(str::to_string),
            ..Default::default()
        },
        width: Some(SizingBehavior::Number(160.0)),
        height: Some(SizingBehavior::Number(24.0)),
        content: TextContent::Plain(content.to_string()),
        font_family: None,
        font_size: None,
        font_weight: None,
        font_style: None,
        letter_spacing: None,
        line_height: None,
        text_align: None,
        text_align_vertical: None,
        text_growth: None,
        underline: None,
        strikethrough: None,
        fill: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: Default::default(),
    })
}

fn frame_node(
    id: &str,
    name: &str,
    role: Option<&str>,
    fill: Option<Vec<PenFill>>,
    children: Vec<PenNode>,
) -> PenNode {
    PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.into()),
            role: role.map(str::to_string),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(240.0)),
            height: Some(SizingBehavior::Number(160.0)),
            fill,
            ..Default::default()
        },
        children: Some(children),
        image_search_query: None,
        reusable: None,
        screen: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        breakpoint: None,
    })
}

fn rectangle_node(id: &str, name: &str, fill: Option<Vec<PenFill>>) -> PenNode {
    rectangle_node_with_sizing(
        id,
        name,
        fill,
        Some(SizingBehavior::Number(240.0)),
        Some(SizingBehavior::Number(160.0)),
    )
}

fn rectangle_node_with_sizing(
    id: &str,
    name: &str,
    fill: Option<Vec<PenFill>>,
    width: Option<SizingBehavior>,
    height: Option<SizingBehavior>,
) -> PenNode {
    PenNode::Rectangle(RectangleNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.into()),
            ..Default::default()
        },
        container: ContainerProps {
            width,
            height,
            fill,
            ..Default::default()
        },
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn solid_fill() -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: "#E5E7EB".into(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

fn image_fill(url: &str) -> PenFill {
    PenFill::Image(ImageFillBody {
        url: url.into(),
        mode: Some(ImageFillMode::Crop),
        original_size: None,
        transform: None,
        tile_scale: None,
        explain: None,
        opacity: None,
        blend_mode: None,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
    })
}
// `#[path]` is required: this file is itself loaded through a `#[path]`
// attribute, so it does not own an implicit `image_search_session_tests/`
// module directory.
#[path = "image_search_session_tests/collection.rs"]
mod collection;
#[path = "image_search_session_tests/lifecycle.rs"]
mod lifecycle;
#[path = "image_search_session_tests/memo.rs"]
mod memo;
#[path = "image_search_session_tests/providers.rs"]
mod providers;

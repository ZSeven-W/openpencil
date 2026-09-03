//! Content-free structural summary of an imported reference page.

use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use serde::{Deserialize, Serialize};

use crate::role_defaults::Theme;

const MAX_SECTIONS: usize = 12;
const MAX_COLUMNS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NavKind {
    None,
    TopBar,
    Sidebar,
    BottomTabs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeroKind {
    None,
    TextLed,
    ImageLed,
    Split,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceSkeleton {
    pub source: String,
    pub width: f64,
    pub sections: Vec<SkeletonSection>,
    pub nav_kind: NavKind,
    pub hero_kind: HeroKind,
    pub column_rhythm: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkeletonSection {
    pub role: String,
    pub height_ratio: f64,
    pub child_count: u8,
    pub layout: String,
    pub has_image: bool,
}

impl ReferenceSkeleton {
    /// Build a deterministic structural summary from an imported page root.
    ///
    /// The role resolver runs on a clone so producing a summary never mutates
    /// the imported document. Names and text are deliberately not consulted
    /// after role inference, and never enter the returned structure.
    pub fn from_root(root: &PenNode, source: &str) -> Option<Self> {
        let root_children = root.children()?;
        if root_children.is_empty() {
            return None;
        }

        let width = root.width_px().unwrap_or(0.0).max(0.0);
        let mut resolved = root.clone();
        crate::role_infer::resolve_forest_roles(
            std::slice::from_mut(&mut resolved),
            width,
            Theme::Unknown,
        );

        let children = resolved.children()?;
        if children.is_empty() {
            return None;
        }
        let section_nodes = if should_unwrap(&resolved) {
            children[0].children().unwrap_or(children).as_slice()
        } else {
            children.as_slice()
        };
        if section_nodes.is_empty() {
            return None;
        }

        let total_height = root
            .height_px()
            .filter(|height| height.is_finite() && *height > 0.0)
            .unwrap_or_else(|| {
                section_nodes
                    .iter()
                    .filter_map(PenNodeExt::height_px)
                    .filter(|height| height.is_finite() && *height > 0.0)
                    .sum()
            });
        let mut sections: Vec<SkeletonSection> = section_nodes
            .iter()
            .map(|node| skeleton_section(node, total_height))
            .collect();
        sections.truncate(MAX_SECTIONS);

        let nav_kind = detect_nav_kind(&resolved, &sections, section_nodes);
        let hero_kind = detect_hero_kind(&sections, section_nodes);
        let column_rhythm = section_nodes
            .iter()
            .enumerate()
            .take(sections.len())
            .filter(|(index, _)| is_content_section(*index, &sections, hero_kind))
            .map(|(_, node)| horizontal_child_count(node))
            .collect();

        Some(Self {
            source: normalize_source(source),
            width,
            sections,
            nav_kind,
            hero_kind,
            column_rhythm,
        })
    }

    /// Render only the structural guide that is safe to send to the planner.
    pub fn render(&self) -> String {
        let mut lines = vec![
            "REFERENCE SKELETON (structure only — do NOT copy any text, brand, image, or color from the reference; this is a section order + rhythm guide):".to_string(),
            format!(
                "source: {}, width: {}, nav: {}, hero: {}",
                self.source,
                self.width,
                nav_name(self.nav_kind),
                hero_name(self.hero_kind)
            ),
        ];
        for (index, section) in self.sections.iter().enumerate() {
            let image = if section.has_image { ", image" } else { "" };
            lines.push(format!(
                "{}. role={} height≈{}% children={} layout={}{}",
                index + 1,
                section.role,
                (section.height_ratio * 100.0).round(),
                section.child_count,
                section.layout,
                image
            ));
        }
        let rhythm = self
            .column_rhythm
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!("column rhythm: {rhythm}"));
        lines.join("\n")
    }
}

fn should_unwrap(root: &PenNode) -> bool {
    let Some(children) = root.children() else {
        return false;
    };
    if children.len() != 1 {
        return false;
    }
    let wrapper = &children[0];
    let Some(wrapper_children) = wrapper.children() else {
        return false;
    };
    wrapper_children.len() >= 3 && covers_root(wrapper, root)
}

fn covers_root(wrapper: &PenNode, root: &PenNode) -> bool {
    let (Some(root_width), Some(root_height), Some(wrapper_width), Some(wrapper_height)) = (
        root.width_px(),
        root.height_px(),
        wrapper.width_px(),
        wrapper.height_px(),
    ) else {
        return false;
    };
    root_width > 0.0
        && root_height > 0.0
        && wrapper_width >= root_width * 0.9
        && wrapper_height >= root_height * 0.9
}

fn skeleton_section(node: &PenNode, total_height: f64) -> SkeletonSection {
    let height = node.height_px().unwrap_or(0.0);
    let height_ratio = if total_height.is_finite() && total_height > 0.0 {
        (height.max(0.0) / total_height).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let role = node
        .base()
        .role
        .clone()
        .or_else(|| crate::role_infer::infer_role_from_name(node).map(str::to_string))
        .unwrap_or_else(|| "section".to_string());
    SkeletonSection {
        role,
        height_ratio: (height_ratio * 100.0).round() / 100.0,
        child_count: node
            .children()
            .map(|children| children.len().min(u8::MAX as usize) as u8)
            .unwrap_or(0),
        layout: layout_name(node),
        has_image: has_image(node),
    }
}

fn detect_nav_kind(root: &PenNode, sections: &[SkeletonSection], nodes: &[PenNode]) -> NavKind {
    if is_sidebar_root(root, nodes.first()) {
        return NavKind::Sidebar;
    }
    if sections
        .last()
        .is_some_and(|section| is_bottom_role(&section.role))
    {
        return NavKind::BottomTabs;
    }
    if sections
        .iter()
        .take(2)
        .any(|section| is_top_role(&section.role))
    {
        return NavKind::TopBar;
    }
    NavKind::None
}

fn detect_hero_kind(sections: &[SkeletonSection], nodes: &[PenNode]) -> HeroKind {
    let Some((index, section)) = sections
        .iter()
        .enumerate()
        .find(|(index, section)| !is_nav_index(*index, sections) && section.role == "hero")
    else {
        return HeroKind::None;
    };
    let node = &nodes[index];
    let text = has_text(node);
    if section.has_image && text {
        if layout_name(node) == "horizontal"
            && node.children().is_some_and(|children| children.len() >= 2)
        {
            HeroKind::Split
        } else {
            HeroKind::ImageLed
        }
    } else if section.has_image {
        HeroKind::ImageLed
    } else {
        HeroKind::TextLed
    }
}

fn is_content_section(index: usize, sections: &[SkeletonSection], hero_kind: HeroKind) -> bool {
    let Some(section) = sections.get(index) else {
        return false;
    };
    if is_nav_index(index, sections) || section.role == "footer" {
        return false;
    }
    hero_kind == HeroKind::None || section.role != "hero"
}

fn is_nav_index(index: usize, sections: &[SkeletonSection]) -> bool {
    let Some(section) = sections.get(index) else {
        return false;
    };
    (index < 2 && is_top_role(&section.role))
        || (index + 1 == sections.len() && is_bottom_role(&section.role))
}

fn is_sidebar_root(root: &PenNode, first: Option<&PenNode>) -> bool {
    layout_name(root) == "horizontal"
        && first.is_some_and(|node| {
            matches!(layout_name(node).as_str(), "vertical")
                && node.width_px().is_some_and(|width| width <= 320.0)
                && node.base().role.as_deref().is_some_and(is_sidebar_role)
        })
}

fn is_top_role(role: &str) -> bool {
    matches!(
        role.to_ascii_lowercase().as_str(),
        "nav" | "navbar" | "topbar" | "header" | "navigation"
    )
}

fn is_sidebar_role(role: &str) -> bool {
    matches!(role.to_ascii_lowercase().as_str(), "nav" | "sidebar")
}

fn is_bottom_role(role: &str) -> bool {
    matches!(
        role.to_ascii_lowercase().as_str(),
        "bottom-tab-bar" | "tab-bar"
    )
}

fn horizontal_child_count(node: &PenNode) -> u8 {
    let mut maximum = 0;
    collect_horizontal_counts(node, 0, &mut maximum);
    maximum
}

fn collect_horizontal_counts(node: &PenNode, depth: usize, maximum: &mut u8) {
    if depth > 2 {
        return;
    }
    if layout_name(node) == "horizontal" {
        if let Some(children) = node.children() {
            if children.len() >= 2 {
                *maximum = (*maximum).max(children.len().min(MAX_COLUMNS) as u8);
            }
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_horizontal_counts(child, depth + 1, maximum);
        }
    }
}

fn layout_name(node: &PenNode) -> String {
    crate::role_defaults::node_layout_string(node)
        .map(|layout| match layout.as_str() {
            "vertical" => "vertical",
            "horizontal" => "horizontal",
            "grid" => "grid",
            _ => "none",
        })
        .unwrap_or("none")
        .to_string()
}

fn has_image(node: &PenNode) -> bool {
    matches!(node, PenNode::Image(_))
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_image))
}

fn has_text(node: &PenNode) -> bool {
    matches!(node, PenNode::Text(_))
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_text))
}

fn normalize_source(source: &str) -> String {
    let source = source.trim();
    if source.eq_ignore_ascii_case("screenshot") {
        return "screenshot".to_string();
    }
    let source = source
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(source);
    let source = source.split(['/', '?', '#']).next().unwrap_or(source);
    source.to_ascii_lowercase()
}

fn nav_name(kind: NavKind) -> &'static str {
    match kind {
        NavKind::None => "None",
        NavKind::TopBar => "TopBar",
        NavKind::Sidebar => "Sidebar",
        NavKind::BottomTabs => "BottomTabs",
    }
}

fn hero_name(kind: HeroKind) -> &'static str {
    match kind {
        HeroKind::None => "None",
        HeroKind::TextLed => "TextLed",
        HeroKind::ImageLed => "ImageLed",
        HeroKind::Split => "Split",
    }
}

#[cfg(test)]
#[path = "reference_skeleton_tests.rs"]
mod tests;

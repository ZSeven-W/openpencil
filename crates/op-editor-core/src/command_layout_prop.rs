//! `SetNodeLayoutProp` command application — sets layout / text
//! properties that do not have their own typed `EditorCommand` variants.
//!
//! Covers: `layout`, `gap`, `padding`, `letterSpacing`, `lineHeight`,
//! `opacity`, `fontFamily`, `textAlign`, `textAlignVertical`,
//! `textGrowth`, `alignItems`, `justifyContent`, `clipContent`, and
//! sizing keywords for `width` / `height` (`"fit_content"` /
//! `"fill_container"`).
//!
//! Each property dispatches to the canonical node field via a
//! per-variant match, using the validate-then-mutate discipline.

use crate::command::LayoutPropValue;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers::find_node_mut;
use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{AlignItems, JustifyContent, LayoutMode, Padding};
use jian_ops_schema::node::text::{TextAlign, TextAlignVertical, TextGrowth};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

// ── helper: parse justify_content keyword ────────────────────────────────────

fn parse_justify_content(s: &str) -> Option<JustifyContent> {
    match s {
        "start" => Some(JustifyContent::Start),
        "center" => Some(JustifyContent::Center),
        "end" => Some(JustifyContent::End),
        "space_between" => Some(JustifyContent::SpaceBetween),
        "space_around" => Some(JustifyContent::SpaceAround),
        _ => None,
    }
}

// ── helper: parse align_items keyword ────────────────────────────────────────

// Accepts the full CSS-ish alias set TS tolerates (incl. `stretch`,
// `flex-start`, `baseline`, …) via the canonical schema's `from_css`, so a
// set-layout command carrying any value a TS builder/file emits is honored
// rather than silently rejected.
fn parse_align_items(s: &str) -> Option<AlignItems> {
    AlignItems::from_css(s)
}

// ── helper: parse layout keyword ─────────────────────────────────────────────

fn parse_layout_mode(s: &str) -> Option<LayoutMode> {
    match s {
        "none" => Some(LayoutMode::None),
        "vertical" => Some(LayoutMode::Vertical),
        "horizontal" => Some(LayoutMode::Horizontal),
        _ => None,
    }
}

// ── helper: parse text_align keyword ─────────────────────────────────────────

fn parse_text_align(s: &str) -> Option<TextAlign> {
    match s {
        "left" => Some(TextAlign::Left),
        "center" => Some(TextAlign::Center),
        "right" => Some(TextAlign::Right),
        "justify" => Some(TextAlign::Justify),
        _ => None,
    }
}

// ── helper: parse text_align_vertical keyword ────────────────────────────────

fn parse_text_align_vertical(s: &str) -> Option<TextAlignVertical> {
    match s {
        "top" => Some(TextAlignVertical::Top),
        "middle" => Some(TextAlignVertical::Middle),
        "bottom" => Some(TextAlignVertical::Bottom),
        _ => None,
    }
}

// ── helper: parse text_growth keyword ────────────────────────────────────────

fn parse_text_growth(s: &str) -> Option<TextGrowth> {
    match s {
        "auto" => Some(TextGrowth::Auto),
        "fixed-width" => Some(TextGrowth::FixedWidth),
        "fixed-width-height" => Some(TextGrowth::FixedWidthHeight),
        _ => None,
    }
}

// ── helper: parse sizing keyword ──────────────────────────────────────────────

fn parse_sizing_keyword(s: &str) -> Option<SizingBehavior> {
    match s {
        "fit_content" => Some(SizingBehavior::Keyword(SizingKeyword::FitContent)),
        "fill_container" => Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)),
        _ => None,
    }
}

// ── helper: build Padding from LayoutPropValue ────────────────────────────────

fn build_padding(value: &LayoutPropValue) -> Option<Padding> {
    match value {
        LayoutPropValue::Number(n) => {
            if n.is_finite() {
                Some(Padding::Uniform(*n))
            } else {
                None
            }
        }
        LayoutPropValue::NumberArray(arr) => match arr.len() {
            1 => Some(Padding::Uniform(arr[0])),
            2 => Some(Padding::XY([arr[0], arr[1]])),
            4 => Some(Padding::LtrB([arr[0], arr[1], arr[2], arr[3]])),
            _ => None,
        },
        LayoutPropValue::Keyword(_) | LayoutPropValue::Bool(_) => None,
    }
}

// ── impl EditorState ──────────────────────────────────────────────────────────

impl EditorState {
    /// `SetNodeLayoutProp` — set a layout / text property on a node by
    /// name + typed value. Returns `true` when the field was written,
    /// `false` on an unknown (property, value shape) combination or a
    /// missing node.
    pub(crate) fn cmd_set_node_layout_prop(
        &mut self,
        node_id: &NodeId,
        property: &str,
        value: &LayoutPropValue,
    ) -> bool {
        if !node_id.is_real() {
            return false;
        }

        // Read-validate before taking the mutable borrow.
        match property {
            "gap" | "letterSpacing" | "lineHeight" | "opacity" => {
                let LayoutPropValue::Number(n) = value else {
                    return false;
                };
                if !n.is_finite() {
                    return false;
                }
            }
            "padding" => {
                if build_padding(value).is_none() {
                    return false;
                }
            }
            "layout" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                if parse_layout_mode(s).is_none() {
                    return false;
                }
            }
            "justifyContent" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                if parse_justify_content(s).is_none() {
                    return false;
                }
            }
            "alignItems" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                if parse_align_items(s).is_none() {
                    return false;
                }
            }
            "textAlign" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                if parse_text_align(s).is_none() {
                    return false;
                }
            }
            "textAlignVertical" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                if parse_text_align_vertical(s).is_none() {
                    return false;
                }
            }
            "fontFamily" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                if s.trim().is_empty() {
                    return false;
                }
            }
            "textGrowth" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                if parse_text_growth(s).is_none() {
                    return false;
                }
            }
            "width" | "height" => {
                // Sizing keyword variant; numeric pixel writes go through
                // the existing UpdateNode command.
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                if parse_sizing_keyword(s).is_none() {
                    return false;
                }
            }
            "clipContent" => {
                if !matches!(value, LayoutPropValue::Bool(_)) {
                    return false;
                }
            }
            _ => return false,
        }

        // Mutable phase.
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };

        let changed = match property {
            "gap" => {
                let LayoutPropValue::Number(n) = value else {
                    return false;
                };
                set_container_gap(node, *n)
            }
            "padding" => {
                let pad = build_padding(value).expect("validated above");
                set_container_padding(node, pad)
            }
            "layout" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                let mode = parse_layout_mode(s).expect("validated above");
                set_container_layout(node, mode)
            }
            "justifyContent" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                let jc = parse_justify_content(s).expect("validated above");
                set_container_justify(node, jc)
            }
            "alignItems" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                let ai = parse_align_items(s).expect("validated above");
                set_container_align(node, ai)
            }
            "letterSpacing" => {
                let LayoutPropValue::Number(n) = value else {
                    return false;
                };
                set_text_letter_spacing(node, *n)
            }
            "lineHeight" => {
                let LayoutPropValue::Number(n) = value else {
                    return false;
                };
                set_text_line_height(node, *n)
            }
            "textAlign" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                let ta = parse_text_align(s).expect("validated above");
                set_text_align(node, ta)
            }
            "textAlignVertical" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                let ta = parse_text_align_vertical(s).expect("validated above");
                set_text_align_vertical(node, ta)
            }
            "fontFamily" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                set_text_font_family(node, s.trim().to_string())
            }
            "textGrowth" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                let tg = parse_text_growth(s).expect("validated above");
                set_text_growth(node, tg)
            }
            "opacity" => {
                let LayoutPropValue::Number(n) = value else {
                    return false;
                };
                node.base_mut().opacity = Some(NumberOrExpression::Number(*n));
                true
            }
            "width" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                let sb = parse_sizing_keyword(s).expect("validated above");
                set_node_width(node, sb)
            }
            "height" => {
                let LayoutPropValue::Keyword(s) = value else {
                    return false;
                };
                let sb = parse_sizing_keyword(s).expect("validated above");
                set_node_height(node, sb)
            }
            "clipContent" => {
                let LayoutPropValue::Bool(v) = value else {
                    return false;
                };
                set_container_clip_content(node, *v)
            }
            _ => false,
        };
        if changed && property_invalidates_preserved_geometry(property) {
            self.editor_ui.preserve_authored_geometry = false;
        }
        changed
    }
}

// ── field writers ─────────────────────────────────────────────────────────────

fn property_invalidates_preserved_geometry(property: &str) -> bool {
    matches!(
        property,
        "gap"
            | "padding"
            | "layout"
            | "justifyContent"
            | "alignItems"
            | "width"
            | "height"
            | "clipContent"
            | "textGrowth"
    )
}

fn set_container_gap(node: &mut PenNode, gap: f64) -> bool {
    let noe = NumberOrExpression::Number(gap);
    match node {
        PenNode::Frame(n) => {
            n.container.gap = Some(noe);
            true
        }
        PenNode::Group(n) => {
            n.container.gap = Some(noe);
            true
        }
        PenNode::Rectangle(n) => {
            n.container.gap = Some(noe);
            true
        }
        _ => false,
    }
}

fn set_container_layout(node: &mut PenNode, mode: LayoutMode) -> bool {
    match node {
        PenNode::Frame(n) => {
            n.container.layout = Some(mode);
            true
        }
        PenNode::Group(n) => {
            n.container.layout = Some(mode);
            true
        }
        PenNode::Rectangle(n) => {
            n.container.layout = Some(mode);
            true
        }
        _ => false,
    }
}

fn set_container_padding(node: &mut PenNode, pad: Padding) -> bool {
    match node {
        PenNode::Frame(n) => {
            n.container.padding = Some(pad);
            true
        }
        PenNode::Group(n) => {
            n.container.padding = Some(pad);
            true
        }
        PenNode::Rectangle(n) => {
            n.container.padding = Some(pad);
            true
        }
        _ => false,
    }
}

fn set_container_clip_content(node: &mut PenNode, value: bool) -> bool {
    match node {
        PenNode::Frame(n) => {
            n.container.clip_content = Some(value);
            true
        }
        PenNode::Group(n) => {
            n.container.clip_content = Some(value);
            true
        }
        PenNode::Rectangle(n) => {
            n.container.clip_content = Some(value);
            true
        }
        _ => false,
    }
}

fn set_container_justify(node: &mut PenNode, jc: JustifyContent) -> bool {
    match node {
        PenNode::Frame(n) => {
            n.container.justify_content = Some(jc);
            true
        }
        PenNode::Group(n) => {
            n.container.justify_content = Some(jc);
            true
        }
        PenNode::Rectangle(n) => {
            n.container.justify_content = Some(jc);
            true
        }
        _ => false,
    }
}

fn set_container_align(node: &mut PenNode, ai: AlignItems) -> bool {
    match node {
        PenNode::Frame(n) => {
            n.container.align_items = Some(ai);
            true
        }
        PenNode::Group(n) => {
            n.container.align_items = Some(ai);
            true
        }
        PenNode::Rectangle(n) => {
            n.container.align_items = Some(ai);
            true
        }
        _ => false,
    }
}

fn set_text_letter_spacing(node: &mut PenNode, v: f64) -> bool {
    match node {
        PenNode::Text(t) => {
            t.letter_spacing = Some(v);
            true
        }
        _ => false,
    }
}

fn set_text_line_height(node: &mut PenNode, v: f64) -> bool {
    match node {
        PenNode::Text(t) => {
            t.line_height = Some(v);
            true
        }
        _ => false,
    }
}

fn set_text_align(node: &mut PenNode, ta: TextAlign) -> bool {
    match node {
        PenNode::Text(t) => {
            t.text_align = Some(ta);
            true
        }
        _ => false,
    }
}

fn set_text_align_vertical(node: &mut PenNode, ta: TextAlignVertical) -> bool {
    match node {
        PenNode::Text(t) => {
            t.text_align_vertical = Some(ta);
            true
        }
        _ => false,
    }
}

fn set_text_font_family(node: &mut PenNode, family: String) -> bool {
    match node {
        PenNode::Text(t) => {
            t.font_family = Some(family);
            true
        }
        _ => false,
    }
}

fn set_text_growth(node: &mut PenNode, tg: TextGrowth) -> bool {
    match node {
        PenNode::Text(t) => {
            t.text_growth = Some(tg);
            true
        }
        _ => false,
    }
}

fn set_node_width(node: &mut PenNode, sb: SizingBehavior) -> bool {
    match node {
        PenNode::Frame(n) => {
            n.container.width = Some(sb);
            true
        }
        PenNode::Group(n) => {
            n.container.width = Some(sb);
            true
        }
        PenNode::Rectangle(n) => {
            n.container.width = Some(sb);
            true
        }
        PenNode::Ellipse(n) => {
            n.width = Some(sb);
            true
        }
        PenNode::Polygon(n) => {
            n.width = Some(sb);
            true
        }
        PenNode::Path(n) => {
            n.width = Some(sb);
            true
        }
        PenNode::Text(n) => {
            n.width = Some(sb);
            true
        }
        PenNode::TextInput(n) => {
            n.width = Some(sb);
            true
        }
        PenNode::Image(n) => {
            n.width = Some(sb);
            true
        }
        PenNode::IconFont(n) => {
            n.width = Some(sb);
            true
        }
        _ => false,
    }
}

fn set_node_height(node: &mut PenNode, sb: SizingBehavior) -> bool {
    match node {
        PenNode::Frame(n) => {
            n.container.height = Some(sb);
            true
        }
        PenNode::Group(n) => {
            n.container.height = Some(sb);
            true
        }
        PenNode::Rectangle(n) => {
            n.container.height = Some(sb);
            true
        }
        PenNode::Ellipse(n) => {
            n.height = Some(sb);
            true
        }
        PenNode::Polygon(n) => {
            n.height = Some(sb);
            true
        }
        PenNode::Path(n) => {
            n.height = Some(sb);
            true
        }
        PenNode::Text(n) => {
            n.height = Some(sb);
            true
        }
        PenNode::TextInput(n) => {
            n.height = Some(sb);
            true
        }
        PenNode::Image(n) => {
            n.height = Some(sb);
            true
        }
        PenNode::IconFont(n) => {
            n.height = Some(sb);
            true
        }
        _ => false,
    }
}

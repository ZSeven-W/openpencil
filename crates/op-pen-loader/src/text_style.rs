//! Canonical text-content → payload conversion (styled-run aware).
//!
//! Split out of `adapter.rs` (800-line cap) when styled text landed.
//! `TextContent::Styled` keeps its flattened string for back-compat
//! (wrap, inline editing and hit-testing stay flat) and ADDITIONALLY
//! carries one [`TextRunPayload`] per segment so the canvas painter
//! can restyle each painted slice (per-segment font size / weight /
//! fill / italic / underline / strikethrough — TS rich-text parity).

// Two distinct `FontStyleKind` enums exist in the schema: the
// node-level one (`node::text`) and the per-segment one (`style`).
use jian_ops_schema::node::text::FontStyleKind as NodeFontStyle;
use jian_ops_schema::node::TextNode;
use jian_ops_schema::style::FontStyleKind as SegmentFontStyle;

use crate::payload::{NodePayload, TextRunPayload};
use crate::style_payload::parse_hex;

/// Flatten `n.content` onto `p.text` and harvest styled segments into
/// `p.text_runs`. Also resolves the node-level italic / underline /
/// strikethrough flags; per-segment flags inherit the node level when
/// the segment doesn't override them (TS segment-style precedence).
pub(crate) fn apply_text_content(p: &mut NodePayload, n: &TextNode) {
    use jian_ops_schema::node::text::TextContent;
    p.italic = matches!(n.font_style, Some(NodeFontStyle::Italic));
    p.underline = n.underline.unwrap_or(false);
    p.strikethrough = n.strikethrough.unwrap_or(false);
    match &n.content {
        TextContent::Plain(s) => {
            p.text = Some(s.clone());
        }
        TextContent::Styled(segments) => {
            p.text = Some(segments.iter().map(|seg| seg.text.as_str()).collect());
            p.text_runs = segments
                .iter()
                .map(|seg| TextRunPayload {
                    text: seg.text.clone(),
                    font_size: seg.font_size.unwrap_or(0.0).max(0.0),
                    font_weight: seg
                        .font_weight
                        .map(|w| w.min(u16::MAX as u32) as u16)
                        .unwrap_or(0),
                    fill: seg.fill.as_deref().and_then(parse_hex),
                    italic: seg
                        .font_style
                        .as_ref()
                        .map(|s| matches!(s, SegmentFontStyle::Italic))
                        .unwrap_or(p.italic),
                    underline: seg.underline.unwrap_or(p.underline),
                    strikethrough: seg.strikethrough.unwrap_or(p.strikethrough),
                })
                .collect();
        }
    }
}

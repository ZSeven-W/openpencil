//! Horizontal placement of one exported `<text>` line.
//!
//! The canvas aligns each line by absolute left / centre / right inside the
//! text box, whatever its script direction. An SVG `<text>` instead anchors
//! at `x` by its *logical* start, so a right-to-left line (Arabic, Hebrew)
//! written as a plain LTR `<text>` ran its glyphs off the wrong side of its
//! box — clipped by the parent frame down to a letter or two (issue #266).
//! This maps the canvas alignment onto `x` + `direction` + `text-anchor` so
//! the line's visual edge lands where the canvas paints it.

use crate::layout_scene::SceneTextAlign;
use crate::Rect;
use op_editor_core::text_script::{base_direction, BaseDirection};

/// `x` plus the extra attributes (leading space included) for one line.
pub(super) fn line_placement(line: &str, align: SceneTextAlign, rect: Rect) -> (f32, String) {
    let rtl = base_direction(line) == BaseDirection::Rtl;
    let (x, anchor) = match align {
        SceneTextAlign::Center => (rect.origin.x + rect.size.x / 2.0, "middle"),
        // In an RTL run `start` is the visual right edge, `end` the left.
        SceneTextAlign::Right => (
            rect.origin.x + rect.size.x,
            if rtl { "start" } else { "end" },
        ),
        SceneTextAlign::Left | SceneTextAlign::Justify => {
            (rect.origin.x, if rtl { "end" } else { "start" })
        }
    };
    let mut attrs = String::new();
    if rtl {
        attrs.push_str(r#" direction="rtl""#);
    }
    if anchor != "start" {
        attrs.push_str(&format!(r#" text-anchor="{anchor}""#));
    }
    (x, attrs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point2D;

    fn rect() -> Rect {
        Rect {
            origin: Point2D::new(24.0, 0.0),
            size: Point2D::new(372.0, 30.0),
        }
    }

    #[test]
    fn ltr_lines_keep_plain_start_anchoring() {
        let (x, attrs) = line_placement("Hello", SceneTextAlign::Left, rect());
        assert_eq!((x, attrs.as_str()), (24.0, ""));
        let (x, attrs) = line_placement("Hello", SceneTextAlign::Right, rect());
        assert_eq!((x, attrs.as_str()), (396.0, r#" text-anchor="end""#));
    }

    #[test]
    fn rtl_lines_anchor_their_visual_edge_inside_the_box() {
        let line = "مرحباً، أنا نص عربي أصيل";
        let (x, attrs) = line_placement(line, SceneTextAlign::Right, rect());
        assert_eq!((x, attrs.as_str()), (396.0, r#" direction="rtl""#));
        let (x, attrs) = line_placement(line, SceneTextAlign::Left, rect());
        assert_eq!(
            (x, attrs.as_str()),
            (24.0, r#" direction="rtl" text-anchor="end""#)
        );
        let (x, attrs) = line_placement(line, SceneTextAlign::Center, rect());
        assert_eq!(
            (x, attrs.as_str()),
            (210.0, r#" direction="rtl" text-anchor="middle""#)
        );
    }
}

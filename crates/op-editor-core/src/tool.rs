//! Editor tool — what a primary mouse drag does on the canvas.
//!
//! Ported verbatim from `openpencil-shell-core::document::Tool`. The
//! variant set + toolbar order + ident tokens are the editor's stable
//! contract; keeping them identical means the later facade swap
//! (Task 4.7) is a re-export with no behaviour change.

/// Editor tool — what primary mouse drag does on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Select,
    Rect,
    Ellipse,
    Polygon,
    Line,
    Pen,
    Text,
    Frame,
    Hand,
}

impl Tool {
    /// All tools, in toolbar display order. Single source of truth for
    /// the toolbar build path.
    pub const ALL: [Tool; 9] = [
        Tool::Select,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Polygon,
        Tool::Line,
        Tool::Pen,
        Tool::Text,
        Tool::Frame,
        Tool::Hand,
    ];

    /// Stable accesskit / DOM id token (lowercase ASCII).
    pub fn ident(self) -> &'static str {
        match self {
            Tool::Select => "select",
            Tool::Rect => "rect",
            Tool::Ellipse => "ellipse",
            Tool::Polygon => "polygon",
            Tool::Line => "line",
            Tool::Pen => "pen",
            Tool::Text => "text",
            Tool::Frame => "frame",
            Tool::Hand => "hand",
        }
    }

    /// True when this tool sits inside the Toolbar's shape-slot
    /// dropdown — paints whichever variant is currently active.
    pub fn is_shape(self) -> bool {
        matches!(
            self,
            Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line | Tool::Pen
        )
    }
}

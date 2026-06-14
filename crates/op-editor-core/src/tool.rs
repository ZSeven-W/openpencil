//! Editor tool — what a primary mouse drag does on the canvas.
//!
//! Ported verbatim from `openpencil-shell-core::document::Tool`. The
//! variant set + toolbar order + ident tokens are the editor's stable
//! contract; keeping them identical means the later facade swap
//! (Task 4.7) is a re-export with no behaviour change.

/// Editor tool — what primary mouse drag does on the canvas.
///
/// The first nine variants are the original shape / frame / text /
/// navigation set. The trailing ten are the form-widget tools (Phase
/// D2): selecting one and clicking the canvas drops a default widget
/// node of the matching kind.
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
    // --- Form widgets (Phase D2) -------------------------------------
    TextInput,
    TextArea,
    NumberInput,
    Select_,
    RadioGroup,
    Switch,
    Checkbox,
    Slider,
    Progress,
    Tabs,
}

impl Tool {
    /// All tools, in toolbar display order. Single source of truth for
    /// the toolbar build path.
    pub const ALL: [Tool; 19] = [
        Tool::Select,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Polygon,
        Tool::Line,
        Tool::Pen,
        Tool::Text,
        Tool::Frame,
        Tool::Hand,
        Tool::TextInput,
        Tool::TextArea,
        Tool::NumberInput,
        Tool::Select_,
        Tool::RadioGroup,
        Tool::Switch,
        Tool::Checkbox,
        Tool::Slider,
        Tool::Progress,
        Tool::Tabs,
    ];

    /// The ten form-widget tools, in toolbar / flyout display order.
    /// Single source of truth for the widgets flyout build path.
    pub const WIDGETS: [Tool; 10] = [
        Tool::TextInput,
        Tool::TextArea,
        Tool::NumberInput,
        Tool::Select_,
        Tool::RadioGroup,
        Tool::Switch,
        Tool::Checkbox,
        Tool::Slider,
        Tool::Progress,
        Tool::Tabs,
    ];

    /// Stable accesskit / DOM id token (lowercase ASCII). Widget tools
    /// share the canonical `snake_case` node kind string so the token
    /// doubles as the `kind` for the InsertNode / MCP build path.
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
            Tool::TextInput => "text_input",
            Tool::TextArea => "text_area",
            Tool::NumberInput => "number_input",
            Tool::Select_ => "select_widget",
            Tool::RadioGroup => "radio_group",
            Tool::Switch => "switch",
            Tool::Checkbox => "checkbox",
            Tool::Slider => "slider",
            Tool::Progress => "progress",
            Tool::Tabs => "tabs",
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

    /// True when this tool is one of the ten form-widget tools.
    pub fn is_widget(self) -> bool {
        self.widget_kind().is_some()
    }

    /// The canonical `snake_case` node kind a widget tool builds, or
    /// `None` for the non-widget tools. The returned string is exactly
    /// the `kind` the InsertNode / MCP factory ([`crate::command_node`])
    /// resolves into the matching widget `PenNode`, so the canvas-click
    /// path and the command path stay in lock-step.
    pub fn widget_kind(self) -> Option<&'static str> {
        match self {
            Tool::TextInput => Some("text_input"),
            Tool::TextArea => Some("text_area"),
            Tool::NumberInput => Some("number_input"),
            Tool::Select_ => Some("select"),
            Tool::RadioGroup => Some("radio_group"),
            Tool::Switch => Some("switch"),
            Tool::Checkbox => Some("checkbox"),
            Tool::Slider => Some("slider"),
            Tool::Progress => Some("progress"),
            Tool::Tabs => Some("tabs"),
            _ => None,
        }
    }
}

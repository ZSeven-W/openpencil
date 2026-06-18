//! Layout property dispatch helpers for the web property panel.

use super::WidgetHost;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::PropertyFocus;

impl WidgetHost {
    pub(in crate::widget_host) fn set_selected_layout_mode(
        &mut self,
        mode: op_editor_core::FlexLayout,
    ) {
        let id = self.editor_state.selection.anchor.clone();
        if !id.is_real() {
            return;
        }
        let value = match mode {
            op_editor_core::FlexLayout::Free => "none",
            op_editor_core::FlexLayout::Vertical => "vertical",
            op_editor_core::FlexLayout::Horizontal => "horizontal",
        };
        self.editor_state.commit_history();
        let _ = self
            .editor_state
            .apply(op_editor_core::EditorCommand::SetNodeLayoutProp {
                node_id: id,
                property: "layout".to_string(),
                value: op_editor_core::LayoutPropValue::Keyword(value.to_string()),
            });
    }

    pub(in crate::widget_host) fn toggle_selected_sizing(
        &mut self,
        width: bool,
        keyword: SizingKeyword,
    ) {
        let id = self.editor_state.selection.anchor.clone();
        if !id.is_real() {
            return;
        }
        let (is_current, fallback) = {
            let Some(node) = self.editor_state.selected_node() else {
                return;
            };
            let sizing = selected_sizing(node, width);
            let is_current = matches!(sizing, Some(SizingBehavior::Keyword(k)) if *k == keyword);
            let bounds = op_editor_core::aggregate_bounds(node);
            (is_current, if width { bounds.w } else { bounds.h })
        };
        self.editor_state.commit_history();
        if is_current {
            let focus = if width {
                PropertyFocus::SizeW
            } else {
                PropertyFocus::SizeH
            };
            let _ = self
                .editor_state
                .commit_property_edit(focus, fallback.max(0.0) as f32);
        } else {
            let prop = if width { "width" } else { "height" };
            let value = match keyword {
                SizingKeyword::FitContent => "fit_content",
                SizingKeyword::FillContainer => "fill_container",
            };
            let _ = self
                .editor_state
                .apply(op_editor_core::EditorCommand::SetNodeLayoutProp {
                    node_id: id,
                    property: prop.to_string(),
                    value: op_editor_core::LayoutPropValue::Keyword(value.to_string()),
                });
        }
    }

    pub(in crate::widget_host) fn toggle_selected_clip_content(&mut self) {
        let id = self.editor_state.selection.anchor.clone();
        if !id.is_real() {
            return;
        }
        let current = self
            .editor_state
            .selected_node()
            .map(selected_clip_content)
            .unwrap_or(false);
        self.editor_state.commit_history();
        let _ = self
            .editor_state
            .apply(op_editor_core::EditorCommand::SetNodeLayoutProp {
                node_id: id,
                property: "clipContent".to_string(),
                value: op_editor_core::LayoutPropValue::Bool(!current),
            });
    }

    pub(in crate::widget_host) fn set_selected_text_align(
        &mut self,
        value: op_editor_ui::widgets::property_panel::TextAlignValue,
    ) {
        let keyword = match value {
            op_editor_ui::widgets::property_panel::TextAlignValue::Left => "left",
            op_editor_ui::widgets::property_panel::TextAlignValue::Center => "center",
            op_editor_ui::widgets::property_panel::TextAlignValue::Right => "right",
            op_editor_ui::widgets::property_panel::TextAlignValue::Justify => "justify",
        };
        self.set_selected_layout_keyword("textAlign", keyword);
    }

    pub(in crate::widget_host) fn set_selected_layout_align(
        &mut self,
        value: op_editor_ui::widgets::property_panel::LayoutAlignValue,
    ) {
        let keyword = match value {
            op_editor_ui::widgets::property_panel::LayoutAlignValue::Start => "start",
            op_editor_ui::widgets::property_panel::LayoutAlignValue::Center => "center",
            op_editor_ui::widgets::property_panel::LayoutAlignValue::End => "end",
        };
        self.set_selected_layout_keyword("alignItems", keyword);
    }

    pub(in crate::widget_host) fn set_selected_layout_justify(
        &mut self,
        value: op_editor_ui::widgets::property_panel::LayoutJustifyValue,
    ) {
        let keyword = match value {
            op_editor_ui::widgets::property_panel::LayoutJustifyValue::Start => "start",
            op_editor_ui::widgets::property_panel::LayoutJustifyValue::Center => "center",
            op_editor_ui::widgets::property_panel::LayoutJustifyValue::End => "end",
            op_editor_ui::widgets::property_panel::LayoutJustifyValue::SpaceBetween => {
                "space_between"
            }
            op_editor_ui::widgets::property_panel::LayoutJustifyValue::SpaceAround => {
                "space_around"
            }
        };
        self.set_selected_layout_keyword("justifyContent", keyword);
    }

    pub(in crate::widget_host) fn set_selected_text_vertical_align(
        &mut self,
        value: op_editor_ui::widgets::property_panel::TextVerticalAlignValue,
    ) {
        let keyword = match value {
            op_editor_ui::widgets::property_panel::TextVerticalAlignValue::Top => "top",
            op_editor_ui::widgets::property_panel::TextVerticalAlignValue::Middle => "middle",
            op_editor_ui::widgets::property_panel::TextVerticalAlignValue::Bottom => "bottom",
        };
        self.set_selected_layout_keyword("textAlignVertical", keyword);
    }

    pub(in crate::widget_host) fn set_selected_text_growth(
        &mut self,
        value: op_editor_ui::widgets::property_panel::TextGrowthValue,
    ) {
        let keyword = match value {
            op_editor_ui::widgets::property_panel::TextGrowthValue::Auto => "auto",
            op_editor_ui::widgets::property_panel::TextGrowthValue::FixedWidth => "fixed-width",
            op_editor_ui::widgets::property_panel::TextGrowthValue::FixedWidthHeight => {
                "fixed-width-height"
            }
        };
        self.set_selected_layout_keyword("textGrowth", keyword);
    }

    fn set_selected_layout_keyword(&mut self, property: &str, keyword: &str) {
        let id = self.editor_state.selection.anchor.clone();
        if !id.is_real() {
            return;
        }
        self.editor_state.commit_history();
        let _ = self
            .editor_state
            .apply(op_editor_core::EditorCommand::SetNodeLayoutProp {
                node_id: id,
                property: property.to_string(),
                value: op_editor_core::LayoutPropValue::Keyword(keyword.to_string()),
            });
    }
}

fn selected_sizing(node: &PenNode, width: bool) -> Option<&SizingBehavior> {
    match (node, width) {
        (PenNode::Frame(n), true) => n.container.width.as_ref(),
        (PenNode::Frame(n), false) => n.container.height.as_ref(),
        (PenNode::Group(n), true) => n.container.width.as_ref(),
        (PenNode::Group(n), false) => n.container.height.as_ref(),
        (PenNode::Rectangle(n), true) => n.container.width.as_ref(),
        (PenNode::Rectangle(n), false) => n.container.height.as_ref(),
        (PenNode::Ellipse(n), true) => n.width.as_ref(),
        (PenNode::Ellipse(n), false) => n.height.as_ref(),
        (PenNode::Polygon(n), true) => n.width.as_ref(),
        (PenNode::Polygon(n), false) => n.height.as_ref(),
        (PenNode::Path(n), true) => n.width.as_ref(),
        (PenNode::Path(n), false) => n.height.as_ref(),
        (PenNode::Text(n), true) => n.width.as_ref(),
        (PenNode::Text(n), false) => n.height.as_ref(),
        (PenNode::TextInput(n), true) => n.width.as_ref(),
        (PenNode::TextInput(n), false) => n.height.as_ref(),
        (PenNode::Image(n), true) => n.width.as_ref(),
        (PenNode::Image(n), false) => n.height.as_ref(),
        (PenNode::IconFont(n), true) => n.width.as_ref(),
        (PenNode::IconFont(n), false) => n.height.as_ref(),
        (PenNode::TextArea(n), true) => n.width.as_ref(),
        (PenNode::TextArea(n), false) => n.height.as_ref(),
        (PenNode::Select(n), true) => n.width.as_ref(),
        (PenNode::Select(n), false) => n.height.as_ref(),
        (PenNode::Switch(n), true) => n.width.as_ref(),
        (PenNode::Switch(n), false) => n.height.as_ref(),
        (PenNode::Checkbox(n), true) => n.width.as_ref(),
        (PenNode::Checkbox(n), false) => n.height.as_ref(),
        (PenNode::Slider(n), true) => n.width.as_ref(),
        (PenNode::Slider(n), false) => n.height.as_ref(),
        (PenNode::RadioGroup(n), true) => n.width.as_ref(),
        (PenNode::RadioGroup(n), false) => n.height.as_ref(),
        (PenNode::NumberInput(n), true) => n.width.as_ref(),
        (PenNode::NumberInput(n), false) => n.height.as_ref(),
        (PenNode::Progress(n), true) => n.width.as_ref(),
        (PenNode::Progress(n), false) => n.height.as_ref(),
        (PenNode::Tabs(n), true) => n.width.as_ref(),
        (PenNode::Tabs(n), false) => n.height.as_ref(),
        (PenNode::Line(_) | PenNode::Ref(_), _) => None,
    }
}

fn selected_clip_content(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(n) => n.container.clip_content.unwrap_or(false),
        PenNode::Group(n) => n.container.clip_content.unwrap_or(false),
        PenNode::Rectangle(n) => n.container.clip_content.unwrap_or(false),
        _ => false,
    }
}

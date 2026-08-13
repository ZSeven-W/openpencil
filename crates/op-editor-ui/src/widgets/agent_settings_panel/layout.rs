//! Responsive shell geometry for Agent Settings.
//!
//! Paint, hit-testing, scrolling, and host-facing viewport queries all resolve
//! this value instead of independently rebuilding the touch layout.

use crate::widgets::agent_settings_panel_geometry::{
    close_rect, content_paint_clip_rect, content_rect, nav_item_rect,
};
use crate::{Point2D, Rect};
use op_editor_core::editor_ui_state::EditorUiState;

const COMPACT_HEADER_H: f32 = 56.0;
const MEDIUM_HEADER_H: f32 = 60.0;
const TOUCH_TAB_H: f32 = 44.0;
const TOUCH_TAB_GAP: f32 = 4.0;
const TOUCH_CLOSE_TARGET: f32 = 44.0;
const TOUCH_CLOSE_ICON: f32 = 18.0;
const EXPANDED_RAIL_W: f32 = 184.0;
const EXPANDED_HEADER_H: f32 = 64.0;
const EXPANDED_TAB_H: f32 = 48.0;
const TOUCH_CONTENT_MAX_W: f32 = 680.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsSurfaceKind {
    Desktop,
    Compact,
    Medium,
    Expanded,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentSettingsLayout {
    pub surface: Rect,
    pub header: Rect,
    pub navigation: Rect,
    pub content: Rect,
    pub close_target: Rect,
    pub close_icon: Rect,
    pub kind: AgentSettingsSurfaceKind,
    pub vertical_navigation: bool,
    navigation_columns: usize,
}

impl AgentSettingsLayout {
    pub(super) fn resolve(panel: Rect, ui: &EditorUiState, tab_count: usize) -> Self {
        if !ui.touch_chrome() {
            let close = close_rect(panel);
            return Self {
                surface: panel,
                header: Rect {
                    origin: panel.origin,
                    size: Point2D::new(panel.size.x, super::HEADER_HEIGHT),
                },
                navigation: Rect {
                    origin: Point2D::new(
                        panel.origin.x + super::PAD,
                        panel.origin.y + super::TAB_BAR_TOP,
                    ),
                    size: Point2D::new(
                        (panel.size.x - super::PAD * 2.0).max(0.0),
                        super::TAB_HEIGHT,
                    ),
                },
                content: content_rect(panel),
                close_target: close,
                close_icon: close,
                kind: AgentSettingsSurfaceKind::Desktop,
                vertical_navigation: false,
                navigation_columns: tab_count.max(1),
            };
        }

        if ui.expanded_touch_layout() {
            return Self::expanded(panel, tab_count);
        }
        if ui.medium_layout() {
            return Self::medium(panel);
        }
        Self::compact(panel, tab_count)
    }

    fn compact(panel: Rect, tab_count: usize) -> Self {
        let header = Rect {
            origin: panel.origin,
            size: Point2D::new(panel.size.x, COMPACT_HEADER_H),
        };
        // Five account-enabled tabs become a 3+2 grid on phones. This keeps
        // every destination at least 44 pt without squeezing localized copy
        // into overflowing desktop-style pills.
        let navigation_columns = if tab_count > 4 { 3 } else { tab_count.max(1) };
        let navigation_rows = tab_count.max(1).div_ceil(navigation_columns);
        let navigation = Rect {
            origin: Point2D::new(panel.origin.x + 12.0, header.origin.y + header.size.y + 4.0),
            size: Point2D::new(
                (panel.size.x - 24.0).max(0.0),
                navigation_rows as f32 * TOUCH_TAB_H
                    + navigation_rows.saturating_sub(1) as f32 * TOUCH_TAB_GAP,
            ),
        };
        let content_y = navigation.origin.y + navigation.size.y + 16.0;
        let content = Rect {
            origin: Point2D::new(panel.origin.x + 16.0, content_y),
            size: Point2D::new(
                (panel.size.x - 32.0).max(0.0),
                (panel.origin.y + panel.size.y - 16.0 - content_y).max(0.0),
            ),
        };
        let mut layout = Self::touch_shell(
            panel,
            header,
            navigation,
            content,
            AgentSettingsSurfaceKind::Compact,
            false,
        );
        layout.navigation_columns = navigation_columns;
        layout
    }

    fn medium(panel: Rect) -> Self {
        let header = Rect {
            origin: panel.origin,
            size: Point2D::new(panel.size.x, MEDIUM_HEADER_H),
        };
        let column_w = (panel.size.x - 48.0).clamp(0.0, TOUCH_CONTENT_MAX_W);
        let column_x = panel.origin.x + (panel.size.x - column_w) / 2.0;
        let navigation = Rect {
            origin: Point2D::new(column_x, header.origin.y + header.size.y + 4.0),
            size: Point2D::new(column_w, TOUCH_TAB_H),
        };
        let content_y = navigation.origin.y + navigation.size.y + 20.0;
        let content = Rect {
            origin: Point2D::new(column_x, content_y),
            size: Point2D::new(
                column_w,
                (panel.origin.y + panel.size.y - 24.0 - content_y).max(0.0),
            ),
        };
        let mut layout = Self::touch_shell(
            panel,
            header,
            navigation,
            content,
            AgentSettingsSurfaceKind::Medium,
            false,
        );
        layout.navigation_columns = usize::MAX;
        layout
    }

    fn expanded(panel: Rect, tab_count: usize) -> Self {
        let header = Rect {
            origin: panel.origin,
            size: Point2D::new(EXPANDED_RAIL_W, EXPANDED_HEADER_H),
        };
        let navigation = Rect {
            origin: Point2D::new(panel.origin.x + 12.0, header.origin.y + header.size.y + 8.0),
            size: Point2D::new(
                EXPANDED_RAIL_W - 24.0,
                tab_count.max(1) as f32 * EXPANDED_TAB_H
                    + tab_count.saturating_sub(1) as f32 * TOUCH_TAB_GAP,
            ),
        };
        let detail_x = panel.origin.x + EXPANDED_RAIL_W;
        let detail_w = (panel.size.x - EXPANDED_RAIL_W).max(0.0);
        let column_w = (detail_w - 48.0).clamp(0.0, TOUCH_CONTENT_MAX_W);
        let column_x = detail_x + (detail_w - column_w) / 2.0;
        let content_y = panel.origin.y + EXPANDED_HEADER_H + 16.0;
        let content = Rect {
            origin: Point2D::new(column_x, content_y),
            size: Point2D::new(
                column_w,
                (panel.origin.y + panel.size.y - 24.0 - content_y).max(0.0),
            ),
        };
        let mut layout = Self::touch_shell(
            panel,
            header,
            navigation,
            content,
            AgentSettingsSurfaceKind::Expanded,
            true,
        );
        layout.navigation_columns = 1;
        layout
    }

    fn touch_shell(
        panel: Rect,
        header: Rect,
        navigation: Rect,
        content: Rect,
        kind: AgentSettingsSurfaceKind,
        vertical_navigation: bool,
    ) -> Self {
        let close_target = Rect {
            origin: Point2D::new(
                panel.origin.x + panel.size.x - TOUCH_CLOSE_TARGET - 8.0,
                panel.origin.y + 6.0,
            ),
            size: Point2D::new(TOUCH_CLOSE_TARGET, TOUCH_CLOSE_TARGET),
        };
        let close_icon = Rect {
            origin: Point2D::new(
                close_target.origin.x + (close_target.size.x - TOUCH_CLOSE_ICON) / 2.0,
                close_target.origin.y + (close_target.size.y - TOUCH_CLOSE_ICON) / 2.0,
            ),
            size: Point2D::new(TOUCH_CLOSE_ICON, TOUCH_CLOSE_ICON),
        };
        Self {
            surface: panel,
            header,
            navigation,
            content,
            close_target,
            close_icon,
            kind,
            vertical_navigation,
            navigation_columns: 1,
        }
    }

    pub fn nav_item_rect(&self, index: usize, count: usize) -> Rect {
        if self.kind == AgentSettingsSurfaceKind::Desktop {
            return nav_item_rect(self.surface, index, count);
        }
        if self.vertical_navigation {
            return Rect {
                origin: Point2D::new(
                    self.navigation.origin.x,
                    self.navigation.origin.y + index as f32 * (EXPANDED_TAB_H + TOUCH_TAB_GAP),
                ),
                size: Point2D::new(self.navigation.size.x, EXPANDED_TAB_H),
            };
        }
        let count = count.max(1);
        let columns = self.navigation_columns.min(count).max(1);
        let column = index % columns;
        let row = index / columns;
        let width = ((self.navigation.size.x - TOUCH_TAB_GAP * (columns - 1) as f32)
            / columns as f32)
            .max(0.0);
        Rect {
            origin: Point2D::new(
                self.navigation.origin.x + column as f32 * (width + TOUCH_TAB_GAP),
                self.navigation.origin.y + row as f32 * (TOUCH_TAB_H + TOUCH_TAB_GAP),
            ),
            size: Point2D::new(width, TOUCH_TAB_H),
        }
    }

    pub fn content_paint_clip(&self) -> Rect {
        if self.kind == AgentSettingsSurfaceKind::Desktop {
            return content_paint_clip_rect(self.surface);
        }
        Rect {
            origin: Point2D::new(self.content.origin.x - 1.0, self.content.origin.y),
            size: Point2D::new(self.content.size.x + 2.0, self.content.size.y),
        }
    }
}

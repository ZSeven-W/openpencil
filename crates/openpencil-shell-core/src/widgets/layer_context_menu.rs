//! Right-click context menu for LayerPanel rows. Two row sets:
//! layer rows (复制 / 删除 / 创建组件 / 切换锁定 / 切换可见性) and
//! page rows (重命名 / 复制 / 上移 / 下移 / 删除).

use crate::document::{Document, LayerContextMenuState, LayerContextTarget};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

pub const LAYER_CONTEXT_MENU_WIDTH: f32 = 168.0;
const ROW_HEIGHT: f32 = 32.0;
const PAD_Y: f32 = 6.0;
const ROW_FONT: f32 = 13.0;
const ICON_SIZE: f32 = 14.0;

/// All actions either menu can emit. Host dispatches each on click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerContextAction {
    // Layer-row actions
    RenameLayer,
    Duplicate,
    Delete,
    CreateComponent,
    ToggleLock,
    ToggleVisibility,
    // Page-row actions
    RenamePage,
    DuplicatePage,
    MovePageUp,
    MovePageDown,
    DeletePage,
}

#[derive(Debug, Clone, Copy)]
struct Row {
    icon: Icon,
    action: LayerContextAction,
    /// English fallback label. i18n keys land later — for now the
    /// menu uses these literals (matches TS layer-menu text).
    label: &'static str,
    /// True for the destructive `Delete` rows — paints the label
    /// in a red accent so the user sees the irreversible action.
    destructive: bool,
}

const LAYER_ROWS: &[Row] = &[
    Row {
        icon: Icon::Pencil,
        action: LayerContextAction::RenameLayer,
        label: "重命名",
        destructive: false,
    },
    Row {
        icon: Icon::Copy,
        action: LayerContextAction::Duplicate,
        label: "复制",
        destructive: false,
    },
    Row {
        icon: Icon::Component,
        action: LayerContextAction::CreateComponent,
        label: "创建组件",
        destructive: false,
    },
    Row {
        icon: Icon::Lock,
        action: LayerContextAction::ToggleLock,
        label: "切换锁定",
        destructive: false,
    },
    Row {
        icon: Icon::EyeOff,
        action: LayerContextAction::ToggleVisibility,
        label: "切换可见性",
        destructive: false,
    },
    Row {
        icon: Icon::Trash,
        action: LayerContextAction::Delete,
        label: "删除",
        destructive: true,
    },
];

const PAGE_ROWS: &[Row] = &[
    Row {
        icon: Icon::Pencil,
        action: LayerContextAction::RenamePage,
        label: "重命名",
        destructive: false,
    },
    Row {
        icon: Icon::Copy,
        action: LayerContextAction::DuplicatePage,
        label: "复制",
        destructive: false,
    },
    Row {
        icon: Icon::ArrowUp,
        action: LayerContextAction::MovePageUp,
        label: "上移",
        destructive: false,
    },
    Row {
        icon: Icon::ArrowDown,
        action: LayerContextAction::MovePageDown,
        label: "下移",
        destructive: false,
    },
    Row {
        icon: Icon::Trash,
        action: LayerContextAction::DeletePage,
        label: "删除",
        destructive: true,
    },
];

pub struct LayerContextMenu {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: LayerContextMenuState,
    rows: &'static [Row],
    /// Index of the currently-hovered row (None when the cursor is
    /// outside the menu). Host updates via `hovered_row_at` on
    /// every cursor-move while the menu is open.
    pub hovered_row: Option<usize>,
}

impl LayerContextMenu {
    pub fn for_state(doc: &Document, state: LayerContextMenuState) -> Self {
        let rows = match &state.target {
            LayerContextTarget::Layer(_) => LAYER_ROWS,
            LayerContextTarget::Page(_) => PAGE_ROWS,
        };
        let hovered_row = state.hovered_row.map(|i| i as usize);
        Self {
            id: WidgetId::new(3000),
            theme: doc.theme(),
            state,
            rows,
            hovered_row,
        }
    }

    /// Row index the cursor is currently over. Returns None when
    /// outside the menu's interior. Same geometry as `hit_test`,
    /// kept separate so the host can wire it from `cursor_move`
    /// without committing an action.
    pub fn hovered_row_at(&self, point: Point2D) -> Option<usize> {
        let rect = self.rect();
        if point.x < rect.origin.x
            || point.x > rect.origin.x + rect.size.x
            || point.y < rect.origin.y + PAD_Y
            || point.y > rect.origin.y + rect.size.y - PAD_Y
        {
            return None;
        }
        let local = point.y - rect.origin.y - PAD_Y;
        let idx = (local / ROW_HEIGHT) as usize;
        if idx < self.rows.len() {
            Some(idx)
        } else {
            None
        }
    }

    pub fn rect(&self) -> Rect {
        Rect {
            origin: Point2D::new(self.state.anchor_x, self.state.anchor_y),
            size: Point2D::new(
                LAYER_CONTEXT_MENU_WIDTH,
                PAD_Y * 2.0 + self.rows.len() as f32 * ROW_HEIGHT,
            ),
        }
    }

    /// Hit-test the menu. None when the cursor's outside the menu
    /// (caller closes the menu silently).
    pub fn hit_test(&self, point: Point2D) -> Option<LayerContextAction> {
        let rect = self.rect();
        if point.x < rect.origin.x
            || point.x > rect.origin.x + rect.size.x
            || point.y < rect.origin.y + PAD_Y
            || point.y > rect.origin.y + rect.size.y - PAD_Y
        {
            return None;
        }
        let local = point.y - rect.origin.y - PAD_Y;
        let idx = (local / ROW_HEIGHT) as usize;
        self.rows.get(idx).map(|r| r.action)
    }
}

impl Widget for LayerContextMenu {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox { rect: self.rect() }
    }
    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 8.0, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, 8.0, self.theme.border, 1.0);
        let destructive_color = Color {
            r: 0.94,
            g: 0.32,
            b: 0.32,
            a: 1.0,
        };
        let mut y = rect.origin.y + PAD_Y;
        for (i, row) in self.rows.iter().enumerate() {
            let row_rect = Rect {
                origin: Point2D::new(rect.origin.x, y),
                size: Point2D::new(rect.size.x, ROW_HEIGHT),
            };
            if self.hovered_row == Some(i) {
                let hover_rect = Rect {
                    origin: Point2D::new(rect.origin.x + 4.0, y + 2.0),
                    size: Point2D::new(rect.size.x - 8.0, ROW_HEIGHT - 4.0),
                };
                cx.backend
                    .fill_round_rect(hover_rect, 4.0, self.theme.row_selected);
            }
            let fg = if row.destructive {
                destructive_color
            } else {
                self.theme.foreground
            };
            draw_icon(
                cx.backend,
                row.icon,
                Point2D::new(row_rect.origin.x + 14.0, row_rect.origin.y + 9.0),
                ICON_SIZE,
                fg,
                1.4,
            );
            let label = TextLayout::single_run(
                row.label,
                "system-ui",
                ROW_FONT,
                to_jian_color(fg),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &label,
                Point2D::new(row_rect.origin.x + 40.0, row_rect.origin.y + 21.0),
            );
            y += ROW_HEIGHT;
        }
    }
    fn access_node(&self) -> accesskit::Node {
        let mut n = accesskit::Node::new(accesskit::Role::Menu);
        n.set_label("Layer context menu");
        n
    }
}

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

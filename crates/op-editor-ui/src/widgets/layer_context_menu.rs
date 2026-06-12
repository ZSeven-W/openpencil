//! Right-click context menu for LayerPanel rows. Two row sets:
//! layer rows (Duplicate / Delete / Create component / Toggle lock /
//! Toggle visibility) and page rows (Rename / Duplicate / Move up /
//! Move down / Delete).
//!
//! Deliberate divergence from the TS menu
//! (`apps/web/src/components/panels/layer-context-menu.tsx:30-78`):
//! a Rename row is added for layer rows (TS renames via row
//! double-click; the native shell routes both through the same
//! `start_rename_layer` draft). The four boolean-op rows mirror the
//! TS `requireBoolean` gating (`canBooleanOp` — ≥ 2 selected, all
//! boolean-able shapes) and dispatch to the host's skia-backed
//! `apply_boolean_op`.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::{LayerContextMenuState, LayerContextTarget};
use op_editor_core::EditorState;

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
    GroupSelection,
    /// Boolean path ops — TS `boolean-union` etc. rows
    /// (`layer-context-menu.tsx:35-57`), shown only when
    /// `canBooleanOp` passes.
    BooleanUnion,
    BooleanSubtract,
    BooleanIntersect,
    BooleanExclude,
    CreateComponent,
    /// Shown instead of CreateComponent when the target is a reusable
    /// component — sheds the `reusable` flag (TS detach case 1).
    DetachComponent,
    /// Shown instead of CreateComponent when the target is a `Ref`
    /// instance — materializes it into an independent subtree.
    DetachInstance,
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
    /// `op-i18n` key for the row label — resolved against the active
    /// locale at paint time.
    label_key: &'static str,
    /// True for the destructive `Delete` rows — paints the label
    /// in a red accent so the user sees the irreversible action.
    destructive: bool,
}

const LAYER_ROWS: &[Row] = &[
    Row {
        icon: Icon::Pencil,
        action: LayerContextAction::RenameLayer,
        label_key: "common.rename",
        destructive: false,
    },
    Row {
        icon: Icon::Copy,
        action: LayerContextAction::Duplicate,
        label_key: "common.duplicate",
        destructive: false,
    },
    Row {
        icon: Icon::Component,
        action: LayerContextAction::GroupSelection,
        label_key: "layerMenu.groupSelection",
        destructive: false,
    },
    Row {
        icon: Icon::SquaresUnite,
        action: LayerContextAction::BooleanUnion,
        label_key: "layerMenu.booleanUnion",
        destructive: false,
    },
    Row {
        icon: Icon::SquaresSubtract,
        action: LayerContextAction::BooleanSubtract,
        label_key: "layerMenu.booleanSubtract",
        destructive: false,
    },
    Row {
        icon: Icon::SquaresIntersect,
        action: LayerContextAction::BooleanIntersect,
        label_key: "layerMenu.booleanIntersect",
        destructive: false,
    },
    Row {
        icon: Icon::SquaresExclude,
        action: LayerContextAction::BooleanExclude,
        label_key: "layerMenu.booleanExclude",
        destructive: false,
    },
    Row {
        icon: Icon::Component,
        action: LayerContextAction::CreateComponent,
        label_key: "layerMenu.createComponent",
        destructive: false,
    },
    Row {
        icon: Icon::Lock,
        action: LayerContextAction::ToggleLock,
        label_key: "layerMenu.toggleLock",
        destructive: false,
    },
    Row {
        icon: Icon::EyeOff,
        action: LayerContextAction::ToggleVisibility,
        label_key: "layerMenu.toggleVisibility",
        destructive: false,
    },
    Row {
        icon: Icon::Trash,
        action: LayerContextAction::Delete,
        label_key: "common.delete",
        destructive: true,
    },
];

const PAGE_ROWS: &[Row] = &[
    Row {
        icon: Icon::Pencil,
        action: LayerContextAction::RenamePage,
        label_key: "common.rename",
        destructive: false,
    },
    Row {
        icon: Icon::Copy,
        action: LayerContextAction::DuplicatePage,
        label_key: "common.duplicate",
        destructive: false,
    },
    Row {
        icon: Icon::ArrowUp,
        action: LayerContextAction::MovePageUp,
        label_key: "pages.moveUp",
        destructive: false,
    },
    Row {
        icon: Icon::ArrowDown,
        action: LayerContextAction::MovePageDown,
        label_key: "pages.moveDown",
        destructive: false,
    },
    Row {
        icon: Icon::Trash,
        action: LayerContextAction::DeletePage,
        label_key: "common.delete",
        destructive: true,
    },
];

pub struct LayerContextMenu {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: LayerContextMenuState,
    rows: Vec<Row>,
    /// Active UI locale — drives row-label translation in `paint`.
    locale: op_editor_core::Locale,
    /// Index of the currently-hovered row (None when the cursor is
    /// outside the menu). Host updates via `hovered_row_at` on
    /// every cursor-move while the menu is open.
    pub hovered_row: Option<usize>,
}

impl LayerContextMenu {
    pub fn for_state(state: &EditorState, menu: LayerContextMenuState) -> Self {
        let boolean_ok = super::align_toolbar::can_boolean_op(state);
        let mut rows = match &menu.target {
            LayerContextTarget::Layer(_) => LAYER_ROWS
                .iter()
                .copied()
                .filter(|row| match row.action {
                    LayerContextAction::GroupSelection => state.selection_count() > 1,
                    // TS `requireBoolean` rows (`layer-context-menu.tsx:35-57`).
                    LayerContextAction::BooleanUnion
                    | LayerContextAction::BooleanSubtract
                    | LayerContextAction::BooleanIntersect
                    | LayerContextAction::BooleanExclude => boolean_ok,
                    _ => true,
                })
                .collect(),
            LayerContextTarget::Page(_) => PAGE_ROWS.to_vec(),
        };
        // Stateful component slot — a reusable component / Ref
        // instance swaps "Create component" for the matching detach
        // row (TS layer menu parity, gated on the actual node).
        if let LayerContextTarget::Layer(id) = &menu.target {
            use jian_ops_schema::node::PenNode;
            let detach = match op_editor_core::walkers::find_node(state.active_children(), id) {
                Some(PenNode::Frame(f)) if f.reusable == Some(true) => Some(Row {
                    icon: Icon::Diamond,
                    action: LayerContextAction::DetachComponent,
                    label_key: "layerMenu.detachComponent",
                    destructive: false,
                }),
                Some(PenNode::Ref(_)) => Some(Row {
                    icon: Icon::Diamond,
                    action: LayerContextAction::DetachInstance,
                    label_key: "layerMenu.detachInstance",
                    destructive: false,
                }),
                _ => None,
            };
            if let Some(detach) = detach {
                if let Some(pos) = rows
                    .iter()
                    .position(|r| r.action == LayerContextAction::CreateComponent)
                {
                    rows[pos] = detach;
                }
            }
        }
        let hovered_row = menu.hovered_row.map(|i| i as usize);
        Self {
            id: WidgetId::new(3000),
            theme: theme_for(&state.editor_ui),
            state: menu,
            rows,
            locale: state.editor_ui.locale,
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
                op_i18n::translate(self.locale, row.label_key),
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
        n.set_label(op_i18n::translate(self.locale, "a11y.layerContextMenu"));
        n
    }
}

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

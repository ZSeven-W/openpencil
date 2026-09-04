//! Touch chrome shared by phone and tablet size classes.
//!
//! Sizes follow the mobile handoff spec: 52pt app bar, 56pt dock, ≥44pt
//! hit targets, 20pt icons, semantic theme colors only.

use super::panel_control_metrics::mix;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::text_metrics;
use crate::widgets::toolbar::icon_for_shape;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, Theme};
use jian_widgets::centered_text_baseline_y;
use op_editor_core::tool::Tool;
use op_editor_core::EditorState;

pub use super::mobile_more_panel::{
    more_entry_rect, more_hit_test, more_panel_rect, paint_more_panel, MobileMoreEntry,
};

/// Compact app bar height (52pt; the shell's safe-area top is consumed
/// by the engine as a canvas translate).
pub const APP_BAR_HEIGHT: f32 = 52.0;
/// Bottom-sheet top radius.
pub const SHEET_RADIUS: f32 = 20.0;
/// Shared title / close band at the top of touch sheets.
pub const SHEET_HEADER_HEIGHT: f32 = 56.0;
/// Minimum touch target.
pub const TOUCH_TARGET: f32 = 44.0;

/// Keep visual glyphs independent from their generous touch targets.
/// Stretching a 24×24 SVG into a 44×44 target produced the giant close
/// and page arrows visible in the rejected iPad build.
pub fn centered_icon_rect(target: Rect, icon_size: f32) -> Rect {
    let size = icon_size.min(target.size.x).min(target.size.y).max(0.0);
    Rect {
        origin: Point2D::new(
            target.origin.x + (target.size.x - size) / 2.0,
            target.origin.y + (target.size.y - size) / 2.0,
        ),
        size: Point2D::new(size, size),
    }
}

/// The app bar's elevated surface. The top safe-area band paints this
/// same color so bar and band read as one surface — single-sourced here
/// so the two paint sites and the FFI band test cannot drift apart.
pub fn app_bar_surface(theme: Theme) -> Color {
    mix(theme.background, theme.card, 0.42)
}

/// The compact (phone) dock's surface. The bottom safe-area band extends
/// it behind the system gesture area; see [`app_bar_surface`] for why the
/// blend lives in one place.
pub fn dock_surface(theme: Theme) -> Color {
    mix(theme.background, theme.card, 0.48)
}

/// Paint a Lucide glyph inside a larger touch target without changing
/// its canonical 24×24 coordinate system. Every subpath must share the
/// same transform; fitting each subpath independently deforms compound
/// icons such as Settings, Braces and Download.
/// Surface colors the platform safe-area bands should carry so the touch
/// chrome reads as one continuous surface: the top band matches the app
/// bar, and (phones only — tablets float their dock) the bottom band
/// matches the edge-to-edge dock. `None` keeps the theme background.
pub fn safe_area_band_colors(state: &EditorState) -> (Option<Color>, Option<Color>) {
    if !state.editor_ui.touch_chrome() {
        return (None, None);
    }
    let theme = crate::widgets::editor_state_ext::theme_for(&state.editor_ui);
    let top = Some(app_bar_surface(theme));
    // Mirrors the dock's paint gating: a modal sheet or the variables panel
    // hides the dock, so the band returns to the plain background.
    let bottom = if state.editor_ui.compact_layout()
        && state.editor_ui.mobile_sheet.is_none()
        && !state.editor_ui.variables_panel_open
    {
        Some(dock_surface(theme))
    } else {
        None
    };
    (top, bottom)
}

pub fn paint_touch_icon(
    cx: &mut PaintCx<'_>,
    target: Rect,
    icon: Icon,
    icon_size: f32,
    color: Color,
) {
    let rect = centered_icon_rect(target, icon_size);
    draw_icon(cx.backend, icon, rect.origin, rect.size.x, color, 1.75);
}

// ---------------------------------------------------------------------
// App bar
// ---------------------------------------------------------------------

/// App-bar hit results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileAppBarHit {
    /// Layers sheet.
    Layers,
    /// Frame all active-page content in the canvas viewport.
    Fit,
    /// Undo.
    Undo,
    /// Redo.
    Redo,
    /// More (overflow) sheet.
    Overflow,
}

/// The compact app bar: layers + title on the left, fit / undo / redo /
/// overflow on the right.
pub struct MobileAppBar {
    pub title: String,
    pub theme: Theme,
}

impl MobileAppBar {
    pub fn for_editor(state: &EditorState) -> Self {
        let title = state
            .editor_ui
            .file_name_display
            .clone()
            .unwrap_or_else(|| {
                super::editor_state_ext::translate(&state.editor_ui, "common.untitled").to_string()
            });
        Self {
            title,
            theme: super::editor_state_ext::theme_for(&state.editor_ui),
        }
    }

    fn button_rect(rect: Rect, from_right: usize) -> Rect {
        let x = rect.origin.x + rect.size.x - 6.0 - TOUCH_TARGET * (from_right as f32 + 1.0);
        Rect {
            origin: Point2D::new(x, rect.origin.y + (rect.size.y - TOUCH_TARGET) / 2.0),
            size: Point2D::new(TOUCH_TARGET, TOUCH_TARGET),
        }
    }

    pub fn layers_rect(rect: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                rect.origin.x + 6.0,
                rect.origin.y + (rect.size.y - TOUCH_TARGET) / 2.0,
            ),
            size: Point2D::new(TOUCH_TARGET, TOUCH_TARGET),
        }
    }

    pub fn undo_rect(rect: Rect) -> Rect {
        Self::button_rect(rect, 2)
    }

    pub fn fit_rect(rect: Rect) -> Rect {
        Self::button_rect(rect, 3)
    }

    pub fn redo_rect(rect: Rect) -> Rect {
        Self::button_rect(rect, 1)
    }

    pub fn overflow_rect(rect: Rect) -> Rect {
        Self::button_rect(rect, 0)
    }

    fn paint_icon(&self, cx: &mut PaintCx<'_>, rect: Rect, icon: Icon, pressed: bool) {
        if pressed {
            cx.backend.fill_round_rect(
                rect,
                12.0,
                mix(self.theme.card, self.theme.foreground, 0.12),
            );
        }
        let color = if pressed {
            self.theme.foreground
        } else {
            self.theme.foreground.with_alpha(0.82)
        };
        paint_touch_icon(cx, rect, icon, 20.0, color);
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        // Elevated surface + bottom hairline.
        cx.backend.fill_rect(rect, app_bar_surface(self.theme));
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, rect.origin.y + rect.size.y - 1.0),
                size: Point2D::new(rect.size.x, 1.0),
            },
            self.theme.border,
        );

        // Left: layers button.
        self.paint_icon(
            cx,
            Self::layers_rect(rect),
            Icon::from_name("layers").unwrap_or(Icon::Layers),
            false,
        );

        // Center: truncated document title (17pt).
        let layers = Self::layers_rect(rect);
        let title_x = layers.origin.x + TOUCH_TARGET + 4.0;
        let title_right = Self::fit_rect(rect).origin.x - 4.0;
        let title_area = Rect {
            origin: Point2D::new(title_x, rect.origin.y),
            size: Point2D::new((title_right - title_x).max(0.0), rect.size.y),
        };
        let label = crate::TextLayout::single_run(
            &self.title,
            "system-ui",
            17.0,
            self.theme.foreground.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        let label_w = text_metrics::measure_chrome(cx.backend, &self.title, 17.0);
        let text_y = centered_text_baseline_y(title_area, 17.0);
        if label_w <= title_area.size.x {
            cx.backend
                .draw_text(&label, Point2D::new(title_area.origin.x, text_y));
        } else {
            cx.backend.save();
            cx.backend.clip_rect(Rect {
                origin: Point2D::new(title_area.origin.x, rect.origin.y),
                size: Point2D::new(title_area.size.x, rect.size.y),
            });
            cx.backend
                .draw_text(&label, Point2D::new(title_area.origin.x, text_y));
            cx.backend.restore();
        }

        // Right: fit / undo / redo / overflow.
        self.paint_icon(cx, Self::fit_rect(rect), Icon::Focus, false);
        self.paint_icon(
            cx,
            Self::undo_rect(rect),
            Icon::from_name("undo-2").unwrap_or(Icon::Undo),
            false,
        );
        self.paint_icon(
            cx,
            Self::redo_rect(rect),
            Icon::from_name("redo-2").unwrap_or(Icon::Redo),
            false,
        );
        self.paint_icon(
            cx,
            Self::overflow_rect(rect),
            Icon::from_name("ellipsis").unwrap_or(Icon::MoreHorizontal),
            false,
        );
    }

    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<MobileAppBarHit> {
        if Self::layers_rect(rect).contains(point) {
            return Some(MobileAppBarHit::Layers);
        }
        if Self::fit_rect(rect).contains(point) {
            return Some(MobileAppBarHit::Fit);
        }
        if Self::undo_rect(rect).contains(point) {
            return Some(MobileAppBarHit::Undo);
        }
        if Self::redo_rect(rect).contains(point) {
            return Some(MobileAppBarHit::Redo);
        }
        if Self::overflow_rect(rect).contains(point) {
            return Some(MobileAppBarHit::Overflow);
        }
        None
    }
}

// ---------------------------------------------------------------------
// Bottom tool dock
// ---------------------------------------------------------------------

/// Dock hit results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileDockHit {
    /// Tool selection (Select / Pen / Text).
    Tool(Tool),
    /// Shape slot — opens the touch shape picker.
    ShapeSlot,
    /// More sheet.
    More,
}

/// The bottom tool dock: Select / Shape / Pen / Text / More.
pub struct MobileDock {
    pub active: Tool,
    pub shape_tool: Tool,
    pub shape_picker_open: bool,
    pub theme: Theme,
    pub compact: bool,
}

impl MobileDock {
    pub fn for_editor(state: &EditorState) -> Self {
        Self {
            active: state.tool,
            shape_tool: state.editor_ui.shape_tool,
            shape_picker_open: state.editor_ui.shape_picker.open,
            theme: super::editor_state_ext::theme_for(&state.editor_ui),
            compact: state.editor_ui.compact_layout(),
        }
    }

    pub fn slot_count(&self) -> usize {
        if self.compact {
            5
        } else {
            4
        }
    }

    pub fn slot_rect(rect: Rect, index: usize, slot_count: usize) -> Rect {
        let slot_w = rect.size.x / slot_count.max(1) as f32;
        Rect {
            origin: Point2D::new(rect.origin.x + slot_w * index as f32, rect.origin.y),
            size: Point2D::new(slot_w, rect.size.y),
        }
    }

    fn slot_icon(&self, index: usize) -> Icon {
        match index {
            0 => Icon::from_name("mouse-pointer-2").unwrap_or(Icon::Cursor),
            1 => icon_for_shape(self.shape_tool),
            2 => Icon::from_name("pen-tool").unwrap_or(Icon::PenTool),
            3 => Icon::from_name("type").unwrap_or(Icon::Type),
            _ => Icon::from_name("ellipsis").unwrap_or(Icon::MoreHorizontal),
        }
    }

    fn slot_active(&self, index: usize) -> bool {
        match index {
            0 => self.active == Tool::Select,
            1 => matches!(
                self.active,
                Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line
            ),
            2 => self.active == Tool::Pen,
            3 => self.active == Tool::Text,
            _ => false,
        }
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        if self.compact {
            cx.backend.fill_rect(rect, dock_surface(self.theme));
            cx.backend.fill_rect(
                Rect {
                    origin: rect.origin,
                    size: Point2D::new(rect.size.x, 1.0),
                },
                self.theme.border,
            );
        } else {
            cx.backend
                .fill_round_rect(rect, 20.0, mix(self.theme.popover, Color::BLACK, 0.08));
            cx.backend
                .stroke_round_rect(rect, 20.0, self.theme.border, 1.0);
        }

        let slot_count = self.slot_count();
        for index in 0..slot_count {
            let slot = Self::slot_rect(rect, index, slot_count);
            let active = self.slot_active(index);
            let open = index == 1 && self.shape_picker_open;
            let emphasized = active || open;
            let icon = self.slot_icon(index);
            let active_rect = centered_icon_rect(slot, 40.0);
            if emphasized {
                cx.backend
                    .fill_round_rect(active_rect, 12.0, self.theme.row_selected_primary);
            }
            let color = if emphasized {
                self.theme.primary
            } else {
                self.theme.foreground.with_alpha(0.82)
            };
            if index == 1 {
                // Keep the shape glyph and its dropdown cue together inside
                // the 40pt feedback wash. The whole slot remains one touch
                // target; the chevron is a visual affordance, not a second
                // control.
                let (icon_rect, chevron_rect) = shape_slot_glyph_rects(slot);
                draw_icon(
                    cx.backend,
                    icon,
                    icon_rect.origin,
                    icon_rect.size.x,
                    color,
                    1.75,
                );
                draw_icon(
                    cx.backend,
                    if open {
                        Icon::ChevronUp
                    } else {
                        Icon::ChevronDown
                    },
                    chevron_rect.origin,
                    chevron_rect.size.x,
                    color,
                    1.6,
                );
            } else {
                paint_touch_icon(cx, slot, icon, 22.0, color);
            }
        }
    }

    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<MobileDockHit> {
        if !rect.contains(point) {
            return None;
        }
        let slot_count = self.slot_count();
        let index = (((point.x - rect.origin.x) / (rect.size.x / slot_count as f32)).floor()
            as usize)
            .min(slot_count - 1);
        match index {
            0 => Some(MobileDockHit::Tool(Tool::Select)),
            1 => Some(MobileDockHit::ShapeSlot),
            2 => Some(MobileDockHit::Tool(Tool::Pen)),
            3 => Some(MobileDockHit::Tool(Tool::Text)),
            _ => Some(MobileDockHit::More),
        }
    }
}

/// Lay out the shape glyph and its dropdown cue as one centered compound
/// mark. Both remain inside the dock's 40pt active/pressed feedback wash.
fn shape_slot_glyph_rects(slot: Rect) -> (Rect, Rect) {
    const ICON_SIZE: f32 = 20.0;
    const CHEVRON_SIZE: f32 = 9.0;
    const GAP: f32 = 3.0;
    let group_width = ICON_SIZE + GAP + CHEVRON_SIZE;
    let group_x = slot.origin.x + (slot.size.x - group_width) / 2.0;
    let center_y = slot.origin.y + slot.size.y / 2.0;
    (
        Rect {
            origin: Point2D::new(group_x, center_y - ICON_SIZE / 2.0),
            size: Point2D::new(ICON_SIZE, ICON_SIZE),
        },
        Rect {
            origin: Point2D::new(group_x + ICON_SIZE + GAP, center_y - CHEVRON_SIZE / 2.0),
            size: Point2D::new(CHEVRON_SIZE, CHEVRON_SIZE),
        },
    )
}

// ---------------------------------------------------------------------
// Bottom-sheet chrome
// ---------------------------------------------------------------------

/// Compact full-width bottom-sheet geometry.
pub fn sheet_rect(viewport_w: f32, viewport_h: f32, height_ratio: f32) -> Rect {
    let max_h = (viewport_h - 8.0).max(0.0);
    let min_h = 280.0_f32.min(max_h);
    let h = (viewport_h * height_ratio).clamp(min_h, max_h);
    Rect {
        origin: Point2D::new(0.0, viewport_h - h),
        size: Point2D::new(viewport_w, h),
    }
}

/// The sheet's drag handle (a short rounded bar, centered on the top
/// edge).
pub fn sheet_handle_rect(sheet: Rect) -> Rect {
    const W: f32 = 36.0;
    const H: f32 = 4.0;
    Rect {
        origin: Point2D::new(
            sheet.origin.x + (sheet.size.x - W) / 2.0,
            sheet.origin.y + 8.0,
        ),
        size: Point2D::new(W, H),
    }
}

/// The sheet's 44×44 close target (top-right).
pub fn sheet_close_rect(sheet: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            sheet.origin.x + sheet.size.x - TOUCH_TARGET - 10.0,
            sheet.origin.y + 6.0,
        ),
        size: Point2D::new(TOUCH_TARGET, TOUCH_TARGET),
    }
}

/// The visible content below a sheet's title / close band.
///
/// Hosts use this exact rect for paint, hit-testing and scrolling, so
/// controls cannot remain interactive behind the shared header.
pub fn sheet_content_rect(sheet: Rect) -> Rect {
    let header = SHEET_HEADER_HEIGHT.min(sheet.size.y);
    Rect {
        origin: Point2D::new(sheet.origin.x, sheet.origin.y + header),
        size: Point2D::new(sheet.size.x, (sheet.size.y - header).max(0.0)),
    }
}

/// Paint the shared sheet chrome: rounded-top elevated surface, drag
/// handle, title, and close button.
pub fn paint_sheet_chrome(cx: &mut PaintCx<'_>, theme: &Theme, sheet: Rect, title: &str) {
    cx.backend.fill_round_rect_per_corner(
        sheet,
        [SHEET_RADIUS, SHEET_RADIUS, 0.0, 0.0],
        theme.card,
    );
    paint_sheet_header(cx, theme, sheet, title, true);
}

/// Paint only the header affordances over content that already owns the
/// sheet surface. `show_handle` is true on phone bottom sheets and false on
/// tablet side panels.
pub fn paint_sheet_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    sheet: Rect,
    title: &str,
    show_handle: bool,
) {
    if show_handle {
        cx.backend
            .fill_round_rect(sheet_handle_rect(sheet), 2.0, theme.border);
    }
    let label = crate::TextLayout::single_run(
        title,
        "system-ui",
        17.0,
        theme.foreground.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let header = Rect {
        origin: sheet.origin,
        size: Point2D::new(sheet.size.x, SHEET_HEADER_HEIGHT.min(sheet.size.y)),
    };
    cx.backend.draw_text(
        &label,
        Point2D::new(
            sheet.origin.x + 18.0,
            centered_text_baseline_y(header, 17.0),
        ),
    );
    let close = sheet_close_rect(sheet);
    let icon = Icon::from_name("x").unwrap_or(Icon::Close);
    paint_touch_icon(cx, close, icon, 18.0, theme.muted_foreground);
}

/// Selected-node contextual actions. Properties gets the flexible leading
/// segment; Delete stays a distinct 44pt destructive target.
pub fn selection_actions_rect_for(state: &EditorState, viewport_w: f32, viewport_h: f32) -> Rect {
    let dock = super::host_canvas_geometry::touch_dock_rect(state, viewport_w, viewport_h);
    let canvas = super::host_canvas_geometry::canvas_rect(state, viewport_w, viewport_h);
    let width = 152.0_f32.min((viewport_w - 24.0).max(0.0));
    let inline_y = dock.origin.y + (dock.size.y - TOUCH_TARGET) / 2.0;
    let inline_x = canvas.origin.x + canvas.size.x - width - 12.0;
    let clears_dock = inline_x >= dock.origin.x + dock.size.x + 12.0;
    let stacked_y = dock.origin.y
        - 12.0
        - TOUCH_TARGET
        - if state.editor_ui.compact_layout() {
            12.0 + TOUCH_TARGET
        } else {
            0.0
        };
    Rect {
        origin: Point2D::new(
            inline_x.max(12.0),
            (if clears_dock { inline_y } else { stacked_y })
                .max(super::host_canvas_geometry::touch_app_bar_height(state)),
        ),
        size: Point2D::new(width, TOUCH_TARGET),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionActionHit {
    Properties,
    Delete,
}

pub fn selection_action_hit(rect: Rect, point: Point2D) -> Option<SelectionActionHit> {
    if !rect.contains(point) {
        return None;
    }
    if point.x >= rect.origin.x + rect.size.x - TOUCH_TARGET {
        Some(SelectionActionHit::Delete)
    } else {
        Some(SelectionActionHit::Properties)
    }
}

pub fn paint_selection_actions(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, label: &str) {
    cx.backend
        .fill_round_rect(rect, rect.size.y / 2.0, mix(theme.card, Color::BLACK, 0.25));
    cx.backend
        .stroke_round_rect(rect, rect.size.y / 2.0, theme.border, 1.0);
    let delete = Rect {
        origin: Point2D::new(rect.origin.x + rect.size.x - TOUCH_TARGET, rect.origin.y),
        size: Point2D::new(TOUCH_TARGET, rect.size.y),
    };
    cx.backend.stroke_line(
        Point2D::new(delete.origin.x, rect.origin.y + 10.0),
        Point2D::new(delete.origin.x, rect.origin.y + rect.size.y - 10.0),
        theme.border,
        1.0,
    );
    let property_target = Rect {
        origin: rect.origin,
        size: Point2D::new((rect.size.x - TOUCH_TARGET).max(0.0), rect.size.y),
    };
    let icon_target = Rect {
        origin: Point2D::new(property_target.origin.x + 8.0, property_target.origin.y),
        size: Point2D::new(28.0, property_target.size.y),
    };
    paint_touch_icon(
        cx,
        icon_target,
        Icon::from_name("settings-2").unwrap_or(Icon::Settings2),
        18.0,
        theme.foreground.with_alpha(0.82),
    );
    let label_layout = crate::TextLayout::single_run(
        label,
        "system-ui",
        15.0,
        theme.foreground.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(
            property_target.origin.x + 38.0,
            centered_text_baseline_y(property_target, 15.0),
        ),
    );
    paint_touch_icon(
        cx,
        delete,
        Icon::from_name("trash-2").unwrap_or(Icon::Trash),
        18.0,
        theme.destructive,
    );
}

// ---------------------------------------------------------------------
// Page switcher (shared themed pill)
// ---------------------------------------------------------------------

/// Page-pill hit results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagePillHit {
    Prev,
    Next,
}

/// Responsive page switcher placement. Phones keep it centered above the
/// dock; tablets pin it to the lower-left canvas edge so it never reads as
/// content inside a panel.
pub fn page_pill_rect_for(state: &EditorState, viewport_w: f32, viewport_h: f32) -> Rect {
    const TABLET_W: f32 = 120.0;
    const PHONE_W: f32 = 148.0;
    const H: f32 = 44.0;
    let width = if state.editor_ui.compact_layout() {
        PHONE_W
    } else {
        TABLET_W
    };
    let canvas = super::host_canvas_geometry::canvas_rect(state, viewport_w, viewport_h);
    let dock = super::host_canvas_geometry::touch_dock_rect(state, viewport_w, viewport_h);
    let inline_y = dock.origin.y + (dock.size.y - H) / 2.0;
    let inline_x = canvas.origin.x + 12.0;
    let clears_dock = inline_x + width + 12.0 <= dock.origin.x;
    let stacked_y = dock.origin.y - 12.0 - H;
    Rect {
        origin: Point2D::new(
            if state.editor_ui.compact_layout() {
                (viewport_w - width) / 2.0
            } else {
                inline_x
            },
            (if clears_dock { inline_y } else { stacked_y }).max(canvas.origin.y),
        ),
        size: Point2D::new(width, H),
    }
}

pub fn paint_page_pill(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    count: usize,
    active: usize,
) {
    if count <= 1 {
        return;
    }
    cx.backend
        .fill_round_rect(rect, rect.size.y / 2.0, mix(theme.card, Color::BLACK, 0.25));
    let prev = Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y),
        size: Point2D::new(44.0, rect.size.y),
    };
    let next = Rect {
        origin: Point2D::new(rect.origin.x + rect.size.x - 44.0, rect.origin.y),
        size: Point2D::new(44.0, rect.size.y),
    };
    let back = Icon::from_name("chevron-left").unwrap_or(Icon::ChevronLeft);
    let forward = Icon::from_name("chevron-right").unwrap_or(Icon::ChevronRight);
    let prev_color = if active == 0 {
        mix(theme.foreground, theme.card, 0.6)
    } else {
        theme.foreground
    };
    let next_color = if active + 1 >= count {
        mix(theme.foreground, theme.card, 0.6)
    } else {
        theme.foreground
    };
    paint_touch_icon(cx, prev, back, 16.0, prev_color);
    paint_touch_icon(cx, next, forward, 16.0, next_color);
    let label_text = format!("{} / {}", active + 1, count);
    let label = crate::TextLayout::single_run(
        &label_text,
        "system-ui",
        13.0,
        theme.foreground.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let label_x = text_metrics::centered_text_x(cx.backend, &label_text, 13.0, rect);
    cx.backend.draw_text(
        &label,
        Point2D::new(label_x, centered_text_baseline_y(rect, 13.0)),
    );
}

pub fn page_pill_hit(rect: Rect, point: Point2D) -> Option<PagePillHit> {
    if !rect.contains(point) {
        return None;
    }
    let prev = Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y),
        size: Point2D::new(44.0, rect.size.y),
    };
    let next = Rect {
        origin: Point2D::new(rect.origin.x + rect.size.x - 44.0, rect.origin.y),
        size: Point2D::new(44.0, rect.size.y),
    };
    if prev.contains(point) {
        Some(PagePillHit::Prev)
    } else if next.contains(point) {
        Some(PagePillHit::Next)
    } else {
        None
    }
}

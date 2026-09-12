//! The post-generation 成品视图 surface.
//!
//! Geometry, board-row fitting, entrance motion and hit-testing live
//! here; the immediate-mode paint pass is in `result_view_surface_paint.rs`.
//! Like `HomeSurface`, the widget is document-agnostic apart from the
//! board metadata it resolves up front (id / name / authored size) — it
//! never touches a renderer, so it stays wasm32-clean. The real board
//! rasters are blitted by the native host on top of the sheet-coloured
//! placeholder slots this widget paints.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::{EditorState, EditorUiState, ResultHit, ResultViewState};

pub use super::home_surface::HomePalette;

/// Left margin shared by the breadcrumb, title and stage.
pub const RESULT_MARGIN_X: f32 = 80.0;
/// The stage ends this far from the right viewport edge; the reserve
/// holds the 30 px stage↔panel gap plus the panel's 40 px right margin.
const STAGE_RIGHT_RESERVE: f32 = 360.0;
const STAGE_TOP: f32 = 170.0;
const STAGE_BOTTOM_GAP: f32 = 140.0;
const STAGE_RADIUS: f32 = 12.0;
const STAGE_PAD: f32 = 28.0;
/// Vertical slack under the boards for the caption row + breathing room.
const STAGE_RESERVE_H: f32 = 100.0;
const BOARD_GAP: f32 = 30.0;
pub const BOARD_RADIUS: f32 = 10.0;
pub(crate) const CAPTION_H: f32 = 20.0;
const CAPTION_GAP: f32 = 8.0;
/// A hovered board lifts by this much (prototype hover state).
pub(crate) const BOARD_HOVER_LIFT: f32 = 4.0;
const PANEL_W: f32 = 290.0;
const PANEL_RIGHT_MARGIN: f32 = 40.0;
pub(crate) const PANEL_RADIUS: f32 = 16.0;
pub(crate) const PANEL_PAD: f32 = 20.0;
const PANEL_TITLE_BASELINE: f32 = 38.0;
pub(crate) const HINT_LINE_H: f32 = 20.0;
/// CJK hint wrap width at 13 px inside the 250 px panel content box.
pub(crate) const HINT_CHARS_PER_LINE: usize = 19;
pub(crate) const BUTTON_H: f32 = 44.0;
pub(crate) const BUTTON_GAP: f32 = 10.0;
const BREADCRUMB_W: f32 = 80.0;
const BREADCRUMB_H: f32 = 22.0;
const BREADCRUMB_Y: f32 = 86.0;
pub(crate) const TITLE_BASELINE: f32 = 150.0;
/// Fallback board shape when a root's authored size is missing or
/// keyword-sized — the phone aspect Home's App family generates.
const DEFAULT_BOARD_SIZE: (f32, f32) = (375.0, 812.0);

/// The six right-panel actions, in paint and hit order.
pub const RESULT_BUTTON_HITS: [ResultHit; 6] = [
    ResultHit::EditThisScreen,
    ResultHit::PlayPrototype,
    ResultHit::ComponentsVariables,
    ResultHit::Restyle,
    ResultHit::Export,
    ResultHit::FullEdit,
];

/// Every rect the paint pass, the host blit and the tests need.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultLayout {
    pub breadcrumb_back: Rect,
    pub professional: Rect,
    pub stage: Rect,
    pub screens: Vec<Rect>,
    pub captions: Vec<Rect>,
    pub panel: Rect,
    pub buttons: [Rect; 6],
    pub footer: Rect,
}

/// One board the result view presents: id, caption label, authored size.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultBoard {
    pub id: String,
    pub name: String,
    pub size: (f32, f32),
}

impl ResultBoard {
    /// Resolve the result view's `root_ids` against the live document.
    ///
    /// Sizes come from the authored pixel fields (`PenNodeExt`); a board
    /// whose size is keyword-driven or missing falls back to the phone
    /// aspect rather than collapsing the row. Names fall back to the
    /// kind label so a caption is never blank.
    pub fn collect(state: &EditorState) -> Vec<Self> {
        let children = state.active_children();
        state
            .editor_ui
            .result_view
            .root_ids
            .iter()
            .map(|id| {
                let node = children
                    .iter()
                    .find(|node| PenNodeExt::id_str(*node) == id.as_str());
                let size = node
                    .and_then(|node| {
                        match (PenNodeExt::width_px(node), PenNodeExt::height_px(node)) {
                            (Some(w), Some(h)) if w > 0.0 && h > 0.0 => Some((w as f32, h as f32)),
                            _ => None,
                        }
                    })
                    .unwrap_or(DEFAULT_BOARD_SIZE);
                let name = node
                    .map(|node| PenNodeExt::base(node).name.clone().unwrap_or_default())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| "界面".to_string());
                Self {
                    id: id.clone(),
                    name,
                    size,
                }
            })
            .collect()
    }
}

/// ease-out-cubic — the settle curve the canvas layout transition uses.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// A board's entrance phase: `(rise_offset_y, alpha)`. Boards fade in
/// while rising 12 px, staggered 70 ms apart over 320 ms each.
pub fn board_enter(index: usize, shown_at_ms: u64, now_ms: u64) -> (f32, f32) {
    let start = shown_at_ms.saturating_add(op_editor_core::RESULT_ENTER_STAGGER_MS * index as u64);
    let elapsed = now_ms.saturating_sub(start);
    let t = (elapsed as f32 / op_editor_core::RESULT_ENTER_BOARD_MS as f32).clamp(0.0, 1.0);
    let eased = ease_out_cubic(t);
    ((1.0 - eased) * 12.0, eased)
}

/// The right panel's entrance phase: slides in from +16 px over 260 ms.
pub fn panel_enter(shown_at_ms: u64, now_ms: u64) -> f32 {
    let elapsed = now_ms.saturating_sub(shown_at_ms);
    let t = (elapsed as f32 / op_editor_core::RESULT_ENTER_PANEL_MS as f32).clamp(0.0, 1.0);
    (1.0 - ease_out_cubic(t)) * 16.0
}

pub struct ResultViewSurface<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: &'a ResultViewState,
    pub ui: &'a EditorUiState,
    pub boards: Vec<ResultBoard>,
    pub now_ms: u64,
}

impl<'a> ResultViewSurface<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<Self> {
        state.editor_ui.result_view.visible.then(|| Self {
            id: WidgetId::new(7650),
            theme: theme_for(&state.editor_ui),
            state: &state.editor_ui.result_view,
            ui: &state.editor_ui,
            boards: ResultBoard::collect(state),
            now_ms,
        })
    }

    /// Fit `board_sizes` into the stage left→right: equal heights, width
    /// by aspect, uniformly scaled down if the row would overflow.
    pub fn layout_for(
        viewport_width: f32,
        viewport_height: f32,
        board_sizes: &[(f32, f32)],
    ) -> ResultLayout {
        let width = viewport_width.max(1.0);
        let height = viewport_height.max(1.0);
        let professional = Rect::xywh((width - 270.0).max(RESULT_MARGIN_X), 22.0, 222.0, 28.0);
        let breadcrumb_back = Rect::xywh(RESULT_MARGIN_X, BREADCRUMB_Y, BREADCRUMB_W, BREADCRUMB_H);
        let stage = Rect::xywh(
            RESULT_MARGIN_X,
            STAGE_TOP,
            (width - RESULT_MARGIN_X - STAGE_RIGHT_RESERVE).max(160.0),
            (height - STAGE_TOP - STAGE_BOTTOM_GAP).max(160.0),
        );
        let panel = Rect::xywh(
            width - PANEL_RIGHT_MARGIN - PANEL_W,
            STAGE_TOP,
            PANEL_W,
            stage.size.y,
        );

        let count = board_sizes.len();
        let inner_w = (stage.size.x - STAGE_PAD * 2.0).max(1.0);
        let board_h = (stage.size.y - STAGE_RESERVE_H).max(60.0);
        // Fit left→right at one shared height; if the row would overflow,
        // shrink every board by the same factor (the 30 px gap is fixed,
        // only the boards scale) so equal heights survive the shrink.
        let gap_total = BOARD_GAP * count.saturating_sub(1) as f32;
        let widths: Vec<f32> = board_sizes
            .iter()
            .map(|&(w, h)| w * board_h / h.max(1.0))
            .collect();
        let widths_sum: f32 = widths.iter().sum::<f32>();
        let scale = ((inner_w - gap_total).max(1.0) / widths_sum.max(1.0)).min(1.0);
        let row_w = widths_sum * scale + gap_total;
        let content_h = board_h * scale + CAPTION_GAP + CAPTION_H;
        let mut x = stage.origin.x + (stage.size.x - row_w) / 2.0;
        let top = stage.origin.y + ((stage.size.y - content_h) / 2.0).max(0.0);
        let mut screens = Vec::with_capacity(count);
        let mut captions = Vec::with_capacity(count);
        for width in widths {
            let scaled = width * scale;
            screens.push(Rect::xywh(x, top, scaled, board_h * scale));
            captions.push(Rect::xywh(
                x,
                top + board_h * scale + CAPTION_GAP,
                scaled,
                CAPTION_H,
            ));
            x += scaled + BOARD_GAP;
        }

        let hint_lines = Self::hint_line_count();
        let buttons_top =
            panel.origin.y + PANEL_TITLE_BASELINE + HINT_LINE_H + hint_lines as f32 * HINT_LINE_H;
        let mut buttons = [Rect::ZERO; 6];
        for (index, button) in buttons.iter_mut().enumerate() {
            *button = Rect::xywh(
                panel.origin.x + PANEL_PAD,
                buttons_top + index as f32 * (BUTTON_H + BUTTON_GAP),
                panel.size.x - PANEL_PAD * 2.0,
                BUTTON_H,
            );
        }
        let footer = Rect::xywh(
            panel.origin.x + PANEL_PAD,
            panel.origin.y + panel.size.y - 34.0,
            panel.size.x - PANEL_PAD * 2.0,
            20.0,
        );
        ResultLayout {
            breadcrumb_back,
            professional,
            stage,
            screens,
            captions,
            panel,
            buttons,
            footer,
        }
    }

    pub fn layout(&self, viewport_width: f32, viewport_height: f32) -> ResultLayout {
        let sizes: Vec<(f32, f32)> = self.boards.iter().map(|board| board.size).collect();
        Self::layout_for(viewport_width, viewport_height, &sizes)
    }

    /// The rect board `index` actually paints (and the host blits) at
    /// `now_ms`: resting position plus hover lift plus entrance rise.
    pub fn screen_draw_rect(&self, layout: &ResultLayout, index: usize) -> Option<Rect> {
        let rect = layout.screens.get(index).copied()?;
        let mut out = rect;
        if self.state.hover == Some(ResultHit::Screen(index)) {
            out.origin.y -= BOARD_HOVER_LIFT;
        }
        out.origin.y += board_enter(index, self.state.shown_at_ms, self.now_ms).0;
        Some(out)
    }

    pub fn hit_test(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        point: Point2D,
    ) -> Option<ResultHit> {
        let layout = self.layout(viewport_width, viewport_height);
        if layout.professional.contains(point) {
            return Some(ResultHit::Professional);
        }
        if layout.breadcrumb_back.contains(point) {
            return Some(ResultHit::BackHome);
        }
        for (index, rect) in layout.buttons.into_iter().enumerate() {
            if rect.contains(point) {
                return Some(RESULT_BUTTON_HITS[index]);
            }
        }
        for (index, rect) in layout.screens.iter().enumerate() {
            if rect.contains(point) {
                return Some(ResultHit::Screen(index));
            }
        }
        None
    }

    /// Number of wrapped hint lines — exposed so `layout_for` (a static
    /// without surface state) and the paint pass share one answer.
    fn hint_line_count() -> usize {
        let hint = "这是可编辑的设计稿，不是截图：组件、变量、图层都在。先改一处，再试点或导出。";
        hint.chars().count().div_ceil(HINT_CHARS_PER_LINE).max(1)
    }
}

impl Widget for ResultViewSurface<'_> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, cx.available_width, 900.0),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        self.paint_result_view(cx, rect);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Main);
        node.set_label("成品");
        node
    }
}

#[path = "result_view_surface_paint.rs"]
mod paint;

impl ResultViewSurface<'_> {
    fn paint_result_view(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        paint::paint_result_view(self, cx, rect);
    }
}

#[cfg(test)]
#[path = "result_view_surface_tests.rs"]
mod tests;

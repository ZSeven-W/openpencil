//! The works reader: the normal-mode view of a generated work on a touch
//! host (phone or tablet).
//!
//! A full-screen composition over the SAME `WorkspaceState` and document
//! the desktop workspace reads: a header (← Home, title, 普通 / 专业), the
//! stage where the host paints the real canvas framed on one board, a
//! pager for multi-board families, a one-line status with Stop / Retry,
//! and a fixed bottom bar (继续对话 / 改这一页). Tablets arrange the same
//! targets in their own forms (`works_reader_tablet.rs`): a thumbnail
//! strip instead of the pager, and — in landscape — a side panel.
//!
//! Geometry and hit-testing live here and are the ONE answer both the
//! paint pass (`works_reader_paint.rs`) and the host press tier use. The
//! stage rect is also what `host_canvas_geometry::canvas_region` returns
//! while the reader is up, so the canvas paint, the camera fits and every
//! canvas hit-test agree on where the board is.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use op_editor_core::{reader_is_paged, EditorState, ReaderHit, WorkspacePhase, WorkspaceState};

#[path = "works_reader_tablet.rs"]
pub mod tablet;
pub use tablet::{
    side_panel_w, tablet_chat_rect, thumb_plate, thumb_w_for, READER_STRIP_H,
    READER_TABLET_HEADER_H, THUMB_LABEL_H,
};

/// Which composition the reader takes. Phones keep the compact stack;
/// a tablet picks by orientation (an iPad Pro is Expanded in both, so
/// the size class alone cannot tell portrait from landscape).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderForm {
    Phone,
    TabletPortrait,
    TabletLandscape,
}

impl ReaderForm {
    pub fn for_ui(ui: &op_editor_core::EditorUiState, viewport_w: f32, viewport_h: f32) -> Self {
        if !ui.touch_chrome() || ui.compact_layout() {
            Self::Phone
        } else if viewport_w > viewport_h {
            Self::TabletLandscape
        } else {
            Self::TabletPortrait
        }
    }

    pub fn is_tablet(self) -> bool {
        self != Self::Phone
    }
}

/// The reader's header height — what the canvas origin answers while
/// the reader is up. It depends only on the size class, never on the
/// orientation, so `canvas_origin` needs no viewport.
pub fn reader_header_h(ui: &op_editor_core::EditorUiState) -> f32 {
    if ui.compact_layout() {
        READER_HEADER_H
    } else {
        READER_TABLET_HEADER_H
    }
}

/// Header height: the 44 pt back / mode targets with 6 px breathing.
pub const READER_HEADER_H: f32 = 56.0;
/// Pager row under the stage (44 pt prev / next).
pub const READER_PAGER_H: f32 = 48.0;
/// One-line status row (44 pt action target).
pub const READER_STATUS_H: f32 = 44.0;
/// Fixed bottom bar: 48 px buttons with 14 px padding.
pub const READER_BOTTOM_H: f32 = 76.0;

/// The 44 pt floor every reader target keeps.
const TOUCH: f32 = 44.0;
const SIDE_PAD: f32 = 16.0;
const BACK_X: f32 = 6.0;
const MODE_RIGHT_PAD: f32 = 10.0;
const MODE_SEG_MIN_W: f32 = 48.0;
const MODE_SEG_GAP: f32 = 2.0;
const PAGE_LABEL_W: f32 = 72.0;
const BOTTOM_PAD: f32 = 14.0;
const BOTTOM_GAP: f32 = 10.0;
const BUTTON_H: f32 = 48.0;
/// Share of the bottom bar the secondary 继续对话 button takes.
const CONTINUE_FRACTION: f32 = 0.38;

/// Rough label width for layout-time sizing (no backend at layout):
/// ASCII at ~0.55 em, everything else a full em. Paint centres the real
/// glyphs inside whatever rect this picks, so it only has to be close.
pub fn estimate_label_w(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { size * 0.55 } else { size })
        .sum()
}

/// Every rect the reader's paint pass, the host press tier and the tests
/// need.
#[derive(Debug, Clone, PartialEq)]
pub struct ReaderLayout {
    pub form: ReaderForm,
    pub header: Rect,
    pub back: Rect,
    /// Title + subtitle column between the back button and the switch.
    pub title: Rect,
    pub mode_switch: Rect,
    pub mode_normal: Rect,
    pub mode_professional: Rect,
    /// Where the host paints the real canvas.
    pub stage: Rect,
    /// The pager row; `None` for a single long page.
    pub pager: Option<Rect>,
    pub prev: Option<Rect>,
    pub next: Option<Rect>,
    pub page_label: Option<Rect>,
    /// Tablet thumbnail strip row (replaces the pager), when paged.
    pub strip: Option<Rect>,
    /// The strip's tiles on show: `(board slot, tile rect)`.
    pub thumbs: Vec<(usize, Rect)>,
    /// Landscape tablet side panel (status, page, reply, actions).
    pub side_panel: Option<Rect>,
    /// Landscape: the "page N of M" block under the status.
    pub page_info: Option<Rect>,
    /// Landscape: the latest-reply card, when the panel has room.
    pub reply: Option<Rect>,
    pub status: Rect,
    /// The status row's Stop / Retry target, when the phase offers one.
    pub status_action: Option<Rect>,
    pub bottom_bar: Rect,
    pub continue_chat: Rect,
    pub edit_page: Rect,
}

/// The stage alone — what `canvas_region` answers while the reader is up.
/// Kept as its own function of the same inputs so the canvas geometry
/// does not have to build (or know about) the whole layout.
pub fn reader_stage_rect(viewport_w: f32, viewport_h: f32, paged: bool) -> Rect {
    let vw = viewport_w.max(1.0);
    let vh = viewport_h.max(1.0);
    let pager_h = if paged { READER_PAGER_H } else { 0.0 };
    let bottom = pager_h + READER_STATUS_H + READER_BOTTOM_H;
    Rect::xywh(
        0.0,
        READER_HEADER_H,
        vw,
        (vh - READER_HEADER_H - bottom).max(0.0),
    )
}

/// The stage for any reader form (`chat_open`: the tablet chat is up).
pub fn reader_stage_rect_for(
    form: ReaderForm,
    viewport_w: f32,
    viewport_h: f32,
    paged: bool,
    chat_open: bool,
) -> Rect {
    match form {
        ReaderForm::Phone => reader_stage_rect(viewport_w, viewport_h, paged),
        _ => tablet::tablet_stage_rect(form, viewport_w, viewport_h, paged, chat_open),
    }
}

/// Whether the chat is open over the reader.
pub fn reader_chat_open(ui: &op_editor_core::EditorUiState) -> bool {
    ui.mobile_sheet == Some(op_editor_core::size_class::MobileSheetKind::Ai)
}

/// Whether the reader over `state` reserves its pager row — counted
/// without allocating, since `canvas_region` asks on every canvas query.
pub fn reader_paged_for(state: &EditorState) -> bool {
    let boards = state
        .active_children()
        .iter()
        .filter(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
        .count();
    reader_is_paged(state.editor_ui.workspace.family, boards)
}

/// Pure layout. `normal_label` / `professional_label` size the mode
/// switch; `action_label` sizes the status action (`None` = no action).
pub fn reader_layout(
    viewport_w: f32,
    viewport_h: f32,
    paged: bool,
    normal_label: &str,
    professional_label: &str,
    action_label: Option<&str>,
) -> ReaderLayout {
    let vw = viewport_w.max(1.0);
    let vh = viewport_h.max(1.0);
    let header = Rect::xywh(0.0, 0.0, vw, READER_HEADER_H);
    let top = (READER_HEADER_H - TOUCH) / 2.0;
    let back = Rect::xywh(BACK_X, top, TOUCH, TOUCH);

    let normal_w = (estimate_label_w(normal_label, 12.0) + 16.0).max(MODE_SEG_MIN_W);
    let professional_w = (estimate_label_w(professional_label, 12.0) + 16.0).max(MODE_SEG_MIN_W);
    let switch_w = normal_w + MODE_SEG_GAP + professional_w;
    let mode_switch = Rect::xywh(vw - MODE_RIGHT_PAD - switch_w, top, switch_w, TOUCH);
    let mode_normal = Rect::xywh(mode_switch.origin.x, top, normal_w, TOUCH);
    let mode_professional = Rect::xywh(
        mode_switch.origin.x + normal_w + MODE_SEG_GAP,
        top,
        professional_w,
        TOUCH,
    );
    let title_x = back.origin.x + back.size.x + 4.0;
    let title = Rect::xywh(
        title_x,
        top,
        (mode_switch.origin.x - 8.0 - title_x).max(0.0),
        TOUCH,
    );

    let stage = reader_stage_rect(vw, vh, paged);
    let bottom_bar = Rect::xywh(0.0, vh - READER_BOTTOM_H, vw, READER_BOTTOM_H);
    let status = Rect::xywh(
        0.0,
        bottom_bar.origin.y - READER_STATUS_H,
        vw,
        READER_STATUS_H,
    );
    let (pager, prev, next, page_label) = if paged {
        let row = Rect::xywh(0.0, status.origin.y - READER_PAGER_H, vw, READER_PAGER_H);
        let y = row.origin.y + (READER_PAGER_H - TOUCH) / 2.0;
        let label = Rect::xywh((vw - PAGE_LABEL_W) / 2.0, y, PAGE_LABEL_W, TOUCH);
        let prev = Rect::xywh(label.origin.x - TOUCH, y, TOUCH, TOUCH);
        let next = Rect::xywh(label.origin.x + PAGE_LABEL_W, y, TOUCH, TOUCH);
        (Some(row), Some(prev), Some(next), Some(label))
    } else {
        (None, None, None, None)
    };
    let status_action = action_label.map(|label| {
        let w = (estimate_label_w(label, 13.0) + 28.0).max(72.0);
        Rect::xywh(vw - SIDE_PAD - w, status.origin.y, w, READER_STATUS_H)
    });

    let inner_w = (vw - SIDE_PAD * 2.0 - BOTTOM_GAP).max(0.0);
    let continue_w = (inner_w * CONTINUE_FRACTION).floor();
    let button_y = bottom_bar.origin.y + BOTTOM_PAD;
    let continue_chat = Rect::xywh(SIDE_PAD, button_y, continue_w, BUTTON_H);
    let edit_page = Rect::xywh(
        SIDE_PAD + continue_w + BOTTOM_GAP,
        button_y,
        (inner_w - continue_w).max(0.0),
        BUTTON_H,
    );

    ReaderLayout {
        form: ReaderForm::Phone,
        header,
        back,
        title,
        mode_switch,
        mode_normal,
        mode_professional,
        stage,
        pager,
        prev,
        next,
        page_label,
        strip: None,
        thumbs: Vec::new(),
        side_panel: None,
        page_info: None,
        reply: None,
        status,
        status_action,
        bottom_bar,
        continue_chat,
        edit_page,
    }
}

/// The reader widget, built per frame from the editor state.
pub struct WorksReader<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: &'a WorkspaceState,
    pub ui: &'a op_editor_core::EditorUiState,
    pub title: String,
    /// The active page's board ids, in document order.
    pub boards: Vec<String>,
    pub now_ms: u64,
    /// Where the host's canvas drew the current board, in screen px.
    /// When set, the stage outside it is masked so the reader shows one
    /// page — not the neighbours the canvas also painted.
    pub board_screen: Option<Rect>,
    /// Width / height of the first board — sizes the tablet strip tiles.
    pub board_aspect: f32,
    /// The board on show's name (the tablet side panel's page line).
    pub current_name: Option<String>,
    /// The latest finished AI reply (the tablet side panel's card).
    pub last_reply: Option<String>,
}

impl<'a> WorksReader<'a> {
    /// `Some` exactly while the reader owns the screen.
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<Self> {
        if !state.editor_ui.works_reader_visible() {
            return None;
        }
        let boards = op_editor_core::preview_slideshow::active_page_boards(state);
        let selected = state
            .editor_ui
            .workspace
            .selected
            .min(boards.len().saturating_sub(1));
        let (board_aspect, current_name) = extras::board_facts(state, &boards, selected);
        Some(Self {
            id: WidgetId::new(7720),
            theme: theme_for(&state.editor_ui),
            state: &state.editor_ui.workspace,
            ui: &state.editor_ui,
            title: super::workspace_surface::workspace_title(state),
            boards,
            now_ms,
            board_screen: None,
            board_aspect,
            current_name,
            last_reply: extras::last_reply(state),
        })
    }

    /// Builder: the current board's on-screen rect (see `board_screen`).
    pub fn with_board_screen(mut self, rect: Option<Rect>) -> Self {
        self.board_screen = rect;
        self
    }

    fn tr(&self, key: &'static str) -> &'static str {
        op_i18n::translate(self.ui.locale, key)
    }

    /// Whether the pager row is reserved.
    pub fn paged(&self) -> bool {
        reader_is_paged(self.state.family, self.boards.len())
    }

    /// The board slot on show, clamped into the current board list.
    pub fn current_index(&self) -> usize {
        self.state.selected.min(self.boards.len().saturating_sub(1))
    }

    /// The status row's action for the current phase: Stop while a run
    /// is live, Retry once it failed or was stopped, nothing when done.
    /// Retry re-sends the stored brief, so a work opened for reading (no
    /// brief) offers none rather than a button that cannot do anything.
    pub fn status_action(&self) -> Option<ReaderHit> {
        match self.state.phase {
            WorkspacePhase::Generating => Some(ReaderHit::Stop),
            WorkspacePhase::Failed | WorkspacePhase::Stopped
                if !self.state.brief.trim().is_empty() =>
            {
                Some(ReaderHit::Retry)
            }
            _ => None,
        }
    }

    pub fn status_action_label(&self) -> Option<&'static str> {
        self.status_action().map(|hit| match hit {
            ReaderHit::Stop => self.tr("ai.stopGenerating"),
            _ => self.tr("workspace.retry"),
        })
    }

    /// 改这一页 needs a finished (or interrupted) run and a board to bind.
    /// While a run is live the next send would replace it, so the button
    /// paints disabled and its press does nothing.
    pub fn edit_enabled(&self) -> bool {
        self.state.phase != WorkspacePhase::Generating && !self.boards.is_empty()
    }

    /// The composition for a `viewport_w × viewport_h` screen.
    pub fn form(&self, viewport_w: f32, viewport_h: f32) -> ReaderForm {
        ReaderForm::for_ui(self.ui, viewport_w, viewport_h)
    }

    pub fn layout(&self, viewport_w: f32, viewport_h: f32) -> ReaderLayout {
        let form = self.form(viewport_w, viewport_h);
        if form.is_tablet() {
            return tablet::tablet_reader_layout(
                form,
                viewport_w,
                viewport_h,
                &tablet::TabletInputs {
                    paged: self.paged(),
                    chat_open: reader_chat_open(self.ui),
                    normal_label: self.tr("home.mode.normal"),
                    professional_label: self.tr("home.mode.professional"),
                    action_label: self.status_action_label(),
                    board_count: self.boards.len(),
                    current: self.current_index(),
                    thumb_w: thumb_w_for(self.board_aspect),
                },
            );
        }
        reader_layout(
            viewport_w,
            viewport_h,
            self.paged(),
            self.tr("home.mode.normal"),
            self.tr("home.mode.professional"),
            self.status_action_label(),
        )
    }

    pub fn hit_test(&self, viewport_w: f32, viewport_h: f32, point: Point2D) -> Option<ReaderHit> {
        let layout = self.layout(viewport_w, viewport_h);
        self.hit_test_layout(&layout, point)
    }

    /// Hit-test against a prebuilt layout (the host presses against the
    /// same rects the frame painted). `None` inside the reader means a
    /// dead spot (a disabled button, header padding) — the host still
    /// swallows it, the reader is a takeover.
    pub fn hit_test_layout(&self, layout: &ReaderLayout, point: Point2D) -> Option<ReaderHit> {
        if layout.header.contains(point) {
            if layout.back.contains(point) {
                return Some(ReaderHit::Back);
            }
            if layout.mode_normal.contains(point) {
                return Some(ReaderHit::ModeNormal);
            }
            if layout.mode_professional.contains(point) {
                return Some(ReaderHit::ModeProfessional);
            }
            return None;
        }
        // The targets never overlap each other or the stage, so the
        // order below only decides what a DEAD spot (bar padding, a
        // disabled button) resolves to: nothing.
        if layout.continue_chat.contains(point) {
            return Some(ReaderHit::ContinueChat);
        }
        if layout.edit_page.contains(point) {
            return self.edit_enabled().then_some(ReaderHit::EditPage);
        }
        if layout
            .status_action
            .is_some_and(|rect| rect.contains(point))
        {
            return self.status_action();
        }
        if let Some((index, _)) = layout.thumbs.iter().find(|(_, rect)| rect.contains(point)) {
            return Some(ReaderHit::Page(*index));
        }
        let count = self.boards.len();
        let current = self.current_index();
        if layout.prev.is_some_and(|rect| rect.contains(point)) {
            return (current > 0).then_some(ReaderHit::Prev);
        }
        if layout.next.is_some_and(|rect| rect.contains(point)) {
            return (current + 1 < count).then_some(ReaderHit::Next);
        }
        layout.stage.contains(point).then_some(ReaderHit::Stage)
    }
}

impl Widget for WorksReader<'_> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, cx.available_width, 844.0),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        paint::paint_reader(self, cx, rect);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Main);
        node.set_label(self.title.clone());
        node
    }
}

#[path = "works_reader_paint.rs"]
mod paint;

#[path = "works_reader_extras.rs"]
mod extras;

#[cfg(test)]
#[path = "works_reader_tests.rs"]
mod tests;

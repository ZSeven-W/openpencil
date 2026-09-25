//! The desktop-first Studio Home surface (制图台).
//!
//! Geometry and hit-testing live here; the immediate-mode paint pass is
//! in `home_surface_paint.rs`. The surface is intentionally document-
//! agnostic: it reads the same `EditorState` as the canvas and never
//! creates a second model.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use op_editor_core::{EditorState, HomeFamily, HomeHit, HomeState};

#[path = "home_surface_palette.rs"]
mod palette;
pub(crate) use palette::fade;
pub use palette::StudioPalette;

#[path = "home_surface_copy.rs"]
mod copy;

#[path = "home_surface_model.rs"]
mod model;
pub use model::{
    home_model_picker_rects, model_chip_label, model_chip_width, paint_connect_more_row,
    CONNECT_MORE_ROW_GAP, CONNECT_MORE_ROW_H, HOME_MODEL_PICKER_GAP, HOME_MODEL_PICKER_W,
    MODEL_CHIP_H,
};

#[path = "home_surface_connect.rs"]
mod connect;

#[path = "home_surface_focus.rs"]
mod focus;
#[path = "home_surface_variants.rs"]
mod variants_toggle;

#[path = "home_surface_brand.rs"]
mod brand_chip;

#[path = "home_surface_layout.rs"]
pub(crate) mod layout;

#[path = "home_surface_tablet.rs"]
pub mod tablet;
#[path = "home_works_list.rs"]
pub(crate) mod works;
pub use layout::compact::BOTTOM_NAV_H as HOME_BOTTOM_NAV_H;
pub use layout::max_scroll_for_mode;
pub use layout::{HomeLayout, EXPLORE_FAMILIES};
pub use works::{works_list_layout, CurrentWork, WorksListLayout, WORKS_RECENT_CAP};

/// Top bar height; the page content scrolls under it.
pub const HOME_TOPBAR_H: f32 = 56.0;

/// ease-out-cubic — the settle curve the entrance choreography uses.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// One block of the Home entrance choreography (welcome, tabs, panels,
/// explore cards stagger 0/60/120/180 ms over 220 ms, rise 8 px).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeEnterBlock {
    Welcome,
    Tabs,
    Panels,
    Explore,
    Recent,
}

impl HomeEnterBlock {
    /// `(start_ms, duration_ms)` — rise is 8 px for every block.
    pub const fn timing(self) -> (u64, u64) {
        match self {
            Self::Welcome => (0, 220),
            Self::Tabs => (60, 220),
            Self::Panels => (120, 220),
            Self::Explore => (180, 220),
            Self::Recent => (180, 220),
        }
    }
}

/// A block's entrance phase at `now_ms`: `(rise_offset_y, alpha)` with
/// ease-out timing. Rise offsets paint the block `dy` px BELOW its final
/// rect; alpha fades every colour. A `shown_at_ms` of 0 means "not
/// started" — the surface paints settled (`t = 1`).
pub fn home_enter(block: HomeEnterBlock, shown_at_ms: u64, now_ms: u64) -> (f32, f32) {
    if shown_at_ms == 0 {
        return (0.0, 1.0);
    }
    let (start, duration) = block.timing();
    let elapsed = now_ms.saturating_sub(shown_at_ms.saturating_add(start));
    let t = (elapsed as f32 / duration as f32).clamp(0.0, 1.0);
    let eased = ease_out_cubic(t);
    ((1.0 - eased) * 8.0, eased)
}

/// The example-art crossfade phase at `now_ms` (0 → 1 over
/// [`op_editor_core::HOME_ART_SWITCH_MS`]); 1 when nothing is switching.
pub fn art_switch_phase(switched_at_ms: u64, now_ms: u64) -> f32 {
    if switched_at_ms == 0 {
        return 1.0;
    }
    let elapsed = now_ms.saturating_sub(switched_at_ms);
    ease_out_cubic((elapsed as f32 / op_editor_core::HOME_ART_SWITCH_MS as f32).clamp(0.0, 1.0))
}

pub struct HomeSurface<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: &'a HomeState,
    pub ui: &'a op_editor_core::EditorUiState,
    pub now_ms: u64,
    /// The model chip's label, derived once per surface from the chat
    /// selection (or the localized connect hint when no agent is
    /// usable) so layout and paint cannot disagree.
    pub chip_label: String,
    /// Whether any chat agent can answer — drives the chip's empty
    /// state and the send button's disabled fill.
    pub usable_agent: bool,
    /// Names of the chat composer's staged attachments (the shared
    /// `chat.pending_attachments` list is the single source of truth).
    pub attachment_names: Vec<String>,
    /// Recent `.op` files (basename only), capped to the row's five
    /// chips.
    pub recent_files: Vec<String>,
    /// The 作品 page's recent rows (basename only), in `recent_files`
    /// order so a row index is a `recent_files` index.
    pub works_recent: Vec<String>,
    /// The live document as the 作品 page's current-work card.
    pub current_work: Option<CurrentWork>,
}

/// The first `cap` recent files, basename only.
fn recent_basenames(state: &EditorState, cap: usize) -> Vec<String> {
    state
        .editor_ui
        .recent_files
        .iter()
        .take(cap)
        .map(|file| {
            file.path
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&file.path)
                .to_string()
        })
        .collect()
}

impl<'a> HomeSurface<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<Self> {
        state.editor_ui.home.visible.then(|| Self {
            id: WidgetId::new(7600),
            theme: theme_for(&state.editor_ui),
            state: &state.editor_ui.home,
            ui: &state.editor_ui,
            now_ms,
            chip_label: model::model_chip_label(state),
            usable_agent: state.has_usable_chat_agent(),
            attachment_names: state
                .chat
                .pending_attachments
                .iter()
                .map(|attachment| attachment.name.clone())
                .collect(),
            recent_files: recent_basenames(state, 5),
            works_recent: recent_basenames(state, WORKS_RECENT_CAP),
            current_work: CurrentWork::for_editor(state),
        })
    }

    pub fn layout_for(
        viewport_width: f32,
        viewport_height: f32,
        task: HomeFamily,
        model_chip_label_w: f32,
    ) -> HomeLayout {
        layout::layout_for(viewport_width, viewport_height, task, model_chip_label_w)
    }

    pub fn layout_for_scrolled(
        viewport_width: f32,
        viewport_height: f32,
        task: HomeFamily,
        scroll_y: f32,
        model_chip_label_w: f32,
    ) -> HomeLayout {
        layout::layout_for_scrolled(
            viewport_width,
            viewport_height,
            task,
            scroll_y,
            model_chip_label_w,
        )
    }

    pub fn max_scroll_for(
        viewport_width: f32,
        viewport_height: f32,
        task: HomeFamily,
        model_chip_label_w: f32,
    ) -> f32 {
        layout::max_scroll_for(viewport_width, viewport_height, task, model_chip_label_w)
    }

    pub fn layout(&self, viewport_width: f32, viewport_height: f32) -> HomeLayout {
        self.layout_at_scroll(viewport_width, viewport_height, self.state.scroll_y)
    }

    /// [`Self::layout`] as if the page were scrolled to `scroll_y` (clamped
    /// into range the same way) — the keyboard focus order reads the page
    /// at the top so a target scrolled under the top bar still counts.
    pub fn layout_at_scroll(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        scroll_y: f32,
    ) -> HomeLayout {
        let chip_w = model::model_chip_width(&self.chip_label);
        let compact = self.ui.compact_layout();
        if tablet::is_touch_tablet(self.ui) {
            let visible_h = (viewport_height - self.ui.keyboard_occlusion.max(0.0)).max(0.0);
            let max_scroll = self.tablet_max_scroll(viewport_width, visible_h);
            return self.tablet_layout(
                viewport_width,
                viewport_height,
                scroll_y.clamp(0.0, max_scroll),
            );
        }
        // The page lays out against the full viewport — a software
        // keyboard covers the bottom, it does not resize the window — but
        // the band it covers has to come out of the SCROLL RANGE or the
        // content under it can never be brought into view.
        let max_scroll = layout::max_scroll_for_mode(
            viewport_width,
            (viewport_height - self.ui.keyboard_occlusion.max(0.0)).max(0.0),
            self.state.task,
            chip_w,
            compact,
            self.ui.locale,
        );
        let focused = compact && self.state.composer_focused;
        let mut layout = layout::layout_for_scrolled_mode(
            viewport_width,
            viewport_height,
            self.state.task,
            // The scroll still applies while focused: on a short phone the
            // lifted composer can need the keyboard reveal to bring Send up.
            scroll_y.clamp(0.0, max_scroll),
            chip_w,
            compact,
            self.ui.locale,
        );
        if focused {
            layout::compact::collapse_for_focused_composer(&mut layout);
        }
        if self.ui.touch_chrome() {
            connect::drop_cli_row_for_touch(&mut layout);
        }
        if self.state.variants_unavailable {
            layout.variants = Rect::ZERO;
        }
        layout
    }

    /// The touch-tablet page: the wide composition with tablet margins
    /// and the 作品 grid in place of the recent chip row.
    fn tablet_layout(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        scroll_y: f32,
    ) -> HomeLayout {
        let mut layout = layout::wide_layout_for_scrolled(
            viewport_width,
            viewport_height,
            self.state.task,
            scroll_y,
            model::model_chip_width(&self.chip_label),
            self.ui.locale,
            tablet::TABLET_PAD_X,
        );
        tablet::adapt_layout(
            &mut layout,
            self.current_work.is_some(),
            self.works_recent.len(),
        );
        connect::drop_cli_row_for_touch(&mut layout);
        if self.state.variants_unavailable {
            layout.variants = Rect::ZERO;
        }
        layout
    }

    /// The tablet page's furthest scroll for a `visible_h`-tall view
    /// (the viewport minus any software keyboard).
    pub fn tablet_max_scroll(&self, viewport_width: f32, visible_h: f32) -> f32 {
        let layout = self.tablet_layout(viewport_width, visible_h, 0.0);
        (layout.recent.origin.y + layout.recent.size.y + tablet::PAGE_PAD_BOTTOM - visible_h)
            .max(0.0)
    }

    /// The tablet 作品 grid for `layout` (`None` off tablets).
    pub fn works_grid(&self, layout: &HomeLayout) -> Option<tablet::WorksGrid> {
        tablet::is_touch_tablet(self.ui).then(|| {
            tablet::works_grid(
                layout.recent.origin.x,
                layout.recent.origin.y,
                layout.recent.size.x,
                self.current_work.is_some(),
                self.works_recent.len(),
            )
        })
    }

    /// Whether the phone's 作品 page is on show (compact only; a wide
    /// Home has no bottom nav to reach it from).
    pub fn works_page(&self) -> bool {
        self.state.works_open && self.ui.compact_layout()
    }

    /// The active task's example prompt — what 使用这个示例 fills.
    pub fn example_prompt(&self) -> &'static str {
        copy::task_copy(self.ui.locale, self.state.task, self.state.task_draft()).example
    }

    /// Any task's example prompt (the explore cards select a task and
    /// fill its example in one press).
    pub fn example_prompt_for(&self, family: HomeFamily) -> &'static str {
        copy::task_copy(self.ui.locale, family, self.state.draft_for(family)).example
    }

    /// What the Send button does right now. Paint and the host's press
    /// both read this, so the label never promises a different action
    /// than the press takes.
    pub fn send_mode(&self) -> op_editor_core::HomeSendMode {
        self.state
            .send_mode(self.example_prompt(), self.usable_agent)
    }

    /// Whether the Send button's "the example will be used" hint shows:
    /// hovering the button while the box is still empty.
    pub fn send_example_hint_visible(&self) -> bool {
        self.state.hover == Some(HomeHit::Send)
            && self.state.draft.trim().is_empty()
            && self.send_mode() == op_editor_core::HomeSendMode::UseExample
    }

    /// The i18n key of the tooltip floating over the Send button, if any:
    /// a website import's timed failure, the import explanation while a
    /// link-only draft's button is hovered, else the example hint.
    pub fn send_hint_key(&self) -> Option<&'static str> {
        let import = &self.state.site_import;
        if import.hint_visible(self.now_ms) {
            return Some("home.siteImport.failed");
        }
        let hovered = self.state.hover == Some(HomeHit::Send);
        if hovered
            && !import.is_importing()
            && self.send_mode() == op_editor_core::HomeSendMode::ImportSite
        {
            return Some("home.siteImport.hint");
        }
        self.send_example_hint_visible()
            .then_some("home.submit.exampleHint")
    }

    /// The Send label's i18n key for the current mode.
    pub fn send_label_key(&self) -> &'static str {
        copy::send_label_key(self.send_mode(), self.state.site_import.is_importing())
    }

    /// The brand chip and its remove button, when a brand is being read or
    /// is staged (`None` otherwise).
    pub fn brand_chip_rects(
        &self,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<(Rect, Rect)> {
        let layout = self.layout(viewport_width, viewport_height);
        brand_chip::chip_rects(self, layout.input_box)
    }

    pub fn hit_test(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        point: Point2D,
    ) -> Option<HomeHit> {
        let layout = self.layout(viewport_width, viewport_height);
        self.hit_test_layout(viewport_width, viewport_height, &layout, point)
    }

    /// Hit-test against a prebuilt `layout` (the focus order validates
    /// its targets against the unscrolled page).
    pub fn hit_test_layout(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        layout: &HomeLayout,
        point: Point2D,
    ) -> Option<HomeHit> {
        // The 接入卡 is modal over the whole surface: presses inside its
        // rows act, presses anywhere else (even on Home chrome) close it.
        if self.state.connect_card_open {
            return Some(connect::connect_card_hit(layout, point).unwrap_or(HomeHit::ConnectClose));
        }
        // A touch tablet grows the desktop-sized targets to 44 pt.
        let tablet = tablet::is_touch_tablet(self.ui);
        let hits = |rect: Rect| tablet::hits(tablet, rect, point);
        if hits(layout.professional) {
            return Some(HomeHit::Professional);
        }
        if layout.mode_normal.size.y > 0.0 && layout.mode_normal.contains(point) {
            return Some(HomeHit::ModeNormal);
        }
        // The compact top bar's settings gear shares the bottom nav's
        // settings destination.
        if layout.settings.size.y > 0.0 && layout.settings.contains(point) {
            return Some(HomeHit::NavSettings);
        }
        if hits(layout.open_file) {
            return Some(HomeHit::OpenFile);
        }
        if self.ui.account_ui_available && hits(layout.account) {
            return Some(HomeHit::Account);
        }
        // The compact bottom nav (创作 / 作品 / 设置) is pinned chrome
        // painted over the scrolling page, so it answers before any
        // page target.
        for (index, rect) in layout.nav_items.iter().enumerate() {
            if rect.size.y > 0.0 && rect.contains(point) {
                return Some(match index {
                    0 => HomeHit::NavCreate,
                    1 => HomeHit::NavProjects,
                    _ => HomeHit::NavSettings,
                });
            }
        }
        // The 作品 page replaces the whole 创作 column under the chrome.
        if self.works_page() {
            let works = self.works_layout(viewport_width, viewport_height);
            return self.works_hit(&works, point);
        }
        if self.state.more_open && layout.more_popover.contains(point) {
            for (index, rect) in layout.more_rows.iter().enumerate() {
                if rect.contains(point) {
                    let hidden = HomeLayout::hidden_tasks(layout.tabs_row.size.x, self.state.task);
                    return hidden.get(index).copied().map(HomeHit::MoreItem);
                }
            }
            return Some(HomeHit::More);
        }
        if hits(layout.more_button) {
            return Some(HomeHit::More);
        }
        for (index, rect) in layout.tabs.into_iter().enumerate() {
            if rect.contains(point) {
                return Some(HomeHit::Tab(HomeFamily::ALL[index]));
            }
        }
        for (index, rect) in layout.segment_options.into_iter().enumerate() {
            if rect.size.x > 0.0 && hits(rect) {
                return Some(HomeHit::Segment(index as u8));
            }
        }
        if self.state.replace_pending {
            if layout.replace_use.contains(point) {
                return Some(HomeHit::ReplaceConfirm);
            }
            if layout.replace_keep.contains(point) {
                return Some(HomeHit::ReplaceKeep);
            }
        }
        if hits(layout.use_example) {
            // While a workspace is active on this document the footer
            // link returns to it instead of offering the example.
            return Some(if self.ui.workspace.active {
                HomeHit::BackToWorkspace
            } else {
                HomeHit::UseExample
            });
        }
        if layout.send.contains(point) {
            return Some(HomeHit::Send);
        }
        if hits(layout.model_chip) {
            return Some(HomeHit::ModelChip);
        }
        if hits(layout.screenshot) {
            return Some(HomeHit::Attachment);
        }
        if hits(layout.reference_link) {
            return Some(HomeHit::ReferenceLink);
        }
        if hits(layout.figma) {
            return Some(HomeHit::Figma);
        }
        if layout.variants.size.x > 0.0 && hits(layout.variants) {
            return Some(HomeHit::Variants);
        }
        if let Some((_, close)) = brand_chip::chip_rects(self, layout.input_box) {
            if close.contains(point) {
                return Some(HomeHit::BrandClear);
            }
        }
        if layout.input_box.contains(point) {
            return Some(HomeHit::Sheet);
        }
        for (index, rect) in layout.explore_cards.into_iter().enumerate() {
            if rect.contains(point) {
                return EXPLORE_FAMILIES
                    .get(index)
                    .copied()
                    .map(HomeHit::ExploreCard);
            }
        }
        if hits(layout.new_canvas) {
            return Some(HomeHit::NewCanvas);
        }
        if let Some(grid) = self.works_grid(layout) {
            return grid
                .cards
                .iter()
                .find(|(_, rect)| rect.contains(point))
                .map(|(hit, _)| *hit);
        }
        if layout.recent.size.y > 0.0 {
            for (index, rect) in layout.recent_chips.iter().enumerate() {
                if rect.size.x > 0.0 && rect.contains(point) && index < self.recent_files.len() {
                    return Some(HomeHit::Recent(index));
                }
            }
        }
        None
    }

    /// The IME caret rect inside the composer's input box.
    pub fn focused_input_caret_rect(&self, viewport_width: f32, viewport_height: f32) -> Rect {
        let layout = self.layout(viewport_width, viewport_height);
        let text = self.state.input.text();
        let caret = jian_core::text_input::prev_char_boundary(
            text,
            self.state.input.caret().min(text.len()),
        );
        // Single-line estimate on the first wrapped row; the IME panel
        // only needs a stable anchor near the caret.
        let x = layout.input_box.origin.x
            + 14.0
            + (text[..caret].chars().count() as f32 * 8.0).min(layout.input_box.size.x - 16.0);
        Rect::xywh(x, layout.input_box.origin.y + 13.0, 1.5, 20.0)
    }

    /// Map a press inside the input box to a caret byte offset
    /// (single-line estimate, matching the caret anchor above).
    pub fn input_offset_at(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        point: Point2D,
    ) -> Option<usize> {
        let rect = self.layout(viewport_width, viewport_height).input_box;
        rect.contains(point)
            .then_some(self.state.input.text().len())
    }
}

impl Widget for HomeSurface<'_> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, cx.available_width, 900.0),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        self.paint_home(cx, rect);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Main);
        node.set_label("制图台");
        node
    }
}

#[path = "home_surface_paint.rs"]
mod paint;

impl HomeSurface<'_> {
    fn paint_home(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        paint::paint_home(self, cx, rect);
    }
}

#[cfg(test)]
#[path = "home_surface_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "home_surface_layout_tests.rs"]
mod layout_tests;

#[cfg(test)]
#[path = "home_surface_compact_layout_tests.rs"]
mod compact_layout_tests;

#[cfg(test)]
#[path = "home_surface_motion_tests.rs"]
mod motion_tests;

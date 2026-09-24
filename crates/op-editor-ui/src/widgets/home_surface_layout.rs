//! Geometry for the Studio Home surface: the scroll-translated rect set
//! both paint and hit-test read. Split from the `home_surface` spine so
//! each stays under the repo's 800-line ceiling.

use super::connect::connect_card_rects;
use super::copy;
use super::model::MODEL_CHIP_H;
use super::HOME_TOPBAR_H;
use crate::Rect;
use op_editor_core::HomeFamily;

/// Page shell: max width incl. the 8 px side padding, so the content
/// column is 1280 at 1440 (prototype `.page-shell`).
const SHELL_MAX_W: f32 = 1296.0;
const SHELL_PAD_X: f32 = 8.0;
/// 36 px headline at 1.36 line-height (prototype `.welcome h1`).
const PAGE_PAD_TOP: f32 = 24.0;
/// Short windows (prototype `@media 851+ and max-height:950`) tighten
/// the page rhythm so explore + recent fit under 949 px at 1440.
const PAGE_PAD_TOP_SHORT: f32 = 22.0;
const SHORT_WINDOW_MAX_H: f32 = 950.0;
const WELCOME_H1_H: f32 = 49.0;
const WELCOME_SUB_H: f32 = 26.0;
const WELCOME_SUB_GAP: f32 = 4.0;
/// Wide `.welcome{margin-bottom:18px}`.
const WELCOME_TABS_GAP: f32 = 18.0;
/// Wide `.task-tab{height:60px}`.
const TAB_H: f32 = 60.0;
/// Wide tab gap 12; viewports ≤1180 compact to 8
/// (`.layout-wide … .task-tabs{gap:12px}`, `@media(max-width:1180px)`).
const TAB_GAP: f32 = 12.0;
const TAB_GAP_NARROW: f32 = 8.0;
const TAB_GAP_BREAK_W: f32 = 1180.0;
/// Hairline sits this far under the tab row (wide padding-bottom 12).
const TABS_ROW_PAD_BOTTOM: f32 = 12.0;
/// Wide `.task-selector{margin-bottom:14px}` under the hairline.
const TABS_PANELS_GAP: f32 = 14.0;
/// Wide `.workspace>.panel{height:370px}`.
const PANEL_H: f32 = 370.0;
/// Stacked (≤850) panel heights (wide `@media(max-width:850px)`).
const PANEL_H_STACKED_COMPOSER: f32 = 335.0;
const PANEL_H_STACKED_PREVIEW: f32 = 400.0;
const PANEL_GAP: f32 = 18.0;
/// Composer : preview = 1.05fr : 1fr.
const COMPOSER_FRAC: f32 = 1.05;
/// Viewports narrower than this stack the two panels.
const STACK_BREAK_W: f32 = 850.0;
/// Content columns narrower than this compact the tab row to 4 + 更多.
const TABS_COMPACT_W: f32 = 700.0;
const EXPLORE_TOP_GAP: f32 = 26.0;
const EXPLORE_TOP_GAP_SHORT: f32 = 22.0;
const EXPLORE_HEADING_H: f32 = 22.0;
const EXPLORE_HEADING_GAP: f32 = 15.0;
const EXPLORE_CARD_H: f32 = 171.0;
const EXPLORE_CARD_H_SHORT: f32 = 167.0;
const EXPLORE_CARD_H_NARROW: f32 = 162.0;
const EXPLORE_CARD_H_STACKED: f32 = 178.0;
const EXPLORE_CARD_GAP: f32 = 16.0;
const EXPLORE_NARROW_MAX_W: f32 = 1060.0;
const RECENT_TOP_GAP: f32 = 20.0;
const RECENT_TOP_GAP_SHORT: f32 = 18.0;
const RECENT_PAD_TOP: f32 = 13.0;
const RECENT_H: f32 = 50.0;
/// The composer's inner chrome (prototype `.composer` 16/18/18 padding,
/// wide label row `padding-top:0; min-height:41px; padding-bottom:10px`,
/// 42 px tools band, 44 px submit row).
const COMPOSER_PAD_X: f32 = 18.0;
const COMPOSER_PAD_TOP: f32 = 16.0;
const COMPOSER_PAD_BOTTOM: f32 = 18.0;
const LABEL_ROW_H: f32 = 41.0;
const TOOLS_BAND_H: f32 = 52.0;
const SUBMIT_ROW_H: f32 = 44.0;
const INPUT_MIN_H: f32 = 130.0;
/// Wide `.preview-heading{min-height:59px}` (65 on unstacked viewports
/// ≤1060, where the prototype shrinks the phones).
const PREVIEW_HEADING_H: f32 = 59.0;
const PREVIEW_HEADING_H_NARROW: f32 = 65.0;
pub const SEGMENT_OPTION_H: f32 = 25.0;
const SEGMENT_PAD: f32 = 2.0;
const SEGMENT_OPTION_PAD_X: f32 = 17.0;
/// The 更多 button width at compact widths.
const MORE_BUTTON_W: f32 = 92.0;
/// The 更多 popover: caption + one row per hidden task.
pub const MORE_POPOVER_W: f32 = 248.0;
pub const MORE_ROW_H: f32 = 46.0;
const MORE_CAPTION_H: f32 = 26.0;
/// Fixed recent-project chip width (names ellipsize in paint).
pub const RECENT_CHIP_W: f32 = 132.0;

/// Every rect the surface paints, in viewport coordinates (already
/// scroll-translated). Zero-sized rects mark absent targets.
#[derive(Debug, Clone, PartialEq)]
pub struct HomeLayout {
    // Top bar (pinned; never scroll-translated)
    pub open_file: Rect,
    /// The top bar's account avatar. Zero-sized when the host has not
    /// enabled the account gate, so it cannot be hit or painted.
    pub account: Rect,
    pub professional: Rect,
    /// The 普通 / 专业 segmented control of the compact (phone) top bar:
    /// the whole control plus its 普通 segment. The 专业 segment reuses
    /// `professional`. Zero on the desktop composition.
    pub mode_switch: Rect,
    pub mode_normal: Rect,
    /// The compact top bar's settings gear (shares the bottom nav's
    /// NavSettings hit). Zero on the desktop composition.
    pub settings: Rect,
    // Page column
    pub welcome: Rect,
    pub welcome_sub: Rect,
    pub tabs_row: Rect,
    pub tabs: [Rect; 7],
    /// The 更多 ▾ button (zero when all seven tabs fit).
    pub more_button: Rect,
    /// Popover rect + one row per hidden task (zero unless compact).
    pub more_popover: Rect,
    pub more_rows: [Rect; 4],
    pub composer: Rect,
    pub preview: Rect,
    pub label_row: Rect,
    /// Segmented control (zero when the task has none) + its options
    /// (zero-padded to 3).
    pub segment: Rect,
    pub segment_options: [Rect; 3],
    pub input_box: Rect,
    pub tools_row: Rect,
    pub screenshot: Rect,
    pub reference_link: Rect,
    pub figma: Rect,
    /// The 3-directions toggle (zero on the compact composition).
    pub variants: Rect,
    pub submit_row: Rect,
    pub model_chip: Rect,
    pub send: Rect,
    /// Inline replace-confirm strip inside the input box (zero unless
    /// `replace_pending`).
    pub replace_strip: Rect,
    pub replace_keep: Rect,
    pub replace_use: Rect,
    pub preview_heading: Rect,
    pub preview_art: Rect,
    pub preview_footer: Rect,
    pub use_example: Rect,
    pub explore_heading: Rect,
    pub explore_cards: [Rect; 3],
    pub recent: Rect,
    pub recent_chips: [Rect; 5],
    pub new_canvas: Rect,
    /// The 接入卡 modal centred over the composer.
    pub connect_card: Rect,
    pub connect_rows: [Rect; 3],
    /// The compact (phone) bottom nav (创作 / 作品 / 设置): the bar plus
    /// its three equal items. Zero on the desktop composition.
    pub bottom_nav: Rect,
    pub nav_items: [Rect; 3],
}

impl HomeLayout {
    /// The task ids the tab row shows: all seven on wide content, or
    /// three fixed heads plus the selected hidden task when compact (so
    /// the active task is always visible, prototype `taskKeys()`).
    pub fn visible_tasks(content_w: f32, task: HomeFamily) -> Vec<HomeFamily> {
        if content_w >= TABS_COMPACT_W {
            return HomeFamily::ALL.to_vec();
        }
        let heads: Vec<HomeFamily> = HomeFamily::ALL.iter().copied().take(3).collect();
        if heads.contains(&task) {
            HomeFamily::ALL.iter().copied().take(4).collect()
        } else {
            let mut visible = heads;
            visible.push(task);
            visible
        }
    }

    /// The tasks hidden from the tab row (the 更多 popover's rows).
    pub fn hidden_tasks(content_w: f32, task: HomeFamily) -> Vec<HomeFamily> {
        let visible = Self::visible_tasks(content_w, task);
        HomeFamily::ALL
            .iter()
            .copied()
            .filter(|family| !visible.contains(family))
            .collect()
    }
}

/// Resolve the full rect set (see `HomeLayout`).
pub fn layout_for(
    viewport_width: f32,
    viewport_height: f32,
    task: HomeFamily,
    model_chip_label_w: f32,
) -> HomeLayout {
    layout_for_scrolled(
        viewport_width,
        viewport_height,
        task,
        0.0,
        model_chip_label_w,
    )
}

#[allow(clippy::too_many_lines)]
pub fn layout_for_scrolled(
    viewport_width: f32,
    viewport_height: f32,
    task: HomeFamily,
    scroll_y: f32,
    model_chip_label_w: f32,
) -> HomeLayout {
    layout_for_scrolled_mode(
        viewport_width,
        viewport_height,
        task,
        scroll_y,
        model_chip_label_w,
        false,
    )
}

/// The mode-aware core behind [`layout_for_scrolled`]. `compact` is the
/// host's `EditorUiState::compact_layout()` — the phone (touch +
/// compact size class) composition; every other caller keeps the
/// desktop branch and its exact rects.
pub(crate) fn layout_for_scrolled_mode(
    viewport_width: f32,
    viewport_height: f32,
    task: HomeFamily,
    scroll_y: f32,
    model_chip_label_w: f32,
    compact: bool,
) -> HomeLayout {
    if compact {
        return compact::compact_layout_for_scrolled(
            viewport_width,
            viewport_height,
            task,
            scroll_y,
            model_chip_label_w,
        );
    }
    wide_layout_for_scrolled(
        viewport_width,
        viewport_height,
        task,
        scroll_y,
        model_chip_label_w,
    )
}

#[allow(clippy::too_many_lines)]
fn wide_layout_for_scrolled(
    viewport_width: f32,
    viewport_height: f32,
    task: HomeFamily,
    scroll_y: f32,
    model_chip_label_w: f32,
) -> HomeLayout {
    let width = viewport_width.max(1.0);
    let viewport_height = viewport_height.max(1.0);
    let scroll = -scroll_y.max(0.0);
    let stacked = width < STACK_BREAK_W;
    // Short unstacked windows tighten the page rhythm (prototype
    // `@media(min-width:851) and (max-height:950px)`): 22 px page pad,
    // 22 px explore gap, 167 px cards, 18 px recent gap.
    let short_window = !stacked && viewport_height <= SHORT_WINDOW_MAX_H;
    let shell_w = width.min(SHELL_MAX_W);
    let content_x = (width - shell_w) / 2.0 + SHELL_PAD_X;
    let content_w = (shell_w - SHELL_PAD_X * 2.0).max(240.0);
    let translate = |mut y: f32, h: f32| {
        y += scroll;
        Rect::xywh(content_x, y, content_w, h)
    };

    // ── top bar (pinned) ───────────────────────────────────────────
    let professional_w = copy::estimate_text_w(
        copy::home_str(self_locale(), "home.topbar.professional"),
        15.0,
    ) + 30.0
        + 17.0
        + 9.0;
    let open_file_w =
        copy::estimate_text_w(copy::home_str(self_locale(), "home.topbar.openFile"), 15.0)
            + 30.0
            + 17.0
            + 9.0;
    let professional = Rect::xywh(
        width - 30.0 - professional_w,
        (HOME_TOPBAR_H - 38.0) / 2.0,
        professional_w,
        38.0,
    );
    let open_file = Rect::xywh(
        professional.origin.x - 10.0 - open_file_w,
        professional.origin.y,
        open_file_w,
        38.0,
    );
    // The account avatar, left of 打开文件 and the same height as the
    // buttons beside it. A square: it carries an avatar or an initial,
    // never a label, so it reads as the account and not a third action.
    let account = Rect::xywh(
        open_file.origin.x - 12.0 - 38.0,
        open_file.origin.y,
        38.0,
        38.0,
    );

    // ── welcome ────────────────────────────────────────────────────
    let welcome = translate(
        HOME_TOPBAR_H
            + if short_window {
                PAGE_PAD_TOP_SHORT
            } else {
                PAGE_PAD_TOP
            },
        WELCOME_H1_H,
    );
    let welcome_sub = translate(
        welcome.origin.y - scroll + WELCOME_H1_H + WELCOME_SUB_GAP,
        WELCOME_SUB_H,
    );

    // ── task tabs ──────────────────────────────────────────────────
    let tabs_y = welcome_sub.origin.y - scroll + WELCOME_SUB_H + WELCOME_TABS_GAP;
    let tabs_row = translate(tabs_y, TAB_H + TABS_ROW_PAD_BOTTOM);
    let tab_gap = if width <= TAB_GAP_BREAK_W {
        TAB_GAP_NARROW
    } else {
        TAB_GAP
    };
    let compact = content_w < TABS_COMPACT_W;
    let visible = HomeLayout::visible_tasks(content_w, task);
    let more_w = if compact { MORE_BUTTON_W } else { 0.0 };
    let visible_count = visible.len() as f32;
    let tab_w = ((content_w - more_w - tab_gap * (visible_count - 1.0 + compact as u8 as f32))
        / visible_count)
        .max(64.0);
    let mut tabs = [Rect::ZERO; 7];
    for family in HomeFamily::ALL {
        if let Some(slot) = visible.iter().position(|f| *f == family) {
            tabs[HomeFamily::ALL.iter().position(|f| *f == family).unwrap()] = Rect::xywh(
                content_x + slot as f32 * (tab_w + tab_gap),
                tabs_y + scroll,
                tab_w,
                TAB_H,
            );
        }
    }
    let more_button = if compact {
        Rect::xywh(
            content_x + visible_count * (tab_w + tab_gap),
            tabs_y + scroll,
            MORE_BUTTON_W - tab_gap,
            TAB_H,
        )
    } else {
        Rect::ZERO
    };
    let hidden = HomeLayout::hidden_tasks(content_w, task);
    let popover_h = MORE_CAPTION_H + hidden.len() as f32 * MORE_ROW_H + 10.0;
    let more_popover = if compact {
        Rect::xywh(
            more_button.origin.x,
            tabs_y + scroll + TAB_H + 8.0,
            MORE_POPOVER_W,
            popover_h,
        )
    } else {
        Rect::ZERO
    };
    let mut more_rows = [Rect::ZERO; 4];
    for (index, _family) in hidden.iter().enumerate().take(4) {
        more_rows[index] = Rect::xywh(
            more_popover.origin.x + 8.0,
            more_popover.origin.y + MORE_CAPTION_H + index as f32 * MORE_ROW_H,
            MORE_POPOVER_W - 16.0,
            MORE_ROW_H,
        );
    }

    // ── the two panels ─────────────────────────────────────────────
    let panels_y = tabs_y + TAB_H + TABS_ROW_PAD_BOTTOM + TABS_PANELS_GAP;
    let (composer, preview) = if stacked {
        let composer = Rect::xywh(
            content_x,
            panels_y + scroll,
            content_w,
            PANEL_H_STACKED_COMPOSER,
        );
        let preview = Rect::xywh(
            content_x,
            panels_y + PANEL_H_STACKED_COMPOSER + PANEL_GAP + scroll,
            content_w,
            PANEL_H_STACKED_PREVIEW,
        );
        (composer, preview)
    } else {
        let total = content_w - PANEL_GAP;
        let composer_w = (total * COMPOSER_FRAC / (COMPOSER_FRAC + 1.0)).round();
        let preview_w = total - composer_w;
        (
            Rect::xywh(content_x, panels_y + scroll, composer_w, PANEL_H),
            Rect::xywh(
                content_x + composer_w + PANEL_GAP,
                panels_y + scroll,
                preview_w,
                PANEL_H,
            ),
        )
    };

    // ── composer internals ─────────────────────────────────────────
    let inner_x = composer.origin.x + COMPOSER_PAD_X;
    let inner_w = composer.size.x - COMPOSER_PAD_X * 2.0;
    let label_row = Rect::xywh(
        inner_x,
        composer.origin.y + COMPOSER_PAD_TOP,
        inner_w,
        LABEL_ROW_H,
    );
    let labels = copy::segment_labels(self_locale(), task);
    let mut option_widths = [0.0f32; 3];
    for (index, label) in labels.iter().take(3).enumerate() {
        option_widths[index] = copy::estimate_text_w(label, 12.0) + SEGMENT_OPTION_PAD_X * 2.0;
    }
    let segment_w: f32 = option_widths.iter().take(labels.len()).sum::<f32>() + SEGMENT_PAD * 2.0;
    let segment = if labels.is_empty() {
        Rect::ZERO
    } else {
        Rect::xywh(
            inner_x + inner_w - segment_w,
            label_row.origin.y + (LABEL_ROW_H - SEGMENT_OPTION_H - SEGMENT_PAD * 2.0) / 2.0,
            segment_w,
            SEGMENT_OPTION_H + SEGMENT_PAD * 2.0,
        )
    };
    let mut segment_options = [Rect::ZERO; 3];
    let mut option_x = segment.origin.x + SEGMENT_PAD;
    for index in 0..labels.len().min(3) {
        segment_options[index] = Rect::xywh(
            option_x,
            segment.origin.y + SEGMENT_PAD,
            option_widths[index],
            SEGMENT_OPTION_H,
        );
        option_x += option_widths[index];
    }
    let tools_row = Rect::xywh(
        inner_x,
        composer.origin.y + composer.size.y - COMPOSER_PAD_BOTTOM - SUBMIT_ROW_H - TOOLS_BAND_H,
        inner_w,
        TOOLS_BAND_H,
    );
    let input_box = Rect::xywh(
        inner_x,
        label_row.origin.y + LABEL_ROW_H,
        inner_w,
        (tools_row.origin.y - (label_row.origin.y + LABEL_ROW_H)).max(INPUT_MIN_H),
    );
    let screenshot = Rect::xywh(
        tools_row.origin.x + 1.0,
        tools_row.origin.y + 13.0,
        copy::estimate_text_w(copy::home_str(self_locale(), "home.tools.screenshot"), 14.0)
            + 18.0
            + 8.0,
        28.0,
    );
    let reference_link = Rect::xywh(
        screenshot.origin.x + screenshot.size.x + 21.0,
        screenshot.origin.y,
        copy::estimate_text_w(copy::home_str(self_locale(), "home.tools.link"), 14.0) + 18.0 + 8.0,
        28.0,
    );
    let figma = Rect::xywh(
        reference_link.origin.x + reference_link.size.x + 21.0,
        screenshot.origin.y,
        copy::estimate_text_w("Figma", 14.0) + 16.0 + 8.0 + 2.0,
        28.0,
    );
    let variants = Rect::xywh(
        figma.origin.x + figma.size.x + 21.0,
        screenshot.origin.y,
        copy::estimate_text_w(copy::home_str(self_locale(), "home.tools.variants"), 13.0)
            + super::variants_toggle::VARIANTS_CHIP_PAD,
        28.0,
    );
    let submit_row = Rect::xywh(
        inner_x,
        composer.origin.y + composer.size.y - COMPOSER_PAD_BOTTOM - SUBMIT_ROW_H,
        inner_w,
        SUBMIT_ROW_H,
    );
    let send_w = 171.0;
    let send = Rect::xywh(
        submit_row.origin.x + submit_row.size.x - send_w,
        submit_row.origin.y,
        send_w,
        SUBMIT_ROW_H,
    );
    let chip_w = model_chip_label_w;
    let model_chip = Rect::xywh(
        submit_row.origin.x,
        submit_row.origin.y + (SUBMIT_ROW_H - MODEL_CHIP_H) / 2.0,
        chip_w,
        MODEL_CHIP_H,
    );
    let replace_strip = Rect::xywh(
        input_box.origin.x + 10.0,
        input_box.origin.y + input_box.size.y - 40.0,
        input_box.size.x - 20.0,
        32.0,
    );
    let replace_keep = Rect::xywh(
        replace_strip.origin.x + replace_strip.size.x - 170.0,
        replace_strip.origin.y,
        78.0,
        32.0,
    );
    let replace_use = Rect::xywh(
        replace_strip.origin.x + replace_strip.size.x - 86.0,
        replace_strip.origin.y,
        86.0,
        32.0,
    );

    // ── preview internals ──────────────────────────────────────────
    let preview_pad_x = 18.0;
    let heading_h = if stacked || width > EXPLORE_NARROW_MAX_W {
        PREVIEW_HEADING_H
    } else {
        PREVIEW_HEADING_H_NARROW
    };
    let preview_heading = Rect::xywh(
        preview.origin.x + preview_pad_x,
        preview.origin.y + 16.0,
        preview.size.x - preview_pad_x * 2.0,
        heading_h,
    );
    let preview_footer = Rect::xywh(
        preview.origin.x + preview_pad_x,
        preview.origin.y + preview.size.y - 12.0 - 24.0,
        preview.size.x - preview_pad_x * 2.0,
        24.0,
    );
    let preview_art = Rect::xywh(
        preview_heading.origin.x,
        preview_heading.origin.y + preview_heading.size.y,
        preview_heading.size.x,
        (preview_footer.origin.y - (preview_heading.origin.y + preview_heading.size.y)).max(60.0),
    );
    let use_example_w =
        copy::estimate_text_w(copy::home_str(self_locale(), "home.preview.use"), 12.0) + 22.0;
    let use_example = Rect::xywh(
        preview_footer.origin.x + preview_footer.size.x - use_example_w,
        preview_footer.origin.y,
        use_example_w,
        24.0,
    );

    // ── explore + recent ───────────────────────────────────────────
    // Everything below derives from the already scroll-translated
    // panel rects, so these ys stay absolute.
    let panels_bottom =
        (preview.origin.y + preview.size.y).max(composer.origin.y + composer.size.y);
    let explore_card_h = if stacked {
        EXPLORE_CARD_H_STACKED
    } else if width <= EXPLORE_NARROW_MAX_W {
        EXPLORE_CARD_H_NARROW
    } else if short_window {
        EXPLORE_CARD_H_SHORT
    } else {
        EXPLORE_CARD_H
    };
    let explore_y = panels_bottom
        + if short_window {
            EXPLORE_TOP_GAP_SHORT
        } else {
            EXPLORE_TOP_GAP
        };
    let explore_heading = Rect::xywh(content_x, explore_y + scroll, content_w, EXPLORE_HEADING_H);
    let cards_y = explore_y + EXPLORE_HEADING_H + EXPLORE_HEADING_GAP;
    let card_w = ((content_w - EXPLORE_CARD_GAP * 2.0) / 3.0).max(120.0);
    let explore_cards = [
        Rect::xywh(content_x, cards_y + scroll, card_w, explore_card_h),
        Rect::xywh(
            content_x + card_w + EXPLORE_CARD_GAP,
            cards_y + scroll,
            card_w,
            explore_card_h,
        ),
        Rect::xywh(
            content_x + (card_w + EXPLORE_CARD_GAP) * 2.0,
            cards_y + scroll,
            card_w,
            explore_card_h,
        ),
    ];
    let cards_bottom = cards_y + explore_card_h;
    let recent = Rect::xywh(
        content_x,
        cards_bottom
            + if short_window {
                RECENT_TOP_GAP_SHORT
            } else {
                RECENT_TOP_GAP
            }
            + RECENT_PAD_TOP
            + scroll,
        content_w,
        RECENT_H,
    );
    let new_canvas_w =
        copy::estimate_text_w(copy::home_str(self_locale(), "home.recent.newCanvas"), 12.0)
            + 16.0
            + 12.0;
    let new_canvas = Rect::xywh(
        recent.origin.x + recent.size.x - new_canvas_w,
        recent.origin.y + (RECENT_H - 32.0) / 2.0,
        new_canvas_w,
        32.0,
    );
    let mut recent_chips = [Rect::ZERO; 5];
    let mut chip_x = recent.origin.x + 96.0;
    for chip in recent_chips.iter_mut() {
        *chip = Rect::xywh(chip_x, recent.origin.y + 9.0, RECENT_CHIP_W, 32.0);
        chip_x += RECENT_CHIP_W + 10.0;
    }

    let (connect_card, connect_rows) = connect_card_rects(composer);

    HomeLayout {
        open_file,
        account,
        professional,
        welcome,
        welcome_sub,
        tabs_row,
        tabs,
        more_button,
        more_popover,
        more_rows,
        composer,
        preview,
        label_row,
        segment,
        segment_options,
        input_box,
        tools_row,
        screenshot,
        reference_link,
        figma,
        variants,
        submit_row,
        model_chip,
        send,
        replace_strip,
        replace_keep,
        replace_use,
        preview_heading,
        preview_art,
        preview_footer,
        use_example,
        explore_heading,
        explore_cards,
        recent,
        recent_chips,
        new_canvas,
        connect_card,
        connect_rows,
        mode_switch: Rect::ZERO,
        mode_normal: Rect::ZERO,
        settings: Rect::ZERO,
        bottom_nav: Rect::ZERO,
        nav_items: [Rect::ZERO; 3],
    }
}

/// The furthest `scroll_y` may go before the page's last row leaves the
/// viewport.
pub fn max_scroll_for(
    viewport_width: f32,
    viewport_height: f32,
    task: HomeFamily,
    model_chip_label_w: f32,
) -> f32 {
    max_scroll_for_mode(
        viewport_width,
        viewport_height,
        task,
        model_chip_label_w,
        false,
    )
}

/// The mode-aware core behind [`max_scroll_for`]: the compact page's
/// last row is the featured example card, and its scrollable height is
/// bounded by the bottom nav, not the viewport edge.
pub fn max_scroll_for_mode(
    viewport_width: f32,
    viewport_height: f32,
    task: HomeFamily,
    model_chip_label_w: f32,
    compact: bool,
) -> f32 {
    if compact {
        let layout = compact::compact_layout_for_scrolled(
            viewport_width,
            viewport_height,
            task,
            0.0,
            model_chip_label_w,
        );
        let visible_bottom = viewport_height - compact::BOTTOM_NAV_H;
        return (layout.preview.origin.y + layout.preview.size.y + compact::PAGE_PAD_BOTTOM
            - visible_bottom)
            .max(0.0);
    }
    let layout = layout_for(viewport_width, viewport_height, task, model_chip_label_w);
    (layout.recent.origin.y + layout.recent.size.y + 20.0 - viewport_height).max(0.0)
}

/// The phone (compact) branch of the Home geometry — the 2×4 task
/// grid, the compact composer, the single featured example, the bottom
/// nav, and the 普通 / 专业 top bar switch.
#[path = "home_surface_layout_compact.rs"]
pub(crate) mod compact;

/// The 看看还能做什么 cards (knowledge / tutorial / poster).
pub const EXPLORE_FAMILIES: [HomeFamily; 3] = [
    HomeFamily::KnowledgeCards,
    HomeFamily::ScreenshotTutorial,
    HomeFamily::EventPoster,
];

/// Layout-time locale for the label estimates. English keeps the rect
/// estimates stable across locales; paint re-measures every label for
/// real, so only hit-rect sizes depend on this approximation.
fn self_locale() -> op_editor_core::Locale {
    op_editor_core::Locale::EnUs
}

/// The icon→copy gap inside a wide tab (13 px; viewports ≤1180 use the
/// prototype's compact 10 px, where the tab padding also drops to 10).
pub fn tab_copy_gap(viewport_width: f32) -> f32 {
    if viewport_width <= TAB_GAP_BREAK_W {
        10.0
    } else {
        13.0
    }
}

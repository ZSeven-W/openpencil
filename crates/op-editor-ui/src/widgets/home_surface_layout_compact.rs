//! The compact (phone) branch of the Studio Home geometry: the 2×4 task
//! grid with the weakened 空白画布 eighth tile, the compact composer,
//! ONE featured example for the current task, the pinned bottom nav
//! (创作 / 作品 / 设置), and the 普通 / 专业 segmented control in the
//! top bar. Metrics follow the mobile prototype (`mobile.css`,
//! `professional.css`); every touch target keeps the 44 pt floor the
//! touch chrome enforces (`mobile_chrome::TOUCH_TARGET`).

use super::super::copy;
use super::super::HOME_TOPBAR_H;
use super::{connect_card_rects, self_locale, HomeLayout, MODEL_CHIP_H};
use crate::Rect;
use op_editor_core::HomeFamily;

/// `.mobile-scroll` side padding (17 px) — the phone column has no max
/// shell width.
pub(super) const PAGE_PAD_X: f32 = 17.0;
/// `.mobile-scroll` top padding.
const PAGE_PAD_TOP: f32 = 20.0;
/// `.mobile-scroll` bottom padding — the last row's breathing room over
/// the nav.
pub(super) const PAGE_PAD_BOTTOM: f32 = 24.0;
/// `.mobile-bottom-nav` height (67 px + safe area; the safe area belongs
/// to the platform shell, not the layout).
pub const BOTTOM_NAV_H: f32 = 67.0;
/// `.mobile-hero h1`: 27 px × 1.4 line-height.
const HERO_H1_H: f32 = 38.0;
/// `.mobile-hero p`: 12 px × 1.4, `margin-top: 4px`.
const HERO_SUB_H: f32 = 17.0;
const HERO_SUB_GAP: f32 = 4.0;
/// `.mobile-hero{margin-bottom:16px}`.
const HERO_MARGIN_BOTTOM: f32 = 16.0;
/// `.mobile-task-grid`: 4 columns, 5 px gap, `margin-bottom: 15px`.
const GRID_COLS: usize = 4;
const GRID_GAP: f32 = 5.0;
const GRID_MARGIN_BOTTOM: f32 = 15.0;
/// `.mobile-task{min-height:63px}` — icon 22, gap 6, label 11.
const TILE_H: f32 = 63.0;
/// `.mobile-composer`: 10/13/12 padding, radius 17.
const COMPOSER_PAD_X: f32 = 13.0;
const COMPOSER_PAD_TOP: f32 = 10.0;
const COMPOSER_PAD_BOTTOM: f32 = 12.0;
/// `.composer-heading{min-height:40px}` raised to the 44 pt touch floor
/// (the prototype's own real-device rule bumps its options to 44).
const LABEL_ROW_H: f32 = 44.0;
const LABEL_ROW_GAP: f32 = 5.0;
/// `.mobile-input-wrap textarea{height:85px}`.
const INPUT_H: f32 = 85.0;
/// `.material-actions{min-height:44px}`.
const TOOLS_H: f32 = 44.0;
/// `.composer-actions` row: 44 px chips and a 44 px 开始设计 button.
const SUBMIT_H: f32 = 44.0;
/// The composer's inner breathing gap between the input box and the
/// tools band (block flow in the prototype; 6 px reads at 1×).
const INPUT_TOOLS_GAP: f32 = 6.0;
/// `.mobile-examples{margin-top:21px}`, `.section-heading` 22 px tall
/// with a 7 px gap to the card.
const EXAMPLE_TOP_GAP: f32 = 21.0;
const EXAMPLE_HEADING_H: f32 = 22.0;
const EXAMPLE_HEADING_GAP: f32 = 7.0;
/// `.featured-example{min-height:133px}`, radius 14, padding 12; the
/// left text column takes 44 %, the art 56 %.
const EXAMPLE_CARD_H: f32 = 133.0;
const EXAMPLE_LEFT_FRAC: f32 = 0.44;
/// The send button's fixed width (`.primary{padding:0 19px}` around a
/// 13 px label and its ↗ arrow).
const SEND_W: f32 = 132.0;
/// `.mode-switch`: one 44 pt control (44 = 2 borders + 2×2 padding +
/// 38 px buttons), 2 px gaps, 48 px minimum per segment
/// (`min-width:43px`, widened to the touch floor), radius 10 / 7.
const MODE_SEG_H: f32 = 44.0;
const MODE_SEG_MIN_W: f32 = 48.0;
const MODE_SEG_GAP: f32 = 2.0;
/// `.mobile-header` paddings (`professional.css` overrides): 16 left
/// (the brand's own x in the compact painter), 10 right, 6 gap.
const HEADER_PAD_RIGHT: f32 = 10.0;
const HEADER_GAP: f32 = 6.0;
/// The settings gear: a 44 pt square icon target.
const GEAR_W: f32 = 44.0;

/// Resolve the compact rect set (see [`HomeLayout`]). The top bar and
/// the bottom nav stay pinned; the page column between them scrolls.
#[allow(clippy::too_many_lines)]
pub(super) fn compact_layout_for_scrolled(
    viewport_width: f32,
    viewport_height: f32,
    task: HomeFamily,
    scroll_y: f32,
    model_chip_label_w: f32,
) -> HomeLayout {
    let width = viewport_width.max(1.0);
    let height = viewport_height.max(1.0);
    let scroll = -scroll_y.max(0.0);
    let content_x = PAGE_PAD_X;
    let content_w = (width - PAGE_PAD_X * 2.0).max(200.0);
    let translate = |y: f32, h: f32| Rect::xywh(content_x, y + scroll, content_w, h);
    let locale = self_locale();

    // ── pinned top bar: brand | … | 普通/专业 | gear ─────────────────
    let gear = Rect::xywh(
        width - HEADER_PAD_RIGHT - GEAR_W,
        (HOME_TOPBAR_H - GEAR_W) / 2.0,
        GEAR_W,
        GEAR_W,
    );
    let normal_w = (copy::estimate_text_w(copy::home_str(locale, "home.mode.normal"), 12.0) + 16.0)
        .max(MODE_SEG_MIN_W);
    let professional_w =
        (copy::estimate_text_w(copy::home_str(locale, "home.mode.professional"), 12.0) + 16.0)
            .max(MODE_SEG_MIN_W);
    let mode_switch = Rect::xywh(
        gear.origin.x - HEADER_GAP - normal_w - MODE_SEG_GAP - professional_w,
        (HOME_TOPBAR_H - MODE_SEG_H) / 2.0,
        normal_w + MODE_SEG_GAP + professional_w,
        MODE_SEG_H,
    );
    let mode_normal = Rect::xywh(
        mode_switch.origin.x,
        mode_switch.origin.y,
        normal_w,
        MODE_SEG_H,
    );
    // The 专业 segment reuses the desktop's `professional` hit rect so
    // both compositions answer the same HomeHit.
    let professional = Rect::xywh(
        mode_switch.origin.x + normal_w + MODE_SEG_GAP,
        mode_switch.origin.y,
        professional_w,
        MODE_SEG_H,
    );
    // The phone top bar carries no account avatar and no 打开文件
    // button; the account and file entries live in the canvas More
    // sheet and the bottom nav's settings page.
    let open_file = Rect::ZERO;
    let account = Rect::ZERO;
    let settings = gear;

    // ── hero ─────────────────────────────────────────────────────────
    let welcome = translate(HOME_TOPBAR_H + PAGE_PAD_TOP, HERO_H1_H);
    let welcome_sub = translate(
        welcome.origin.y - scroll + HERO_H1_H + HERO_SUB_GAP,
        HERO_SUB_H,
    );

    // ── the 2×4 task grid (7 tasks + the weakened blank tile) ────────
    let grid_y = welcome_sub.origin.y - scroll + HERO_SUB_H + HERO_MARGIN_BOTTOM;
    let tile_w = (content_w - GRID_GAP * (GRID_COLS as f32 - 1.0)) / GRID_COLS as f32;
    let tabs_row = translate(grid_y, TILE_H * 2.0 + GRID_GAP);
    let mut tabs = [Rect::ZERO; 7];
    for (index, slot) in tabs.iter_mut().enumerate() {
        let (row, col) = (index / GRID_COLS, index % GRID_COLS);
        *slot = Rect::xywh(
            content_x + col as f32 * (tile_w + GRID_GAP),
            grid_y + scroll + row as f32 * (TILE_H + GRID_GAP),
            tile_w,
            TILE_H,
        );
    }
    // The eighth slot (row 1, col 3) is the weakened 空白画布 tile.
    let new_canvas = Rect::xywh(
        content_x + 3.0 * (tile_w + GRID_GAP),
        grid_y + scroll + TILE_H + GRID_GAP,
        tile_w,
        TILE_H,
    );

    // ── the compact composer card ────────────────────────────────────
    let composer_y = grid_y + TILE_H * 2.0 + GRID_GAP + GRID_MARGIN_BOTTOM;
    let composer_h = COMPOSER_PAD_TOP
        + LABEL_ROW_H
        + LABEL_ROW_GAP
        + INPUT_H
        + INPUT_TOOLS_GAP
        + TOOLS_H
        + SUBMIT_H
        + COMPOSER_PAD_BOTTOM;
    let composer = translate(composer_y, composer_h);
    let inner_x = composer.origin.x + COMPOSER_PAD_X;
    let inner_w = composer.size.x - COMPOSER_PAD_X * 2.0;
    let label_row = Rect::xywh(
        inner_x,
        composer.origin.y + COMPOSER_PAD_TOP,
        inner_w,
        LABEL_ROW_H,
    );
    // The task's segmented options right of the label, sized to their
    // copy at the compact 11 px (prototype `.mobile-options button`).
    let labels = copy::segment_labels(locale, task);
    let mut option_widths = [0.0f32; 3];
    for (index, label) in labels.iter().take(3).enumerate() {
        option_widths[index] = copy::estimate_text_w(label, 11.0) + 18.0;
    }
    let segment_w: f32 = option_widths.iter().take(labels.len()).sum::<f32>() + 4.0;
    let segment = if labels.is_empty() {
        Rect::ZERO
    } else {
        Rect::xywh(
            inner_x + inner_w - segment_w,
            label_row.origin.y,
            segment_w,
            LABEL_ROW_H,
        )
    };
    let mut segment_options = [Rect::ZERO; 3];
    let mut option_x = segment.origin.x + 2.0;
    for index in 0..labels.len().min(3) {
        // Options fill the 44 pt row — the prototype's real-device rule
        // (`min-height:44px`) and the touch floor agree.
        segment_options[index] = Rect::xywh(
            option_x,
            segment.origin.y,
            option_widths[index],
            LABEL_ROW_H,
        );
        option_x += option_widths[index];
    }
    let input_box = Rect::xywh(
        inner_x,
        label_row.origin.y + LABEL_ROW_H + LABEL_ROW_GAP,
        inner_w,
        INPUT_H,
    );
    let tools_row = Rect::xywh(
        inner_x,
        input_box.origin.y + INPUT_H + INPUT_TOOLS_GAP,
        inner_w,
        TOOLS_H,
    );
    // 加图片 / 加链接: icon 17 + 5 gap + label (11 px).
    let screenshot = Rect::xywh(
        tools_row.origin.x,
        tools_row.origin.y,
        copy::estimate_text_w(copy::home_str(locale, "home.tools.screenshot"), 11.0) + 17.0 + 5.0,
        TOOLS_H,
    );
    let reference_link = Rect::xywh(
        screenshot.origin.x + screenshot.size.x + 17.0,
        tools_row.origin.y,
        copy::estimate_text_w(copy::home_str(locale, "home.tools.link"), 11.0) + 17.0 + 5.0,
        TOOLS_H,
    );
    let figma = Rect::ZERO;
    let submit_row = Rect::xywh(inner_x, tools_row.origin.y + TOOLS_H, inner_w, SUBMIT_H);
    let send = Rect::xywh(
        submit_row.origin.x + submit_row.size.x - SEND_W,
        submit_row.origin.y,
        SEND_W,
        SUBMIT_H,
    );
    let model_chip = Rect::xywh(
        submit_row.origin.x,
        submit_row.origin.y,
        model_chip_label_w.min(submit_row.size.x - SEND_W - 10.0),
        MODEL_CHIP_H + 6.0,
    );
    let replace_strip = Rect::xywh(
        input_box.origin.x + 8.0,
        input_box.origin.y + input_box.size.y - 40.0,
        input_box.size.x - 16.0,
        34.0,
    );
    let replace_keep = Rect::xywh(
        replace_strip.origin.x + replace_strip.size.x - 158.0,
        replace_strip.origin.y,
        74.0,
        34.0,
    );
    let replace_use = Rect::xywh(
        replace_strip.origin.x + replace_strip.size.x - 80.0,
        replace_strip.origin.y,
        80.0,
        34.0,
    );

    // ── the single featured example ──────────────────────────────────
    let heading_y = composer_y + composer_h + EXAMPLE_TOP_GAP;
    let explore_heading = translate(heading_y, EXAMPLE_HEADING_H);
    let preview = translate(
        heading_y + EXAMPLE_HEADING_H + EXAMPLE_HEADING_GAP,
        EXAMPLE_CARD_H,
    );
    let card_pad = 12.0;
    let left_w = (preview.size.x * EXAMPLE_LEFT_FRAC).floor();
    let preview_heading = Rect::xywh(
        preview.origin.x + card_pad,
        preview.origin.y + card_pad,
        (left_w - card_pad).max(40.0),
        EXAMPLE_CARD_H - card_pad * 2.0,
    );
    let preview_art = Rect::xywh(
        preview.origin.x + left_w,
        preview.origin.y + card_pad,
        (preview.size.x - left_w - card_pad).max(40.0),
        EXAMPLE_CARD_H - card_pad * 2.0,
    );
    // The whole card is the example's tap target (prototype
    // `#mobile-use-example` is the card-sized button).
    let use_example = preview;
    let preview_footer = Rect::xywh(
        preview_heading.origin.x,
        preview.origin.y + EXAMPLE_CARD_H - card_pad - 30.0,
        preview_heading.size.x,
        30.0,
    );

    // ── the pinned bottom nav ────────────────────────────────────────
    let bottom_nav = Rect::xywh(0.0, height - BOTTOM_NAV_H, width, BOTTOM_NAV_H);
    let item_w = width / 3.0;
    let nav_items = [
        Rect::xywh(0.0, bottom_nav.origin.y, item_w, BOTTOM_NAV_H),
        Rect::xywh(item_w, bottom_nav.origin.y, item_w, BOTTOM_NAV_H),
        Rect::xywh(item_w * 2.0, bottom_nav.origin.y, item_w, BOTTOM_NAV_H),
    ];

    let (connect_card, connect_rows) = connect_card_rects(composer);

    HomeLayout {
        open_file,
        account,
        professional,
        mode_switch,
        mode_normal,
        settings,
        welcome,
        welcome_sub,
        tabs_row,
        tabs,
        more_button: Rect::ZERO,
        more_popover: Rect::ZERO,
        more_rows: [Rect::ZERO; 4],
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
        explore_cards: [Rect::ZERO; 3],
        recent: Rect::ZERO,
        recent_chips: [Rect::ZERO; 5],
        new_canvas,
        connect_card,
        connect_rows,
        bottom_nav,
        nav_items,
    }
}

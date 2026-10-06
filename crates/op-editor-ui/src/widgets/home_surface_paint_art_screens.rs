//! The three coffee-app phone screens (首页 / 菜单 / 订单) for the
//! App task's example art — the per-screen content painted inside the
//! phone chrome from `home_surface_paint_art.rs`. Every size derives
//! from the phone width via `cqw` units, mirroring the prototype's
//! `artwork.css` container queries.

use super::super::fade;
use super::art::{draw_coffee_photo, PhoneScreen, PHONE_REFERENCE_W};
use super::cards::{text, text_weighted};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

/// Screen 1 首页: greeting, two-line title, search pill, coffee hero
/// and the classic-latte price strip (`scenes.js` `home`).
#[allow(clippy::too_many_lines)]
pub(super) fn paint_home_screen(cx: &mut PaintCx<'_>, frame: &PhoneScreen<'_>) {
    let PhoneScreen {
        phone,
        inner_x,
        inner_w,
        content_bottom,
        status_h,
        inks,
        alpha,
    } = *frame;
    let w = phone.size.x;
    let q = |cqw: f32| cqw * w / 100.0;
    let s = w / PHONE_REFERENCE_W;
    let mut y = phone.origin.y + status_h + q(2.0);
    text(
        cx,
        "Good Morning",
        Point2D::new(inner_x, y + q(4.0)),
        q(4.0),
        fade(Color::rgb_u8(0xA3, 0x83, 0x6A), alpha),
    );
    y += q(7.0);
    for line in ["一杯好咖啡", "开启美好的一天"] {
        text_weighted(
            cx,
            line,
            Point2D::new(inner_x, y + q(8.6)),
            q(8.6),
            inks.ink,
            700,
        );
        y += q(12.5);
    }
    y += q(6.0);
    // Search pill.
    let search_h = q(11.0);
    cx.backend.fill_round_rect(
        Rect::xywh(inner_x, y, inner_w, search_h),
        search_h / 2.0,
        fade(Color::WHITE, alpha),
    );
    draw_icon(
        cx.backend,
        Icon::Search,
        Point2D::new(inner_x + q(3.0), y + (search_h - q(4.5)) / 2.0),
        q(4.5),
        fade(Color::rgb_u8(0xAC, 0xAB, 0xA8), alpha),
        1.6,
    );
    text(
        cx,
        "搜索你喜欢的咖啡",
        Point2D::new(inner_x + q(9.0), y + search_h / 2.0 + q(1.4)),
        q(3.8),
        fade(Color::rgb_u8(0xAC, 0xAB, 0xA8), alpha),
    );
    y += search_h + q(5.0);
    // Price strip: fixed-height block the hero sits on top of.
    let price_h = q(43.0);
    let hero = Rect::xywh(
        inner_x,
        y,
        inner_w,
        (content_bottom - price_h - y).max(q(20.0)),
    );
    cx.backend.save();
    cx.backend
        .clip_round_rect_per_corner(hero, [7.0 * s, 7.0 * s, 0.0, 0.0]);
    if !draw_coffee_photo(cx.backend, hero, alpha, 0.0) {
        cx.backend
            .fill_rect(hero, fade(Color::rgb_u8(0xF3, 0xE9, 0xDD), alpha));
    }
    cx.backend.restore();
    let strip = Rect::xywh(inner_x, content_bottom - price_h, inner_w, price_h);
    let cream = fade(Color::rgb_u8(0xF6, 0xEA, 0xDC), alpha);
    cx.backend
        .fill_round_rect_per_corner(strip, [0.0, 0.0, 8.0 * s, 8.0 * s], cream);
    let pad = q(5.0);
    text_weighted(
        cx,
        "经典拿铁",
        Point2D::new(strip.origin.x + pad, strip.origin.y + pad + q(6.0)),
        q(6.0),
        inks.ink,
        700,
    );
    text(
        cx,
        "Classic Latte",
        Point2D::new(
            strip.origin.x + pad,
            strip.origin.y + pad + q(6.0) + q(2.0) + q(4.0),
        ),
        q(4.0),
        fade(Color::rgb_u8(0x8C, 0x77, 0x68), alpha),
    );
    let row_y = strip.origin.y + pad + q(6.0) + q(2.0) + q(4.0) + q(2.0);
    text_weighted(
        cx,
        "¥28",
        Point2D::new(strip.origin.x + pad, row_y + q(11.0)),
        q(8.0),
        inks.ink,
        700,
    );
    let circle = q(15.0);
    let circle_rect = Rect::xywh(
        strip.origin.x + strip.size.x - pad - circle,
        row_y,
        circle,
        circle,
    );
    cx.backend
        .fill_oval(circle_rect, fade(Color::rgb_u8(0x3D, 0x2B, 0x1F), alpha));
    draw_icon(
        cx.backend,
        Icon::ArrowUpRight,
        Point2D::new(
            circle_rect.origin.x + (circle - q(7.0)) / 2.0,
            circle_rect.origin.y + (circle - q(7.0)) / 2.0,
        ),
        q(7.0),
        fade(Color::WHITE, alpha),
        2.0,
    );
}

/// Screen 2 菜单: chip row plus five product rows with coffee thumbs
/// and blue + circles (`scenes.js` `menu`).
#[allow(clippy::too_many_lines)]
pub(super) fn paint_menu_screen(cx: &mut PaintCx<'_>, frame: &PhoneScreen<'_>) {
    let PhoneScreen {
        phone,
        inner_x,
        inner_w,
        content_bottom,
        status_h,
        inks,
        alpha,
    } = *frame;
    let w = phone.size.x;
    let q = |cqw: f32| cqw * w / 100.0;
    let s = w / PHONE_REFERENCE_W;
    let mut y = phone.origin.y + status_h + q(4.0);
    text_weighted(
        cx,
        "菜单",
        Point2D::new(inner_x, y + q(9.0)),
        q(9.0),
        inks.ink,
        700,
    );
    y += q(9.0) + q(7.0);
    // Chip row: 全所有 a blue pill, the rest grey squares.
    let chip_h = q(11.0);
    let labels = ["全部", "咖啡", "茶饮", "轻食"];
    let chip_ws: [f32; 4] = [
        q(4.4) * 2.0 + q(10.0),
        q(4.4) * 2.0 + q(4.0),
        q(4.4) * 2.0 + q(4.0),
        q(4.4) * 2.0 + q(4.0),
    ];
    let gap_sum = inner_w - chip_ws.iter().sum::<f32>();
    let chip_gap = gap_sum / 3.0;
    let mut chip_x = inner_x;
    for (index, label) in labels.into_iter().enumerate() {
        let chip = Rect::xywh(chip_x, y, chip_ws[index], chip_h);
        if index == 0 {
            cx.backend.fill_round_rect(chip, chip_h / 2.0, inks.blue);
            text(
                cx,
                label,
                Point2D::new(
                    chip.origin.x + q(5.0),
                    chip.origin.y + chip_h / 2.0 + q(1.6),
                ),
                q(4.4),
                fade(Color::WHITE, alpha),
            );
        } else {
            cx.backend.fill_round_rect(
                chip,
                10.0 * s,
                fade(Color::rgb_u8(0xEF, 0xEE, 0xEA), alpha),
            );
            text(
                cx,
                label,
                Point2D::new(
                    chip.origin.x + q(2.0),
                    chip.origin.y + chip_h / 2.0 + q(1.6),
                ),
                q(4.4),
                fade(Color::rgb_u8(0x7E, 0x7D, 0x7B), alpha),
            );
        }
        chip_x += chip_ws[index] + chip_gap;
    }
    y += chip_h + q(5.0);
    // Five rows share the remaining height (`justify-space-around`).
    let rows = [
        ("美式咖啡", "Americano", "¥22", -40.0f32),
        ("拿铁", "Caffè Latte", "¥28", 0.0),
        ("卡布奇诺", "Cappuccino", "¥26", -30.0),
        ("焦糖玛奇朵", "Caramel Macchiato", "¥32", -15.0),
        ("摩卡", "Mocha", "¥30", 0.0),
    ];
    let row_h = (content_bottom - y) / rows.len() as f32;
    let thumb = q(22.0);
    for (index, (name, english, price, tone)) in rows.into_iter().enumerate() {
        let row_y = y + index as f32 * row_h;
        let centre_y = row_y + row_h / 2.0;
        let thumb_rect = Rect::xywh(inner_x, centre_y - thumb / 2.0, thumb, thumb);
        cx.backend.fill_round_rect(
            thumb_rect,
            4.0 * s,
            fade(Color::rgb_u8(0xF3, 0xE9, 0xDD), alpha),
        );
        if !draw_coffee_photo(cx.backend, thumb_rect, alpha, tone) {
            cx.backend.fill_round_rect(
                thumb_rect,
                4.0 * s,
                fade(Color::rgb_u8(0xE0, 0xCF, 0xBA), alpha),
            );
        }
        let text_x = thumb_rect.origin.x + thumb + q(3.0);
        text_weighted(
            cx,
            name,
            Point2D::new(text_x, centre_y - q(3.0)),
            q(5.0),
            inks.ink,
            600,
        );
        text(
            cx,
            english,
            Point2D::new(text_x, centre_y + q(2.5)),
            q(3.5),
            fade(Color::rgb_u8(0x7E, 0x7E, 0x7E), alpha),
        );
        text_weighted(
            cx,
            price,
            Point2D::new(text_x, centre_y + q(8.5)),
            q(5.0),
            inks.ink,
            600,
        );
        let plus = q(8.0);
        cx.backend.fill_oval(
            Rect::xywh(inner_x + inner_w - plus, centre_y - plus / 2.0, plus, plus),
            inks.blue,
        );
        draw_icon(
            cx.backend,
            Icon::Plus,
            Point2D::new(
                inner_x + inner_w - plus + (plus - q(4.5)) / 2.0,
                centre_y - q(2.25),
            ),
            q(4.5),
            fade(Color::WHITE, alpha),
            2.2,
        );
    }
}

/// Screen 3 订单: pickup tabs, tilted bag, waiting note and the
/// receipt card with its 4-stop progress track (`scenes.js` `order`).
#[allow(clippy::too_many_lines)]
pub(super) fn paint_order_screen(cx: &mut PaintCx<'_>, frame: &PhoneScreen<'_>) {
    let PhoneScreen {
        phone,
        inner_x,
        inner_w,
        content_bottom,
        status_h,
        inks,
        alpha,
    } = *frame;
    let w = phone.size.x;
    let q = |cqw: f32| cqw * w / 100.0;
    let s = w / PHONE_REFERENCE_W;
    let centre_x = inner_x + inner_w / 2.0;
    let mut y = phone.origin.y + status_h + q(4.0);
    text_weighted(
        cx,
        "订单",
        Point2D::new(inner_x, y + q(9.0)),
        q(9.0),
        inks.ink,
        700,
    );
    y += q(9.0) + q(7.0);
    // Pickup / history tabs on a grey track.
    let tabs_h = q(11.0);
    let track = Rect::xywh(inner_x, y, inner_w, tabs_h);
    cx.backend.fill_round_rect(
        track,
        10.0 * s,
        fade(Color::rgb_u8(0xF2, 0xF2, 0xF1), alpha),
    );
    let active_w = q(4.5) * 2.0 + q(20.0);
    let active = Rect::xywh(
        track.origin.x + q(2.0),
        track.origin.y + q(2.0),
        active_w,
        tabs_h - q(4.0),
    );
    cx.backend
        .fill_round_rect(active, 10.0 * s, fade(Color::WHITE, alpha));
    text(
        cx,
        "待取单",
        Point2D::new(
            active.origin.x + q(10.0),
            active.origin.y + active.size.y / 2.0 + q(1.7),
        ),
        q(4.5),
        fade(Color::rgb_u8(0x44, 0x44, 0x44), alpha),
    );
    text(
        cx,
        "历史订单",
        Point2D::new(
            centre_x + q(8.0),
            active.origin.y + active.size.y / 2.0 + q(1.7),
        ),
        q(4.5),
        fade(Color::rgb_u8(0x9B, 0x9D, 0xA3), alpha),
    );
    y += tabs_h + q(11.0);
    // Tilted paper bag.
    let bag = Rect::xywh(centre_x - q(13.0), y, q(26.0), q(30.0));
    cx.backend.save();
    cx.backend.rotate(
        -6.0_f32.to_radians(),
        Point2D::new(
            bag.origin.x + bag.size.x / 2.0,
            bag.origin.y + bag.size.y / 2.0,
        ),
    );
    cx.backend
        .fill_round_rect(bag, 3.0 * s, fade(Color::rgb_u8(0xF6, 0xD3, 0xA4), alpha));
    draw_icon(
        cx.backend,
        Icon::ShoppingBag,
        Point2D::new(
            bag.origin.x + (bag.size.x - q(17.0)) / 2.0,
            bag.origin.y + (bag.size.y - q(17.0)) / 2.0,
        ),
        q(17.0),
        fade(Color::rgb_u8(0x99, 0x75, 0x45), alpha),
        1.6,
    );
    cx.backend.restore();
    y += q(30.0) + q(8.0);
    text_weighted(
        cx,
        "你的订单很快就好",
        Point2D::new(centre_x - q(21.0), y + q(6.0)),
        q(6.0),
        inks.ink,
        600,
    );
    y += q(6.0) + q(7.0);
    let note = fade(Color::rgb_u8(0xB0, 0xA9, 0xA2), alpha);
    for line in ["我们正在为你制作", "请耐心等候～"] {
        text(
            cx,
            line,
            Point2D::new(centre_x - q(13.5), y + q(4.5)),
            q(4.5),
            note,
        );
        y += q(7.6);
    }
    y += q(4.0);
    // Receipt card: item row + 4-stop progress track.
    let receipt_h = content_bottom - y;
    let receipt = Rect::xywh(inner_x, y, inner_w, receipt_h);
    cx.backend
        .fill_round_rect(receipt, 5.0 * s, fade(Color::WHITE, alpha));
    let pad = q(3.0);
    let thumb_h = q(24.0);
    let thumb = q(22.0);
    let thumb_rect = Rect::xywh(
        receipt.origin.x + pad,
        receipt.origin.y + q(5.0),
        thumb,
        thumb_h,
    );
    cx.backend.fill_round_rect(
        thumb_rect,
        4.0 * s,
        fade(Color::rgb_u8(0xF3, 0xE9, 0xDD), alpha),
    );
    draw_coffee_photo(cx.backend, thumb_rect, alpha, 0.0);
    let text_x = thumb_rect.origin.x + thumb + q(3.0);
    text_weighted(
        cx,
        "拿铁",
        Point2D::new(text_x, receipt.origin.y + q(5.0) + q(5.0)),
        q(5.0),
        inks.ink,
        600,
    );
    text(
        cx,
        "大杯 / 冰 / 标准",
        Point2D::new(text_x, receipt.origin.y + q(5.0) + q(10.5)),
        q(3.5),
        fade(Color::rgb_u8(0x7E, 0x7E, 0x7E), alpha),
    );
    text_weighted(
        cx,
        "¥28",
        Point2D::new(text_x, receipt.origin.y + q(5.0) + q(16.5)),
        q(5.0),
        inks.ink,
        600,
    );
    text(
        cx,
        "制作中",
        Point2D::new(
            receipt.origin.x + receipt.size.x - pad - q(10.0),
            receipt.origin.y + q(5.0) + q(5.0),
        ),
        q(4.0),
        inks.blue,
    );
    // Track: line 40 % blue, four dots, four labels.
    let track_y = receipt.origin.y + receipt.size.y - q(15.0);
    let track_left = receipt.origin.x + q(2.0);
    let track_right = receipt.origin.x + receipt.size.x - q(2.0);
    let track_h = q(3.0);
    cx.backend.fill_round_rect(
        Rect::xywh(track_left, track_y, track_right - track_left, track_h),
        track_h / 2.0,
        fade(Color::rgb_u8(0xEC, 0xED, 0xEF), alpha),
    );
    cx.backend.fill_round_rect(
        Rect::xywh(
            track_left,
            track_y,
            (track_right - track_left) * 0.4,
            track_h,
        ),
        track_h / 2.0,
        inks.blue,
    );
    let dot = q(6.0);
    let dot_grey = fade(Color::rgb_u8(0xD7, 0xDC, 0xE5), alpha);
    let labels = ["已接单", "制作中", "待取餐", "完成"];
    let step = (track_right - track_left - dot) / 3.0;
    for (index, label) in labels.iter().enumerate() {
        let dot_x = track_left + index as f32 * step;
        cx.backend.fill_oval(
            Rect::xywh(
                dot_x + 1.0,
                track_y + track_h / 2.0 - dot / 2.0 + 1.0,
                dot - 2.0,
                dot - 2.0,
            ),
            fade(Color::WHITE, alpha),
        );
        cx.backend.stroke_oval(
            Rect::xywh(dot_x, track_y + track_h / 2.0 - dot / 2.0, dot, dot),
            if index < 2 { inks.blue } else { dot_grey },
            2.0 * s,
        );
        text(
            cx,
            label,
            Point2D::new(dot_x + dot / 2.0 - q(4.5), track_y + dot + q(4.0)),
            q(3.4),
            fade(Color::rgb_u8(0xAA, 0xAA, 0xAA), alpha),
        );
    }
}

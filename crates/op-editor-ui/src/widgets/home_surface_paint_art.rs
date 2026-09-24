//! The App task's rich example artwork: the embedded coffee demo
//! photo, the phone chrome, and the desktop counter window, redrawn
//! from the prototype's `scenes.js`, `artwork.css`, and
//! `expanded-scenes.js`. The three phone screens' content lives in
//! `home_surface_paint_art_screens.rs`. Sizes derive from the
//! phone/window width the way the prototype's `cqw` units do.

use super::super::fade;
use super::art_copy::{art, text_fit, text_fit_centred, text_fit_right};
use super::cards::text_weighted;
use super::StudioPalette;
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, ImageAdjustments, ImageDrawMode, Point2D, Rect, RenderBackend};
use op_editor_core::HomeDevice;
use op_i18n::Locale;

/// Stable cache id for the embedded demo photo (mirrors the login
/// modal's brand-logo id policy).
const COFFEE_DEMO_IMAGE_ID: u64 = 0x484f_4d45_4346_4631;
const COFFEE_DEMO_JPG: &[u8] = include_bytes!("../../assets/home_examples/coffee-demo.jpg");

/// The prototype drew its phones ~162 px wide; the fixed-px chrome
/// (border, radii, notch) scales by the ratio to that reference.
pub(super) const PHONE_REFERENCE_W: f32 = 162.0;

/// Draw the demo coffee photo, fill-cropped into `rect`.
/// `saturation` uses the image-adjustment slider scale (0 = neutral,
/// CSS `saturate(.6)` ≈ −40). Returns `false` while the first decode
/// is still in flight so the caller can paint its placeholder.
pub(super) fn draw_coffee_photo(
    backend: &mut dyn RenderBackend,
    rect: Rect,
    opacity: f32,
    saturation: f32,
) -> bool {
    if !has_cached_image_bytes(COFFEE_DEMO_IMAGE_ID) {
        store_remote_image_bytes(COFFEE_DEMO_IMAGE_ID, COFFEE_DEMO_JPG.to_vec());
    }
    let max_edge = required_raster_edge(rect, backend.dpi_scale());
    let sharp = backend.image_decoded(COFFEE_DEMO_IMAGE_ID, COFFEE_DEMO_JPG, max_edge);
    if !sharp {
        note_pending_decode(COFFEE_DEMO_IMAGE_ID, max_edge);
    }
    if sharp || backend.image_resident(COFFEE_DEMO_IMAGE_ID) {
        let adjustments = ImageAdjustments {
            saturation,
            ..ImageAdjustments::default()
        };
        backend.draw_image_with_options(
            rect,
            COFFEE_DEMO_IMAGE_ID,
            COFFEE_DEMO_JPG,
            ImageDrawMode::Fill,
            adjustments,
            opacity,
            0.0,
        );
        true
    } else {
        false
    }
}

/// The App task's art: three phone mock-ups (mobile) or one desktop
/// counter window (desktop), scaled to fill `area` (prototype
/// `phones()` / `desktopApplication()`). `phase` is the art-switch
/// progress: opacity .2→1, rise 8 px, scale .985→1. Every string on
/// the mock-ups is `locale`'s (see `home_surface_paint_art_copy.rs`).
pub(super) fn paint_app_art(
    cx: &mut PaintCx<'_>,
    area: Rect,
    _palette: StudioPalette,
    device: HomeDevice,
    phase: f32,
    locale: Locale,
) {
    let dy = (1.0 - phase) * 8.0;
    let alpha = 0.2 + 0.8 * phase;
    let scale = 0.985 + 0.015 * phase;
    let centre = Point2D::new(
        area.origin.x + area.size.x / 2.0,
        area.origin.y + area.size.y / 2.0,
    );
    cx.backend.save();
    cx.backend.scale(Point2D::new(scale, scale), centre);
    let area = Rect::xywh(area.origin.x, area.origin.y + dy, area.size.x, area.size.y);
    match device {
        HomeDevice::Mobile => {
            // `.phones`: aspect .475 phones, 16 px gap, ≤344 px tall.
            let gap = 16.0;
            let phone_h = (area.size.y - 2.0).min(344.0);
            let phone_w = phone_h * 0.475;
            let total = phone_w * 3.0 + gap * 2.0;
            let start_x = area.origin.x + (area.size.x - total) / 2.0;
            let y = area.origin.y + (area.size.y - phone_h).max(0.0) / 2.0;
            for screen in 0..3 {
                let x = start_x + screen as f32 * (phone_w + gap);
                paint_phone(
                    cx,
                    Rect::xywh(x, y, phone_w, phone_h),
                    screen,
                    alpha,
                    locale,
                );
            }
        }
        HomeDevice::Desktop => {
            // `.counter-app`: aspect 1.5 window.
            let win_h = (area.size.y - 2.0).min(260.0);
            let win_w = (win_h * 1.5).min(area.size.x - 4.0);
            let win_h = win_w / 1.5;
            let x = area.origin.x + (area.size.x - win_w) / 2.0;
            let y = area.origin.y + (area.size.y - win_h).max(0.0) / 2.0;
            paint_counter_window(cx, Rect::xywh(x, y, win_w, win_h), alpha, locale);
        }
    }
    cx.backend.restore();
}

/// Pre-faded demo-art inks: the mock-ups keep the prototype's literal
/// palette at full alpha; the switch fade washes them together.
pub(super) struct PhoneInks {
    pub(super) ink: Color,
    pub(super) muted: Color,
    pub(super) blue: Color,
}

/// Everything one phone screen needs to paint its content: the phone
/// rect, the inner column, the content bottom edge (above the nav),
/// the status row height, the pre-faded inks, and the switch alpha.
pub(super) struct PhoneScreen<'a> {
    pub(super) phone: Rect,
    pub(super) inner_x: f32,
    pub(super) inner_w: f32,
    pub(super) content_bottom: f32,
    pub(super) status_h: f32,
    pub(super) inks: &'a PhoneInks,
    pub(super) alpha: f32,
    pub(super) locale: Locale,
}

pub(super) fn phone_inks(alpha: f32) -> PhoneInks {
    PhoneInks {
        ink: fade(Color::rgb_u8(0x20, 0x1B, 0x18), alpha),
        muted: fade(Color::rgb_u8(0x9A, 0xA3, 0xB0), alpha),
        blue: fade(Color::rgb_u8(0x07, 0x5B, 0xFF), alpha),
    }
}

#[allow(clippy::too_many_lines)]
fn paint_phone(cx: &mut PaintCx<'_>, phone: Rect, screen: u8, alpha: f32, locale: Locale) {
    let w = phone.size.x;
    let q = |cqw: f32| cqw * w / 100.0;
    let s = w / PHONE_REFERENCE_W;
    let inks = phone_inks(alpha);
    // `.phone`: 0 2px 0 #85888a + 0 4px 7px #31466525 under a #fffaf5
    // body with a 3.5 px #202327 border.
    cx.backend.fill_drop_shadow(
        Rect::xywh(
            phone.origin.x + 2.0 * s,
            phone.origin.y + 4.0 * s,
            w - 4.0 * s,
            phone.size.y - 4.0 * s,
        ),
        23.0 * s,
        7.0,
        fade(Color::rgb_u8(0x31, 0x46, 0x65), 0.15 * alpha),
    );
    let body = Color::rgb_u8(0xFF, 0xFA, 0xF5);
    cx.backend
        .fill_round_rect(phone, 23.0 * s, fade(body, alpha));
    cx.backend.stroke_round_rect(
        phone,
        23.0 * s,
        fade(Color::rgb_u8(0x20, 0x23, 0x27), alpha),
        3.5 * s,
    );

    let status_h = q(16.0);
    let nav_h = q(23.0);
    let inner_x = phone.origin.x + q(6.0);
    let inner_w = w - q(12.0);
    let content_bottom = phone.origin.y + phone.size.y - nav_h;

    // Screen content clips inside the bezel.
    cx.backend.save();
    cx.backend.clip_round_rect(
        Rect::xywh(
            phone.origin.x + 3.5 * s,
            phone.origin.y + 3.5 * s,
            w - 7.0 * s,
            phone.size.y - 7.0 * s,
        ),
        20.0 * s,
    );
    let frame = PhoneScreen {
        phone,
        inner_x,
        inner_w,
        content_bottom,
        status_h,
        inks: &inks,
        alpha,
        locale,
    };
    match screen {
        0 => super::art_screens::paint_home_screen(cx, &frame),
        1 => super::art_screens::paint_menu_screen(cx, &frame),
        _ => super::art_screens::paint_order_screen(cx, &frame),
    }

    // Status row: "9:41" left, abstract signal glyphs right.
    text_weighted(
        cx,
        "9:41",
        Point2D::new(
            phone.origin.x + q(8.0),
            phone.origin.y + status_h / 2.0 + q(1.6),
        ),
        q(4.4),
        inks.ink,
        700,
    );
    let glyph_y = phone.origin.y + status_h / 2.0 - q(1.1);
    for (index, half_w) in [1.6f32, 0.8, 1.6].into_iter().enumerate() {
        cx.backend.fill_round_rect(
            Rect::xywh(
                phone.origin.x + w - q(8.0) - q(7.0) + index as f32 * q(2.6),
                glyph_y,
                q(half_w),
                q(2.2),
            ),
            q(0.5),
            inks.muted,
        );
    }
    // Notch bar over the status row's centre.
    cx.backend.fill_round_rect(
        Rect::xywh(
            phone.origin.x + w * 0.37,
            phone.origin.y + 5.0 * s,
            w * 0.26,
            6.0 * s,
        ),
        5.0 * s,
        fade(Color::rgb_u8(0x18, 0x1B, 0x20), alpha),
    );

    // Bottom nav: four items, the screen's own in blue.
    let nav_y = content_bottom;
    cx.backend.fill_rect(
        Rect::xywh(phone.origin.x, nav_y, w, nav_h + q(4.0)),
        fade(Color::WHITE, alpha),
    );
    let items: [(Icon, &str); 4] = [
        (Icon::Home, art(locale, "home.art.navHome")),
        (Icon::LayoutGrid, art(locale, "home.art.menu")),
        (Icon::ShoppingBag, art(locale, "home.art.orders")),
        (Icon::User, art(locale, "home.art.navMe")),
    ];
    let item_w = w / 4.0;
    for (index, (icon, label)) in items.into_iter().enumerate() {
        let on = index as u8 == screen;
        let color = if on { inks.blue } else { inks.muted };
        let centre_x = phone.origin.x + item_w * (index as f32 + 0.5);
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(centre_x - q(4.0), nav_y + q(4.0)),
            q(8.0),
            color,
            1.6,
        );
        text_fit_centred(
            cx,
            label,
            centre_x,
            nav_y + q(15.0),
            q(4.0),
            item_w - q(2.0),
            color,
            400,
        );
    }
    cx.backend.restore();
}

/// The desktop variant: a Daybreak 门店工作台 counter window with a
/// sidebar, a 2×2 stat grid and four in-flight orders
/// (`expanded-scenes.js` `desktopApplication`).
#[allow(clippy::too_many_lines)]
fn paint_counter_window(cx: &mut PaintCx<'_>, window: Rect, alpha: f32, locale: Locale) {
    let ww = window.size.x;
    let q = |cqw: f32| cqw * ww / 100.0;
    let s = ww / 360.0;
    let ink = fade(Color::rgb_u8(0x2D, 0x3C, 0x55), alpha);
    let blue = fade(Color::rgb_u8(0x07, 0x5B, 0xFF), alpha);
    cx.backend.fill_drop_shadow(
        Rect::xywh(
            window.origin.x + 5.0 * s,
            window.origin.y + 5.0 * s,
            ww - 10.0 * s,
            window.size.y - 5.0 * s,
        ),
        8.0,
        17.0,
        fade(Color::rgb_u8(0x1F, 0x38, 0x51), 0.14 * alpha),
    );
    cx.backend
        .fill_round_rect(window, 8.0, fade(Color::rgb_u8(0xF7, 0xF9, 0xFD), alpha));
    cx.backend.stroke_round_rect(
        window,
        8.0,
        fade(Color::rgb_u8(0x31, 0x3B, 0x4C), alpha),
        4.0 * s,
    );
    // Title bar.
    let header_h = q(7.0);
    let header = Rect::xywh(window.origin.x, window.origin.y, ww, header_h);
    cx.backend
        .fill_round_rect_per_corner(header, [8.0, 8.0, 0.0, 0.0], fade(Color::WHITE, alpha));
    text_weighted(
        cx,
        "Daybreak",
        Point2D::new(header.origin.x + q(3.0), header.origin.y + q(4.5)),
        q(2.5),
        ink,
        700,
    );
    for dot in 0..3 {
        cx.backend.fill_oval(
            Rect::xywh(
                header.origin.x + ww - q(3.0) - q(5.0) + dot as f32 * q(2.0),
                header.origin.y + q(3.4),
                q(1.0),
                q(1.0),
            ),
            fade(Color::rgb_u8(0xC4, 0xCD, 0xDD), alpha),
        );
    }
    // Sidebar: three items, the first one current.
    let aside_w = ww * 0.22;
    let aside = Rect::xywh(
        window.origin.x,
        window.origin.y + header_h,
        aside_w,
        window.size.y - header_h,
    );
    cx.backend.fill_rect(aside, fade(Color::WHITE, alpha));
    cx.backend.stroke_line(
        Point2D::new(aside.origin.x + aside_w, aside.origin.y),
        Point2D::new(aside.origin.x + aside_w, aside.origin.y + aside.size.y),
        fade(Color::rgb_u8(0xE1, 0xE8, 0xF3), alpha),
        1.0,
    );
    let mut sy = aside.origin.y + q(4.0);
    text_fit(
        cx,
        art(locale, "home.art.workbench"),
        Point2D::new(aside.origin.x + q(2.0), sy + q(1.9)),
        q(1.9),
        aside_w - q(3.0),
        ink,
        700,
    );
    sy += q(1.9) + q(5.0);
    let items = [
        art(locale, "home.art.orderMgmt"),
        art(locale, "home.art.overview"),
        art(locale, "home.art.products"),
    ];
    for (index, item) in items.into_iter().enumerate() {
        let row = Rect::xywh(aside.origin.x + q(1.0), sy, aside_w - q(2.0), q(4.5));
        if index == 0 {
            cx.backend
                .fill_round_rect(row, q(0.7), fade(Color::rgb_u8(0xE9, 0xF1, 0xFF), alpha));
        }
        text_fit(
            cx,
            item,
            Point2D::new(
                row.origin.x + q(1.5),
                row.origin.y + row.size.y / 2.0 + q(0.7),
            ),
            q(1.5),
            row.size.x - q(2.0),
            if index == 0 { blue } else { ink },
            400,
        );
        sy += q(4.5) + q(3.0);
    }
    text_fit(
        cx,
        art(locale, "home.art.storeName"),
        Point2D::new(
            aside.origin.x + q(2.0),
            aside.origin.y + aside.size.y - q(4.0),
        ),
        q(1.4),
        aside_w - q(3.0),
        fade(Color::rgb_u8(0xA7, 0xB2, 0xC5), alpha),
        400,
    );
    // Main column: heading, 2×2 stats, in-flight orders.
    let main_x = window.origin.x + aside_w + q(4.0);
    let main_w = window.origin.x + ww - q(4.0) - main_x;
    let mut my = aside.origin.y + q(3.0);
    // The open badge claims its measured width first; the greeting
    // shrinks into whatever is left of the row.
    let open_w = text_fit_right(
        cx,
        art(locale, "home.art.open"),
        main_x + main_w,
        my + q(2.0),
        q(1.3),
        main_w * 0.3,
        fade(Color::rgb_u8(0x41, 0xA0, 0x79), alpha),
        400,
    );
    text_fit(
        cx,
        art(locale, "home.art.greeting"),
        Point2D::new(main_x, my + q(2.8)),
        q(2.8),
        main_w - open_w - q(4.0),
        ink,
        600,
    );
    cx.backend.fill_oval(
        Rect::xywh(
            main_x + main_w - open_w - q(1.6),
            my + q(1.4),
            q(1.0),
            q(1.0),
        ),
        fade(Color::rgb_u8(0x41, 0xA0, 0x79), alpha),
    );
    my += q(2.8) + q(3.0);
    // 2×2 stat grid.
    let stats = [
        (art(locale, "home.art.todayOrders"), "128"),
        (art(locale, "home.art.toMake"), "06"),
        (art(locale, "home.art.completed"), "122"),
        (art(locale, "home.art.avgTicket"), "¥26"),
    ];
    let cell_gap = q(2.0);
    let cell_w = (main_w - cell_gap) / 2.0;
    let cell_h = q(9.5);
    for (index, (label, value)) in stats.into_iter().enumerate() {
        let cell = Rect::xywh(
            main_x + (index % 2) as f32 * (cell_w + cell_gap),
            my + (index / 2) as f32 * (cell_h + cell_gap),
            cell_w,
            cell_h,
        );
        cx.backend
            .fill_round_rect(cell, q(1.0), fade(Color::WHITE, alpha));
        cx.backend.stroke_round_rect(
            cell,
            q(1.0),
            fade(Color::rgb_u8(0xE0, 0xE7, 0xF3), alpha),
            1.0,
        );
        text_fit(
            cx,
            label,
            Point2D::new(cell.origin.x + q(2.0), cell.origin.y + q(3.3)),
            q(1.3),
            cell_w - q(4.0),
            fade(Color::rgb_u8(0x94, 0xA2, 0xB8), alpha),
            400,
        );
        text_weighted(
            cx,
            value,
            Point2D::new(cell.origin.x + q(2.0), cell.origin.y + q(8.2)),
            q(4.2),
            ink,
            700,
        );
    }
    my += cell_h * 2.0 + cell_gap + q(2.5);
    // Orders table: header + four rows.
    let table = Rect::xywh(
        main_x,
        my,
        main_w,
        window.origin.y + window.size.y - q(3.0) - my,
    );
    cx.backend
        .fill_round_rect(table, q(1.0), fade(Color::WHITE, alpha));
    cx.backend.stroke_round_rect(
        table,
        q(1.0),
        fade(Color::rgb_u8(0xE0, 0xE7, 0xF2), alpha),
        1.0,
    );
    let count = art(locale, "home.art.orderCount").replace("{{count}}", "6");
    let count_w = text_fit_right(
        cx,
        &count,
        table.origin.x + table.size.x - q(2.5),
        table.origin.y + q(3.0),
        q(1.2),
        table.size.x * 0.3,
        fade(Color::rgb_u8(0x9E, 0xAB, 0xC0), alpha),
        400,
    );
    text_fit(
        cx,
        art(locale, "home.art.activeOrders"),
        Point2D::new(table.origin.x + q(2.5), table.origin.y + q(3.6)),
        q(2.0),
        table.size.x - count_w - q(7.0),
        ink,
        600,
    );
    let pickup = art(locale, "home.art.pickupInStore");
    let delivery = art(locale, "home.art.delivery");
    let making = art(locale, "home.art.inProgress");
    let start = art(locale, "home.art.startMaking");
    let orders = [
        ("#1028", "home.art.classicLatte", 2, 1, pickup, start),
        ("#1027", "home.art.oatLatte", 1, 2, pickup, making),
        ("#1026", "home.art.americano", 2, 4, delivery, making),
        ("#1025", "home.art.cappuccino", 1, 5, pickup, making),
    ]
    .map(|(number, drink, qty, minutes, how, action)| {
        let ago = art(locale, "home.art.minutesAgo").replace("{{count}}", &minutes.to_string());
        (
            format!("{number}  {} × {qty}", art(locale, drink)),
            format!("{ago} · {how}"),
            action,
        )
    });
    let body_top = table.origin.y + q(5.0);
    let row_h = (table.origin.y + table.size.y - q(1.0) - body_top) / 4.0;
    for (index, (title, meta, action)) in orders.into_iter().enumerate() {
        let row_y = body_top + index as f32 * row_h;
        if index > 0 {
            cx.backend.stroke_line(
                Point2D::new(table.origin.x, row_y),
                Point2D::new(table.origin.x + table.size.x, row_y),
                fade(Color::rgb_u8(0xEA, 0xF0, 0xF8), alpha),
                1.0,
            );
        }
        // The action pill sizes to its label (capped), and the order
        // text shrinks into the space left of it.
        let pill_size = super::art_copy::fit_size(cx, action, q(1.4), 400, table.size.x * 0.3);
        let pill_w = super::art_copy::measure(cx, action, pill_size, 400) + q(2.0);
        let pill = Rect::xywh(
            table.origin.x + table.size.x - q(2.5) - pill_w,
            row_y + q(1.2),
            pill_w,
            q(3.2),
        );
        let text_w = pill.origin.x - (table.origin.x + q(2.5)) - q(1.5);
        text_fit(
            cx,
            &title,
            Point2D::new(table.origin.x + q(2.5), row_y + q(2.2)),
            q(1.8),
            text_w,
            ink,
            400,
        );
        text_fit(
            cx,
            &meta,
            Point2D::new(table.origin.x + q(2.5), row_y + q(4.4)),
            q(1.2),
            text_w,
            fade(Color::rgb_u8(0xA0, 0xAE, 0xC3), alpha),
            400,
        );
        cx.backend
            .fill_round_rect(pill, q(0.5), fade(Color::rgb_u8(0xF0, 0xF5, 0xFF), alpha));
        text_fit(
            cx,
            action,
            Point2D::new(pill.origin.x + q(1.0), pill.origin.y + q(2.1)),
            pill_size,
            pill_w,
            blue,
            400,
        );
    }
}

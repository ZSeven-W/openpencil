//! Lucide d-strings, part 2 — content / media / status / editor glyphs
//! (`MAIL` … `SQUARES_EXCLUDE`). Carved off `icons_data.rs` so every
//! file stays under the 800-line ceiling; the parent module
//! glob-re-exports both parts, so `icons.rs` still reaches every
//! constant through `super::icons_data::*`.
//!
//! Source: https://github.com/lucide-icons/lucide/tree/main/icons
//! License: ISC.

// Lucide `mail.svg` — envelope outline with flap chevron.
pub(crate) const MAIL: &[&str] = &[
    "M22 7a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2",
    "M22 7v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V7",
    "M22 7l-10 7L2 7",
];

// Lucide `smartphone.svg` — rounded rect with home-pill at the bottom.
pub(crate) const SMARTPHONE: &[&str] = &[
    "M5 2h14a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z",
    "M11 18h2",
];

// Lucide `chrome.svg` — outer ring + concentric inner ring (r=4
// at viewBox centre 12,12) + three radial spokes. Previous inner
// path arced through (12, 8) so the hole rendered as an off-centre
// "G"-like crescent — visible in login.op's Google button.
pub(crate) const CHROME: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
    "M21.17 8H12",
    "M3.95 6.06L8.54 14",
    "M10.88 21.94L15.5 14",
];

// Apple-mark glyph — single closed path; not in upstream lucide
// but kept here so `.op` files that author `iconFontName:"apple"` get
// a recognisable mark instead of an honest-but-blank placeholder.
pub(crate) const APPLE: &[&str] = &[
    "M16 10c-.6 0-2.1.8-3.5.8-1.4 0-2.4-.8-3.6-.8-1.9 0-4.1 1.6-4.1 4.6 0 2.7 1.5 5.4 3.4 6.4 1 .5 1.7 0 2.3 0 .7 0 1.4.5 2.4 0 1.9-1 3.4-3.7 3.4-6.4 0-3-2.2-4.6-4.1-4.6Z",
    "M14 6c.8-1 1.4-2.2 1.1-3.5-.9.1-2.1.7-2.8 1.6-.7.8-1.3 2-1.1 3.2 1 .1 2-.4 2.8-1.3z",
];

// Lucide `user.svg` — head circle + shoulders arc.
pub(crate) const USER: &[&str] = &[
    "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2",
    "M12 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
];

// ── Additional first-party glyphs surfaced via `Icon::from_name` ──
pub(crate) const CLOCK: &[&str] = &["M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z", "M12 6v6l4 2"];
pub(crate) const CALENDAR: &[&str] = &[
    "M8 2v4",
    "M16 2v4",
    "M3 8a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    "M3 10h18",
];
pub(crate) const STAR: &[&str] = &[
    "M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z",
];
pub(crate) const HEART: &[&str] = &[
    "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z",
];
pub(crate) const HOME: &[&str] = &[
    "M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8",
    "M3 10a2 2 0 0 1 .709-1.528l7-5.999a2 2 0 0 1 2.582 0l7 5.999A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
];
pub(crate) const BELL: &[&str] = &[
    "M10.268 21a2 2 0 0 0 3.464 0",
    "M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326",
];
pub(crate) const PLAY: &[&str] =
    &["M6 3a1 1 0 0 1 1.539-.843l13 8a1 1 0 0 1 0 1.686l-13 8A1 1 0 0 1 6 19z"];
pub(crate) const MAP_PIN: &[&str] = &[
    "M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0",
    "M12 12a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z",
];
pub(crate) const PHONE: &[&str] = &[
    "M13.832 16.568a1 1 0 0 0 1.213-.303l.355-.465A2 2 0 0 1 17 15h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2A18 18 0 0 1 2 4a2 2 0 0 1 2-2h3a2 2 0 0 1 2 2v3a2 2 0 0 1-.8 1.6l-.468.351a1 1 0 0 0-.292 1.233 14 14 0 0 0 6.392 6.384",
];
pub(crate) const CAMERA: &[&str] = &[
    "M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3z",
    "M12 17a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
];
pub(crate) const VIDEO: &[&str] = &[
    "m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.87a.5.5 0 0 0-.752-.432L16 10.5",
    "M2 8a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z",
];
pub(crate) const MUSIC: &[&str] = &[
    "M9 18V5l12-2v13",
    "M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
    "M21 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
];
pub(crate) const SHARE: &[&str] = &[
    "M12 2v13",
    "m16 6-4-4-4 4",
    "M4 11v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8",
];
pub(crate) const INFO: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "M12 16v-4",
    "M12 8h.01",
];
pub(crate) const ALERT_CIRCLE: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "M12 8v4",
    "M12 16h.01",
];
pub(crate) const HELP_CIRCLE: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3",
    "M12 17h.01",
];

pub(crate) const CHEVRON_LEFT: &[&str] = &["m15 18-6-6 6-6"];
pub(crate) const MORE_VERTICAL: &[&str] = &[
    "M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M12 6a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M12 20a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
];
pub(crate) const MORE_HORIZONTAL: &[&str] = &[
    "M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M19 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M5 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
];

pub(crate) const ARROW_RIGHT: &[&str] = &["M5 12h14", "M13 5l7 7-7 7"];
pub(crate) const ARROW_LEFT: &[&str] = &["M19 12H5", "M11 5l-7 7 7 7"];
pub(crate) const CHECK_CIRCLE: &[&str] =
    &["M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z", "m9 12 2 2 4-4"];
pub(crate) const ALERT_TRIANGLE: &[&str] = &[
    "M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z",
    "M12 9v4",
    "M12 17h.01",
];
pub(crate) const ALERT_OCTAGON: &[&str] = &[
    "M7.86 2h8.28L22 7.86v8.28L16.14 22H7.86L2 16.14V7.86z",
    "M12 8v4",
    "M12 16h.01",
];
pub(crate) const STICKY_NOTE: &[&str] = &[
    "M15.5 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V8.5z",
    "M15 3v6h6",
];
pub(crate) const BAR_CHART_2: &[&str] = &["M18 20V10", "M12 20V4", "M6 20v-6"];
pub(crate) const BOLD: &[&str] = &["M6 12h9a4 4 0 0 1 0 8H6z", "M6 4h8a4 4 0 0 1 0 8H6z"];
pub(crate) const ITALIC: &[&str] = &["M19 4h-9", "M14 20H5", "M15 4 9 20"];
pub(crate) const UNDERLINE: &[&str] = &["M6 4v6a6 6 0 0 0 12 0V4", "M4 20h16"];
pub(crate) const STRIKETHROUGH: &[&str] = &[
    "M16 4H9a3 3 0 0 0-2.83 4",
    "M14 12a4 4 0 0 1 0 8H6",
    "M4 12h16",
];
pub(crate) const SHOPPING_CART: &[&str] = &[
    "M9 22a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M20 22a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M1 1h4l2.68 13.39a2 2 0 0 0 2 1.61h9.72a2 2 0 0 0 2-1.61L23 6H6",
];
pub(crate) const SHOPPING_BAG: &[&str] = &[
    "M6 2 3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z",
    "M3 6h18",
    "M16 10a4 4 0 0 1-8 0",
];
pub(crate) const SEND: &[&str] = &["m22 2-7 20-4-9-9-4z", "M22 2 11 13"];
pub(crate) const PAPERCLIP: &[&str] = &[
    "m16 6-8.414 8.586a2 2 0 0 0 2.829 2.829l8.414-8.586a4 4 0 1 0-5.657-5.657l-8.379 8.551a6 6 0 1 0 8.485 8.485l8.379-8.551",
];
pub(crate) const MESSAGE_CIRCLE: &[&str] = &["M7.9 20A9 9 0 1 0 4 16.1L2 22z"];
pub(crate) const ROCKET: &[&str] = &[
    "M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z",
    "M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z",
    "M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0",
    "M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5",
];
pub(crate) const MENU: &[&str] = &["M4 12h16", "M4 6h16", "M4 18h16"];
pub(crate) const CREDIT_CARD: &[&str] = &[
    "M3 5h18a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z",
    "M2 10h20",
];
pub(crate) const X_CIRCLE: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "m15 9-6 6",
    "m9 9 6 6",
];

pub(crate) const TRENDING_UP: &[&str] = &["M16 7h6v6", "m22 7-8.5 8.5-5-5L2 17"];
pub(crate) const TRENDING_DOWN: &[&str] = &["M16 17h6v-6", "m22 17-8.5-8.5-5 5L2 7"];
pub(crate) const COMPASS: &[&str] = &[
    "m16.24 7.76-1.804 5.411a2 2 0 0 1-1.265 1.265L7.76 16.24l1.804-5.411a2 2 0 0 1 1.265-1.265z",
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
];
pub(crate) const REFRESH_CW: &[&str] = &[
    "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8",
    "M21 3v5h-5",
    "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16",
    "M8 16H3v5",
];
pub(crate) const LAYOUT_DASHBOARD: &[&str] = &[
    "M3 3h7a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
    "M14 3h7a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
    "M14 11h7a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1z",
    "M3 16h7a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1v-4a1 1 0 0 1 1-1z",
];
pub(crate) const USERS: &[&str] = &[
    "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2",
    "M16 3.128a4 4 0 0 1 0 7.744",
    "M22 21v-2a4 4 0 0 0-3-3.87",
    "M5 7a4 4 0 1 0 8 0 4 4 0 0 0-8 0z",
];
pub(crate) const PACKAGE: &[&str] = &[
    "M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z",
    "M12 22V12",
    // <polyline points="3.29 7 12 12 20.71 7"/>
    "M3.29 7L12 12L20.71 7",
    "m7.5 4.27 9 5.15",
];
pub(crate) const ZAP: &[&str] = &[
    "M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z",
];
pub(crate) const SLIDERS_HORIZONTAL: &[&str] = &[
    "M10 5H3",
    "M12 19H3",
    "M14 3v4",
    "M16 17v4",
    "M21 12h-9",
    "M21 19h-5",
    "M21 5h-7",
    "M8 10v4",
    "M8 12H3",
];
pub(crate) const ACTIVITY: &[&str] = &[
    "M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2",
];
pub(crate) const LOADER: &[&str] = &[
    "M12 2v4",
    "m16.2 7.8 2.9-2.9",
    "M18 12h4",
    "m16.2 16.2 2.9 2.9",
    "M12 18v4",
    "m4.9 19.1 2.9-2.9",
    "M2 12h4",
    "m4.9 4.9 2.9 2.9",
];
pub(crate) const FOCUS: &[&str] = &[
    // Inner circle: r=3 at (12,12) per lucide@0.545.0 — previously
    // shipped as r=4, which made the focus crosshair too large.
    "M9 12a3 3 0 1 0 6 0 3 3 0 0 0-6 0z",
    "M3 7V5a2 2 0 0 1 2-2h2",
    "M17 3h2a2 2 0 0 1 2 2v2",
    "M21 17v2a2 2 0 0 1-2 2h-2",
    "M7 21H5a2 2 0 0 1-2-2v-2",
];
pub(crate) const CHART_LINE: &[&str] = &["M3 3v16a2 2 0 0 0 2 2h16", "m19 9-5 5-4-4-3 3"];
pub(crate) const SETTINGS2: &[&str] = &[
    "M14 17H5",
    // Top track starts at x=19 (not 20) per lucide@0.545.0.
    "M19 7h-9",
    // Two filled circles, r=3 at (17,17) and (7,7).
    "M14 17a3 3 0 1 0 6 0 3 3 0 0 0-6 0z",
    "M4 7a3 3 0 1 0 6 0 3 3 0 0 0-6 0z",
];

// === Git overflow-menu icons (lucide@0.545.0) ===

// Lucide `file-search.svg` — circle cx=5 cy=14 r=3 → two-arc path.
pub(crate) const FILE_SEARCH: &[&str] = &[
    "M14 2v4a2 2 0 0 0 2 2h4",
    "M4.268 21a2 2 0 0 0 1.727 1H18a2 2 0 0 0 2-2V7l-5-5H6a2 2 0 0 0-2 2v3",
    "m9 18-1.5-1.5",
    "M2 14A3 3 0 1 0 8 14A3 3 0 1 0 2 14Z",
];

// Lucide `file-down.svg` — export-a-document row in the export menu.
pub(crate) const FILE_DOWN: &[&str] = &[
    "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7z",
    "M14 2v6h6",
    "M12 18v-6",
    "m9 15 3 3 3-3",
];

// Lucide `user-x.svg` — circle cx=9 cy=7 r=4 + two <line> → "M…L…".
pub(crate) const USER_X: &[&str] = &[
    "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2",
    "M5 7A4 4 0 1 0 13 7A4 4 0 1 0 5 7Z",
    "M17 8L22 13",
    "M22 8L17 13",
];

// Lucide `key.svg` — circle cx=7.5 cy=15.5 r=5.5 → two-arc path.
pub(crate) const KEY: &[&str] = &[
    "m15.5 7.5 2.3 2.3a1 1 0 0 0 1.4 0l2.1-2.1a1 1 0 0 0 0-1.4L19 4",
    "m21 2-9.6 9.6",
    "M2 15.5A5.5 5.5 0 1 0 13 15.5A5.5 5.5 0 1 0 2 15.5Z",
];

// Lucide `log-out.svg`.
pub(crate) const LOG_OUT: &[&str] = &[
    "m16 17 5-5-5-5",
    "M21 12H9",
    "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4",
];

// === Align toolbar icons (lucide@0.545.0) ===
// Rounded rects expanded to "M…H…A…V…A…H…A…V…A…Z" paths.

pub(crate) const ALIGN_LEFT: &[&str] = &[
    // align-start-vertical: small + wide rect snapped to x=6, vert. spine at x=2.
    "M8 14 H13 A2 2 0 0 1 15 16 V18 A2 2 0 0 1 13 20 H8 A2 2 0 0 1 6 18 V16 A2 2 0 0 1 8 14 Z",
    "M8 4 H20 A2 2 0 0 1 22 6 V8 A2 2 0 0 1 20 10 H8 A2 2 0 0 1 6 8 V6 A2 2 0 0 1 8 4 Z",
    "M2 2v20",
];

pub(crate) const ALIGN_CENTER_H: &[&str] = &[
    // align-center-vertical: center spine + two pill clips.
    "M12 2v20",
    "M8 10H4a2 2 0 0 1-2-2V6c0-1.1.9-2 2-2h4",
    "M16 10h4a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-4",
    "M8 20H7a2 2 0 0 1-2-2v-2c0-1.1.9-2 2-2h1",
    "M16 14h1a2 2 0 0 1 2 2v2a2 2 0 0 1-2 2h-1",
];

pub(crate) const ALIGN_RIGHT: &[&str] = &[
    // align-end-vertical: rects flush to spine on right (x=22).
    "M4 4 H16 A2 2 0 0 1 18 6 V8 A2 2 0 0 1 16 10 H4 A2 2 0 0 1 2 8 V6 A2 2 0 0 1 4 4 Z",
    "M11 14 H16 A2 2 0 0 1 18 16 V18 A2 2 0 0 1 16 20 H11 A2 2 0 0 1 9 18 V16 A2 2 0 0 1 11 14 Z",
    "M22 22V2",
];

pub(crate) const ALIGN_TOP: &[&str] = &[
    // align-start-horizontal: tall + short rect snapped to y=6, horiz. spine at y=2.
    "M6 6 H8 A2 2 0 0 1 10 8 V20 A2 2 0 0 1 8 22 H6 A2 2 0 0 1 4 20 V8 A2 2 0 0 1 6 6 Z",
    "M16 6 H18 A2 2 0 0 1 20 8 V13 A2 2 0 0 1 18 15 H16 A2 2 0 0 1 14 13 V8 A2 2 0 0 1 16 6 Z",
    "M22 2H2",
];

pub(crate) const ALIGN_CENTER_V: &[&str] = &[
    // align-center-horizontal: center spine + two pill clips top/bottom.
    "M2 12h20",
    "M10 16v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-4",
    "M10 8V4a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v4",
    "M20 16v1a2 2 0 0 1-2 2h-2a2 2 0 0 1-2-2v-1",
    "M14 8V7c0-1.1.9-2 2-2h2a2 2 0 0 1 2 2v1",
];

pub(crate) const ALIGN_BOTTOM: &[&str] = &[
    // align-end-horizontal: rects flush to spine on bottom (y=22).
    "M6 2 H8 A2 2 0 0 1 10 4 V16 A2 2 0 0 1 8 18 H6 A2 2 0 0 1 4 16 V4 A2 2 0 0 1 6 2 Z",
    "M16 9 H18 A2 2 0 0 1 20 11 V16 A2 2 0 0 1 18 18 H16 A2 2 0 0 1 14 16 V11 A2 2 0 0 1 16 9 Z",
    "M22 22H2",
];

pub(crate) const DISTRIBUTE_H: &[&str] = &[
    // align-horizontal-distribute-center: two rects + tick marks above/below.
    "M6 5 H8 A2 2 0 0 1 10 7 V17 A2 2 0 0 1 8 19 H6 A2 2 0 0 1 4 17 V7 A2 2 0 0 1 6 5 Z",
    "M16 7 H18 A2 2 0 0 1 20 9 V15 A2 2 0 0 1 18 17 H16 A2 2 0 0 1 14 15 V9 A2 2 0 0 1 16 7 Z",
    "M17 22v-5",
    "M17 7V2",
    "M7 22v-3",
    "M7 5V2",
];

pub(crate) const DISTRIBUTE_V: &[&str] = &[
    // align-vertical-distribute-center: two rects + tick marks left/right.
    "M22 17h-3",
    "M22 7h-5",
    "M5 17H2",
    "M7 7H2",
    "M7 14 H17 A2 2 0 0 1 19 16 V18 A2 2 0 0 1 17 20 H7 A2 2 0 0 1 5 18 V16 A2 2 0 0 1 7 14 Z",
    "M9 4 H15 A2 2 0 0 1 17 6 V8 A2 2 0 0 1 15 10 H9 A2 2 0 0 1 7 8 V6 A2 2 0 0 1 9 4 Z",
];

/// Line-height glyph for the typography section — a vertical
/// double-arrow beside three text rules. Ported from the TS
/// `LineHeightIcon` inline SVG (12×12), scaled to the 24×24 viewBox.
pub(crate) const LINE_HEIGHT: &[&str] = &[
    "M4 4 L4 20",
    "M8 8 L4 4 L0 8",
    "M8 16 L4 20 L0 16",
    "M11 6 L18 6",
    "M11 12 L22 12",
    "M11 18 L18 18",
];

/// lucide `milestone` (signpost) — used on the Git panel's
/// "保存为里程碑" commit button. Two post segments + the sign body.
/// Verbatim from lucide-react 0.511.0.
pub(crate) const MILESTONE: &[&str] = &[
    "M12 13v8",
    "M12 3v3",
    "M4 6a1 1 0 0 0-1 1v5a1 1 0 0 0 1 1h13a2 2 0 0 0 1.152-.365l3.424-2.317a1 1 0 0 0 0-1.635l-3.424-2.318A2 2 0 0 0 17 6z",
];

/// lucide `wrench` — used by TS for delegated/orchestrated tool cards.
pub(crate) const WRENCH: &[&str] = &[
    "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94z",
];

/// lucide `squares-unite` (0.545.0) — Boolean Union menu row.
pub(crate) const SQUARES_UNITE: &[&str] = &[
    "M4 16a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v3a1 1 0 0 0 1 1h3a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H10a2 2 0 0 1-2-2v-3a1 1 0 0 0-1-1z",
];

/// lucide `squares-subtract` — Boolean Subtract row.
pub(crate) const SQUARES_SUBTRACT: &[&str] = &[
    "M10 22a2 2 0 0 1-2-2",
    "M16 22h-2",
    "M16 4a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h3a1 1 0 0 0 1-1v-5a2 2 0 0 1 2-2h5a1 1 0 0 0 1-1z",
    "M20 8a2 2 0 0 1 2 2",
    "M22 14v2",
    "M22 20a2 2 0 0 1-2 2",
];

/// lucide `squares-intersect` — Boolean Intersect row.
pub(crate) const SQUARES_INTERSECT: &[&str] = &[
    "M10 22a2 2 0 0 1-2-2",
    "M14 2a2 2 0 0 1 2 2",
    "M16 22h-2",
    "M2 10V8",
    "M2 4a2 2 0 0 1 2-2",
    "M20 8a2 2 0 0 1 2 2",
    "M22 14v2",
    "M22 20a2 2 0 0 1-2 2",
    "M4 16a2 2 0 0 1-2-2",
    "M8 10a2 2 0 0 1 2-2h5a1 1 0 0 1 1 1v5a2 2 0 0 1-2 2H9a1 1 0 0 1-1-1z",
    "M8 2h2",
];

/// lucide `squares-exclude` — Boolean Exclude row.
pub(crate) const SQUARES_EXCLUDE: &[&str] = &[
    "M16 12v2a2 2 0 0 1-2 2H9a1 1 0 0 0-1 1v3a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V10a2 2 0 0 0-2-2h0",
    "M4 16a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v3a1 1 0 0 1-1 1h-5a2 2 0 0 0-2 2v2",
];

/// Custom stacked-plates glyph — the left rail's Layers tab when the
/// rail is too narrow for its label. Hand-authored in the shared 24×24
/// viewBox rather than sourced from lucide: the tab row needs a plate
/// stack that reads at 14 px, and lucide's `layers` carries rounded
/// join detail that turns to mush at that size.
pub(crate) const LAYERS_STACK: &[&str] = &[
    "M12 3 3 7.5 12 12 21 7.5Z",
    "M3 12 12 16.5 21 12",
    "M3 16.5 12 21 21 16.5",
];

/// Lucide `layers.svg` — semantic layer hierarchy entry for touch chrome.
pub(crate) const LAYERS: &[&str] = &[
    "M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z",
    "M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12",
    "M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17",
];

/// Custom presentation-screen glyph — the left rail's Slides tab when
/// the rail is too narrow for its label. A screen on a stand, which is
/// what a board in a deck is; the scenario-specific names (cards and
/// the rest) share it rather than each inventing a glyph.
pub(crate) const PRESENTATION_SCREEN: &[&str] = &["M3 4h18v11H3Z", "M12 15v3", "M8.5 21h7"];

/// lucide `brain` — the chat footer's thinking-mode toggle.
pub(crate) const BRAIN: &[&str] = &[
    "M12 18V5",
    "M15 13a4.17 4.17 0 0 1-3-4 4.17 4.17 0 0 1-3 4",
    "M17.598 6.5A3 3 0 1 0 12 5a3 3 0 1 0-5.598 1.5",
    "M17.997 5.125a4 4 0 0 1 2.526 5.77",
    "M18 18a4 4 0 0 0 2-7.464",
    "M19.967 17.483A4 4 0 1 1 12 18a4 4 0 1 1-7.967-.517",
    "M6 18a4 4 0 0 1-2-7.464",
    "M6.003 5.125a4 4 0 0 0-2.526 5.77",
];

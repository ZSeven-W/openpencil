//! Lucide d-string constants — all `Icon::paths` data lives here so
//! `icons.rs` stays under the 800-line file ceiling. New icons:
//! add the d-string here, register a variant in `icons.rs::Icon` +
//! `paths()` + `from_name()`. Sourced verbatim from
//! https://github.com/lucide-icons/lucide (ISC).

// Source: https://github.com/lucide-icons/lucide/tree/main/icons
// License: ISC.

pub(super) const CURSOR: &[&str] = &[
    "M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z",
];

pub(super) const SQUARE: &[&str] = &[
    // Lucide ships <rect x=3 y=3 w=18 h=18 rx=2/>; expanded to a
    // round-rect path so stroke_svg_path can render it uniformly.
    "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
];

pub(super) const CHEVRON_DOWN: &[&str] = &["m6 9 6 6 6-6"];

pub(super) const CHEVRON_RIGHT: &[&str] = &["m9 18 6-6-6-6"];

pub(super) const TYPE: &[&str] = &[
    "M12 4v16",
    "M4 7V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2",
    "M9 20h6",
];

pub(super) const FRAME: &[&str] = &[
    // Four <line> elements expanded to "M…L…" path strings.
    "M22 6L2 6",
    "M22 18L2 18",
    "M6 2L6 22",
    "M18 2L18 22",
];

pub(super) const HAND: &[&str] = &[
    "M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2",
    "M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2",
    "M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8",
    "M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15",
];

pub(super) const UNDO: &[&str] = &[
    "M9 14 4 9l5-5",
    "M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11",
];

pub(super) const REDO: &[&str] = &[
    "m15 14 5-5-5-5",
    "M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13",
];

pub(super) const BRACES: &[&str] = &[
    "M8 3H7a2 2 0 0 0-2 2v5a2 2 0 0 1-2 2 2 2 0 0 1 2 2v5c0 1.1.9 2 2 2h1",
    "M16 21h1a2 2 0 0 0 2-2v-5c0-1.1.9-2 2-2a2 2 0 0 1-2-2V5a2 2 0 0 0-2-2h-1",
];

pub(super) const BOOK_OPEN: &[&str] = &[
    "M12 7v14",
    "M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z",
];

pub(super) const PLUS: &[&str] = &["M5 12h14", "M12 5v14"];

pub(super) const MINUS: &[&str] = &["M5 12h14"];

pub(super) const SEARCH: &[&str] = &[
    "m21 21-4.34-4.34",
    // <circle cx=11 cy=11 r=8/> expanded to a two-arc path.
    "M3 11A8 8 0 1 0 19 11A8 8 0 1 0 3 11Z",
];

pub(super) const SUN: &[&str] = &[
    // <circle cx=12 cy=12 r=4/>
    "M8 12A4 4 0 1 0 16 12A4 4 0 1 0 8 12Z",
    "M12 2v2",
    "M12 20v2",
    "m4.93 4.93 1.41 1.41",
    "m17.66 17.66 1.41 1.41",
    "M2 12h2",
    "M20 12h2",
    "m6.34 17.66-1.41 1.41",
    "m19.07 4.93-1.41 1.41",
];

pub(super) const MOON: &[&str] = &[
    // Lucide moon — crescent.
    "M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z",
];

pub(super) const GLOBE: &[&str] = &[
    // <circle cx=12 cy=12 r=10/>
    "M2 12A10 10 0 1 0 22 12A10 10 0 1 0 2 12Z",
    "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20",
    "M2 12h20",
];

pub(super) const MAXIMIZE: &[&str] = &[
    "M8 3H5a2 2 0 0 0-2 2v3",
    "M21 8V5a2 2 0 0 0-2-2h-3",
    "M3 16v3a2 2 0 0 0 2 2h3",
    "M16 21h3a2 2 0 0 0 2-2v-3",
];

pub(super) const MINIMIZE_2: &[&str] = &["M4 14h6v6", "M20 10h-6V4"];

pub(super) const HASH: &[&str] = &[
    // 4 <line> elements.
    "M4 9L20 9",
    "M4 15L20 15",
    "M10 3L8 21",
    "M16 3L14 21",
];

pub(super) const PANEL_LEFT: &[&str] = &[
    // <rect x=3 y=3 w=18 h=18 rx=2/> + <path d="M9 3v18"/>
    "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
    "M9 3v18",
];

pub(super) const FOLDER_OPEN: &[&str] = &[
    "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2",
];

pub(super) const HISTORY: &[&str] = &[
    "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8",
    "M3 3v5h5",
    "M12 7v5l4 2",
];

pub(super) const FILE_PLUS: &[&str] = &[
    "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z",
    "M14 2v4a2 2 0 0 0 2 2h4",
    "M9 15h6",
    "M12 18v-6",
];

pub(super) const GIT_FORK: &[&str] = &[
    "M12 18 m-3 0 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0",
    "M6 6 m-3 0 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0",
    "M18 6 m-3 0 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0",
    "M18 9v2c0 .6-.4 1-1 1H7c-.6 0-1-.4-1-1V9",
    "M12 12v3",
];

pub(super) const GIT_BRANCH: &[&str] = &[
    // lucide git-branch: line + two r=3 circles + connecting arc.
    "M6 3v12",
    "M18 6 m-3 0 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0",
    "M6 18 m-3 0 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0",
    "M18 9a9 9 0 0 1-9 9",
];

pub(super) const SPARKLES: &[&str] = &[
    "M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z",
    "M20 2v4",
    "M22 4h-4",
    // <circle cx=4 cy=20 r=2/>
    "M2 20A2 2 0 1 0 6 20A2 2 0 1 0 2 20Z",
];

pub(super) const CLOSE: &[&str] = &["M18 6 6 18", "m6 6 12 12"];

// Mirror of CHEVRON_DOWN flipped vertically — Lucide
// `chevron-up.svg` `d="m18 15-6-6-6 6"`.
pub(super) const CHEVRON_UP: &[&str] = &["m18 15-6-6-6 6"];

// Lucide `message-square.svg` — speech-bubble outline used by the
// collapsed AI chat pill.
pub(super) const MESSAGE_SQUARE: &[&str] =
    &["M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"];

// Lucide `layout-grid.svg` — 4 rounded-rect cells.
pub(super) const LAYOUT_GRID: &[&str] = &[
    "M4 3h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
    "M15 3h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
    "M15 14h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z",
    "M4 14h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z",
];

// Lucide `rows-3.svg` — round-rect with 2 horizontal dividers.
pub(super) const ROWS_3: &[&str] = &[
    "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
    "M21 9H3",
    "M21 15H3",
];

// Lucide `columns-3.svg` — round-rect with 2 vertical dividers.
pub(super) const COLUMNS_3: &[&str] = &[
    "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
    "M9 3v18",
    "M15 3v18",
];

// Lucide `rotate-cw.svg`.
pub(super) const ROTATE_CW: &[&str] = &[
    "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8",
    "M21 3v5h-5",
];

// Lucide `diamond.svg`.
pub(super) const DIAMOND: &[&str] = &[
    "M2.7 10.3a2.41 2.41 0 0 0 0 3.41l7.59 7.59a2.41 2.41 0 0 0 3.41 0l7.59-7.59a2.41 2.41 0 0 0 0-3.41l-7.59-7.59a2.41 2.41 0 0 0-3.41 0Z",
];

// Lucide `component.svg`.
pub(super) const COMPONENT: &[&str] = &[
    "M15.536 11.293a1 1 0 0 0 0 1.414l2.376 2.377a1 1 0 0 0 1.414 0l2.377-2.377a1 1 0 0 0 0-1.414l-2.377-2.377a1 1 0 0 0-1.414 0z",
    "M2.297 11.293a1 1 0 0 0 0 1.414l2.377 2.377a1 1 0 0 0 1.414 0l2.377-2.377a1 1 0 0 0 0-1.414L6.088 8.916a1 1 0 0 0-1.414 0z",
    "M8.916 17.912a1 1 0 0 0 0 1.415l2.377 2.376a1 1 0 0 0 1.414 0l2.377-2.376a1 1 0 0 0 0-1.415l-2.377-2.376a1 1 0 0 0-1.414 0z",
    "M8.916 4.674a1 1 0 0 0 0 1.414l2.377 2.376a1 1 0 0 0 1.414 0l2.377-2.376a1 1 0 0 0 0-1.414l-2.377-2.377a1 1 0 0 0-1.414 0z",
];

// Lucide `unlink.svg`.
pub(super) const UNLINK: &[&str] = &[
    "m18.84 12.25 1.72-1.71h-.02a5.004 5.004 0 0 0-.12-7.07 5.006 5.006 0 0 0-6.95 0l-1.72 1.71",
    "m5.17 11.75-1.71 1.71a5.004 5.004 0 0 0 .12 7.07 5.006 5.006 0 0 0 6.95 0l1.71-1.71",
    "M8 2L8 5",
    "M2 8L5 8",
    "M16 19L16 22",
    "M19 16L22 16",
];

// Lucide `check.svg`.
pub(super) const CHECK: &[&str] = &["M20 6 9 17l-5-5"];

// Lucide `github.svg`.
pub(super) const GITHUB: &[&str] = &[
    "M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4",
    "M9 18c-4.51 2-5-2-7-2",
];

// Lucide `bot.svg` — friendly robot, used for code-CLI providers.
pub(super) const BOT: &[&str] = &[
    "M12 8V4H8",
    "M2 14h2",
    "M20 14h2",
    "M15 13v2",
    "M9 13v2",
    "M12 8H8a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-8a2 2 0 0 0-2-2h-4",
];

// Lucide `square-terminal.svg` — chevron + underline inside a
// rounded square. Three separate <path>/<rect> elements lifted
// straight from the source SVG.
pub(super) const TERMINAL: &[&str] = &[
    "m7 11 2-2-2-2",
    "M11 13h4",
    "M3 5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
];

// Lucide `image.svg`.
pub(super) const IMAGE: &[&str] = &[
    "M21 15V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4",
    "M9 9a2 2 0 1 0 0 .01",
    "m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21",
];

// Lucide `save.svg` (floppy disk).
pub(super) const SAVE: &[&str] = &[
    "M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
    "M7 3v4a1 1 0 0 0 1 1h7",
    "M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7",
];

// Lucide `download.svg`.
pub(super) const DOWNLOAD: &[&str] = &[
    "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4",
    "M7 10l5 5 5-5",
    "M12 15V3",
];

// Lucide `file-text.svg`.
pub(super) const FILE_TEXT: &[&str] = &[
    "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7z",
    "M14 2v6h6",
    "M16 13H8",
    "M16 17H8",
    "M10 9H8",
];

// Lucide `settings.svg`.
pub(super) const SETTINGS: &[&str] = &[
    "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z",
    "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z",
];

// Lucide `arrow-up-right.svg`.
pub(super) const ARROW_UP_RIGHT: &[&str] = &["M7 7h10v10", "M7 17 17 7"];

// Lucide `circle.svg`.
pub(super) const CIRCLE: &[&str] = &["M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Z"];

// Lucide `triangle.svg`.
pub(super) const TRIANGLE: &[&str] =
    &["M13.73 4a2 2 0 0 0-3.46 0l-8.15 14a2 2 0 0 0 1.73 3h16.34a2 2 0 0 0 1.73-3Z"];

// Lucide `pen-tool.svg`.
pub(super) const PEN_TOOL: &[&str] = &[
    "M15.707 21.293a1 1 0 0 1-1.414 0l-1.586-1.586a1 1 0 0 1 0-1.414l5.586-5.586a1 1 0 0 1 1.414 0l1.586 1.586a1 1 0 0 1 0 1.414z",
    "m18 13-1.375-6.874a1 1 0 0 0-.746-.776L3.235 2.028a1 1 0 0 0-1.207 1.207L5.35 15.879a1 1 0 0 0 .776.746L13 18",
    "m2.3 2.3 7.286 7.286",
    "M11 11a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z",
];

// Lucide `image-plus.svg`.
pub(super) const IMAGE_PLUS: &[&str] = &[
    "M16 5h6",
    "M19 2v6",
    "M21 11.5V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7.5",
    "m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21",
    "M9 9a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z",
];

// Lucide `eye.svg`.
pub(super) const EYE: &[&str] = &[
    "M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0Z",
    "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z",
];

// Lucide `lock.svg` — rect body + closed shackle.
pub(super) const LOCK: &[&str] = &[
    "M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2z",
    "M7 11V7a5 5 0 0 1 10 0v4",
];

// Lucide `lock-open.svg` — rect body + half-open shackle
// (right side of the arc is cut so the lock reads as "open").
pub(super) const LOCK_OPEN: &[&str] = &[
    "M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2z",
    "M7 11V7a5 5 0 0 1 9.9-1",
];

// Lucide `eye-off.svg` — eye with diagonal strike.
pub(super) const EYE_OFF: &[&str] = &[
    "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49",
    "M14.084 14.158a3 3 0 0 1-4.242-4.242",
    "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143",
    "m2 2 20 20",
];

// Lucide `trash-2` — line-art trash can.
pub(super) const TRASH: &[&str] = &[
    "M3 6h18",
    "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6",
    "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",
    "M10 11v6",
    "M14 11v6",
];

// Lucide `copy` — two stacked rectangles.
pub(super) const COPY: &[&str] = &[
    "M20 9h-9a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2v-9a2 2 0 0 0-2-2z",
    "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1",
];

// Lucide `pencil`.
pub(super) const PENCIL: &[&str] = &[
    "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
    "M15 5l4 4",
];

// Lucide `arrow-up`.
pub(super) const ARROW_UP: &[&str] = &["M12 19V5", "M5 12l7-7 7 7"];

// Lucide `arrow-down`.
pub(super) const ARROW_DOWN: &[&str] = &["M12 5v14", "M5 12l7 7 7-7"];

// Lucide `mail.svg` — envelope outline with flap chevron.
pub(super) const MAIL: &[&str] = &[
    "M22 7a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2",
    "M22 7v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V7",
    "M22 7l-10 7L2 7",
];

// Lucide `smartphone.svg` — rounded rect with home-pill at the bottom.
pub(super) const SMARTPHONE: &[&str] = &[
    "M5 2h14a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z",
    "M11 18h2",
];

// Lucide `chrome.svg` — outer ring + concentric inner ring (r=4
// at viewBox centre 12,12) + three radial spokes. Previous inner
// path arced through (12, 8) so the hole rendered as an off-centre
// "G"-like crescent — visible in login.op's Google button.
pub(super) const CHROME: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
    "M21.17 8H12",
    "M3.95 6.06L8.54 14",
    "M10.88 21.94L15.5 14",
];

// Apple-mark glyph — single closed path; not in upstream lucide
// but kept here so `.op` files that author `iconFontName:"apple"` get
// a recognisable mark instead of an honest-but-blank placeholder.
pub(super) const APPLE: &[&str] = &[
    "M16 10c-.6 0-2.1.8-3.5.8-1.4 0-2.4-.8-3.6-.8-1.9 0-4.1 1.6-4.1 4.6 0 2.7 1.5 5.4 3.4 6.4 1 .5 1.7 0 2.3 0 .7 0 1.4.5 2.4 0 1.9-1 3.4-3.7 3.4-6.4 0-3-2.2-4.6-4.1-4.6Z",
    "M14 6c.8-1 1.4-2.2 1.1-3.5-.9.1-2.1.7-2.8 1.6-.7.8-1.3 2-1.1 3.2 1 .1 2-.4 2.8-1.3z",
];

// Lucide `user.svg` — head circle + shoulders arc.
pub(super) const USER: &[&str] = &[
    "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2",
    "M12 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
];

// ── Additional first-party glyphs surfaced via `Icon::from_name` ──
pub(super) const CLOCK: &[&str] = &["M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z", "M12 6v6l4 2"];
pub(super) const CALENDAR: &[&str] = &[
    "M8 2v4",
    "M16 2v4",
    "M3 8a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    "M3 10h18",
];
pub(super) const STAR: &[&str] = &[
    "M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z",
];
pub(super) const HEART: &[&str] = &[
    "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z",
];
pub(super) const HOME: &[&str] = &[
    "M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8",
    "M3 10a2 2 0 0 1 .709-1.528l7-5.999a2 2 0 0 1 2.582 0l7 5.999A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
];
pub(super) const BELL: &[&str] = &[
    "M10.268 21a2 2 0 0 0 3.464 0",
    "M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326",
];
pub(super) const PLAY: &[&str] =
    &["M6 3a1 1 0 0 1 1.539-.843l13 8a1 1 0 0 1 0 1.686l-13 8A1 1 0 0 1 6 19z"];
pub(super) const MAP_PIN: &[&str] = &[
    "M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0",
    "M12 12a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z",
];
pub(super) const PHONE: &[&str] = &[
    "M13.832 16.568a1 1 0 0 0 1.213-.303l.355-.465A2 2 0 0 1 17 15h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2A18 18 0 0 1 2 4a2 2 0 0 1 2-2h3a2 2 0 0 1 2 2v3a2 2 0 0 1-.8 1.6l-.468.351a1 1 0 0 0-.292 1.233 14 14 0 0 0 6.392 6.384",
];
pub(super) const CAMERA: &[&str] = &[
    "M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3z",
    "M12 17a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
];
pub(super) const VIDEO: &[&str] = &[
    "m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.87a.5.5 0 0 0-.752-.432L16 10.5",
    "M2 8a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z",
];
pub(super) const MUSIC: &[&str] = &[
    "M9 18V5l12-2v13",
    "M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
    "M21 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
];
pub(super) const SHARE: &[&str] = &[
    "M12 2v13",
    "m16 6-4-4-4 4",
    "M4 11v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8",
];
pub(super) const INFO: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "M12 16v-4",
    "M12 8h.01",
];
pub(super) const ALERT_CIRCLE: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "M12 8v4",
    "M12 16h.01",
];
pub(super) const HELP_CIRCLE: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3",
    "M12 17h.01",
];

pub(super) const CHEVRON_LEFT: &[&str] = &["m15 18-6-6 6-6"];
pub(super) const MORE_VERTICAL: &[&str] = &[
    "M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M12 6a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M12 20a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
];
pub(super) const MORE_HORIZONTAL: &[&str] = &[
    "M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M19 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M5 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
];

pub(super) const ARROW_RIGHT: &[&str] = &["M5 12h14", "M13 5l7 7-7 7"];
pub(super) const ARROW_LEFT: &[&str] = &["M19 12H5", "M11 5l-7 7 7 7"];
pub(super) const CHECK_CIRCLE: &[&str] =
    &["M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z", "m9 12 2 2 4-4"];
pub(super) const ALERT_TRIANGLE: &[&str] = &[
    "M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z",
    "M12 9v4",
    "M12 17h.01",
];
pub(super) const ALERT_OCTAGON: &[&str] = &[
    "M7.86 2h8.28L22 7.86v8.28L16.14 22H7.86L2 16.14V7.86z",
    "M12 8v4",
    "M12 16h.01",
];
pub(super) const STICKY_NOTE: &[&str] = &[
    "M15.5 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V8.5z",
    "M15 3v6h6",
];
pub(super) const BAR_CHART_2: &[&str] = &["M18 20V10", "M12 20V4", "M6 20v-6"];
pub(super) const BOLD: &[&str] = &["M6 12h9a4 4 0 0 1 0 8H6z", "M6 4h8a4 4 0 0 1 0 8H6z"];
pub(super) const ITALIC: &[&str] = &["M19 4h-9", "M14 20H5", "M15 4 9 20"];
pub(super) const UNDERLINE: &[&str] = &["M6 4v6a6 6 0 0 0 12 0V4", "M4 20h16"];
pub(super) const STRIKETHROUGH: &[&str] = &[
    "M16 4H9a3 3 0 0 0-2.83 4",
    "M14 12a4 4 0 0 1 0 8H6",
    "M4 12h16",
];
pub(super) const SHOPPING_CART: &[&str] = &[
    "M9 22a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M20 22a1 1 0 1 0 0-2 1 1 0 0 0 0 2z",
    "M1 1h4l2.68 13.39a2 2 0 0 0 2 1.61h9.72a2 2 0 0 0 2-1.61L23 6H6",
];
pub(super) const SHOPPING_BAG: &[&str] = &[
    "M6 2 3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z",
    "M3 6h18",
    "M16 10a4 4 0 0 1-8 0",
];
pub(super) const SEND: &[&str] = &["m22 2-7 20-4-9-9-4z", "M22 2 11 13"];
pub(super) const PAPERCLIP: &[&str] = &[
    "m16 6-8.414 8.586a2 2 0 0 0 2.829 2.829l8.414-8.586a4 4 0 1 0-5.657-5.657l-8.379 8.551a6 6 0 1 0 8.485 8.485l8.379-8.551",
];
pub(super) const MESSAGE_CIRCLE: &[&str] = &["M7.9 20A9 9 0 1 0 4 16.1L2 22z"];
pub(super) const ROCKET: &[&str] = &[
    "M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z",
    "M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z",
    "M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0",
    "M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5",
];
pub(super) const MENU: &[&str] = &["M4 12h16", "M4 6h16", "M4 18h16"];
pub(super) const CREDIT_CARD: &[&str] = &[
    "M3 5h18a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z",
    "M2 10h20",
];
pub(super) const X_CIRCLE: &[&str] = &[
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
    "m15 9-6 6",
    "m9 9 6 6",
];

pub(super) const TRENDING_UP: &[&str] = &["M16 7h6v6", "m22 7-8.5 8.5-5-5L2 17"];
pub(super) const TRENDING_DOWN: &[&str] = &["M16 17h6v-6", "m22 17-8.5-8.5-5 5L2 7"];
pub(super) const COMPASS: &[&str] = &[
    "m16.24 7.76-1.804 5.411a2 2 0 0 1-1.265 1.265L7.76 16.24l1.804-5.411a2 2 0 0 1 1.265-1.265z",
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z",
];
pub(super) const REFRESH_CW: &[&str] = &[
    "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8",
    "M21 3v5h-5",
    "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16",
    "M8 16H3v5",
];
pub(super) const LAYOUT_DASHBOARD: &[&str] = &[
    "M3 3h7a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
    "M14 3h7a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
    "M14 11h7a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1z",
    "M3 16h7a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1v-4a1 1 0 0 1 1-1z",
];
pub(super) const USERS: &[&str] = &[
    "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2",
    "M16 3.128a4 4 0 0 1 0 7.744",
    "M22 21v-2a4 4 0 0 0-3-3.87",
    "M5 7a4 4 0 1 0 8 0 4 4 0 0 0-8 0z",
];
pub(super) const PACKAGE: &[&str] = &[
    "M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z",
    "M12 22V12",
    // <polyline points="3.29 7 12 12 20.71 7"/>
    "M3.29 7L12 12L20.71 7",
    "m7.5 4.27 9 5.15",
];
pub(super) const ZAP: &[&str] = &[
    "M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z",
];
pub(super) const SLIDERS_HORIZONTAL: &[&str] = &[
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
pub(super) const ACTIVITY: &[&str] = &[
    "M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2",
];
pub(super) const LOADER: &[&str] = &[
    "M12 2v4",
    "m16.2 7.8 2.9-2.9",
    "M18 12h4",
    "m16.2 16.2 2.9 2.9",
    "M12 18v4",
    "m4.9 19.1 2.9-2.9",
    "M2 12h4",
    "m4.9 4.9 2.9 2.9",
];
pub(super) const FOCUS: &[&str] = &[
    // Inner circle: r=3 at (12,12) per lucide@0.545.0 — previously
    // shipped as r=4, which made the focus crosshair too large.
    "M9 12a3 3 0 1 0 6 0 3 3 0 0 0-6 0z",
    "M3 7V5a2 2 0 0 1 2-2h2",
    "M17 3h2a2 2 0 0 1 2 2v2",
    "M21 17v2a2 2 0 0 1-2 2h-2",
    "M7 21H5a2 2 0 0 1-2-2v-2",
];
pub(super) const CHART_LINE: &[&str] = &["M3 3v16a2 2 0 0 0 2 2h16", "m19 9-5 5-4-4-3 3"];
pub(super) const SETTINGS2: &[&str] = &[
    "M14 17H5",
    // Top track starts at x=19 (not 20) per lucide@0.545.0.
    "M19 7h-9",
    // Two filled circles, r=3 at (17,17) and (7,7).
    "M14 17a3 3 0 1 0 6 0 3 3 0 0 0-6 0z",
    "M4 7a3 3 0 1 0 6 0 3 3 0 0 0-6 0z",
];

// === Align toolbar icons (lucide@0.545.0) ===
// Rounded rects expanded to "M…H…A…V…A…H…A…V…A…Z" paths.

pub(super) const ALIGN_LEFT: &[&str] = &[
    // align-start-vertical: small + wide rect snapped to x=6, vert. spine at x=2.
    "M8 14 H13 A2 2 0 0 1 15 16 V18 A2 2 0 0 1 13 20 H8 A2 2 0 0 1 6 18 V16 A2 2 0 0 1 8 14 Z",
    "M8 4 H20 A2 2 0 0 1 22 6 V8 A2 2 0 0 1 20 10 H8 A2 2 0 0 1 6 8 V6 A2 2 0 0 1 8 4 Z",
    "M2 2v20",
];

pub(super) const ALIGN_CENTER_H: &[&str] = &[
    // align-center-vertical: center spine + two pill clips.
    "M12 2v20",
    "M8 10H4a2 2 0 0 1-2-2V6c0-1.1.9-2 2-2h4",
    "M16 10h4a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-4",
    "M8 20H7a2 2 0 0 1-2-2v-2c0-1.1.9-2 2-2h1",
    "M16 14h1a2 2 0 0 1 2 2v2a2 2 0 0 1-2 2h-1",
];

pub(super) const ALIGN_RIGHT: &[&str] = &[
    // align-end-vertical: rects flush to spine on right (x=22).
    "M4 4 H16 A2 2 0 0 1 18 6 V8 A2 2 0 0 1 16 10 H4 A2 2 0 0 1 2 8 V6 A2 2 0 0 1 4 4 Z",
    "M11 14 H16 A2 2 0 0 1 18 16 V18 A2 2 0 0 1 16 20 H11 A2 2 0 0 1 9 18 V16 A2 2 0 0 1 11 14 Z",
    "M22 22V2",
];

pub(super) const ALIGN_TOP: &[&str] = &[
    // align-start-horizontal: tall + short rect snapped to y=6, horiz. spine at y=2.
    "M6 6 H8 A2 2 0 0 1 10 8 V20 A2 2 0 0 1 8 22 H6 A2 2 0 0 1 4 20 V8 A2 2 0 0 1 6 6 Z",
    "M16 6 H18 A2 2 0 0 1 20 8 V13 A2 2 0 0 1 18 15 H16 A2 2 0 0 1 14 13 V8 A2 2 0 0 1 16 6 Z",
    "M22 2H2",
];

pub(super) const ALIGN_CENTER_V: &[&str] = &[
    // align-center-horizontal: center spine + two pill clips top/bottom.
    "M2 12h20",
    "M10 16v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-4",
    "M10 8V4a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v4",
    "M20 16v1a2 2 0 0 1-2 2h-2a2 2 0 0 1-2-2v-1",
    "M14 8V7c0-1.1.9-2 2-2h2a2 2 0 0 1 2 2v1",
];

pub(super) const ALIGN_BOTTOM: &[&str] = &[
    // align-end-horizontal: rects flush to spine on bottom (y=22).
    "M6 2 H8 A2 2 0 0 1 10 4 V16 A2 2 0 0 1 8 18 H6 A2 2 0 0 1 4 16 V4 A2 2 0 0 1 6 2 Z",
    "M16 9 H18 A2 2 0 0 1 20 11 V16 A2 2 0 0 1 18 18 H16 A2 2 0 0 1 14 16 V11 A2 2 0 0 1 16 9 Z",
    "M22 22H2",
];

pub(super) const DISTRIBUTE_H: &[&str] = &[
    // align-horizontal-distribute-center: two rects + tick marks above/below.
    "M6 5 H8 A2 2 0 0 1 10 7 V17 A2 2 0 0 1 8 19 H6 A2 2 0 0 1 4 17 V7 A2 2 0 0 1 6 5 Z",
    "M16 7 H18 A2 2 0 0 1 20 9 V15 A2 2 0 0 1 18 17 H16 A2 2 0 0 1 14 15 V9 A2 2 0 0 1 16 7 Z",
    "M17 22v-5",
    "M17 7V2",
    "M7 22v-3",
    "M7 5V2",
];

pub(super) const DISTRIBUTE_V: &[&str] = &[
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
pub(super) const LINE_HEIGHT: &[&str] = &[
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
pub(super) const MILESTONE: &[&str] = &[
    "M12 13v8",
    "M12 3v3",
    "M4 6a1 1 0 0 0-1 1v5a1 1 0 0 0 1 1h13a2 2 0 0 0 1.152-.365l3.424-2.317a1 1 0 0 0 0-1.635l-3.424-2.318A2 2 0 0 0 17 6z",
];

/// lucide `wrench` — used by TS for delegated/orchestrated tool cards.
pub(super) const WRENCH: &[&str] = &[
    "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94z",
];

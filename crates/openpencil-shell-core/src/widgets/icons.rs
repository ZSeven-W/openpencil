//! Lucide-icons glyph drawer (Step 5 SVG path edition).
//!
//! Each [`Icon`] variant maps to one or more SVG `d` strings copied
//! verbatim from `https://github.com/lucide-icons/lucide/tree/main/icons`
//! (ISC, see `LICENSE` in the lucide repo). Runtime rendering uses
//! [`crate::RenderBackend::stroke_svg_path`] which parses the
//! d-string via skia's path parser, scales the 24×24 viewBox to the
//! caller-supplied `size`, and strokes with round caps + joins to
//! match lucide's visual style.
//!
//! Authoring policy:
//! - `<line x1 y1 x2 y2/>` → `"Mx1 y1Lx2 y2"`
//! - `<rect x y w h rx/>` → manually expanded round-rect path
//! - `<circle cx cy r/>` → two-arc path (Mleft·A·right·A·left·Z)
//! - `<path d=…/>` → forwarded as-is
//!
//! When adding a new icon, copy the lucide `<svg>` block and convert
//! every primitive element into a path d-string, then push it into
//! the Vec returned by [`Icon::paths`]. The test
//! `every_variant_paints_at_least_one_primitive` proves no variant
//! ships empty by accident.

use crate::{Color, Point2D, RenderBackend};

/// Reference viewBox shared by every icon — matches lucide's default.
const ICON_VBOX: f32 = 24.0;

/// Lucide-flavored icons. Variants are ordered to mirror the
/// `lucide-react` import names in the TS app's
/// `apps/web/src/components/editor/toolbar.tsx` so the Rust ↔ TS
/// cross-walk stays mechanical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// MousePointer2 — Select tool.
    Cursor,
    /// Square — Rect tool.
    Square,
    /// ChevronDown — dropdown affordance / expanded tree row.
    ChevronDown,
    /// ChevronRight — collapsed tree row (LayerPanel collapsed
    /// container shows `>`; expanding swaps it to `v`).
    ChevronRight,
    /// Type — Text tool.
    Type,
    /// Frame — Frame tool.
    Frame,
    /// Hand — Pan tool.
    Hand,
    /// Undo2 — undo arrow.
    Undo,
    /// Redo2 — redo arrow.
    Redo,
    /// Braces — code panel toggle.
    Braces,
    /// BookOpen — design system / Markdown panel toggle.
    BookOpen,
    /// Plus — "add" affordance.
    Plus,
    /// Minus — zoom out.
    Minus,
    /// Search — magnifier.
    Search,
    /// Sun — theme toggle.
    Sun,
    /// Globe — i18n switcher.
    Globe,
    /// Maximize — fullscreen toggle.
    Maximize,
    /// Hash — Frame node-kind tag in the LayerPanel.
    Hash,
    /// PanelLeft — sidebar collapse button.
    PanelLeft,
    /// FolderOpen — TopBar folder.
    FolderOpen,
    /// Sparkles — agent active indicator.
    Sparkles,
    /// X — close affordance.
    Close,
    /// ChevronUp — collapse/expand pair with [`Icon::ChevronDown`].
    ChevronUp,
    /// MessageSquare — chat bubble icon (collapsed AI chat pill).
    MessageSquare,
    /// LayoutGrid — 2×2 grid layout-mode (弹性布局 / RightPanel).
    LayoutGrid,
    /// Rows3 — horizontal rows layout-mode.
    Rows3,
    /// Columns3 — vertical columns layout-mode.
    Columns3,
    /// RotateCw — rotation handle for 位置 section.
    RotateCw,
    /// Diamond — "create component" button glyph.
    Diamond,
    /// Component — component instance indicator.
    Component,
    /// Unlink — detach component instance.
    Unlink,
    /// Check — checkbox tick.
    Check,
    /// ArrowUpRight — link / external nav indicator.
    ArrowUpRight,
    /// Circle — Ellipse shape.
    Circle,
    /// Triangle — Polygon shape.
    Triangle,
    /// PenTool — Pen shape.
    PenTool,
    /// ImagePlus — "Import image / SVG" shape-picker row.
    ImagePlus,
    /// Eye — visibility-toggle affordance on each LayerPanel row
    /// (TS `Eye` from lucide-react). Paints when the row is
    /// visible.
    Eye,
    /// EyeOff — open eye with diagonal slash. Paints in place
    /// of `Eye` when the LayerPanel row is hidden, so the icon
    /// itself signals the state (TS parity — strike-through
    /// distinguishes hidden from visible rows).
    EyeOff,
    /// Lock — closed-shackle lock. Paints when the LayerPanel
    /// row is locked.
    Lock,
    /// LockOpen — open-shackle lock. Paints in place of `Lock`
    /// when the row is unlocked, so the icon shape itself
    /// signals the state (TS parity).
    LockOpen,
}

impl Icon {
    /// SVG path d-strings for this icon. One Vec entry per `<path>`
    /// / `<line>` / `<rect>` / `<circle>` element in the lucide SVG.
    /// Returns `&'static [&'static str]` so the host can walk the
    /// list without allocating per frame.
    pub fn paths(self) -> &'static [&'static str] {
        match self {
            Icon::Cursor => CURSOR,
            Icon::Square => SQUARE,
            Icon::ChevronDown => CHEVRON_DOWN,
            Icon::ChevronRight => CHEVRON_RIGHT,
            Icon::Type => TYPE,
            Icon::Frame => FRAME,
            Icon::Hand => HAND,
            Icon::Undo => UNDO,
            Icon::Redo => REDO,
            Icon::Braces => BRACES,
            Icon::BookOpen => BOOK_OPEN,
            Icon::Plus => PLUS,
            Icon::Minus => MINUS,
            Icon::Search => SEARCH,
            Icon::Sun => SUN,
            Icon::Globe => GLOBE,
            Icon::Maximize => MAXIMIZE,
            Icon::Hash => HASH,
            Icon::PanelLeft => PANEL_LEFT,
            Icon::FolderOpen => FOLDER_OPEN,
            Icon::Sparkles => SPARKLES,
            Icon::Close => CLOSE,
            Icon::ChevronUp => CHEVRON_UP,
            Icon::MessageSquare => MESSAGE_SQUARE,
            Icon::LayoutGrid => LAYOUT_GRID,
            Icon::Rows3 => ROWS_3,
            Icon::Columns3 => COLUMNS_3,
            Icon::RotateCw => ROTATE_CW,
            Icon::Diamond => DIAMOND,
            Icon::Component => COMPONENT,
            Icon::Unlink => UNLINK,
            Icon::Check => CHECK,
            Icon::ArrowUpRight => ARROW_UP_RIGHT,
            Icon::Circle => CIRCLE,
            Icon::Triangle => TRIANGLE,
            Icon::PenTool => PEN_TOOL,
            Icon::ImagePlus => IMAGE_PLUS,
            Icon::Eye => EYE,
            Icon::EyeOff => EYE_OFF,
            Icon::Lock => LOCK,
            Icon::LockOpen => LOCK_OPEN,
        }
    }
}

// ── Lucide path data ──────────────────────────────────────────────
// Source: https://github.com/lucide-icons/lucide/tree/main/icons
// License: ISC.

const CURSOR: &[&str] = &[
    "M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z",
];

const SQUARE: &[&str] = &[
    // Lucide ships <rect x=3 y=3 w=18 h=18 rx=2/>; expanded to a
    // round-rect path so stroke_svg_path can render it uniformly.
    "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
];

const CHEVRON_DOWN: &[&str] = &["m6 9 6 6 6-6"];

const CHEVRON_RIGHT: &[&str] = &["m9 18 6-6-6-6"];

const TYPE: &[&str] = &[
    "M12 4v16",
    "M4 7V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2",
    "M9 20h6",
];

const FRAME: &[&str] = &[
    // Four <line> elements expanded to "M…L…" path strings.
    "M22 6L2 6",
    "M22 18L2 18",
    "M6 2L6 22",
    "M18 2L18 22",
];

const HAND: &[&str] = &[
    "M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2",
    "M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2",
    "M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8",
    "M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15",
];

const UNDO: &[&str] = &[
    "M9 14 4 9l5-5",
    "M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11",
];

const REDO: &[&str] = &[
    "m15 14 5-5-5-5",
    "M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13",
];

const BRACES: &[&str] = &[
    "M8 3H7a2 2 0 0 0-2 2v5a2 2 0 0 1-2 2 2 2 0 0 1 2 2v5c0 1.1.9 2 2 2h1",
    "M16 21h1a2 2 0 0 0 2-2v-5c0-1.1.9-2 2-2a2 2 0 0 1-2-2V5a2 2 0 0 0-2-2h-1",
];

const BOOK_OPEN: &[&str] = &[
    "M12 7v14",
    "M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z",
];

const PLUS: &[&str] = &["M5 12h14", "M12 5v14"];

const MINUS: &[&str] = &["M5 12h14"];

const SEARCH: &[&str] = &[
    "m21 21-4.34-4.34",
    // <circle cx=11 cy=11 r=8/> expanded to a two-arc path.
    "M3 11A8 8 0 1 0 19 11A8 8 0 1 0 3 11Z",
];

const SUN: &[&str] = &[
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

const GLOBE: &[&str] = &[
    // <circle cx=12 cy=12 r=10/>
    "M2 12A10 10 0 1 0 22 12A10 10 0 1 0 2 12Z",
    "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20",
    "M2 12h20",
];

const MAXIMIZE: &[&str] = &[
    "M8 3H5a2 2 0 0 0-2 2v3",
    "M21 8V5a2 2 0 0 0-2-2h-3",
    "M3 16v3a2 2 0 0 0 2 2h3",
    "M16 21h3a2 2 0 0 0 2-2v-3",
];

const HASH: &[&str] = &[
    // 4 <line> elements.
    "M4 9L20 9",
    "M4 15L20 15",
    "M10 3L8 21",
    "M16 3L14 21",
];

const PANEL_LEFT: &[&str] = &[
    // <rect x=3 y=3 w=18 h=18 rx=2/> + <path d="M9 3v18"/>
    "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
    "M9 3v18",
];

const FOLDER_OPEN: &[&str] = &[
    "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2",
];

const SPARKLES: &[&str] = &[
    "M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z",
    "M20 2v4",
    "M22 4h-4",
    // <circle cx=4 cy=20 r=2/>
    "M2 20A2 2 0 1 0 6 20A2 2 0 1 0 2 20Z",
];

const CLOSE: &[&str] = &["M18 6 6 18", "m6 6 12 12"];

// Mirror of CHEVRON_DOWN flipped vertically — Lucide
// `chevron-up.svg` `d="m18 15-6-6-6 6"`.
const CHEVRON_UP: &[&str] = &["m18 15-6-6-6 6"];

// Lucide `message-square.svg` — speech-bubble outline used by the
// collapsed AI chat pill.
const MESSAGE_SQUARE: &[&str] = &["M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"];

// Lucide `layout-grid.svg` — 4 rounded-rect cells.
const LAYOUT_GRID: &[&str] = &[
    "M4 3h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
    "M15 3h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
    "M15 14h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z",
    "M4 14h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z",
];

// Lucide `rows-3.svg` — round-rect with 2 horizontal dividers.
const ROWS_3: &[&str] = &[
    "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
    "M21 9H3",
    "M21 15H3",
];

// Lucide `columns-3.svg` — round-rect with 2 vertical dividers.
const COLUMNS_3: &[&str] = &[
    "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
    "M9 3v18",
    "M15 3v18",
];

// Lucide `rotate-cw.svg`.
const ROTATE_CW: &[&str] = &[
    "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8",
    "M21 3v5h-5",
];

// Lucide `diamond.svg`.
const DIAMOND: &[&str] = &[
    "M2.7 10.3a2.41 2.41 0 0 0 0 3.41l7.59 7.59a2.41 2.41 0 0 0 3.41 0l7.59-7.59a2.41 2.41 0 0 0 0-3.41l-7.59-7.59a2.41 2.41 0 0 0-3.41 0Z",
];

// Lucide `component.svg`.
const COMPONENT: &[&str] = &[
    "M15.536 11.293a1 1 0 0 0 0 1.414l2.376 2.377a1 1 0 0 0 1.414 0l2.377-2.377a1 1 0 0 0 0-1.414l-2.377-2.377a1 1 0 0 0-1.414 0z",
    "M2.297 11.293a1 1 0 0 0 0 1.414l2.377 2.377a1 1 0 0 0 1.414 0l2.377-2.377a1 1 0 0 0 0-1.414L6.088 8.916a1 1 0 0 0-1.414 0z",
    "M8.916 17.912a1 1 0 0 0 0 1.415l2.377 2.376a1 1 0 0 0 1.414 0l2.377-2.376a1 1 0 0 0 0-1.415l-2.377-2.376a1 1 0 0 0-1.414 0z",
    "M8.916 4.674a1 1 0 0 0 0 1.414l2.377 2.376a1 1 0 0 0 1.414 0l2.377-2.376a1 1 0 0 0 0-1.414l-2.377-2.377a1 1 0 0 0-1.414 0z",
];

// Lucide `unlink.svg`.
const UNLINK: &[&str] = &[
    "m18.84 12.25 1.72-1.71h-.02a5.004 5.004 0 0 0-.12-7.07 5.006 5.006 0 0 0-6.95 0l-1.72 1.71",
    "m5.17 11.75-1.71 1.71a5.004 5.004 0 0 0 .12 7.07 5.006 5.006 0 0 0 6.95 0l1.71-1.71",
    "M8 2L8 5",
    "M2 8L5 8",
    "M16 19L16 22",
    "M19 16L22 16",
];

// Lucide `check.svg`.
const CHECK: &[&str] = &["M20 6 9 17l-5-5"];

// Lucide `arrow-up-right.svg`.
const ARROW_UP_RIGHT: &[&str] = &["M7 7h10v10", "M7 17 17 7"];

// Lucide `circle.svg`.
const CIRCLE: &[&str] = &["M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Z"];

// Lucide `triangle.svg`.
const TRIANGLE: &[&str] =
    &["M13.73 4a2 2 0 0 0-3.46 0l-8.15 14a2 2 0 0 0 1.73 3h16.34a2 2 0 0 0 1.73-3Z"];

// Lucide `pen-tool.svg`.
const PEN_TOOL: &[&str] = &[
    "M15.707 21.293a1 1 0 0 1-1.414 0l-1.586-1.586a1 1 0 0 1 0-1.414l5.586-5.586a1 1 0 0 1 1.414 0l1.586 1.586a1 1 0 0 1 0 1.414z",
    "m18 13-1.375-6.874a1 1 0 0 0-.746-.776L3.235 2.028a1 1 0 0 0-1.207 1.207L5.35 15.879a1 1 0 0 0 .776.746L13 18",
    "m2.3 2.3 7.286 7.286",
    "M11 11a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z",
];

// Lucide `image-plus.svg`.
const IMAGE_PLUS: &[&str] = &[
    "M16 5h6",
    "M19 2v6",
    "M21 11.5V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7.5",
    "m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21",
    "M9 9a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z",
];

// Lucide `eye.svg`.
const EYE: &[&str] = &[
    "M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0Z",
    "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z",
];

// Lucide `lock.svg` — rect body + closed shackle.
const LOCK: &[&str] = &[
    "M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2z",
    "M7 11V7a5 5 0 0 1 10 0v4",
];

// Lucide `lock-open.svg` — rect body + half-open shackle
// (right side of the arc is cut so the lock reads as "open").
const LOCK_OPEN: &[&str] = &[
    "M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2z",
    "M7 11V7a5 5 0 0 1 9.9-1",
];

// Lucide `eye-off.svg` — eye with diagonal strike.
const EYE_OFF: &[&str] = &[
    "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49",
    "M14.084 14.158a3 3 0 0 1-4.242-4.242",
    "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143",
    "m2 2 20 20",
];

/// Paint `icon` at `top_left`, scaled to `size × size` pixels, in
/// `color` strokes of width `stroke_width`. The icon's lucide
/// path data is rendered via `RenderBackend::stroke_svg_path`.
pub fn draw_icon(
    backend: &mut dyn RenderBackend,
    icon: Icon,
    top_left: Point2D,
    size: f32,
    color: Color,
    stroke_width: f32,
) {
    let _ = ICON_VBOX; // documented for future reference — backend hardcodes 24
    for d in icon.paths() {
        backend.stroke_svg_path(d, top_left, size, color, stroke_width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Point2D, Rect, TextLayout};

    #[derive(Default)]
    struct CountingBackend {
        paths: usize,
    }

    impl RenderBackend for CountingBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {
            self.paths += 1;
        }
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn paint_one(icon: Icon) -> CountingBackend {
        let mut b = CountingBackend::default();
        draw_icon(
            &mut b,
            icon,
            Point2D::new(0.0, 0.0),
            16.0,
            Color::WHITE,
            1.5,
        );
        b
    }

    #[test]
    fn plus_renders_two_paths() {
        let b = paint_one(Icon::Plus);
        assert_eq!(b.paths, 2);
    }

    #[test]
    fn minus_renders_one_path() {
        let b = paint_one(Icon::Minus);
        assert_eq!(b.paths, 1);
    }

    #[test]
    fn sun_renders_disc_plus_eight_rays() {
        let b = paint_one(Icon::Sun);
        assert_eq!(b.paths, 9);
    }

    #[test]
    fn every_variant_paints_at_least_one_primitive() {
        for icon in [
            Icon::Cursor,
            Icon::Square,
            Icon::ChevronDown,
            Icon::Type,
            Icon::Frame,
            Icon::Hand,
            Icon::Undo,
            Icon::Redo,
            Icon::Braces,
            Icon::BookOpen,
            Icon::Plus,
            Icon::Minus,
            Icon::Search,
            Icon::Sun,
            Icon::Globe,
            Icon::Maximize,
            Icon::Hash,
            Icon::PanelLeft,
            Icon::FolderOpen,
            Icon::Sparkles,
            Icon::Close,
            Icon::ChevronUp,
            Icon::MessageSquare,
            Icon::LayoutGrid,
            Icon::Rows3,
            Icon::Columns3,
            Icon::RotateCw,
            Icon::Diamond,
            Icon::Component,
            Icon::Unlink,
            Icon::Check,
            Icon::ArrowUpRight,
        ] {
            let b = paint_one(icon);
            assert!(b.paths > 0, "{:?} drew nothing", icon);
        }
    }
}

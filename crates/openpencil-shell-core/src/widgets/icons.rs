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
    /// Trash2 — proper delete affordance (line-art trash can).
    Trash,
    /// Copy — duplicate / overlapping rectangles.
    Copy,
    /// Pencil — rename / edit affordance.
    Pencil,
    /// ArrowUp — move up.
    ArrowUp,
    /// ArrowDown — move down.
    ArrowDown,
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
    /// Lucide `github.svg` — used for the GitHub Copilot provider.
    Github,
    /// Lucide `bot.svg` — used for OpenCode / generic-bot providers.
    Bot,
    /// Lucide `terminal.svg` — used for CLI providers.
    Terminal,
    /// Lucide `image.svg` — used for the Images settings tab.
    Image,
    /// Lucide `settings.svg` — used for the System settings tab.
    Settings,
    /// Lucide `save.svg` — file save / save-as menu rows.
    Save,
    /// Lucide `download.svg` — export image menu row.
    Download,
    /// Lucide glyphs surfaced via `Icon::from_name` for canonical
    /// `icon_font` nodes. Covers the names that real `.op` files +
    /// the TS element-builders authored against — extend as new
    /// fixtures surface unknown lucide names.
    Mail,
    Smartphone,
    Chrome,
    Apple,
    User,
    Clock,
    Calendar,
    Star,
    Heart,
    Home,
    Bell,
    Play,
    MapPin,
    Phone,
    Camera,
    Video,
    Music,
    Share,
    Info,
    AlertCircle,
    HelpCircle,
    ChevronLeft,
    MoreVertical,
    MoreHorizontal,
    TrendingUp,
    TrendingDown,
    Compass,
    RefreshCw,
    LayoutDashboard,
    Users,
    Package,
    Zap,
    SlidersHorizontal,
    Activity,
    Loader,
    Focus,
    ChartLine,
    Settings2,
    ArrowRight,
    ArrowLeft,
    CheckCircle,
    AlertTriangle,
    AlertOctagon,
    StickyNote,
    BarChart2,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    ShoppingCart,
    ShoppingBag,
    Send,
    MessageCircle,
    Rocket,
    Menu,
    CreditCard,
    XCircle,
    /// Lucide `file-text.svg` — recent file rows.
    FileText,
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
            Icon::Trash => TRASH,
            Icon::Copy => COPY,
            Icon::Pencil => PENCIL,
            Icon::ArrowUp => ARROW_UP,
            Icon::ArrowDown => ARROW_DOWN,
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
            Icon::Github => GITHUB,
            Icon::Bot => BOT,
            Icon::Terminal => TERMINAL,
            Icon::Image => IMAGE,
            Icon::Settings => SETTINGS,
            Icon::Save => SAVE,
            Icon::Download => DOWNLOAD,
            Icon::FileText => FILE_TEXT,
            Icon::Mail => MAIL,
            Icon::Smartphone => SMARTPHONE,
            Icon::Chrome => CHROME,
            Icon::Apple => APPLE,
            Icon::User => USER,
            Icon::Clock => CLOCK,
            Icon::Calendar => CALENDAR,
            Icon::Star => STAR,
            Icon::Heart => HEART,
            Icon::Home => HOME,
            Icon::Bell => BELL,
            Icon::Play => PLAY,
            Icon::MapPin => MAP_PIN,
            Icon::Phone => PHONE,
            Icon::Camera => CAMERA,
            Icon::Video => VIDEO,
            Icon::Music => MUSIC,
            Icon::Share => SHARE,
            Icon::Info => INFO,
            Icon::AlertCircle => ALERT_CIRCLE,
            Icon::HelpCircle => HELP_CIRCLE,
            Icon::ChevronLeft => CHEVRON_LEFT,
            Icon::MoreVertical => MORE_VERTICAL,
            Icon::MoreHorizontal => MORE_HORIZONTAL,
            Icon::TrendingUp => TRENDING_UP,
            Icon::TrendingDown => TRENDING_DOWN,
            Icon::Compass => COMPASS,
            Icon::RefreshCw => REFRESH_CW,
            Icon::LayoutDashboard => LAYOUT_DASHBOARD,
            Icon::Users => USERS,
            Icon::Package => PACKAGE,
            Icon::Zap => ZAP,
            Icon::SlidersHorizontal => SLIDERS_HORIZONTAL,
            Icon::Activity => ACTIVITY,
            Icon::Loader => LOADER,
            Icon::Focus => FOCUS,
            Icon::ChartLine => CHART_LINE,
            Icon::Settings2 => SETTINGS2,
            Icon::ArrowRight => ARROW_RIGHT,
            Icon::ArrowLeft => ARROW_LEFT,
            Icon::CheckCircle => CHECK_CIRCLE,
            Icon::AlertTriangle => ALERT_TRIANGLE,
            Icon::AlertOctagon => ALERT_OCTAGON,
            Icon::StickyNote => STICKY_NOTE,
            Icon::BarChart2 => BAR_CHART_2,
            Icon::Bold => BOLD,
            Icon::Italic => ITALIC,
            Icon::Underline => UNDERLINE,
            Icon::Strikethrough => STRIKETHROUGH,
            Icon::ShoppingCart => SHOPPING_CART,
            Icon::ShoppingBag => SHOPPING_BAG,
            Icon::Send => SEND,
            Icon::MessageCircle => MESSAGE_CIRCLE,
            Icon::Rocket => ROCKET,
            Icon::Menu => MENU,
            Icon::CreditCard => CREDIT_CARD,
            Icon::XCircle => X_CIRCLE,
        }
    }

    /// Resolve a lucide kebab-case glyph name to an [`Icon`]. Used by
    /// the canonical `.op` loader: `IconFontNode.iconFontName` carries
    /// strings like `pen-tool` / `mail` / `eye-off`; the renderer
    /// looks them up here so authored icons paint as lucide glyphs.
    /// Returns `None` for names the chrome doesn't carry — the
    /// renderer falls back to an honest placeholder rather than
    /// silently dropping the node.
    pub fn from_name(name: &str) -> Option<Icon> {
        Some(match name {
            "pen-tool" => Icon::PenTool,
            "mail" => Icon::Mail,
            "lock" => Icon::Lock,
            "lock-open" | "unlock" => Icon::LockOpen,
            "eye" => Icon::Eye,
            "eye-off" => Icon::EyeOff,
            "smartphone" | "mobile" => Icon::Smartphone,
            "chrome" => Icon::Chrome,
            "apple" => Icon::Apple,
            "user" | "person" => Icon::User,
            "search" => Icon::Search,
            "settings" => Icon::Settings,
            "image" | "image-icon" => Icon::Image,
            "github" => Icon::Github,
            "globe" => Icon::Globe,
            "terminal" => Icon::Terminal,
            "trash" | "trash-2" => Icon::Trash,
            "plus" => Icon::Plus,
            "minus" => Icon::Minus,
            "check" => Icon::Check,
            "x" | "close" => Icon::Close,
            "chevron-down" => Icon::ChevronDown,
            "chevron-right" => Icon::ChevronRight,
            "chevron-up" => Icon::ChevronUp,
            "arrow-up" => Icon::ArrowUp,
            "arrow-down" => Icon::ArrowDown,
            "arrow-up-right" => Icon::ArrowUpRight,
            "rotate-cw" => Icon::RotateCw,
            "pencil" | "edit" => Icon::Pencil,
            "copy" => Icon::Copy, "save" => Icon::Save, "download" => Icon::Download,
            "file-text" => Icon::FileText,
            "folder-open" | "folder" => Icon::FolderOpen,
            "sparkles" => Icon::Sparkles, "diamond" => Icon::Diamond,
            "component" => Icon::Component,
            "circle" => Icon::Circle, "triangle" => Icon::Triangle,
            "square" | "rectangle" => Icon::Square,
            "hash" => Icon::Hash, "type" | "text" => Icon::Type,
            "frame" => Icon::Frame, "hand" => Icon::Hand,
            "cursor" | "mouse-pointer" => Icon::Cursor,
            "maximize" | "fullscreen" => Icon::Maximize, "sun" => Icon::Sun,
            "panel-left" => Icon::PanelLeft,
            "braces" | "code" => Icon::Braces, "book-open" => Icon::BookOpen,
            "message-square" | "chat" => Icon::MessageSquare,
            "layout-grid" => Icon::LayoutGrid,
            "rows-3" | "rows" => Icon::Rows3, "columns-3" | "columns" => Icon::Columns3,
            "bot" => Icon::Bot,
            "undo" | "undo-2" => Icon::Undo, "redo" | "redo-2" => Icon::Redo,
            "clock" | "timer" => Icon::Clock,
            "calendar" | "calendar-days" => Icon::Calendar,
            "star" | "favorite" => Icon::Star,
            "heart" | "like" => Icon::Heart,
            "home" | "house" => Icon::Home,
            "bell" | "notifications" => Icon::Bell,
            "play" | "play-circle" => Icon::Play,
            "map-pin" | "pin" | "location" | "map-marker" => Icon::MapPin,
            "phone" | "telephone" => Icon::Phone,
            "camera" => Icon::Camera,
            "video" | "play-video" => Icon::Video,
            "music" | "music-2" => Icon::Music,
            "share" | "share-2" => Icon::Share,
            "info" | "information" => Icon::Info,
            "alert-circle" | "alert" | "warning" => Icon::AlertCircle,
            "help-circle" | "help" | "question" => Icon::HelpCircle,
            "chevron-left" | "back" => Icon::ChevronLeft,
            "more-vertical" | "more" | "ellipsis-vertical" | "dots-vertical" => Icon::MoreVertical,
            "more-horizontal" | "ellipsis" | "ellipsis-horizontal" | "dots-horizontal" => Icon::MoreHorizontal,
            "trending-up" | "trend-up" => Icon::TrendingUp,
            "trending-down" | "trend-down" => Icon::TrendingDown,
            "compass" | "navigation" => Icon::Compass,
            "refresh-cw" | "refresh" | "rotate" | "reload" => Icon::RefreshCw,
            "layout-dashboard" | "dashboard" => Icon::LayoutDashboard,
            "users" | "team" | "group" => Icon::Users,
            "package" | "box" => Icon::Package,
            "zap" | "lightning" | "bolt" => Icon::Zap,
            "sliders-horizontal" | "sliders" | "filter" => Icon::SlidersHorizontal,
            "activity" | "pulse" => Icon::Activity,
            "loader" | "spinner" => Icon::Loader,
            "focus" | "target" => Icon::Focus,
            "chart-line" | "line-chart" => Icon::ChartLine,
            "settings-2" | "tune" => Icon::Settings2,
            "arrow-right" | "forward" => Icon::ArrowRight,
            "arrow-left" => Icon::ArrowLeft,
            "check-circle" | "check-circle-2" => Icon::CheckCircle,
            "alert-triangle" | "warning-triangle" => Icon::AlertTriangle,
            "alert-octagon" | "octagon-alert" => Icon::AlertOctagon,
            "sticky-note" | "note" => Icon::StickyNote,
            "bar-chart" | "bar-chart-2" | "chart-bar" => Icon::BarChart2,
            "bold" => Icon::Bold,
            "italic" => Icon::Italic,
            "underline" => Icon::Underline,
            "strikethrough" => Icon::Strikethrough,
            "shopping-cart" | "cart" => Icon::ShoppingCart,
            "shopping-bag" | "bag" => Icon::ShoppingBag,
            "send" | "arrow-send" => Icon::Send,
            "message-circle" | "comment" => Icon::MessageCircle,
            "rocket" => Icon::Rocket,
            "menu" | "hamburger" => Icon::Menu,
            "credit-card" | "card" => Icon::CreditCard,
            "x-circle" | "cancel" => Icon::XCircle,
            _ => return None,
        })
    }
}

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

/// Paint a canonical `icon_font` node — lucide glyph by name,
/// scaled into `rect` with aspect preserved. Mirrors TS
/// `drawIconFont` (packages/pen-renderer/src/node-renderer.ts).
/// Unknown names stroke a small dot at the centre — same shape
/// as the TS `FALLBACK_ICON_D` (`M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0`)
/// so the user sees an honest "unknown glyph" mark instead of
/// a solid block.
pub fn paint_icon_font_node(
    backend: &mut dyn RenderBackend,
    name: &str,
    rect: crate::Rect,
    fill: Option<Color>,
) {
    let size = rect.size.x.min(rect.size.y).max(0.0);
    if size <= 0.0 { return; }
    let color = fill.unwrap_or(Color { r: 0.39, g: 0.45, b: 0.55, a: 1.0 });
    let top_left = Point2D::new(
        rect.origin.x + (rect.size.x - size) / 2.0,
        rect.origin.y + (rect.size.y - size) / 2.0,
    );
    let stroke_width = (size / 24.0 * 2.0).max(1.0);
    if let Some(icon) = Icon::from_name(name) {
        draw_icon(backend, icon, top_left, size, color, stroke_width);
    } else {
        backend.stroke_svg_path(FALLBACK_ICON_D, top_left, size, color, stroke_width);
    }
}

/// Lucide-style dot glyph — small filled circle at viewBox centre.
/// Used as the unknown-icon fallback (TS parity with
/// `FALLBACK_ICON_D` in `node-renderer.ts`).
const FALLBACK_ICON_D: &str = "M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0";

use super::icons_data::*;
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

    #[test]
    fn first_party_icon_font_names_all_resolve() {
        // Every `iconFontName` value emitted by the TS element-builders
        // — both literal `iconFontName: '...'` AND values fed through
        // builder defaults / param indirection (e.g. callout severity
        // maps, input-with-action's `action_icon` default, toolbar-v1
        // formatting glyphs) — must resolve via `Icon::from_name`.
        // Scanned across `packages/pen-core/src/element-builders/` on
        // 2026-05-13; extend this list when new names land.
        for name in [
            // Direct iconFontName literals
            "calendar", "check", "chevron-down", "chevron-left", "chevron-right",
            "clock", "map-pin", "more-vertical", "play", "search", "star", "x",
            // Builder defaults / indirection
            "arrow-right", "check-circle", "alert-triangle", "alert-octagon",
            "sticky-note", "bar-chart-2", "bold", "italic", "underline",
            "shopping-cart", "shopping-bag", "message-circle", "rocket",
            "menu", "credit-card",
            // pencil-demo.op fixture sweep (2026-05-13) — covers
            // 56 occurrences that previously fell through.
            "trending-up", "trending-down", "compass", "refresh-cw",
            "layout-dashboard", "users", "package", "zap",
            "sliders-horizontal", "activity", "loader", "focus",
            "chart-line", "settings-2",
        ] {
            assert!(
                Icon::from_name(name).is_some(),
                "first-party iconFontName {:?} fell through to placeholder",
                name
            );
        }
    }
}

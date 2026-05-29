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
    /// Sun — theme toggle (paints in dark mode).
    Sun,
    /// Moon — theme toggle (paints in light mode).
    Moon,
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
    /// GitBranch — TopBar git-panel toggle next to the file name.
    GitBranch,
    /// History — git empty-state clock glyph.
    History,
    /// FilePlus — git empty-state "Init / create local history" card.
    FilePlus,
    /// GitFork — git empty-state "Clone from a remote" card.
    GitFork,
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
    /// LayoutGrid — 2×2 grid layout-mode (flex layout / RightPanel).
    LayoutGrid,
    /// Rows3 — horizontal rows layout-mode.
    Rows3,
    /// Columns3 — vertical columns layout-mode.
    Columns3,
    /// RotateCw — rotation handle for the Position section.
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
    Paperclip,
    MessageCircle,
    Rocket,
    Menu,
    CreditCard,
    XCircle,
    /// Lucide `file-text.svg` — recent file rows.
    FileText,
    /// Lucide `align-start-vertical` — align selection's left edges.
    AlignLeft,
    /// Lucide `align-center-vertical` — align selection horizontal centers.
    AlignCenterH,
    /// Lucide `align-end-vertical` — align selection's right edges.
    AlignRight,
    /// Lucide `align-start-horizontal` — align selection's top edges.
    AlignTop,
    /// Lucide `align-center-horizontal` — align selection vertical centers.
    AlignCenterV,
    /// Lucide `align-end-horizontal` — align selection's bottom edges.
    AlignBottom,
    /// Lucide `align-horizontal-distribute-center` — equal center spacing on X.
    DistributeH,
    /// Lucide `align-vertical-distribute-center` — equal center spacing on Y.
    DistributeV,
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
            Icon::Moon => MOON,
            Icon::Globe => GLOBE,
            Icon::Maximize => MAXIMIZE,
            Icon::Hash => HASH,
            Icon::PanelLeft => PANEL_LEFT,
            Icon::FolderOpen => FOLDER_OPEN,
            Icon::GitBranch => GIT_BRANCH,
            Icon::History => HISTORY,
            Icon::FilePlus => FILE_PLUS,
            Icon::GitFork => GIT_FORK,
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
            Icon::Paperclip => PAPERCLIP,
            Icon::MessageCircle => MESSAGE_CIRCLE,
            Icon::Rocket => ROCKET,
            Icon::Menu => MENU,
            Icon::CreditCard => CREDIT_CARD,
            Icon::XCircle => X_CIRCLE,
            Icon::AlignLeft => ALIGN_LEFT,
            Icon::AlignCenterH => ALIGN_CENTER_H,
            Icon::AlignRight => ALIGN_RIGHT,
            Icon::AlignTop => ALIGN_TOP,
            Icon::AlignCenterV => ALIGN_CENTER_V,
            Icon::AlignBottom => ALIGN_BOTTOM,
            Icon::DistributeH => DISTRIBUTE_H,
            Icon::DistributeV => DISTRIBUTE_V,
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
            "copy" => Icon::Copy,
            "save" => Icon::Save,
            "download" => Icon::Download,
            "file-text" => Icon::FileText,
            "folder-open" | "folder" => Icon::FolderOpen,
            "git-branch" | "git" => Icon::GitBranch,
            "history" | "clock-history" => Icon::History,
            "file-plus" => Icon::FilePlus,
            "git-fork" => Icon::GitFork,
            "sparkles" => Icon::Sparkles,
            "diamond" => Icon::Diamond,
            "component" => Icon::Component,
            "circle" => Icon::Circle,
            "triangle" => Icon::Triangle,
            "square" | "rectangle" => Icon::Square,
            "hash" => Icon::Hash,
            "type" | "text" => Icon::Type,
            "frame" => Icon::Frame,
            "hand" => Icon::Hand,
            "cursor" | "mouse-pointer" => Icon::Cursor,
            "maximize" | "fullscreen" => Icon::Maximize,
            "sun" => Icon::Sun,
            "moon" => Icon::Moon,
            "panel-left" => Icon::PanelLeft,
            "braces" | "code" => Icon::Braces,
            "book-open" => Icon::BookOpen,
            "message-square" | "chat" => Icon::MessageSquare,
            "layout-grid" => Icon::LayoutGrid,
            "rows-3" | "rows" => Icon::Rows3,
            "columns-3" | "columns" => Icon::Columns3,
            "bot" => Icon::Bot,
            "undo" | "undo-2" => Icon::Undo,
            "redo" | "redo-2" => Icon::Redo,
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
            "more-horizontal" | "ellipsis" | "ellipsis-horizontal" | "dots-horizontal" => {
                Icon::MoreHorizontal
            }
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
            "paperclip" | "attachment" => Icon::Paperclip,
            "message-circle" | "comment" => Icon::MessageCircle,
            "rocket" => Icon::Rocket,
            "menu" | "hamburger" => Icon::Menu,
            "credit-card" | "card" => Icon::CreditCard,
            "x-circle" | "cancel" => Icon::XCircle,
            "align-left" | "align-start-vertical" => Icon::AlignLeft,
            "align-center-h" | "align-center-vertical" | "align-horizontal-center" => {
                Icon::AlignCenterH
            }
            "align-right" | "align-end-vertical" => Icon::AlignRight,
            "align-top" | "align-start-horizontal" => Icon::AlignTop,
            "align-center-v" | "align-center-horizontal" | "align-vertical-center" => {
                Icon::AlignCenterV
            }
            "align-bottom" | "align-end-horizontal" => Icon::AlignBottom,
            "distribute-h" | "align-horizontal-distribute-center" | "distribute-horizontal" => {
                Icon::DistributeH
            }
            "distribute-v" | "align-vertical-distribute-center" | "distribute-vertical" => {
                Icon::DistributeV
            }
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

pub struct IconPathData<'a> {
    pub d: &'a str,
    pub style: super::icon_catalog::IconRenderStyle,
    pub viewbox: f32,
}

pub fn draw_icon_data(
    backend: &mut dyn RenderBackend,
    data: IconPathData<'_>,
    top_left: Point2D,
    size: f32,
    color: Color,
    stroke_width: f32,
) {
    match data.style {
        super::icon_catalog::IconRenderStyle::Stroke => {
            backend.stroke_svg_path(data.d, top_left, size, color, stroke_width);
        }
        super::icon_catalog::IconRenderStyle::Fill => {
            backend.fill_svg_path(data.d, top_left, size, data.viewbox.max(1.0), color);
        }
    }
}

pub fn draw_icon_catalog_entry(
    backend: &mut dyn RenderBackend,
    icon: &super::icon_catalog::IconCatalogEntry,
    top_left: Point2D,
    size: f32,
    color: Color,
    stroke_width: f32,
) {
    draw_icon_data(
        backend,
        IconPathData {
            d: &icon.d,
            style: icon.style,
            viewbox: icon.width.max(icon.height),
        },
        top_left,
        size,
        color,
        stroke_width,
    );
}

/// Paint a canonical `icon_font` node by Iconify collection + name,
/// scaled into `rect` with aspect preserved. Unknown names stroke a
/// small dot at the centre so the user sees an honest fallback mark.
pub fn paint_icon_font_node(
    backend: &mut dyn RenderBackend,
    family: &str,
    name: &str,
    rect: crate::Rect,
    fill: Option<Color>,
) {
    let size = rect.size.x.min(rect.size.y).max(0.0);
    if size <= 0.0 {
        return;
    }
    let color = fill.unwrap_or(Color {
        r: 0.39,
        g: 0.45,
        b: 0.55,
        a: 1.0,
    });
    let top_left = Point2D::new(
        rect.origin.x + (rect.size.x - size) / 2.0,
        rect.origin.y + (rect.size.y - size) / 2.0,
    );
    let stroke_width = (size / 24.0 * 2.0).max(1.0);
    let family = if family.trim().is_empty() {
        "lucide"
    } else {
        family.trim()
    };
    if let Some(icon) = super::icon_catalog::lookup_icon(family, name) {
        draw_icon_catalog_entry(backend, icon, top_left, size, color, stroke_width);
    } else if family == "lucide" {
        if let Some(icon) = Icon::from_name(name) {
            draw_icon(backend, icon, top_left, size, color, stroke_width);
        } else {
            backend.stroke_svg_path(FALLBACK_ICON_D, top_left, size, color, stroke_width);
        }
    } else {
        backend.stroke_svg_path(FALLBACK_ICON_D, top_left, size, color, stroke_width);
    }
}

/// Lucide-style dot glyph — small filled circle at viewBox centre.
/// Used as the unknown-icon fallback (TS parity with
/// `FALLBACK_ICON_D` in `node-renderer.ts`).
const FALLBACK_ICON_D: &str = "M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0";

use super::icons_data::*;

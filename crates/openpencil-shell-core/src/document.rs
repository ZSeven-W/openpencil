//! Step 2 kill-spike — minimal `Document` model for the Rust shell.
//!
//! TS-side `apps/web` ships `pen-types::PenDocument` with ~25 node
//! types, design variables, multi-page layout, fills/strokes/effects,
//! component instances, and so on. Step 2 starts the Rust mirror with
//! the smallest surface that lets shell-core editor-UI widgets
//! (LayerPanel / PropertyPanel / Toolbar) consume something
//! resembling a real document. Phase parity (per memory
//! `feedback_rust_port_feature_parity.md`: "TS → Rust 移植必须含
//! v0.8.0+ 全部功能") is a journey — this commit lays the spine.
//!
//! Shape choices (matching pen-types loosely):
//!
//! - [`NodeId(u64)`] is the stable identifier for tree nodes.
//!   0 is reserved for an absent / "no selection" sentinel — see
//!   [`NodeId::is_real`]. Real ids start at 1.
//! - [`NodeKind`] enumerates the document-side node kinds. Step 2
//!   covers Frame / Group / Rect / Text — enough for the editor-UI
//!   demo. Component instances / images / paths land in Step 3+.
//! - [`Node`] is a recursive tree node holding kind + name +
//!   children. Fills / strokes / position / size are deliberately
//!   omitted for Slice 1 — the editor UI only needs name + structure to
//!   draw the LayerPanel; PropertyPanel uses kind for the row set.
//! - [`Page`] groups nodes; [`Document`] holds pages plus a
//!   selection sentinel.
//!
//! Mobile + wasm32 clean: this module imports nothing platform-
//! specific. shell-core stays on the existing wasm32-clean cargo
//! check baseline (spec §1.2) AND the new mobile (iOS / Android)
//! widget render stack (per 2026-05-10 user directive).

/// Stable identifier for [`Node`]s within a [`Document`]. The layer
/// host assigns these from a counter at insertion time; editor-UI
/// widgets convert them to `widgets::WidgetId` via
/// [`NodeId::to_widget_id`] when they need the renderer-side id.
///
/// `NodeId::NONE` (inner value 0) is reserved as the "no node"
/// sentinel — used by [`Document::selected`] to mean "nothing
/// selected". Real ids start at 1.
///
/// The inner `u64` is **private** so the only way to mint an id is
/// via [`NodeId::new`] (panics on 0 in BOTH debug + release) or
/// the [`NodeId::NONE`] constant. Per codex Step 2 R1 BLOCK: the
/// previous `pub u64` field let release-mode callers construct
/// `NodeId(0)` Nodes that would shadow the NONE sentinel and
/// confuse `Document::selected_node`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl NodeId {
    /// Sentinel id for "no node" (used by `Document::selected` and
    /// by sample fixtures that need a placeholder).
    pub const NONE: NodeId = NodeId(0);

    /// Returns `true` for non-sentinel ids (id != 0).
    pub const fn is_real(self) -> bool {
        self.0 != 0
    }

    /// Construct a real (non-sentinel) id. Panics in BOTH debug
    /// and release builds if `id == 0` — callers that need the
    /// sentinel use [`NodeId::NONE`] directly. Hard panic (not
    /// `debug_assert`) per codex Step 2 R1 BLOCK fix.
    #[inline]
    pub const fn new(id: u64) -> Self {
        if id == 0 {
            panic!("NodeId::new(0) — id 0 is reserved for NodeId::NONE");
        }
        Self(id)
    }

    /// Inner numeric id. Used by `to_widget_id`, by serde (Step
    /// 4+), and by tests that need to inspect the underlying value.
    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Convert to an editor-UI-side `WidgetId`. The mapping is
    /// identity today; both share the `u64` payload and reserve 0
    /// as the "no node / no widget" sentinel. If the editor UI
    /// ever needs a wider numeric range (multiple widgets per
    /// node), this helper becomes the seam to evolve.
    #[inline]
    pub const fn to_widget_id(self) -> crate::widgets::WidgetId {
        crate::widgets::WidgetId(self.0)
    }
}

/// Node kinds covered by Step 2. Mirrors the most common subset of
/// `pen-types::PenNode.type` values seen in real documents:
/// frames hold layout, groups hold logical groupings, rects + text
/// are leaf primitives. The editor-UI demo uses `kind` to drive the
/// PropertyPanel's row set (different kinds expose different
/// properties).
///
/// `Other(String)` covers unknown / future kinds round-tripped from
/// a serialised document so the host never errors on a node it
/// doesn't recognise — the editor falls back to a generic property
/// row set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Frame,
    Group,
    Rect,
    Ellipse,
    Polygon,
    Line,
    Text,
    Other(String),
}

impl NodeKind {
    /// Short human-facing label used by the LayerPanel's secondary
    /// column and by the PropertyPanel header row.
    pub fn label(&self) -> &str {
        match self {
            NodeKind::Frame => "Frame",
            NodeKind::Group => "Group",
            NodeKind::Rect => "Rect",
            NodeKind::Ellipse => "Ellipse",
            NodeKind::Polygon => "Polygon",
            NodeKind::Line => "Line",
            NodeKind::Text => "Text",
            NodeKind::Other(s) => s.as_str(),
        }
    }
}

/// Solid stroke (outline) descriptor for [`Node::stroke`]. Step 3
/// keeps it minimal — color + width. Future `pen-types` parity adds
/// line cap / join / dash array.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub color: crate::Color,
    pub width: f32,
}

/// Document tree node — id + kind + display name + children, plus
/// Step 3 paint data:
/// - `bounds` (origin + size in document px) — `Rect::ZERO` for
///   container nodes that only use their children's bounds.
/// - `fill` (optional solid color) — None = no fill.
/// - `stroke` (optional outline) — None = no stroke.
/// - `text` (optional string) — populated for `NodeKind::Text` (the
///   `name` field is the layer-list label, this is the actual
///   rendered text).
///
/// Builder-style mutators (`with_bounds` / `with_fill` /
/// `with_stroke` / `with_text`) chain off `Node::leaf` /
/// `Node::with_children` so existing call sites keep working
/// while sample fixtures + Step 4+ document I/O can fluently
/// configure paint data.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: String,
    /// Document-space rectangle for this node. `Rect::ZERO` means
    /// "use children's bounds" / "container only".
    pub bounds: crate::Rect,
    pub fill: Option<crate::Color>,
    pub stroke: Option<Stroke>,
    /// Rendered text content — only meaningful for `NodeKind::Text`
    /// nodes. CanvasViewport draws this string at `bounds.origin`
    /// at a fixed font size; richer typography (per-run color,
    /// weight, etc.) lands in Step 4+ alongside `pen-types`
    /// `TextRun` parity.
    pub text: Option<String>,
    /// Rotation about the node's bounds center, in radians.
    /// Positive values rotate clockwise (matches the screen
    /// convention: y grows downward). 0 = no rotation.
    pub rotation: f32,
    /// Visibility flag — when true the canvas skips paint AND
    /// hit-test for this node and its subtree (the layer panel
    /// row still paints, dimmed). Toggled by the eye icon. TS
    /// parity with `pen-types::PenNode.visible`.
    pub hidden: bool,
    /// Lock flag — when true the canvas skips hit-test (the node
    /// can't be selected or dragged), but paint still happens.
    /// Toggled by the lock icon. TS parity with
    /// `pen-types::PenNode.locked`.
    pub locked: bool,
    /// LayerPanel-only expand state. When `true`, the row's
    /// children are hidden from the layer tree (chevron paints
    /// as `>`). Default `false` so containers start expanded.
    /// Canvas paint + hit-test ignore this flag — it's purely
    /// a tree-view toggle. TS parity with
    /// `pen-types::PenNode.collapsed`.
    pub collapsed: bool,
    /// Per-node fill type — drives the property panel's fill
    /// section body (solid hex / linear-gradient stops /
    /// radial-gradient stops / image fill). Defaults to
    /// `FillType::Solid`. Previously shared across all nodes
    /// via `Document.ui.fill_type`, which leaked the picker
    /// state across selection changes (codex full-audit
    /// CONCERN-fix 2026-05-11).
    pub fill_type: FillType,
    pub children: Vec<Node>,
}

impl Node {
    pub fn leaf(id: u64, kind: NodeKind, name: impl Into<String>) -> Self {
        Self {
            id: NodeId::new(id),
            kind,
            name: name.into(),
            bounds: crate::Rect::ZERO,
            fill: None,
            stroke: None,
            text: None,
            rotation: 0.0,
            hidden: false,
            locked: false,
            collapsed: false,
            fill_type: FillType::Solid,
            children: Vec::new(),
        }
    }

    pub fn with_children(
        id: u64,
        kind: NodeKind,
        name: impl Into<String>,
        children: Vec<Node>,
    ) -> Self {
        Self {
            id: NodeId::new(id),
            kind,
            name: name.into(),
            bounds: crate::Rect::ZERO,
            fill: None,
            stroke: None,
            text: None,
            rotation: 0.0,
            hidden: false,
            locked: false,
            collapsed: false,
            fill_type: FillType::Solid,
            children,
        }
    }

    /// Builder: set bounds (consume self, return new Node).
    pub fn with_bounds(mut self, bounds: crate::Rect) -> Self {
        self.bounds = bounds;
        self
    }

    /// Builder: set solid fill.
    pub fn with_fill(mut self, fill: crate::Color) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Builder: set stroke (color + width).
    pub fn with_stroke(mut self, color: crate::Color, width: f32) -> Self {
        self.stroke = Some(Stroke { color, width });
        self
    }

    /// Builder: set the rendered text (only meaningful when
    /// `kind == NodeKind::Text`).
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Search the subtree for a node with the given id, returning a
    /// borrow if found. Used by PropertyPanel to look up the
    /// currently-selected node when the editor host wants to draw
    /// per-node properties.
    pub fn find(&self, id: NodeId) -> Option<&Node> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(hit) = child.find(id) {
                return Some(hit);
            }
        }
        None
    }

    /// Effective bounds for the node — returns `bounds` directly
    /// when the node carries its own rect, otherwise unions the
    /// `aggregate_bounds` of every child (recursively). Container
    /// nodes (Group / Other / unbounded Frame) ship with
    /// `bounds = Rect::ZERO`; the property panel needs the visual
    /// extent of their subtree to show meaningful W/H rather than
    /// "0 × 0" (codex Step 6 stop-hook fix: "group bounds").
    pub fn aggregate_bounds(&self) -> crate::Rect {
        if self.bounds.size.x > 0.0 || self.bounds.size.y > 0.0 {
            return self.bounds;
        }
        let mut iter = self
            .children
            .iter()
            .map(Node::aggregate_bounds)
            .filter(|r| r.size.x > 0.0 || r.size.y > 0.0);
        let Some(first) = iter.next() else {
            return crate::Rect::ZERO;
        };
        let (mut min_x, mut min_y) = (first.origin.x, first.origin.y);
        let (mut max_x, mut max_y) = (first.origin.x + first.size.x, first.origin.y + first.size.y);
        for r in iter {
            min_x = min_x.min(r.origin.x);
            min_y = min_y.min(r.origin.y);
            max_x = max_x.max(r.origin.x + r.size.x);
            max_y = max_y.max(r.origin.y + r.size.y);
        }
        crate::Rect::xywh(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// A document page. `pen-types::PenPage` mirror — id + name +
/// top-level node children. Single-page documents (Step 2 default)
/// still go through this layer so the multi-page upgrade is just an
/// extension to `Document::pages.len() > 1`.
#[derive(Debug, Clone)]
pub struct Page {
    pub id: NodeId,
    pub name: String,
    pub children: Vec<Node>,
}

impl Page {
    pub fn new(id: u64, name: impl Into<String>, children: Vec<Node>) -> Self {
        Self {
            id: NodeId::new(id),
            name: name.into(),
            children,
        }
    }

    /// Walk the page's child forest looking for a node by id. Pages
    /// themselves are NOT searchable through this — only their
    /// descendants. The host typically asks "is the selection
    /// pointing at a real node?" via this helper.
    pub fn find(&self, id: NodeId) -> Option<&Node> {
        for child in &self.children {
            if let Some(hit) = child.find(id) {
                return Some(hit);
            }
        }
        None
    }
}

/// A document — the editor's subject. Holds pages, the active
/// page index, and a selection sentinel; the rest of the document
/// model (variables, components, styles, artboards) is Step 3+.
///
/// Selection is **page-scoped** by design (codex Step 2 R1
/// CONCERN-1): the editor UI only ever shows one page at a time
/// (`active_page_index`), so a "selection" outside that page is
/// effectively no selection. This keeps LayerPanel and
/// PropertyPanel pinned to the same content.
#[derive(Debug, Clone)]
pub struct Document {
    pub pages: Vec<Page>,
    /// Index into `pages` for the page currently shown in the
    /// editor viewport. Defaults to 0; multi-page picker UI lives
    /// in Step 3+.
    pub active_page_index: usize,
    /// Anchor selection — the most-recently clicked node id, or
    /// `NodeId::NONE` when nothing is selected. Always equal to
    /// the last entry in `selected_set` while a selection is
    /// active. Many single-select paths (property panel, color
    /// editors, etc.) read this field directly.
    pub selected: NodeId,
    /// Full selection set. Mirrors TS `canvas-store.selectedIds`.
    /// Empty when no selection is active; non-empty contains the
    /// anchor (`selected`) as its last entry. Maintained by the
    /// `set_single_selection` / `toggle_selection` / `clear_selection`
    /// / `select_all_top_level` helpers — callers should NOT mutate
    /// it directly (the helpers keep the anchor invariant).
    pub selected_set: Vec<NodeId>,
    /// Cross-action clipboard buffer. Filled by `copy_selected` /
    /// `cut_selected` with deep clones of the selected nodes
    /// (preserving the original ids); drained by `paste_clipboard`
    /// which mints fresh ids on the way out. Mirrors TS
    /// `canvas-store.clipboard`.
    pub clipboard: Vec<Node>,
    /// Currently-active editor tool. Mirrors the TS app's
    /// `canvas-store.tool` field — drives toolbar highlight + the
    /// canvas hit-test mode. Defaults to `Tool::Select`.
    pub tool: Tool,
    /// Pan/zoom state for the canvas viewport. Step 5 infinite
    /// canvas — `CanvasViewport` applies this transform when
    /// painting nodes; mouse wheel + Hand-tool drag mutate it.
    pub viewport: Viewport,
    /// AI chat panel state — input buffer, message list, focus.
    /// Step 5 P2: drives the floating AIChatPanel dynamic UI.
    pub chat: ChatState,
    /// Step 6: chrome layout flags (left sidebar open / closed,
    /// right rail visibility, etc). Mirrors the TS app's
    /// `canvas-store.layerPanelOpen` field.
    pub ui: UiState,
}

/// Chrome-level UI state — toggles for collapsible chrome
/// surfaces. Kept on `Document` so toggling propagates through
/// the same store as everything else.
#[derive(Debug, Clone)]
pub struct UiState {
    /// Whether the left LayerPanel is shown. Default true.
    pub sidebar_open: bool,
    /// Resizable LayerPanel width (logical px). Drag the right
    /// edge of the panel to resize.
    pub layer_panel_width: f32,
    /// Resizable PropertyPanel width (logical px). Drag the left
    /// edge of the panel to resize.
    pub property_panel_width: f32,
    /// Which property-panel input has keyboard focus. `None` =
    /// no input focused; the panel paints all inputs muted.
    pub property_focus: Option<PropertyFocus>,
    /// Draft string for the focused property-panel input. Filled
    /// from the snapshot value when a row is clicked, mutated by
    /// `apply_text` / `apply_backspace`, parsed + committed on
    /// Enter, discarded on Escape.
    pub property_input_draft: String,
    /// Caret-blink anchor for the focused property-panel input —
    /// reset on focus + every keystroke (mirrors `chat.caret_anchor_ms`).
    pub property_caret_anchor_ms: u64,
    /// Select-all-on-focus flag — true right after a click into
    /// an input, false after the first edit. While true, the next
    /// keystroke / backspace clears the seeded draft so the user
    /// can type a fresh value without backspacing first.
    pub property_draft_select_all: bool,
    /// Active theme — swapped by the TopBar Sun icon. Drives
    /// every widget's `Theme` lookup so the entire chrome flips
    /// together.
    pub theme_mode: ThemeMode,
    /// UI locale — cycled via the TopBar Globe icon. Drives the
    /// `t(key)` lookup widgets use for chrome strings.
    pub locale: Locale,
    /// Whether the TopBar Globe-icon dropdown is open. Click the
    /// Globe to toggle; click a row to set + close; click outside
    /// to close.
    pub locale_picker_open: bool,
    /// Whether the Toolbar shape-tool dropdown is open. The shape
    /// slot in the toolbar shows the icon for `shape_tool`; click
    /// it to toggle this picker. Picker rows: Rectangle / Ellipse /
    /// Polygon / Line / Icon / Import Image or SVG / Pen.
    pub shape_picker_open: bool,
    /// Last-selected shape tool — drives the toolbar shape slot's
    /// icon. Defaults to Rect. Always one of Rect / Ellipse /
    /// Polygon / Line / Pen (Icon + Import are one-shot actions
    /// that don't promote to the active tool).
    pub shape_tool: Tool,
    /// Active flex-layout mode for the property panel's "弹性布局"
    /// row. Lives on UiState until the Node schema grows a real
    /// LayoutSettings field; visual-only for now.
    pub flex_layout: FlexLayout,
    /// "尺寸" checkboxes — TS app stores these per-node; we
    /// surface them as document-level toggles until Node grows a
    /// SizeOptions field. Visual + interactive only.
    pub size_fill_width: bool,
    pub size_fill_height: bool,
    pub size_hug_width: bool,
    pub size_hug_height: bool,
    pub size_clip_content: bool,
    /// Whether the fill-type dropdown is open. The actual
    /// `FillType` value moved to `Node.fill_type` on 2026-05-11
    /// so each node remembers its own picker choice (codex
    /// full-audit CONCERN fix). The picker-open flag stays in
    /// UI state since it's transient overlay state, not a
    /// per-node property.
    pub fill_type_picker_open: bool,
    /// Currently-hovered LayerPanel row, or `None` when the
    /// cursor isn't over a row. Drives the hover-reveal of the
    /// eye + lock icons (TS parity: icons only appear on
    /// hovered / active rows). Host updates this on every
    /// cursor-move event.
    pub hovered_layer_id: Option<NodeId>,
}

/// Variants the Fill section's type-selector pill exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillType {
    Solid,
    LinearGradient,
    RadialGradient,
    Image,
}

/// Three flex-layout modes the property panel exposes — Free
/// (no layout), Vertical, Horizontal. Mirrors the TS app's
/// `layout.flexLayout` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexLayout {
    Free,
    Vertical,
    Horizontal,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sidebar_open: true,
            layer_panel_width: 240.0,
            property_panel_width: 280.0,
            property_focus: None,
            property_input_draft: String::new(),
            property_caret_anchor_ms: 0,
            property_draft_select_all: false,
            theme_mode: ThemeMode::Dark,
            locale: Locale::ZhCn,
            locale_picker_open: false,
            shape_picker_open: false,
            shape_tool: Tool::Rect,
            flex_layout: FlexLayout::Free,
            size_fill_width: false,
            size_fill_height: false,
            size_hug_width: false,
            size_hug_height: false,
            size_clip_content: false,
            fill_type_picker_open: false,
            hovered_layer_id: None,
        }
    }
}

/// Light/dark switch — light palette is stubbed in
/// [`crate::theme::Theme::light`] and ready to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn flipped(self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }
}

/// UI locale — full set of 15 mirrored from
/// `apps/web/src/i18n/locales/`. ZhCn + EnUs ship with complete
/// chrome translation tables; the rest fall through to EnUs
/// (visually obvious that translation is pending).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    EnUs,
    ZhCn,
    ZhTw,
    Ja,
    Ko,
    Fr,
    Es,
    De,
    Pt,
    Ru,
    Hi,
    Tr,
    Th,
    Vi,
    Id,
}

impl Locale {
    /// All locales in TopBar dropdown order — matches TS app.
    pub const ALL: [Locale; 15] = [
        Locale::EnUs,
        Locale::ZhCn,
        Locale::ZhTw,
        Locale::Ja,
        Locale::Ko,
        Locale::Fr,
        Locale::Es,
        Locale::De,
        Locale::Pt,
        Locale::Ru,
        Locale::Hi,
        Locale::Tr,
        Locale::Th,
        Locale::Vi,
        Locale::Id,
    ];

    /// Cycle to the next locale (round-trips through `ALL`).
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&l| l == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Native-script display name (matches the TS dropdown).
    pub fn display_name(self) -> &'static str {
        match self {
            Locale::EnUs => "English",
            Locale::ZhCn => "简体中文",
            Locale::ZhTw => "繁體中文",
            Locale::Ja => "日本語",
            Locale::Ko => "한국어",
            Locale::Fr => "Français",
            Locale::Es => "Español",
            Locale::De => "Deutsch",
            Locale::Pt => "Português",
            Locale::Ru => "Русский",
            Locale::Hi => "हिन्दी",
            Locale::Tr => "Türkçe",
            Locale::Th => "ไทย",
            Locale::Vi => "Tiếng Việt",
            Locale::Id => "Bahasa Indonesia",
        }
    }
}

/// Identifier for a property-panel input row. The host hit-tests
/// the panel and stores the focused input here so paint can
/// render the focused box with primary border + caret bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyFocus {
    PositionX,
    PositionY,
    Rotation,
    PositionR,
    SizeW,
    SizeH,
    Opacity,
    FillHex,
    StrokeHex,
    StrokeWidth,
}

mod chat;
pub use chat::{ChatAnchor, ChatMessage, ChatRole, ChatState};

/// Pan + zoom state for the infinite canvas. Mirrors the TS app's
/// `canvas-store.viewport` field (panX / panY / zoom).
///
/// Pan units are LOGICAL canvas pixels (top-left origin matches
/// the canvas widget rect). Zoom is a multiplier — 1.0 = 100%
/// (one canvas pixel = one document pixel), 2.0 = 200%, etc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

impl Viewport {
    /// Identity viewport — origin pan, 100% zoom.
    pub const IDENTITY: Viewport = Viewport {
        pan_x: 0.0,
        pan_y: 0.0,
        zoom: 1.0,
    };
    /// Min/max zoom — matches the TS app's `canvas-constants`
    /// (10% to 800%). Anything tighter clips usability without
    /// adding precision.
    pub const MIN_ZOOM: f32 = 0.1;
    pub const MAX_ZOOM: f32 = 8.0;

    /// Apply a wheel zoom step centered on `cursor` (canvas-local
    /// coordinates). Positive `delta` zooms in, negative out.
    /// Keeps the document point under the cursor stationary.
    pub fn zoom_at(&mut self, cursor: crate::Point2D, delta: f32) {
        let prev_zoom = self.zoom;
        let factor = (delta * 0.0015).exp();
        let new_zoom = (prev_zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        // Recover the document-space point at cursor BEFORE zoom,
        // then re-anchor pan so it stays at cursor AFTER zoom.
        let doc_x = (cursor.x - self.pan_x) / prev_zoom;
        let doc_y = (cursor.y - self.pan_y) / prev_zoom;
        self.zoom = new_zoom;
        self.pan_x = cursor.x - doc_x * new_zoom;
        self.pan_y = cursor.y - doc_y * new_zoom;
    }

    /// Translate pan by `(dx, dy)` (canvas-local pixels). Used by
    /// the Hand-tool drag.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.pan_x += dx;
        self.pan_y += dy;
    }

    /// Convert a canvas-local point to document space (inverse of
    /// the painted transform).
    pub fn to_document(&self, p: crate::Point2D) -> crate::Point2D {
        crate::Point2D::new(
            (p.x - self.pan_x) / self.zoom,
            (p.y - self.pan_y) / self.zoom,
        )
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Editor tool — what action a primary mouse drag on the canvas
/// performs. Subset of the TS `canvas-store` tool union; expanded
/// in lockstep as Rust shell hit-test logic catches up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rect,
    Ellipse,
    Polygon,
    Line,
    Pen,
    Text,
    Frame,
    Hand,
}

impl Tool {
    /// All tools, in toolbar display order. Single source of truth
    /// for the toolbar build path.
    pub const ALL: [Tool; 9] = [
        Tool::Select,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Polygon,
        Tool::Line,
        Tool::Pen,
        Tool::Text,
        Tool::Frame,
        Tool::Hand,
    ];

    /// Stable accesskit / DOM id token (lowercase ASCII).
    pub fn ident(self) -> &'static str {
        match self {
            Tool::Select => "select",
            Tool::Rect => "rect",
            Tool::Ellipse => "ellipse",
            Tool::Polygon => "polygon",
            Tool::Line => "line",
            Tool::Pen => "pen",
            Tool::Text => "text",
            Tool::Frame => "frame",
            Tool::Hand => "hand",
        }
    }

    /// Whether this tool sits inside the Toolbar's shape-tool
    /// dropdown (Rect / Ellipse / Polygon / Line / Pen). The
    /// shape slot in the toolbar paints whichever of these is
    /// currently active.
    pub fn is_shape(self) -> bool {
        matches!(
            self,
            Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line | Tool::Pen
        )
    }
}

mod mutators;
mod walkers;
pub use walkers::ReorderDirection;

#[cfg(test)]
mod tests_geometry;
#[cfg(test)]
mod tests_mutators;

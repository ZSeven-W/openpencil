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

/// Floating AI chat panel state — mirrors the TS app's
/// `useAIStore` (messages, input draft, focused flag).
#[derive(Debug, Clone)]
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub focused: bool,
    /// Which canvas corner the floating chat panel snaps to.
    /// User can drag the panel by its header; on release the
    /// host computes the nearest corner and updates this field.
    pub anchor: ChatAnchor,
    /// Collapsed state — when true the panel paints only the
    /// 36 px header strip (clicking the chevron toggles).
    pub collapsed: bool,
    /// Last user-action timestamp (focus / keystroke) in
    /// milliseconds — drives the caret blink phase via
    /// [`jian_core::anim::blink_visible`]. Reset on focus and on
    /// every key event so the caret reappears immediately when
    /// the user types instead of mid-blink.
    pub caret_anchor_ms: u64,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            focused: false,
            anchor: ChatAnchor::BottomLeft,
            collapsed: false,
            caret_anchor_ms: 0,
        }
    }
}

/// Which corner of the canvas region the AI chat panel sits in.
/// Step 5 P2: 4-corner edge snap on drag release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ChatAnchor {
    /// Pick the nearest corner to the given panel-center point
    /// inside the canvas rect. `(canvas_x0, canvas_y0)` is the
    /// canvas top-left, `(canvas_w, canvas_h)` its size.
    pub fn nearest(
        center: crate::Point2D,
        canvas_x0: f32,
        canvas_y0: f32,
        canvas_w: f32,
        canvas_h: f32,
    ) -> Self {
        let mid_x = canvas_x0 + canvas_w / 2.0;
        let mid_y = canvas_y0 + canvas_h / 2.0;
        let left = center.x < mid_x;
        let top = center.y < mid_y;
        match (top, left) {
            (true, true) => ChatAnchor::TopLeft,
            (true, false) => ChatAnchor::TopRight,
            (false, true) => ChatAnchor::BottomLeft,
            (false, false) => ChatAnchor::BottomRight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatState {
    /// Append the focused input as a new user message + a stub
    /// assistant echo, then clear the buffer. Real AI streaming
    /// lands in Step 6+ (matches TS app's `aiStore.send` flow).
    pub fn send(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }
        let user_msg = ChatMessage {
            role: ChatRole::User,
            content: trimmed.to_string(),
        };
        let echo = ChatMessage {
            role: ChatRole::Assistant,
            content: format!("(stub) Got it — \"{}\"", trimmed),
        };
        self.messages.push(user_msg);
        self.messages.push(echo);
        self.input.clear();
    }
}

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

impl Document {
    /// Active theme — driven by `ui.theme_mode`. Widgets call this
    /// instead of hardcoding `Theme::dark()` so the entire chrome
    /// flips together when the user clicks the TopBar Sun icon.
    pub fn theme(&self) -> crate::Theme {
        match self.ui.theme_mode {
            ThemeMode::Dark => crate::Theme::dark(),
            ThemeMode::Light => crate::Theme::light(),
        }
    }

    /// Translate a chrome string by key. Keys are stable English
    /// identifiers; values come from a per-locale table. Unknown
    /// keys fall through to the key itself so callers get a
    /// visible "missing translation" instead of an empty render.
    pub fn t<'a>(&self, key: &'a str) -> &'a str {
        crate::i18n::translate(self.ui.locale, key)
    }

    /// Empty document with one empty default page named "Page 1".
    /// Used by host smoke fixtures.
    pub fn empty() -> Self {
        Self {
            pages: vec![Page::new(1, "Page 1", Vec::new())],
            active_page_index: 0,
            selected: NodeId::NONE,
            selected_set: Vec::new(),
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
        }
    }

    /// Sample document for the Step 2/3 editor-UI demo: one page
    /// with a frame containing a title (text) + a button (group
    /// of rect + text). Driven by document data instead of
    /// hardcoded TreeWidget items. Selection is set to the title
    /// so PropertyPanel has something to render. Step 3 adds
    /// concrete geometry + fills + strokes + text content so the
    /// CanvasViewport can actually render a recognisable mock.
    pub fn sample() -> Self {
        use crate::{Color, Rect};

        // Id allocations: page=1, frame=10, title=11, button=12,
        // button_rect=13, button_text=14. Stable across runs so
        // tests can assert specific ids.
        //
        // Layout (document coordinates, top-left origin):
        //   Frame    (40, 40)–(360, 240)   white fill, black 1px stroke
        //     Title  (60, 60)–(*, *)       text "Hello OpenPencil", no bg
        //     Button group at (60, 130)
        //       Rect   (60, 130)–(180, 36) blue fill, no stroke
        //       Text   (76, 152)–(*, *)    text "Click me", no bg
        let title = Node::leaf(11, NodeKind::Text, "Title")
            .with_bounds(Rect::xywh(60.0, 60.0, 240.0, 28.0))
            .with_text("Hello OpenPencil");
        let button_rect = Node::leaf(13, NodeKind::Rect, "Button background")
            .with_bounds(Rect::xywh(60.0, 130.0, 180.0, 36.0))
            .with_fill(Color::BLUE);
        let button_text = Node::leaf(14, NodeKind::Text, "Click me")
            .with_bounds(Rect::xywh(76.0, 152.0, 160.0, 16.0))
            .with_text("Click me");
        let button = Node::with_children(
            12,
            NodeKind::Group,
            "Button",
            vec![button_rect, button_text],
        );
        let frame = Node::with_children(10, NodeKind::Frame, "Frame", vec![title, button])
            .with_bounds(Rect::xywh(40.0, 40.0, 360.0, 240.0))
            .with_fill(Color::WHITE)
            .with_stroke(Color::BLACK, 1.0);
        let doc = Self {
            pages: vec![Page::new(1, "Page 1", vec![frame])],
            active_page_index: 0,
            selected: NodeId::new(11), // "Title"
            selected_set: vec![NodeId::new(11)],
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
        };
        debug_assert!(
            doc.validate().is_ok(),
            "Document::sample() failed self-validation"
        );
        doc
    }

    /// The page currently shown in the editor viewport. Returns
    /// `None` if `active_page_index` is out of range (only happens
    /// after an external mutation that didn't preserve the
    /// invariant; callers can use `Document::validate` to detect).
    pub fn active_page(&self) -> Option<&Page> {
        self.pages.get(self.active_page_index)
    }

    /// Append a fresh empty page and switch to it. The page's id
    /// is minted past `max_node_id() + 1` so it can't collide with
    /// any existing node id; the name follows the `"Page N"` pattern
    /// (where N = pages.len() + 1 BEFORE the insert) to match the
    /// existing default-page-name convention. The new selection is
    /// cleared since the freshly-added page has no children.
    ///
    /// Returns the new page's index, or `None` when id allocation
    /// would overflow `u64::MAX`. Mirrors TS `addPage()` (the `+`
    /// button on the LayerPanel Pages header).
    pub fn add_page(&mut self) -> Option<usize> {
        let next_id = self.max_node_id().checked_add(1)?;
        let n = self.pages.len() + 1;
        let page = Page::new(next_id, format!("Page {}", n), Vec::new());
        self.pages.push(page);
        let new_index = self.pages.len() - 1;
        self.active_page_index = new_index;
        self.clear_selection();
        Some(new_index)
    }

    /// Get the anchor-selected node (TS `selectedIds[0]`). ONLY
    /// searches the active page (codex Step 2 R1 CONCERN-1). A
    /// selection on a non-active page returns `None`.
    pub fn selected_node(&self) -> Option<&Node> {
        if !self.selected.is_real() {
            return None;
        }
        self.active_page()?.find(self.selected)
    }

    /// True iff `id` is in the active selection set.
    pub fn is_selected(&self, id: NodeId) -> bool {
        self.selected_set.contains(&id)
    }

    /// Number of nodes in the active selection set.
    pub fn selection_count(&self) -> usize {
        self.selected_set.len()
    }

    /// Replace the selection with a single node + anchor on it.
    /// TS parity: `setSelection([id], id)`. Idempotent.
    pub fn set_single_selection(&mut self, id: NodeId) {
        if id.is_real() {
            self.selected_set.clear();
            self.selected_set.push(id);
            self.selected = id;
        } else {
            self.clear_selection();
        }
    }

    /// Shift-click semantics: if `id` is already in the set,
    /// remove it (and pick a new anchor); otherwise add it as
    /// the new anchor. TS parity: `toggleSelection(id)`.
    pub fn toggle_selection(&mut self, id: NodeId) {
        if !id.is_real() {
            return;
        }
        if let Some(pos) = self.selected_set.iter().position(|n| *n == id) {
            self.selected_set.remove(pos);
            // Anchor needs a new home. Last entry (most-recently
            // added survivor) is the natural choice.
            self.selected = self.selected_set.last().copied().unwrap_or(NodeId::NONE);
        } else {
            self.selected_set.push(id);
            self.selected = id;
        }
    }

    /// Clear both anchor + set. Idempotent.
    pub fn clear_selection(&mut self) {
        self.selected_set.clear();
        self.selected = NodeId::NONE;
    }

    /// Whether `id` resolves to a node that can be mutated via
    /// selection-aware helpers (`translate_selected`,
    /// `set_selected_bounds`, etc.). Hidden + locked nodes are
    /// non-editable; everything else is. Mirrors TS
    /// `isNodeEditable(id)` from `document-store`.
    pub fn is_editable(&self, id: NodeId) -> bool {
        let Some(node) = self.active_page().and_then(|p| p.find(id)) else {
            return false;
        };
        !node.hidden && !node.locked
    }

    /// Whether `id` AND every descendant are editable — stricter
    /// gate than `is_editable`, used by destructive ops
    /// (`delete_selected`) so deleting an editable Frame can't
    /// wipe a locked / hidden child along with it. A locked or
    /// hidden node anywhere in the subtree protects the
    /// ancestor.
    pub fn is_subtree_editable(&self, id: NodeId) -> bool {
        let Some(node) = self.active_page().and_then(|p| p.find(id)) else {
            return false;
        };
        subtree_all_editable(node)
    }

    /// Toggle the `hidden` flag on the node with this id. Returns
    /// true on success. Mirrors TS `useDocumentStore.toggleVisible`.
    pub fn toggle_node_hidden(&mut self, id: NodeId) -> bool {
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        toggle_hidden_walk(&mut page.children, id)
    }

    /// Toggle the `collapsed` flag on the node with this id —
    /// LayerPanel-only state, doesn't affect canvas paint or
    /// hit-test.
    pub fn toggle_node_collapsed(&mut self, id: NodeId) -> bool {
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        toggle_collapsed_walk(&mut page.children, id)
    }

    /// Toggle the `locked` flag on the node with this id.
    pub fn toggle_node_locked(&mut self, id: NodeId) -> bool {
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        toggle_locked_walk(&mut page.children, id)
    }

    /// Copy every node in the selection set into the clipboard
    /// (deep clones, original ids preserved). Returns true when
    /// at least one node was copied; false when nothing was
    /// selected. Mirrors TS `Cmd+C`.
    pub fn copy_selected(&mut self) -> bool {
        if self.selected_set.is_empty() {
            return false;
        }
        let Some(page) = self.active_page() else {
            return false;
        };
        let mut buf: Vec<Node> = Vec::with_capacity(self.selected_set.len());
        for id in &self.selected_set {
            if let Some(node) = page.find(*id) {
                buf.push(node.clone());
            }
        }
        if buf.is_empty() {
            return false;
        }
        self.clipboard = buf;
        true
    }

    /// Copy the selection into the clipboard then delete it.
    /// Returns true when both steps succeeded. Mirrors TS `Cmd+X`.
    pub fn cut_selected(&mut self) -> bool {
        if !self.copy_selected() {
            return false;
        }
        self.delete_selected()
    }

    /// Paste every clipboard node into the active page as a
    /// top-level sibling, offset by `(offset_doc_px, offset_doc_px)`,
    /// minting fresh ids from `next_id`. Replaces selection with
    /// the new ids. Returns the new ids in paste order, or empty
    /// when nothing was pasted (empty clipboard or id-allocator
    /// overflow). Mirrors TS `Cmd+V`.
    ///
    /// Anchor-aware insertion (paste-inside-container,
    /// paste-as-sibling) is the TS polish; v1 always pastes at
    /// the top level — matches TS's fallback path when no anchor
    /// is selected.
    pub fn paste_clipboard(&mut self, next_id: &mut u64, offset_doc_px: f32) -> Vec<NodeId> {
        if self.clipboard.is_empty() {
            return Vec::new();
        }
        let Some(safe) = self.max_node_id().checked_add(1) else {
            return Vec::new();
        };
        *next_id = (*next_id).max(safe);
        // Verify total subtree headroom before any mint so a
        // partially-pasted document is impossible on overflow.
        let total: u64 = self.clipboard.iter().map(subtree_size).sum();
        if next_id.checked_add(total).is_none() {
            return Vec::new();
        }
        // Clone clipboard out so `pages.get_mut` doesn't alias
        // `self.clipboard`.
        let originals = self.clipboard.clone();
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return Vec::new();
        };
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(originals.len());
        for original in &originals {
            let mut clone = deep_clone_with_new_ids(original, next_id);
            shift_subtree(&mut clone, offset_doc_px, offset_doc_px);
            new_ids.push(clone.id);
            page.children.push(clone);
        }
        if !new_ids.is_empty() {
            self.selected = *new_ids.last().unwrap();
            self.selected_set = new_ids.clone();
        }
        new_ids
    }

    /// Top-level node ids on the active page whose aggregate
    /// bounds intersect `rect` (doc space). Used by the marquee
    /// rect-select on release. Mirrors TS
    /// `SpatialIndex::searchRect`. Descends only into top-level
    /// children — same as the click hit-test, so the result set
    /// can be selected as a unit.
    pub fn nodes_intersecting_doc_rect(&self, rect: crate::Rect) -> Vec<NodeId> {
        let Some(page) = self.active_page() else {
            return Vec::new();
        };
        let nx = rect.origin.x.min(rect.origin.x + rect.size.x);
        let ny = rect.origin.y.min(rect.origin.y + rect.size.y);
        let nw = rect.size.x.abs();
        let nh = rect.size.y.abs();
        let mut out = Vec::new();
        for child in &page.children {
            let b = child.aggregate_bounds();
            if b.size.x <= 0.0 && b.size.y <= 0.0 {
                continue;
            }
            let bx = b.origin.x.min(b.origin.x + b.size.x);
            let by = b.origin.y.min(b.origin.y + b.size.y);
            let bw = b.size.x.abs();
            let bh = b.size.y.abs();
            // AABB intersection test.
            if bx + bw < nx || nx + nw < bx || by + bh < ny || ny + nh < by {
                continue;
            }
            out.push(child.id);
        }
        out
    }

    /// Whether the right-rail property panel should currently
    /// paint. Single source of truth so the host's
    /// `canvas_region` math, the panel's `for_selection_at`
    /// gate, and `apply_press` commit-on-blur all stay in lock-
    /// step. Today: single-select with a resolvable anchor.
    /// Multi-select hides pending an aggregated-properties UI;
    /// stale single anchors (e.g. selection points at a node on
    /// a non-active page, or an id that's been removed) hide too
    /// since the panel itself returns `None` from
    /// `for_selection_at` in that case (codex stop-hook fix:
    /// reserving the rail when the panel won't paint left a
    /// blank strip).
    pub fn property_panel_visible(&self) -> bool {
        self.selection_count() == 1 && self.selected_node().is_some()
    }

    /// Cmd/Ctrl+A — select every top-level node on the active
    /// page (TS parity with `getActivePageChildren(...).map(id)`).
    /// Anchor is the last node so subsequent edits read the
    /// top-of-stack as "primary".
    pub fn select_all_top_level(&mut self) -> bool {
        let Some(page) = self.active_page() else {
            return false;
        };
        if page.children.is_empty() {
            return false;
        }
        self.selected_set = page.children.iter().map(|n| n.id).collect();
        self.selected = self.selected_set.last().copied().unwrap_or(NodeId::NONE);
        true
    }

    /// Convenience: first page (or panic if pages is empty). Used
    /// by tests + sample fixtures that don't care about
    /// active-page semantics. New code should prefer
    /// `active_page` for the actual rendering target.
    pub fn first_page(&self) -> &Page {
        self.pages
            .first()
            .expect("Document::first_page on empty pages — use Document::empty for a default page")
    }

    /// Hit-test the active page at a document-space point. Returns
    /// the topmost node id whose bounds (or aggregate bounds for
    /// containers) contain `point`. Walks children in reverse z-
    /// order (last child = top-most) so a stack of overlapping
    /// rects resolves to the visually topmost one. `None` if the
    /// click is in canvas dead space or no active page exists.
    pub fn node_at_doc_point(&self, point: crate::Point2D) -> Option<NodeId> {
        let zoom = self.viewport.zoom.max(0.0001);
        let page = self.active_page()?;
        for child in page.children.iter().rev() {
            if let Some(hit) = hit_test_walk(child, point, zoom) {
                return Some(hit);
            }
        }
        None
    }

    /// Overwrite the selected node's rotation (radians, clockwise).
    pub fn set_selected_rotation(&mut self, radians: f32) {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return;
        };
        for child in &mut page.children {
            if set_rotation_walk(child, sel, radians) {
                return;
            }
        }
    }

    /// Overwrite the selected leaf node's bounds. Only updates
    /// nodes that carry their own bounds (size > 0); container
    /// nodes (Group / unbounded Frame) are skipped — their
    /// "bounds" are derived from children and resizing them needs
    /// per-child scaling which lands in a later milestone.
    pub fn set_selected_bounds(&mut self, bounds: crate::Rect) {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return;
        };
        for child in &mut page.children {
            if set_bounds_walk(child, sel, bounds) {
                return;
            }
        }
    }

    /// Translate every node in the selection set by `(dx, dy)`
    /// document px. Container nodes cascade to descendants
    /// (`translate_walk`'s subtree translate). When two selected
    /// nodes have an ancestor-descendant relationship, only the
    /// ancestor is translated so the descendant isn't shifted
    /// twice (TS parity with the dedup in `use-edit-shortcuts.ts`
    /// nudge handler). No-op when nothing is selected or the
    /// active page is missing.
    pub fn translate_selected(&mut self, dx: f32, dy: f32) {
        if self.selected_set.is_empty() {
            return;
        }
        // Filter out hidden + locked nodes — those aren't
        // mutable. Done up-front because the page borrow below
        // is mutable.
        let editable: Vec<NodeId> = self
            .selected_set
            .iter()
            .copied()
            .filter(|id| self.is_editable(*id))
            .collect();
        if editable.is_empty() {
            return;
        }
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return;
        };
        for target in &editable {
            // Skip if any ancestor (within the active page tree)
            // is also in the selection — that ancestor's cascade
            // already shifted this descendant.
            if !is_ancestor_in_set(&page.children, *target, &editable) {
                for child in page.children.iter_mut() {
                    if translate_walk(child, *target, dx, dy) {
                        break;
                    }
                }
            }
        }
    }

    /// Apply a parsed property edit to the selected node. Mirrors
    /// the TS `useDocumentStore` mutation handlers — only this
    /// helper writes back to bounds, so call sites can stay
    /// declarative ("commit X = 120" rather than "find the node,
    /// clone bounds, mutate one axis, write back").
    ///
    /// Returns `true` if the edit landed on a real node; `false`
    /// when there's no selection or the active page can't be
    /// found. Container nodes (Group / unbounded Frame) currently
    /// no-op — their bounds are derived from children — but the
    /// API still returns `true` because the host should still
    /// clear the input draft + focus.
    pub fn commit_property_edit(&mut self, focus: PropertyFocus, value: f32) -> bool {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return false;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        for child in &mut page.children {
            if commit_property_walk(child, sel, focus, value) {
                return true;
            }
        }
        false
    }

    /// Set the fill / stroke colour on the selected node. Used by
    /// the hex inputs in the property panel — split from
    /// `commit_property_edit` because Color isn't a single f32.
    /// Write the picker's fill-type choice to the selected
    /// node's `fill_type`. Editable-gated so locked / hidden
    /// nodes can't be mutated. Returns true when the edit
    /// lands. No-op when nothing is selected.
    pub fn set_selected_fill_type(&mut self, fill_type: FillType) -> bool {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return false;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        for child in &mut page.children {
            if set_fill_type_walk(child, sel, fill_type) {
                return true;
            }
        }
        false
    }

    pub fn set_selected_color(&mut self, is_fill: bool, color: crate::Color) -> bool {
        if !self.selected.is_real() || !self.is_editable(self.selected) {
            return false;
        }
        let sel = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        for child in &mut page.children {
            if set_color_walk(child, sel, is_fill, color) {
                return true;
            }
        }
        false
    }

    /// Remove every node in the selection set from its parent's
    /// children. Returns true on success (selection cleared
    /// after). No-op when nothing is selected.
    ///
    /// TS parity: `for id in selectedIds: removeNode(id)` from
    /// `use-edit-shortcuts.ts`. Used by Delete / Backspace.
    pub fn delete_selected(&mut self) -> bool {
        if self.selected_set.is_empty() {
            return false;
        }
        // Filter out hidden + locked nodes — those aren't
        // removable via the user-facing Delete shortcut. Use the
        // SUBTREE-editable gate so deleting an editable Frame
        // can't take a locked / hidden child down with it (codex
        // stop-hook BLOCK: "nested protected selections can
        // still be deleted via selected ancestor"). TS parity:
        // locked rows ignore Delete in `use-edit-shortcuts.ts`.
        let (deletable, kept): (Vec<NodeId>, Vec<NodeId>) = self
            .selected_set
            .iter()
            .copied()
            .partition(|id| self.is_subtree_editable(*id));
        if deletable.is_empty() {
            return false;
        }
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        let mut removed_any = false;
        for id in &deletable {
            if remove_from_children(&mut page.children, *id) {
                removed_any = true;
            }
        }
        if removed_any {
            // Anchor + set survive the protected ids (locked /
            // hidden nodes). If everything got deleted the set
            // collapses to empty.
            self.selected_set = kept;
            self.selected = self.selected_set.last().copied().unwrap_or(NodeId::NONE);
            true
        } else {
            false
        }
    }

    /// Clone every node in the selection set (deep-clone with
    /// fresh ids), insert each as the next sibling at
    /// `offset_doc_px` from the original, and replace the
    /// selection with the new ids. Returns the new anchor id
    /// (last clone) on success.
    ///
    /// TS parity: `selectedIds.map(duplicateNode)` from
    /// `use-clipboard-shortcuts.ts`. Used by Cmd/Ctrl+D.
    pub fn duplicate_selected(&mut self, next_id: &mut u64, offset_doc_px: f32) -> Option<NodeId> {
        if self.selected_set.is_empty() {
            return None;
        }
        // Lift the allocator past every existing id so we never
        // mint a duplicate even when the document was loaded
        // with ids greater than the host's running counter (codex
        // CONCERN: external docs with ids ≥ next_id would
        // otherwise silently collide).
        //
        // `checked_add(1)` instead of `saturating_add` so a
        // document carrying `NodeId(u64::MAX)` returns None
        // cleanly instead of saturating to u64::MAX and minting
        // a collision (the saturating overflow lane was a
        // theoretical edge but worth being explicit about).
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let targets: Vec<NodeId> = self.selected_set.clone();
        let page = self.pages.get_mut(self.active_page_index)?;
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(targets.len());
        for target in targets {
            if let Some(new_id) =
                duplicate_in_children(&mut page.children, target, next_id, offset_doc_px)
            {
                new_ids.push(new_id);
            }
        }
        if new_ids.is_empty() {
            return None;
        }
        self.selected = *new_ids.last().unwrap();
        self.selected_set = new_ids;
        Some(self.selected)
    }

    /// Largest `NodeId` (by raw value) anywhere in the document,
    /// across all pages. Used as a one-shot guard so the duplicate
    /// allocator can never collide with a real id.
    pub fn max_node_id(&self) -> u64 {
        let mut max = 0u64;
        for page in &self.pages {
            max = max.max(page.id.raw());
            for child in &page.children {
                max = max.max(max_id_walk(child));
            }
        }
        max
    }

    /// Bump the selected node up or down by one position in its
    /// parent's children vec, which changes its paint order
    /// (children paint earlier-to-later, so later index = on top).
    /// Returns true on success.
    ///
    /// TS parity: `reorderNode(id, 'up' | 'down')`. Bound to `]`
    /// (`Up` → towards front) and `[` (`Down` → towards back).
    pub fn reorder_selected(&mut self, direction: ReorderDirection) -> bool {
        if !self.selected.is_real() {
            return false;
        }
        let target = self.selected;
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return false;
        };
        reorder_in_children(&mut page.children, target, direction)
    }

    /// Clear the active selection. Distinct from
    /// `ui.property_focus` clear — Escape calls both. Alias for
    /// `clear_selection` kept for readability at call sites.
    pub fn deselect_all(&mut self) {
        self.clear_selection();
    }

    /// Walk every node id in every page, returning the first
    /// duplicate id found (or `None` if all ids are unique). Used
    /// by `Document::sample()` debug-asserts and by `validate`.
    /// Codex Step 2 R1 CONCERN-2: previously nothing checked id
    /// uniqueness, so a Document built with duplicate ids would
    /// have `selected_node` returning the first hit while
    /// LayerPanel might mark several rows "selected".
    pub fn find_duplicate_id(&self) -> Option<NodeId> {
        let mut seen = std::collections::HashSet::new();
        for page in &self.pages {
            // Pages share the id namespace with nodes, so include
            // page ids in the uniqueness scan.
            if !seen.insert(page.id) {
                return Some(page.id);
            }
            for child in &page.children {
                if let Some(dup) = find_duplicate_walk(child, &mut seen) {
                    return Some(dup);
                }
            }
        }
        None
    }

    /// Run light invariant checks on the document. Returns `Err`
    /// with a human-readable message on the first violation:
    /// - `pages` is empty (a document must have at least one
    ///   page; use `Document::empty()` to construct a default
    ///   single-page document)
    /// - duplicate node id anywhere in any page
    /// - `active_page_index` out of range
    ///
    /// Codex Step 2 R2 CONCERN-1: the prior version skipped the
    /// `active_page_index` check when `pages.is_empty()`, leaving
    /// a (Document { pages: vec![], active_page_index: 99, … })
    /// silently valid. Empty pages is itself an invariant
    /// violation; this version rejects it explicitly so the
    /// active_page_index check applies unconditionally.
    pub fn validate(&self) -> Result<(), String> {
        if self.pages.is_empty() {
            return Err("Document::pages is empty (use Document::empty() for the default single-page shape)".to_string());
        }
        if let Some(dup) = self.find_duplicate_id() {
            return Err(format!("duplicate NodeId: {:?}", dup));
        }
        if self.active_page_index >= self.pages.len() {
            return Err(format!(
                "active_page_index {} out of range (pages.len()={})",
                self.active_page_index,
                self.pages.len()
            ));
        }
        Ok(())
    }
}

/// Recursive helper for `Document::set_selected_color`.
fn set_color_walk(node: &mut Node, target: NodeId, is_fill: bool, color: crate::Color) -> bool {
    if node.id == target {
        if is_fill {
            node.fill = Some(color);
        } else {
            // Preserve stroke width if a stroke already exists;
            // otherwise spin one up at 1 px.
            let width = node.stroke.map(|s| s.width).unwrap_or(1.0);
            node.stroke = Some(Stroke { color, width });
        }
        return true;
    }
    for child in &mut node.children {
        if set_color_walk(child, target, is_fill, color) {
            return true;
        }
    }
    false
}

/// Recursive helper for `Document::set_selected_rotation`. Sets
/// the matched node's rotation in radians. Containers carry
/// rotation too — rendering applies the transform around the
/// node's aggregate-bounds center.
fn set_rotation_walk(node: &mut Node, target: NodeId, radians: f32) -> bool {
    if node.id == target {
        node.rotation = radians;
        return true;
    }
    for child in &mut node.children {
        if set_rotation_walk(child, target, radians) {
            return true;
        }
    }
    false
}

/// Recursive helper for `Document::set_selected_bounds`. Walks
/// the active page until it finds `target`, then overwrites its
/// bounds. Containers (bounds=ZERO) are no-ops.
fn set_bounds_walk(node: &mut Node, target: NodeId, new_bounds: crate::Rect) -> bool {
    if node.id == target {
        if node.bounds.size.x > 0.0 || node.bounds.size.y > 0.0 {
            node.bounds = new_bounds;
        }
        return true;
    }
    for child in &mut node.children {
        if set_bounds_walk(child, target, new_bounds) {
            return true;
        }
    }
    false
}

/// Recursive helper for `Document::node_at_doc_point` — returns
/// the topmost id whose bounds contain `point`. When a node
/// carries rotation, the test point is inverse-rotated about the
/// node's bounds centre BEFORE testing children + self, so the
/// hit area matches what the renderer paints.
fn hit_test_walk(node: &Node, point: crate::Point2D, zoom: f32) -> Option<NodeId> {
    // Hidden nodes skip canvas hit-test entirely — subtree
    // inherits (children of a hidden Frame are unclickable too).
    if node.hidden {
        return None;
    }
    let bounds = node.aggregate_bounds();
    // Rotation pivot is kind-aware so a Line node whose
    // `node.bounds` carries a negative dimension (collapsing the
    // aggregate to `Rect::ZERO`) still rotates around the segment
    // midpoint instead of (0, 0).
    let local = if node.rotation.abs() > f32::EPSILON {
        if let Some(pivot) = rotation_pivot(node, bounds) {
            let dx = point.x - pivot.x;
            let dy = point.y - pivot.y;
            let cos_t = (-node.rotation).cos();
            let sin_t = (-node.rotation).sin();
            crate::Point2D::new(
                pivot.x + dx * cos_t - dy * sin_t,
                pivot.y + dx * sin_t + dy * cos_t,
            )
        } else {
            point
        }
    } else {
        point
    };
    for child in node.children.iter().rev() {
        if let Some(hit) = hit_test_walk(child, local, zoom) {
            return Some(hit);
        }
    }
    // Locked nodes themselves can't be selected via canvas hit,
    // but their children still can (TS parity:
    // `spatial-index.ts` filters per-node, not subtree). This
    // check runs AFTER the child walk so descendants of a locked
    // Frame remain hittable; only the Frame body is opt-out.
    if node.locked {
        return None;
    }
    if point_in_node(node, local, bounds, zoom) {
        return Some(node.id);
    }
    None
}

/// Rotation pivot for hit-test. Most kinds rotate around the
/// aggregate-bounds center; Lines need a kind-specific path
/// because a Line with a negative-size dimension collapses its
/// aggregate to `Rect::ZERO` (the segment midpoint is still well
/// defined from `node.bounds`). Returns `None` when no valid
/// pivot exists (e.g. a degenerate Line that's just a point).
fn rotation_pivot(node: &Node, bounds: crate::Rect) -> Option<crate::Point2D> {
    if matches!(node.kind, NodeKind::Line) {
        let raw = node.bounds;
        if raw.size.x.abs() < f32::EPSILON && raw.size.y.abs() < f32::EPSILON {
            return None;
        }
        return Some(crate::Point2D::new(
            raw.origin.x + raw.size.x / 2.0,
            raw.origin.y + raw.size.y / 2.0,
        ));
    }
    if bounds.size.x > 0.0 && bounds.size.y > 0.0 {
        return Some(crate::Point2D::new(
            bounds.origin.x + bounds.size.x / 2.0,
            bounds.origin.y + bounds.size.y / 2.0,
        ));
    }
    None
}

/// Per-NodeKind hit-test. Frames / Groups / Rects / Text / Other
/// use the axis-aligned bounds, matching their fill paint. Ellipse
/// / Polygon / Line use tighter geometry so the click area matches
/// what `canvas_viewport::paint_node` actually paints (codex audit:
/// previously every kind used rect bounds, so users could "click
/// inside the bounding box but outside the painted oval/triangle/
/// stroke" and still select).
fn point_in_node(node: &Node, local: crate::Point2D, bounds: crate::Rect, zoom: f32) -> bool {
    // Lines get a dedicated path because:
    //   - horizontal / vertical segments have one zero dimension
    //     on the bounds rect; the AABB pre-filter used by other
    //     kinds would reject those even when the click is right
    //     on the stroke.
    //   - negative-size bounds (right-to-left / bottom-to-top)
    //     collapse to `Rect::ZERO` in `Node::aggregate_bounds`,
    //     so the Line path reads `node.bounds` directly. The
    //     distance-to-segment helper is sign-independent.
    if matches!(node.kind, NodeKind::Line) {
        let raw = node.bounds;
        // A true point (both dims 0) is not a hittable segment.
        if raw.size.x.abs() < f32::EPSILON && raw.size.y.abs() < f32::EPSILON {
            return false;
        }
        let from = raw.origin;
        let to = crate::Point2D::new(raw.origin.x + raw.size.x, raw.origin.y + raw.size.y);
        let stroke_half = node.stroke.map(|s| s.width / 2.0).unwrap_or(1.0);
        // Slack target: 4 screen px regardless of zoom. The point
        // is already in doc space, so scale by `1/zoom` to get
        // the doc-space equivalent (low zoom → bigger doc-space
        // slack; high zoom → tighter).
        let screen_slack = 4.0 / zoom.max(0.0001);
        return distance_point_to_segment(local, from, to) <= stroke_half + screen_slack;
    }
    // Non-line kinds need real positive area on both axes — the
    // ellipse / triangle / rect paint paths all expect a positive
    // rect.
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return false;
    }
    // Axis-aligned bounding check is a cheap reject for every
    // kind — the tighter checks below only need to refine the
    // hit inside the bounds.
    let in_bounds = local.x >= bounds.origin.x
        && local.x <= bounds.origin.x + bounds.size.x
        && local.y >= bounds.origin.y
        && local.y <= bounds.origin.y + bounds.size.y;
    if !in_bounds {
        return false;
    }
    let cx = bounds.origin.x + bounds.size.x / 2.0;
    let cy = bounds.origin.y + bounds.size.y / 2.0;
    let rx = (bounds.size.x / 2.0).max(0.0001);
    let ry = (bounds.size.y / 2.0).max(0.0001);
    match node.kind {
        NodeKind::Ellipse => {
            let dx = (local.x - cx) / rx;
            let dy = (local.y - cy) / ry;
            dx * dx + dy * dy <= 1.0
        }
        NodeKind::Polygon => {
            // Triangle vertices: top-center, bottom-left, bottom-
            // right. Mirrors `canvas_viewport.rs::paint_node`'s
            // `fill_polygon` call so hit-test follows paint.
            let top = crate::Point2D::new(cx, bounds.origin.y);
            let bl = crate::Point2D::new(bounds.origin.x, bounds.origin.y + bounds.size.y);
            let br = crate::Point2D::new(
                bounds.origin.x + bounds.size.x,
                bounds.origin.y + bounds.size.y,
            );
            point_in_triangle(local, top, bl, br)
        }
        // Frame, Group, Rect, Text, Other — bounds-only hit.
        _ => true,
    }
}

fn point_in_triangle(
    p: crate::Point2D,
    a: crate::Point2D,
    b: crate::Point2D,
    c: crate::Point2D,
) -> bool {
    // Sign-of-cross-product method. Inside iff all three edge
    // signs match (or any is zero — point on the edge).
    let s1 = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let s2 = (c.x - b.x) * (p.y - b.y) - (c.y - b.y) * (p.x - b.x);
    let s3 = (a.x - c.x) * (p.y - c.y) - (a.y - c.y) * (p.x - c.x);
    let has_neg = s1 < 0.0 || s2 < 0.0 || s3 < 0.0;
    let has_pos = s1 > 0.0 || s2 > 0.0 || s3 > 0.0;
    !(has_neg && has_pos)
}

fn distance_point_to_segment(p: crate::Point2D, a: crate::Point2D, b: crate::Point2D) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < f32::EPSILON {
        let pdx = p.x - a.x;
        let pdy = p.y - a.y;
        return (pdx * pdx + pdy * pdy).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    let cx = a.x + t * dx;
    let cy = a.y + t * dy;
    let pdx = p.x - cx;
    let pdy = p.y - cy;
    (pdx * pdx + pdy * pdy).sqrt()
}

/// Recursive helper for `Document::translate_selected`. Returns
/// `true` once `target` has been translated. Both the matched
/// node AND every descendant are moved — child bounds are stored
/// in document space (not relative to the parent), so dragging a
/// bounded Frame must shift the children too or they'd detach.
fn translate_walk(node: &mut Node, target: NodeId, dx: f32, dy: f32) -> bool {
    if node.id == target {
        translate_subtree(node, dx, dy);
        return true;
    }
    for child in &mut node.children {
        if translate_walk(child, target, dx, dy) {
            return true;
        }
    }
    false
}

/// Return true if `target` is a descendant (not equal to itself)
/// of any node in `set` within `children`. Used by
/// `translate_selected` to dedupe shifts when both an ancestor
/// and one of its descendants are in the selection set.
fn is_ancestor_in_set(children: &[Node], target: NodeId, set: &[NodeId]) -> bool {
    for child in children {
        if descendant_contains(child, target) && child.id != target && set.contains(&child.id) {
            return true;
        }
        if is_ancestor_in_set(&child.children, target, set) {
            return true;
        }
    }
    false
}

/// True iff `target` equals `node` or appears in its subtree.
fn descendant_contains(node: &Node, target: NodeId) -> bool {
    if node.id == target {
        return true;
    }
    node.children.iter().any(|c| descendant_contains(c, target))
}

fn translate_subtree(node: &mut Node, dx: f32, dy: f32) {
    if node.bounds.size.x > 0.0 || node.bounds.size.y > 0.0 {
        node.bounds.origin.x += dx;
        node.bounds.origin.y += dy;
    }
    for child in &mut node.children {
        translate_subtree(child, dx, dy);
    }
}

/// Recursive helper for `Document::commit_property_edit`. Returns
/// `true` once the edit lands on the matching node.
fn commit_property_walk(node: &mut Node, sel: NodeId, focus: PropertyFocus, value: f32) -> bool {
    if node.id == sel {
        match focus {
            PropertyFocus::PositionX => node.bounds.origin.x = value,
            PropertyFocus::PositionY => node.bounds.origin.y = value,
            PropertyFocus::SizeW => node.bounds.size.x = value.max(0.0),
            PropertyFocus::SizeH => node.bounds.size.y = value.max(0.0),
            PropertyFocus::Rotation => {
                // Property panel ships degrees; node stores radians.
                node.rotation = value.to_radians();
            }
            PropertyFocus::StrokeWidth => {
                let color = node.stroke.map(|s| s.color).unwrap_or(crate::Color::BLACK);
                node.stroke = Some(Stroke {
                    color,
                    width: value.max(0.0),
                });
            }
            // Hex inputs go through `Document::set_selected_color`
            // (Color isn't a single f32). Opacity + corner-radius
            // are visual-only until the node schema grows the
            // field — accept the edit but don't persist.
            PropertyFocus::PositionR
            | PropertyFocus::Opacity
            | PropertyFocus::FillHex
            | PropertyFocus::StrokeHex => {}
        }
        return true;
    }
    for child in &mut node.children {
        if commit_property_walk(child, sel, focus, value) {
            return true;
        }
    }
    false
}

/// Direction for `Document::reorder_selected` — picks which
/// neighbour the selected node swaps with in its parent's
/// children vec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReorderDirection {
    /// Towards the front of the paint order (higher index, drawn
    /// on top). Bound to `]`.
    Up,
    /// Towards the back of the paint order (lower index, drawn
    /// underneath). Bound to `[`.
    Down,
}

/// Recursive helper for `Document::delete_selected`. Returns true
/// when the target was found + removed from `children` or any of
/// its descendants' children.
fn remove_from_children(children: &mut Vec<Node>, target: NodeId) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        children.remove(idx);
        return true;
    }
    for child in children.iter_mut() {
        if remove_from_children(&mut child.children, target) {
            return true;
        }
    }
    false
}

/// Deep-clone `node`, allocating a fresh id from `next_id` for
/// every descendant. Bounds and other fields copy verbatim;
/// callers shift `bounds.origin` by an offset if they want the
/// clone to land visually beside the original.
/// True iff `node` and every descendant are editable (none have
/// `hidden` or `locked` set). Backs `Document::is_subtree_editable`.
fn subtree_all_editable(node: &Node) -> bool {
    if node.hidden || node.locked {
        return false;
    }
    node.children.iter().all(subtree_all_editable)
}

fn toggle_hidden_walk(children: &mut Vec<Node>, target: NodeId) -> bool {
    for child in children.iter_mut() {
        if child.id == target {
            child.hidden = !child.hidden;
            return true;
        }
        if toggle_hidden_walk(&mut child.children, target) {
            return true;
        }
    }
    false
}

fn set_fill_type_walk(node: &mut Node, target: NodeId, fill_type: FillType) -> bool {
    if node.id == target {
        node.fill_type = fill_type;
        return true;
    }
    for child in &mut node.children {
        if set_fill_type_walk(child, target, fill_type) {
            return true;
        }
    }
    false
}

fn toggle_collapsed_walk(children: &mut Vec<Node>, target: NodeId) -> bool {
    for child in children.iter_mut() {
        if child.id == target {
            child.collapsed = !child.collapsed;
            return true;
        }
        if toggle_collapsed_walk(&mut child.children, target) {
            return true;
        }
    }
    false
}

fn toggle_locked_walk(children: &mut Vec<Node>, target: NodeId) -> bool {
    for child in children.iter_mut() {
        if child.id == target {
            child.locked = !child.locked;
            return true;
        }
        if toggle_locked_walk(&mut child.children, target) {
            return true;
        }
    }
    false
}

fn deep_clone_with_new_ids(node: &Node, next_id: &mut u64) -> Node {
    let id = NodeId::new(*next_id);
    *next_id += 1;
    let children: Vec<Node> = node
        .children
        .iter()
        .map(|c| deep_clone_with_new_ids(c, next_id))
        .collect();
    Node {
        id,
        kind: node.kind.clone(),
        name: node.name.clone(),
        bounds: node.bounds,
        rotation: node.rotation,
        fill: node.fill,
        stroke: node.stroke,
        text: node.text.clone(),
        hidden: node.hidden,
        locked: node.locked,
        collapsed: node.collapsed,
        fill_type: node.fill_type,
        children,
    }
}

/// Recursive helper for `Document::duplicate_selected`. Returns
/// the new node's id when the target was found and a sibling
/// clone was inserted.
///
/// Before allocating any id, verifies the allocator has enough
/// headroom for the entire subtree (one id per node + one
/// trailing increment past the last mint). Returning None when
/// the subtree won't fit guarantees the document isn't left in a
/// half-cloned state on overflow.
fn duplicate_in_children(
    children: &mut Vec<Node>,
    target: NodeId,
    next_id: &mut u64,
    offset: f32,
) -> Option<NodeId> {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        let size = subtree_size(&children[idx]);
        // The clone walk mints `size` ids and runs one final
        // `*next_id += 1` past the last mint. Both must stay
        // representable, so the required headroom is `size`.
        next_id.checked_add(size)?;
        let mut clone = deep_clone_with_new_ids(&children[idx], next_id);
        // Offset the clone so the user sees it next to the
        // original instead of pixel-perfectly stacked on top.
        // Matches TS `cloneNodesWithNewIds({ offset: 10 })`.
        shift_subtree(&mut clone, offset, offset);
        let new_id = clone.id;
        children.insert(idx + 1, clone);
        return Some(new_id);
    }
    for child in children.iter_mut() {
        if let Some(new_id) = duplicate_in_children(&mut child.children, target, next_id, offset) {
            return Some(new_id);
        }
    }
    None
}

/// Node count in `node`'s subtree (including `node`). Used as
/// `duplicate_in_children`'s headroom check so we reject up-front
/// when the allocator can't fit the entire clone without overflow.
fn subtree_size(node: &Node) -> u64 {
    let mut n = 1u64;
    for child in &node.children {
        n = n.saturating_add(subtree_size(child));
    }
    n
}

fn shift_subtree(node: &mut Node, dx: f32, dy: f32) {
    if node.bounds.size.x > 0.0 || node.bounds.size.y > 0.0 {
        node.bounds.origin.x += dx;
        node.bounds.origin.y += dy;
    }
    for child in &mut node.children {
        shift_subtree(child, dx, dy);
    }
}

/// Recursive helper for `Document::reorder_selected`.
fn reorder_in_children(
    children: &mut Vec<Node>,
    target: NodeId,
    direction: ReorderDirection,
) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        match direction {
            ReorderDirection::Up if idx + 1 < children.len() => {
                children.swap(idx, idx + 1);
                return true;
            }
            ReorderDirection::Down if idx > 0 => {
                children.swap(idx, idx - 1);
                return true;
            }
            _ => return false,
        }
    }
    for child in children.iter_mut() {
        if reorder_in_children(&mut child.children, target, direction) {
            return true;
        }
    }
    false
}

/// Recursive helper for `Document::max_node_id`.
fn max_id_walk(node: &Node) -> u64 {
    let mut max = node.id.raw();
    for child in &node.children {
        max = max.max(max_id_walk(child));
    }
    max
}

fn find_duplicate_walk(
    node: &Node,
    seen: &mut std::collections::HashSet<NodeId>,
) -> Option<NodeId> {
    if !seen.insert(node.id) {
        return Some(node.id);
    }
    for child in &node.children {
        if let Some(dup) = find_duplicate_walk(child, seen) {
            return Some(dup);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_selected_removes_top_level_node_and_clears_selection() {
        let mut doc = Document::sample();
        let target = NodeId::new(10);
        doc.set_single_selection(target);
        assert!(doc.active_page().unwrap().find(target).is_some());
        assert!(doc.delete_selected());
        assert_eq!(doc.selected, NodeId::NONE);
        assert!(doc.active_page().unwrap().find(target).is_none());
    }

    #[test]
    fn delete_selected_removes_nested_node() {
        let mut doc = Document::sample();
        let nested = NodeId::new(13); // Button background — descendant of Frame 10
        doc.set_single_selection(nested);
        assert!(doc.active_page().unwrap().find(nested).is_some());
        assert!(doc.delete_selected());
        assert!(doc.active_page().unwrap().find(nested).is_none());
        // Parent must remain.
        assert!(doc.active_page().unwrap().find(NodeId::new(10)).is_some());
    }

    #[test]
    fn delete_selected_returns_false_when_unselected() {
        let mut doc = Document::sample();
        doc.clear_selection();
        assert!(!doc.delete_selected());
    }

    #[test]
    fn duplicate_selected_clones_subtree_with_fresh_ids_and_selects_it() {
        let mut doc = Document::sample();
        doc.set_single_selection(NodeId::new(10)); // bounded Frame with children
        let mut next_id = 1_000u64;
        let before_descendant = doc.active_page().unwrap().find(NodeId::new(13)).cloned();
        assert!(before_descendant.is_some());

        let clone_id = doc
            .duplicate_selected(&mut next_id, 10.0)
            .expect("duplicate should return new id");
        assert!(clone_id.is_real());
        assert_eq!(doc.selected, clone_id);
        // Original still present.
        assert!(doc.active_page().unwrap().find(NodeId::new(10)).is_some());
        // Clone has fresh id (different from any original).
        assert!(doc.active_page().unwrap().find(clone_id).is_some());
        // Clone origin shifted by offset.
        let original = doc.active_page().unwrap().find(NodeId::new(10)).unwrap();
        let clone = doc.active_page().unwrap().find(clone_id).unwrap();
        assert!((clone.bounds.origin.x - original.bounds.origin.x - 10.0).abs() < 1e-3);
        assert!((clone.bounds.origin.y - original.bounds.origin.y - 10.0).abs() < 1e-3);
        // Clone preserves descendant count.
        assert_eq!(clone.children.len(), original.children.len());
        // No id collision in the page (would be caught by validate).
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn reorder_selected_up_moves_to_higher_index() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![
            Node::leaf(1, NodeKind::Rect, "A"),
            Node::leaf(2, NodeKind::Rect, "B"),
            Node::leaf(3, NodeKind::Rect, "C"),
        ];
        doc.set_single_selection(NodeId::new(2)); // middle
        assert!(doc.reorder_selected(ReorderDirection::Up));
        let ids: Vec<u64> = doc.pages[root_idx]
            .children
            .iter()
            .map(|n| n.id.raw())
            .collect();
        assert_eq!(ids, vec![1, 3, 2]);
    }

    #[test]
    fn reorder_selected_down_moves_to_lower_index() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![
            Node::leaf(1, NodeKind::Rect, "A"),
            Node::leaf(2, NodeKind::Rect, "B"),
            Node::leaf(3, NodeKind::Rect, "C"),
        ];
        doc.set_single_selection(NodeId::new(2));
        assert!(doc.reorder_selected(ReorderDirection::Down));
        let ids: Vec<u64> = doc.pages[root_idx]
            .children
            .iter()
            .map(|n| n.id.raw())
            .collect();
        assert_eq!(ids, vec![2, 1, 3]);
    }

    #[test]
    fn reorder_selected_at_edges_is_noop() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![
            Node::leaf(1, NodeKind::Rect, "A"),
            Node::leaf(2, NodeKind::Rect, "B"),
        ];
        doc.set_single_selection(NodeId::new(1));
        assert!(!doc.reorder_selected(ReorderDirection::Down));
        doc.set_single_selection(NodeId::new(2));
        assert!(!doc.reorder_selected(ReorderDirection::Up));
    }

    #[test]
    fn duplicate_selected_lifts_allocator_past_existing_max_id() {
        // Codex CONCERN-1 regression: external docs may carry ids
        // larger than the host's `next_node_id` counter. The
        // mutator must lift the allocator past the document's max
        // before minting a new id so no collision is possible.
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![
            Node::leaf(5000, NodeKind::Rect, "A"),
            Node::leaf(5001, NodeKind::Rect, "B"),
        ];
        doc.set_single_selection(NodeId::new(5000));
        let mut next_id = 100u64;
        let clone_id = doc
            .duplicate_selected(&mut next_id, 0.0)
            .expect("duplicate should succeed");
        assert!(
            clone_id.raw() > 5001,
            "clone id {} must exceed the document max id",
            clone_id.raw()
        );
        // Allocator is updated so subsequent calls also stay
        // collision-free.
        assert!(next_id > 5001);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn duplicate_selected_returns_none_when_max_id_overflows() {
        // Codex CONCERN-3 follow-up: if a document somehow carries
        // `NodeId(u64::MAX)`, `max_node_id + 1` would overflow.
        // Return None instead of saturating and minting a
        // colliding id.
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![
            Node::leaf(u64::MAX, NodeKind::Rect, "boundary"),
            Node::leaf(7, NodeKind::Rect, "small"),
        ];
        doc.set_single_selection(NodeId::new(7));
        let mut next_id = 100u64;
        assert!(doc.duplicate_selected(&mut next_id, 0.0).is_none());
        // No id mutation occurred — count is unchanged.
        assert_eq!(doc.pages[root_idx].children.len(), 2);
    }

    #[test]
    fn duplicate_selected_rejects_when_subtree_exhausts_id_space() {
        // Codex CONCERN-5: even when `max_node_id + 1` fits, a
        // multi-node subtree may still walk past `u64::MAX`
        // partway through, leaving the document half-cloned. The
        // headroom precheck must reject in that case so the page
        // is untouched.
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        // 3-node subtree: parent + 2 children. Small ids so
        // `max_node_id + 1` doesn't trip the earlier guard.
        let parent = Node::with_children(
            7,
            NodeKind::Group,
            "group",
            vec![
                Node::leaf(8, NodeKind::Rect, "a"),
                Node::leaf(9, NodeKind::Rect, "b"),
            ],
        );
        doc.pages[root_idx].children = vec![parent];
        doc.set_single_selection(NodeId::new(7));

        // Allocator close enough to `u64::MAX` that minting all 3
        // ids would overflow `*next_id += 1` past the last mint.
        let mut next_id = u64::MAX - 2;
        assert!(doc.duplicate_selected(&mut next_id, 0.0).is_none());
        // Page untouched.
        assert_eq!(doc.pages[root_idx].children.len(), 1);
        // Allocator unchanged — no half-mint state.
        assert_eq!(next_id, u64::MAX - 2);
    }

    fn build_shape_doc() -> Document {
        // Single page with one of each shape kind at known
        // bounds. Each shape occupies a 100×100 rect.
        let mut doc = Document::empty();
        let page_idx = doc.active_page_index;
        doc.pages[page_idx].children = vec![
            Node::leaf(101, NodeKind::Rect, "R")
                .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0)),
            Node::leaf(102, NodeKind::Ellipse, "E")
                .with_bounds(crate::Rect::xywh(200.0, 0.0, 100.0, 100.0)),
            Node::leaf(103, NodeKind::Polygon, "P")
                .with_bounds(crate::Rect::xywh(400.0, 0.0, 100.0, 100.0)),
            Node::leaf(104, NodeKind::Line, "L")
                .with_bounds(crate::Rect::xywh(600.0, 0.0, 100.0, 100.0))
                .with_stroke(crate::Color::BLACK, 2.0),
        ];
        doc
    }

    #[test]
    fn hit_test_rect_uses_bounds() {
        let doc = build_shape_doc();
        // Rect: anywhere inside the bounds is a hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)),
            Some(NodeId::new(101))
        );
        // Corner edge.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(0.5, 99.5)),
            Some(NodeId::new(101))
        );
    }

    #[test]
    fn hit_test_ellipse_uses_ellipse_geometry() {
        let doc = build_shape_doc();
        // Centre of ellipse bounds (200..300, 0..100) is (250, 50).
        // Inside the inscribed oval → hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(250.0, 50.0)),
            Some(NodeId::new(102))
        );
        // Corner of bounds rect → INSIDE rect-bounds but OUTSIDE
        // the ellipse → no hit (was a hit under the old rect-only
        // path; codex audit fix).
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(201.0, 1.0)), None);
        // Point on the perimeter (left edge of oval, at vertical
        // midpoint) → hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(200.5, 50.0)),
            Some(NodeId::new(102))
        );
    }

    #[test]
    fn hit_test_polygon_uses_triangle_geometry() {
        let doc = build_shape_doc();
        // Triangle vertices: top-center (450, 0), bottom-left
        // (400, 100), bottom-right (500, 100).
        // Centroid (~450, 66.7) → hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(450.0, 70.0)),
            Some(NodeId::new(103))
        );
        // Top-left corner of bounds (401, 1) → outside the
        // triangle (above the left edge) → no hit.
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(401.0, 1.0)), None);
        // Top-right corner of bounds → outside the triangle.
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(499.0, 1.0)), None);
        // Bottom-center, on the base → hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(450.0, 99.0)),
            Some(NodeId::new(103))
        );
    }

    #[test]
    fn hit_test_line_uses_stroke_proximity() {
        let doc = build_shape_doc();
        // Line stroke = 2 px → threshold ≈ 1 + 4 = 5 doc px.
        // Diagonal from (600, 0) to (700, 100).
        // Midpoint (650, 50) → exactly on the line → hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(650.0, 50.0)),
            Some(NodeId::new(104))
        );
        // Near the diagonal (within slack) → hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(648.0, 51.0)),
            Some(NodeId::new(104))
        );
        // Far from the diagonal (top-right corner of bounds) →
        // no hit (was a hit under rect-only path).
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(699.0, 1.0)), None);
        // Bottom-left corner of bounds → far from the diagonal,
        // no hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(601.0, 99.0)),
            None
        );
    }

    #[test]
    fn hit_test_horizontal_line_with_zero_height_bounds_is_grabbable() {
        // Codex CONCERN: zero-y bounds (horizontal line) was
        // rejected by the AABB pre-filter. Line kind now has its
        // own path that runs distance-to-segment unconditionally.
        let mut doc = Document::empty();
        let page_idx = doc.active_page_index;
        doc.pages[page_idx].children = vec![Node::leaf(50, NodeKind::Line, "horiz")
            .with_bounds(crate::Rect::xywh(0.0, 50.0, 100.0, 0.0))
            .with_stroke(crate::Color::BLACK, 2.0)];
        // Click directly on the segment.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)),
            Some(NodeId::new(50))
        );
        // Click 3 px above — within stroke (1) + 4 screen px slack
        // at zoom 1 = 5 doc px threshold.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(50.0, 47.0)),
            Some(NodeId::new(50))
        );
        // Click well above — no hit.
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, 40.0)), None);
    }

    #[test]
    fn hit_test_vertical_line_with_zero_width_bounds_is_grabbable() {
        let mut doc = Document::empty();
        let page_idx = doc.active_page_index;
        doc.pages[page_idx].children = vec![Node::leaf(51, NodeKind::Line, "vert")
            .with_bounds(crate::Rect::xywh(50.0, 0.0, 0.0, 100.0))
            .with_stroke(crate::Color::BLACK, 2.0)];
        // Click directly on the segment.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)),
            Some(NodeId::new(51))
        );
        // Click 3 px to the right — within threshold.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(53.0, 50.0)),
            Some(NodeId::new(51))
        );
        // Click well to the right — no hit.
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(60.0, 50.0)), None);
    }

    #[test]
    fn hit_test_line_slack_scales_with_zoom() {
        // At zoom = 0.5 (zoomed out), 4 screen px = 8 doc px slack
        // so a click 7 doc px from the segment is still a hit.
        // At zoom = 2.0 (zoomed in), 4 screen px = 2 doc px, so
        // the same 7-px-offset click misses.
        let mut doc = Document::empty();
        let page_idx = doc.active_page_index;
        doc.pages[page_idx].children = vec![Node::leaf(60, NodeKind::Line, "diag")
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 0.0))
            .with_stroke(crate::Color::BLACK, 0.0)];

        // Threshold = stroke_half (0) + 4/zoom. Click 7 doc px
        // above the segment at zoom 0.5 → 4/0.5 = 8 doc px slack
        // → hit. Same click at zoom 2 → 4/2 = 2 doc px → miss.
        doc.viewport.zoom = 0.5;
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(50.0, -7.0)),
            Some(NodeId::new(60))
        );
        doc.viewport.zoom = 2.0;
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, -7.0)), None);
    }

    #[test]
    fn hit_test_line_with_negative_size_bounds_is_grabbable() {
        // Codex CONCERN follow-up: `aggregate_bounds` collapses
        // negative-size leaves to `Rect::ZERO`, so a Line drawn
        // right-to-left or bottom-to-top would be unhittable if
        // the Line path read aggregate bounds. Reading `node.bounds`
        // directly preserves the sign and the distance helper is
        // sign-independent.
        let mut doc = Document::empty();
        let page_idx = doc.active_page_index;
        doc.pages[page_idx].children = vec![Node::leaf(70, NodeKind::Line, "rev")
            // Right-to-left: origin (100, 0), size (-100, 50)
            // → segment from (100, 0) to (0, 50).
            .with_bounds(crate::Rect::xywh(100.0, 0.0, -100.0, 50.0))
            .with_stroke(crate::Color::BLACK, 2.0)];
        // Midpoint of the segment (50, 25) → hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(50.0, 25.0)),
            Some(NodeId::new(70))
        );
        // Endpoint (0, 50) → hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(0.0, 50.0)),
            Some(NodeId::new(70))
        );
        // Far above the segment → no hit.
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, 0.0)), None);
    }

    #[test]
    fn hit_test_rotated_negative_size_line_uses_segment_midpoint_pivot() {
        // Codex CONCERN follow-up: rotation pivot was computed
        // from aggregate_bounds which collapses negative-size
        // Lines to ZERO — meaning a rotated negative Line rotated
        // around (0, 0) instead of its midpoint. The kind-aware
        // `rotation_pivot` helper fixes it.
        use std::f32::consts::PI;
        let mut doc = Document::empty();
        let page_idx = doc.active_page_index;
        // Bottom-to-top line: origin (50, 100), size (0, -100) →
        // collapses to aggregate=ZERO. Vertical segment from
        // (50, 100) up to (50, 0). Midpoint (50, 50). Rotating
        // 90° around (50, 50) gives a horizontal segment from
        // (100, 50) to (0, 50).
        doc.pages[page_idx].children = vec![Node {
            id: NodeId::new(80),
            kind: NodeKind::Line,
            name: "rev_vert".into(),
            bounds: crate::Rect::xywh(50.0, 100.0, 0.0, -100.0),
            fill: None,
            stroke: Some(Stroke {
                color: crate::Color::BLACK,
                width: 2.0,
            }),
            text: None,
            rotation: PI / 2.0,
            hidden: false,
            locked: false,
            collapsed: false,
            fill_type: FillType::Solid,
            children: Vec::new(),
        }];
        // Click at (50, 50) — midpoint (invariant under rotation).
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)),
            Some(NodeId::new(80))
        );
        // After 90° rotation, the segment is horizontal from
        // (100, 50) to (0, 50). Click at (25, 50) → hit.
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(25.0, 50.0)),
            Some(NodeId::new(80))
        );
        // Click at (50, 90) — was on the un-rotated segment, but
        // after 90° rotation it would land off the line → no hit.
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, 90.0)), None);
    }

    #[test]
    fn hit_test_zero_size_node_is_never_hit() {
        let mut doc = Document::empty();
        let page_idx = doc.active_page_index;
        doc.pages[page_idx].children =
            vec![Node::leaf(7, NodeKind::Rect, "z")
                .with_bounds(crate::Rect::xywh(0.0, 0.0, 0.0, 0.0))];
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(0.0, 0.0)), None);
    }

    #[test]
    fn deselect_all_clears_selection() {
        let mut doc = Document::sample();
        doc.set_single_selection(NodeId::new(10));
        doc.deselect_all();
        assert_eq!(doc.selected, NodeId::NONE);
        assert!(doc.selected_set.is_empty());
    }

    #[test]
    fn set_single_selection_replaces_set_and_anchor() {
        let mut doc = Document::sample();
        doc.selected_set = vec![NodeId::new(11), NodeId::new(13)];
        doc.selected = NodeId::new(13);
        doc.set_single_selection(NodeId::new(14));
        assert_eq!(doc.selected_set, vec![NodeId::new(14)]);
        assert_eq!(doc.selected, NodeId::new(14));
        assert_eq!(doc.selection_count(), 1);
        doc.set_single_selection(NodeId::NONE);
        assert!(doc.selected_set.is_empty());
        assert_eq!(doc.selected, NodeId::NONE);
    }

    #[test]
    fn toggle_selection_adds_then_removes_and_picks_new_anchor() {
        let mut doc = Document::sample();
        doc.clear_selection();
        doc.toggle_selection(NodeId::new(11));
        assert_eq!(doc.selected_set, vec![NodeId::new(11)]);
        assert_eq!(doc.selected, NodeId::new(11));
        doc.toggle_selection(NodeId::new(13));
        assert_eq!(doc.selected_set, vec![NodeId::new(11), NodeId::new(13)]);
        assert_eq!(doc.selected, NodeId::new(13));
        doc.toggle_selection(NodeId::new(13));
        assert_eq!(doc.selected_set, vec![NodeId::new(11)]);
        assert_eq!(doc.selected, NodeId::new(11));
        doc.toggle_selection(NodeId::new(11));
        assert!(doc.selected_set.is_empty());
        assert_eq!(doc.selected, NodeId::NONE);
    }

    #[test]
    fn select_all_top_level_populates_set_with_active_page_children() {
        let mut doc = Document::sample();
        doc.clear_selection();
        assert!(doc.select_all_top_level());
        assert_eq!(doc.selected_set, vec![NodeId::new(10)]);
        assert_eq!(doc.selected, NodeId::new(10));
        let mut empty = Document::empty();
        assert!(!empty.select_all_top_level());
        assert!(empty.selected_set.is_empty());
    }

    #[test]
    fn delete_selected_removes_every_node_in_the_set() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![
            Node::leaf(1, NodeKind::Rect, "a"),
            Node::leaf(2, NodeKind::Rect, "b"),
            Node::leaf(3, NodeKind::Rect, "c"),
        ];
        doc.selected_set = vec![NodeId::new(1), NodeId::new(3)];
        doc.selected = NodeId::new(3);
        assert!(doc.delete_selected());
        let ids: Vec<u64> = doc.pages[root_idx]
            .children
            .iter()
            .map(|n| n.id.raw())
            .collect();
        assert_eq!(ids, vec![2]);
        assert!(doc.selected_set.is_empty());
        assert_eq!(doc.selected, NodeId::NONE);
    }

    #[test]
    fn duplicate_selected_clones_every_node_in_the_set() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        // IDs 2 + 3 — page id 1 already occupies the namespace
        // so child id 1 would collide on `validate`.
        doc.pages[root_idx].children = vec![
            Node::leaf(2, NodeKind::Rect, "a"),
            Node::leaf(3, NodeKind::Rect, "b"),
        ];
        doc.selected_set = vec![NodeId::new(2), NodeId::new(3)];
        doc.selected = NodeId::new(3);
        let mut next_id = 100u64;
        let anchor = doc
            .duplicate_selected(&mut next_id, 0.0)
            .expect("duplicate set");
        assert_eq!(doc.pages[root_idx].children.len(), 4);
        assert_eq!(doc.selected_set.len(), 2);
        assert_eq!(doc.selected, anchor);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn toggle_node_hidden_flips_flag_and_skips_canvas_hit_test() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![Node::leaf(7, NodeKind::Rect, "a")
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0))];
        let p = crate::Point2D::new(50.0, 50.0);
        assert_eq!(doc.node_at_doc_point(p), Some(NodeId::new(7)));
        assert!(doc.toggle_node_hidden(NodeId::new(7)));
        // Hidden → canvas hit-test ignores it.
        assert_eq!(doc.node_at_doc_point(p), None);
        // Toggle again to unhide.
        assert!(doc.toggle_node_hidden(NodeId::new(7)));
        assert_eq!(doc.node_at_doc_point(p), Some(NodeId::new(7)));
    }

    #[test]
    fn set_selected_fill_type_writes_per_node_and_does_not_leak() {
        // Codex full-audit CONCERN regression: `fill_type` used
        // to live on `Document.ui`, so picking a different
        // selection inherited the prior picker choice. Now it's
        // a per-node field — each node remembers its own type.
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![
            Node::leaf(2, NodeKind::Rect, "a"),
            Node::leaf(3, NodeKind::Rect, "b"),
        ];
        // Both nodes start as Solid (Node::leaf default).
        assert_eq!(
            doc.active_page()
                .unwrap()
                .find(NodeId::new(2))
                .unwrap()
                .fill_type,
            FillType::Solid
        );
        // Select "a", set to LinearGradient.
        doc.set_single_selection(NodeId::new(2));
        assert!(doc.set_selected_fill_type(FillType::LinearGradient));
        assert_eq!(
            doc.active_page()
                .unwrap()
                .find(NodeId::new(2))
                .unwrap()
                .fill_type,
            FillType::LinearGradient
        );
        // Selecting "b" must NOT inherit "a"'s LinearGradient.
        doc.set_single_selection(NodeId::new(3));
        assert_eq!(
            doc.active_page()
                .unwrap()
                .find(NodeId::new(3))
                .unwrap()
                .fill_type,
            FillType::Solid
        );
        // Going back to "a" still shows LinearGradient.
        assert_eq!(
            doc.active_page()
                .unwrap()
                .find(NodeId::new(2))
                .unwrap()
                .fill_type,
            FillType::LinearGradient
        );
    }

    #[test]
    fn set_selected_fill_type_respects_locked_and_hidden() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        let mut locked = Node::leaf(4, NodeKind::Rect, "lock");
        locked.locked = true;
        let mut hidden = Node::leaf(5, NodeKind::Rect, "hide");
        hidden.hidden = true;
        doc.pages[root_idx].children = vec![locked, hidden];
        doc.set_single_selection(NodeId::new(4));
        assert!(!doc.set_selected_fill_type(FillType::Image));
        assert_eq!(
            doc.active_page()
                .unwrap()
                .find(NodeId::new(4))
                .unwrap()
                .fill_type,
            FillType::Solid
        );
        doc.set_single_selection(NodeId::new(5));
        assert!(!doc.set_selected_fill_type(FillType::Image));
        assert_eq!(
            doc.active_page()
                .unwrap()
                .find(NodeId::new(5))
                .unwrap()
                .fill_type,
            FillType::Solid
        );
    }

    #[test]
    fn locked_selection_blocks_translate_set_bounds_rotation_color_and_property_edit() {
        // Codex stop-hook BLOCK: locked nodes selected via the
        // layer panel must not be mutated by translate / resize /
        // rotate / property edits / color setters.
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        let mut locked = Node::leaf(40, NodeKind::Rect, "locked")
            .with_bounds(crate::Rect::xywh(10.0, 10.0, 50.0, 50.0))
            .with_fill(crate::Color::WHITE);
        locked.locked = true;
        doc.pages[root_idx].children = vec![locked];
        doc.set_single_selection(NodeId::new(40));

        let before = doc.selected_node().unwrap().bounds;
        doc.translate_selected(7.0, 3.0);
        assert_eq!(doc.selected_node().unwrap().bounds, before);
        doc.set_selected_bounds(crate::Rect::xywh(0.0, 0.0, 1.0, 1.0));
        assert_eq!(doc.selected_node().unwrap().bounds, before);
        doc.set_selected_rotation(1.5);
        assert!(doc.selected_node().unwrap().rotation.abs() < f32::EPSILON);
        assert!(!doc.set_selected_color(true, crate::Color::BLACK));
        assert_eq!(doc.selected_node().unwrap().fill, Some(crate::Color::WHITE));
        assert!(!doc.commit_property_edit(PropertyFocus::PositionX, 999.0));
        assert_eq!(
            doc.selected_node().unwrap().bounds.origin.x,
            before.origin.x
        );
    }

    #[test]
    fn hidden_selection_blocks_translate_and_property_edit() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        let mut hidden = Node::leaf(41, NodeKind::Rect, "hidden")
            .with_bounds(crate::Rect::xywh(10.0, 10.0, 50.0, 50.0));
        hidden.hidden = true;
        doc.pages[root_idx].children = vec![hidden];
        doc.set_single_selection(NodeId::new(41));
        let before = doc.selected_node().unwrap().bounds;
        doc.translate_selected(7.0, 3.0);
        assert_eq!(doc.selected_node().unwrap().bounds, before);
        assert!(!doc.commit_property_edit(PropertyFocus::PositionX, 999.0));
        assert_eq!(
            doc.selected_node().unwrap().bounds.origin.x,
            before.origin.x
        );
    }

    #[test]
    fn delete_selected_protects_ancestor_of_locked_descendant() {
        // Codex stop-hook BLOCK: selecting an editable Frame +
        // its locked child must not let `delete_selected` wipe
        // the child by collapsing the parent.
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        let mut child = Node::leaf(61, NodeKind::Rect, "child");
        child.locked = true;
        let frame = Node::with_children(60, NodeKind::Frame, "frame", vec![child])
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
        doc.pages[root_idx].children = vec![frame];
        // Select the Frame only.
        doc.set_single_selection(NodeId::new(60));
        // delete_selected must refuse — the Frame's subtree
        // contains a locked child. is_subtree_editable returns
        // false → nothing deletable.
        assert!(!doc.delete_selected());
        // Frame still in document.
        assert!(doc.active_page().unwrap().find(NodeId::new(60)).is_some());
        assert!(doc.active_page().unwrap().find(NodeId::new(61)).is_some());

        // Same scenario, hidden descendant.
        let mut doc2 = Document::empty();
        let root_idx2 = doc2.active_page_index;
        let mut child2 = Node::leaf(71, NodeKind::Rect, "child");
        child2.hidden = true;
        let frame2 = Node::with_children(70, NodeKind::Frame, "frame", vec![child2])
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
        doc2.pages[root_idx2].children = vec![frame2];
        doc2.set_single_selection(NodeId::new(70));
        assert!(!doc2.delete_selected());
        assert!(doc2.active_page().unwrap().find(NodeId::new(70)).is_some());
    }

    #[test]
    fn delete_selected_protects_locked_and_hidden() {
        // Mixed selection: one normal, one locked, one hidden.
        // Delete removes only the normal id; the others stay
        // selected (so the user can unlock/unhide).
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        let normal = Node::leaf(50, NodeKind::Rect, "normal");
        let mut locked = Node::leaf(51, NodeKind::Rect, "locked");
        locked.locked = true;
        let mut hidden = Node::leaf(52, NodeKind::Rect, "hidden");
        hidden.hidden = true;
        doc.pages[root_idx].children = vec![normal, locked, hidden];
        doc.selected_set = vec![NodeId::new(50), NodeId::new(51), NodeId::new(52)];
        doc.selected = NodeId::new(52);
        assert!(doc.delete_selected());
        // Normal id removed; locked + hidden survive.
        let ids: Vec<u64> = doc.pages[root_idx]
            .children
            .iter()
            .map(|n| n.id.raw())
            .collect();
        assert_eq!(ids, vec![51, 52]);
        let mut surviving: Vec<u64> = doc.selected_set.iter().map(|i| i.raw()).collect();
        surviving.sort();
        assert_eq!(surviving, vec![51, 52]);
    }

    #[test]
    fn locked_frame_children_remain_hittable() {
        // Codex CONCERN-Q5a regression: TS `spatial-index.ts`
        // filters locked per-node, not by subtree — children of
        // a locked Frame stay clickable. Rust must match.
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        let child = Node::leaf(21, NodeKind::Rect, "inner")
            .with_bounds(crate::Rect::xywh(20.0, 20.0, 40.0, 40.0));
        let mut frame = Node::with_children(20, NodeKind::Frame, "outer", vec![child])
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
        frame.locked = true;
        doc.pages[root_idx].children = vec![frame];
        // Click inside the child's rect → child selected (locked
        // doesn't propagate to children).
        assert_eq!(
            doc.node_at_doc_point(crate::Point2D::new(40.0, 40.0)),
            Some(NodeId::new(21))
        );
        // Click inside the Frame BUT outside the child → no hit
        // (the Frame body is locked).
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(80.0, 80.0)), None);
    }

    #[test]
    fn hidden_frame_children_skip_hit_test() {
        // Hidden cascades to subtree — child of a hidden Frame
        // is unclickable even when the click is in its own
        // bounds.
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        let child = Node::leaf(31, NodeKind::Rect, "inner")
            .with_bounds(crate::Rect::xywh(20.0, 20.0, 40.0, 40.0));
        let mut frame = Node::with_children(30, NodeKind::Frame, "outer", vec![child])
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
        frame.hidden = true;
        doc.pages[root_idx].children = vec![frame];
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(40.0, 40.0)), None);
    }

    #[test]
    fn toggle_node_locked_skips_canvas_hit_test_but_keeps_paint() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![Node::leaf(8, NodeKind::Rect, "a")
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0))];
        assert!(doc.toggle_node_locked(NodeId::new(8)));
        // Locked → canvas hit-test ignores it (TS parity:
        // locked layers can't be selected via canvas click; the
        // user has to click the layer-panel row to unlock).
        assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)), None);
        // Node still in document (so it still paints; canvas
        // viewport's paint_node only skips on `hidden`, not
        // `locked`).
        let node = doc
            .active_page()
            .unwrap()
            .find(NodeId::new(8))
            .expect("locked node still in document");
        assert!(node.locked);
        assert!(!node.hidden);
    }

    #[test]
    fn nodes_intersecting_doc_rect_returns_overlapping_top_level_ids() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![
            Node::leaf(2, NodeKind::Rect, "a").with_bounds(crate::Rect::xywh(0.0, 0.0, 50.0, 50.0)),
            Node::leaf(3, NodeKind::Rect, "b")
                .with_bounds(crate::Rect::xywh(100.0, 100.0, 50.0, 50.0)),
            Node::leaf(4, NodeKind::Rect, "c")
                .with_bounds(crate::Rect::xywh(40.0, 40.0, 30.0, 30.0)),
        ];
        // Rect (10, 10, 80, 80) overlaps "a" (touches origin) and
        // "c" (fully contained). Misses "b" at (100, 100).
        let hits = doc.nodes_intersecting_doc_rect(crate::Rect::xywh(10.0, 10.0, 80.0, 80.0));
        let ids: Vec<u64> = hits.iter().map(|i| i.raw()).collect();
        assert_eq!(ids, vec![2, 4]);
    }

    #[test]
    fn nodes_intersecting_doc_rect_handles_negative_size() {
        // Marquee drags going right-to-left or bottom-to-top
        // produce negative-size rects. The helper normalizes both
        // the query rect and node bounds via abs().
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![Node::leaf(2, NodeKind::Rect, "a")
            .with_bounds(crate::Rect::xywh(20.0, 20.0, 10.0, 10.0))];
        // Negative-size rect spanning (10, 10) → (40, 40).
        let hits = doc.nodes_intersecting_doc_rect(crate::Rect::xywh(40.0, 40.0, -30.0, -30.0));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0], NodeId::new(2));
    }

    #[test]
    fn nodes_intersecting_doc_rect_skips_degenerate_nodes() {
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        doc.pages[root_idx].children = vec![Node::leaf(2, NodeKind::Rect, "zero")
            .with_bounds(crate::Rect::xywh(5.0, 5.0, 0.0, 0.0))];
        let hits = doc.nodes_intersecting_doc_rect(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
        assert!(hits.is_empty());
    }

    #[test]
    fn property_panel_visible_only_for_single_selection() {
        // Codex stop-hook guard: the host's canvas_region uses
        // this gate to decide whether to reserve the right rail.
        // Multi-select must hide the panel AND give the canvas
        // back the rail's width.
        let mut doc = Document::sample();
        // No selection → hidden.
        doc.clear_selection();
        assert!(!doc.property_panel_visible());
        // Single selection → visible.
        doc.set_single_selection(NodeId::new(10));
        assert!(doc.property_panel_visible());
        // Multi-select → hidden.
        doc.toggle_selection(NodeId::new(11));
        assert_eq!(doc.selection_count(), 2);
        assert!(!doc.property_panel_visible());
        // Back to single → visible.
        doc.set_single_selection(NodeId::new(10));
        assert!(doc.property_panel_visible());
    }

    #[test]
    fn property_panel_visible_hides_stale_single_anchor() {
        // Codex stop-hook BLOCK guard: the panel paint gate
        // requires `selected_node().is_some()`, so the canvas-
        // region gate must too. A single-anchor selection that
        // resolves to no node (id not in active page, or stale
        // after a delete) must NOT reserve the right rail.
        let mut doc = Document::sample();
        // Point selection at a non-existent id.
        doc.set_single_selection(NodeId::new(9999));
        assert_eq!(doc.selection_count(), 1);
        assert!(doc.selected_node().is_none());
        assert!(!doc.property_panel_visible());

        // Selection on a node from a non-active page is also
        // unresolved (selected_node only scopes to active page).
        let mut doc2 = Document::empty();
        doc2.pages.push(Page::new(
            2,
            "p2",
            vec![Node::leaf(99, NodeKind::Rect, "alpha")],
        ));
        doc2.active_page_index = 0; // page 1 active
        doc2.set_single_selection(NodeId::new(99)); // node on page 2
        assert_eq!(doc2.selection_count(), 1);
        assert!(doc2.selected_node().is_none());
        assert!(!doc2.property_panel_visible());
    }

    #[test]
    fn multi_select_handle_hit_test_is_disabled() {
        // Codex CONCERN guard: handles aren't painted on multi-
        // select, so the hit-test must not return them either.
        use crate::widgets::{rotation_corner_at_point, selection_handle_at_point};
        let mut doc = Document::sample();
        // 2 selected → handles hidden, hit-test returns None.
        doc.selected_set = vec![NodeId::new(11), NodeId::new(13)];
        doc.selected = NodeId::new(13);
        let canvas_rect = crate::Rect::xywh(0.0, 0.0, 800.0, 600.0);
        // A point that would hit the anchor's top-left handle if
        // hit-test ran — at zoom 1, pan 0, anchor 13 bounds origin.
        let node = doc.selected_node().unwrap();
        let bounds = node.aggregate_bounds();
        let handle_point = crate::Point2D::new(
            canvas_rect.origin.x + bounds.origin.x,
            canvas_rect.origin.y + bounds.origin.y,
        );
        assert!(
            selection_handle_at_point(canvas_rect, &doc, handle_point).is_none(),
            "multi-select must not expose handle hit-tests"
        );
        assert!(
            rotation_corner_at_point(canvas_rect, &doc, handle_point).is_none(),
            "multi-select must not expose rotation hit-tests"
        );
        // Collapse to single-select → handles are interactive again.
        doc.set_single_selection(NodeId::new(13));
        assert!(selection_handle_at_point(canvas_rect, &doc, handle_point).is_some());
    }

    #[test]
    fn translate_selected_dedups_ancestor_descendant_pairs() {
        // Codex CONCERN guard: selecting BOTH a container AND one
        // of its descendants must NOT double-shift the descendant.
        let mut doc = Document::empty();
        let root_idx = doc.active_page_index;
        let child = Node::leaf(11, NodeKind::Rect, "child")
            .with_bounds(crate::Rect::xywh(50.0, 50.0, 20.0, 20.0));
        let frame = Node::with_children(10, NodeKind::Frame, "frame", vec![child])
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 200.0, 200.0));
        doc.pages[root_idx].children = vec![frame];
        doc.selected_set = vec![NodeId::new(10), NodeId::new(11)];
        doc.selected = NodeId::new(11);

        let child_before = doc
            .active_page()
            .unwrap()
            .find(NodeId::new(11))
            .unwrap()
            .bounds
            .origin;
        doc.translate_selected(10.0, 5.0);
        let child_after = doc
            .active_page()
            .unwrap()
            .find(NodeId::new(11))
            .unwrap()
            .bounds
            .origin;
        assert!((child_after.x - child_before.x - 10.0).abs() < 1e-3);
        assert!((child_after.y - child_before.y - 5.0).abs() < 1e-3);
    }

    #[test]
    fn translate_selected_moves_bounded_frame_with_descendants() {
        // Codex stop-hook: translating a bounded Frame must also
        // shift its descendants (whose bounds are document-space
        // absolute) — otherwise the children "detach" visually.
        let mut doc = Document::sample();
        doc.set_single_selection(NodeId::new(10)); // Frame
        let frame_before = doc.selected_node().unwrap().bounds.origin;
        // Walk a known descendant ahead of time.
        let descendant_before = doc
            .active_page()
            .unwrap()
            .find(NodeId::new(13)) // Button background rect
            .unwrap()
            .bounds
            .origin;
        doc.translate_selected(50.0, 25.0);
        let frame_after = doc.selected_node().unwrap().bounds.origin;
        let descendant_after = doc
            .active_page()
            .unwrap()
            .find(NodeId::new(13))
            .unwrap()
            .bounds
            .origin;
        assert!((frame_after.x - frame_before.x - 50.0).abs() < 1e-3);
        assert!((frame_after.y - frame_before.y - 25.0).abs() < 1e-3);
        // Descendant must shift by the SAME delta — proves it
        // didn't detach when its parent moved.
        assert!((descendant_after.x - descendant_before.x - 50.0).abs() < 1e-3);
        assert!((descendant_after.y - descendant_before.y - 25.0).abs() < 1e-3);
    }

    #[test]
    fn node_id_sentinel_is_zero() {
        assert_eq!(NodeId::NONE.raw(), 0);
        assert!(!NodeId::NONE.is_real());
        assert!(NodeId::new(1).is_real());
    }

    #[test]
    fn node_id_to_widget_id_round_trips_inner_value() {
        let nid = NodeId::new(42);
        let wid = nid.to_widget_id();
        assert_eq!(wid.0, 42);
    }

    #[test]
    fn node_find_walks_subtree() {
        let leaf = Node::leaf(3, NodeKind::Rect, "leaf");
        let mid = Node::with_children(2, NodeKind::Group, "mid", vec![leaf]);
        let root = Node::with_children(1, NodeKind::Frame, "root", vec![mid]);
        assert_eq!(root.find(NodeId::new(1)).unwrap().name, "root");
        assert_eq!(root.find(NodeId::new(2)).unwrap().name, "mid");
        assert_eq!(root.find(NodeId::new(3)).unwrap().name, "leaf");
        assert!(root.find(NodeId::new(99)).is_none());
    }

    #[test]
    fn document_sample_has_expected_shape() {
        let doc = Document::sample();
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.pages[0].name, "Page 1");
        // Frame > [Title, Button > [bg, text]]
        let frame = &doc.pages[0].children[0];
        assert_eq!(frame.kind, NodeKind::Frame);
        assert_eq!(frame.children.len(), 2);
        assert_eq!(frame.children[0].kind, NodeKind::Text);
        assert_eq!(frame.children[1].kind, NodeKind::Group);
        assert_eq!(frame.children[1].children.len(), 2);
    }

    #[test]
    fn document_selected_node_returns_real_hit() {
        let doc = Document::sample();
        let sel = doc.selected_node().unwrap();
        assert_eq!(sel.id, NodeId::new(11));
        assert_eq!(sel.name, "Title");
        assert_eq!(sel.kind, NodeKind::Text);
    }

    #[test]
    fn document_empty_has_no_selection() {
        let doc = Document::empty();
        assert_eq!(doc.selected, NodeId::NONE);
        assert!(doc.selected_node().is_none());
    }

    #[test]
    fn document_find_unknown_id_is_none() {
        let mut doc = Document::sample();
        doc.set_single_selection(NodeId::new(9999));
        assert!(doc.selected_node().is_none());
    }

    #[test]
    fn node_kind_label_matches_variant() {
        assert_eq!(NodeKind::Frame.label(), "Frame");
        assert_eq!(NodeKind::Group.label(), "Group");
        assert_eq!(NodeKind::Rect.label(), "Rect");
        assert_eq!(NodeKind::Text.label(), "Text");
        assert_eq!(NodeKind::Other("Custom".into()).label(), "Custom");
    }

    #[test]
    #[should_panic(expected = "id 0 is reserved for NodeId::NONE")]
    fn node_id_new_zero_panics_in_release_too() {
        // Codex Step 2 R1 BLOCK fix: hard panic, not just
        // debug_assert. `#[should_panic]` test runs in both debug
        // and release configurations.
        let _ = NodeId::new(0);
    }

    #[test]
    fn document_sample_passes_validate() {
        // Codex Step 2 R1 CONCERN-2: sample() must be invariant-
        // clean (no duplicate ids, valid active_page_index).
        let doc = Document::sample();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn document_validate_catches_duplicate_node_id() {
        let doc = Document {
            pages: vec![Page::new(
                1,
                "p",
                vec![
                    Node::leaf(2, NodeKind::Rect, "a"),
                    Node::leaf(2, NodeKind::Rect, "b"), // dup id 2
                ],
            )],
            active_page_index: 0,
            selected: NodeId::NONE,
            selected_set: Vec::new(),
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
        };
        let result = doc.validate();
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("duplicate NodeId"),
            "validate should mention duplicate"
        );
    }

    #[test]
    fn document_validate_catches_active_page_index_out_of_range() {
        let mut doc = Document::sample();
        doc.active_page_index = 99;
        let result = doc.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("active_page_index"));
    }

    #[test]
    fn document_validate_catches_empty_pages() {
        // Codex Step 2 R2 CONCERN-1: an empty-pages document with
        // any active_page_index used to silently pass validate().
        // Now empty pages is itself a violation.
        let doc = Document {
            pages: Vec::new(),
            active_page_index: 0,
            selected: NodeId::NONE,
            selected_set: Vec::new(),
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
        };
        let result = doc.validate();
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("pages is empty"),
            "validate should mention empty pages"
        );

        // Also covers the previously-uncaught case: empty pages
        // + bogus active_page_index → still rejected, AND the
        // empty-check fires FIRST (not the range check). Codex
        // Step 2 R3 CONCERN: prior version only asserted
        // `is_err()`, leaving ordering ambiguous.
        let doc2 = Document {
            pages: Vec::new(),
            active_page_index: 99,
            selected: NodeId::NONE,
            selected_set: Vec::new(),
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
        };
        let err2 = doc2.validate().unwrap_err();
        assert!(
            err2.contains("pages is empty"),
            "empty-pages check must fire before active_page_index range check; got: {err2}"
        );
        assert!(
            !err2.contains("active_page_index"),
            "empty-pages check must short-circuit; got both: {err2}"
        );
    }

    #[test]
    fn document_selected_node_scopes_to_active_page() {
        // Codex Step 2 R1 CONCERN-1: a selection on page 1 must
        // NOT show up when page 2 is active.
        let page1 = Page::new(1, "p1", vec![Node::leaf(10, NodeKind::Rect, "P1-A")]);
        let page2 = Page::new(2, "p2", vec![Node::leaf(20, NodeKind::Rect, "P2-A")]);
        let doc = Document {
            pages: vec![page1, page2],
            active_page_index: 1,      // page 2 active
            selected: NodeId::new(10), // selection points at page 1's node
            selected_set: vec![NodeId::new(10)],
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
        };
        // Selection is on a non-active page → returns None.
        assert!(doc.selected_node().is_none());

        // Switch active page to 0 and the same selection now
        // resolves.
        let doc2 = Document {
            active_page_index: 0,
            ..doc
        };
        assert_eq!(doc2.selected_node().unwrap().name, "P1-A");
    }

    #[test]
    fn document_active_page_returns_indexed_page() {
        let doc = Document {
            pages: vec![
                Page::new(1, "first", vec![]),
                Page::new(2, "second", vec![]),
            ],
            active_page_index: 1,
            selected: NodeId::NONE,
            selected_set: Vec::new(),
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
        };
        assert_eq!(doc.active_page().unwrap().name, "second");
    }

    #[test]
    fn document_active_page_returns_none_when_index_out_of_range() {
        let doc = Document {
            pages: vec![Page::new(1, "only", vec![])],
            active_page_index: 5,
            selected: NodeId::NONE,
            selected_set: Vec::new(),
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
        };
        assert!(doc.active_page().is_none());
    }

    #[test]
    fn node_id_raw_returns_inner_value() {
        assert_eq!(NodeId::new(42).raw(), 42);
        assert_eq!(NodeId::NONE.raw(), 0);
    }

    #[test]
    fn add_page_appends_named_page_and_switches() {
        let mut doc = Document::sample();
        let before = doc.pages.len();
        let max_before = doc.max_node_id();
        let new_idx = doc.add_page().expect("add_page should succeed");
        assert_eq!(doc.pages.len(), before + 1);
        assert_eq!(new_idx, before);
        assert_eq!(doc.active_page_index, before);
        let new_page = doc.active_page().unwrap();
        assert_eq!(new_page.name, format!("Page {}", before + 1));
        // Fresh id must NOT collide with any existing node id.
        assert!(new_page.id.raw() > max_before);
        // Newly-added page is empty; selection clears.
        assert!(new_page.children.is_empty());
        assert_eq!(doc.selected, NodeId::NONE);
        assert!(doc.selected_set.is_empty());

        // Second call exercises the `"Page N"` formula for n > 1.
        let new_idx_2 = doc.add_page().expect("second add_page");
        assert_eq!(new_idx_2, before + 1);
        assert_eq!(
            doc.active_page().unwrap().name,
            format!("Page {}", before + 2)
        );
    }

    #[test]
    fn add_page_returns_none_on_id_overflow() {
        // Fabricate a document whose largest id is u64::MAX so the
        // next page mint would overflow.
        let doc = Document {
            pages: vec![Page::new(u64::MAX, "max", vec![])],
            active_page_index: 0,
            selected: NodeId::NONE,
            selected_set: Vec::new(),
            clipboard: Vec::new(),
            tool: Tool::Select,
            viewport: Viewport::IDENTITY,
            chat: ChatState::default(),
            ui: UiState::default(),
        };
        let mut doc = doc;
        assert_eq!(doc.add_page(), None);
        assert_eq!(doc.pages.len(), 1, "overflow must leave pages untouched");
    }
}

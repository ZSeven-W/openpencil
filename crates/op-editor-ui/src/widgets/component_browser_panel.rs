//! `ComponentBrowserPanel` — the floating UIKit-browser panel.
//!
//! The Rust counterpart of the TS `component-browser-panel.tsx`: a
//! draggable floating panel listing every UIKit component the editor
//! ships with, grouped by category pills. Clicking a card queues an
//! instantiate request the desktop host drains against the viewport
//! centre. The panel is platform-free: the host paints it, maps a
//! click to a [`ComponentBrowserHit`], and owns the drag + insert.

use crate::theme::Theme;
use crate::widgets::button::{paint_button_feedback_wash, paint_ghost_button_feedback};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::{
    ButtonPressTarget, ComponentBrowserButton, ComponentCategory, EditorState, KitComponent,
    Locale, UIKit,
};

/// Panel width in logical px.
pub const COMPONENT_BROWSER_PANEL_W: f32 = 520.0;
/// Panel height in logical px.
pub const COMPONENT_BROWSER_PANEL_H: f32 = 460.0;

pub(in crate::widgets) const PAD: f32 = 14.0;
pub(in crate::widgets) const HEADER_H: f32 = 40.0;
const SEARCH_H: f32 = 42.0;
const PILL_ROW_H: f32 = 36.0;
/// The "Kit:" filter row below the header (TS kit-selector row).
pub(in crate::widgets) const KIT_ROW_H: f32 = 28.0;
/// One imported-kit management row (name / count / delete).
pub(in crate::widgets) const IMPORTED_ROW_H: f32 = 24.0;
const CLOSE_BTN: f32 = 24.0;
/// Gap between the header action buttons (TS `gap-1`).
const HEADER_BTN_GAP: f32 = 4.0;
const CARD_GAP: f32 = 8.0;
const CARD_COLS: usize = 3;
const CARD_H: f32 = 96.0;
/// Card preview-rect inset from the card edges.
const PREVIEW_INSET: f32 = 10.0;
/// Pill rect height + horizontal padding.
const PILL_H: f32 = 22.0;
const PILL_PAD_X: f32 = 10.0;
const PILL_GAP: f32 = 6.0;
/// Approximate glyph advance at the 11 px body size.
pub(in crate::widgets) const CHAR_W: f32 = 6.0;

/// The 6 categories actually shipped by the built-in starter kit
/// plus "All" — paint walks this in order and skips a category with
/// no matching components in the active kit set.
const CATEGORIES: [(Option<ComponentCategory>, &str); 9] = [
    (None, "componentBrowser.category.all"),
    (
        Some(ComponentCategory::Buttons),
        "componentBrowser.category.buttons",
    ),
    (
        Some(ComponentCategory::Inputs),
        "componentBrowser.category.inputs",
    ),
    (
        Some(ComponentCategory::Cards),
        "componentBrowser.category.cards",
    ),
    (
        Some(ComponentCategory::Navigation),
        "componentBrowser.category.nav",
    ),
    (
        Some(ComponentCategory::Layout),
        "componentBrowser.category.layout",
    ),
    (
        Some(ComponentCategory::Feedback),
        "componentBrowser.category.feedback",
    ),
    (
        Some(ComponentCategory::DataDisplay),
        "componentBrowser.category.data",
    ),
    (
        Some(ComponentCategory::Other),
        "componentBrowser.category.other",
    ),
];

/// What a click landed on inside the Component-Browser panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentBrowserHit {
    /// The header ✕ — close the panel.
    Close,
    /// The header Download button — queue a kit export (TS
    /// `handleExport`).
    ExportKit,
    /// The header Upload button — queue a kit import (TS
    /// `handleImport`).
    ImportKit,
    /// The kit-filter dropdown control — open / dismiss the popover.
    ToggleKitPicker,
    /// A kit-picker popover row — set the kit filter (`None` = All).
    SelectKitFilter(Option<String>),
    /// The Trash button on an imported-kit row — arm the inline
    /// delete confirmation (TS `setConfirmDeleteKitId`).
    RequestDeleteKit(String),
    /// The inline Delete confirm button — remove the kit (TS
    /// `handleDeleteKit`).
    ConfirmDeleteKit(String),
    /// The inline Cancel button — disarm the confirmation.
    CancelDeleteKit,
    /// The header bar (not on a button) — start a panel drag.
    DragHeader,
    /// A category pill — set the active category filter.
    SelectCategory(Option<ComponentCategory>),
    /// A component card — queue an instantiate of `(kit_id, comp_id)`.
    InsertComponent(String, String),
    /// Inside the panel but not on an interactive target.
    Inside,
}

/// The floating Component-Browser panel, built from an [`EditorState`].
/// Field visibility is widgets-scoped so the kit-management strip
/// (`component_browser_kits.rs`, sibling at the 800-line cap) can
/// share geometry + state.
pub struct ComponentBrowserPanel<'a> {
    pub(in crate::widgets) state: &'a EditorState,
    pub(in crate::widgets) theme: Theme,
    pub(in crate::widgets) locale: Locale,
    /// Currently-active category filter (`None` = All).
    active_category: Option<ComponentCategory>,
    /// Which target the cursor is over — drives the hover wash.
    pub(in crate::widgets) hover: Option<ComponentBrowserButton>,
    /// Which Component-Browser button is actively pressed.
    pub(in crate::widgets) pressed: Option<ComponentBrowserButton>,
}

impl<'a> ComponentBrowserPanel<'a> {
    /// Build the panel for the editor, or `None` when it is closed.
    pub fn for_editor(state: &'a EditorState) -> Option<ComponentBrowserPanel<'a>> {
        if !state.editor_ui.component_browser_open {
            return None;
        }
        Some(ComponentBrowserPanel {
            state,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            active_category: state.editor_ui.component_browser_category,
            hover: state.editor_ui.component_browser_hover,
            pressed: match state.editor_ui.pressed_button {
                Some(ButtonPressTarget::ComponentBrowser(button)) => Some(button),
                _ => None,
            },
        })
    }

    /// Resolve a pointer to a hoverable target (close / category pill /
    /// card). Shares the same `close_rect` / `pill_rects` / `card_rects`
    /// geometry as [`Self::hit_test`] so the wash lands on the clicked
    /// area; cards are index-addressable so no string ids are needed.
    pub fn hover_at(&self, panel: Rect, point: Point2D) -> Option<ComponentBrowserButton> {
        use ComponentBrowserButton as B;
        if !contains(panel, point) {
            return None;
        }
        // The open kit-picker popover sits over everything else.
        if self.state.editor_ui.component_browser_kit_picker_open {
            return self.hover_kit_picker(panel, point);
        }
        if contains(Self::close_rect(panel), point) {
            return Some(B::Close);
        }
        if contains(Self::import_rect(panel), point) {
            return Some(B::ImportKit);
        }
        if contains(Self::export_rect(panel), point) {
            return Some(B::ExportKit);
        }
        if point.y <= panel.origin.y + HEADER_H {
            return None;
        }
        if let Some(hover) = self.hover_kit_strip(panel, point) {
            return Some(hover);
        }
        for (rect, cat) in self.pill_rects(panel) {
            if contains(rect, point) {
                return Some(B::Category(cat));
            }
        }
        self.card_rects(panel)
            .into_iter()
            .position(|r| contains(r, point))
            .map(B::Card)
    }

    pub(in crate::widgets) fn t(&self, key: &'static str) -> &'static str {
        crate::i18n::translate(self.locale, key)
    }

    pub(in crate::widgets) fn is_pressed(&self, button: ComponentBrowserButton) -> bool {
        self.pressed == Some(button)
    }

    /// Vertical space the kit strip (filter row + imported-kit rows)
    /// occupies between the header and the category pills.
    pub(in crate::widgets) fn kit_strip_height(&self) -> f32 {
        KIT_ROW_H + self.imported_kits().len() as f32 * IMPORTED_ROW_H
    }

    /// The kits the panel browses — built-in + imported, load order.
    pub(in crate::widgets) fn kits(&self) -> &[UIKit] {
        &self.state.ui_kits
    }

    /// Imported (non-built-in) kits, in load order — the management
    /// rows under the kit-filter row (TS `importedKits`).
    pub(in crate::widgets) fn imported_kits(&self) -> Vec<&UIKit> {
        self.state.ui_kits.iter().filter(|k| !k.built_in).collect()
    }

    /// Categories actually present across the in-scope kits — pills
    /// for empty categories are hidden. "All" is always shown. The
    /// active kit filter narrows the scope when set.
    fn visible_categories(&self) -> Vec<(Option<ComponentCategory>, &'static str)> {
        let kit_filter = self.state.editor_ui.component_browser_kit_id.as_deref();
        let mut have = std::collections::HashSet::new();
        for kit in self.kits() {
            if let Some(want) = kit_filter {
                if kit.id != want {
                    continue;
                }
            }
            for comp in &kit.components {
                have.insert(comp.category);
            }
        }
        CATEGORIES
            .iter()
            .copied()
            .filter(|(c, _)| c.is_none() || c.is_some_and(|cat| have.contains(&cat)))
            .collect()
    }

    /// Walk every loaded kit yielding `(kit_id, component)` pairs the
    /// current filters admit (category pill, kit selector, search
    /// query — name + tags substring-match). Paint + hit-test share
    /// this so indices into the card grid agree. Widgets-scoped so the
    /// kit-strip sibling's tests can assert the kit filter narrows it.
    pub(in crate::widgets) fn filtered(&self) -> Vec<(&str, &KitComponent)> {
        let query = self
            .state
            .editor_ui
            .component_browser_search
            .trim()
            .to_lowercase();
        let kit_filter = self.state.editor_ui.component_browser_kit_id.as_deref();
        let mut out = Vec::new();
        for kit in self.kits() {
            if let Some(want) = kit_filter {
                if kit.id != want {
                    continue;
                }
            }
            for comp in &kit.components {
                if let Some(active) = self.active_category {
                    if comp.category != active {
                        continue;
                    }
                }
                if !query.is_empty()
                    && !comp.name.to_lowercase().contains(&query)
                    && !comp.tags.iter().any(|t| t.to_lowercase().contains(&query))
                {
                    continue;
                }
                out.push((kit.id.as_str(), comp));
            }
        }
        out
    }

    /// The three header buttons — Download (export), Upload (import),
    /// ✕ (close), right-aligned in TS order.
    fn close_rect(panel: Rect) -> Rect {
        let y = panel.origin.y + (HEADER_H - CLOSE_BTN) / 2.0;
        Rect {
            origin: Point2D::new(panel.origin.x + panel.size.x - PAD - CLOSE_BTN, y),
            size: Point2D::new(CLOSE_BTN, CLOSE_BTN),
        }
    }

    /// Upload — import a kit file (left of the close button).
    pub(in crate::widgets) fn import_rect(panel: Rect) -> Rect {
        let close = Self::close_rect(panel);
        Rect {
            origin: Point2D::new(close.origin.x - HEADER_BTN_GAP - CLOSE_BTN, close.origin.y),
            size: Point2D::new(CLOSE_BTN, CLOSE_BTN),
        }
    }

    /// Download — export the document's components as a kit (left of
    /// the import button).
    pub(in crate::widgets) fn export_rect(panel: Rect) -> Rect {
        let import = Self::import_rect(panel);
        Rect {
            origin: Point2D::new(
                import.origin.x - HEADER_BTN_GAP - CLOSE_BTN,
                import.origin.y,
            ),
            size: Point2D::new(CLOSE_BTN, CLOSE_BTN),
        }
    }

    /// The pill row's rects + their categories, in paint order. The
    /// pill row sits below the kit strip (TS panel order: header /
    /// kit selector / imported kits / pills / search / grid).
    fn pill_rects(&self, panel: Rect) -> Vec<(Rect, Option<ComponentCategory>)> {
        let pills = self.visible_categories();
        let mut out = Vec::with_capacity(pills.len());
        let mut x = panel.origin.x + PAD;
        let y = panel.origin.y + HEADER_H + self.kit_strip_height() + (PILL_ROW_H - PILL_H) / 2.0;
        for (cat, label_key) in pills {
            let label = self.t(label_key);
            let w = label.chars().count() as f32 * CHAR_W + PILL_PAD_X * 2.0;
            out.push((
                Rect {
                    origin: Point2D::new(x, y),
                    size: Point2D::new(w, PILL_H),
                },
                cat,
            ));
            x += w + PILL_GAP;
        }
        out
    }

    /// Top edge of the (clipped) card grid region.
    fn grid_clip_top(&self, panel: Rect) -> f32 {
        panel.origin.y + HEADER_H + self.kit_strip_height() + PILL_ROW_H + SEARCH_H
    }

    /// Card slot rects, in row-major order — paint + hit-test share
    /// this so a click maps onto the right component index.
    fn card_rects(&self, panel: Rect) -> Vec<Rect> {
        let count = self.filtered().len();
        let left = panel.origin.x + PAD;
        let top = self.grid_clip_top(panel) + CARD_GAP;
        let cols = CARD_COLS as f32;
        let inner_w = panel.size.x - PAD * 2.0;
        let card_w = (inner_w - CARD_GAP * (cols - 1.0)) / cols;
        (0..count)
            .map(|i| {
                let row = (i / CARD_COLS) as f32;
                let col = (i % CARD_COLS) as f32;
                Rect {
                    origin: Point2D::new(
                        left + col * (card_w + CARD_GAP),
                        top + row * (CARD_H + CARD_GAP),
                    ),
                    size: Point2D::new(card_w, CARD_H),
                }
            })
            .collect()
    }

    /// Map a click at `point` onto a [`ComponentBrowserHit`].
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<ComponentBrowserHit> {
        if !contains(panel, point) {
            return None;
        }
        // While the kit-filter popover is open it behaves like a
        // native `<select>`: a click either picks an option or
        // dismisses the popover, nothing else fires.
        if self.state.editor_ui.component_browser_kit_picker_open {
            return Some(self.hit_test_kit_picker(panel, point));
        }
        if contains(Self::close_rect(panel), point) {
            return Some(ComponentBrowserHit::Close);
        }
        if contains(Self::import_rect(panel), point) {
            return Some(ComponentBrowserHit::ImportKit);
        }
        if contains(Self::export_rect(panel), point) {
            return Some(ComponentBrowserHit::ExportKit);
        }
        // The header bar (above the kit strip) starts a drag.
        if point.y <= panel.origin.y + HEADER_H {
            return Some(ComponentBrowserHit::DragHeader);
        }
        if let Some(hit) = self.hit_test_kit_strip(panel, point) {
            return Some(hit);
        }
        for (rect, cat) in self.pill_rects(panel) {
            if contains(rect, point) {
                return Some(ComponentBrowserHit::SelectCategory(cat));
            }
        }
        let filtered = self.filtered();
        for (i, rect) in self.card_rects(panel).into_iter().enumerate() {
            if contains(rect, point) {
                let (kit_id, comp) = &filtered[i];
                return Some(ComponentBrowserHit::InsertComponent(
                    kit_id.to_string(),
                    comp.id.clone(),
                ));
            }
        }
        Some(ComponentBrowserHit::Inside)
    }

    /// Paint the panel into `rect`.
    pub fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 12.0, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, 12.0, self.theme.border, 1.0);

        // Header: title + close.
        self.text(
            cx,
            self.t("componentBrowser.title"),
            rect.origin.x + PAD,
            rect.origin.y + HEADER_H / 2.0 + 4.0,
            13.0,
            self.theme.foreground,
        );
        let close = Self::close_rect(rect);
        cx.backend.fill_round_rect(close, 6.0, self.theme.muted);
        let close_hovered = self.hover == Some(ComponentBrowserButton::Close);
        let close_pressed = self.is_pressed(ComponentBrowserButton::Close);
        let close_color = paint_ghost_button_feedback(
            cx.backend,
            &self.theme,
            close,
            close_hovered,
            close_pressed,
        );
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(close.origin.x + 5.0, close.origin.y + 5.0),
            CLOSE_BTN - 10.0,
            close_color,
            1.5,
        );
        // Header kit actions — Download (export) + Upload (import),
        // TS `component-browser-panel.tsx` header buttons.
        for (btn, icon, hover_target) in [
            (
                Self::export_rect(rect),
                Icon::Download,
                op_editor_core::ComponentBrowserButton::ExportKit,
            ),
            (
                Self::import_rect(rect),
                Icon::Upload,
                op_editor_core::ComponentBrowserButton::ImportKit,
            ),
        ] {
            let hovered = self.hover == Some(hover_target);
            let pressed = self.is_pressed(hover_target);
            let icon_color =
                paint_ghost_button_feedback(cx.backend, &self.theme, btn, hovered, pressed);
            draw_icon(
                cx.backend,
                icon,
                Point2D::new(btn.origin.x + 5.0, btn.origin.y + 5.0),
                CLOSE_BTN - 10.0,
                icon_color,
                1.5,
            );
        }
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, rect.origin.y + HEADER_H),
                size: Point2D::new(rect.size.x, 1.0),
            },
            self.theme.border,
        );

        // Kit-filter row + imported-kit management rows (sibling file).
        self.paint_kit_strip(cx, rect);

        // Search field. The native host routes keyboard input to this
        // filter whenever the panel is open.
        let search_rect = Rect {
            origin: Point2D::new(
                rect.origin.x + PAD,
                rect.origin.y + HEADER_H + self.kit_strip_height() + PILL_ROW_H + 8.0,
            ),
            size: Point2D::new(rect.size.x - PAD * 2.0, 28.0),
        };
        cx.backend
            .fill_round_rect(search_rect, 6.0, self.theme.muted);
        cx.backend
            .stroke_round_rect(search_rect, 6.0, self.theme.border, 1.0);
        draw_icon(
            cx.backend,
            Icon::Search,
            Point2D::new(search_rect.origin.x + 8.0, search_rect.origin.y + 7.0),
            14.0,
            self.theme.muted_foreground,
            1.4,
        );
        let query = self.state.editor_ui.component_browser_search.trim();
        let (search_text, search_color) = if query.is_empty() {
            (
                self.t("componentBrowser.searchComponents"),
                self.theme.muted_foreground,
            )
        } else {
            (query, self.theme.foreground)
        };
        // Ctrl/Cmd+A highlights the whole query (painted behind the text).
        if self.state.editor_ui.component_browser_select_all && !query.is_empty() {
            crate::widgets::text_selection::paint_single_line_selection(
                cx,
                &self.theme,
                query,
                search_rect.origin.x + 30.0,
                search_rect.origin.y + 19.0,
                12.0,
                search_rect.origin.x + search_rect.size.x - 8.0,
            );
        }
        self.text(
            cx,
            search_text,
            search_rect.origin.x + 30.0,
            search_rect.origin.y + 19.0,
            12.0,
            search_color,
        );

        // Category pills.
        for (pill, cat) in self.pill_rects(rect) {
            let active = cat == self.active_category;
            let (fill, fg) = if active {
                (self.theme.primary, self.theme.primary_foreground)
            } else {
                (self.theme.muted, self.theme.muted_foreground)
            };
            cx.backend.fill_round_rect(pill, PILL_H / 2.0, fill);
            let target = ComponentBrowserButton::Category(cat);
            paint_button_feedback_wash(
                cx.backend,
                &self.theme,
                pill,
                PILL_H / 2.0,
                self.hover == Some(target),
                self.is_pressed(target),
            );
            let label = self.label_for(cat);
            self.text(
                cx,
                label,
                pill.origin.x + PILL_PAD_X,
                pill.origin.y + PILL_H / 2.0 + 4.0,
                11.0,
                fg,
            );
        }
        let clip_top = self.grid_clip_top(rect);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, clip_top),
                size: Point2D::new(rect.size.x, 1.0),
            },
            self.theme.border,
        );

        // Card grid — clip so an over-tall component list cannot bleed
        // past the panel foot.
        cx.backend.save();
        cx.backend.clip_rect(Rect {
            origin: Point2D::new(rect.origin.x, clip_top + 1.0),
            size: Point2D::new(
                rect.size.x,
                (rect.origin.y + rect.size.y - clip_top - 1.0).max(0.0),
            ),
        });
        let filtered = self.filtered();
        if filtered.is_empty() {
            self.text(
                cx,
                self.t("componentBrowser.empty"),
                rect.origin.x + rect.size.x / 2.0 - 60.0,
                clip_top + 40.0,
                12.0,
                self.theme.muted_foreground,
            );
        } else {
            for (i, ((_, comp), card_rect)) in
                filtered.iter().zip(self.card_rects(rect)).enumerate()
            {
                let target = ComponentBrowserButton::Card(i);
                let hovered = self.hover == Some(target);
                self.paint_card(cx, card_rect, comp, hovered, self.is_pressed(target));
            }
        }
        cx.backend.restore();

        // Kit-filter popover paints last so it overlays the strip and
        // anything below it.
        if self.state.editor_ui.component_browser_kit_picker_open {
            self.paint_kit_picker(cx, rect);
        }
    }

    /// Translated label for a category — `None` = "All".
    fn label_for(&self, cat: Option<ComponentCategory>) -> &'static str {
        let key = CATEGORIES
            .iter()
            .find_map(|(c, k)| if *c == cat { Some(*k) } else { None })
            .unwrap_or("componentBrowser.category.all");
        self.t(key)
    }

    /// Paint one component card — surface fill + a tinted preview rect
    /// hinting at the component's footprint + the component name.
    fn paint_card(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        comp: &KitComponent,
        hovered: bool,
        pressed: bool,
    ) {
        cx.backend.fill_round_rect(rect, 8.0, self.theme.muted);
        paint_button_feedback_wash(cx.backend, &self.theme, rect, 8.0, hovered, pressed);
        cx.backend
            .stroke_round_rect(rect, 8.0, self.theme.border, 1.0);
        // Preview rect — scaled down to fit, anchored centre-top.
        let avail_w = rect.size.x - PREVIEW_INSET * 2.0;
        let avail_h = rect.size.y - PREVIEW_INSET * 2.0 - 18.0;
        let scale = (avail_w / comp.width.max(1.0))
            .min(avail_h / comp.height.max(1.0))
            .clamp(0.1, 1.0);
        let pw = (comp.width * scale).max(8.0);
        let ph = (comp.height * scale).max(2.0);
        let preview = Rect {
            origin: Point2D::new(
                rect.origin.x + (rect.size.x - pw) / 2.0,
                rect.origin.y + PREVIEW_INSET,
            ),
            size: Point2D::new(pw, ph),
        };
        cx.backend.fill_round_rect(preview, 4.0, self.theme.accent);
        // Component name.
        let label = truncate(&comp.name, 22);
        let label_w = label.chars().count() as f32 * CHAR_W;
        self.text(
            cx,
            &label,
            rect.origin.x + (rect.size.x - label_w) / 2.0,
            rect.origin.y + rect.size.y - 10.0,
            11.0,
            self.theme.foreground,
        );
    }

    pub(in crate::widgets) fn text(
        &self,
        cx: &mut PaintCx<'_>,
        s: &str,
        x: f32,
        baseline: f32,
        size: f32,
        color: Color,
    ) {
        let layout = TextLayout::single_run(
            s,
            "system-ui",
            size,
            (color).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&layout, Point2D::new(x, baseline));
    }
}

pub(in crate::widgets) fn contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}

pub(in crate::widgets) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

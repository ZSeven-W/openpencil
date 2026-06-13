//! `DesignMdPanel` — the floating Design-MD editor panel.
//!
//! The Rust counterpart of the TS `design-md-panel.tsx` +
//! `design-md-editor.tsx`: a draggable floating panel that shows the
//! open document's design-system brief (`PenDocument.design_md`) —
//! project name, visual theme, colour palette, typography and the
//! component / layout / notes sections — each collapsible, with the
//! markdown body rendered through the lightweight syntax-highlighting
//! renderer in [`crate::widgets::design_md_markdown`].
//!
//! The panel is platform-free: the desktop host paints it, maps a
//! click to a [`DesignMdHit`], and owns import / export / drag.

use crate::theme::Theme;
use crate::widgets::design_md_markdown::{parse_blocks, parse_inline, wrap_runs, MdBlock, MdRun};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::{DesignMdSpec, EditorState, Locale};

/// Panel width in logical px.
pub const DESIGN_MD_PANEL_W: f32 = 480.0;
/// Panel height in logical px.
pub const DESIGN_MD_PANEL_H: f32 = 560.0;

const PAD: f32 = 14.0;
const HEADER_H: f32 = 40.0;
/// Header-button side length + gap.
const BTN: f32 = 24.0;
const BTN_GAP: f32 = 6.0;
const SECTION_HEADER_H: f32 = 28.0;
const SECTION_GAP: f32 = 6.0;
/// Body text line height.
const LINE_H: f32 = 16.0;
/// Heading line height.
const HEADING_H: f32 = 18.0;
/// Colour-swatch row height.
const COLOR_ROW_H: f32 = 24.0;
const FOOTER_H: f32 = 22.0;
/// Gap between markdown blocks.
const BLOCK_GAP: f32 = 5.0;
/// Approximate glyph advance at the 11 px body size.
const CHAR_W: f32 = 6.0;
/// Body-text left indent inside a section.
const BODY_INDENT: f32 = 12.0;

/// The 6 design-md sections, in display order. The index doubles as
/// the `EditorUiState.design_md_expanded` bitmask position.
const SECTION_COUNT: usize = 6;

/// What a click landed on inside the Design-MD panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignMdHit {
    /// The header ✕ — close the panel.
    Close,
    /// The header import button — pick + parse a `.md` file.
    Import,
    /// The header export button — write the design.md to disk.
    Export,
    /// The footer "remove" link — clear `design_md`.
    Remove,
    /// A section header row — toggle section `index` (0..6) expanded.
    ToggleSection(u8),
    /// The header bar (not on a button) — start a panel drag.
    DragHeader,
    /// Inside the panel but not on an interactive target.
    Inside,
}

/// The floating Design-MD panel, built from an [`EditorState`].
pub struct DesignMdPanel<'a> {
    spec: Option<&'a DesignMdSpec>,
    theme: Theme,
    locale: Locale,
    /// Bitmask of expanded sections (bit 0 = theme … 5 = notes).
    expanded: u8,
    /// Which target the cursor is over — drives the hover wash.
    hover: Option<op_editor_core::DesignMdButton>,
}

/// One rendered line within a section body.
enum RenderLine {
    /// A markdown heading — font size + pre-split inline runs.
    Heading(f32, Vec<MdRun>),
    /// A paragraph / bullet continuation line.
    Body(Vec<MdRun>),
    /// A bullet's first line (gets a `•` marker).
    Bullet(Vec<MdRun>),
    /// A colour-palette row — index into `color_palette`.
    Color(usize),
    /// Vertical spacing between blocks.
    Gap,
}

impl RenderLine {
    fn height(&self) -> f32 {
        match self {
            RenderLine::Heading(..) => HEADING_H,
            RenderLine::Body(_) | RenderLine::Bullet(_) => LINE_H,
            RenderLine::Color(_) => COLOR_ROW_H,
            RenderLine::Gap => BLOCK_GAP,
        }
    }
}

/// One section's computed layout — header rect + (optional) body.
struct SectionLayout {
    /// Section index 0..6.
    index: u8,
    /// i18n key for the section title.
    title_key: &'static str,
    /// Clickable header-row rect.
    header: Rect,
    /// Whether the section is expanded.
    expanded: bool,
    /// `y` where the body begins (panel coords).
    body_top: f32,
    /// Rendered body lines (empty when collapsed).
    lines: Vec<RenderLine>,
}

impl<'a> DesignMdPanel<'a> {
    /// Build the panel for the editor, or `None` when it is closed.
    pub fn for_editor(state: &'a EditorState) -> Option<DesignMdPanel<'a>> {
        if !state.editor_ui.design_md_panel_open {
            return None;
        }
        Some(DesignMdPanel {
            spec: state.doc.design_md.as_ref(),
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            expanded: state.editor_ui.design_md_expanded,
            hover: state.editor_ui.design_md_hover,
        })
    }

    /// Resolve a pointer to a hoverable button. Reuses [`Self::hit_test`]
    /// and keeps only the button variants (drag-header / inside → None).
    pub fn hover_at(&self, panel: Rect, point: Point2D) -> Option<op_editor_core::DesignMdButton> {
        use op_editor_core::DesignMdButton as B;
        match self.hit_test(panel, point)? {
            DesignMdHit::Close => Some(B::Close),
            DesignMdHit::Import => Some(B::Import),
            DesignMdHit::Export => Some(B::Export),
            DesignMdHit::Remove => Some(B::Remove),
            DesignMdHit::ToggleSection(i) => Some(B::ToggleSection(i)),
            DesignMdHit::DragHeader | DesignMdHit::Inside => None,
        }
    }

    /// Translate `key` through the active locale tables.
    fn t(&self, key: &'static str) -> &'static str {
        crate::i18n::translate(self.locale, key)
    }

    /// Whether the spec carries content worth a section view (a bare
    /// `raw` / `project_name` does not count — mirrors the TS check).
    fn has_content(&self) -> bool {
        let Some(s) = self.spec else {
            return false;
        };
        s.visual_theme.is_some()
            || s.color_palette.as_ref().is_some_and(|c| !c.is_empty())
            || s.typography.is_some()
            || s.component_styles.is_some()
            || s.layout_principles.is_some()
            || s.generation_notes.is_some()
    }

    /// The three header buttons `[import, export, close]`, right-aligned.
    fn header_buttons(panel: Rect) -> [Rect; 3] {
        let y = panel.origin.y + (HEADER_H - BTN) / 2.0;
        let right = panel.origin.x + panel.size.x - PAD;
        let slot = |from_right: f32| Rect {
            origin: Point2D::new(right - (from_right + 1.0) * BTN - from_right * BTN_GAP, y),
            size: Point2D::new(BTN, BTN),
        };
        // from_right 0 = close (rightmost) … 2 = import.
        [slot(2.0), slot(1.0), slot(0.0)]
    }

    /// The raw markdown content of section `index`, capped at `limit`.
    fn section_content(&self, index: u8) -> Option<(String, usize)> {
        let s = self.spec?;
        match index {
            0 => s.visual_theme.clone().map(|c| (c, 600)),
            2 => s
                .typography
                .as_ref()
                .and_then(|t| t.scale.clone())
                .map(|c| (c, 600)),
            3 => s.component_styles.clone().map(|c| (c, 1000)),
            4 => s.layout_principles.clone().map(|c| (c, 1000)),
            5 => s.generation_notes.clone().map(|c| (c, 600)),
            _ => None,
        }
    }

    /// Whether section `index` has any content to show.
    fn section_present(&self, index: u8) -> bool {
        if index == 1 {
            self.spec
                .and_then(|s| s.color_palette.as_ref())
                .is_some_and(|c| !c.is_empty())
        } else {
            self.section_content(index).is_some()
        }
    }

    /// i18n title key for section `index`.
    fn section_title_key(index: u8) -> &'static str {
        match index {
            0 => "designMd.visualTheme",
            1 => "designMd.colors",
            2 => "designMd.typography",
            3 => "designMd.componentStyles",
            4 => "designMd.layoutPrinciples",
            _ => "designMd.generationNotes",
        }
    }

    /// Render section `index`'s body into positioned lines.
    fn section_lines(&self, index: u8) -> Vec<RenderLine> {
        // The colour section is a swatch list, not markdown.
        if index == 1 {
            let count = self
                .spec
                .and_then(|s| s.color_palette.as_ref())
                .map_or(0, |c| c.len());
            return (0..count).map(RenderLine::Color).collect();
        }
        let Some((content, limit)) = self.section_content(index) else {
            return Vec::new();
        };
        let budget = ((DESIGN_MD_PANEL_W - PAD * 2.0 - BODY_INDENT) / CHAR_W) as usize;
        let mut lines = Vec::new();
        for (bi, block) in parse_blocks(&content, limit).into_iter().enumerate() {
            if bi > 0 {
                lines.push(RenderLine::Gap);
            }
            match block {
                MdBlock::Heading { level, text } => {
                    let size = if level <= 3 { 12.0 } else { 11.5 };
                    lines.push(RenderLine::Heading(size, parse_inline(&text)));
                }
                MdBlock::Paragraph(text) => {
                    for wl in wrap_runs(&parse_inline(&text), budget) {
                        lines.push(RenderLine::Body(wl));
                    }
                }
                MdBlock::Bullet(text) => {
                    let wrapped = wrap_runs(&parse_inline(&text), budget.saturating_sub(2));
                    for (i, wl) in wrapped.into_iter().enumerate() {
                        if i == 0 {
                            lines.push(RenderLine::Bullet(wl));
                        } else {
                            lines.push(RenderLine::Body(wl));
                        }
                    }
                }
            }
        }
        lines
    }

    /// Compute every section's layout — shared by paint + hit-test so
    /// they always agree on row positions.
    fn layout(&self, panel: Rect) -> Vec<SectionLayout> {
        let left = panel.origin.x + PAD;
        let inner_w = panel.size.x - PAD * 2.0;
        let mut y = panel.origin.y + HEADER_H + SECTION_GAP;
        // A project-name line sits above the sections when present.
        if self.spec.and_then(|s| s.project_name.as_deref()).is_some() {
            y += HEADING_H + SECTION_GAP;
        }
        let mut out = Vec::new();
        for index in 0..SECTION_COUNT as u8 {
            if !self.section_present(index) {
                continue;
            }
            let expanded = self.expanded & (1 << index) != 0;
            let header = Rect {
                origin: Point2D::new(left, y),
                size: Point2D::new(inner_w, SECTION_HEADER_H),
            };
            y += SECTION_HEADER_H;
            let body_top = y;
            let lines = if expanded {
                self.section_lines(index)
            } else {
                Vec::new()
            };
            let body_h: f32 = lines.iter().map(RenderLine::height).sum();
            y += body_h + SECTION_GAP;
            out.push(SectionLayout {
                index,
                title_key: Self::section_title_key(index),
                header,
                expanded,
                body_top,
                lines,
            });
        }
        out
    }

    /// The footer "remove" link rect — below the last section.
    fn remove_rect(&self, panel: Rect) -> Rect {
        let bottom = self
            .layout(panel)
            .last()
            .map(|s| s.body_top + s.lines.iter().map(RenderLine::height).sum::<f32>())
            .unwrap_or(panel.origin.y + HEADER_H + SECTION_GAP);
        Rect {
            origin: Point2D::new(panel.origin.x + PAD, bottom + SECTION_GAP),
            size: Point2D::new(140.0, FOOTER_H),
        }
    }

    /// Map a click at `point` onto a [`DesignMdHit`]. `None` when the
    /// click is outside the panel rect.
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<DesignMdHit> {
        if !contains(panel, point) {
            return None;
        }
        let [import, export, close] = Self::header_buttons(panel);
        if contains(close, point) {
            return Some(DesignMdHit::Close);
        }
        if contains(import, point) {
            return Some(DesignMdHit::Import);
        }
        if contains(export, point) {
            return Some(DesignMdHit::Export);
        }
        // Anywhere else in the header bar starts a drag.
        if point.y <= panel.origin.y + HEADER_H {
            return Some(DesignMdHit::DragHeader);
        }
        if self.has_content() {
            for sec in self.layout(panel) {
                if contains(sec.header, point) {
                    return Some(DesignMdHit::ToggleSection(sec.index));
                }
            }
            if contains(self.remove_rect(panel), point) {
                return Some(DesignMdHit::Remove);
            }
        }
        Some(DesignMdHit::Inside)
    }

    /// Paint the panel into `rect`.
    pub fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 12.0, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, 12.0, self.theme.border, 1.0);

        // Header — title + import / export / close.
        self.text(
            cx,
            self.t("designMd.title"),
            rect.origin.x + PAD,
            rect.origin.y + HEADER_H / 2.0 + 4.0,
            13.0,
            self.theme.foreground,
        );
        let [import, export, close] = Self::header_buttons(rect);
        use op_editor_core::DesignMdButton as B;
        self.icon_button(cx, import, Icon::FolderOpen, self.hover == Some(B::Import));
        self.icon_button(cx, export, Icon::Download, self.hover == Some(B::Export));
        self.icon_button(cx, close, Icon::Close, self.hover == Some(B::Close));
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, rect.origin.y + HEADER_H),
                size: Point2D::new(rect.size.x, 1.0),
            },
            self.theme.border,
        );

        // Body — clipped so long content cannot bleed past the panel.
        cx.backend.save();
        cx.backend.clip_rect(Rect {
            origin: Point2D::new(rect.origin.x, rect.origin.y + HEADER_H + 1.0),
            size: Point2D::new(rect.size.x, rect.size.y - HEADER_H - 1.0),
        });
        if self.has_content() {
            self.paint_content(cx, rect);
        } else {
            self.paint_empty(cx, rect);
        }
        cx.backend.restore();
    }

    /// Paint the empty-state message.
    fn paint_empty(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let cx_mid = rect.origin.x + rect.size.x / 2.0;
        let cy = rect.origin.y + rect.size.y / 2.0;
        let heading = self.t("designMd.empty");
        let hint = self.t("designMd.emptyHint");
        self.text(
            cx,
            heading,
            cx_mid - heading.chars().count() as f32 * 3.5,
            cy - 6.0,
            13.0,
            self.theme.foreground,
        );
        self.text(
            cx,
            hint,
            cx_mid - hint.chars().count() as f32 * 3.0,
            cy + 14.0,
            11.0,
            self.theme.muted_foreground,
        );
    }

    /// Paint the project name + every present section.
    fn paint_content(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let left = rect.origin.x + PAD;
        if let Some(name) = self.spec.and_then(|s| s.project_name.as_deref()) {
            self.text(
                cx,
                &truncate(name, 56),
                left,
                rect.origin.y + HEADER_H + SECTION_GAP + 12.0,
                13.0,
                self.theme.foreground,
            );
        }
        for sec in self.layout(rect) {
            self.paint_section_header(cx, &sec);
            if sec.expanded {
                self.paint_section_body(cx, rect, &sec);
            }
        }
        // Footer "remove" link.
        let remove = self.remove_rect(rect);
        let remove_hovered = self.hover == Some(op_editor_core::DesignMdButton::Remove);
        if remove_hovered {
            cx.backend
                .fill_round_rect(remove, 6.0, self.theme.button_hover);
        }
        self.text(
            cx,
            self.t("designMd.remove"),
            remove.origin.x,
            remove.origin.y + 13.0,
            11.0,
            if remove_hovered {
                self.theme.foreground
            } else {
                self.theme.muted_foreground
            },
        );
    }

    /// Paint one section's collapsible header row.
    fn paint_section_header(&self, cx: &mut PaintCx<'_>, sec: &SectionLayout) {
        cx.backend
            .fill_round_rect(sec.header, 7.0, self.theme.muted);
        if self.hover == Some(op_editor_core::DesignMdButton::ToggleSection(sec.index)) {
            cx.backend
                .fill_round_rect(sec.header, 7.0, self.theme.button_hover);
        }
        let chevron = if sec.expanded { "▾" } else { "▸" };
        let baseline = sec.header.origin.y + SECTION_HEADER_H / 2.0 + 4.0;
        self.text(
            cx,
            chevron,
            sec.header.origin.x + 8.0,
            baseline,
            10.0,
            self.theme.muted_foreground,
        );
        self.text(
            cx,
            self.t(sec.title_key),
            sec.header.origin.x + 22.0,
            baseline,
            11.0,
            self.theme.foreground,
        );
    }

    /// Paint one expanded section's body lines.
    fn paint_section_body(&self, cx: &mut PaintCx<'_>, panel: Rect, sec: &SectionLayout) {
        let left = panel.origin.x + PAD + BODY_INDENT;
        let mut y = sec.body_top;
        for line in &sec.lines {
            match line {
                RenderLine::Heading(size, runs) => {
                    self.paint_runs(cx, runs, left, y + 12.0, *size, true);
                }
                RenderLine::Body(runs) => {
                    self.paint_runs(cx, runs, left, y + 11.0, 11.0, false);
                }
                RenderLine::Bullet(runs) => {
                    self.text(
                        cx,
                        "•",
                        left - 10.0,
                        y + 11.0,
                        11.0,
                        self.theme.muted_foreground,
                    );
                    self.paint_runs(cx, runs, left, y + 11.0, 11.0, false);
                }
                RenderLine::Color(i) => {
                    self.paint_color_row(cx, panel, *i, y);
                }
                RenderLine::Gap => {}
            }
            y += line.height();
        }
    }

    /// Paint one colour-palette row — swatch + name + hex/role.
    fn paint_color_row(&self, cx: &mut PaintCx<'_>, panel: Rect, index: usize, y: f32) {
        let Some(color) = self
            .spec
            .and_then(|s| s.color_palette.as_ref())
            .and_then(|c| c.get(index))
        else {
            return;
        };
        let left = panel.origin.x + PAD + BODY_INDENT;
        let swatch = Rect {
            origin: Point2D::new(left, y + 3.0),
            size: Point2D::new(16.0, 16.0),
        };
        cx.backend
            .fill_round_rect(swatch, 4.0, hex_to_color(&color.hex));
        cx.backend
            .stroke_round_rect(swatch, 4.0, self.theme.border, 1.0);
        let text_x = left + 24.0;
        self.text(
            cx,
            &truncate(&color.name, 28),
            text_x,
            y + 11.0,
            11.0,
            self.theme.foreground,
        );
        let detail = if color.role.is_empty() {
            color.hex.clone()
        } else {
            format!("{} — {}", color.hex, color.role)
        };
        self.text(
            cx,
            &truncate(&detail, 52),
            text_x,
            y + 22.0,
            10.0,
            self.theme.muted_foreground,
        );
    }

    /// Paint a sequence of inline-styled runs left-to-right. `heading`
    /// brightens plain text; non-headings render plain text muted.
    fn paint_runs(
        &self,
        cx: &mut PaintCx<'_>,
        runs: &[MdRun],
        x: f32,
        baseline: f32,
        size: f32,
        heading: bool,
    ) {
        let mut cur = x;
        for run in runs {
            match run {
                MdRun::Plain(s) => {
                    let color = if heading {
                        self.theme.foreground
                    } else {
                        self.theme.muted_foreground
                    };
                    self.text(cx, s, cur, baseline, size, color);
                    cur += s.chars().count() as f32 * CHAR_W;
                }
                MdRun::Bold(s) => {
                    self.text(cx, s, cur, baseline, size, self.theme.foreground);
                    cur += s.chars().count() as f32 * CHAR_W;
                }
                MdRun::Code(s) => {
                    let w = s.chars().count() as f32 * CHAR_W;
                    cx.backend.fill_round_rect(
                        Rect {
                            origin: Point2D::new(cur - 2.0, baseline - 10.0),
                            size: Point2D::new(w + 4.0, 14.0),
                        },
                        3.0,
                        self.theme.muted,
                    );
                    self.text(cx, s, cur, baseline, size, self.theme.foreground);
                    cur += w + 4.0;
                }
                MdRun::Color(hex) => {
                    let swatch = Rect {
                        origin: Point2D::new(cur, baseline - 9.0),
                        size: Point2D::new(10.0, 10.0),
                    };
                    cx.backend.fill_round_rect(swatch, 2.0, hex_to_color(hex));
                    cx.backend
                        .stroke_round_rect(swatch, 2.0, self.theme.border, 1.0);
                    self.text(cx, hex, cur + 14.0, baseline, size, self.theme.foreground);
                    cur += 14.0 + hex.chars().count() as f32 * CHAR_W;
                }
            }
        }
    }

    /// Paint one square header icon button.
    fn icon_button(&self, cx: &mut PaintCx<'_>, rect: Rect, icon: Icon, hovered: bool) {
        cx.backend.fill_round_rect(rect, 6.0, self.theme.muted);
        if hovered {
            cx.backend
                .fill_round_rect(rect, 6.0, self.theme.button_hover);
        }
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(rect.origin.x + 5.0, rect.origin.y + 5.0),
            BTN - 10.0,
            if hovered {
                self.theme.foreground
            } else {
                self.theme.muted_foreground
            },
            1.5,
        );
    }

    /// Draw one line of text.
    fn text(
        &self,
        cx: &mut PaintCx<'_>,
        s: &str,
        x: f32,
        baseline_y: f32,
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
        cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
    }
}

/// Whether `point` is inside `rect`.
fn contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}

/// Char truncation with an ellipsis.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// Parse a `#RRGGBB` hex into a [`Color`], falling back to mid-grey.
fn hex_to_color(hex: &str) -> Color {
    match op_editor_core::parse_hex_rgb(hex) {
        Some((r, g, b)) => Color { r, g, b, a: 1.0 },
        None => Color {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        },
    }
}

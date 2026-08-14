//! The `DESIGN.md` import box.
//!
//! Two ways in, one destination. A user who already has the guide on screen
//! pastes it; a user who has it on disk picks the file. Both hand the same
//! text to the same parser, so neither is a fallback for the other and
//! neither can produce a style the other could not.
//!
//! The box is what the import button opens on *every* host, including one
//! with a native file dialog. It used to fork — a host with a dialog went
//! straight to it and never showed this — which meant each platform offered
//! exactly one of the two routes, and a user who knew the other existed had
//! no way to reach it. The "choose file" button paints only where a host can
//! actually open a dialog, so the box never advertises a control that would
//! do nothing.
//!
//! It is a layer *inside* the Asset Center rather than a modal of its own:
//! the user is mid-task in the Styles tab, and replacing the gallery with a
//! separate window would lose the context they are importing into. That is
//! also why it owns every press inside the panel while it is up — a control
//! it covers must not fire through it.
//!
//! Multi-line by necessity. Every other chrome input here is one line and
//! flattens a paste to match; a `DESIGN.md` flattened to one line is no
//! longer a `DESIGN.md`, so this one keeps the newlines and wraps them.

use op_editor_core::SceneTemplateFocus;

use super::prompt_center_panel::estimated_text_width;
use super::scene_template_panel::{SceneTemplateHit, SceneTemplatePanel};
use super::scene_template_panel_paint::truncate_to_width;
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

/// Hover token for the paste box's confirm button.
pub(super) const STYLE_IMPORT_CONFIRM_HOVER: usize = usize::MAX - 160;
/// Hover token for its cancel button.
pub(super) const STYLE_IMPORT_CANCEL_HOVER: usize = usize::MAX - 161;
/// Hover token for its "choose file" button.
pub(super) const STYLE_IMPORT_PICK_HOVER: usize = usize::MAX - 162;

const BOX_RADIUS: f32 = 14.0;
const BOX_MAX_W: f32 = 660.0;
const BOX_MAX_H: f32 = 480.0;
const BOX_PAD: f32 = 22.0;
const TITLE_SIZE: f32 = 16.0;
const HINT_SIZE: f32 = 11.5;
const TEXT_SIZE: f32 = 11.5;
const TEXT_LINE_H: f32 = 16.0;
const BUTTON_W: f32 = 92.0;
const BUTTON_GAP: f32 = 10.0;
/// Wider than the confirm / cancel pair: its label is a phrase rather than a
/// verb, and every locale spells it longer than "Import".
const PICK_BUTTON_W: f32 = 148.0;
/// Below this the button is too small to read as one; a box that narrow drops
/// it rather than painting a stub, and the paste route still works.
const PICK_BUTTON_MIN_W: f32 = 56.0;
const SCRIM: crate::Color = crate::Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.45,
};

impl SceneTemplatePanel<'_> {
    /// Whether the paste box is up.
    pub fn style_import_open(&self) -> bool {
        self.state.editor_ui.scene_template_center.import.open
    }

    /// The box itself, centred in the panel.
    pub fn style_import_rect(&self, panel: Rect) -> Rect {
        let width = (panel.size.x - 96.0).clamp(240.0, BOX_MAX_W);
        let height = (panel.size.y - 96.0).clamp(200.0, BOX_MAX_H);
        Rect::xywh(
            panel.origin.x + (panel.size.x - width) / 2.0,
            panel.origin.y + (panel.size.y - height) / 2.0,
            width,
            height,
        )
    }

    /// The text area a pasted document lands in.
    pub fn style_import_text_rect(&self, panel: Rect) -> Rect {
        let box_rect = self.style_import_rect(panel);
        let button_h = self.style_import_action_height();
        let top = box_rect.origin.y + BOX_PAD + 30.0 + 22.0;
        let bottom = box_rect.origin.y + box_rect.size.y - BOX_PAD - button_h - 12.0 - 20.0;
        Rect::xywh(
            box_rect.origin.x + BOX_PAD,
            top,
            (box_rect.size.x - BOX_PAD * 2.0).max(0.0),
            (bottom - top).max(0.0),
        )
    }

    /// The confirm button, bottom-right.
    pub fn style_import_confirm_rect(&self, panel: Rect) -> Rect {
        let box_rect = self.style_import_rect(panel);
        let button_h = self.style_import_action_height();
        Rect::xywh(
            box_rect.origin.x + box_rect.size.x - BOX_PAD - BUTTON_W,
            box_rect.origin.y + box_rect.size.y - BOX_PAD - button_h,
            BUTTON_W,
            button_h,
        )
    }

    /// The cancel button, immediately left of confirm.
    pub fn style_import_cancel_rect(&self, panel: Rect) -> Rect {
        let confirm = self.style_import_confirm_rect(panel);
        Rect::xywh(
            confirm.origin.x - BUTTON_GAP - BUTTON_W,
            confirm.origin.y,
            BUTTON_W,
            self.style_import_action_height(),
        )
    }

    /// The "choose file" button, bottom-left — `None` on a host with no file
    /// dialog.
    ///
    /// Gated on the host capability rather than painted-and-disabled: a
    /// control that is visible but inert is a worse answer to "can I import a
    /// file here" than no control at all.
    pub fn style_import_pick_rect(&self, panel: Rect) -> Option<Rect> {
        if !self.state.editor_ui.style_import_file_picker_supported {
            return None;
        }
        let box_rect = self.style_import_rect(panel);
        let left = box_rect.origin.x + BOX_PAD;
        // It shares the row with cancel + confirm, and those two own their end
        // of it: the confirm button is where a user's hand already is. So this
        // one takes what is left rather than growing into them.
        let width = PICK_BUTTON_W.min(self.style_import_cancel_rect(panel).origin.x - left - 12.0);
        if width < PICK_BUTTON_MIN_W {
            return None;
        }
        Some(Rect::xywh(
            left,
            box_rect.origin.y + box_rect.size.y - BOX_PAD - self.style_import_action_height(),
            width,
            self.style_import_action_height(),
        ))
    }

    /// Route a press while the paste box is up.
    ///
    /// `None` means the box is closed and the press belongs to the gallery.
    /// Every other press inside the panel resolves here, including one on
    /// gallery chrome the box happens to cover: a press that fell through to
    /// a button hidden under a scrim would fire something the user cannot see.
    pub fn style_import_hit(&self, panel: Rect, point: Point2D) -> Option<SceneTemplateHit> {
        if !self.style_import_open() {
            return None;
        }
        if self.style_import_confirm_rect(panel).contains(point) {
            return Some(SceneTemplateHit::ConfirmStyleImport);
        }
        if self.style_import_cancel_rect(panel).contains(point) {
            return Some(SceneTemplateHit::CancelStyleImport);
        }
        if self
            .style_import_pick_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SceneTemplateHit::PickStyleImportFile);
        }
        let text = self.style_import_text_rect(panel);
        if text.contains(point) {
            return Some(SceneTemplateHit::FocusStyleImport(
                self.style_import_caret_at(text, point),
            ));
        }
        // Outside the box but still inside the panel: a press on the dimmed
        // gallery. It dismisses, matching what the dimming promises — the
        // same contract the gallery's own scrim keeps with the canvas.
        if !self.style_import_rect(panel).contains(point) {
            return Some(SceneTemplateHit::CancelStyleImport);
        }
        Some(SceneTemplateHit::InsideStyleImport)
    }

    /// Caret index for a press in the wrapped text area.
    ///
    /// Walks the same wrapped lines paint draws, so a click lands on the
    /// character under the pointer rather than on whatever a single-line
    /// measurement would have put there.
    fn style_import_caret_at(&self, rect: Rect, point: Point2D) -> usize {
        let text = self
            .state
            .editor_ui
            .scene_template_center
            .import
            .text
            .text();
        let inner_w = (rect.size.x - 20.0).max(1.0);
        let row = ((point.y - (rect.origin.y + 10.0)) / TEXT_LINE_H).max(0.0) as usize;
        let relative_x = (point.x - (rect.origin.x + 10.0)).max(0.0);

        let lines = wrap_document(text, inner_w, TEXT_SIZE);
        let Some((start, line)) = lines.get(row) else {
            return text.len();
        };
        let mut width = 0.0;
        for (offset, character) in line.char_indices() {
            let advance = estimated_text_width(&character.to_string(), TEXT_SIZE);
            if relative_x < width + advance / 2.0 {
                return start + offset;
            }
            width += advance;
        }
        start + line.len()
    }

    /// Paint the paste box over the gallery.
    pub fn paint_style_import(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        if !self.style_import_open() {
            return;
        }
        cx.backend.fill_rect(panel, SCRIM);
        let box_rect = self.style_import_rect(panel);
        cx.backend
            .fill_round_rect(box_rect, BOX_RADIUS, self.theme.popover);
        cx.backend
            .stroke_round_rect(box_rect, BOX_RADIUS, self.theme.border, 1.0);

        let left = box_rect.origin.x + BOX_PAD;
        let inner_w = (box_rect.size.x - BOX_PAD * 2.0).max(0.0);
        self.paint_text(
            cx,
            self.t("assetCenter.style.importTitle", "导入 DESIGN.md"),
            Point2D::new(left, box_rect.origin.y + BOX_PAD + 18.0),
            TITLE_SIZE,
            self.theme.foreground,
        );
        // Two hints, because the box genuinely offers two different things
        // depending on the host. Naming a file route on a host that has none
        // would send the user looking for a button that is not painted.
        let hint = if self.style_import_pick_rect(panel).is_some() {
            self.t(
                "assetCenter.style.importHintFile",
                "选择 DESIGN.md 文件,或在下方粘贴全文",
            )
        } else {
            self.t(
                "assetCenter.style.importHint",
                "粘贴 DESIGN.md 全文,然后确认导入",
            )
        };
        self.paint_text(
            cx,
            &truncate_to_width(hint, inner_w, HINT_SIZE),
            Point2D::new(left, box_rect.origin.y + BOX_PAD + 40.0),
            HINT_SIZE,
            self.theme.muted_foreground,
        );

        self.paint_style_import_text(cx, panel);

        // One line between the text area and the buttons, and a failure owns
        // it. The source hint answers "where do I get one of these" for a user
        // who opened the box with nothing to paste — deliberately prose, not a
        // link, because this is a canvas surface with no link affordance and a
        // fake one would be a control that does nothing when pressed. An error
        // is about the document they just handed over, so it displaces the
        // hint rather than sharing the button row: the row now starts with the
        // "choose file" button on the left, which is exactly where the message
        // used to be painted.
        let status_y = box_rect.origin.y + box_rect.size.y
            - BOX_PAD
            - self.style_import_action_height()
            - 14.0;
        match self.state.editor_ui.scene_template_center.import.error_key {
            Some(key) => self.paint_text(
                cx,
                &truncate_to_width(op_i18n::translate(self.locale, key), inner_w, HINT_SIZE),
                Point2D::new(left, status_y),
                HINT_SIZE,
                self.theme.destructive,
            ),
            None => self.paint_text(
                cx,
                &truncate_to_width(
                    self.t(
                        "assetCenter.style.importSource",
                        "可以从 styles.refero.design 等 DESIGN.md 风格库复制内容",
                    ),
                    inner_w,
                    HINT_SIZE,
                ),
                Point2D::new(left, status_y),
                HINT_SIZE,
                self.theme.muted_foreground,
            ),
        }

        if let Some(rect) = self.style_import_pick_rect(panel) {
            self.paint_style_import_action(
                cx,
                rect,
                &truncate_to_width(
                    self.t("assetCenter.style.importPickFile", "选择文件…"),
                    (rect.size.x - 12.0).max(0.0),
                    HINT_SIZE + 1.0,
                ),
                STYLE_IMPORT_PICK_HOVER,
                false,
            );
        }
        self.paint_style_import_action(
            cx,
            self.style_import_cancel_rect(panel),
            self.t("assetCenter.style.importCancel", "取消"),
            STYLE_IMPORT_CANCEL_HOVER,
            false,
        );
        self.paint_style_import_action(
            cx,
            self.style_import_confirm_rect(panel),
            self.t("assetCenter.style.importConfirm", "导入"),
            STYLE_IMPORT_CONFIRM_HOVER,
            true,
        );
    }

    fn paint_style_import_text(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let rect = self.style_import_text_rect(panel);
        cx.backend.fill_round_rect(rect, 8.0, self.theme.input);
        let focused =
            self.state.editor_ui.scene_template_center.focus == SceneTemplateFocus::Import;
        cx.backend.stroke_round_rect(
            rect,
            8.0,
            if focused {
                self.theme.ring
            } else {
                self.theme.border
            },
            1.0,
        );

        let text = self
            .state
            .editor_ui
            .scene_template_center
            .import
            .text
            .text();
        if text.is_empty() {
            self.paint_text(
                cx,
                self.t("assetCenter.style.importPlaceholder", "在此粘贴 DESIGN.md"),
                Point2D::new(rect.origin.x + 10.0, rect.origin.y + 22.0),
                TEXT_SIZE,
                self.theme.muted_foreground,
            );
            return;
        }

        cx.backend.save();
        cx.backend.clip_rect(rect);
        let inner_w = (rect.size.x - 20.0).max(1.0);
        let rows = ((rect.size.y - 16.0) / TEXT_LINE_H).max(1.0) as usize;
        // The head of the document, not a scrolling view of it. A paste is
        // reviewed by recognizing it, and the parser reads the whole text
        // regardless of how much of it is on screen.
        for (index, (_, line)) in wrap_document(text, inner_w, TEXT_SIZE)
            .into_iter()
            .take(rows)
            .enumerate()
        {
            self.paint_text(
                cx,
                &line,
                Point2D::new(
                    rect.origin.x + 10.0,
                    rect.origin.y + 22.0 + index as f32 * TEXT_LINE_H,
                ),
                TEXT_SIZE,
                self.theme.foreground,
            );
        }
        cx.backend.restore();
    }

    fn paint_style_import_action(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        label: &str,
        token: usize,
        primary: bool,
    ) {
        let (fill, foreground) = if primary {
            (self.theme.primary, self.theme.primary_foreground)
        } else {
            (self.theme.muted, self.theme.foreground)
        };
        cx.backend.fill_round_rect(rect, 8.0, fill);
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            rect,
            8.0,
            self.state.editor_ui.scene_template_center.hover == Some(token),
            self.is_pressed(token),
        );
        let label_w = estimated_text_width(label, HINT_SIZE + 1.0);
        self.paint_text(
            cx,
            label,
            Point2D::new(
                rect.origin.x + ((rect.size.x - label_w) / 2.0).max(6.0),
                jian_widgets::centered_text_baseline_y(rect, HINT_SIZE + 1.0),
            ),
            HINT_SIZE + 1.0,
            foreground,
        );
    }
}

/// Wrap a markdown document into display lines, honouring its own newlines.
///
/// Each entry carries the byte offset the line starts at in the original
/// text, which is what lets a click resolve to a caret position — the
/// alternative, re-deriving offsets by summing line lengths, silently
/// miscounts every line the wrapper broke rather than the author.
///
/// Blank lines are preserved: a markdown document's paragraph breaks are part
/// of how it reads, and collapsing them would make the pasted text look like
/// a different document from the one the user copied.
fn wrap_document(text: &str, max_width: f32, size: f32) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    let mut paragraph_start = 0_usize;
    for paragraph in text.split('\n') {
        let mut current = String::new();
        let mut current_start = paragraph_start;
        let mut width = 0.0;
        for (offset, character) in paragraph.char_indices() {
            let advance = estimated_text_width(&character.to_string(), size);
            if width + advance > max_width && !current.is_empty() {
                lines.push((current_start, std::mem::take(&mut current)));
                current_start = paragraph_start + offset;
                width = 0.0;
            }
            current.push(character);
            width += advance;
        }
        lines.push((current_start, current));
        // The `\n` the split consumed still occupies a byte.
        paragraph_start += paragraph.len() + 1;
        if lines.len() > 4096 {
            break;
        }
    }
    lines
}

#[cfg(test)]
#[path = "scene_template_style_import_tests.rs"]
mod scene_template_style_import_tests;

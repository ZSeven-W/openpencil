//! Theme-preset dropdown — the functional preset menu for variables panels.
//!
//! TS source: `apps/web/src/components/panels/variable-theme-manager.tsx`
//! :287-377 — popover `w-56` (224 px), `py-1`, `rounded-xl`, rows:
//! save-as-name (button or inline input), separator, saved-preset
//! list (name + per-row delete) or `variables.noPresets`, separator,
//! Import from File, Export to File.
//!
//! Documented divergences:
//! - The delete (trash) affordance paints on EVERY preset row instead
//!   of only on row hover (TS `opacity-0 group-hover:opacity-100`) —
//!   the Rust variables surfaces track no live row hover yet.
//! - The popover is LEFT-aligned to its anchor; TS uses `right-0`
//!   (right-aligned to the preset button). Left alignment keeps the
//!   host-side anchor simple and stable.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::Locale;
use op_editor_core::EditorState;

/// TS `w-56` = 14rem = 224 px.
pub const PRESET_MENU_W: f32 = 224.0;
/// TS `mt-1` gap between the anchor button and the popover.
const ANCHOR_GAP: f32 = 4.0;
/// TS `py-1` popover padding.
const PAD_Y: f32 = 4.0;
/// Action rows (`px-3 py-2`, 13 px text).
const ACTION_ROW_H: f32 = 36.0;
/// The save-as-name input row (`px-3 py-2` wrapper + input).
const INPUT_ROW_H: f32 = 40.0;
/// Saved-preset rows (`px-3 py-1.5`).
const LIST_ROW_H: f32 = 30.0;
/// `variables.noPresets` placeholder row (`px-3 py-2`, 12 px text).
const EMPTY_ROW_H: f32 = 34.0;
/// `h-px` separator + `my-1` margins.
const SEPARATOR_H: f32 = 9.0;
/// TS `rounded-xl`.
const RADIUS: f32 = 12.0;
const PAD_X: f32 = 12.0;
const ICON_SIZE: f32 = 14.0;
const TRASH_SIZE: f32 = 12.0;

/// One actionable target inside the open preset dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetMenuHit {
    /// "Save Current as Preset…" row (swaps to the name input).
    SaveCurrent,
    /// The active save-as-name input row.
    NameInput,
    /// Load the saved preset at index (merge into the document).
    Load(usize),
    /// Delete the saved preset at index.
    Delete(usize),
    /// "Import from File…" row.
    Import,
    /// "Export to File…" row.
    Export,
    /// Inside the popover but not on a row — swallow (TS clicks
    /// inside the popover never close it).
    Blank,
}

/// Snapshot widget for the preset dropdown. Built per frame from
/// `EditorState` like the other variables widgets.
pub struct ThemePresetMenu {
    theme: Theme,
    locale: Locale,
    preset_names: Vec<String>,
    name_input_active: bool,
    name_draft: String,
    caret_pos: usize,
}

impl ThemePresetMenu {
    pub fn for_editor(state: &EditorState) -> Self {
        let name_input_active = state.editor_ui.preset_name_input_active();
        Self {
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            preset_names: state.theme_presets.iter().map(|p| p.name.clone()).collect(),
            name_input_active,
            name_draft: if name_input_active {
                state.ui.property_input_draft.clone()
            } else {
                String::new()
            },
            caret_pos: state.ui.property_caret_pos,
        }
    }

    /// Whether the dropdown should show at all.
    pub fn is_open(state: &EditorState) -> bool {
        state.editor_ui.variables_preset_menu_open
    }

    fn save_row_h(&self) -> f32 {
        if self.name_input_active {
            INPUT_ROW_H
        } else {
            ACTION_ROW_H
        }
    }

    fn list_h(&self) -> f32 {
        if self.preset_names.is_empty() {
            EMPTY_ROW_H
        } else {
            self.preset_names.len() as f32 * LIST_ROW_H
        }
    }

    fn menu_h(&self) -> f32 {
        PAD_Y * 2.0
            + self.save_row_h()
            + SEPARATOR_H
            + self.list_h()
            + SEPARATOR_H
            + ACTION_ROW_H * 2.0
    }

    /// Popover rect for an anchor button rect (left-aligned, below).
    pub fn menu_rect(&self, anchor: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                anchor.origin.x,
                anchor.origin.y + anchor.size.y + ANCHOR_GAP,
            ),
            size: Point2D::new(PRESET_MENU_W, self.menu_h()),
        }
    }

    fn save_row_rect(&self, menu: Rect) -> Rect {
        row(menu, menu.origin.y + PAD_Y, self.save_row_h())
    }

    fn list_top(&self, menu: Rect) -> f32 {
        menu.origin.y + PAD_Y + self.save_row_h() + SEPARATOR_H
    }

    fn preset_row_rect(&self, menu: Rect, idx: usize) -> Rect {
        row(
            menu,
            self.list_top(menu) + idx as f32 * LIST_ROW_H,
            LIST_ROW_H,
        )
    }

    /// Per-row trash button (right-aligned inside the row).
    fn delete_rect(&self, row_rect: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                row_rect.origin.x + row_rect.size.x - PAD_X - 18.0,
                row_rect.origin.y + (row_rect.size.y - 18.0) / 2.0,
            ),
            size: Point2D::new(18.0, 18.0),
        }
    }

    fn import_row_rect(&self, menu: Rect) -> Rect {
        row(
            menu,
            self.list_top(menu) + self.list_h() + SEPARATOR_H,
            ACTION_ROW_H,
        )
    }

    fn export_row_rect(&self, menu: Rect) -> Rect {
        let import = self.import_row_rect(menu);
        row(menu, import.origin.y + import.size.y, ACTION_ROW_H)
    }

    /// `None` = outside the popover (the host decides whether that
    /// closes it — TS closes on outside mousedown unless the press
    /// is on the anchor button, which toggles instead).
    pub fn hit_test(&self, menu: Rect, point: Point2D) -> Option<PresetMenuHit> {
        if !menu.contains(point) {
            return None;
        }
        if self.save_row_rect(menu).contains(point) {
            return Some(if self.name_input_active {
                PresetMenuHit::NameInput
            } else {
                PresetMenuHit::SaveCurrent
            });
        }
        for idx in 0..self.preset_names.len() {
            let row_rect = self.preset_row_rect(menu, idx);
            if row_rect.contains(point) {
                return Some(if self.delete_rect(row_rect).contains(point) {
                    PresetMenuHit::Delete(idx)
                } else {
                    PresetMenuHit::Load(idx)
                });
            }
        }
        if self.import_row_rect(menu).contains(point) {
            return Some(PresetMenuHit::Import);
        }
        if self.export_row_rect(menu).contains(point) {
            return Some(PresetMenuHit::Export);
        }
        Some(PresetMenuHit::Blank)
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, menu: Rect) {
        let theme = self.theme;
        cx.backend.fill_round_rect(menu, RADIUS, theme.popover);
        cx.backend
            .stroke_round_rect(menu, RADIUS, theme.border, 1.0);

        // Save row — button or inline name input.
        let save = self.save_row_rect(menu);
        if self.name_input_active {
            self.paint_name_input(cx, save);
        } else {
            action_row(
                cx,
                theme,
                save,
                Icon::Save,
                op_i18n::translate(self.locale, "variables.savePreset"),
            );
        }
        separator(cx, theme, menu, save.origin.y + save.size.y + PAD_Y);

        // Saved presets list.
        if self.preset_names.is_empty() {
            let empty = row(menu, self.list_top(menu), EMPTY_ROW_H);
            draw_text(
                cx,
                op_i18n::translate(self.locale, "variables.noPresets"),
                12.0,
                (theme.muted_foreground).with_alpha(0.5),
                empty.origin.x + PAD_X,
                empty.origin.y + 21.0,
            );
        } else {
            for (idx, name) in self.preset_names.iter().enumerate() {
                let row_rect = self.preset_row_rect(menu, idx);
                cx.backend.save();
                cx.backend.clip_rect(row_rect);
                draw_text(
                    cx,
                    name,
                    13.0,
                    theme.foreground,
                    row_rect.origin.x + PAD_X,
                    row_rect.origin.y + 20.0,
                );
                cx.backend.restore();
                let trash = self.delete_rect(row_rect);
                draw_icon(
                    cx.backend,
                    Icon::Trash,
                    Point2D::new(
                        trash.origin.x + (trash.size.x - TRASH_SIZE) / 2.0,
                        trash.origin.y + (trash.size.y - TRASH_SIZE) / 2.0,
                    ),
                    TRASH_SIZE,
                    theme.muted_foreground,
                    1.4,
                );
            }
        }
        let list_bottom = self.list_top(menu) + self.list_h();
        separator(cx, theme, menu, list_bottom + PAD_Y);

        // Import / Export rows.
        action_row(
            cx,
            theme,
            self.import_row_rect(menu),
            Icon::Upload,
            op_i18n::translate(self.locale, "variables.importPreset"),
        );
        action_row(
            cx,
            theme,
            self.export_row_rect(menu),
            Icon::Download,
            op_i18n::translate(self.locale, "variables.exportPreset"),
        );
    }

    fn paint_name_input(&self, cx: &mut PaintCx<'_>, save: Rect) {
        let theme = self.theme;
        let input = Rect {
            origin: Point2D::new(save.origin.x + PAD_X, save.origin.y + 6.0),
            size: Point2D::new(save.size.x - PAD_X * 2.0, save.size.y - 12.0),
        };
        cx.backend.fill_round_rect(input, 8.0, theme.muted);
        cx.backend.stroke_round_rect(input, 8.0, theme.primary, 1.0);
        let text_x = input.origin.x + 8.0;
        let baseline = input.origin.y + input.size.y / 2.0 + 4.5;
        if self.name_draft.is_empty() {
            draw_text(
                cx,
                op_i18n::translate(self.locale, "variables.presetName"),
                13.0,
                theme.muted_foreground,
                text_x,
                baseline,
            );
        } else {
            draw_text(
                cx,
                &self.name_draft,
                13.0,
                theme.foreground,
                text_x,
                baseline,
            );
        }
        // Solid caret — same convention as the variables panel's
        // inline inputs (no blink).
        let clipped = boundary_at_or_before(&self.name_draft, self.caret_pos);
        let caret_x = text_x + cx.backend.measure_text(&self.name_draft[..clipped], 13.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(caret_x, input.origin.y + (input.size.y - 16.0) / 2.0),
                size: Point2D::new(1.5, 16.0),
            },
            theme.foreground,
        );
    }
}

fn row(menu: Rect, y: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D::new(menu.origin.x, y),
        size: Point2D::new(menu.size.x, h),
    }
}

fn action_row(cx: &mut PaintCx<'_>, theme: Theme, rect: Rect, icon: Icon, label: &str) {
    let center_y = rect.origin.y + rect.size.y / 2.0;
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + PAD_X, center_y - ICON_SIZE / 2.0),
        ICON_SIZE,
        theme.muted_foreground,
        1.6,
    );
    draw_text(
        cx,
        label,
        13.0,
        theme.foreground,
        rect.origin.x + PAD_X + ICON_SIZE + 10.0,
        center_y + 4.5,
    );
}

fn separator(cx: &mut PaintCx<'_>, theme: Theme, menu: Rect, y: f32) {
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(menu.origin.x + 1.0, y),
            size: Point2D::new(menu.size.x - 2.0, 1.0),
        },
        (theme.border).with_alpha(0.5),
    );
}

fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, baseline_y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
}

fn boundary_at_or_before(value: &str, pos: usize) -> usize {
    let mut clipped = pos.min(value.len());
    while clipped > 0 && !value.is_char_boundary(clipped) {
        clipped -= 1;
    }
    clipped
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::variable::{VariableKind, VariableScalar};

    fn anchor() -> Rect {
        Rect {
            origin: Point2D::new(100.0, 50.0),
            size: Point2D::new(122.0, 32.0),
        }
    }

    fn state_with_presets(count: usize) -> EditorState {
        let mut state = EditorState::new();
        assert!(state.create_variable(
            "brand",
            VariableKind::Color,
            VariableScalar::Str("#ff0000".into()),
        ));
        for i in 0..count {
            assert!(state.save_theme_preset(&format!("kit-{i}"), 10 + i as u64));
        }
        state.editor_ui.variables_preset_menu_open = true;
        state
    }

    #[test]
    fn menu_rect_is_left_aligned_224_below_the_anchor() {
        let menu = ThemePresetMenu::for_editor(&state_with_presets(0));
        let rect = menu.menu_rect(anchor());
        assert_eq!(rect.origin.x, 100.0);
        assert_eq!(rect.origin.y, 50.0 + 32.0 + 4.0);
        assert_eq!(rect.size.x, PRESET_MENU_W);
        // Empty menu still covers the native panel's 144px stub.
        assert!(rect.size.y > 144.0);
    }

    #[test]
    fn hit_test_resolves_every_row_kind() {
        let menu = ThemePresetMenu::for_editor(&state_with_presets(2));
        let rect = menu.menu_rect(anchor());

        let save = menu.save_row_rect(rect);
        assert_eq!(
            menu.hit_test(rect, center(save)),
            Some(PresetMenuHit::SaveCurrent)
        );
        let row1 = menu.preset_row_rect(rect, 1);
        assert_eq!(
            menu.hit_test(rect, Point2D::new(row1.origin.x + PAD_X, center(row1).y)),
            Some(PresetMenuHit::Load(1))
        );
        assert_eq!(
            menu.hit_test(rect, center(menu.delete_rect(row1))),
            Some(PresetMenuHit::Delete(1))
        );
        assert_eq!(
            menu.hit_test(rect, center(menu.import_row_rect(rect))),
            Some(PresetMenuHit::Import)
        );
        assert_eq!(
            menu.hit_test(rect, center(menu.export_row_rect(rect))),
            Some(PresetMenuHit::Export)
        );
        assert_eq!(
            menu.hit_test(rect, Point2D::new(rect.origin.x - 4.0, rect.origin.y)),
            None
        );
    }

    #[test]
    fn name_input_replaces_the_save_row_when_focused() {
        let mut state = state_with_presets(0);
        state.editor_ui.variables_preset_name_focus = true;
        state.ui.property_input_draft = "Mid".to_string();
        state.ui.property_caret_pos = 3;
        let menu = ThemePresetMenu::for_editor(&state);
        assert!(menu.name_input_active);
        assert_eq!(menu.name_draft, "Mid");
        let rect = menu.menu_rect(anchor());
        assert_eq!(
            menu.hit_test(rect, center(menu.save_row_rect(rect))),
            Some(PresetMenuHit::NameInput)
        );
        // Focus flag without the menu open must stay inert.
        state.editor_ui.variables_preset_menu_open = false;
        let stale = ThemePresetMenu::for_editor(&state);
        assert!(!stale.name_input_active);
    }

    fn center(r: Rect) -> Point2D {
        Point2D::new(r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0)
    }
}

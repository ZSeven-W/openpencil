//! Component-Browser kit-management strip — the kit-filter row, the
//! imported-kit rows (with inline delete confirmation) and the
//! kit-filter popover. Sibling of `component_browser_panel.rs` (kept
//! out of the panel file for the 800-line cap).
//!
//! Mirrors the TS `component-browser-panel.tsx` kit surface:
//! - a "Kit:" selector row (TS uses a native `<select>`; this paints
//!   a popover over the panel),
//! - one management row per imported kit — name, "N components",
//!   Trash → inline Delete / Cancel confirm.

use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::component_browser_panel::{
    truncate, ComponentBrowserHit, ComponentBrowserPanel, CHAR_W, HEADER_H, IMPORTED_ROW_H,
    KIT_ROW_H, PAD,
};
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Point2D, Rect};
use op_editor_core::ComponentBrowserButton as Btn;

/// Dropdown control width on the "Kit:" row.
const KIT_DROPDOWN_W: f32 = 200.0;
const KIT_DROPDOWN_H: f32 = 20.0;
/// Popover option-row height.
const OPTION_H: f32 = 22.0;
/// Trash button square on an imported row.
const TRASH_BTN: f32 = 18.0;
/// Inline confirm-button height (TS `text-[10px] px-1.5 py-0.5`).
const CONFIRM_H: f32 = 16.0;
const CONFIRM_PAD_X: f32 = 6.0;
const CONFIRM_GAP: f32 = 4.0;

impl ComponentBrowserPanel<'_> {
    // -- geometry -----------------------------------------------------------

    fn kit_row_top(panel: Rect) -> f32 {
        panel.origin.y + HEADER_H
    }

    /// The kit-filter dropdown control on the "Kit:" row.
    pub(in crate::widgets) fn kit_dropdown_rect(&self, panel: Rect) -> Rect {
        let label_w = self.t("componentBrowser.kit").chars().count() as f32 * CHAR_W;
        let x = panel.origin.x + PAD + label_w + 8.0;
        let w = KIT_DROPDOWN_W.min(panel.size.x - (x - panel.origin.x) - PAD);
        Rect {
            origin: Point2D::new(
                x,
                Self::kit_row_top(panel) + (KIT_ROW_H - KIT_DROPDOWN_H) / 2.0,
            ),
            size: Point2D::new(w, KIT_DROPDOWN_H),
        }
    }

    /// One full-width rect per imported kit, in load order.
    fn imported_row_rects(&self, panel: Rect) -> Vec<Rect> {
        let top = Self::kit_row_top(panel) + KIT_ROW_H;
        (0..self.imported_kits().len())
            .map(|i| Rect {
                origin: Point2D::new(panel.origin.x, top + i as f32 * IMPORTED_ROW_H),
                size: Point2D::new(panel.size.x, IMPORTED_ROW_H),
            })
            .collect()
    }

    fn trash_rect(panel: Rect, row: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                panel.origin.x + panel.size.x - PAD - TRASH_BTN,
                row.origin.y + (IMPORTED_ROW_H - TRASH_BTN) / 2.0,
            ),
            size: Point2D::new(TRASH_BTN, TRASH_BTN),
        }
    }

    /// `(delete_rect, cancel_rect)` for an armed confirmation row.
    fn confirm_rects(&self, panel: Rect, row: Rect) -> (Rect, Rect) {
        let delete_label = self.t("common.delete");
        let cancel_label = self.t("common.cancel");
        let delete_w = delete_label.chars().count() as f32 * 5.0 + CONFIRM_PAD_X * 2.0;
        let cancel_w = cancel_label.chars().count() as f32 * 5.0 + CONFIRM_PAD_X * 2.0;
        let y = row.origin.y + (IMPORTED_ROW_H - CONFIRM_H) / 2.0;
        let cancel_x = panel.origin.x + panel.size.x - PAD - cancel_w;
        let delete_x = cancel_x - CONFIRM_GAP - delete_w;
        (
            Rect {
                origin: Point2D::new(delete_x, y),
                size: Point2D::new(delete_w, CONFIRM_H),
            },
            Rect {
                origin: Point2D::new(cancel_x, y),
                size: Point2D::new(cancel_w, CONFIRM_H),
            },
        )
    }

    /// Popover option rows — `(rect, kit_id)` where `None` = "All".
    /// Index 0 is "All", then every loaded kit in order (TS lists all
    /// kits in its `<select>`, suffixing imported ones).
    fn kit_option_rects(&self, panel: Rect) -> Vec<(Rect, Option<String>)> {
        let anchor = self.kit_dropdown_rect(panel);
        let mut out = Vec::with_capacity(self.kits().len() + 1);
        let mut y = anchor.origin.y + anchor.size.y + 2.0;
        let mut push = |id: Option<String>, y: &mut f32| {
            out.push((
                Rect {
                    origin: Point2D::new(anchor.origin.x, *y),
                    size: Point2D::new(anchor.size.x, OPTION_H),
                },
                id,
            ));
            *y += OPTION_H;
        };
        push(None, &mut y);
        for kit in self.kits() {
            push(Some(kit.id.clone()), &mut y);
        }
        out
    }

    fn kit_popover_rect(&self, panel: Rect) -> Rect {
        let opts = self.kit_option_rects(panel);
        let first = opts.first().expect("popover always has the All row").0;
        Rect {
            origin: first.origin,
            size: Point2D::new(first.size.x, opts.len() as f32 * OPTION_H),
        }
    }

    // -- hit-test + hover ---------------------------------------------------

    /// Popover-open click routing: an option picks, anything else
    /// dismisses (native-`<select>` semantics).
    pub(in crate::widgets) fn hit_test_kit_picker(
        &self,
        panel: Rect,
        point: Point2D,
    ) -> ComponentBrowserHit {
        for (rect, id) in self.kit_option_rects(panel) {
            if rect.contains(point) {
                return ComponentBrowserHit::SelectKitFilter(id);
            }
        }
        ComponentBrowserHit::ToggleKitPicker
    }

    pub(in crate::widgets) fn hover_kit_picker(&self, panel: Rect, point: Point2D) -> Option<Btn> {
        self.kit_option_rects(panel)
            .iter()
            .position(|(rect, _)| rect.contains(point))
            .map(Btn::KitOption)
    }

    /// Hit-test the kit row + imported-kit rows. `None` falls through
    /// to the pills / cards below the strip.
    pub(in crate::widgets) fn hit_test_kit_strip(
        &self,
        panel: Rect,
        point: Point2D,
    ) -> Option<ComponentBrowserHit> {
        if self.kit_dropdown_rect(panel).contains(point) {
            return Some(ComponentBrowserHit::ToggleKitPicker);
        }
        let confirm_id = self
            .state
            .editor_ui
            .component_browser_confirm_delete_kit
            .as_deref();
        let imported = self.imported_kits();
        for (i, row) in self.imported_row_rects(panel).into_iter().enumerate() {
            if !row.contains(point) {
                continue;
            }
            let kit = imported[i];
            if confirm_id == Some(kit.id.as_str()) {
                let (delete, cancel) = self.confirm_rects(panel, row);
                if delete.contains(point) {
                    return Some(ComponentBrowserHit::ConfirmDeleteKit(kit.id.clone()));
                }
                if cancel.contains(point) {
                    return Some(ComponentBrowserHit::CancelDeleteKit);
                }
            } else if Self::trash_rect(panel, row).contains(point) {
                return Some(ComponentBrowserHit::RequestDeleteKit(kit.id.clone()));
            }
            return Some(ComponentBrowserHit::Inside);
        }
        None
    }

    pub(in crate::widgets) fn hover_kit_strip(&self, panel: Rect, point: Point2D) -> Option<Btn> {
        if self.kit_dropdown_rect(panel).contains(point) {
            return Some(Btn::KitFilter);
        }
        let confirm_id = self
            .state
            .editor_ui
            .component_browser_confirm_delete_kit
            .as_deref();
        let imported = self.imported_kits();
        for (i, row) in self.imported_row_rects(panel).into_iter().enumerate() {
            if !row.contains(point) {
                continue;
            }
            if confirm_id == Some(imported[i].id.as_str()) {
                let (delete, cancel) = self.confirm_rects(panel, row);
                if delete.contains(point) {
                    return Some(Btn::KitConfirmDelete(i));
                }
                if cancel.contains(point) {
                    return Some(Btn::KitCancelDelete(i));
                }
            } else if Self::trash_rect(panel, row).contains(point) {
                return Some(Btn::KitDelete(i));
            }
            return None;
        }
        None
    }

    // -- paint --------------------------------------------------------------

    /// The kit-filter row + imported-kit rows, between the header and
    /// the category pills.
    pub(in crate::widgets) fn paint_kit_strip(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let row_top = Self::kit_row_top(panel);
        // "Kit:" label.
        self.text(
            cx,
            self.t("componentBrowser.kit"),
            panel.origin.x + PAD,
            row_top + KIT_ROW_H / 2.0 + 4.0,
            11.0,
            self.theme.muted_foreground,
        );
        // Dropdown control: current filter label + chevron.
        let dropdown = self.kit_dropdown_rect(panel);
        let current = self.kit_filter_label();
        jian_widgets::components::select_trigger::SelectTrigger {
            label: &current,
            placeholder: "",
            hovered: self.hover == Some(Btn::KitFilter),
            pressed: self.is_pressed(Btn::KitFilter),
            enabled: true,
            font_size: 11.0,
        }
        .paint(
            cx.backend,
            dropdown,
            &crate::widgets::button::tokens_from_theme(&self.theme),
        );
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(panel.origin.x, row_top + KIT_ROW_H),
                size: Point2D::new(panel.size.x, 1.0),
            },
            self.theme.border,
        );

        // Imported-kit management rows.
        let confirm_id = self
            .state
            .editor_ui
            .component_browser_confirm_delete_kit
            .as_deref();
        let imported = self.imported_kits();
        for (i, row) in self.imported_row_rects(panel).into_iter().enumerate() {
            let kit = imported[i];
            self.text(
                cx,
                &truncate(&kit.name, 28),
                panel.origin.x + PAD,
                row.origin.y + IMPORTED_ROW_H / 2.0 + 4.0,
                11.0,
                self.theme.foreground,
            );
            if confirm_id == Some(kit.id.as_str()) {
                let (delete, cancel) = self.confirm_rects(panel, row);
                let delete_hovered = self.hover == Some(Btn::KitConfirmDelete(i));
                cx.backend
                    .fill_round_rect(delete, 4.0, self.theme.destructive);
                paint_button_feedback_wash(
                    cx.backend,
                    &self.theme,
                    delete,
                    4.0,
                    delete_hovered,
                    self.is_pressed(Btn::KitConfirmDelete(i)),
                );
                self.text(
                    cx,
                    self.t("common.delete"),
                    delete.origin.x + CONFIRM_PAD_X,
                    delete.origin.y + CONFIRM_H / 2.0 + 3.5,
                    10.0,
                    self.theme.primary_foreground,
                );
                cx.backend.fill_round_rect(cancel, 4.0, self.theme.muted);
                paint_button_feedback_wash(
                    cx.backend,
                    &self.theme,
                    cancel,
                    4.0,
                    self.hover == Some(Btn::KitCancelDelete(i)),
                    self.is_pressed(Btn::KitCancelDelete(i)),
                );
                self.text(
                    cx,
                    self.t("common.cancel"),
                    cancel.origin.x + CONFIRM_PAD_X,
                    cancel.origin.y + CONFIRM_H / 2.0 + 3.5,
                    10.0,
                    self.theme.muted_foreground,
                );
            } else {
                // "N components" count, right-aligned before the trash.
                let count_label = format!(
                    "{} {}",
                    kit.components.len(),
                    self.t("componentBrowser.components")
                );
                let trash = Self::trash_rect(panel, row);
                let count_w = count_label.chars().count() as f32 * 5.0;
                self.text(
                    cx,
                    &count_label,
                    trash.origin.x - 8.0 - count_w,
                    row.origin.y + IMPORTED_ROW_H / 2.0 + 3.5,
                    10.0,
                    self.theme.muted_foreground,
                );
                paint_button_feedback_wash(
                    cx.backend,
                    &self.theme,
                    trash,
                    4.0,
                    self.hover == Some(Btn::KitDelete(i)),
                    self.is_pressed(Btn::KitDelete(i)),
                );
                draw_icon(
                    cx.backend,
                    Icon::Trash,
                    Point2D::new(trash.origin.x + 3.0, trash.origin.y + 3.0),
                    TRASH_BTN - 6.0,
                    self.theme.muted_foreground,
                    1.4,
                );
            }
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(panel.origin.x, row.origin.y + IMPORTED_ROW_H),
                    size: Point2D::new(panel.size.x, 1.0),
                },
                self.theme.border,
            );
        }
    }

    /// The open kit-filter popover (painted after everything else).
    pub(in crate::widgets) fn paint_kit_picker(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let popover = self.kit_popover_rect(panel);
        cx.backend.fill_round_rect(popover, 6.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(popover, 6.0, self.theme.border, 1.0);
        let active = self.state.editor_ui.component_browser_kit_id.as_deref();
        for (i, (rect, id)) in self.kit_option_rects(panel).into_iter().enumerate() {
            paint_button_feedback_wash(
                cx.backend,
                &self.theme,
                rect,
                4.0,
                self.hover == Some(Btn::KitOption(i)),
                self.is_pressed(Btn::KitOption(i)),
            );
            let label = match id.as_deref() {
                None => self.t("componentBrowser.all").to_string(),
                Some(kit_id) => self.option_label(kit_id),
            };
            let selected = id.as_deref() == active;
            self.text(
                cx,
                &truncate(&label, ((rect.size.x - 12.0) / CHAR_W) as usize),
                rect.origin.x + 6.0,
                rect.origin.y + OPTION_H / 2.0 + 4.0,
                11.0,
                if selected {
                    self.theme.primary
                } else {
                    self.theme.foreground
                },
            );
        }
    }

    /// Label for the dropdown control — the active kit's name or
    /// "All".
    fn kit_filter_label(&self) -> String {
        match self.state.editor_ui.component_browser_kit_id.as_deref() {
            None => self.t("componentBrowser.all").to_string(),
            Some(id) => self.option_label(id),
        }
    }

    /// Kit display label — imported kits get the TS `(imported)`
    /// suffix.
    fn option_label(&self, kit_id: &str) -> String {
        match self.kits().iter().find(|k| k.id == kit_id) {
            Some(kit) if kit.built_in => kit.name.clone(),
            Some(kit) => format!("{} {}", kit.name, self.t("componentBrowser.imported")),
            None => kit_id.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::{COMPONENT_BROWSER_PANEL_H, COMPONENT_BROWSER_PANEL_W};
    use crate::{Color, RenderBackend, TextLayout};
    use op_editor_core::uikit::{KitComponent, UIKit};
    use op_editor_core::ButtonPressTarget;
    use op_editor_core::EditorState;

    fn imported_kit(id: &str) -> UIKit {
        let starter = op_editor_core::uikit::builtin_starter_kit();
        let comp = starter
            .components
            .into_iter()
            .find(|c| c.id == "btn-primary")
            .expect("starter component");
        UIKit {
            id: id.to_string(),
            name: format!("Kit {id}"),
            version: "1.0.0".to_string(),
            built_in: false,
            components: vec![KitComponent {
                id: "c1".to_string(),
                name: "Imported Button".to_string(),
                category: op_editor_core::ComponentCategory::Buttons,
                tags: vec!["button".to_string()],
                width: 120.0,
                height: 40.0,
                template: comp.template,
            }],
            variables: None,
        }
    }

    fn open_state_with_import() -> EditorState {
        let mut state = EditorState::new();
        state.editor_ui.component_browser_open = true;
        state.ui_kits.push(imported_kit("imp-1"));
        state
    }

    fn panel_rect() -> Rect {
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(COMPONENT_BROWSER_PANEL_W, COMPONENT_BROWSER_PANEL_H),
        }
    }

    fn centre(r: Rect) -> Point2D {
        Point2D::new(r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0)
    }

    #[derive(Default)]
    struct CaptureBackend {
        round_fills: Vec<(Rect, f32, Color)>,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}

        fn end_frame(&mut self) {}

        fn fill_rect(&mut self, _rect: Rect, _color: Color) {}

        fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {}

        fn draw_text(&mut self, _layout: &TextLayout, _origin: Point2D) {}

        fn clip_rect(&mut self, _rect: Rect) {}

        fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {}

        fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
            self.round_fills.push((rect, radius, color));
        }

        fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {}

        fn stroke_svg_path(
            &mut self,
            _d: &str,
            _top_left: Point2D,
            _size: f32,
            _color: Color,
            _width: f32,
        ) {
        }

        fn save(&mut self) {}

        fn restore(&mut self) {}

        fn translate(&mut self, _offset: Point2D) {}

        fn resize(&mut self, _width: u32, _height: u32) {}

        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn header_buttons_hit_export_import_close() {
        let state = open_state_with_import();
        let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
        let rect = panel_rect();
        assert_eq!(
            panel.hit_test(rect, centre(ComponentBrowserPanel::export_rect(rect))),
            Some(ComponentBrowserHit::ExportKit)
        );
        assert_eq!(
            panel.hit_test(rect, centre(ComponentBrowserPanel::import_rect(rect))),
            Some(ComponentBrowserHit::ImportKit)
        );
    }

    #[test]
    fn kit_dropdown_toggles_and_popover_selects_filter() {
        let mut state = open_state_with_import();
        {
            let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
            let rect = panel_rect();
            let dropdown = panel.kit_dropdown_rect(rect);
            assert_eq!(
                panel.hit_test(rect, centre(dropdown)),
                Some(ComponentBrowserHit::ToggleKitPicker)
            );
        }
        state.editor_ui.component_browser_kit_picker_open = true;
        let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
        let rect = panel_rect();
        let options = panel.kit_option_rects(rect);
        // Index 0 = All, then the kits in load order; the imported kit
        // is last.
        assert_eq!(
            panel.hit_test(rect, centre(options[0].0)),
            Some(ComponentBrowserHit::SelectKitFilter(None))
        );
        let (last_rect, last_id) = options.last().expect("imported option").clone();
        assert_eq!(last_id.as_deref(), Some("imp-1"));
        assert_eq!(
            panel.hit_test(rect, centre(last_rect)),
            Some(ComponentBrowserHit::SelectKitFilter(Some(
                "imp-1".to_string()
            )))
        );
        // A click off the options dismisses (select semantics) — even
        // over the close button.
        let close_centre = centre(Rect {
            origin: Point2D::new(rect.origin.x + rect.size.x - 28.0, rect.origin.y + 8.0),
            size: Point2D::new(24.0, 24.0),
        });
        assert_eq!(
            panel.hit_test(rect, close_centre),
            Some(ComponentBrowserHit::ToggleKitPicker)
        );
    }

    #[test]
    fn imported_row_delete_arms_then_confirms_or_cancels() {
        let mut state = open_state_with_import();
        let rect = panel_rect();
        {
            let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
            let row = panel.imported_row_rects(rect)[0];
            let trash = ComponentBrowserPanel::trash_rect(rect, row);
            assert_eq!(
                panel.hit_test(rect, centre(trash)),
                Some(ComponentBrowserHit::RequestDeleteKit("imp-1".to_string()))
            );
        }
        state.editor_ui.component_browser_confirm_delete_kit = Some("imp-1".to_string());
        let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
        let row = panel.imported_row_rects(rect)[0];
        let (delete, cancel) = panel.confirm_rects(rect, row);
        assert_eq!(
            panel.hit_test(rect, centre(delete)),
            Some(ComponentBrowserHit::ConfirmDeleteKit("imp-1".to_string()))
        );
        assert_eq!(
            panel.hit_test(rect, centre(cancel)),
            Some(ComponentBrowserHit::CancelDeleteKit)
        );
    }

    #[test]
    fn kit_filter_narrows_cards_and_strip_height_tracks_imports() {
        let mut state = open_state_with_import();
        let total: usize = state.ui_kits.iter().map(|k| k.components.len()).sum();
        {
            let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
            assert_eq!(panel.filtered().len(), total);
            assert_eq!(
                panel.kit_strip_height(),
                KIT_ROW_H + IMPORTED_ROW_H,
                "one imported kit = one management row"
            );
        }
        state.editor_ui.component_browser_kit_id = Some("imp-1".to_string());
        let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
        assert_eq!(panel.filtered().len(), 1);
        assert_eq!(panel.filtered()[0].0, "imp-1");
    }

    #[test]
    fn hover_targets_cover_the_kit_strip() {
        let mut state = open_state_with_import();
        let rect = panel_rect();
        {
            let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
            let dropdown = panel.kit_dropdown_rect(rect);
            assert_eq!(panel.hover_at(rect, centre(dropdown)), Some(Btn::KitFilter));
            assert_eq!(
                panel.hover_at(rect, centre(ComponentBrowserPanel::import_rect(rect))),
                Some(Btn::ImportKit)
            );
            assert_eq!(
                panel.hover_at(rect, centre(ComponentBrowserPanel::export_rect(rect))),
                Some(Btn::ExportKit)
            );
            let row = panel.imported_row_rects(rect)[0];
            let trash = ComponentBrowserPanel::trash_rect(rect, row);
            assert_eq!(panel.hover_at(rect, centre(trash)), Some(Btn::KitDelete(0)));
        }
        state.editor_ui.component_browser_kit_picker_open = true;
        let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
        let options = panel.kit_option_rects(rect);
        assert_eq!(
            panel.hover_at(rect, centre(options[1].0)),
            Some(Btn::KitOption(1))
        );
    }

    #[test]
    fn for_editor_picks_up_pressed_component_browser_button() {
        let mut state = open_state_with_import();
        state.editor_ui.pressed_button = Some(ButtonPressTarget::ComponentBrowser(Btn::ImportKit));

        let panel = ComponentBrowserPanel::for_editor(&state).expect("open");

        assert_eq!(panel.pressed, Some(Btn::ImportKit));
    }

    #[test]
    fn pressed_header_button_paints_pressed_feedback() {
        let mut state = open_state_with_import();
        state.editor_ui.pressed_button = Some(ButtonPressTarget::ComponentBrowser(Btn::ImportKit));
        let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
        let rect = panel_rect();
        let import = ComponentBrowserPanel::import_rect(rect);
        let expected = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(fill, radius, color)| *fill == import
                    && *radius == 6.0
                    && *color == expected),
            "pressed import button should paint shared pressed feedback"
        );
    }

    #[test]
    fn pressed_kit_dropdown_paints_pressed_feedback() {
        let mut state = open_state_with_import();
        state.editor_ui.pressed_button = Some(ButtonPressTarget::ComponentBrowser(Btn::KitFilter));
        let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
        let rect = panel_rect();
        let dropdown = panel.kit_dropdown_rect(rect);
        let expected = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(fill, radius, color)| *fill == dropdown
                    && *radius == 4.0
                    && *color == expected),
            "pressed kit dropdown should paint shared pressed feedback"
        );
    }

    #[test]
    fn pressed_imported_kit_delete_button_paints_pressed_feedback() {
        let mut state = open_state_with_import();
        state.editor_ui.pressed_button =
            Some(ButtonPressTarget::ComponentBrowser(Btn::KitDelete(0)));
        let panel = ComponentBrowserPanel::for_editor(&state).expect("open");
        let rect = panel_rect();
        let row = panel.imported_row_rects(rect)[0];
        let trash = ComponentBrowserPanel::trash_rect(rect, row);
        let expected = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(fill, radius, color)| *fill == trash
                    && *radius == 4.0
                    && *color == expected),
            "pressed imported-kit delete should paint shared pressed feedback"
        );
    }
}

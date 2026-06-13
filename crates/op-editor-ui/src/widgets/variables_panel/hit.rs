//! VariablesPanel hit-testing — split from the spine to honor the
//! 800-line cap. The press-target mapping shares every rect helper
//! with paint via `super::*` so the two can't drift.

use super::*;

impl VariablesPanel {
    /// Map a screen-space point to the row or chip it falls in.
    /// Honors `dropdown_open` first so a click on a value row of
    /// the open dropdown overlay wins over the chip / row beneath.
    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<VariablesPanelHit> {
        if !(rect).contains(point) {
            return None;
        }
        // Resize handles hug the panel edges and float above content
        // (TS z-10 strips), so they win first.
        if let Some(edge) = self.resize_edge_at(rect, point) {
            return Some(VariablesPanelHit::Resize(edge));
        }
        if (close_rect(rect)).contains(point) {
            return Some(VariablesPanelHit::Close);
        }
        // Row `⋯` overflow menu — floating overlay above rows AND the
        // footer, so it hit-tests before both.
        if let Some((display_idx, menu)) = self.row_menu_rect(rect) {
            if (menu).contains(point) {
                let source = self.rows[display_idx].source_idx;
                let row = ((point.y - menu.origin.y) / ADD_VARIABLE_MENU_ROW_HEIGHT).floor();
                return match row as usize {
                    0 => Some(VariablesPanelHit::RowMenuRename(source)),
                    1 => Some(VariablesPanelHit::RowMenuDelete(source)),
                    _ => None,
                };
            }
        }
        if let Some(axis) = self.theme_menu_open.as_deref() {
            if let Some(hit) = theme_menu_hit(
                self.theme_menu_rect(rect, axis),
                point,
                axis,
                self.theme_tab_labels().len(),
            ) {
                return Some(hit);
            }
        }
        if let Some(value) = self.variant_menu_open.as_deref() {
            if let Some(hit) = variant_menu_hit(
                self.variant_menu_rect(rect, value),
                point,
                value,
                self.variant_column_labels().len(),
            ) {
                return Some(hit);
            }
        }
        for (idx, axis) in self.theme_tab_labels().iter().enumerate() {
            if (self.theme_tab_rect(rect, idx)).contains(point) {
                if *axis == self.active_axis_label() {
                    return Some(VariablesPanelHit::ToggleThemeMenu((*axis).to_string()));
                }
                return Some(VariablesPanelHit::ThemeTab((*axis).to_string()));
            }
        }
        if self.add_menu_open {
            if let Some(hit) = add_variable_menu_hit(rect, point) {
                return Some(hit);
            }
        }
        if self.preset_menu_open && (self.preset_menu_rect(rect)).contains(point) {
            return Some(VariablesPanelHit::TogglePresetMenu);
        }
        if (self.add_theme_rect(rect)).contains(point) {
            return Some(VariablesPanelHit::AddTheme);
        }
        if (self.preset_rect(rect)).contains(point) {
            return Some(VariablesPanelHit::TogglePresetMenu);
        }
        if (add_variant_rect(rect)).contains(point) {
            return Some(VariablesPanelHit::AddVariant);
        }
        for (idx, value) in self.variant_column_labels().iter().enumerate() {
            if (self.variant_header_rect(rect, idx)).contains(point) {
                return Some(VariablesPanelHit::ToggleVariantMenu((*value).to_string()));
            }
        }
        if (add_variable_rect(rect)).contains(point) {
            return Some(VariablesPanelHit::ToggleAddVariableMenu);
        }
        // Search filter input.
        if self.search_visible() && (self.search_row_rect(rect)).contains(point) {
            return Some(VariablesPanelHit::SearchBox);
        }
        // Dropdown overlay — top-most. Paints when the host
        // marked an axis open AND that axis is one of the
        // active-theme chips.
        if let Some(open_axis) = self.dropdown_open.as_deref() {
            if let Some((chip_idx, _chip)) = self
                .chips
                .iter()
                .enumerate()
                .find(|(_, c)| c.axis == open_axis)
            {
                if let Some(values) = self.axis_values(open_axis) {
                    let chip_rect = self.chip_rect(rect, chip_idx);
                    let menu_y_start = chip_rect.origin.y + chip_rect.size.y + 4.0;
                    let menu_rect = Rect {
                        origin: Point2D::new(chip_rect.origin.x, menu_y_start),
                        size: Point2D::new(
                            DROPDOWN_WIDTH,
                            DROPDOWN_ROW_HEIGHT * (values.len() as f32),
                        ),
                    };
                    if (menu_rect).contains(point) {
                        let row = ((point.y - menu_y_start) / DROPDOWN_ROW_HEIGHT).floor();
                        if row >= 0.0 {
                            let r = row as usize;
                            if r < values.len() {
                                return Some(VariablesPanelHit::AxisDropdownItem {
                                    axis: open_axis.to_string(),
                                    value: values[r].clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
        // Theme value chip in the column header.
        if !self.chips.is_empty() {
            for (i, _chip) in self.chips.iter().enumerate() {
                if (self.chip_rect(rect, i)).contains(point) {
                    return Some(VariablesPanelHit::AxisChip(i));
                }
            }
        }
        // Variable rows — scroll-aware: the cursor maps into row
        // space through the clamped scroll offset, and only points
        // inside the rows viewport count.
        let viewport = self.rows_viewport(rect);
        if !(viewport).contains(point) {
            return None;
        }
        let idx =
            ((point.y - viewport.origin.y + self.effective_scroll(rect)) / ROW_HEIGHT).floor();
        if idx >= 0.0 {
            let i = idx as usize;
            if let Some(row) = self.rows.get(i) {
                let source = row.source_idx;
                if (self.row_menu_button_rect(rect, i)).contains(point) {
                    return Some(VariablesPanelHit::RowMenuToggle(source));
                }
                if (self.name_cell_rect_at(rect, i)).contains(point) {
                    return Some(VariablesPanelHit::NameCell(source));
                }
                let variant_count = self.variant_column_labels().len().max(1);
                for variant in 0..variant_count {
                    let cell = self.value_cell_rect_at(rect, i, variant, variant_count);
                    if (cell).contains(point) {
                        // Color cells split into a swatch (picker)
                        // and a hex text region (inline editing) —
                        // mirrors TS ColorCell's two inputs.
                        if matches!(row.kind, VariableKind::Color) {
                            if (color_swatch_hit_rect(cell)).contains(point) {
                                return Some(VariablesPanelHit::ColorSwatch {
                                    row: source,
                                    variant,
                                });
                            }
                            if (color_hex_rect(cell)).contains(point) {
                                return Some(VariablesPanelHit::ValueCell {
                                    row: source,
                                    variant,
                                });
                            }
                            return Some(VariablesPanelHit::Row(source));
                        }
                        return Some(VariablesPanelHit::ValueCell {
                            row: source,
                            variant,
                        });
                    }
                }
                return Some(VariablesPanelHit::Row(source));
            }
        }
        None
    }
}

fn add_variable_menu_hit(rect: Rect, point: Point2D) -> Option<VariablesPanelHit> {
    let menu = add_variable_menu_rect(rect);
    if !(menu).contains(point) {
        return None;
    }
    let row = ((point.y - menu.origin.y) / ADD_VARIABLE_MENU_ROW_HEIGHT).floor() as usize;
    match row {
        0 => Some(VariablesPanelHit::AddVariableColor),
        1 => Some(VariablesPanelHit::AddVariableNumber),
        2 => Some(VariablesPanelHit::AddVariableString),
        _ => None,
    }
}

fn theme_menu_hit(
    rect: Rect,
    point: Point2D,
    axis: &str,
    theme_count: usize,
) -> Option<VariablesPanelHit> {
    if !(rect).contains(point) {
        return None;
    }
    let row = ((point.y - rect.origin.y) / ADD_VARIABLE_MENU_ROW_HEIGHT).floor() as usize;
    match row {
        0 => Some(VariablesPanelHit::ThemeMenuRename(axis.to_string())),
        1 if theme_count > 1 => Some(VariablesPanelHit::ThemeMenuDelete(axis.to_string())),
        _ => None,
    }
}

fn variant_menu_hit(
    rect: Rect,
    point: Point2D,
    value: &str,
    variant_count: usize,
) -> Option<VariablesPanelHit> {
    if !(rect).contains(point) {
        return None;
    }
    let row = ((point.y - rect.origin.y) / ADD_VARIABLE_MENU_ROW_HEIGHT).floor() as usize;
    match row {
        0 => Some(VariablesPanelHit::VariantMenuRename(value.to_string())),
        1 if variant_count > 1 => Some(VariablesPanelHit::VariantMenuDelete(value.to_string())),
        _ => None,
    }
}

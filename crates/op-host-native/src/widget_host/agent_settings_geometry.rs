//! Keyboard-aware Agent Settings geometry shared by paint and input paths.

use super::WidgetHostNative;
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
use op_editor_ui::Rect;

const FOCUSED_FIELD_GAP: f32 = 12.0;

impl WidgetHostNative {
    fn agent_settings_owns_keyboard(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        let settings_font_picker = ui.font_picker.open
            && matches!(
                ui.font_picker_purpose,
                Some(op_editor_core::FontPickerPurpose::MissingFont {
                    surface: op_editor_core::MissingFontSurface::Settings,
                    ..
                })
            );
        ui.agent_settings_open
            && !ui.collab_join_input_active()
            && (ui.agent_settings.focus.is_some() || settings_font_picker)
    }

    /// Resolve the settings surface against the stable editor viewport, then
    /// cap only its bottom at the software keyboard. Recomputing `rect` from a
    /// shorter viewport would shift a centred tablet surface (and its header /
    /// navigation); clipping its height preserves every top-side origin.
    pub(in crate::widget_host) fn agent_settings_geometry(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (AgentSettingsPanel<'_>, Rect) {
        let panel = AgentSettingsPanel::for_editor_at(&self.editor_state, self.now_ms);
        let mut panel_rect = panel.rect(viewport_w, viewport_h);
        if self.editor_state.editor_ui.touch_chrome()
            && self.agent_settings_owns_keyboard()
            && self.keyboard_occlusion > 0.0
        {
            let visible_bottom = self.keyboard_visible_bottom(viewport_h);
            let panel_bottom = panel_rect.origin.y + panel_rect.size.y;
            panel_rect.size.y = (panel_bottom.min(visible_bottom) - panel_rect.origin.y).max(0.0);
        }
        (panel, panel_rect)
    }

    /// Scroll the active settings interaction into the keyboard-safe content
    /// body. Ordinary fields reveal themselves; an open built-in model menu
    /// uses as much of the safe body as possible and keeps a stable slice while
    /// its checkbox rows are selected. Geometry is document-space; paint and
    /// hit-testing apply the same scroll offset later, so this updates only the
    /// canonical scroll state.
    pub(in crate::widget_host) fn ensure_focused_agent_settings_visible(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if !self.agent_settings_owns_keyboard()
            || !self.editor_state.editor_ui.touch_chrome()
            || self.keyboard_occlusion <= 0.0
            || viewport_w <= 0.0
            || viewport_h <= 0.0
        {
            return false;
        }

        let next_scroll = {
            let (panel, panel_rect) = self.agent_settings_geometry(viewport_w, viewport_h);
            let content = panel.resolved_content_viewport(panel_rect);
            let field = match panel.focused_input_rect(panel_rect) {
                Some(field) if content.size.y > 0.0 => field,
                _ => return false,
            };
            let current = panel.effective_scroll(panel_rect);
            let visible_top = content.origin.y + FOCUSED_FIELD_GAP;
            let visible_bottom = content.origin.y + content.size.y - FOCUSED_FIELD_GAP;
            let desired = if let Some(menu) = panel.focused_model_menu_rect(panel_rect) {
                reveal_menu_scroll(current, menu, visible_top, visible_bottom)
            } else {
                let field_top = field.origin.y - current;
                let field_bottom = field_top + field.size.y;
                if field_bottom > visible_bottom {
                    current + field_bottom - visible_bottom
                } else if field_top < visible_top {
                    current - (visible_top - field_top)
                } else {
                    current
                }
            };
            desired.clamp(0.0, panel.max_scroll(panel_rect))
        };

        let scroll = &mut self.editor_state.editor_ui.agent_settings.scroll_y.offset;
        if (*scroll - next_scroll).abs() <= f32::EPSILON {
            return false;
        }
        *scroll = next_scroll;
        true
    }
}

/// Pick the nearest outer-scroll offset that exposes as much of `menu` as the
/// keyboard-safe viewport can hold. When the menu is shorter than the viewport,
/// the valid interval fully contains it; when it is taller, the interval keeps
/// the viewport fully covered by it. Clamping the current offset into that
/// interval is deliberately stable across checkbox presses, so selecting one
/// model cannot snap the outer surface back to the text field.
fn reveal_menu_scroll(current: f32, menu: Rect, visible_top: f32, visible_bottom: f32) -> f32 {
    let visible_h = (visible_bottom - visible_top).max(0.0);
    if visible_h <= f32::EPSILON {
        return current;
    }
    let menu_bottom = menu.origin.y + menu.size.y;
    let (min_scroll, max_scroll) = if menu.size.y <= visible_h {
        // Fully contain the menu in the visible band.
        (menu_bottom - visible_bottom, menu.origin.y - visible_top)
    } else {
        // The menu cannot fit; cover the whole visible band with the nearest
        // slice of it, maximizing the usable option area.
        (menu.origin.y - visible_top, menu_bottom - visible_bottom)
    };
    current.clamp(min_scroll, max_scroll)
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::agent_settings::{
        BuiltinAgentField, BuiltinAgentKind, BuiltinModelMenuTarget, SettingsFocus,
    };
    use op_editor_core::size_class::EditorSizeClass;
    use op_editor_core::{
        BuiltinModelCatalogRefreshOutcome, BuiltinModelCatalogTarget, BuiltinModelOption,
    };
    use op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit;
    use op_editor_ui::Point2D;

    fn touch_host(size_class: EditorSizeClass) -> WidgetHostNative {
        let mut host = WidgetHostNative::new();
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.touch = true;
        ui.size_class = size_class;
        ui.agent_settings_open = true;
        host
    }

    fn add_agent(host: &mut WidgetHostNative) {
        host.editor_state_mut()
            .editor_ui
            .agent_settings
            .add_builtin_agent_with_defaults("Provider", "sk-test", "model");
    }

    fn add_catalog_agent_with_open_model_menu(host: &mut WidgetHostNative) {
        let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
        let id = settings.add_builtin_agent_config(
            "Provider",
            "sk-test",
            "model-0",
            BuiltinAgentKind::OpenAiCompat,
            "https://api.example.com/v1",
        );
        let request = settings
            .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent(id), 1)
            .expect("configured provider starts discovery");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("current discovery config");
        settings.take_pending_builtin_model_catalog_refresh();
        assert!(
            settings.apply_builtin_model_catalog_refresh_outcome_if_current(
                &expected,
                &request,
                BuiltinModelCatalogRefreshOutcome::Success {
                    models: (0..8)
                        .map(|index| {
                            BuiltinModelOption::new(
                                format!("model-{index}"),
                                format!("Model {index}"),
                            )
                        })
                        .collect(),
                },
            )
        );
        settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::Model,
        });
        settings.builtin_model_menu_open = Some(BuiltinModelMenuTarget::Agent(0));
        host.editor_state_mut()
            .editor_ui
            .settings_input
            .set_text("model-0");
    }

    fn find_edit(host: &WidgetHostNative, viewport_w: f32, viewport_h: f32) -> Point2D {
        let (panel, panel_rect) = host.agent_settings_geometry(viewport_w, viewport_h);
        let content = panel.resolved_content_viewport(panel_rect);
        let mut y = content.origin.y;
        while y < content.origin.y + content.size.y {
            let mut x = content.origin.x;
            while x < content.origin.x + content.size.x {
                if panel.hit_test(panel_rect, Point2D::new(x, y))
                    == AgentSettingsHit::EditBuiltinAgent(0)
                {
                    return Point2D::new(x, y);
                }
                x += 2.0;
            }
            y += 2.0;
        }
        panic!("touch edit target must be reachable");
    }

    #[test]
    fn keyboard_without_settings_owner_keeps_390_and_834_geometry() {
        for (viewport_w, viewport_h, size_class, keyboard_h) in [
            (390.0, 844.0, EditorSizeClass::Compact, 500.0),
            (834.0, 1112.0, EditorSizeClass::Medium, 700.0),
        ] {
            let mut host = touch_host(size_class);
            host.last_viewport_w = viewport_w;
            host.last_viewport_h = viewport_h;
            let expected =
                AgentSettingsPanel::for_editor(host.editor_state()).rect(viewport_w, viewport_h);

            assert!(host.set_keyboard_occlusion(keyboard_h));
            let (_, actual) = host.agent_settings_geometry(viewport_w, viewport_h);

            assert_eq!(actual, expected, "{viewport_w}pt ownerless surface");
        }
    }

    #[test]
    fn keyboard_caps_body_without_moving_390_and_834_chrome() {
        for (viewport_w, viewport_h, size_class, keyboard_h) in [
            (390.0, 844.0, EditorSizeClass::Compact, 500.0),
            (834.0, 1112.0, EditorSizeClass::Medium, 700.0),
        ] {
            let mut host = touch_host(size_class);
            add_agent(&mut host);
            host.editor_state_mut().editor_ui.agent_settings.focus =
                Some(SettingsFocus::BuiltinAgent {
                    index: 0,
                    field: BuiltinAgentField::DisplayName,
                });
            host.last_viewport_w = viewport_w;
            host.last_viewport_h = viewport_h;
            let base_panel = AgentSettingsPanel::for_editor(host.editor_state());
            let base_rect = base_panel.rect(viewport_w, viewport_h);
            let base_layout = base_panel.resolved_layout(base_rect);

            assert!(host.set_keyboard_occlusion(keyboard_h));
            let (panel, capped_rect) = host.agent_settings_geometry(viewport_w, viewport_h);
            let capped_layout = panel.resolved_layout(capped_rect);

            assert_eq!(capped_rect.origin, base_rect.origin);
            assert_eq!(capped_rect.size.x, base_rect.size.x);
            assert_eq!(capped_layout.header.origin, base_layout.header.origin);
            assert_eq!(
                capped_layout.navigation.origin,
                base_layout.navigation.origin
            );
            assert_eq!(
                capped_rect.origin.y + capped_rect.size.y,
                host.keyboard_visible_bottom(viewport_h)
            );
            assert!(capped_layout.content.size.y < base_layout.content.size.y);
        }
    }

    #[test]
    fn focus_press_reveals_field_above_keyboard_at_390_and_834() {
        for (viewport_w, viewport_h, size_class, keyboard_h) in [
            (390.0, 844.0, EditorSizeClass::Compact, 500.0),
            (834.0, 1112.0, EditorSizeClass::Medium, 700.0),
        ] {
            let mut host = touch_host(size_class);
            add_agent(&mut host);
            host.last_viewport_w = viewport_w;
            host.last_viewport_h = viewport_h;
            assert!(host.set_keyboard_occlusion(keyboard_h));
            let edit = find_edit(&host, viewport_w, viewport_h);

            assert!(host.dispatch_agent_settings_press(edit.x, edit.y, viewport_w, viewport_h,));

            let (panel, panel_rect) = host.agent_settings_geometry(viewport_w, viewport_h);
            let content = panel.resolved_content_viewport(panel_rect);
            let field = panel.focused_input_rect(panel_rect).expect("focused field");
            let scroll = panel.effective_scroll(panel_rect);
            let field_bottom = field.origin.y - scroll + field.size.y;
            let visible_bottom = content.origin.y + content.size.y - FOCUSED_FIELD_GAP;
            assert!(
                field_bottom <= visible_bottom + 0.01,
                "{viewport_w}pt focused field must clear keyboard: {field_bottom} > {visible_bottom}"
            );
            assert!(scroll > 0.0, "{viewport_w}pt focus press must reveal");
        }
    }

    #[test]
    fn open_model_menu_maximizes_keyboard_safe_area_and_multi_select_keeps_scroll() {
        for (viewport_w, viewport_h, size_class, keyboard_h) in [
            (390.0, 844.0, EditorSizeClass::Compact, 500.0),
            (834.0, 1112.0, EditorSizeClass::Medium, 700.0),
        ] {
            let mut host = touch_host(size_class);
            add_catalog_agent_with_open_model_menu(&mut host);
            host.last_viewport_w = viewport_w;
            host.last_viewport_h = viewport_h;

            assert!(host.set_keyboard_occlusion(keyboard_h));

            let (point, before_scroll) = {
                let (panel, panel_rect) = host.agent_settings_geometry(viewport_w, viewport_h);
                let content = panel.resolved_content_viewport(panel_rect);
                let menu = panel
                    .focused_model_menu_rect(panel_rect)
                    .expect("focused model menu");
                let scroll = panel.effective_scroll(panel_rect);
                let visible_top = content.origin.y + FOCUSED_FIELD_GAP;
                let visible_bottom = content.origin.y + content.size.y - FOCUSED_FIELD_GAP;
                let menu_top = menu.origin.y - scroll;
                let menu_bottom = menu_top + menu.size.y;
                let overlap =
                    (menu_bottom.min(visible_bottom) - menu_top.max(visible_top)).max(0.0);
                let maximum = menu.size.y.min((visible_bottom - visible_top).max(0.0));
                assert!(
                    (overlap - maximum).abs() <= 0.01,
                    "{viewport_w}pt menu should use the maximum keyboard-safe area: {overlap} != {maximum}"
                );

                // Touch row 1 rather than the already-saved first row. The
                // reveal policy keeps at least the top portion of an oversized
                // menu visible on both phone and tablet keyboard geometries.
                let point = Point2D::new(menu.origin.x + menu.size.x / 2.0, menu_top + 6.0 + 66.0);
                assert_eq!(
                    panel.hit_test(panel_rect, point),
                    AgentSettingsHit::SelectBuiltinModel {
                        index: Some(0),
                        row: 1,
                    },
                    "{viewport_w}pt second touch row must remain hittable"
                );
                (point, scroll)
            };

            assert!(host.dispatch_agent_settings_press(point.x, point.y, viewport_w, viewport_h,));

            let settings = &host.editor_state().editor_ui.agent_settings;
            assert!(settings.builtin_agents[0].has_model("model-1"));
            assert_eq!(
                settings.builtin_model_menu_open,
                Some(BuiltinModelMenuTarget::Agent(0))
            );
            assert!(
                (settings.scroll_y.offset - before_scroll).abs() <= 0.01,
                "{viewport_w}pt multi-select must not snap the outer body back to the field"
            );
        }
    }
}

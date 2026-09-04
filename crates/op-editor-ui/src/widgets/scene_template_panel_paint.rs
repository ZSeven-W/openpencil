//! Scene Template Center chrome painting, split from geometry to keep both
//! compact. The cards themselves live in `scene_template_card_paint.rs`.

use op_editor_core::SceneTemplateFocus;

use super::asset_center_style_layout::STYLE_SECTION_HEADER_H;
use super::panel_controls::{
    paint_accent_button, paint_panel_chip, paint_segmented_control, ButtonSpec, SegmentState,
};
use super::scene_template_card_actions::{
    BASIS_CHIP_LABEL_SIZE, BASIS_CHIP_PAD_X, SCENE_TEMPLATE_BASIS_CHIP_HOVER,
};
use super::scene_template_panel::{
    filter_hover_token, tab_hover_token, SceneTemplatePanel, CHIP_RADIUS, CONTROL_RADIUS,
    GENERATE_HINT_SIZE, GENERATE_INPUT_PAD_X, GENERATE_TEXT_SIZE, SCENE_TEMPLATE_CLOSE_HOVER,
    SCENE_TEMPLATE_GENERATE_HOVER, SEARCH_PAD_X, SEARCH_TEXT_SIZE, TITLE_SIZE,
};
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::prompt_center_panel::estimated_text_width;
use crate::widgets::property_panel_text_input::paint_text_input_view;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, Point2D, Rect, TextLayout};

/// Corner radius of the gallery frame itself. Larger than a dropdown's
/// because the shape is read at canvas scale, not at menu scale.
const PANEL_RADIUS: f32 = 16.0;

/// Section-heading text size — the same size the Styles tab uses, so the two
/// tabs' "mine / built-in" bands read as one visual language.
const SECTION_SIZE: f32 = 11.5;

impl SceneTemplatePanel<'_> {
    /// Paint the complete gallery.
    pub fn paint(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        cx.backend
            .fill_round_rect(panel, PANEL_RADIUS, self.theme.popover);
        cx.backend
            .stroke_round_rect(panel, PANEL_RADIUS, self.theme.border, 1.0);
        self.paint_header(cx, panel);
        self.paint_tab_chips(cx, panel);
        self.paint_search(cx, panel);
        self.paint_filter_chips(cx, panel);
        self.paint_generate_row(cx, panel);
        match self.tab() {
            op_editor_core::AssetCenterTab::Templates => self.paint_cards(cx, panel),
            op_editor_core::AssetCenterTab::Styles => self.paint_style_cards(cx, panel),
        }
        // Last, over everything: the paste box is the panel's topmost layer,
        // matching the press ladder that gives it every click inside the panel.
        self.paint_style_import(cx, panel);
    }

    /// The tab row: a segmented control, not two pills.
    ///
    /// Which asset family the panel is showing is a mode, and a mode with
    /// exactly one answer is what a segmented control is for. Two separate
    /// pills said neither thing — they read as filters that happened to be
    /// mutually exclusive by luck.
    fn paint_tab_chips(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let tabs = op_editor_core::AssetCenterTab::ALL;
        let labels: Vec<&str> = tabs.into_iter().map(|tab| self.tab_label(tab)).collect();
        let states: Vec<SegmentState> = tabs
            .into_iter()
            .enumerate()
            .map(|(index, tab)| SegmentState {
                selected: self.tab() == tab,
                hovered: self.state.editor_ui.scene_template_center.hover
                    == Some(tab_hover_token(index)),
                pressed: self.is_pressed(tab_hover_token(index)),
            })
            .collect();
        paint_segmented_control(
            cx,
            &self.theme,
            self.tab_track_rect(panel),
            &labels,
            &states,
        );
    }

    fn paint_header(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let content = Self::content_rect(panel);
        self.paint_text(
            cx,
            self.t("assetCenter.title", "资产中心"),
            Point2D::new(
                content.origin.x,
                jian_widgets::centered_text_baseline_y(
                    Rect::xywh(
                        content.origin.x,
                        panel.origin.y,
                        content.size.x,
                        self.header_height_for(panel),
                    ),
                    TITLE_SIZE,
                ),
            ),
            TITLE_SIZE,
            self.theme.foreground,
        );

        let close = self.close_rect_for(panel);
        jian_widgets::components::icon_button::IconButton {
            icon_paths: Icon::Close.paths(),
            hovered: self.state.editor_ui.scene_template_center.hover
                == Some(SCENE_TEMPLATE_CLOSE_HOVER),
            pressed: self.is_pressed(SCENE_TEMPLATE_CLOSE_HOVER),
            active: false,
            enabled: true,
            icon_size: close.size.x - 14.0,
            stroke_width: 1.5,
        }
        .paint(
            cx.backend,
            close,
            &crate::widgets::button::tokens_from_theme(&self.theme),
        );

        cx.backend.fill_rect(
            Rect::xywh(
                panel.origin.x,
                panel.origin.y + self.header_height_for(panel),
                panel.size.x,
                1.0,
            ),
            self.theme.border,
        );
    }

    fn paint_search(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let rect = self.search_rect_for(panel);
        self.paint_field_frame(cx, rect, Icon::Search);
        paint_text_input_view(
            cx,
            &self.theme,
            &self.state.editor_ui.scene_template_center.search,
            rect,
            SEARCH_TEXT_SIZE,
            SEARCH_PAD_X,
            jian_widgets::centered_text_baseline_y(rect, SEARCH_TEXT_SIZE),
            self.now_ms(),
            match self.tab() {
                op_editor_core::AssetCenterTab::Templates => {
                    self.t("sceneTemplate.searchPlaceholder", "搜索场景或模板")
                }
                op_editor_core::AssetCenterTab::Styles => {
                    self.t("assetCenter.style.searchPlaceholder", "搜索风格或标签")
                }
            },
            self.field_focused(SceneTemplateFocus::Search),
        );
    }

    /// The box and leading glyph every text field in this panel is drawn in.
    ///
    /// One helper rather than two copies, because the search field and the
    /// topic field are the same control with a different placeholder — and
    /// two copies is how they ended up with the glyph at a hard-coded
    /// `y + 11` that only centred at one field height.
    fn paint_field_frame(&self, cx: &mut PaintCx<'_>, rect: Rect, icon: Icon) {
        cx.backend
            .fill_round_rect(rect, CONTROL_RADIUS, self.theme.muted);
        cx.backend
            .stroke_round_rect(rect, CONTROL_RADIUS, self.theme.input, 1.0);
        const GLYPH: f32 = 16.0;
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(
                rect.origin.x + (SEARCH_PAD_X - GLYPH) / 2.0,
                rect.origin.y + (rect.size.y - GLYPH) / 2.0,
            ),
            GLYPH,
            self.theme.muted_foreground,
            1.4,
        );
    }

    /// The prompt-to-deck row: a topic field, a generate button, and one line
    /// saying what pressing it does. The sentence is not decoration — the row
    /// replaces the document, and a control that quietly discards the canvas
    /// has to say so before it is pressed, not after.
    fn paint_generate_row(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let Some(input) = self.generate_input_rect(panel) else {
            return;
        };
        let button = self
            .generate_button_rect(panel)
            .expect("the button rect exists whenever the input does");

        self.paint_basis_chip(cx, panel);

        self.paint_field_frame(cx, input, Icon::Sparkles);
        paint_text_input_view(
            cx,
            &self.theme,
            &self.state.editor_ui.scene_template_center.generate,
            input,
            GENERATE_TEXT_SIZE,
            GENERATE_INPUT_PAD_X,
            jian_widgets::centered_text_baseline_y(input, GENERATE_TEXT_SIZE),
            self.now_ms(),
            self.t(
                "sceneTemplate.generate.placeholder",
                "描述主题，AI 直接生成整副演示文稿",
            ),
            self.field_focused(SceneTemplateFocus::Generate),
        );

        // Same radius as the field it sits beside, and the same pointer
        // ladder the tab selection rides — a flat blue slab that never
        // changed under the cursor was the one control in the row that did
        // not answer being pointed at.
        paint_accent_button(
            cx,
            &self.theme,
            ButtonSpec {
                rect: button,
                radius: CONTROL_RADIUS,
                label: self.t("sceneTemplate.generate.button", "生成"),
                label_size: GENERATE_TEXT_SIZE,
                hovered: self.state.editor_ui.scene_template_center.hover
                    == Some(SCENE_TEMPLATE_GENERATE_HOVER),
                pressed: self.is_pressed(SCENE_TEMPLATE_GENERATE_HOVER),
            },
        );

        self.paint_text(
            cx,
            // The deck-specific wording belongs to the Templates tab, where
            // the row is gated to the slides scene. On the Styles tab the row
            // means "generate anything, in this aesthetic".
            match self.tab() {
                op_editor_core::AssetCenterTab::Templates => self.t(
                    "sceneTemplate.generate.hint",
                    "新建一个文档，按主题直接生成整副演示文稿。",
                ),
                op_editor_core::AssetCenterTab::Styles => self.t(
                    "assetCenter.style.generateHint",
                    "新建一个文档，按主题生成；已钉住的风格会被直接采用。",
                ),
            },
            Point2D::new(input.origin.x + 2.0, input.origin.y + input.size.y + 18.0),
            GENERATE_HINT_SIZE,
            self.theme.muted_foreground,
        );
    }

    fn paint_filter_chips(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        for (index, (rect, filter)) in self.filter_chip_rects(panel).into_iter().enumerate() {
            paint_panel_chip(
                cx,
                &self.theme,
                rect,
                self.filter_label(filter),
                SegmentState {
                    selected: self.state.editor_ui.scene_template_center.filter == filter,
                    hovered: self.state.editor_ui.scene_template_center.hover
                        == Some(filter_hover_token(index)),
                    pressed: self.is_pressed(filter_hover_token(index)),
                },
            );
        }
    }

    fn paint_cards(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let viewport = self.cards_viewport(panel);
        let user_cards = self.user_cards();
        let templates = self.filtered();
        if user_cards.is_empty() && templates.is_empty() {
            self.paint_text(
                cx,
                self.t("sceneTemplate.empty", "没有匹配的模板"),
                Point2D::new(viewport.origin.x + 4.0, viewport.origin.y + 28.0),
                12.0,
                self.theme.muted_foreground,
            );
            return;
        }

        cx.backend.save();
        cx.backend.clip_rect(viewport);
        let layout = self.template_layout(panel);
        for header in &layout.headers {
            if !Self::row_visible(header.rect, viewport) {
                continue;
            }
            self.paint_text(
                cx,
                if header.is_user {
                    self.t("assetCenter.template.mine", "我的模板")
                } else {
                    self.t("assetCenter.template.builtIn", "内置模板")
                },
                Point2D::new(
                    header.rect.origin.x,
                    header.rect.origin.y + STYLE_SECTION_HEADER_H - 12.0,
                ),
                SECTION_SIZE,
                self.theme.muted_foreground,
            );
        }
        for (index, rect) in layout.cards {
            // Cheap reject for rows scrolled out of view: their rects are
            // still computed so hover and paint agree on indices.
            if !Self::row_visible(rect, viewport) {
                continue;
            }
            if index < user_cards.len() {
                self.paint_user_card(cx, rect, &user_cards[index], index);
            } else {
                self.paint_card(cx, rect, templates[index - user_cards.len()], index);
            }
        }
        cx.backend.restore();
    }

    fn row_visible(rect: Rect, viewport: Rect) -> bool {
        rect.origin.y <= viewport.origin.y + viewport.size.y
            && rect.origin.y + rect.size.y >= viewport.origin.y
    }

    /// "基于：极简 Keynote ×" — the standing answer to "in what style?".
    ///
    /// It reads as a chip rather than as a line of help text because it is
    /// removable, and the × has to look like the thing that removes it: the
    /// pin behind this chip steers every generation until it is cleared.
    fn paint_basis_chip(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let (Some(chip), Some(label)) = (self.basis_chip_rect(panel), self.basis_chip_label())
        else {
            return;
        };
        // Same surface + hairline a resting filter chip gets: it is a chip,
        // and one that painted itself differently would read as a different
        // kind of object standing in the control row.
        let radius = CHIP_RADIUS;
        cx.backend
            .fill_round_rect(chip, radius, self.theme.secondary);
        cx.backend
            .stroke_round_rect(chip, radius, self.theme.border, 1.0);

        let dismiss_w = self
            .basis_chip_dismiss_rect(panel)
            .map(|rect| rect.size.x)
            .unwrap_or(0.0);
        let text_w = (chip.size.x - BASIS_CHIP_PAD_X - dismiss_w).max(0.0);
        self.paint_text(
            cx,
            &truncate_to_width(&label, text_w, BASIS_CHIP_LABEL_SIZE),
            Point2D::new(
                chip.origin.x + BASIS_CHIP_PAD_X,
                jian_widgets::centered_text_baseline_y(chip, BASIS_CHIP_LABEL_SIZE),
            ),
            BASIS_CHIP_LABEL_SIZE,
            self.theme.foreground,
        );

        let Some(dismiss) = self.basis_chip_dismiss_rect(panel) else {
            return;
        };
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            dismiss,
            radius,
            self.state.editor_ui.scene_template_center.hover
                == Some(SCENE_TEMPLATE_BASIS_CHIP_HOVER),
            self.is_pressed(SCENE_TEMPLATE_BASIS_CHIP_HOVER),
        );
        const GLYPH: f32 = 11.0;
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(
                dismiss.origin.x + (dismiss.size.x - GLYPH) / 2.0,
                dismiss.origin.y + (dismiss.size.y - GLYPH) / 2.0,
            ),
            GLYPH,
            self.theme.muted_foreground,
            1.5,
        );
    }

    pub(super) fn t(&self, key: &'static str, fallback: &'static str) -> &'static str {
        let translated = op_i18n::translate(self.locale, key);
        if translated == key {
            fallback
        } else {
            translated
        }
    }

    pub(super) fn paint_text(
        &self,
        cx: &mut PaintCx<'_>,
        text: &str,
        position: Point2D,
        size: f32,
        color: Color,
    ) {
        let layout = TextLayout::single_run(
            text,
            "system-ui",
            size,
            color.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&layout, position);
    }
}

pub(super) fn truncate_to_width(text: &str, max_width: f32, size: f32) -> String {
    if estimated_text_width(text, size) <= max_width {
        return text.to_string();
    }
    let ellipsis_w = estimated_text_width("…", size);
    let mut out = String::new();
    let mut width = 0.0;
    for character in text.chars() {
        let advance = estimated_text_width(&character.to_string(), size);
        if width + advance + ellipsis_w > max_width {
            break;
        }
        out.push(character);
        width += advance;
    }
    out.push('…');
    out
}

//! Shared Images-tab geometry. Touch metrics live here so paint, hit-testing,
//! and scroll height all walk the same boxes while desktop keeps its legacy
//! pixel density.

use super::*;

pub(super) const DESKTOP_TITLE_H: f32 = 36.0;
pub(super) const DESKTOP_ADVANCED_ROW_H: f32 = 24.0;
pub(super) const DESKTOP_SECTION_TITLE_H: f32 = 36.0;
pub(super) const DESKTOP_ROW_H: f32 = 36.0;
pub(super) const DESKTOP_BTN_H: f32 = 28.0;
pub(super) const DESKTOP_PROFILE_ROW_H: f32 = 32.0;
pub(super) const DESKTOP_PROFILE_FIELD_H: f32 = 24.0;
pub(super) const DESKTOP_PROVIDER_OPTION_H: f32 = 24.0;

const SECTION_GAP: f32 = 28.0;
const DESKTOP_SUBTITLE_H: f32 = 22.0;
const DESKTOP_ROW_VGAP: f32 = 10.0;
const DESKTOP_LABEL_W: f32 = 110.0;
const DESKTOP_BODY_GAP: f32 = 14.0;
const DESKTOP_REGISTER_ROW_H: f32 = 36.0;
const DESKTOP_PROFILE_FORM_TOP: f32 = 40.0;
const DESKTOP_PROFILE_TEST_BTN_W: f32 = 56.0;
const DESKTOP_PROFILE_TEST_GAP: f32 = 8.0;

const TOUCH_TITLE_H: f32 = 44.0;
const TOUCH_ADVANCED_ROW_H: f32 = 44.0;
const TOUCH_SECTION_TITLE_H: f32 = 52.0;
const TOUCH_SUBTITLE_H: f32 = 28.0;
const TOUCH_SEARCH_BLOCK_H: f32 = 70.0;
const TOUCH_SEARCH_LABEL_H: f32 = 20.0;
const TOUCH_FIELD_H: f32 = 44.0;
const TOUCH_REGISTER_ROW_H: f32 = 52.0;
const TOUCH_COMPACT_REGISTER_ROW_H: f32 = 104.0;
const TOUCH_PROFILE_ROW_H: f32 = 52.0;
const TOUCH_PROFILE_FORM_TOP: f32 = 64.0;
const TOUCH_PROFILE_FIELD_STEP: f32 = 68.0;
const TOUCH_PROFILE_LABEL_H: f32 = 20.0;
const TOUCH_PROFILE_TEST_ROW_H: f32 = 52.0;
const TOUCH_PROVIDER_OPTION_H: f32 = 44.0;
const TOUCH_BUTTON_W: f32 = 92.0;

const PROFILE_ROW_GAP: f32 = 6.0;
const PROFILE_ROW_INSET_X: f32 = 8.0;
const PROFILE_LIST_TOP_GAP: f32 = 8.0;
const PROFILE_SIDE_PAD: f32 = 12.0;
const DESKTOP_TEST_BTN_W: f32 = 56.0;
const DESKTOP_ADD_BTN_W: f32 = 72.0;
const REGISTER_LINK_W: f32 = 220.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct ImagesDensity {
    pub touch: bool,
    pub compact: bool,
}

impl ImagesDensity {
    pub(super) fn new(content: Rect, touch: bool) -> Self {
        Self {
            touch,
            compact: touch && content.size.x < 480.0,
        }
    }

    pub(super) fn title_h(self) -> f32 {
        if self.touch {
            TOUCH_TITLE_H
        } else {
            DESKTOP_TITLE_H
        }
    }

    pub(super) fn advanced_row_h(self) -> f32 {
        if self.touch {
            TOUCH_ADVANCED_ROW_H
        } else {
            DESKTOP_ADVANCED_ROW_H
        }
    }

    pub(super) fn section_title_h(self) -> f32 {
        if self.touch {
            TOUCH_SECTION_TITLE_H
        } else {
            DESKTOP_SECTION_TITLE_H
        }
    }

    pub(super) fn button_h(self) -> f32 {
        if self.touch {
            TOUCH_FIELD_H
        } else {
            DESKTOP_BTN_H
        }
    }

    pub(super) fn test_button_w(self) -> f32 {
        if self.touch {
            TOUCH_BUTTON_W
        } else {
            DESKTOP_TEST_BTN_W
        }
    }

    pub(super) fn add_button_w(self) -> f32 {
        if self.touch {
            TOUCH_BUTTON_W
        } else {
            DESKTOP_ADD_BTN_W
        }
    }

    pub(super) fn profile_header_h(self) -> f32 {
        if self.touch {
            TOUCH_PROFILE_ROW_H
        } else {
            DESKTOP_PROFILE_ROW_H
        }
    }

    fn subtitle_h(self) -> f32 {
        if self.touch {
            TOUCH_SUBTITLE_H
        } else {
            DESKTOP_SUBTITLE_H
        }
    }

    fn register_row_h(self) -> f32 {
        if self.compact {
            TOUCH_COMPACT_REGISTER_ROW_H
        } else if self.touch {
            TOUCH_REGISTER_ROW_H
        } else {
            DESKTOP_REGISTER_ROW_H
        }
    }
}

fn advanced_body_h(content: Rect, touch: bool) -> f32 {
    let density = ImagesDensity::new(content, touch);
    if touch {
        density.subtitle_h() + TOUCH_SEARCH_BLOCK_H * 2.0 + density.register_row_h()
    } else {
        DESKTOP_SUBTITLE_H
            + DESKTOP_ROW_H
            + DESKTOP_ROW_VGAP
            + DESKTOP_ROW_H
            + DESKTOP_BODY_GAP
            + DESKTOP_REGISTER_ROW_H
    }
}

pub(super) fn image_gen_section_top_for_ui(
    content: Rect,
    settings: &AgentSettings,
    touch: bool,
) -> f32 {
    let density = ImagesDensity::new(content, touch);
    let mut y = content.origin.y + density.title_h() + density.advanced_row_h();
    if settings.images_advanced_open {
        y += advanced_body_h(content, touch);
    }
    y + SECTION_GAP
}

pub(in crate::widgets) fn content_height_for_ui(
    settings: &AgentSettings,
    content_w: f32,
    touch: bool,
) -> f32 {
    let content = Rect::xywh(0.0, 0.0, content_w, 0.0);
    let density = ImagesDensity::new(content, touch);
    let mut h = density.title_h() + density.advanced_row_h();
    if settings.images_advanced_open {
        h += advanced_body_h(content, touch);
    }
    h + SECTION_GAP
        + density.section_title_h()
        + PROFILE_LIST_TOP_GAP
        + profile_list_h_for_ui(settings, touch)
        + 24.0
}

pub(super) fn profile_list_h_for_ui(settings: &AgentSettings, touch: bool) -> f32 {
    if settings.image_gen_profiles.is_empty() {
        if touch {
            100.0
        } else {
            80.0
        }
    } else {
        settings
            .image_gen_profiles
            .iter()
            .enumerate()
            .map(|(index, _)| profile_row_h_for_ui(settings, index, touch))
            .sum::<f32>()
            + settings.image_gen_profiles.len().saturating_sub(1) as f32 * PROFILE_ROW_GAP
    }
}

pub(super) fn profile_row_h_for_ui(settings: &AgentSettings, index: usize, touch: bool) -> f32 {
    let density = ImagesDensity::new(Rect::xywh(0.0, 0.0, 600.0, 0.0), touch);
    if !is_editing_profile(settings, index) {
        return density.profile_header_h();
    }
    if !touch {
        return DESKTOP_PROFILE_ROW_H + 8.0 + 5.0 * DESKTOP_ROW_H;
    }
    let menu_h = if settings.image_gen_provider_menu_open == Some(index) {
        TOUCH_PROVIDER_OPTION_H * ImageGenProvider::ALL.len() as f32
    } else {
        0.0
    };
    TOUCH_PROFILE_FORM_TOP
        + 5.0 * TOUCH_PROFILE_FIELD_STEP
        + TOUCH_PROFILE_TEST_ROW_H
        + menu_h
        + 12.0
}

pub(super) fn advanced_toggle_rect_for_ui(content: Rect, touch: bool) -> Rect {
    let density = ImagesDensity::new(content, touch);
    Rect::xywh(
        content.origin.x,
        content.origin.y + density.title_h(),
        if touch { content.size.x } else { 140.0 },
        density.advanced_row_h(),
    )
}

pub(super) fn search_label_rect_for_ui(content: Rect, index: usize, touch: bool) -> Rect {
    let density = ImagesDensity::new(content, touch);
    let y = content.origin.y + density.title_h() + density.advanced_row_h() + density.subtitle_h();
    if touch {
        Rect::xywh(
            content.origin.x,
            y + index as f32 * TOUCH_SEARCH_BLOCK_H,
            content.size.x,
            TOUCH_SEARCH_LABEL_H,
        )
    } else {
        let row_y = y + if index == 0 {
            0.0
        } else {
            DESKTOP_ROW_H + DESKTOP_ROW_VGAP
        };
        Rect::xywh(content.origin.x, row_y, DESKTOP_LABEL_W, DESKTOP_ROW_H)
    }
}

pub(super) fn search_field_rect_for_ui(content: Rect, index: usize, touch: bool) -> Rect {
    let label = search_label_rect_for_ui(content, index, touch);
    if touch {
        Rect::xywh(
            content.origin.x,
            label.origin.y + TOUCH_SEARCH_LABEL_H,
            content.size.x,
            TOUCH_FIELD_H,
        )
    } else {
        Rect::xywh(
            content.origin.x + DESKTOP_LABEL_W,
            label.origin.y,
            content.size.x - DESKTOP_LABEL_W,
            DESKTOP_ROW_H,
        )
    }
}

fn register_link_y_for_ui(content: Rect, touch: bool) -> f32 {
    let density = ImagesDensity::new(content, touch);
    if touch {
        content.origin.y
            + density.title_h()
            + density.advanced_row_h()
            + density.subtitle_h()
            + TOUCH_SEARCH_BLOCK_H * 2.0
    } else {
        content.origin.y
            + DESKTOP_TITLE_H
            + DESKTOP_ADVANCED_ROW_H
            + DESKTOP_SUBTITLE_H
            + DESKTOP_ROW_H
            + DESKTOP_ROW_VGAP
            + DESKTOP_ROW_H
            + DESKTOP_BODY_GAP
    }
}

pub(super) fn register_link_rect_for_ui(content: Rect, touch: bool) -> Rect {
    let density = ImagesDensity::new(content, touch);
    let reserved = if touch && !density.compact {
        density.test_button_w() + 12.0
    } else {
        0.0
    };
    Rect::xywh(
        content.origin.x,
        register_link_y_for_ui(content, touch),
        REGISTER_LINK_W.min((content.size.x - reserved).max(44.0)),
        if touch {
            TOUCH_FIELD_H
        } else {
            DESKTOP_REGISTER_ROW_H
        },
    )
}

pub(super) fn test_btn_rect_for_ui(content: Rect, settings: &AgentSettings, touch: bool) -> Rect {
    if !settings.images_advanced_open {
        return Rect::xywh(0.0, 0.0, 0.0, 0.0);
    }
    let density = ImagesDensity::new(content, touch);
    let y = if density.compact {
        register_link_y_for_ui(content, touch) + TOUCH_REGISTER_ROW_H
    } else {
        register_link_y_for_ui(content, touch)
            + (density.register_row_h() - density.button_h()) / 2.0
    };
    Rect::xywh(
        if density.compact {
            content.origin.x
        } else {
            content.origin.x + content.size.x - density.test_button_w()
        },
        y,
        if density.compact {
            content.size.x
        } else {
            density.test_button_w()
        },
        density.button_h(),
    )
}

pub(super) fn add_btn_rect_for_ui(content: Rect, settings: &AgentSettings, touch: bool) -> Rect {
    let density = ImagesDensity::new(content, touch);
    let top = image_gen_section_top_for_ui(content, settings, touch);
    Rect::xywh(
        content.origin.x + content.size.x - density.add_button_w(),
        top + (density.section_title_h() - density.button_h()) / 2.0,
        density.add_button_w(),
        density.button_h(),
    )
}

pub(super) fn profile_row_rect_for_ui(
    content: Rect,
    settings: &AgentSettings,
    index: usize,
    touch: bool,
) -> Rect {
    let density = ImagesDensity::new(content, touch);
    let top = image_gen_section_top_for_ui(content, settings, touch)
        + density.section_title_h()
        + PROFILE_LIST_TOP_GAP;
    let y = settings
        .image_gen_profiles
        .iter()
        .enumerate()
        .take(index)
        .fold(top, |acc, (i, _)| {
            acc + profile_row_h_for_ui(settings, i, touch) + PROFILE_ROW_GAP
        });
    Rect::xywh(
        content.origin.x + PROFILE_ROW_INSET_X,
        y,
        (content.size.x - PROFILE_ROW_INSET_X * 2.0).max(0.0),
        profile_row_h_for_ui(settings, index, touch),
    )
}

pub(super) fn profile_active_indicator_rect(row: Rect, touch: bool) -> Rect {
    let header_h = if touch {
        TOUCH_PROFILE_ROW_H
    } else {
        DESKTOP_PROFILE_ROW_H
    };
    let dot = if touch { 18.0 } else { 14.0 };
    Rect::xywh(
        row.origin.x + 8.0,
        row.origin.y + (header_h - dot) / 2.0,
        dot,
        dot,
    )
}

pub(super) fn profile_active_target_rect(row: Rect, touch: bool) -> Rect {
    if touch {
        Rect::xywh(row.origin.x, row.origin.y + 4.0, 44.0, 44.0)
    } else {
        profile_active_indicator_rect(row, false)
    }
}

pub(super) fn profile_remove_rect_for_ui(row: Rect, touch: bool) -> Rect {
    let h = if touch {
        TOUCH_PROFILE_ROW_H
    } else {
        DESKTOP_PROFILE_ROW_H
    };
    let w = if touch { 44.0 } else { 32.0 };
    Rect::xywh(row.origin.x + row.size.x - w, row.origin.y, w, h)
}

pub(super) fn profile_remove_hover_rect_for_ui(row: Rect, touch: bool) -> Rect {
    let target = profile_remove_rect_for_ui(row, touch);
    let inset = if touch { 4.0 } else { 2.0 };
    Rect::xywh(
        target.origin.x + inset,
        target.origin.y + inset,
        target.size.x - inset * 2.0,
        target.size.y - inset * 2.0,
    )
}

pub(super) fn profile_chevron_rect_for_ui(row: Rect, touch: bool) -> Rect {
    let remove_w = if touch { 44.0 } else { 32.0 };
    let size = if touch { 44.0 } else { 24.0 };
    let header_h = if touch {
        TOUCH_PROFILE_ROW_H
    } else {
        DESKTOP_PROFILE_ROW_H
    };
    Rect::xywh(
        row.origin.x + row.size.x - remove_w - size,
        row.origin.y + (header_h - size) / 2.0,
        size,
        size,
    )
}

pub(super) fn profile_header_rect_for_ui(row: Rect, touch: bool) -> Rect {
    Rect::xywh(
        row.origin.x,
        row.origin.y,
        row.size.x,
        if touch {
            TOUCH_PROFILE_ROW_H
        } else {
            DESKTOP_PROFILE_ROW_H
        },
    )
}

fn profile_field_block_y(
    row: Rect,
    settings: &AgentSettings,
    index: usize,
    field_index: usize,
    touch: bool,
) -> f32 {
    if !touch {
        return row.origin.y + DESKTOP_PROFILE_FORM_TOP + field_index as f32 * DESKTOP_ROW_H;
    }
    let mut y =
        row.origin.y + TOUCH_PROFILE_FORM_TOP + field_index as f32 * TOUCH_PROFILE_FIELD_STEP;
    if field_index >= 2 && settings.image_gen_provider_menu_open == Some(index) {
        y += TOUCH_PROVIDER_OPTION_H * ImageGenProvider::ALL.len() as f32;
    }
    if field_index >= 3 {
        y += TOUCH_PROFILE_TEST_ROW_H;
    }
    y
}

pub(super) fn profile_field_label_rect_for_ui(
    row: Rect,
    settings: &AgentSettings,
    index: usize,
    field_index: usize,
    touch: bool,
) -> Rect {
    let y = profile_field_block_y(row, settings, index, field_index, touch);
    if touch {
        Rect::xywh(
            row.origin.x + PROFILE_SIDE_PAD,
            y,
            row.size.x - PROFILE_SIDE_PAD * 2.0,
            TOUCH_PROFILE_LABEL_H,
        )
    } else {
        Rect::xywh(
            row.origin.x + 12.0,
            y,
            DESKTOP_LABEL_W - 12.0,
            DESKTOP_PROFILE_FIELD_H,
        )
    }
}

pub(super) fn profile_field_rect_for_ui(
    row: Rect,
    settings: &AgentSettings,
    index: usize,
    field_index: usize,
    touch: bool,
) -> Rect {
    let label = profile_field_label_rect_for_ui(row, settings, index, field_index, touch);
    if touch {
        Rect::xywh(
            row.origin.x + PROFILE_SIDE_PAD,
            label.origin.y + TOUCH_PROFILE_LABEL_H,
            row.size.x - PROFILE_SIDE_PAD * 2.0,
            TOUCH_FIELD_H,
        )
    } else {
        Rect::xywh(
            row.origin.x + DESKTOP_LABEL_W,
            label.origin.y,
            row.size.x - DESKTOP_LABEL_W - 12.0,
            DESKTOP_PROFILE_FIELD_H,
        )
    }
}

pub(super) fn profile_input_rect_for_ui(
    row: Rect,
    settings: &AgentSettings,
    index: usize,
    field: ImageGenField,
    touch: bool,
) -> Rect {
    let mut input =
        profile_field_rect_for_ui(row, settings, index, profile_field_index(field), touch);
    if !touch && matches!(field, ImageGenField::ApiKey) {
        input.size.x =
            (input.size.x - DESKTOP_PROFILE_TEST_GAP - DESKTOP_PROFILE_TEST_BTN_W).max(48.0);
    }
    input
}

pub(super) fn profile_test_btn_rect_for_ui(
    row: Rect,
    settings: &AgentSettings,
    index: usize,
    touch: bool,
) -> Rect {
    let input = profile_field_rect_for_ui(row, settings, index, 2, touch);
    if touch {
        Rect::xywh(
            row.origin.x + PROFILE_SIDE_PAD,
            input.origin.y + input.size.y + 8.0,
            row.size.x - PROFILE_SIDE_PAD * 2.0,
            TOUCH_FIELD_H,
        )
    } else {
        Rect::xywh(
            input.origin.x + input.size.x - DESKTOP_PROFILE_TEST_BTN_W,
            input.origin.y,
            DESKTOP_PROFILE_TEST_BTN_W,
            DESKTOP_PROFILE_FIELD_H,
        )
    }
}

pub(super) fn profile_provider_rect_for_ui(
    row: Rect,
    settings: &AgentSettings,
    index: usize,
    touch: bool,
) -> Rect {
    profile_field_rect_for_ui(row, settings, index, 1, touch)
}

pub(super) fn profile_provider_option_rect_for_ui(
    row: Rect,
    settings: &AgentSettings,
    index: usize,
    option_index: usize,
    touch: bool,
) -> Rect {
    let provider = profile_provider_rect_for_ui(row, settings, index, touch);
    let option_h = if touch {
        TOUCH_PROVIDER_OPTION_H
    } else {
        DESKTOP_PROVIDER_OPTION_H
    };
    Rect::xywh(
        provider.origin.x,
        provider.origin.y + provider.size.y + option_index as f32 * option_h,
        provider.size.x,
        option_h,
    )
}

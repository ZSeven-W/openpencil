//! Video metadata section for an image-node property panel.
//!
//! The image's own `src` remains the poster and is intentionally read-only in
//! this section. Only the video URL and playback policy are editable.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{NodeSnapshot, PropertyPanelAction};
use crate::widgets::property_panel_inputs::{
    paint_input_with_prefix_focused_state, paint_section_divider, paint_section_label,
    INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_sections::EditContext;
use crate::widgets::{text_metrics, PaintCx};
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::{PropertyFocus, VideoPlaybackField};

const NOTE_HEIGHT: f32 = 28.0;
const TOGGLE_ROW_HEIGHT: f32 = 24.0;
const TOGGLE_ROW_GAP: f32 = 4.0;

/// Height consumed by the Video section, including its divider and trailing
/// walker gap.
pub fn video_section_height() -> f32 {
    SECTION_HEADER_HEIGHT
        + INPUT_HEIGHT
        + 6.0
        + NOTE_HEIGHT
        + 6.0
        + TOGGLE_ROW_HEIGHT * 3.0
        + TOGGLE_ROW_GAP * 2.0
        + 12.0
        + 1.0
        + SECTION_GAP
}

/// Push the URL input and five playback-toggle hit rectangles.
pub fn push_video_input_rects(inputs: &mut Vec<(PropertyFocus, Rect)>, x: f32, y: f32, width: f32) {
    let usable_w = width - PAD_X * 2.0;
    let y = y + SECTION_HEADER_HEIGHT;
    let url = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    inputs.push((PropertyFocus::VideoSrc, url));
}

/// Push the five playback-toggle hit rectangles.
pub fn push_video_action_rects(
    actions: &mut Vec<(PropertyPanelAction, Rect)>,
    x: f32,
    y: f32,
    width: f32,
) {
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let mut y = y + SECTION_HEADER_HEIGHT + INPUT_HEIGHT + 6.0 + NOTE_HEIGHT + 6.0;
    push_toggle(
        actions,
        PropertyPanelAction::ToggleVideoAutoplay,
        x + PAD_X,
        y,
        half_w,
    );
    push_toggle(
        actions,
        PropertyPanelAction::ToggleVideoLoop,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
    );
    y += TOGGLE_ROW_HEIGHT + TOGGLE_ROW_GAP;
    push_toggle(
        actions,
        PropertyPanelAction::ToggleVideoMuted,
        x + PAD_X,
        y,
        half_w,
    );
    push_toggle(
        actions,
        PropertyPanelAction::ToggleVideoHoldLastFrame,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
    );
    y += TOGGLE_ROW_HEIGHT + TOGGLE_ROW_GAP;
    push_toggle(
        actions,
        PropertyPanelAction::ToggleVideoClickToReplay,
        x + PAD_X,
        y,
        usable_w,
    );
}

fn push_toggle(
    actions: &mut Vec<(PropertyPanelAction, Rect)>,
    action: PropertyPanelAction,
    x: f32,
    y: f32,
    width: f32,
) {
    actions.push((
        action,
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(width, TOGGLE_ROW_HEIGHT),
        },
    ));
}

/// Paint the selected image node's video metadata.
#[allow(clippy::too_many_arguments)]
pub fn paint_video_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let Some(video) = snapshot.video.as_ref() else {
        return y;
    };
    let mut y = paint_section_label(
        cx,
        theme,
        op_i18n::translate(locale, "video.title"),
        x,
        y,
        width,
    );
    let url_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(width - PAD_X * 2.0, INPUT_HEIGHT),
    };
    paint_input_with_prefix_focused_state(
        cx,
        theme,
        url_rect,
        op_i18n::translate(locale, "video.url"),
        edit.value_for(PropertyFocus::VideoSrc, &video.src),
        edit.focus == Some(PropertyFocus::VideoSrc),
        edit.caret_at(PropertyFocus::VideoSrc),
        edit.select_all_at(PropertyFocus::VideoSrc),
        edit.input_at(PropertyFocus::VideoSrc),
        edit.now_ms,
    );
    y += INPUT_HEIGHT + 6.0;

    let poster_note = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(width - PAD_X * 2.0, NOTE_HEIGHT),
    };
    cx.backend
        .fill_round_rect(poster_note, INPUT_RADIUS, theme.muted);
    draw_icon(
        cx.backend,
        Icon::ImagePlus,
        Point2D::new(poster_note.origin.x + 7.0, poster_note.origin.y + 6.0),
        16.0,
        theme.muted_foreground,
        1.3,
    );
    let poster = TextLayout::single_run(
        op_i18n::translate(locale, "video.posterThisImage"),
        "system-ui",
        11.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &poster,
        Point2D::new(poster_note.origin.x + 29.0, poster_note.origin.y + 18.0),
    );
    y += NOTE_HEIGHT + 6.0;

    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    paint_toggle(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, TOGGLE_ROW_HEIGHT),
        },
        op_i18n::translate(locale, "video.autoplay"),
        video.autoplay,
    );
    paint_toggle(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, TOGGLE_ROW_HEIGHT),
        },
        op_i18n::translate(locale, "video.loop"),
        video.loop_video,
    );
    y += TOGGLE_ROW_HEIGHT + TOGGLE_ROW_GAP;
    paint_toggle(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, TOGGLE_ROW_HEIGHT),
        },
        op_i18n::translate(locale, "video.muted"),
        video.muted,
    );
    paint_toggle(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, TOGGLE_ROW_HEIGHT),
        },
        op_i18n::translate(locale, "video.holdLastFrame"),
        video.hold_last_frame,
    );
    y += TOGGLE_ROW_HEIGHT + TOGGLE_ROW_GAP;
    paint_toggle(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(usable_w, TOGGLE_ROW_HEIGHT),
        },
        op_i18n::translate(locale, "video.clickToReplay"),
        video.click_to_replay,
    );
    y += TOGGLE_ROW_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + 1.0 + SECTION_GAP
}

fn paint_toggle(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, label: &str, checked: bool) {
    let box_size = 16.0;
    let box_rect = Rect {
        origin: Point2D::new(
            rect.origin.x,
            rect.origin.y + (rect.size.y - box_size) / 2.0,
        ),
        size: Point2D::new(box_size, box_size),
    };
    jian_widgets::components::checkbox::Checkbox {
        checked,
        enabled: true,
    }
    .paint(
        cx.backend,
        box_rect,
        &crate::widgets::button::tokens_from_theme(theme),
    );
    let label = text_metrics::fit_chrome(cx.backend, label, (rect.size.x - 22.0).max(0.0), 11.0);
    let layout = TextLayout::single_run(
        &label,
        "system-ui",
        11.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            rect.origin.x + 22.0,
            rect.origin.y + rect.size.y / 2.0 + 4.0,
        ),
    );
}

/// Map one toggle action to its corresponding command field.
pub fn playback_field(action: &PropertyPanelAction) -> Option<VideoPlaybackField> {
    match action {
        PropertyPanelAction::ToggleVideoAutoplay => Some(VideoPlaybackField::Autoplay),
        PropertyPanelAction::ToggleVideoLoop => Some(VideoPlaybackField::Loop),
        PropertyPanelAction::ToggleVideoMuted => Some(VideoPlaybackField::Muted),
        PropertyPanelAction::ToggleVideoHoldLastFrame => Some(VideoPlaybackField::HoldLastFrame),
        PropertyPanelAction::ToggleVideoClickToReplay => Some(VideoPlaybackField::ClickToReplay),
        _ => None,
    }
}

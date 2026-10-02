//! Separate page-count choice for App screens, shared by all Home compositions.
use super::{copy, HomeSurface, StudioPalette};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::{AppPages, HomeFamily, HomeHit, Locale};

pub(super) fn height(family: HomeFamily) -> f32 {
    if family == HomeFamily::AppUi {
        48.0
    } else {
        0.0
    }
}

pub(super) fn options(family: HomeFamily, row: Rect, locale: Locale) -> [Rect; 2] {
    if family != HomeFamily::AppUi {
        return [Rect::ZERO; 2];
    }
    let labels = labels(locale);
    let widths = labels.map(|s| (copy::estimate_text_w(s, 12.0) + 24.0).max(64.0));
    let available = (row.size.x - 56.0).max(0.0);
    let scale = (available / (widths[0] + widths[1])).min(1.0);
    let mut x = row.origin.x + 52.0;
    widths.map(|width| {
        let rect = Rect::xywh(x, row.origin.y + 2.0, width * scale, 44.0);
        x += rect.size.x;
        rect
    })
}

fn labels(locale: Locale) -> [&'static str; 2] {
    [
        copy::home_str(locale, "home.pages.single"),
        copy::home_str(locale, "home.pages.multiple"),
    ]
}

pub(super) fn paint(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    options: [Rect; 2],
    rise: f32,
    palette: StudioPalette,
) {
    if options[0].size.x == 0.0 {
        return;
    }
    use super::paint::cards::{text, text_weighted};
    let locale = surface.ui.locale;
    let mut rects = options;
    for rect in &mut rects {
        rect.origin.y += rise;
    }
    let group = Rect::xywh(
        rects[0].origin.x,
        rects[0].origin.y,
        rects[0].size.x + rects[1].size.x,
        44.0,
    );
    let caption = Rect::xywh(group.origin.x - 52.0, group.origin.y, 44.0, 44.0);
    let caption_text = crate::util::ellipsize_to_width(
        copy::home_str(locale, "home.pages.label"),
        caption.size.x,
        |s| cx.backend.measure_text_family(s, 12.0, copy::SANS),
    );
    text(
        cx,
        &caption_text,
        Point2D::new(
            caption.origin.x,
            jian_widgets::centered_text_baseline_y(caption, 12.0),
        ),
        12.0,
        palette.sub,
    );
    cx.backend.fill_round_rect(group, 8.0, palette.segment_bg);
    cx.backend
        .stroke_round_rect(group, 8.0, palette.segment_line, 1.0);
    let selected = usize::from(surface.state.task_draft().app_pages == AppPages::Multiple);
    for (index, (rect, label)) in rects.into_iter().zip(labels(locale)).enumerate() {
        let on = index == selected;
        if on || surface.state.hover == Some(HomeHit::Segment(index as u8 + 2)) {
            cx.backend.fill_round_rect(rect, 6.0, palette.blue_soft);
        }
        let weight = if on { 600 } else { 400 };
        let label = crate::util::ellipsize_to_width(label, rect.size.x - 12.0, |s| {
            cx.backend
                .measure_text_family_styled(s, 12.0, copy::SANS, weight, false)
        });
        let width = cx
            .backend
            .measure_text_family_styled(&label, 12.0, copy::SANS, weight, false);
        text_weighted(
            cx,
            &label,
            Point2D::new(
                rect.origin.x + (rect.size.x - width) / 2.0,
                jian_widgets::centered_text_baseline_y(rect, 12.0),
            ),
            12.0,
            if on { palette.blue } else { palette.sub },
            weight,
        );
    }
}

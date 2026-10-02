//! Recent work names get available width and retain distinguishing suffixes.
use super::{HomeLayout, HomeSurface, StudioPalette};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::HomeHit;

pub(super) fn adapt_layout(layout: &mut HomeLayout, count: usize) {
    if layout.recent.size.x <= 0.0 {
        return;
    }
    let count = count.min(5);
    layout.recent_chips = [Rect::ZERO; 5];
    if count == 0 {
        return;
    }
    let left = layout.recent.origin.x + 96.0;
    let available = (layout.new_canvas.origin.x - 12.0 - left).max(0.0);
    let width = ((available - (count - 1) as f32 * 10.0) / count as f32).clamp(0.0, 260.0);
    for (index, chip) in layout.recent_chips.iter_mut().take(count).enumerate() {
        *chip = Rect::xywh(
            left + index as f32 * (width + 10.0),
            layout.recent.origin.y + 9.0,
            width,
            32.0,
        );
    }
}

pub(super) fn display_name(name: &str) -> &str {
    name.strip_suffix(".op")
        .or_else(|| name.strip_suffix(".pen"))
        .unwrap_or(name)
}

pub(super) fn fit_name(name: &str, width: f32, mut measure: impl FnMut(&str) -> f32) -> String {
    let name = display_name(name);
    if measure(name) <= width {
        return name.to_string();
    }
    let chars: Vec<_> = name.chars().collect();
    for keep in (1..chars.len()).rev() {
        let head = keep.div_ceil(2);
        let tail = keep / 2;
        let candidate = format!(
            "{}…{}",
            chars[..head].iter().collect::<String>(),
            chars[chars.len() - tail..].iter().collect::<String>()
        );
        if measure(&candidate) <= width {
            return candidate;
        }
    }
    if measure("…") <= width {
        "…".into()
    } else {
        String::new()
    }
}

pub(super) fn paint_hover_name(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    dy: f32,
    palette: StudioPalette,
) {
    let Some(HomeHit::Recent(index)) = surface.state.hover else {
        return;
    };
    let (Some(name), Some(chip)) = (
        surface.recent_files.get(index),
        layout.recent_chips.get(index),
    ) else {
        return;
    };
    let label = fit_name(name, (chip.size.x - 22.0).max(0.0), |s| {
        cx.backend.measure_text_family(s, 12.0, "system-ui")
    });
    if label == display_name(name) || chip.size.x <= 0.0 {
        return;
    }
    let max_w = layout.recent.size.x.min(560.0) - 24.0;
    let mut lines = Vec::new();
    let mut line = String::new();
    for character in name.chars() {
        let candidate = format!("{line}{character}");
        if !line.is_empty()
            && cx
                .backend
                .measure_text_family(&candidate, 12.0, "system-ui")
                > max_w
        {
            lines.push(std::mem::take(&mut line));
        }
        line.push(character);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    let width = lines
        .iter()
        .map(|s| cx.backend.measure_text_family(s, 12.0, "system-ui"))
        .fold(0.0_f32, f32::max)
        + 20.0;
    let height = lines.len() as f32 * 17.0 + 16.0;
    let x = chip
        .origin
        .x
        .min(layout.recent.origin.x + layout.recent.size.x - width);
    let rect = Rect::xywh(x, chip.origin.y + dy - height - 8.0, width, height);
    cx.backend.fill_round_rect(rect, 8.0, palette.raised);
    cx.backend
        .stroke_round_rect(rect, 8.0, palette.raised_line, 1.0);
    for (line, content) in lines.iter().enumerate() {
        super::paint::cards::text(
            cx,
            content,
            Point2D::new(x + 10.0, rect.origin.y + 21.0 + line as f32 * 17.0),
            12.0,
            palette.ink,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_similar_names_keep_the_distinguishing_version_suffix() {
        let measure = |s: &str| s.chars().count() as f32 * 12.0;
        let a = fit_name("季度新品发布会完整设计稿-版本一.op", 144.0, measure);
        let b = fit_name("季度新品发布会完整设计稿-版本二.op", 144.0, measure);
        assert!(a.contains('…') && b.contains('…'));
        assert!(a.ends_with("版本一") && b.ends_with("版本二"));
        assert_ne!(a, b);
        assert!(measure(&a) <= 144.0 && measure(&b) <= 144.0);
    }
    #[test]
    fn recent_targets_never_overlap_the_new_canvas_action() {
        let mut state = op_editor_core::EditorState::new();
        state.editor_ui.home.visible = true;
        state.editor_ui.recent_files = (0..5)
            .map(|i| op_editor_core::RecentFile {
                path: format!("/tmp/季度发布会完整设计稿-v{i}.op"),
                modified_at: 0,
            })
            .collect();
        for width in [700.0, 1180.0, 1440.0] {
            let surface = HomeSurface::for_editor(&state).unwrap();
            let layout = surface.layout(width, 1000.0);
            for (i, rect) in layout.recent_chips.iter().enumerate() {
                assert!(rect.size.x >= 44.0);
                assert!(rect.origin.x + rect.size.x + 11.9 <= layout.new_canvas.origin.x);
                assert_eq!(
                    surface.hit_test(
                        width,
                        1000.0,
                        Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0)
                    ),
                    Some(HomeHit::Recent(i))
                );
            }
        }
    }
}

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

/// Visible labels stay in path order. Only colliding work names gain the
/// shortest distinguishing folder suffix; paths and click indices stay intact.
pub(super) fn work_labels(paths: &[&str]) -> Vec<String> {
    let parts: Vec<Vec<&str>> = paths
        .iter()
        .map(|p| {
            p.split(['/', '\\'])
                .filter(|part| !part.is_empty())
                .collect()
        })
        .collect();
    let names: Vec<&str> = parts
        .iter()
        .zip(paths)
        .map(|(p, path)| p.last().copied().unwrap_or(path))
        .collect();
    let stems: Vec<&str> = names.iter().map(|n| display_name(n)).collect();
    let parents: Vec<&[&str]> = parts
        .iter()
        .map(|p| &p[..p.len().saturating_sub(1)])
        .collect();
    let suffix = |parent: &[&str], depth: usize| {
        if parent.is_empty() {
            ".".to_string()
        } else {
            parent[parent.len().saturating_sub(depth)..].join("/")
        }
    };
    stems
        .iter()
        .enumerate()
        .map(|(i, stem)| {
            let peers: Vec<usize> = (0..stems.len())
                .filter(|&j| j != i && stems[j] == *stem)
                .collect();
            if peers.is_empty() {
                return (*stem).to_string();
            }
            let depth_limit = peers
                .iter()
                .map(|&j| parents[j].len())
                .chain(std::iter::once(parents[i].len()))
                .max()
                .unwrap_or(1)
                .max(1);
            let mut qualifier = suffix(parents[i], depth_limit);
            for depth in 1..=depth_limit {
                let candidate = suffix(parents[i], depth);
                if peers
                    .iter()
                    .all(|&j| suffix(parents[j], depth) != candidate)
                {
                    qualifier = candidate;
                    break;
                }
            }
            // Different supported formats can share the same stem and folder.
            if peers
                .iter()
                .any(|&j| parents[j] == parents[i] && names[j] != names[i])
            {
                if let Some(extension) = names[i].strip_prefix(stem).filter(|s| !s.is_empty()) {
                    qualifier.push_str(" · ");
                    qualifier.push_str(extension);
                }
            }
            format!("{stem} · {qualifier}")
        })
        .collect()
}

pub(super) fn fit_name(name: &str, width: f32, measure: impl FnMut(&str) -> f32) -> String {
    fit_label(display_name(name), width, measure)
}

pub(super) fn fit_label(name: &str, width: f32, mut measure: impl FnMut(&str) -> f32) -> String {
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
    let label = fit_label(name, (chip.size.x - 22.0).max(0.0), |s| {
        cx.backend.measure_text_family(s, 12.0, "system-ui")
    });
    let path = surface
        .ui
        .recent_files
        .get(index)
        .map(|f| f.path.as_str())
        .unwrap_or(name);
    let basename = path.rsplit(['/', '\\']).next().unwrap_or(path);
    if (label == *name && name == display_name(basename)) || chip.size.x <= 0.0 {
        return;
    }
    let max_w = layout.recent.size.x.min(560.0) - 24.0;
    let mut lines = Vec::new();
    let mut line = String::new();
    for character in path.chars() {
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
    fn same_work_names_from_different_folders_remain_distinguishable() {
        let mut state = op_editor_core::EditorState::new();
        state.editor_ui.home.visible = true;
        state.editor_ui.recent_files = [
            "/工作/咖啡/设计/作品.op",
            "/工作/茶馆/设计/作品.op",
            "/工作/宣传/活动.op",
        ]
        .into_iter()
        .map(|path| op_editor_core::RecentFile {
            path: path.into(),
            modified_at: 0,
        })
        .collect();
        let surface = HomeSurface::for_editor(&state).unwrap();
        assert_ne!(surface.recent_files[0], surface.recent_files[1]);
        assert!(surface.recent_files[0].contains("咖啡/设计"));
        assert!(surface.recent_files[1].contains("茶馆/设计"));
        assert_eq!(surface.recent_files[2], "活动");
        assert_eq!(surface.works_recent[0], surface.recent_files[0]);
        assert_eq!(
            state.editor_ui.recent_files[0].path,
            "/工作/咖啡/设计/作品.op"
        );
    }
    #[test]
    fn long_similar_names_keep_the_distinguishing_version_suffix() {
        let measure = |s: &str| s.chars().count() as f32 * 12.0;
        let a = fit_label(
            display_name("季度新品发布会完整设计稿-版本一.op"),
            144.0,
            measure,
        );
        let b = fit_label(
            display_name("季度新品发布会完整设计稿-版本二.op"),
            144.0,
            measure,
        );
        assert!(a.contains('…') && b.contains('…'));
        assert!(a.ends_with("版本一") && b.ends_with("版本二"));
        assert_ne!(a, b);
        assert!(measure(&a) <= 144.0 && measure(&b) <= 144.0);
    }

    #[test]
    fn paths_and_formats_keep_a_minimal_real_disambiguator() {
        assert_eq!(
            work_labels(&[r"C:\咖啡\设计\作品.op", r"C:\茶馆\设计\作品.op"]),
            vec!["作品 · 咖啡/设计", "作品 · 茶馆/设计"]
        );
        assert_eq!(
            work_labels(&["/咖啡/作品.op", "/茶馆/作品.op"]),
            vec!["作品 · 咖啡", "作品 · 茶馆"]
        );
        assert_eq!(
            work_labels(&["/设计/作品.op", "/设计/作品.pen"]),
            vec!["作品 · 设计 · .op", "作品 · 设计 · .pen"]
        );
        let labels = work_labels(&["/原稿.op/作品.op", "/改稿.op/作品.op"]);
        assert_eq!(
            fit_label(&labels[0], 500.0, |s| s.chars().count() as f32 * 8.0),
            "作品 · 原稿.op"
        );
        assert_eq!(
            work_labels(&["/项目/作品.op", "/更深/项目/作品.op"]),
            vec!["作品 · 项目", "作品 · 更深/项目"]
        );
    }

    #[test]
    fn disambiguated_labels_keep_home_and_phone_targets_in_original_order() {
        let mut state = op_editor_core::EditorState::new();
        state.editor_ui.home.visible = true;
        state.editor_ui.recent_files = ["/原稿/作品.op", "/改稿/作品.op"]
            .into_iter()
            .map(|path| op_editor_core::RecentFile {
                path: path.into(),
                modified_at: 0,
            })
            .collect();
        let surface = HomeSurface::for_editor(&state).unwrap();
        let layout = surface.layout(1440.0, 1080.0);
        for (index, rect) in layout.recent_chips.iter().take(2).enumerate() {
            assert_eq!(
                surface.hit_test(
                    1440.0,
                    1080.0,
                    Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0)
                ),
                Some(HomeHit::Recent(index))
            );
        }
        let phone = surface.works_layout(390.0, 844.0);
        for (index, rect) in phone.rows.iter().take(2).enumerate() {
            assert_eq!(
                surface.works_hit(
                    &phone,
                    Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0)
                ),
                Some(HomeHit::WorksRecent(index))
            );
        }
        assert_eq!(state.editor_ui.recent_files[1].path, "/改稿/作品.op");
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

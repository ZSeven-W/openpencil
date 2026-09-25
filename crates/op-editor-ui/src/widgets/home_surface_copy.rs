//! i18n copy adapter for the Studio Home surface: task metadata resolved
//! through the `home.task.*` keys, the per-task segmented-control spec,
//! and the tab icon mapping. One module so paint, hit-test, and the host
//! read the same copy.

use op_editor_core::{HomeDevice, HomeFamily, InfoKind, Locale, SlideRatio, TaskDraft};
use op_i18n::translate;

pub(crate) const SANS: &str = "system-ui";

pub(crate) fn home_str(locale: Locale, key: &'static str) -> &'static str {
    translate(locale, key)
}

/// The per-task resolved copy block (device / info-kind aware).
#[derive(Debug, Clone, Copy)]
pub(crate) struct TaskCopy {
    pub name: &'static str,
    pub summary: &'static str,
    pub label: &'static str,
    pub placeholder: &'static str,
    pub example: &'static str,
    pub example_title: &'static str,
    pub example_desc: &'static str,
    pub example_pages: &'static str,
}

/// `translate` matches on static key literals, so the variant overrides
/// resolve through one static match instead of a formatted string.
pub(crate) fn task_copy(locale: Locale, family: HomeFamily, draft: &TaskDraft) -> TaskCopy {
    let keys: [&'static str; 8] = match family {
        HomeFamily::AppUi => [
            "home.task.app.name",
            "home.task.app.summary",
            "home.task.app.label",
            if draft.device == HomeDevice::Desktop {
                "home.task.app.desktopPlaceholder"
            } else {
                "home.task.app.placeholder"
            },
            if draft.device == HomeDevice::Desktop {
                "home.task.app.desktopExample"
            } else {
                "home.task.app.example"
            },
            if draft.device == HomeDevice::Desktop {
                "home.task.app.desktopTitle"
            } else {
                "home.task.app.exampleTitle"
            },
            if draft.device == HomeDevice::Desktop {
                "home.task.app.desktopDesc"
            } else {
                "home.task.app.exampleDesc"
            },
            if draft.device == HomeDevice::Desktop {
                "home.task.app.desktopPages"
            } else {
                "home.task.app.examplePages"
            },
        ],
        HomeFamily::Web => [
            "home.task.web.name",
            "home.task.web.summary",
            "home.task.web.label",
            "home.task.web.placeholder",
            "home.task.web.example",
            "home.task.web.exampleTitle",
            "home.task.web.exampleDesc",
            "home.task.web.examplePages",
        ],
        HomeFamily::Presentation => [
            "home.task.presentation.name",
            "home.task.presentation.summary",
            "home.task.presentation.label",
            "home.task.presentation.placeholder",
            "home.task.presentation.example",
            "home.task.presentation.exampleTitle",
            "home.task.presentation.exampleDesc",
            "home.task.presentation.examplePages",
        ],
        HomeFamily::KnowledgeCards => [
            "home.task.knowledge.name",
            "home.task.knowledge.summary",
            "home.task.knowledge.label",
            "home.task.knowledge.placeholder",
            "home.task.knowledge.example",
            "home.task.knowledge.exampleTitle",
            "home.task.knowledge.exampleDesc",
            "home.task.knowledge.examplePages",
        ],
        HomeFamily::ScreenshotTutorial => [
            "home.task.tutorial.name",
            "home.task.tutorial.summary",
            "home.task.tutorial.label",
            "home.task.tutorial.placeholder",
            "home.task.tutorial.example",
            "home.task.tutorial.exampleTitle",
            "home.task.tutorial.exampleDesc",
            "home.task.tutorial.examplePages",
        ],
        HomeFamily::Infographic => [
            "home.task.infographic.name",
            "home.task.infographic.summary",
            "home.task.infographic.label",
            match draft.info_kind {
                InfoKind::Data => "home.task.infographic.placeholder",
                InfoKind::Flow => "home.task.infographic.flowPlaceholder",
                InfoKind::Comparison => "home.task.infographic.comparisonPlaceholder",
            },
            match draft.info_kind {
                InfoKind::Data => "home.task.infographic.example",
                InfoKind::Flow => "home.task.infographic.flowExample",
                InfoKind::Comparison => "home.task.infographic.comparisonExample",
            },
            match draft.info_kind {
                InfoKind::Data => "home.task.infographic.exampleTitle",
                InfoKind::Flow => "home.task.infographic.flowTitle",
                InfoKind::Comparison => "home.task.infographic.comparisonTitle",
            },
            match draft.info_kind {
                InfoKind::Data => "home.task.infographic.exampleDesc",
                InfoKind::Flow => "home.task.infographic.flowDesc",
                InfoKind::Comparison => "home.task.infographic.comparisonDesc",
            },
            match draft.info_kind {
                InfoKind::Data => "home.task.infographic.examplePages",
                InfoKind::Flow => "home.task.infographic.flowPages",
                InfoKind::Comparison => "home.task.infographic.comparisonPages",
            },
        ],
        HomeFamily::EventPoster => [
            "home.task.poster.name",
            "home.task.poster.summary",
            "home.task.poster.label",
            "home.task.poster.placeholder",
            "home.task.poster.example",
            "home.task.poster.exampleTitle",
            "home.task.poster.exampleDesc",
            "home.task.poster.examplePages",
        ],
    };
    TaskCopy {
        name: translate(locale, keys[0]),
        summary: translate(locale, keys[1]),
        label: translate(locale, keys[2]),
        placeholder: translate(locale, keys[3]),
        example: translate(locale, keys[4]),
        example_title: translate(locale, keys[5]),
        example_desc: translate(locale, keys[6]),
        example_pages: translate(locale, keys[7]),
    }
}

/// One segmented control's option labels (empty = the task has none).
pub(crate) fn segment_labels(locale: Locale, family: HomeFamily) -> Vec<&'static str> {
    match family {
        HomeFamily::AppUi => vec![
            translate(locale, "home.segment.mobile"),
            translate(locale, "home.segment.desktop"),
        ],
        HomeFamily::Presentation => vec![
            translate(locale, "home.segment.wide"),
            translate(locale, "home.segment.classic"),
        ],
        HomeFamily::Infographic => vec![
            translate(locale, "home.segment.infoData"),
            translate(locale, "home.segment.infoFlow"),
            translate(locale, "home.segment.infoCompare"),
        ],
        _ => Vec::new(),
    }
}

/// Index of the currently selected segmented option for `family`'s draft.
pub(crate) fn segment_index(family: HomeFamily, draft: &TaskDraft) -> u8 {
    match family {
        HomeFamily::AppUi => match draft.device {
            HomeDevice::Mobile => 0,
            HomeDevice::Desktop => 1,
        },
        HomeFamily::Presentation => match draft.ratio {
            SlideRatio::Wide169 => 0,
            SlideRatio::Classic43 => 1,
        },
        HomeFamily::Infographic => match draft.info_kind {
            InfoKind::Data => 0,
            InfoKind::Flow => 1,
            InfoKind::Comparison => 2,
        },
        _ => 0,
    }
}

/// The lucide glyph each task tab paints.
pub(crate) fn task_icon(family: HomeFamily) -> crate::widgets::icons::Icon {
    use crate::widgets::icons::Icon;
    match family {
        HomeFamily::AppUi => Icon::Smartphone,
        HomeFamily::Web => Icon::Globe,
        HomeFamily::Presentation => Icon::PresentationScreen,
        HomeFamily::KnowledgeCards => Icon::LayoutGrid,
        HomeFamily::ScreenshotTutorial => Icon::Image,
        HomeFamily::Infographic => Icon::BarChart2,
        HomeFamily::EventPoster => Icon::ImagePlus,
    }
}

/// The Send button's label for what the press will actually do — the
/// wide and compact composers both read it so the two faces of one button
/// cannot drift.
/// `importing` is a website import already running (the button then
/// says so instead of offering a second one).
pub(crate) fn send_label_key(mode: op_editor_core::HomeSendMode, importing: bool) -> &'static str {
    match mode {
        op_editor_core::HomeSendMode::Start => "home.submit.start",
        op_editor_core::HomeSendMode::UseExample => "home.submit.example",
        op_editor_core::HomeSendMode::Connect => "home.submit.connect",
        op_editor_core::HomeSendMode::ImportSite if importing => "home.siteImport.running",
        op_editor_core::HomeSendMode::ImportSite => "home.siteImport.action",
    }
}

/// The largest label size (from `base` down to 11 px) whose measured width
/// fits `max_w`, so a long translation of the Send label shrinks inside the
/// fixed button instead of spilling past its rounded edge.
pub(crate) fn fit_label_size(
    backend: &mut dyn crate::RenderBackend,
    label: &str,
    base: f32,
    max_w: f32,
) -> f32 {
    let mut size = base;
    while size > 11.0 && backend.measure_text_family(label, size, SANS) > max_w {
        size -= 0.5;
    }
    size
}

/// Backend-free width estimate for the studio labels: a full-width glyph
/// (CJK, kana, hangul, fullwidth forms) is its point size, every other
/// letter ≈ 0.55 em — Cyrillic, Greek and accented Latin included, which
/// counting as full width spread the Russian tool row off the composer.
/// Sizes rects in the layout; paint still measures for real.
pub(crate) fn estimate_text_w(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|character| {
            if is_full_width(character) {
                size
            } else {
                size * 0.55
            }
        })
        .sum()
}

fn is_full_width(character: char) -> bool {
    matches!(
        u32::from(character),
        0x1100..=0x115F     // Hangul Jamo
            | 0x2E80..=0x303F // CJK radicals, punctuation
            | 0x3040..=0x33FF // kana, CJK compatibility
            | 0x3400..=0x4DBF // CJK extension A
            | 0x4E00..=0x9FFF // CJK unified ideographs
            | 0xAC00..=0xD7AF // Hangul syllables
            | 0xF900..=0xFAFF // CJK compatibility ideographs
            | 0xFE30..=0xFE4F // CJK compatibility forms
            | 0xFF00..=0xFF60 // fullwidth forms
            | 0xFFE0..=0xFFE6
    )
}

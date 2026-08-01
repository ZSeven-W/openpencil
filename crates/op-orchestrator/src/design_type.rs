//! 设计类型分类 —— port of
//! `apps/web/src/services/ai/design-type-presets.ts`。
//!
//! `detect_design_type` 首个命中胜出:Component → Mobile →
//! Desktop → 默认 LandingPage。每个类型带一个固定 preset。

/// 四种设计类型(TS `DesignType` union)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignType {
    MobileScreen,
    DesktopScreen,
    LandingPage,
    Component,
    /// Presentation deck — 16:9 slides rather than a scrolling page.
    Slides,
}

/// 一个设计类型的尺寸 preset(TS `DesignTypePreset`)。
/// `height` = 区块总高(0 = 按区块数自适应);`root_height` =
/// 根 frame 显式高(0 = 自适应)。preset 上无 layout/gap/fill。
#[derive(Debug, Clone, Copy)]
pub struct DesignTypePreset {
    pub type_: DesignType,
    pub width: f64,
    pub height: f64,
    pub root_height: f64,
    pub default_sections: &'static [&'static str],
}

const COMPONENT: DesignTypePreset = DesignTypePreset {
    type_: DesignType::Component,
    width: 400.0,
    height: 0.0,
    root_height: 0.0,
    default_sections: &["Component"],
};
const MOBILE: DesignTypePreset = DesignTypePreset {
    type_: DesignType::MobileScreen,
    width: 375.0,
    height: 812.0,
    root_height: 812.0,
    default_sections: &["Top Summary", "Main Content"],
};
const DESKTOP: DesignTypePreset = DesignTypePreset {
    type_: DesignType::DesktopScreen,
    width: 1200.0,
    height: 800.0,
    root_height: 800.0,
    default_sections: &["Header", "Main Content", "Actions"],
};
/// A deck's artboard is the projector, not a viewport: 1920x1080 fixed, never
/// content-sized. `skills/domains/slides.md` states the same contract to the
/// model ("each slide is a 16:9 frame, 1920x1080"), and without this preset the
/// planner handed it a 1200-wide landing page to build that contract inside —
/// the guidance and the skeleton disagreed, and the skeleton wins.
const SLIDES: DesignTypePreset = DesignTypePreset {
    type_: DesignType::Slides,
    width: 1920.0,
    height: 1080.0,
    root_height: 1080.0,
    default_sections: &["Title", "Body"],
};
const LANDING: DesignTypePreset = DesignTypePreset {
    type_: DesignType::LandingPage,
    width: 1200.0,
    height: 0.0,
    root_height: 0.0,
    default_sections: &["Header", "Main Content", "Supporting Content", "Footer"],
};

/// 单组件触发词(Latin)—— 对齐 TS `COMPONENT_TRIGGER_LATIN_RE`。
const COMPONENT_TRIGGER_LATIN: &[&str] = &[
    "card", "badge", "chip", "tag", "tile", "pill", "label", "row", "item", "button", "toggle",
    "switch", "selector", "modal", "dialog", "tooltip", "popover", "sheet", "widget", "panel",
    "avatar", "stepper", "stat", "metric", "chart",
];
/// 单组件触发词(CJK)—— `COMPONENT_TRIGGER_CJK_RE`。
const COMPONENT_TRIGGER_CJK: &[&str] = &[
    "卡片",
    "徽章",
    "标签",
    "按钮",
    "开关",
    "对话框",
    "提示",
    "气泡",
    "图表",
];
/// 取消单组件资格的词 —— `COMPONENT_DISQUALIFIER_RE`。
const COMPONENT_DISQUALIFIER: &[&str] = &[
    "slide",
    "slides",
    "deck",
    "presentation",
    "keynote",
    "ppt",
    "幻灯片",
    "演示",
    "路演",
    "screen",
    "page",
    "app",
    "home",
    "onboarding",
    "flow",
    "mobile",
    "phone",
    "ios",
    "android",
    "dashboard",
    "admin",
    "workspace",
    "console",
    "网页",
    "页面",
    "屏幕",
    "手机",
    "移动端",
    "管理",
    "后台",
    "控制台",
    "工作台",
    "工作区",
];
/// 移动端触发词 —— 命中 → MobileScreen。
/// 演示文稿触发词 —— 与 `skills/domains/slides.md` 的 trigger keywords 同源,
/// 两处必须一起改:语料决定模型拿到什么规则,这里决定它在多大的画布上用。
const SLIDES_WORDS: &[&str] = &[
    "slide",
    "slides",
    "deck",
    "presentation",
    "keynote",
    "ppt",
    "幻灯片",
    "演示文稿",
    "演示稿",
    "路演",
];
const MOBILE_WORDS: &[&str] = &["mobile", "手机", "phone", "移动端", "ios", "android"];
/// 数据型工作区 / dashboard 触发词 —— 命中 → DesktopScreen。
const DASHBOARD_WORDS: &[&str] = &[
    "dashboard",
    "admin",
    "workspace",
    "console",
    "管理",
    "后台",
    "控制台",
    "工作台",
    "工作区",
];

/// `needle` 在 `haystack` 中出现 —— ASCII needle 按 JS `\b` 词边界
/// 匹配(避免 "app" 误命中 "happen");含非 ASCII 的 needle(CJK)
/// 按子串(TS 的 CJK 段本就无 `\b`)。`pub(crate)` —— Plan B 的
/// `infer_tags_from_prompt` 复用同一 helper。
pub(crate) fn contains_word(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    if !needle.is_ascii() {
        return haystack.contains(needle);
    }
    let bytes = haystack.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(needle) {
        let start = from + rel;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_word(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_word(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// `haystack`(已小写)是否含 `needles` 任一(按 `contains_word`)。
fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| contains_word(haystack, n))
}

/// 按 prompt 的意图分类 → preset。首个命中胜出。
pub fn detect_design_type(prompt: &str) -> DesignTypePreset {
    let lower = prompt.to_lowercase();
    // ① 单组件:触发词命中 且 disqualifier 不命中。
    let trigger = contains_any(&lower, COMPONENT_TRIGGER_LATIN)
        || contains_any(&lower, COMPONENT_TRIGGER_CJK);
    if trigger && !contains_any(&lower, COMPONENT_DISQUALIFIER) {
        return COMPONENT;
    }
    // ② 演示文稿。放在移动端之前:"手机端演示" 说的是内容形态是 deck,
    //    而 deck 的画幅是投影比例,不是手机屏。
    if contains_any(&lower, SLIDES_WORDS) {
        return SLIDES;
    }
    // ③ 移动端。
    if contains_any(&lower, MOBILE_WORDS) {
        return MOBILE;
    }
    // ④ 数据型工作区 / dashboard。
    if contains_any(&lower, DASHBOARD_WORDS) {
        return DESKTOP;
    }
    // ⑤ 默认:多区块落地页。
    LANDING
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_requests_get_the_projector_artboard() {
        for prompt in [
            "做一个季度汇报 PPT",
            "设计一套融资路演幻灯片",
            "a pitch deck for our seed round",
            "design a keynote presentation about onboarding",
            "5-slide deck",
        ] {
            let preset = detect_design_type(prompt);
            assert_eq!(preset.type_, DesignType::Slides, "{prompt}");
            assert_eq!((preset.width, preset.height), (1920.0, 1080.0), "{prompt}");
        }
    }

    #[test]
    fn a_deck_beats_the_single_component_reading() {
        // "the cover card of my deck" is a deck; the component trigger `card`
        // must not win, which is why the deck words are disqualifiers too.
        assert_eq!(detect_design_type("PPT 封面卡片").type_, DesignType::Slides);
        assert_eq!(
            detect_design_type("the title card for my pitch deck").type_,
            DesignType::Slides
        );
        // A plain component request is untouched.
        assert_eq!(
            detect_design_type("a profile card").type_,
            DesignType::Component
        );
    }

    #[test]
    fn a_deck_beats_the_mobile_reading() {
        // The content form decides the artboard: a deck shown on a phone is
        // still 16:9, not 375x812.
        assert_eq!(
            detect_design_type("手机上看的演示文稿").type_,
            DesignType::Slides
        );
        assert_eq!(
            detect_design_type("a mobile login screen").type_,
            DesignType::MobileScreen
        );
    }

    #[test]
    fn component_detected_and_not_disqualified() {
        // "profile card" → 触发词 card,无 disqualifier → Component
        assert_eq!(
            detect_design_type("a profile card").type_,
            DesignType::Component
        );
        assert_eq!(
            detect_design_type("一个统计徽章").type_,
            DesignType::Component
        );
    }

    #[test]
    fn component_disqualified_by_screen_word() {
        // "card" 触发,但 "screen" 是 disqualifier → 落到后续分类
        let p = detect_design_type("a card on the home screen");
        assert_eq!(p.type_, DesignType::LandingPage);
    }

    #[test]
    fn mobile_detected() {
        let p = detect_design_type("a mobile login screen");
        assert_eq!(p.type_, DesignType::MobileScreen);
        assert_eq!(p.width, 375.0);
        assert_eq!(p.height, 812.0);
    }

    #[test]
    fn dashboard_detected() {
        let p = detect_design_type("an analytics dashboard");
        assert_eq!(p.type_, DesignType::DesktopScreen);
        assert_eq!(p.width, 1200.0);
    }

    #[test]
    fn default_is_landing_page() {
        let p = detect_design_type("a coffee brand site");
        assert_eq!(p.type_, DesignType::LandingPage);
        assert_eq!(p.width, 1200.0);
        assert_eq!(p.root_height, 0.0);
    }
}

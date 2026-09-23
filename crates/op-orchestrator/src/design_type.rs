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
    /// Social card series — a fixed portrait board (小红书 / 公众号 图文),
    /// delivered as a set of independent images rather than one scrolling
    /// page or one component.
    Card,
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
/// The card system's primary spec (`card-system-0808.md` §5): XHS 竖版 3:4.
/// The other three specs (1:1, 公众号封面对, 9:16) are per-request overrides,
/// not separate presets — `requested_root_dimensions` already honours an
/// explicit size, so the preset only has to carry the default.
const CARD: DesignTypePreset = DesignTypePreset {
    type_: DesignType::Card,
    width: 1080.0,
    height: 1440.0,
    root_height: 1440.0,
    default_sections: &["Cover", "Body"],
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

/// `375×812` / `390x844` / `360 x 800` written into the brief is a phone
/// frame even when the words around it are "App 首页" or "screen": a portrait
/// board 320-480 wide is never a desktop or landing canvas.
fn mentions_phone_dimensions(lower: &str) -> bool {
    let chars: Vec<char> = lower.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let width: u32 = chars[start..i]
            .iter()
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        let mut j = i;
        while j < chars.len() && chars[j] == ' ' {
            j += 1;
        }
        if j < chars.len() && (chars[j] == 'x' || chars[j] == '×' || chars[j] == '*') {
            j += 1;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            let hstart = j;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            let height: u32 = chars[hstart..j]
                .iter()
                .collect::<String>()
                .parse()
                .unwrap_or(0);
            if (320..=480).contains(&width) && height > width {
                return true;
            }
        }
    }
    false
}
/// Words that mean "social card series" on their own — a platform name or a
/// delivery format, neither of which describes anything else we generate.
/// (motion50 fix 2: 轮播/carousel used to live here but is an ordinary App /
/// web component — it moved to `CARD_NOUNS` so it only reads as a card set
/// when a series word backs it up.)
const CARD_PLATFORM_WORDS: &[&str] = &[
    "小红书",
    "小紅書",
    "xiaohongshu",
    "xhs",
    "rednote",
    "公众号",
    "公眾號",
    "图文",
    "圖文",
];
/// `卡片` / `card` is ALSO the component rung's trigger word, so on its own it
/// stays ambiguous. Paired with a series word it is unambiguously a set.
/// 轮播/輪播/carousel sit here since motion50 fix 2: a carousel is a component
/// (`CARD_PLATFORM_WORDS` alone used to classify any brief mentioning one as a
/// card series — measured on app-10: an explicit 375×812 App brief rendered as
/// two 1080×1440 boards).
const CARD_NOUNS: &[&str] = &["卡片", "卡", "card", "cards", "轮播", "輪播", "carousel"];
const CARD_SERIES_WORDS: &[&str] = &["系列", "一套", "多张", "多張", "组图", "組圖", "series"];
/// A card noun inside a COMPONENT request is still a component — "卡片组件"
/// asks for one card, not a set of them. Mirrors how the deck rung uses
/// `COMPONENT_DISQUALIFIER` in the opposite direction.
const CARD_DISQUALIFIER: &[&str] = &["组件", "組件", "component"];

/// Is this a social-card-series request? See the word tables above for why
/// the platform words stand alone while the card nouns need a series word.
pub(crate) fn is_card_series_prompt(lower: &str) -> bool {
    is_card_series(lower)
}

/// An explicit screen declaration in the brief (motion50 fix 2): mobile
/// words, a written phone frame size, dashboard words, or slides words.
/// When the brief names its surface, the card platform words must not
/// single-handedly reclassify it as a Card board.
/// A landing-page / website brief declares its canvas as clearly as
/// "375×812" does. web-10 ("…内饰图文两段…") and web-13 ("…服务三卡…")
/// in the 0918 motion50 rerun were still routed to Card / Component
/// because "官网（1440 宽长页）" counted for nothing here.
const LANDING_WORDS: &[&str] = &[
    "官网",
    "落地页",
    "长页",
    "网站",
    "站点",
    "landing",
    "website",
    "homepage",
];

fn declares_screen_type(lower: &str) -> bool {
    contains_any(lower, LANDING_WORDS)
        || contains_any(lower, MOBILE_WORDS)
        || mentions_phone_dimensions(lower)
        || contains_any(lower, DASHBOARD_WORDS)
        || contains_any(lower, SLIDES_WORDS)
}

fn is_card_series(lower: &str) -> bool {
    if contains_any(lower, CARD_DISQUALIFIER) {
        return false;
    }
    // The strong card-noun + series-word combo always reads as a card set.
    let strong = contains_any(lower, CARD_NOUNS) && contains_any(lower, CARD_SERIES_WORDS);
    if contains_any(lower, CARD_PLATFORM_WORDS) {
        // Platform words alone are downsized (motion50 fix 2): an explicit
        // screen declaration (mobile words / phone dimensions / dashboard /
        // slides) wins — "电商 App…商品页(大图轮播占位)" is a phone screen
        // whose section happens to be a carousel, not a card series.
        return strong || !declares_screen_type(lower);
    }
    strong
}
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
    // ① 社交卡片系列。必须排在单组件之前:`卡片` 本身就是 COMPONENT 的触发
    //    词,「做一套小红书卡片」会被它截胡成 400px 宽的单组件(card-system
    //    -0808.md §8.2 P0-1 实测)。同构于 deck 词进 COMPONENT_DISQUALIFIER
    //    的既有先例 —— 内容形态决定画幅,名词只是名词。
    if is_card_series(&lower) {
        return CARD;
    }
    // ② 单组件:触发词命中 且 disqualifier 不命中。
    let trigger = contains_any(&lower, COMPONENT_TRIGGER_LATIN)
        || contains_any(&lower, COMPONENT_TRIGGER_CJK);
    // Phone geometry is a screen contract even when a component trigger appears.
    if trigger
        && !contains_any(&lower, COMPONENT_DISQUALIFIER)
        && !contains_any(&lower, LANDING_WORDS)
        && !mentions_phone_dimensions(&lower)
    {
        return COMPONENT;
    }
    // ③ 演示文稿。放在移动端之前:"手机端演示" 说的是内容形态是 deck,
    //    而 deck 的画幅是投影比例,不是手机屏。
    if contains_any(&lower, SLIDES_WORDS) {
        return SLIDES;
    }
    // ④ 移动端。
    if contains_any(&lower, MOBILE_WORDS) || mentions_phone_dimensions(&lower) {
        return MOBILE;
    }
    // ⑤ 数据型工作区 / dashboard。
    if contains_any(&lower, DASHBOARD_WORDS) {
        return DESKTOP;
    }
    // ⑥ 默认:多区块落地页。
    LANDING
}

// ── Tree-side form classification ────────────────────────────────────────────

/// The tree-side judge of what a root frame IS, as opposed to what the prompt
/// asked for ([`detect_design_type`] above).
///
/// It lives in `op-design-lint` because the detectors there need the same
/// answer the repair passes here do, and that crate sits below this one — see
/// [`op_design_lint::design_form`] for the full rationale. Re-exported so
/// every `crate::design_type::…` import path in the orchestrator is unchanged.
pub use op_design_lint::design_form::{
    classify_root_form, classify_root_form_node, classify_root_form_value, DesignForm,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_form_reads_the_artboard_not_the_prompt() {
        // 0808-gm-1.op's page: 1200 x 2977 marketing site.
        assert_eq!(
            classify_root_form(Some(1200.0), Some(2977.0)),
            DesignForm::Page
        );
        assert_eq!(
            classify_root_form(Some(1200.0), Some(800.0)),
            DesignForm::Page
        );
        assert_eq!(
            classify_root_form(Some(375.0), Some(812.0)),
            DesignForm::MobileScreen
        );
        assert_eq!(
            classify_root_form(Some(1920.0), Some(1080.0)),
            DesignForm::Deck
        );
        // A deck-wide artboard that is NOT 16:9 is a wide page, not a board.
        assert_eq!(
            classify_root_form(Some(1920.0), Some(6000.0)),
            DesignForm::Page
        );
    }

    #[test]
    fn a_card_board_is_not_mistaken_for_a_phone_screen() {
        // The repair layer keys off DesignForm, and the one outcome that
        // would actively damage a card is being read as a 375-wide phone
        // screen (status-bar injection, bottom-nav chrome, mobile reflow).
        // Portrait cards read as the Card form (DS P1.5); the square board
        // keeps its previous Page judgement. None may read as a phone.
        for (w, h, expected) in [
            (1080.0, 1440.0, DesignForm::Card), // XHS 竖版 3:4 — the primary spec
            (1080.0, 1080.0, DesignForm::Page), // XHS 方版 1:1
            (1080.0, 1920.0, DesignForm::Card), // 通用 9:16
        ] {
            let form = classify_root_form(Some(w), Some(h));
            assert_ne!(form, DesignForm::MobileScreen, "{w}x{h}");
            assert_eq!(form, expected, "{w}x{h}");
        }
    }

    #[test]
    fn an_unsized_or_tablet_root_is_unknown_never_a_default() {
        assert_eq!(classify_root_form(None, None), DesignForm::Unknown);
        assert_eq!(classify_root_form(Some(0.0), None), DesignForm::Unknown);
        // Tablet band: neither phone chrome nor desktop gutters are safe.
        assert_eq!(classify_root_form(Some(768.0), None), DesignForm::Unknown);
        assert!(!DesignForm::Unknown.is_scrolling_page());
    }

    #[test]
    fn root_form_from_json_ignores_keyword_sizes() {
        use serde_json::json;
        assert_eq!(
            classify_root_form_value(&json!({"width": 1200, "height": 2977})),
            DesignForm::Page
        );
        assert_eq!(
            classify_root_form_value(&json!({"width": "fill_container"})),
            DesignForm::Unknown
        );
    }

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

    /// The Scene Template Center's generate row takes a topic, not a prompt,
    /// and wraps it before sending. That wrapper is the only thing telling
    /// this classifier the result is a deck — an unwrapped "Q3 复盘" reads as
    /// a landing page and the slides entry point would hand back a
    /// 1200-wide scrolling page. Asserted per locale because the wrapper is
    /// translated: a translation that loses the trigger word breaks the
    /// entry point silently, and it breaks here instead.
    #[test]
    fn the_generate_rows_wrapper_reads_as_a_deck_in_every_locale() {
        use op_editor_core::scene_template_prompt::slides_generate_prompt;

        // Topics chosen to be hostile: none says "deck" on its own, and the
        // last two carry a component trigger ("卡片" / "card") that would win
        // the classifier's first rung without the wrapper's disqualifier.
        for topic in [
            "Q3 复盘",
            "quarterly review",
            "如何做用户访谈",
            "会员卡片权益",
            "our loyalty card program",
        ] {
            assert_ne!(
                detect_design_type(topic).type_,
                DesignType::Slides,
                "{topic}: a bare topic should not already read as a deck, \
                 or this test proves nothing about the wrapper"
            );
            for locale in op_editor_core::Locale::ALL {
                let wrapped = slides_generate_prompt(locale, topic).expect("a topic wraps");
                let preset = detect_design_type(&wrapped);
                assert_eq!(
                    preset.type_,
                    DesignType::Slides,
                    "{locale:?} / {topic}: {wrapped}"
                );
                assert_eq!((preset.width, preset.height), (1920.0, 1080.0));
            }
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
    fn phone_dimensions_in_the_brief_mean_a_mobile_screen() {
        assert_eq!(
            detect_design_type("外卖 App 首页（375×812）：顶部地址与搜索").type_,
            DesignType::MobileScreen
        );
        assert_eq!(
            detect_design_type("Workout detail screen (390x844) with a hero image").type_,
            DesignType::MobileScreen
        );
        assert_eq!(
            detect_design_type(
                "短视频平台首页（375×812，暗色）：顶部分类标签、推荐视频六卡、底部导航。"
            )
            .type_,
            DesignType::MobileScreen
        );
        assert_ne!(
            detect_design_type("官网首页（1440×900，浅色）").type_,
            DesignType::MobileScreen
        );
        assert_ne!(
            detect_design_type("横幅 1200x300 banner").type_,
            DesignType::MobileScreen
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
    fn a_social_card_series_gets_the_portrait_board() {
        // `卡片` is ALSO the component trigger, so before the card rung existed
        // "做一套小红书卡片" resolved to a 400px-wide component
        // (`card-system-0808.md` §8.2 P0-1). Platform words stand alone.
        for prompt in [
            "帮我做一套小红书卡片：如何早起",
            "小红书图文 5 张",
            "做一组公众号图文",
            "an xhs carousel about morning routines",
            "make a card series about note taking",
            "做一套卡片系列讲复利",
        ] {
            let preset = detect_design_type(prompt);
            assert_eq!(preset.type_, DesignType::Card, "{prompt}");
            assert_eq!((preset.width, preset.height), (1080.0, 1440.0), "{prompt}");
            assert_eq!(preset.root_height, 1440.0, "{prompt}");
        }
    }

    #[test]
    fn a_single_card_request_is_still_a_component() {
        // The card rung runs FIRST, so it has to hand these back untouched.
        assert_eq!(
            detect_design_type("卡片组件").type_,
            DesignType::Component,
            "a component request that happens to name a card"
        );
        assert_eq!(
            detect_design_type("a card component for the design system").type_,
            DesignType::Component
        );
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
    fn a_deck_or_screen_still_beats_a_bare_card_noun() {
        // "PPT 封面卡片" has a card NOUN but no series word and no platform
        // word — the deck reading must survive the new first rung.
        assert_eq!(detect_design_type("PPT 封面卡片").type_, DesignType::Slides);
        assert_eq!(
            detect_design_type("the title card for my pitch deck").type_,
            DesignType::Slides
        );
        assert_eq!(
            detect_design_type("a card on the home screen").type_,
            DesignType::LandingPage
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

    // ── motion50 fix 2: carousel is an ordinary App component, not a card
    //    platform ─────────────────────────────────────────────────────────────

    /// motion50 app-10 (lane2): "轮播" inside a brief that explicitly declares
    /// a phone frame must not flip the request to the Card preset — the run
    /// produced two 1080×1440 boards with the 375×812 content crammed into
    /// their top-left corners.
    #[test]
    fn a_carousel_inside_an_explicit_phone_brief_is_not_a_card_series() {
        let app10 = "电商商品详情 App 两屏可交互（375×812）：商品页(大图轮播占位+价格+规格选择+评价三条+底部加购条)、购物车页(商品两条+金额汇总+结算)。交互：规格胶囊 onTap 切换，「加入购物车」onTap 进购物车。动效：价格数字滚动，规格切换 transition，加购按钮 pressed 缩放。高级黑白 + 一处荧光强调。";
        let preset = detect_design_type(app10);
        assert_eq!(preset.type_, DesignType::MobileScreen, "{app10}");
        assert_eq!((preset.width, preset.height), (375.0, 812.0), "{app10}");
    }

    #[test]
    fn an_app_banner_carousel_is_a_mobile_screen() {
        assert_eq!(
            detect_design_type("App 首页含 banner 轮播（375×812）").type_,
            DesignType::MobileScreen
        );
    }

    /// The genuine card readings are untouched: with no screen declaration in
    /// the brief, platform words (轮播 included) still deliver a Card board.
    #[test]
    fn a_carousel_series_without_a_screen_declaration_stays_a_card() {
        assert_eq!(
            detect_design_type("小红书图文轮播一套 6 张").type_,
            DesignType::Card
        );
        assert_eq!(detect_design_type("公众号图文").type_, DesignType::Card);
    }

    #[test]
    fn a_declared_landing_page_outranks_card_and_component_triggers() {
        // web-10 / web-13 of the 0918 rerun: "图文" (card platform word) and
        // "卡片" (component trigger) inside an explicit 官网/长页 brief.
        let car = "新能源汽车车型页（1440 宽长页）：满幅车身英雄、性能三指标、外观颜色选择、内饰图文两段、续航图表、预约试驾表单、页脚。";
        assert_eq!(detect_design_type(car).type_, DesignType::LandingPage);
        let counsel = "心理咨询服务官网（1440 宽长页）：英雄（插画+预约 CTA）、服务三卡片、咨询师四人、流程四步、常见问题、页脚。";
        assert_eq!(detect_design_type(counsel).type_, DesignType::LandingPage);
        // The plain forms keep their rungs.
        assert_eq!(
            detect_design_type("做一个卡片组件").type_,
            DesignType::Component
        );
        assert_eq!(detect_design_type("公众号图文").type_, DesignType::Card);
    }
}

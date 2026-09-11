//! First-launch drafting-table state and prompt contracts.
//!
//! Home is an entry surface over the same `EditorState` as the canvas. It
//! owns only transient chrome state and the small prompt wrapper that turns a
//! family selection into the existing chat-design request shape.

use jian_core::text_input::TextInputState;

/// The persisted first-launch entry preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntrySurface {
    #[default]
    Home,
    Canvas,
}

impl EntrySurface {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Canvas => "canvas",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Self {
        match value {
            "canvas" => Self::Canvas,
            _ => Self::Home,
        }
    }
}

/// Home's four deliberately small scenario families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HomeFamily {
    AppUi,
    KnowledgeCards,
    ScreenshotTutorial,
    EventPoster,
}

impl HomeFamily {
    pub const ALL: [Self; 4] = [
        Self::AppUi,
        Self::KnowledgeCards,
        Self::ScreenshotTutorial,
        Self::EventPoster,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::AppUi => "App 界面",
            Self::KnowledgeCards => "知识卡片",
            Self::ScreenshotTutorial => "截图教程",
            Self::EventPoster => "活动海报",
        }
    }

    pub const fn placeholder(self) -> &'static str {
        match self {
            Self::AppUi => "说说你想做的 App 界面，例如：取餐预约，3 个手机页面",
            Self::KnowledgeCards => "把一段内容做成一套可编辑的知识卡片",
            Self::ScreenshotTutorial => "把这些截图串成一篇步骤图",
            Self::EventPoster => "帮我做一组活动海报，包含时间、地点和报名方式",
        }
    }

    pub const fn expected_outputs(self) -> &'static [&'static str] {
        match self {
            Self::AppUi => &["可编辑组件", "变量", "图层"],
            Self::KnowledgeCards => &["卡片套组", "可编辑文字", "发布版式"],
            Self::ScreenshotTutorial => &["步骤图", "截图标注", "可编辑图层"],
            Self::EventPoster => &["主视觉海报", "活动信息版式", "可编辑图层"],
        }
    }

    /// Wrap the user's brief with the family and device contract understood
    /// by the normal chat-design launch path.
    pub fn generation_prompt(self, draft: &str, device: HomeDevice) -> Option<String> {
        let draft = draft.trim();
        if draft.is_empty() {
            return None;
        }
        let prompt = match self {
            Self::AppUi => format!(
                "请设计一套可编辑的高保真 {} 界面（目标设备：{}）。交付一组完整界面：统一的组件、变量与图层结构，屏幕之间用 onTap 串起导航。\n\n用户需求：{}",
                if device == HomeDevice::Mobile { "mobile app" } else { "web app" },
                device.label(),
                draft
            ),
            Self::KnowledgeCards => format!(
                "请把以下内容制作成一套可编辑的知识卡片（carousel），保留清晰的信息层级、统一的视觉系统和可复用图层。\n\n用户需求：{}",
                draft
            ),
            Self::ScreenshotTutorial => format!(
                "请把以下截图或说明制作成一套可编辑的截图教程（screenshot tutorial），按步骤组织画面并为关键操作加清晰标注。\n\n用户需求：{}",
                draft
            ),
            Self::EventPoster => format!(
                "请制作一组可编辑的活动海报（event-poster-deck），以活动主视觉、时间地点和报名信息为核心，保持整组海报一致。\n\n用户需求：{}",
                draft
            ),
        };
        Some(prompt)
    }
}

/// The two device contexts shown after App 界面 is bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HomeDevice {
    #[default]
    Mobile,
    Desktop,
}

impl HomeDevice {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Mobile => "手机",
            Self::Desktop => "桌面",
        }
    }
}

/// Interactive target ids used by Home hover and pressed feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HomeHit {
    Sheet,
    Send,
    Professional,
    Chip(HomeFamily),
    Card(HomeFamily),
    Device(HomeDevice),
    Attachment,
    TryExample,
    ReferenceLink,
    Figma,
    Recent,
    NewCanvas,
    OpenFile,
    Footer,
}

/// Transient state for the drafting-table Home surface.
#[derive(Debug, Clone)]
pub struct HomeState {
    pub visible: bool,
    pub bound: Option<HomeFamily>,
    pub device: HomeDevice,
    pub draft: String,
    pub hover: Option<HomeHit>,
    pub pressed: Option<HomeHit>,
    /// The same caret/selection machinery used by the chat composer. `draft`
    /// remains the public Home contract; this field keeps native text input,
    /// IME, clipboard, and caret edits lossless.
    pub input: TextInputState,
}

impl Default for HomeState {
    fn default() -> Self {
        Self {
            visible: false,
            bound: None,
            device: HomeDevice::Mobile,
            draft: String::new(),
            hover: None,
            pressed: None,
            input: TextInputState::default(),
        }
    }
}

impl HomeState {
    pub fn bind(&mut self, family: HomeFamily) -> bool {
        let changed = self.bound != Some(family);
        self.bound = Some(family);
        changed
    }

    pub fn unbind(&mut self) -> bool {
        let changed = self.bound.is_some();
        self.bound = None;
        changed
    }

    pub fn toggle_family(&mut self, family: HomeFamily) -> bool {
        if self.bound == Some(family) {
            self.unbind()
        } else {
            self.bind(family)
        }
    }

    pub fn set_draft(&mut self, draft: impl Into<String>) {
        self.draft = draft.into();
        self.input.set_text(self.draft.clone());
    }

    pub fn insert_text(&mut self, text: &str, now_ms: u64) -> bool {
        if text.is_empty() {
            return false;
        }
        self.input.insert_str(text, now_ms);
        self.draft = self.input.text().to_string();
        true
    }

    pub fn backspace(&mut self, now_ms: u64) -> bool {
        let before = (self.input.text().to_string(), self.input.selection());
        self.input.backspace(now_ms);
        let changed = before != (self.input.text().to_string(), self.input.selection());
        if changed {
            self.draft = self.input.text().to_string();
        }
        changed
    }

    pub fn delete_forward(&mut self, now_ms: u64) -> bool {
        let before = (self.input.text().to_string(), self.input.selection());
        self.input.delete_forward(now_ms);
        let changed = before != (self.input.text().to_string(), self.input.selection());
        if changed {
            self.draft = self.input.text().to_string();
        }
        changed
    }

    pub fn move_caret(&mut self, forward: bool, extend: bool, now_ms: u64) {
        if forward {
            self.input.move_right(extend, now_ms);
        } else {
            self.input.move_left(extend, now_ms);
        }
    }

    pub fn select_all(&mut self, now_ms: u64) {
        self.input.select_all();
        self.input.touch(now_ms);
    }

    pub fn set_caret(&mut self, offset: usize, now_ms: u64) {
        self.input
            .set_caret(offset.min(self.input.text().len()), now_ms);
    }

    pub fn generation_prompt(&self) -> Option<String> {
        self.bound
            .unwrap_or(HomeFamily::AppUi)
            .generation_prompt(&self.draft, self.device)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_surface_round_trips_wire_values() {
        assert_eq!(
            EntrySurface::from_str(EntrySurface::Home.as_str()),
            EntrySurface::Home
        );
        assert_eq!(
            EntrySurface::from_str(EntrySurface::Canvas.as_str()),
            EntrySurface::Canvas
        );
        assert_eq!(EntrySurface::from_str("old-value"), EntrySurface::Home);
    }

    #[test]
    fn every_family_wraps_non_empty_draft_and_preserves_device() {
        for family in HomeFamily::ALL {
            let mobile = family
                .generation_prompt("  取餐预约  ", HomeDevice::Mobile)
                .unwrap();
            let desktop = family
                .generation_prompt("取餐预约", HomeDevice::Desktop)
                .unwrap();
            assert!(mobile.contains("取餐预约"));
            assert!(desktop.contains("取餐预约"));
            if family == HomeFamily::AppUi {
                assert!(mobile.contains("手机"));
                assert!(desktop.contains("桌面"));
            }
        }
    }

    #[test]
    fn family_binding_toggles_between_bound_and_unbound() {
        let mut home = HomeState::default();
        assert!(home.bind(HomeFamily::AppUi));
        assert!(!home.bind(HomeFamily::AppUi));
        assert!(home.toggle_family(HomeFamily::AppUi));
        assert_eq!(home.bound, None);
        assert!(home.toggle_family(HomeFamily::KnowledgeCards));
        assert_eq!(home.bound, Some(HomeFamily::KnowledgeCards));
    }
}

//! Studio Home state: the seven creation tasks, their per-task drafts,
//! and the prompt contracts that steer the orchestrator's design-type
//! detection.
//!
//! Home is an entry surface over the same `EditorState` as the canvas. It
//! owns only transient chrome state: which task is active, one draft per
//! task (text + options), and the small prompt wrapper that turns a task
//! selection into the existing chat-design request shape.

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

/// Home's seven creation tasks. The discriminant order is the tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HomeFamily {
    #[default]
    AppUi,
    Web,
    Presentation,
    KnowledgeCards,
    ScreenshotTutorial,
    Infographic,
    EventPoster,
}

impl HomeFamily {
    pub const ALL: [Self; 7] = [
        Self::AppUi,
        Self::Web,
        Self::Presentation,
        Self::KnowledgeCards,
        Self::ScreenshotTutorial,
        Self::Infographic,
        Self::EventPoster,
    ];

    /// Stable persistence/test id.
    pub const fn id(self) -> &'static str {
        match self {
            Self::AppUi => "app",
            Self::Web => "web",
            Self::Presentation => "presentation",
            Self::KnowledgeCards => "knowledge",
            Self::ScreenshotTutorial => "tutorial",
            Self::Infographic => "infographic",
            Self::EventPoster => "poster",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|f| f.id() == value)
    }

    /// Short zh label (the result-view breadcrumb still paints it; the
    /// Home surface itself resolves copy through i18n `home.task.<id>.*`).
    pub const fn label(self) -> &'static str {
        match self {
            Self::AppUi => "应用界面",
            Self::Web => "网站设计",
            Self::Presentation => "演示文稿",
            Self::KnowledgeCards => "图文卡片",
            Self::ScreenshotTutorial => "截图教程",
            Self::Infographic => "信息图",
            Self::EventPoster => "活动海报",
        }
    }

    /// The user-visible name of the active infographic sub-kind.
    pub const fn info_kind_label(kind: InfoKind) -> &'static str {
        match kind {
            InfoKind::Data => "数据",
            InfoKind::Flow => "流程",
            InfoKind::Comparison => "对比",
        }
    }

    /// Wrap the task's draft with the contract the orchestrator's
    /// `detect_design_type` and the desktop launch path understand.
    pub fn generation_prompt(self, draft: &TaskDraft) -> Option<String> {
        let text = draft.text.trim();
        if text.is_empty() {
            return None;
        }
        let prompt = match self {
            Self::AppUi if draft.device == HomeDevice::Mobile => format!(
                "请设计一套可编辑的高保真手机 App 界面（mobile app，375×812）。\
交付一组完整界面：统一的组件、变量与图层结构，屏幕之间用 onTap 串起导航。\n\n用户需求：{text}"
            ),
            Self::AppUi => format!(
                "请设计一套可编辑的高保真桌面端应用界面（desktop app，1440 宽的 dashboard \
工作台）。交付一组完整界面：统一的组件、变量与图层结构。\n\n用户需求：{text}"
            ),
            Self::Web => format!(
                "请设计一个完整的纵向滚动网站页面（landing page，1440 宽）。\
从首屏主视觉到页脚分区块组织内容，保持统一的视觉系统与可编辑图层。\n\n用户需求：{text}"
            ),
            Self::Presentation => format!(
                "请做一份 {} 页的 PPT 演示文稿（slides，{}）。封面、正文与结束页风格统一，\
文字与图形保持可编辑图层。\n\n用户需求：{text}",
                slide_count(text),
                match draft.ratio {
                    SlideRatio::Wide169 => "16:9",
                    SlideRatio::Classic43 => "4:3",
                }
            ),
            Self::KnowledgeCards => format!(
                "请做一套图文卡片（card，竖版 3:4，多页轮播）。保持统一排版与视觉系统，\
文字与图形保持可编辑图层。\n\n用户需求：{text}"
            ),
            Self::ScreenshotTutorial => format!(
                "请做一篇截图教程图文（card，竖版，截图配上步骤说明）。按步骤组织画面，\
并为关键操作加清晰标注。\n\n用户需求：{text}"
            ),
            Self::Infographic => format!(
                "请做一张{}信息图长图（card，竖版图文长图）。信息层级清晰，\
数字与图形保持可编辑图层。\n\n用户需求：{text}",
                Self::info_kind_label(draft.info_kind)
            ),
            Self::EventPoster => format!(
                "请做一套活动海报（card，主海报竖版 + 社交方图）。整组海报视觉一致，\
主视觉、时间地点与报名信息完整，图层保持可编辑。\n\n用户需求：{text}"
            ),
        };
        Some(prompt)
    }
}

/// Page count a presentation wrapper names: the first small number in the
/// draft (e.g. "5 页"), else the 5-page default.
fn slide_count(text: &str) -> u32 {
    let mut digits: Option<String> = None;
    // The trailing space flushes a number that ends the draft.
    for character in text.chars().chain([' ']) {
        if character.is_ascii_digit() {
            digits.get_or_insert_with(String::new).push(character);
            continue;
        }
        if let Some(run) = digits.take() {
            if let Ok(count) = run.parse::<u32>() {
                if (1..=30).contains(&count) {
                    return count;
                }
            }
        }
    }
    5
}

/// Total window the entrance choreography animates over (the recent
/// row's 240 ms stagger + 600 ms rise is the longest block; prototype
/// `enter .6s cubic-bezier(.22,1,.36,1)` with a 0/60/120/180/240 ms
/// stagger). Shared by the widget's paint pass and the host's frame
/// scheduler so both agree on when the entrance has fully settled.
pub const HOME_ENTER_WINDOW_MS: u64 = 840;
/// While the entrance is running the scheduler keeps frames coming at
/// this cadence so the staggered rise never freezes mid-motion.
pub const HOME_ENTER_FRAME_MS: u64 = 16;
/// The example art crossfades over this window after a task switch
/// (prototype `art-in .3s`: opacity .2→1, rise 8 px, scale .985→1).
pub const HOME_ART_SWITCH_MS: u64 = 300;
/// The explore card's hover lift plays out over this window in BOTH
/// directions (prototype `.example-card{transition:transform .3s}`).
pub const HOME_HOVER_LIFT_MS: u64 = 300;

/// The App task's screen dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HomeDevice {
    #[default]
    Mobile,
    Desktop,
}

/// The presentation task's aspect choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SlideRatio {
    #[default]
    Wide169,
    Classic43,
}

/// The infographic task's sub-kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InfoKind {
    #[default]
    Data,
    Flow,
    Comparison,
}

/// One task's kept draft: the text plus the options its segmented
/// control owns. Attachments deliberately live in the chat composer's
/// `pending_attachments` — that list is the single source of truth.
#[derive(Debug, Clone, Default)]
pub struct TaskDraft {
    pub text: String,
    pub device: HomeDevice,
    pub ratio: SlideRatio,
    pub info_kind: InfoKind,
}

impl TaskDraft {
    fn cleared() -> Self {
        Self::default()
    }
}

/// Interactive target ids used by Home hover and pressed feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HomeHit {
    /// The composer's input box (caret press).
    Sheet,
    /// One of the seven task tabs.
    Tab(HomeFamily),
    /// The 更多 ▾ button that opens the narrow-viewport task popover.
    More,
    /// One row of the 更多 popover.
    MoreItem(HomeFamily),
    /// The task's segmented control; `0` is the leftmost option.
    Segment(u8),
    /// The 添加截图 tool.
    Attachment,
    /// The 参考链接 tool (disabled M1).
    ReferenceLink,
    /// The Figma tool (disabled M1).
    Figma,
    /// The 3-directions toggle: the next send generates
    /// [`crate::DEFAULT_VARIANT_COUNT`] directions side by side.
    Variants,
    /// The model button on the submit row.
    ModelChip,
    /// The 开始设计 primary button.
    Send,
    /// The preview footer's 使用这个示例 link.
    UseExample,
    /// The preview footer's 回到工作区 link — shown instead of
    /// UseExample while a workspace is active.
    BackToWorkspace,
    /// The replace-confirm strip's 保留 button.
    ReplaceKeep,
    /// The replace-confirm strip's 使用示例 button.
    ReplaceConfirm,
    /// One of the three 看看还能做什么 cards.
    ExploreCard(HomeFamily),
    /// A recent-project chip.
    Recent(usize),
    /// The ＋ 新建空白画布 button.
    NewCanvas,
    /// The top bar's account avatar. Home is where a first-run user
    /// lands, so the way in to an account has to be reachable without
    /// first going to the professional canvas to find it.
    Account,
    /// The top bar's 打开文件 button.
    OpenFile,
    /// The top bar's 进入专业画布 button.
    Professional,
    /// The 普通 half of the compact (phone) top bar's 普通 / 专业
    /// segmented control. Home IS the normal mode, so the press only
    /// confirms the already-selected segment.
    ModeNormal,
    /// The compact bottom nav's 创作 tab (the Home page itself).
    NavCreate,
    /// The compact bottom nav's 作品 tab.
    NavProjects,
    /// The compact bottom nav's 设置 tab (and the top bar's gear).
    NavSettings,
    /// The 作品 page's current (in-memory) work card.
    WorksCurrent,
    /// One recent-file row on the 作品 page (index into `recent_files`).
    WorksRecent(usize),
    /// The 接入卡's free-tier row.
    ConnectFreeTier,
    /// The 接入卡's own-API-key row.
    ConnectApiKey,
    /// The 接入卡's local-CLI row.
    ConnectCli,
    /// Any press outside the open 接入卡 (or its close affordance).
    ConnectClose,
}

/// Transient state for the Studio Home surface.
#[derive(Debug, Clone)]
pub struct HomeState {
    pub visible: bool,
    /// The always-selected task the composer edits.
    pub task: HomeFamily,
    /// One draft per task; `draft`/`input` mirror the active one.
    pub drafts: [TaskDraft; 7],
    pub draft: String,
    pub hover: Option<HomeHit>,
    pub pressed: Option<HomeHit>,
    /// Whether the composer's input box currently owns the text focus.
    /// Desktop Home ignores it (the surface IS the composer, so it always
    /// reports keyboard focus); touch shells raise and dismiss the
    /// software keyboard off this bit, so it must follow the last pressed
    /// target instead of staying latched to the surface.
    pub composer_focused: bool,
    /// The target the keyboard (Tab / Shift+Tab) moved to, drawn with a
    /// focus ring and activated by Enter / Space. Any pointer press drops
    /// it — the ring is focus-VISIBLE, keyboard users only.
    pub key_focus: Option<HomeHit>,
    /// Scroll offset for a home stack that is taller than the viewport.
    /// The top bar stays pinned while the page content scrolls under it.
    pub scroll_y: f32,
    /// Wall-clock instant the entrance choreography phases against, in ms.
    /// `0` = not started: the surface paints settled (no motion) until a
    /// host paint stamps it. Never persisted — it is frame-clock state.
    pub shown_at_ms: u64,
    /// The same caret/selection machinery used by the chat composer. `draft`
    /// remains the public Home contract; this field keeps native text input,
    /// IME, clipboard, and caret edits lossless.
    pub input: TextInputState,
    /// The "先接入一个模型" connect card, opened by Send / the model chip
    /// when no chat agent can answer yet. Modal over the composer.
    pub connect_card_open: bool,
    /// The narrow-viewport 更多 task popover.
    pub more_open: bool,
    /// The inline 替换现有需求？ confirm strip inside the composer.
    pub replace_pending: bool,
    /// Wall-clock instant the preview art last changed task, for the
    /// 300 ms crossfade. `0` = no switch to animate.
    pub art_switched_at_ms: u64,
    /// Wall-clock instant the hovered explore card last changed, for the
    /// 300 ms hover lift (in AND out). `0` = never stamped; the lift
    /// paints settled so an unstamped host degrades to an instant hover.
    pub card_hover_since_ms: u64,
    /// The explore card the cursor most recently LEFT, so only that card
    /// plays the descent half of the lift; every other rest card stays put.
    pub card_hover_leaving: Option<HomeFamily>,
    /// The compact bottom nav's 作品 page is on show instead of the
    /// 创作 page. Phone-only; every hide returns to 创作.
    pub works_open: bool,
    /// The composer's 3-directions toggle: the next send asks for
    /// several distinct directions side by side instead of one design.
    /// Session state shared by every task, like the model chip.
    pub variants_on: bool,
    /// Host capability: this host cannot run several directions (web has
    /// no variants runner yet), so the toggle is not offered at all.
    pub variants_unavailable: bool,
}

impl Default for HomeState {
    fn default() -> Self {
        Self {
            visible: false,
            task: HomeFamily::AppUi,
            drafts: std::array::from_fn(|_| TaskDraft::cleared()),
            draft: String::new(),
            hover: None,
            pressed: None,
            composer_focused: false,
            key_focus: None,
            scroll_y: 0.0,
            shown_at_ms: 0,
            input: TextInputState::default(),
            connect_card_open: false,
            more_open: false,
            replace_pending: false,
            art_switched_at_ms: 0,
            card_hover_since_ms: 0,
            card_hover_leaving: None,
            works_open: false,
            variants_on: false,
            variants_unavailable: false,
        }
    }
}

impl HomeState {
    /// Leave the surface. The entrance stamp resets so the next show
    /// replays the choreography from the top; hover/pressed drop with it.
    pub fn hide(&mut self) {
        self.visible = false;
        self.hover = None;
        self.pressed = None;
        self.composer_focused = false;
        self.key_focus = None;
        self.shown_at_ms = 0;
        self.connect_card_open = false;
        self.more_open = false;
        self.replace_pending = false;
        self.card_hover_since_ms = 0;
        self.card_hover_leaving = None;
        self.works_open = false;
    }

    /// The next frame instant the entrance choreography still needs, or
    /// `None` when hidden, not yet stamped, or past the whole window.
    pub fn entrance_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        if !self.visible || self.shown_at_ms == 0 {
            return None;
        }
        (now_ms.saturating_sub(self.shown_at_ms) < HOME_ENTER_WINDOW_MS)
            .then_some(now_ms.saturating_add(HOME_ENTER_FRAME_MS))
    }

    /// Stamp a hover change between explore cards: `from` is the card the
    /// cursor left (it plays the descent), `now_ms` starts the 300 ms lift.
    pub fn stamp_card_hover(&mut self, from: Option<HomeFamily>, now_ms: u64) {
        self.card_hover_since_ms = now_ms.max(1);
        self.card_hover_leaving = from;
    }

    /// The next frame instant the explore-card hover lift still needs.
    pub fn hover_lift_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        if !self.visible || self.card_hover_since_ms == 0 {
            return None;
        }
        (now_ms.saturating_sub(self.card_hover_since_ms) < HOME_HOVER_LIFT_MS)
            .then_some(now_ms.saturating_add(HOME_ENTER_FRAME_MS))
    }

    /// The next frame instant the art crossfade still needs.
    pub fn art_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        if !self.visible || self.art_switched_at_ms == 0 {
            return None;
        }
        (now_ms.saturating_sub(self.art_switched_at_ms) < HOME_ART_SWITCH_MS)
            .then_some(now_ms.saturating_add(HOME_ENTER_FRAME_MS))
    }

    pub fn task_draft(&self) -> &TaskDraft {
        self.draft_for(self.task)
    }

    /// The kept draft of any task, active or not (explore cards and the
    /// 更多 popover read the options of tasks that are not selected).
    pub fn draft_for(&self, family: HomeFamily) -> &TaskDraft {
        let index = HomeFamily::ALL
            .iter()
            .position(|candidate| *candidate == family)
            .unwrap_or(0);
        &self.drafts[index]
    }

    fn task_draft_mut(&mut self) -> &mut TaskDraft {
        let index = HomeFamily::ALL
            .iter()
            .position(|family| *family == self.task)
            .unwrap_or(0);
        &mut self.drafts[index]
    }

    /// Switch the active task. The live input text is saved into the old
    /// task's draft and the new one is loaded into `input`; the preview
    /// art crossfade restamps. Returns whether anything changed.
    pub fn set_task(&mut self, family: HomeFamily, now_ms: u64) -> bool {
        if self.task == family {
            return false;
        }
        self.task_draft_mut().text = self.input.text().to_string();
        self.task = family;
        let text = self.task_draft().text.clone();
        self.input.set_text(text.clone());
        self.draft = text;
        self.art_switched_at_ms = now_ms.max(1);
        self.more_open = false;
        self.replace_pending = false;
        true
    }

    pub fn set_device(&mut self, device: HomeDevice) {
        self.task_draft_mut().device = device;
    }

    pub fn set_ratio(&mut self, ratio: SlideRatio) {
        self.task_draft_mut().ratio = ratio;
    }

    pub fn set_info_kind(&mut self, kind: InfoKind) {
        self.task_draft_mut().info_kind = kind;
    }

    /// Fill the draft with an example prompt. A non-empty differing draft
    /// arms the inline confirm strip instead (`replace_pending`), exactly
    /// like the prototype's replace dialog. Returns `true` when the text
    /// was filled immediately.
    pub fn use_example(&mut self, example: &str) -> bool {
        if self.draft.trim().is_empty() || self.draft == example {
            self.set_draft(example);
            self.replace_pending = false;
            true
        } else {
            self.replace_pending = true;
            false
        }
    }

    /// The confirm strip's 使用示例 action.
    pub fn confirm_replace_example(&mut self, example: &str) {
        self.set_draft(example);
        self.replace_pending = false;
    }

    /// The confirm strip's 保留 action.
    pub fn keep_draft(&mut self) {
        self.replace_pending = false;
    }

    pub fn set_draft(&mut self, draft: impl Into<String>) {
        self.draft = draft.into();
        self.input.set_text(self.draft.clone());
        self.task_draft_mut().text = self.draft.clone();
    }

    fn sync_input_into_draft(&mut self) {
        self.draft = self.input.text().to_string();
        self.task_draft_mut().text = self.draft.clone();
    }

    /// Fold a committed IME composition (or any external input-state
    /// write) back into the public draft mirror + the active task's
    /// kept draft.
    pub fn sync_committed_input(&mut self) {
        self.sync_input_into_draft();
    }

    pub fn insert_text(&mut self, text: &str, now_ms: u64) -> bool {
        if text.is_empty() {
            return false;
        }
        self.input.insert_str(text, now_ms);
        self.sync_input_into_draft();
        true
    }

    pub fn backspace(&mut self, now_ms: u64) -> bool {
        let before = (self.input.text().to_string(), self.input.selection());
        self.input.backspace(now_ms);
        let changed = before != (self.input.text().to_string(), self.input.selection());
        if changed {
            self.sync_input_into_draft();
        }
        changed
    }

    pub fn delete_forward(&mut self, now_ms: u64) -> bool {
        let before = (self.input.text().to_string(), self.input.selection());
        self.input.delete_forward(now_ms);
        let changed = before != (self.input.text().to_string(), self.input.selection());
        if changed {
            self.sync_input_into_draft();
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
        self.task.generation_prompt(self.task_draft())
    }
}

#[cfg(test)]
#[path = "home_tests.rs"]
mod tests;

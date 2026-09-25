//! One-click start from an empty Home box: the example brief a task
//! shows becomes an instant, editable first draft built from a shipped
//! scene template, which the AI then refines in place.
//!
//! Pressing 开始设计 with nothing typed used to do nothing. Every task
//! already carries an example brief and the catalogue ships 80 accepted
//! scene templates, so the empty press now means "use the example": the
//! matching template loads in seconds with no model call, and a connected
//! model refines those SAME boards toward the example brief instead of
//! generating a fresh design over minutes.
//!
//! This module owns the platform-free decisions: which template a task
//! (plus its sub-options) maps to, what the Send button offers for the
//! current draft, and the refine prompt. Loading the template and
//! launching the refine turn are host work.

use super::home::{HomeDevice, HomeFamily, HomeState, InfoKind, SlideRatio, TaskDraft};

/// What the Home Send button does for the current draft. Paint (label,
/// fill, tooltip) and the press handler both read this one answer so the
/// button can never promise something the press does not do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeSendMode {
    /// A brief of the user's own: generate from it (开始设计).
    Start,
    /// The box is empty or still holds the unchanged example: start from
    /// the example (用示例开始). With a mapped template this is the instant
    /// draft; without one it is an ordinary generation of the example.
    UseExample,
    /// Nothing can run: open the connect card (接入模型).
    Connect,
    /// The box holds only a link and this host can import websites:
    /// import the page as an editable design (导入这个网站). Needs no
    /// model, so it outranks Connect.
    ImportSite,
}

/// The scene template the example brief of `family` (with the task's
/// sub-options) opens as its instant draft, or `None` when no shipped
/// template is the right KIND of deliverable — the caller then falls back
/// to an ordinary generation, because a fast draft of the wrong type is
/// worse than a slow draft of the right one.
///
/// The table is deliberate and deterministic (one template per task and
/// option). Each template was authored FOR its task's example: its title,
/// copy, page count, sections and palette are the example brief's own, and
/// the example card's title / description / pages line (`home.task.*`)
/// describe it. Change a row and its example copy together, or the card
/// promises one design and the click delivers another.
///
/// | Task / option            | Template                        | The example it realises                               |
/// |--------------------------|---------------------------------|-------------------------------------------------------|
/// | App 界面 · 手机          | `coffee-order-app`              | 3 × 375×812 coffee-ordering screens: home, menu, order |
/// | App 界面 · 桌面          | `coffee-counter-desktop`        | 1440×900 晨光咖啡 store console: overview, order list, order detail |
/// | 网页设计                 | `daybreak-coffee-site`          | 1200-wide Daybreak Coffee site: hero, signature drinks, story, stores (warm white + forest green) |
/// | 演示文稿 · 16:9          | `openpencil-intro-deck`         | 5 × 1920×1080 OpenPencil intro: cover, value, flow, cases, close (blue + neon yellow) |
/// | 演示文稿 · 4:3           | `openpencil-intro-deck-43`      | the same five slides recomposed at 1024×768            |
/// | 图文卡片                 | `coffee-world-carousel`         | 5 × 1080×1440 “咖啡的世界”: cover, varieties, origins, roast, flavour (warm apricot) |
/// | 截图教程                 | `focus-mode-tutorial`           | 3 × 3:4 cards, one per Focus-mode step (blue callouts) |
/// | 信息图 · 数据            | `weekly-review-infographic`     | 1080-wide weekly review: 12 posts / 24k reads / 320 follows, bars, three takeaways |
/// | 信息图 · 流程            | `idea-to-publish-flow`          | 1080-wide five linked steps from idea to publish       |
/// | 信息图 · 对比            | `blank-vs-example-contrast`     | 1080-wide blank-canvas vs example start: four-row table, who each suits |
/// | 活动海报                 | `city-music-fest-poster`        | 城市音乐节 9.26 滨江公园 set: 3:4 main poster + 1:1 social square (neon green + black) |
pub fn example_draft_template(family: HomeFamily, draft: &TaskDraft) -> Option<&'static str> {
    match family {
        HomeFamily::AppUi => Some(match draft.device {
            HomeDevice::Mobile => "coffee-order-app",
            HomeDevice::Desktop => "coffee-counter-desktop",
        }),
        HomeFamily::Web => Some("daybreak-coffee-site"),
        HomeFamily::Presentation => Some(match draft.ratio {
            SlideRatio::Wide169 => "openpencil-intro-deck",
            SlideRatio::Classic43 => "openpencil-intro-deck-43",
        }),
        HomeFamily::KnowledgeCards => Some("coffee-world-carousel"),
        HomeFamily::ScreenshotTutorial => Some("focus-mode-tutorial"),
        HomeFamily::Infographic => Some(match draft.info_kind {
            InfoKind::Data => "weekly-review-infographic",
            InfoKind::Flow => "idea-to-publish-flow",
            InfoKind::Comparison => "blank-vs-example-contrast",
        }),
        HomeFamily::EventPoster => Some("city-music-fest-poster"),
    }
}

/// The in-place refine brief sent after the template draft loads.
///
/// It says, in so many words, that the boards on the canvas ARE the
/// starting point: keep their count, sizes and layout skeleton, rewrite
/// the content toward the example, and emit only the nodes that change
/// (under their existing ids). The host pins it to the modify route, so
/// the model edits the selected boards rather than drawing a new design
/// beside them.
pub fn refine_prompt(family: HomeFamily, brief: &str) -> String {
    let deliverable = match family {
        HomeFamily::AppUi => "界面稿",
        HomeFamily::Web => "网站长页",
        HomeFamily::Presentation => "演示文稿",
        HomeFamily::KnowledgeCards => "图文卡片",
        HomeFamily::ScreenshotTutorial => "截图教程",
        HomeFamily::Infographic => "信息图长图",
        HomeFamily::EventPoster => "活动海报",
    };
    format!(
        "画布上已经有一份按模板载入的{deliverable}初稿（已选中的画板）。请在这份初稿上修改，\
不要重新开始：保留现有画板的数量、尺寸和版式骨架，把标题、正文、数字、配色等内容改成符合下面的需求。\
只输出需要修改的节点，并沿用它们原有的 id。\n\n用户需求：{}",
        brief.trim()
    )
}

impl HomeState {
    /// Whether the active draft means "the example": nothing typed, or
    /// exactly the example the task shows (使用这个示例 / an explore card
    /// fills it verbatim).
    pub fn draft_uses_example(&self, example: &str) -> bool {
        let draft = self.draft.trim();
        draft.is_empty() || draft == example.trim()
    }

    /// The template the active task's example opens as an instant draft.
    pub fn example_draft_template(&self) -> Option<&'static str> {
        example_draft_template(self.task, self.task_draft())
    }

    /// The single answer to "what does Send do right now?".
    ///
    /// A mapped template needs no model, so the example path stays open
    /// without one — the user gets the draft plus the connect prompt
    /// instead of a dead end. An example with no template still needs a
    /// model to generate it, so that case keeps today's connect card.
    pub fn send_mode(&self, example: &str, usable_agent: bool) -> HomeSendMode {
        if self.site_import.available
            && super::home::site_import::site_import_url(&self.draft).is_some()
        {
            return HomeSendMode::ImportSite;
        }
        if self.draft_uses_example(example) {
            if self.example_draft_template().is_some() || usable_agent {
                return HomeSendMode::UseExample;
            }
            return HomeSendMode::Connect;
        }
        if usable_agent {
            HomeSendMode::Start
        } else {
            HomeSendMode::Connect
        }
    }
}

#[cfg(test)]
#[path = "home_example_draft_tests.rs"]
mod tests;

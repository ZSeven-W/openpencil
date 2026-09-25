//! Tests for the one-click example start: the task → template table, the
//! Send-mode decision, the refine prompt, and the workspace draft states.

use super::*;
use crate::editor_ui_state::home::HomeDevice;
use crate::editor_ui_state::workspace::{WorkspacePhase, WorkspaceState};
use crate::scene_template_catalog::{scene_template_by_id, TemplateScene};

fn draft(device: HomeDevice, ratio: SlideRatio, info_kind: InfoKind) -> TaskDraft {
    TaskDraft {
        text: String::new(),
        device,
        ratio,
        info_kind,
    }
}

fn default_draft() -> TaskDraft {
    TaskDraft::default()
}

#[test]
fn every_task_option_maps_to_its_documented_template() {
    let table: [(HomeFamily, TaskDraft, Option<&str>); 11] = [
        (
            HomeFamily::AppUi,
            draft(HomeDevice::Mobile, SlideRatio::Wide169, InfoKind::Data),
            Some("coffee-order-app"),
        ),
        (
            HomeFamily::AppUi,
            draft(HomeDevice::Desktop, SlideRatio::Wide169, InfoKind::Data),
            Some("coffee-counter-desktop"),
        ),
        (
            HomeFamily::Web,
            default_draft(),
            Some("saas-landing-orange"),
        ),
        (
            HomeFamily::Presentation,
            draft(HomeDevice::Mobile, SlideRatio::Wide169, InfoKind::Data),
            Some("slide-deck"),
        ),
        (
            HomeFamily::Presentation,
            draft(HomeDevice::Mobile, SlideRatio::Classic43, InfoKind::Data),
            Some("onboarding-training-deck"),
        ),
        (
            HomeFamily::KnowledgeCards,
            default_draft(),
            Some("knowledge-carousel"),
        ),
        (
            HomeFamily::ScreenshotTutorial,
            default_draft(),
            Some("screenshot-tutorial"),
        ),
        (
            HomeFamily::Infographic,
            draft(HomeDevice::Mobile, SlideRatio::Wide169, InfoKind::Data),
            Some("data-report-infographic"),
        ),
        (
            HomeFamily::Infographic,
            draft(HomeDevice::Mobile, SlideRatio::Wide169, InfoKind::Flow),
            Some("steps-flow-infographic"),
        ),
        (
            HomeFamily::Infographic,
            draft(
                HomeDevice::Mobile,
                SlideRatio::Wide169,
                InfoKind::Comparison,
            ),
            Some("concept-contrast-infographic"),
        ),
        (
            HomeFamily::EventPoster,
            default_draft(),
            Some("music-fest-poster-card"),
        ),
    ];
    for (family, options, expected) in table {
        assert_eq!(
            example_draft_template(family, &options),
            expected,
            "{family:?} {options:?}"
        );
    }
}

/// The instant draft must be the right KIND of deliverable. A mapped
/// template is checked against the catalogue's own scene and board size —
/// the facts the workspace view and the export read — so a remap to a
/// template of the wrong shape fails here instead of reaching a user.
#[test]
fn every_mapped_template_is_the_right_kind_of_deliverable() {
    let cases: [(HomeFamily, TaskDraft, TemplateScene, (u32, u32)); 12] = [
        (
            HomeFamily::AppUi,
            draft(HomeDevice::Mobile, SlideRatio::Wide169, InfoKind::Data),
            TemplateScene::App,
            (375, 812),
        ),
        (
            HomeFamily::AppUi,
            draft(HomeDevice::Desktop, SlideRatio::Wide169, InfoKind::Data),
            TemplateScene::App,
            (1440, 900),
        ),
        (
            HomeFamily::Presentation,
            draft(HomeDevice::Mobile, SlideRatio::Classic43, InfoKind::Data),
            TemplateScene::Slides,
            (1024, 768),
        ),
        (
            HomeFamily::Web,
            default_draft(),
            TemplateScene::Web,
            (1200, 0),
        ),
        (
            HomeFamily::Presentation,
            default_draft(),
            TemplateScene::Slides,
            (1920, 1080),
        ),
        (
            HomeFamily::KnowledgeCards,
            default_draft(),
            TemplateScene::Carousel,
            (1080, 1440),
        ),
        (
            HomeFamily::ScreenshotTutorial,
            default_draft(),
            TemplateScene::Tutorial,
            (1080, 1440),
        ),
        (
            HomeFamily::Infographic,
            draft(HomeDevice::Mobile, SlideRatio::Wide169, InfoKind::Data),
            TemplateScene::Infographic,
            (1080, 0),
        ),
        (
            HomeFamily::Infographic,
            draft(HomeDevice::Mobile, SlideRatio::Wide169, InfoKind::Flow),
            TemplateScene::Infographic,
            (1080, 0),
        ),
        (
            HomeFamily::Infographic,
            draft(
                HomeDevice::Mobile,
                SlideRatio::Wide169,
                InfoKind::Comparison,
            ),
            TemplateScene::Infographic,
            (1080, 0),
        ),
        (
            HomeFamily::EventPoster,
            default_draft(),
            TemplateScene::Card,
            (1080, 1440),
        ),
        (
            HomeFamily::Presentation,
            draft(HomeDevice::Mobile, SlideRatio::Wide169, InfoKind::Flow),
            TemplateScene::Slides,
            (1920, 1080),
        ),
    ];
    for (family, options, scene, (width, height)) in cases {
        let id = example_draft_template(family, &options).expect("mapped");
        let template = scene_template_by_id(id).expect("the mapped template ships");
        assert_eq!(template.scene, scene, "{id}");
        assert_eq!(template.frame_width, width, "{id} width");
        if height > 0 {
            assert_eq!(template.frame_height, height, "{id} height");
        } else {
            // A scrolling page / long image: taller than it is wide.
            assert!(
                template.frame_height > template.frame_width * 2,
                "{id} must be a long page"
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        assert!(
            crate::scene_template_catalog::scene_template_document(id).is_some(),
            "{id} has an embedded document"
        );
    }
}

fn home_with(family: HomeFamily, text: &str) -> HomeState {
    let mut home = HomeState::default();
    home.set_task(family, 1);
    home.set_draft(text);
    home
}

#[test]
fn an_empty_box_or_the_unchanged_example_uses_the_example() {
    let example = "为 Daybreak 咖啡品牌设计一个官网。";
    assert!(home_with(HomeFamily::Web, "").draft_uses_example(example));
    assert!(home_with(HomeFamily::Web, "   ").draft_uses_example(example));
    assert!(home_with(HomeFamily::Web, example).draft_uses_example(example));
    assert!(!home_with(HomeFamily::Web, "我的面包店官网").draft_uses_example(example));
}

#[test]
fn send_mode_offers_the_example_even_without_a_model_when_a_template_exists() {
    let example = "示例";
    // Template-backed task: the draft needs no model.
    let web = home_with(HomeFamily::Web, "");
    assert_eq!(web.send_mode(example, false), HomeSendMode::UseExample);
    assert_eq!(web.send_mode(example, true), HomeSendMode::UseExample);
    // App now ships its own screen set, so it too starts without a model.
    let app = home_with(HomeFamily::AppUi, "");
    assert_eq!(app.send_mode(example, true), HomeSendMode::UseExample);
    assert_eq!(app.send_mode(example, false), HomeSendMode::UseExample);
    // A brief of the user's own keeps today's behaviour.
    let own = home_with(HomeFamily::Web, "我的面包店官网");
    assert_eq!(own.send_mode(example, true), HomeSendMode::Start);
    assert_eq!(own.send_mode(example, false), HomeSendMode::Connect);
}

#[test]
fn the_refine_prompt_edits_the_draft_instead_of_starting_over() {
    let prompt = refine_prompt(HomeFamily::Presentation, "  做一份 5 页产品介绍 PPT  ");
    assert!(prompt.contains("不要重新开始"));
    assert!(prompt.contains("沿用它们原有的 id"));
    assert!(prompt.ends_with("用户需求：做一份 5 页产品介绍 PPT"));
}

#[test]
fn a_draft_with_a_model_generates_and_one_without_settles_done_with_the_banner() {
    let mut workspace = WorkspaceState::default();
    workspace.open_for_generation(HomeFamily::Web, "b", TaskDraft::default(), 0, 5, None);
    workspace.adopt_template_draft("saas-landing-orange", true);
    assert_eq!(workspace.phase, WorkspacePhase::Generating);
    assert!(!workspace.draft_banner_visible());

    let mut offline = WorkspaceState::default();
    offline.open_for_generation(HomeFamily::Web, "b", TaskDraft::default(), 0, 5, None);
    offline.adopt_template_draft("saas-landing-orange", false);
    assert_eq!(offline.phase, WorkspacePhase::Done);
    assert!(offline.draft_banner_visible());

    // Connecting and pressing the banner's refine re-enters Generating.
    assert!(offline.begin_draft_refine());
    assert_eq!(offline.phase, WorkspacePhase::Generating);
    assert!(!offline.draft_banner_visible());
    assert_eq!(offline.draft_template, Some("saas-landing-orange"));
}

#[test]
fn a_new_brief_or_document_forgets_the_draft() {
    let mut workspace = WorkspaceState::default();
    workspace.open_for_generation(HomeFamily::Web, "b", TaskDraft::default(), 0, 5, None);
    workspace.adopt_template_draft("saas-landing-orange", false);
    workspace.open_for_generation(HomeFamily::Web, "c", TaskDraft::default(), 0, 6, None);
    assert_eq!(workspace.draft_template, None);
    assert!(!workspace.draft_awaiting_refine);

    workspace.adopt_template_draft("saas-landing-orange", false);
    workspace.reset_for_new_document();
    assert_eq!(workspace.draft_template, None);
    assert!(!workspace.begin_draft_refine(), "nothing left to refine");
}

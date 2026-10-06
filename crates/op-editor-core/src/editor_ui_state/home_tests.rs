//! Tests for the Studio Home state module.

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
fn seven_tasks_round_trip_their_ids_in_tab_order() {
    assert_eq!(HomeFamily::ALL.len(), 7);
    for family in HomeFamily::ALL {
        assert_eq!(HomeFamily::from_id(family.id()), Some(family));
    }
    assert_eq!(HomeFamily::from_id("app"), Some(HomeFamily::AppUi));
    assert_eq!(HomeFamily::from_id("poster"), Some(HomeFamily::EventPoster));
    assert_eq!(HomeFamily::from_id("retired-family"), None);
    assert_eq!(HomeFamily::ALL[0], HomeFamily::AppUi);
    assert_eq!(HomeFamily::ALL[1], HomeFamily::Web);
    assert_eq!(HomeFamily::ALL[2], HomeFamily::Presentation);
}

#[test]
fn default_state_selects_the_app_task_with_empty_drafts() {
    let home = HomeState::default();
    assert_eq!(home.task, HomeFamily::AppUi);
    assert!(home.draft.is_empty());
    assert!(home.drafts.iter().all(|draft| draft.text.is_empty()));
    assert_eq!(home.task_draft().device, HomeDevice::Mobile);
    assert!(!home.visible);
}

#[test]
fn switching_tasks_keeps_each_draft_text_and_options() {
    let mut home = HomeState::default();
    for character in "取餐预约".chars() {
        home.insert_text(&character.to_string(), 1_000);
    }
    home.set_device(HomeDevice::Desktop);
    assert!(home.set_task(HomeFamily::Presentation, 2_000));
    assert!(
        home.draft.is_empty(),
        "the new task starts from its own draft"
    );
    home.set_ratio(SlideRatio::Classic43);
    home.insert_text("五页产品介绍", 2_100);
    // Round-trip back: the first task kept its text and device.
    assert!(home.set_task(HomeFamily::AppUi, 3_000));
    assert_eq!(home.draft, "取餐预约");
    assert_eq!(home.task_draft().device, HomeDevice::Desktop);
    assert!(home.set_task(HomeFamily::Presentation, 4_000));
    assert_eq!(home.draft, "五页产品介绍");
    assert_eq!(home.task_draft().ratio, SlideRatio::Classic43);
    // Re-selecting the same task is a no-op that keeps the draft.
    assert!(!home.set_task(HomeFamily::Presentation, 5_000));
    assert_eq!(home.draft, "五页产品介绍");
}

#[test]
fn info_kind_options_survive_task_switches() {
    let mut home = HomeState::default();
    home.set_task(HomeFamily::Infographic, 1_000);
    home.set_info_kind(InfoKind::Comparison);
    home.set_task(HomeFamily::Web, 2_000);
    home.set_task(HomeFamily::Infographic, 3_000);
    assert_eq!(home.task_draft().info_kind, InfoKind::Comparison);
}

#[test]
fn use_example_fills_an_empty_draft_and_confirms_a_busy_one() {
    let mut home = HomeState::default();
    assert!(home.use_example("做一个咖啡点单 App"));
    assert_eq!(home.draft, "做一个咖啡点单 App");
    assert!(!home.replace_pending);
    // The same example is idempotent.
    assert!(home.use_example("做一个咖啡点单 App"));
    // A differing draft arms the inline confirm strip instead.
    home.set_draft("我自己的需求");
    assert!(!home.use_example("做一个咖啡点单 App"));
    assert!(home.replace_pending);
    assert_eq!(
        home.draft, "我自己的需求",
        "the draft survives until confirmed"
    );
    // 保留 keeps the draft; 使用示例 replaces it.
    home.keep_draft();
    assert!(!home.replace_pending);
    assert_eq!(home.draft, "我自己的需求");
    assert!(!home.use_example("做一个咖啡点单 App"));
    home.confirm_replace_example("做一个咖啡点单 App");
    assert_eq!(home.draft, "做一个咖啡点单 App");
}

#[test]
fn hide_drops_pointer_state_and_resets_the_entrance_stamp() {
    let mut home = HomeState {
        visible: true,
        shown_at_ms: 5_000,
        hover: Some(HomeHit::Send),
        pressed: Some(HomeHit::Send),
        connect_card_open: true,
        more_open: true,
        replace_pending: true,
        ..HomeState::default()
    };
    home.hide();
    assert!(!home.visible);
    assert_eq!(home.hover, None);
    assert_eq!(home.pressed, None);
    assert_eq!(home.shown_at_ms, 0, "the next show must animate again");
    assert!(!home.connect_card_open, "the modal card must not survive");
    assert!(!home.more_open);
    assert!(!home.replace_pending);
}

#[test]
fn entrance_deadline_frames_only_inside_the_window() {
    let mut home = HomeState::default();
    assert_eq!(home.entrance_deadline_ms(5_000), None, "hidden");
    home.visible = true;
    assert_eq!(home.entrance_deadline_ms(5_000), None, "not stamped yet");
    home.shown_at_ms = 5_000;
    assert_eq!(home.entrance_deadline_ms(5_000), Some(5_016));
    assert_eq!(
        home.entrance_deadline_ms(5_000 + HOME_ENTER_WINDOW_MS - 1),
        Some(5_000 + HOME_ENTER_WINDOW_MS - 1 + HOME_ENTER_FRAME_MS)
    );
    assert_eq!(
        home.entrance_deadline_ms(5_000 + HOME_ENTER_WINDOW_MS),
        None,
        "settled"
    );
}

#[test]
fn art_switch_restarts_the_crossfade_window() {
    let mut home = HomeState::default();
    assert_eq!(home.art_deadline_ms(1_000), None);
    home.visible = true;
    home.set_task(HomeFamily::Web, 2_000);
    assert_eq!(home.art_switched_at_ms, 2_000);
    assert_eq!(home.art_deadline_ms(2_000), Some(2_016));
    assert_eq!(
        home.art_deadline_ms(2_000 + HOME_ART_SWITCH_MS - 1),
        Some(2_315)
    );
    assert_eq!(home.art_deadline_ms(2_000 + HOME_ART_SWITCH_MS), None);
}

#[test]
fn every_task_wraps_a_non_empty_draft_with_its_key_phrase() {
    let phrases: [(HomeFamily, &str); 7] = [
        (HomeFamily::AppUi, "手机 App 界面（mobile app，375×812）"),
        (HomeFamily::Web, "纵向滚动网站页面（landing page"),
        (HomeFamily::Presentation, "PPT 演示文稿（slides"),
        (HomeFamily::KnowledgeCards, "图文卡片（card，竖版 3:4"),
        (HomeFamily::ScreenshotTutorial, "截图教程图文（card"),
        (HomeFamily::Infographic, "信息图长图（card"),
        (HomeFamily::EventPoster, "活动海报（card"),
    ];
    for (family, phrase) in phrases {
        let prompt = family
            .generation_prompt(&TaskDraft {
                text: "  取餐预约  ".into(),
                ..TaskDraft::default()
            })
            .unwrap_or_else(|| panic!("{family:?} wraps"));
        assert!(prompt.contains(phrase), "{family:?}: {prompt}");
        assert!(prompt.contains("取餐预约"), "{family:?} keeps the draft");
    }
    // An empty draft never queues a turn.
    for family in HomeFamily::ALL {
        assert!(family.generation_prompt(&TaskDraft::default()).is_none());
    }
}

#[test]
fn app_desktop_wrapper_names_the_desktop_contract() {
    let prompt = HomeFamily::AppUi.generation_prompt(&TaskDraft {
        text: "门店工作台".into(),
        device: HomeDevice::Desktop,
        ..TaskDraft::default()
    });
    assert!(prompt
        .as_deref()
        .is_some_and(|p| p.contains("桌面端应用界面（desktop app")));
    assert!(prompt.as_deref().is_some_and(|p| p.contains("门店工作台")));
}

#[test]
fn presentation_wrapper_reads_the_page_count_from_the_draft() {
    let wrap = |text: &str| {
        HomeFamily::Presentation
            .generation_prompt(&TaskDraft {
                text: text.into(),
                ..TaskDraft::default()
            })
            .unwrap()
    };
    assert!(wrap("产品介绍").contains("5 页"), "default is five pages");
    assert!(wrap("做一份 8 页的介绍").contains("8 页"));
    assert!(wrap("10 页路演").contains("10 页"));
    assert!(
        wrap("第 2026 季度汇报").contains("5 页"),
        "years are not page counts"
    );
    // The named ratio follows the segmented choice.
    assert!(wrap("介绍").contains("16:9"));
    let classic = HomeFamily::Presentation.generation_prompt(&TaskDraft {
        text: "介绍".into(),
        ratio: SlideRatio::Classic43,
        ..TaskDraft::default()
    });
    assert!(classic.unwrap().contains("4:3"));
}

#[test]
fn infographic_wrapper_names_the_active_kind() {
    for (kind, label) in [
        (InfoKind::Data, "数据"),
        (InfoKind::Flow, "流程"),
        (InfoKind::Comparison, "对比"),
    ] {
        let prompt = HomeFamily::Infographic
            .generation_prompt(&TaskDraft {
                text: "整理这些信息".into(),
                info_kind: kind,
                ..TaskDraft::default()
            })
            .unwrap();
        assert!(prompt.contains(&format!("{label}信息图长图")), "{prompt}");
    }
}

/// The wrappers are the only thing telling `detect_design_type` what
/// artboard each Home task needs — a wrapper that loses its trigger
/// words silently hands the launch path the wrong design type.
#[test]
fn orchestrator_detection_classifies_every_task_wrapper() {
    let draft = TaskDraft {
        text: "咖啡点单，三个页面".into(),
        ..TaskDraft::default()
    };
    let cases = [
        (
            HomeFamily::AppUi.generation_prompt(&draft).unwrap(),
            op_orchestrator::DesignType::MobileScreen,
        ),
        (
            HomeFamily::AppUi
                .generation_prompt(&TaskDraft {
                    device: HomeDevice::Desktop,
                    ..draft.clone()
                })
                .unwrap(),
            op_orchestrator::DesignType::DesktopScreen,
        ),
        (
            HomeFamily::Web.generation_prompt(&draft).unwrap(),
            op_orchestrator::DesignType::LandingPage,
        ),
        (
            HomeFamily::Presentation.generation_prompt(&draft).unwrap(),
            op_orchestrator::DesignType::Slides,
        ),
        (
            HomeFamily::KnowledgeCards
                .generation_prompt(&draft)
                .unwrap(),
            op_orchestrator::DesignType::Card,
        ),
        (
            HomeFamily::ScreenshotTutorial
                .generation_prompt(&draft)
                .unwrap(),
            op_orchestrator::DesignType::Card,
        ),
        (
            HomeFamily::Infographic.generation_prompt(&draft).unwrap(),
            op_orchestrator::DesignType::Card,
        ),
        (
            HomeFamily::EventPoster.generation_prompt(&draft).unwrap(),
            op_orchestrator::DesignType::Card,
        ),
    ];
    for (prompt, expected) in cases {
        assert_eq!(
            op_orchestrator::detect_design_type(&prompt).type_,
            expected,
            "{prompt}"
        );
    }
}

#[test]
fn the_hover_stamp_names_the_leaving_card_and_schedules_frames() {
    let mut home = HomeState {
        visible: true,
        ..HomeState::default()
    };
    assert_eq!(home.hover_lift_deadline_ms(1_000), None);
    home.stamp_card_hover(Some(HomeFamily::EventPoster), 1_000);
    assert_eq!(home.card_hover_leaving, Some(HomeFamily::EventPoster));
    assert_eq!(
        home.hover_lift_deadline_ms(1_100),
        Some(1_100 + HOME_ENTER_FRAME_MS)
    );
    assert_eq!(
        home.hover_lift_deadline_ms(1_000 + HOME_HOVER_LIFT_MS),
        None
    );
    home.hide();
    assert_eq!(home.card_hover_since_ms, 0);
    assert_eq!(home.card_hover_leaving, None);
}

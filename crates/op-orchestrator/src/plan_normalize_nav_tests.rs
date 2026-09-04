use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

fn request(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

fn section(id: &str, label: &str) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 375.0,
            height: 120.0,
        },
        id_prefix: String::new(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn mobile_plan(name: &str, labels: &[(&str, &str)]) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: name.into(),
            width: 375.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: labels
            .iter()
            .map(|(id, label)| section(id, label))
            .collect(),
        style_guide_name: None,
    }
}

fn has_bottom_nav(plan: &OrchestratorPlan) -> bool {
    plan.subtasks.iter().any(is_bottom_nav_subtask)
}

#[test]
fn meteo_now_screen_gets_bottom_navigation_backstop() {
    let mut plan = mobile_plan(
        "Meteo - Now Screen",
        &[
            ("hero", "Header & Main Temperature Hero"),
            ("telemetry", "Telemetry & Wind Grid"),
            ("hourly", "Hourly Forecast Strip"),
            ("forecast", "7-Day Forecast List"),
            ("sun-arc", "Sunrise & Sunset Arc"),
        ],
    );

    normalize(
        &mut plan,
        &request("Design the Now screen for the Meteo weather app"),
    );

    assert!(has_bottom_nav(&plan));
    let nav = plan
        .subtasks
        .last()
        .expect("navbar should be appended last");
    assert_eq!(nav.id, "bottom-navigation");
    assert_eq!(nav.parent_frame_id.as_deref(), Some("root"));
    assert_eq!(nav.region.width, 375.0);
}

#[test]
fn detail_screen_does_not_get_implicit_bottom_navigation() {
    let mut plan = mobile_plan(
        "Meteo - Forecast Detail Screen",
        &[
            ("header", "Forecast Header"),
            ("chart", "Precipitation Chart"),
            ("metrics", "Weather Metrics"),
            ("advisory", "Weather Advisory"),
        ],
    );

    normalize(&mut plan, &request("Design a forecast detail screen"));

    assert!(!has_bottom_nav(&plan));
}

#[test]
fn form_flow_does_not_get_nav_even_when_name_mentions_main_screen() {
    let mut plan = mobile_plan(
        "Account Form - Main Screen",
        &[
            ("header", "Account Header"),
            ("fields", "Profile Fields"),
            ("preferences", "Preferences"),
            ("actions", "Save Actions"),
        ],
    );

    normalize(&mut plan, &request("Design a mobile account form"));

    assert!(!has_bottom_nav(&plan));
}

#[test]
fn screen_name_matching_uses_word_boundaries() {
    let mut plan = mobile_plan(
        "Performance Screen",
        &[
            ("summary", "Summary"),
            ("chart", "Trend Chart"),
            ("metrics", "Metrics"),
        ],
    );

    normalize(&mut plan, &request("Design a performance screen"));

    assert!(!has_bottom_nav(&plan), "form must not match performance");
}

#[test]
fn legacy_compound_primary_screen_names_keep_bottom_navigation() {
    for name in [
        "FoodHomeScreen",
        "Community Newsfeed",
        "Discovery Screen",
        "In-App Browser",
    ] {
        let mut plan = mobile_plan(
            name,
            &[
                ("header", "Header"),
                ("content", "Primary Content"),
                ("summary", "Summary"),
            ],
        );

        normalize(&mut plan, &request("Design a primary mobile app screen"));

        assert!(
            has_bottom_nav(&plan),
            "legacy primary-screen marker should still match: {name}"
        );
    }
}

#[test]
fn buy_now_screen_does_not_get_implicit_bottom_navigation() {
    let mut plan = mobile_plan(
        "Buy Now Screen",
        &[
            ("summary", "Order Summary"),
            ("payment", "Payment Method"),
            ("actions", "Purchase Actions"),
        ],
    );

    normalize(&mut plan, &request("Design a Buy Now purchase flow"));

    assert!(!has_bottom_nav(&plan));
}

#[test]
fn explicit_no_nav_request_overrides_primary_screen_backstop() {
    let mut plan = mobile_plan(
        "Meteo - Now Screen",
        &[
            ("hero", "Temperature Hero"),
            ("hourly", "Hourly Forecast"),
            ("forecast", "7-Day Forecast"),
        ],
    );

    normalize(
        &mut plan,
        &request("Design the Meteo Now screen without bottom navigation"),
    );

    assert!(!has_bottom_nav(&plan));
}

#[test]
fn explicit_no_nav_request_removes_planned_bottom_navigation() {
    let mut plan = mobile_plan(
        "Meteo - Now Screen",
        &[
            ("hero", "Temperature Hero"),
            ("hourly", "Hourly Forecast"),
            ("forecast", "7-Day Forecast"),
            ("bottom-nav", "Bottom Navigation"),
        ],
    );

    normalize(
        &mut plan,
        &request("Design the Meteo Now screen with no bottom nav"),
    );

    assert!(
        !has_bottom_nav(&plan),
        "an explicit no-nav instruction must override the model plan"
    );
}

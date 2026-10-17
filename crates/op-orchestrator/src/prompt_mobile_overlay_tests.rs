//! Local overlay coordinates must not count a preceding status bar twice.
use super::*;

const MARKER: &str = "MOBILE OVERLAY COORDINATES:";
const WORKOUT_BRIEF: &str = "Workout detail screen (375×812): hero image with a back button, workout title and meta row, difficulty badges, an exercise list of six with thumbnail, name, sets and reps, a start button pinned at the bottom.";

fn retained_plan() -> OrchestratorPlan {
    // Original planner output from workout 07; its generator later authored
    // back-button y=76 even though the independent status bar already took 62px.
    serde_json::from_str(include_str!(
        "test_fixtures/workout_local_overlay_plan.json"
    ))
    .expect("retained workout plan")
}

fn coordinate_rule(prompt: &str) -> &str {
    prompt
        .lines()
        .find(|line| line.starts_with(MARKER))
        .expect("local coordinate contract")
}

#[test]
fn retained_mobile_hero_gets_local_coordinates_on_every_retry() {
    let mut request = req();
    request.model = Some("glm-5.3-flash".into());
    request.prompt = WORKOUT_BRIEF.into();
    let mobile = retained_plan();
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        let (call, _) = bsp(
            &mobile.subtasks[0],
            &mobile,
            &request,
            AbortFlag::new(),
            reduced,
            minimal,
        );
        let rule = coordinate_rule(&call.user_prompt);
        assert!(rule.contains("x/y are local to their layout:none parent"));
        assert!(rule.contains("BELOW a separate status bar"));
        assert!(rule.contains("never add its height again"));
        assert!(rule.contains("16-24px from the local top"));
        assert!(rule.contains("unless the user's composition says otherwise"));
        assert_eq!(call.user_prompt.matches(MARKER).count(), 1);
        eprintln!(
            "mobile overlay contract: {} chars, {} estimated tokens\n{rule}",
            rule.chars().count(),
            op_ai_skills::budget::estimate_tokens(rule)
        );
    }
}

#[test]
fn desktop_hero_does_not_receive_a_mobile_safe_area_contract() {
    let mut request = req();
    request.model = Some("glm-5.3-flash".into());
    request.prompt = "Desktop workout website with a hero and back button".into();
    let mut desktop = retained_plan();
    desktop.root_frame.width = 1440.0;
    desktop.root_frame.height = 900.0;
    desktop.subtasks[0].region.width = 1440.0;
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        let (call, _) = bsp(
            &desktop.subtasks[0],
            &desktop,
            &request,
            AbortFlag::new(),
            reduced,
            minimal,
        );
        assert!(!call.user_prompt.contains(MARKER));
    }
}

#[test]
fn fullscreen_and_no_chrome_contexts_keep_conditional_safe_area_ownership() {
    for context in ["fullscreen", "no-status-bar", "append", "card"] {
        let mut request = req();
        request.model = Some("glm-5.3-flash".into());
        let mut mobile = retained_plan();
        request.prompt = match context {
            "fullscreen" => {
                mobile.root_frame.layout = Some("none".into());
                "A fullscreen edge-to-edge hero extending behind system chrome; keep the explicitly positioned back button at y=76."
            }
            "no-status-bar" => "A mobile workout preview without any status bar or system chrome.",
            "append" => {
                request.append_context = Some(crate::types::AppendContext {
                    target_parent_id: "existing".into(),
                    target_width: 375.0,
                    existing_section_labels: vec!["Workout photo".into()],
                    is_mobile: true,
                });
                "Add controls to the existing photo viewport; do not assume a status bar exists."
            }
            "card" => {
                mobile.root_frame.height = 300.0;
                mobile.subtasks.truncate(1);
                "A mobile workout card with a decorative action centered on its photo."
            }
            _ => unreachable!(),
        }.into();
        let (call, _) = bsp(
            &mobile.subtasks[0],
            &mobile,
            &request,
            AbortFlag::new(),
            false,
            true,
        );
        let rule = coordinate_rule(&call.user_prompt);
        assert!(
            rule.contains("For a section in vertical flow BELOW a separate status bar"),
            "{context}"
        );
        assert!(
            rule.contains("If the hero really extends BEHIND system chrome"),
            "{context}"
        );
        assert!(
            rule.contains("with no system chrome, add none"),
            "{context}"
        );
        assert!(
            rule.contains(
                "do not reposition card controls, decorations, or explicitly placed actions"
            ),
            "{context}"
        );
        assert!(call.user_prompt.contains(&request.prompt));
    }
}

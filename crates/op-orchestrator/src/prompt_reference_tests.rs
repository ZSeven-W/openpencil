use super::*;
use crate::reference_skeleton::{HeroKind, NavKind, ReferenceSkeleton};

fn skeleton() -> ReferenceSkeleton {
    ReferenceSkeleton {
        source: "example.com".into(),
        width: 1200.0,
        sections: vec![],
        nav_kind: NavKind::None,
        hero_kind: HeroKind::None,
        column_rhythm: vec![],
    }
}

#[test]
fn planning_modes_append_reference_skeleton_to_user_prompt_only_when_present() {
    for mode in [
        PlanningMode::Rich,
        PlanningMode::Minimal,
        PlanningMode::Compact,
    ] {
        let with_skeleton = DesignRequest {
            prompt: "Design a product page".into(),
            reference_skeleton: Some(skeleton()),
            ..Default::default()
        };
        let without_skeleton = DesignRequest {
            prompt: "Design a product page".into(),
            ..Default::default()
        };

        let with_prompt = build_orchestrator_prompt(&with_skeleton, mode, AbortFlag::new());
        let without_prompt = build_orchestrator_prompt(&without_skeleton, mode, AbortFlag::new());

        assert!(with_prompt
            .call_request
            .user_prompt
            .contains("REFERENCE SKELETON"));
        assert!(!without_prompt
            .call_request
            .user_prompt
            .contains("REFERENCE SKELETON"));
    }
}

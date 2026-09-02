//! Scroll-orchestration intent for the sub-agent budget arms.
//!
//! The `scroll-orchestration` skill is keyword-gated at the resolve layer,
//! but the Basic / Standard budget arms (5200 / 6500) cannot fit its
//! ~3000 tokens on top of the always-kept base skills, so on a weak-model
//! tier the teaching was silently dropped for `BudgetExhausted` on every
//! scroll prompt — the same failure shape the deck and card arms exist
//! for. This module answers "does this request want scroll
//! orchestration?" with the SKILL'S OWN trigger keywords so the arm and
//! the resolver can never disagree about what counts as a scroll prompt.

use op_ai_skills::resolver::match_trigger;
use std::collections::HashMap;

/// Name of the generation skill whose trigger defines scroll intent.
pub(crate) const SCROLL_SKILL: &str = "scroll-orchestration";

/// Whether `prompt` fires the `scroll-orchestration` skill's trigger.
/// A missing skill (a trimmed corpus build) means no scroll intent.
pub(crate) fn is_scroll_orchestration_request(prompt: &str) -> bool {
    op_ai_skills::get_skill_by_name(SCROLL_SKILL)
        .is_some_and(|skill| match_trigger(&skill.meta.trigger, prompt, &HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_on_the_skill_keywords_in_either_language() {
        assert!(is_scroll_orchestration_request(
            "做一个桌面端 landing 页，要有滚动视差效果"
        ));
        assert!(is_scroll_orchestration_request(
            "Build a scrollytelling landing page with a sticky nav"
        ));
    }

    #[test]
    fn stays_quiet_on_an_ordinary_page() {
        assert!(!is_scroll_orchestration_request(
            "做一个 SaaS 产品定价页，三档套餐卡片"
        ));
        assert!(!is_scroll_orchestration_request(
            "A login screen for a banking app"
        ));
    }

    #[test]
    fn the_arm_keys_off_the_same_trigger_the_resolver_uses() {
        let skill = op_ai_skills::get_skill_by_name(SCROLL_SKILL).expect("skill registered");
        let op_ai_skills::types::SkillTrigger::Keywords(keywords) = &skill.meta.trigger else {
            panic!("scroll-orchestration must stay keyword-gated");
        };
        for keyword in keywords {
            assert!(
                is_scroll_orchestration_request(&format!("page with {keyword}")),
                "keyword {keyword:?} must fire the arm"
            );
        }
    }
}

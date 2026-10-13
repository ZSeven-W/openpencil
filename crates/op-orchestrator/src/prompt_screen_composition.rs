//! Carry the screen's visual hierarchy into each independently generated part.
use crate::plan::{OrchestratorPlan, Subtask};

pub(super) fn instruction(plan: &OrchestratorPlan, subtask: &Subtask) -> String {
    let canvas = plan
        .root_frame
        .fill
        .as_ref()
        .map(|fills| {
            fills
                .iter()
                .map(|fill| fill.color.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|colors| !colors.is_empty());
    let canvas = canvas.map(|colors| format!(
        "PAGE CANVAS: the existing parent uses {colors}. Your root stays transparent; check text, charts and surfaces against this actual canvas. "
    )).unwrap_or_default();
    let mut owners = plan.subtasks.iter().filter(|candidate| {
        candidate.screen == subtask.screen
            && candidate
                .elements
                .as_deref()
                .is_some_and(|e| e.contains("ARCHETYPE:"))
    });
    let first_owner = owners.next();
    // Model plans can repeat the marker in a header and its hero. Treat
    // conflicting ownership as ambiguous rather than demoting the real hero.
    let owner = first_owner.filter(|_| owners.next().is_none());
    let hierarchy = match owner {
        Some(owner) if owner.id == subtask.id => format!(
            "SIGNATURE MOMENT: this section owns the screen's protagonist ({}). Realize its authored scale and visual idea; do not mute it as a quiet section. Keep supporting metadata subordinate.", owner.label
        ),
        Some(owner) => format!(
            "SIGNATURE MOMENT belongs to \"{}\" on this screen. This section supports it; do not create a second hero or oversized focal card. Quiet means secondary visual weight, NOT invisible data: retain legible chart marks, selected states and functional accents even if a quiet-section note says no accent fills.", owner.label
        ),
        None => "SIGNATURE MOMENT: give the screen one memorable focal module; follow the planned scale and content priority without assuming this section owns the hero. Supporting sections stay subordinate. Keep chart marks and selected states legible, with functional accents.".to_owned(),
    };
    format!("{canvas}{hierarchy}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hero_ownership_is_per_screen_and_keeps_functional_color_in_support() {
        let mut plan = crate::plan::parse_plan(r##"{
            "rootFrame":{"id":"p","name":"App","width":375,"height":812,"fill":[{"type":"solid","color":"#111111"}]},
            "subtasks":[
                {"id":"metric","label":"Weekly metric","screen":"home","elements":"quiet section; ARCHETYPE: progress","region":{"width":375,"height":220}},
                {"id":"chart","label":"Weekly chart","screen":"home","elements":"quiet section: no accent fills","region":{"width":375,"height":180}},
                {"id":"map","label":"Route map","screen":"detail","elements":"ARCHETYPE: route-map","region":{"width":375,"height":300}}
            ]
        }"##).unwrap();
        let hero = instruction(&plan, &plan.subtasks[0]);
        assert!(hero.contains("owns the screen's protagonist"));
        assert!(hero.contains("do not mute"));
        let chart = instruction(&plan, &plan.subtasks[1]);
        assert!(chart.contains("belongs to \"Weekly metric\""));
        assert!(chart.contains("legible chart marks"));
        assert!(chart.contains("#111111"));
        let detail = instruction(&plan, &plan.subtasks[2]);
        assert!(detail.contains("owns the screen's protagonist (Route map)"));
        assert!(!detail.contains("Weekly metric"));
        plan.subtasks[1].elements = Some("ARCHETYPE: progress — chart".into());
        let ambiguous = instruction(&plan, &plan.subtasks[1]);
        assert!(!ambiguous.contains("belongs to"));
        assert!(!ambiguous.contains("owns the screen"));
        assert!(ambiguous.contains("planned scale"));
        assert!(instruction(&plan, &plan.subtasks[2]).contains("owns the screen"));
    }
}

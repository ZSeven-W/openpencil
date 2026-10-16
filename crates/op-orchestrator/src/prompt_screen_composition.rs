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
            && candidate.elements.as_deref().is_some_and(owns_protagonist)
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

fn owns_protagonist(elements: &str) -> bool {
    elements.split("ARCHETYPE:").skip(1).any(|suffix| {
        let archetype = suffix
            .split([';', '\n', '—', ','])
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        // GLM sometimes tags supporting content as an archetype too. It is
        // not a second protagonist, and must not cancel the real owner's
        // hierarchy. Keep custom creative archetypes eligible.
        !archetype.is_empty()
            && !matches!(
                archetype.as_str(),
                "quiet section" | "supporting section" | "none"
            )
    })
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

    #[test]
    fn quiet_archetype_in_real_coffee_plan_does_not_cancel_the_photo_owner() {
        let plan = crate::plan::parse_plan(r##"{
            "rootFrame":{"id":"p","name":"晴日咖啡首页","width":375,"height":812},
            "subtasks":[
                {"id":"brand-header","label":"品牌区","elements":"ARCHETYPE: image-led — protagonist 咖啡照片头图 40%","region":{"width":375,"height":132}},
                {"id":"recommend","label":"推荐饮品","elements":"ARCHETYPE: quiet section; quiet section: secondary visual weight; 一张真实咖啡照片卡片作为推荐主视觉","region":{"width":375,"height":230}},
                {"id":"drink-list","label":"饮品价格列表","elements":"quiet section: secondary visual weight","region":{"width":375,"height":196}}
            ]
        }"##).unwrap();
        assert!(instruction(&plan, &plan.subtasks[0]).contains("owns the screen's protagonist"));
        for task in &plan.subtasks[1..] {
            let prompt = instruction(&plan, task);
            assert!(prompt.contains("belongs to \"品牌区\""));
            assert!(prompt.contains("do not create a second hero"));
        }
        assert!(owns_protagonist(
            "ARCHETYPE: kinetic-collage — custom composition"
        ));
        assert!(!owns_protagonist("ARCHETYPE: quiet section — compact card"));
        assert!(!owns_protagonist("ARCHETYPE: "));
    }
}

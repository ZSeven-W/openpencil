//! Tests for the pure half of side-by-side design directions.

use super::*;
use crate::agent_identity::assign_agent_identities;
use jian_ops_schema::node::PenNode;
use op_ai_skills::style_guide::{style_guide_registry, Platform};
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::json;

const MOBILE_BRIEF: &str = "请设计一套可编辑的高保真手机 App 界面（mobile app，375×812）。\
用户需求：一个咖啡外卖 app";
const WEB_BRIEF: &str = "请设计一个完整的纵向滚动网站页面（landing page，1440 宽）。\
用户需求：一家 SaaS 公司的官网";

fn guide(name: &str) -> &'static op_ai_skills::style_guide::ParsedStyleGuide {
    style_guide_registry()
        .iter()
        .find(|g| g.name == name)
        .expect("corpus guide")
}

fn base_request(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        model: Some("glm-5.3-flash".into()),
        provider: None,
        design_md: None,
        concurrency: 4,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

#[test]
fn directions_get_distinct_guides_in_slot_order() {
    let plans = choose_variant_style_guides(WEB_BRIEF, None, 3);
    assert_eq!(plans.len(), 3);
    assert_eq!(
        plans.iter().map(|p| p.index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    let mut ids: Vec<&str> = plans.iter().map(|p| p.style_guide.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        3,
        "every direction must use its own guide: {plans:?}"
    );
}

#[test]
fn the_choice_is_deterministic() {
    assert_eq!(
        choose_variant_style_guides(MOBILE_BRIEF, None, 4),
        choose_variant_style_guides(MOBILE_BRIEF, None, 4)
    );
}

#[test]
fn a_phone_brief_only_draws_from_the_mobile_shelf() {
    assert_eq!(variant_platform(MOBILE_BRIEF), Platform::Mobile);
    let plans = choose_variant_style_guides(MOBILE_BRIEF, None, 4);
    assert_eq!(plans.len(), 4);
    for plan in &plans {
        assert_eq!(
            guide(&plan.style_guide).platform,
            Platform::Mobile,
            "{} is not a mobile guide",
            plan.style_guide
        );
    }
}

#[test]
fn directions_spread_across_light_and_dark() {
    let plans = choose_variant_style_guides(WEB_BRIEF, None, 3);
    let modes: Vec<Option<&str>> = plans
        .iter()
        .map(|p| super::mode_of(guide(&p.style_guide)))
        .collect();
    assert!(modes.contains(&Some("light")), "{plans:?}");
    assert!(modes.contains(&Some("dark")), "{plans:?}");
}

#[test]
fn a_pinned_guide_is_direction_a() {
    let plans = choose_variant_style_guides(WEB_BRIEF, Some("zen-paper-light"), 3);
    assert_eq!(plans[0].style_guide, "zen-paper-light");
    assert_eq!(plans[0].style_label, "Zen Paper Light");
    assert!(plans[1..]
        .iter()
        .all(|p| p.style_guide != "zen-paper-light"));
}

#[test]
fn a_guide_the_brief_names_is_direction_a() {
    let brief = format!("{WEB_BRIEF}，用 noir-elegant-dark 风格");
    let plans = choose_variant_style_guides(&brief, None, 3);
    assert_eq!(plans[0].style_guide, "noir-elegant-dark");
}

#[test]
fn a_stale_pin_falls_back_to_ranking() {
    let plans = choose_variant_style_guides(WEB_BRIEF, Some("user:deleted"), 2);
    assert_eq!(plans.len(), 2);
    assert!(plans.iter().all(|p| !p.style_guide.starts_with("user:")));
}

#[test]
fn a_small_shelf_tops_up_from_the_web_shelf() {
    // Four card guides ship; asking for more must not loop or repeat.
    let brief = "请做一套图文卡片（card，竖版 3:4，多页轮播）。用户需求：读书笔记";
    let plans = choose_variant_style_guides(brief, None, 6);
    assert_eq!(plans.len(), 6);
    let card = plans
        .iter()
        .filter(|p| guide(&p.style_guide).platform == Platform::Card)
        .count();
    assert_eq!(card, 4, "the card shelf is exhausted first: {plans:?}");
}

#[test]
fn the_variant_request_pins_its_guide_and_shares_the_worker_budget() {
    let mut base = base_request(WEB_BRIEF);
    base.design_md = Some(jian_ops_schema::DesignMdSpec {
        raw: String::new(),
        project_name: Some("Brand".into()),
        visual_theme: None,
        color_palette: None,
        typography: None,
        component_styles: None,
        layout_principles: None,
        generation_notes: None,
    });
    let plans = choose_variant_style_guides(WEB_BRIEF, None, 3);

    let a = variant_request(&base, &plans[0], 3);
    let b = variant_request(&base, &plans[1], 3);
    assert_eq!(
        a.pinned_style_guide.as_deref(),
        Some(plans[0].style_guide.as_str())
    );
    assert_eq!(
        b.pinned_style_guide.as_deref(),
        Some(plans[1].style_guide.as_str())
    );
    assert!(
        a.design_md.is_some(),
        "direction A keeps the user's design.md"
    );
    assert!(b.design_md.is_none(), "other directions drop it");
    assert_eq!(
        a.concurrency, 2,
        "4 workers over 3 directions round up to 2"
    );
    assert_eq!(a.model, base.model, "every tier runs the same pipeline");
}

#[test]
fn progress_is_scoped_per_direction() {
    let identity = &assign_agent_identities(2)[1];
    let nested = Progress::worker_scoped(
        3,
        "home",
        identity.clone(),
        Progress::SubtaskStarted {
            id: "hero".into(),
            label: "Hero".into(),
        },
    );
    match scope_variant_progress(1, "方案 B · Zen", identity, nested) {
        Progress::WorkerScoped(worker) => {
            assert_eq!(worker.group_idx, 1);
            assert_eq!(worker.screen, "方案 B · Zen");
            match *worker.event {
                Progress::SubtaskStarted { id, label } => {
                    assert_eq!(id, "B-hero");
                    assert_eq!(label, "Hero");
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        other => panic!("unexpected {other:?}"),
    }
    let planned = scope_variant_progress(
        0,
        "A",
        identity,
        Progress::Planned {
            subtasks: vec![("nav".into(), "Nav".into())],
        },
    );
    let Progress::WorkerScoped(worker) = planned else {
        panic!("planned must be scoped");
    };
    assert!(matches!(
        *worker.event,
        Progress::Planned { ref subtasks } if subtasks[0].0 == "A-nav"
    ));
    let failed = Progress::VariantFailed {
        index: 0,
        name: "A".into(),
        error: "x".into(),
    };
    assert!(matches!(
        scope_variant_progress(0, "A", identity, failed),
        Progress::VariantFailed { .. }
    ));
}

#[test]
fn humanized_labels_read_as_names() {
    assert_eq!(
        humanize_style_guide_name("editorial-orange-light"),
        "Editorial Orange Light"
    );
    assert_eq!(humanize_style_guide_name("user:my_brand"), "My Brand");
}

fn frame(id: &str, name: &str, x: f64, y: f64, w: f64, h: f64) -> PenNode {
    serde_json::from_value(json!({
        "type": "frame", "id": id, "name": name,
        "x": x, "y": y, "width": w, "height": h, "children": []
    }))
    .expect("frame fixture")
}

#[test]
fn relayout_orders_directions_by_slot_with_a_gap() {
    // C landed first at the far left, A last at the far right.
    let c = [frame("c1", "C", 0.0, 0.0, 375.0, 812.0)];
    let b = [frame("b1", "B", 615.0, 0.0, 375.0, 812.0)];
    let a = [
        frame("a1", "A1", 1230.0, 40.0, 375.0, 812.0),
        frame("a2", "A2", 1705.0, 40.0, 375.0, 812.0),
    ];
    let groups = vec![
        (2, c.iter().collect::<Vec<_>>()),
        (1, b.iter().collect()),
        (0, a.iter().collect()),
    ];
    let moves = plan_variant_relayout(&groups, 0.0, 0.0, 240.0);
    let at = |id: &str| {
        moves
            .iter()
            .find(|(m, ..)| m == id)
            .map(|(_, x, y)| (*x, *y))
    };
    // A keeps its internal 475 px spacing and moves to the origin.
    assert_eq!(at("a1"), Some((0.0, 0.0)));
    assert_eq!(at("a2"), Some((475.0, 0.0)));
    // A spans 850 px; B starts one gap after it, C after B.
    assert_eq!(at("b1"), Some((1090.0, 0.0)));
    assert_eq!(at("c1"), Some((1705.0, 0.0)));
}

#[test]
fn relayout_skips_roots_already_in_place() {
    let a = [frame("a1", "A", 0.0, 0.0, 100.0, 100.0)];
    let b = [frame("b1", "B", 340.0, 0.0, 100.0, 100.0)];
    let groups = vec![(0, a.iter().collect::<Vec<_>>()), (1, b.iter().collect())];
    assert!(plan_variant_relayout(&groups, 0.0, 0.0, 240.0).is_empty());
}

#[test]
fn board_names_carry_the_direction_prefix() {
    let mut roots = vec![
        frame("a", "Home", 0.0, 0.0, 1.0, 1.0),
        frame("b", "", 0.0, 0.0, 1.0, 1.0),
    ];
    prefix_root_names(&mut roots, "方案 A · Zen · ");
    assert_eq!(roots[0].base().name.as_deref(), Some("方案 A · Zen · Home"));
    assert_eq!(roots[1].base().name.as_deref(), Some("方案 A · Zen"));
}

#[test]
fn merged_roots_carry_their_own_palette() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(
        "brand".to_string(),
        serde_json::from_value(json!({"type": "color", "value": "#ff3366"})).unwrap(),
    );
    assert!(state.apply(EditorCommand::SetVariables {
        variables: vars,
        replace: true,
    }));
    let root: PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "r", "name": "Screen", "x": 0, "y": 0,
        "width": 375, "height": 812,
        "fill": [{"type": "solid", "color": "$brand"}],
        "children": []
    }))
    .unwrap();
    assert!(state.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    }));
    let merged = variant_roots_for_merge(&state);
    assert_eq!(merged.len(), 1);
    assert_eq!(
        op_editor_core::fills::first_solid_fill_hex(&merged[0]),
        Some("#ff3366"),
        "a $variable must be resolved before the board leaves its own document"
    );
}

/// Measured on GLM-5.3-Flash: the binding pass rewrote a direction's
/// mesh-gradient hero vertex to `$--primary`, and the merge let it leave
/// the direction unresolved — on the shared page it would take direction
/// A's palette.
#[test]
fn merged_mesh_vertices_carry_their_own_palette() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(
        "--primary".to_string(),
        serde_json::from_value(json!({"type": "color", "value": "#c4f82a"})).unwrap(),
    );
    assert!(state.apply(EditorCommand::SetVariables {
        variables: vars,
        replace: true,
    }));
    let root: PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "r", "name": "Screen", "x": 0, "y": 0,
        "width": 375, "height": 812,
        "fill": [{"type": "mesh_gradient", "rows": 2, "cols": 2, "stops": [
            {"row": 0, "col": 0, "color": "#e4ff6a"},
            {"row": 0, "col": 1, "color": "$--primary"},
            {"row": 1, "col": 0, "color": "#0a0a0a"},
            {"row": 1, "col": 1, "color": "#0a0a0a"}
        ]}],
        "children": []
    }))
    .unwrap();
    assert!(state.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    }));
    let merged = variant_roots_for_merge(&state);
    let json = serde_json::to_string(&merged[0]).unwrap();
    assert!(!json.contains("$--primary"), "{json}");
    assert!(json.contains("#c4f82a"), "{json}");
}

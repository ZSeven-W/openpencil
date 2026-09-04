use super::*;
use crate::types::DesignRequest;
use serde_json::{json, Value};

fn node(value: Value) -> PenNode {
    serde_json::from_value(value).expect("valid PenNode fixture")
}

fn frame(
    id: &str,
    name: &str,
    width: f64,
    height: f64,
    layout: &str,
    children: Vec<Value>,
) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "name": name,
        "width": width,
        "height": height,
        "layout": layout,
        "children": children,
    })
}

fn text(id: &str, content: &str) -> Value {
    json!({"type":"text", "id":id, "name":"Acme copy", "content":content})
}

fn landing_tree() -> PenNode {
    let nav = frame(
        "nav",
        "Navigation Bar",
        1200.0,
        72.0,
        "horizontal",
        vec![text("nav-label", "Acme")],
    );
    let hero = frame(
        "hero",
        "Hero Section",
        1200.0,
        560.0,
        "horizontal",
        vec![
            frame(
                "hero-copy",
                "Acme Story",
                600.0,
                560.0,
                "vertical",
                vec![text("hero-title", "Acme")],
            ),
            json!({
                "type":"image", "id":"hero-image", "name":"Acme image",
                "width":600, "height":560, "src":""
            }),
        ],
    );
    let cards = (1..=3)
        .map(|index| {
            frame(
                &format!("card-{index}"),
                &format!("Acme Card {index}"),
                380.0,
                240.0,
                "vertical",
                vec![text(&format!("card-text-{index}"), "Acme")],
            )
        })
        .collect();
    let features = frame(
        "features",
        "Features",
        1200.0,
        480.0,
        "vertical",
        vec![frame(
            "feature-row",
            "Feature Row",
            1200.0,
            240.0,
            "horizontal",
            cards,
        )],
    );
    let footer = frame("footer", "Footer", 1200.0, 240.0, "vertical", vec![]);
    node(frame(
        "root",
        "Acme Page",
        1200.0,
        1352.0,
        "vertical",
        vec![nav, hero, features, footer],
    ))
}

#[test]
fn landing_tree_extracts_navigation_hero_heights_and_columns() {
    let skeleton = ReferenceSkeleton::from_root(&landing_tree(), "https://example.com/page?ref=1")
        .expect("non-empty landing tree");

    assert_eq!(skeleton.source, "example.com");
    assert_eq!(skeleton.nav_kind, NavKind::TopBar);
    assert_eq!(skeleton.hero_kind, HeroKind::Split);
    assert_eq!(skeleton.sections.len(), 4);
    for (actual, expected) in skeleton
        .sections
        .iter()
        .map(|section| section.height_ratio)
        .zip([0.05, 0.41, 0.36, 0.18])
    {
        assert!((actual - expected).abs() <= 0.02, "{actual} vs {expected}");
    }
    assert_eq!(skeleton.column_rhythm, vec![3]);

    let rendered = skeleton.render();
    assert!(rendered.lines().count() <= 40);
    assert!(rendered.contains("role=hero"));
    assert!(!rendered.contains("Acme"));
}

#[test]
fn single_full_size_wrapper_is_unwrapped() {
    let wrapper_children = (0..3)
        .map(|index| {
            frame(
                &format!("section-{index}"),
                "Content",
                1200.0,
                200.0,
                "vertical",
                vec![],
            )
        })
        .collect();
    let root = node(frame(
        "root",
        "Page",
        1200.0,
        600.0,
        "vertical",
        vec![frame(
            "wrapper",
            "Content Wrapper",
            1200.0,
            600.0,
            "vertical",
            wrapper_children,
        )],
    ));

    let skeleton = ReferenceSkeleton::from_root(&root, "example.com").expect("unwrapped tree");
    assert_eq!(skeleton.sections.len(), 3);
    assert_eq!(skeleton.sections[0].child_count, 0);
}

#[test]
fn empty_root_has_no_skeleton() {
    let root = node(frame("root", "Empty", 1200.0, 600.0, "vertical", vec![]));
    assert_eq!(ReferenceSkeleton::from_root(&root, "screenshot"), None);
}

#[test]
fn design_request_reference_skeleton_round_trips_and_defaults_when_absent() {
    let skeleton = ReferenceSkeleton {
        source: "screenshot".into(),
        width: 375.0,
        sections: vec![SkeletonSection {
            role: "hero".into(),
            height_ratio: 0.5,
            child_count: 2,
            layout: "horizontal".into(),
            has_image: true,
        }],
        nav_kind: NavKind::BottomTabs,
        hero_kind: HeroKind::Split,
        column_rhythm: vec![2],
    };
    let request = DesignRequest {
        prompt: "x".into(),
        reference_skeleton: Some(skeleton.clone()),
        ..Default::default()
    };
    let json = serde_json::to_string(&request).expect("serialize request");
    let back: DesignRequest = serde_json::from_str(&json).expect("deserialize request");
    assert_eq!(back.reference_skeleton, Some(skeleton));

    let minimal: DesignRequest =
        serde_json::from_str(r#"{"prompt":"x"}"#).expect("minimal request must decode");
    assert_eq!(minimal.prompt, "x");
    assert_eq!(minimal.concurrency, 1);
    assert!(minimal.reference_skeleton.is_none());
}

#[test]
fn from_state_bakes_real_layout_heights_for_fit_content_pages() {
    // An imported HTML page: the root and its sections carry no authored
    // heights, only content. `from_root` would see 0% everywhere.
    let page = node(json!({
        "type": "frame", "id": "page", "name": "Page", "width": 1200.0,
        "layout": "vertical", "children": [
            {"type":"frame","id":"nav","name":"Navigation Bar","width":1200.0,"height":72.0,"layout":"horizontal","children":[text("t1","Acme")]},
            {"type":"frame","id":"hero","name":"Hero Section","width":1200.0,"layout":"vertical","padding":80.0,"children":[
                text("t2","Headline"), text("t3","Sub"), text("t4","CTA")]},
            {"type":"frame","id":"footer","name":"Footer","width":1200.0,"height":200.0,"layout":"vertical","children":[text("t5","© Acme")]}
        ]
    }));
    let mut state = op_editor_core::EditorState::new();
    state.active_children_mut().push(page);
    let skeleton =
        ReferenceSkeleton::from_state(&state, "https://acme.example/x").expect("skeleton");
    assert_eq!(skeleton.sections.len(), 3);
    let hero = &skeleton.sections[1];
    assert!(
        hero.height_ratio > 0.0,
        "hero must get a resolved height: {skeleton:?}"
    );
    let total: f64 = skeleton.sections.iter().map(|s| s.height_ratio).sum();
    assert!((total - 1.0).abs() < 0.05, "ratios sum to ~1: {total}");
    assert_eq!(skeleton.source, "acme.example");
    // The caller's state is untouched.
    assert!(state.active_children()[0].height_px().is_none());
}

#[test]
fn a_dominant_main_wrapper_between_nav_and_footer_is_expanded() {
    let main_children = vec![
        frame(
            "hero",
            "Hero",
            1200.0,
            600.0,
            "vertical",
            vec![text("h", "Headline")],
        ),
        frame(
            "why",
            "Why",
            1200.0,
            500.0,
            "horizontal",
            vec![text("w1", "A"), text("w2", "B"), text("w3", "C")],
        ),
        frame(
            "build",
            "Build",
            1200.0,
            500.0,
            "vertical",
            vec![text("b", "D")],
        ),
    ];
    let root = node(frame(
        "page",
        "Page",
        1200.0,
        1900.0,
        "vertical",
        vec![
            frame(
                "nav",
                "Navigation Bar",
                1200.0,
                72.0,
                "horizontal",
                vec![text("n", "Acme")],
            ),
            frame("main", "Main", 1200.0, 1600.0, "vertical", main_children),
            frame(
                "footer",
                "Footer",
                1200.0,
                228.0,
                "vertical",
                vec![text("f", "©")],
            ),
        ],
    ));
    let skeleton = ReferenceSkeleton::from_root(&root, "acme.example").expect("skeleton");
    let roles: Vec<&str> = skeleton.sections.iter().map(|s| s.role.as_str()).collect();
    assert_eq!(
        skeleton.sections.len(),
        5,
        "nav + 3 main children + footer: {roles:?}"
    );
    assert_eq!(roles[0], "navbar");
    assert_eq!(roles[4], "footer");
    assert!(
        skeleton.column_rhythm.contains(&3),
        "the three-column row survives: {:?}",
        skeleton.column_rhythm
    );
}

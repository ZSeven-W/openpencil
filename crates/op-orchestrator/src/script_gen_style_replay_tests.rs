//! Stage receipts for retained model scripts; never calls a provider.

use jian_ops_schema::node::PenNode;
use op_editor_core::EditorState;
use std::path::PathBuf;

#[test]
fn a_compact_hero_cta_keeps_its_authored_button_surface() {
    let (mut nodes, _) = super::parse_script(r##"
        const hero = I(null, {type:"frame",name:"Fullscreen Hero",width:1440,height:760,layout:"none"});
        const cta = I(hero, {type:"frame",name:"hero-cta",layout:"horizontal",width:"fit_content",height:48,fill:[{type:"solid",color:"#FAFAF8"}]});
        I(cta,{type:"text",content:"探索产品",fill:[{type:"solid",color:"#1A1A1A"}]});
        I(cta,{type:"icon_font",iconFontName:"arrow-right",width:16,height:16});
    "##).unwrap();
    crate::role_infer::resolve_forest_roles(&mut nodes, 1440.0, crate::role_defaults::Theme::Light);
    crate::role_post_pass::post_pass_forest(&mut nodes, 1440.0);
    let value = serde_json::to_value(&nodes).unwrap();
    assert_eq!(value[0]["children"][0]["role"], "button");
    assert_eq!(value[0]["children"][0]["fill"][0]["color"], "#FAFAF8");
}

#[test]
fn an_intrinsically_sized_hero_cta_is_also_a_button() {
    let (mut nodes, _) = super::parse_script(r##"
        const cta = I(null,{type:"frame",name:"hero-cta",layout:"horizontal",height:48,fill:[{type:"solid",color:"#FAF8F5"}]});
        I(cta,{type:"text",content:"立即选购"});
    "##).unwrap();
    crate::role_infer::resolve_forest_roles(&mut nodes, 1440.0, crate::role_defaults::Theme::Light);
    crate::role_post_pass::post_pass_forest(&mut nodes, 1440.0);
    let value = serde_json::to_value(&nodes).unwrap();
    assert_eq!(value[0]["role"], "button");
    assert_eq!(value[0]["fill"][0]["color"], "#FAF8F5");
}

#[test]
fn a_wide_cta_section_keeps_its_section_role() {
    let (mut nodes, _) = super::parse_script(r##"
        const section = I(null,{type:"frame",name:"cta-section",layout:"horizontal",width:1440,height:48});
        I(section,{type:"text",content:"开始探索"});
    "##).unwrap();
    crate::role_infer::resolve_forest_roles(&mut nodes, 1440.0, crate::role_defaults::Theme::Light);
    let value = serde_json::to_value(&nodes).unwrap();
    assert_eq!(value[0]["role"], "cta-section");
}

#[test]
#[ignore = "requires a retained model script and document via QA environment variables"]
fn retained_script_style_stage_receipts() {
    let source = PathBuf::from(std::env::var("OPENPENCIL_STYLE_SCRIPT").unwrap());
    let document = PathBuf::from(std::env::var("OPENPENCIL_STYLE_DOCUMENT").unwrap());
    let output = PathBuf::from(std::env::var("OPENPENCIL_STYLE_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut state = EditorState::new();
    state.doc = serde_json::from_str(&std::fs::read_to_string(document).unwrap()).unwrap();
    let (mut nodes, _) = super::parse_script(&std::fs::read_to_string(source).unwrap()).unwrap();
    let snapshot = |name: &str, nodes: &[PenNode]| {
        std::fs::write(
            output.join(format!("{name}.json")),
            serde_json::to_string_pretty(nodes).unwrap(),
        )
        .unwrap();
    };
    snapshot("01-parsed", &nodes);
    let width = std::env::var("OPENPENCIL_STYLE_WIDTH")
        .unwrap()
        .parse()
        .unwrap();
    let background = std::env::var("OPENPENCIL_STYLE_BACKGROUND").unwrap();
    let theme = if background == "#0A0A0A" {
        crate::role_defaults::Theme::Dark
    } else {
        crate::role_defaults::Theme::Light
    };
    crate::role_infer::resolve_forest_roles(&mut nodes, width, theme);
    snapshot("02-roles", &nodes);
    crate::role_post_pass::post_pass_forest(&mut nodes, width);
    snapshot("03-post-pass", &nodes);
    jian_ops_schema::promote::promote_forest(&mut nodes);
    snapshot("04-promote", &nodes);
    crate::tree_heuristics::apply_tree_heuristics(&mut nodes, Some(&background), theme, None);
    snapshot("05-tree", &nodes);
    crate::variable_binding::bind_generated_color_variables(&mut nodes, &state);
    snapshot("06-binding", &nodes);
    crate::role_post_pass::enforce_surface_color_discipline(&mut nodes);
    snapshot("07-surface", &nodes);
}

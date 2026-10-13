//! Stage receipts for retained model scripts; never calls a provider.

use jian_ops_schema::node::PenNode;
use op_editor_core::EditorState;
use std::path::PathBuf;

#[test]
fn generated_subscription_label_keeps_white_after_adoption_into_dark_button() {
    // Retained GLM response: formRow is a binding string, and the redundant
    // ternary inserts the white label next to the button on a light form.
    let (mut nodes, _) = super::parse_script(r##"
        const band=I(null,{type:"frame",name:"Subscription Band",width:1120,layout:"vertical",fill:[{type:"solid",color:"#FFFFFF"}]});
        const formRow=I(band,{type:"frame",name:"Subscribe Form",layout:"horizontal",width:520,height:48,gap:0,alignItems:"center",fill:[{type:"solid",color:"#F5F4F0"}],stroke:{thickness:1,fill:[{type:"solid",color:"#E5E4E0"}]}});
        I(formRow,{type:"text_input",name:"Email Input",width:"fill_container",height:46,placeholder:"请输入您的邮箱地址",value:"",fill:[{type:"solid",color:"#F5F4F0"}],cornerRadius:0});
        I(formRow,{type:"frame",name:"Subscribe Button",layout:"horizontal",width:120,height:46,alignItems:"center",justifyContent:"center",fill:[{type:"solid",color:"#1A1A1A"}],cornerRadius:0,children:[]});
        I(formRow.children === undefined ? formRow : formRow,{type:"text",name:"Button Label",content:"订阅",fontFamily:"Source Sans 3",fontSize:14,fontWeight:600,letterSpacing:1,fill:[{type:"solid",color:"#FFFFFF"}]});
    "##).unwrap();
    let parsed = serde_json::to_value(&nodes).unwrap();
    assert_eq!(
        parsed[0]["children"][0]["children"][2]["fill"][0]["color"],
        "#FFFFFF"
    );
    crate::role_infer::resolve_forest_roles(&mut nodes, 1440.0, crate::role_defaults::Theme::Light);
    crate::role_post_pass::post_pass_forest(&mut nodes, 1440.0);
    let value = serde_json::to_value(&nodes).unwrap();
    let row = &value[0]["children"][0]["children"];
    assert_eq!(row.as_array().unwrap().len(), 2);
    assert_eq!(row[1]["children"][0]["name"], "Button Label");
    assert_eq!(row[1]["children"][0]["fill"][0]["color"], "#FFFFFF");
    assert_eq!(row[1]["fill"][0]["color"], "#1A1A1A");
}

#[test]
fn generated_low_contrast_text_and_icons_are_still_repaired_after_structure() {
    let (mut nodes, _) = super::parse_script(r##"
        const card = I(null,{type:"frame",name:"Card",width:320,layout:"vertical",fill:[{type:"solid",color:"#FFFFFF"}]});
        const wrapper = I(card,{type:"frame",name:"Transparent row",layout:"horizontal"});
        I(wrapper,{type:"text",name:"Body",content:"Body",fill:[{type:"solid",color:"#EEEEEE"}]});
        I(wrapper,{type:"icon_font",name:"Info",iconFontName:"info",width:20,height:20,fill:[{type:"solid",color:"#EEEEEE"}]});
        const button = I(card,{type:"frame",name:"Subscribe Button",width:120,height:46,layout:"horizontal",fill:[{type:"solid",color:"#17191D"}]});
        I(button,{type:"text",name:"Button Label",content:"订阅",fill:[{type:"solid",color:"#303238"}]});
    "##).unwrap();
    crate::role_infer::resolve_forest_roles(&mut nodes, 1440.0, crate::role_defaults::Theme::Light);
    crate::role_post_pass::post_pass_forest(&mut nodes, 1440.0);
    let value = serde_json::to_value(&nodes).unwrap();
    let children = &value[0]["children"];
    assert_eq!(children[0]["children"][0]["fill"][0]["color"], "#17191D");
    assert_eq!(children[0]["children"][1]["fill"][0]["color"], "#17191D");
    assert_eq!(children[1]["children"][0]["fill"][0]["color"], "#FAF8F3");
}

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

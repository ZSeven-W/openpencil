use crate::{Codegen, Html};

fn source() -> jian_ops_schema::PenDocument {
    jian_ops_schema::load_str(
        r#"{"version":"1.1","children":[
          {"type":"frame","id":"home","screen":"/","x":100,"width":240,"height":180,"children":[
            {"type":"rectangle","id":"card","width":60,"height":40,"animations":[{
              "trigger":"manual","id":"reveal","durationMs":200,
              "keyframes":[{"offset":0,"opacity":0.4},{"offset":1,"opacity":1}]}]},
            {"type":"rectangle","id":"trigger","width":60,"height":40,"events":{"onTap":[
              {"play_motion":{"target":"card","animation":"reveal","mode":"restart"}}]}},
            {"type":"rectangle","id":"detail-button","width":60,"height":40,"events":{"onTap":[
              {"push":{"path":"'/detail'","transition":{"kind":"slideUp","durationMs":320}}}]}}
          ]},
          {"type":"frame","id":"detail","screen":"/detail","x":400,"width":240,"height":180,"children":[
            {"type":"rectangle","id":"back","width":60,"height":40,"events":{"onTap":[{"pop":null}]}}
          ]}
        ]}"#,
    )
    .unwrap()
    .value
}

#[test]
fn html_wires_manual_motion_and_routes_as_data() {
    let html = Html.generate(&source());
    assert!(html.contains("data-op-stage "));
    assert!(html.contains("data-op-screen=\"/detail\" hidden inert"));
    assert!(html.contains("data-op-target=\"card\""));
    assert!(html.contains(r#""animation":"reveal""#));
    assert!(html.contains(r#""path":[[3,"/detail"],[32]]"#));
    assert!(html.contains("role=\"button\" tabindex=\"0\""));
    assert!(html.contains("window.opPlayMotion?.(target.element, action.animation, action.mode)"));
    assert!(html.contains("\"duration\":320"));
    assert!(html.contains("\"outgoing_on_top\":true"));
}

fn program(html: &str) -> serde_json::Value {
    let (_, block) = html.split_once(r#"id="op-prototype">"#).unwrap();
    serde_json::from_str(block.split_once("</script>").unwrap().0).unwrap()
}

#[test]
fn dynamic_routes_and_nested_chains_compile_as_data() {
    let doc = jian_ops_schema::load_str(include_str!(
        "../../op-preview-core/tests/fixtures/motion/html_dynamic_prototype.op"
    ))
    .unwrap()
    .value;
    let html = Html.generate(&doc);
    let wire = program(&html);
    assert!(html.contains(r#"data-op-screen="/alternate" hidden inert"#));
    let actions = &wire["nodes"]["launch"]["events"]["onTap"];
    assert_eq!(actions[0]["op"], "toggle");
    assert_eq!(actions[1]["then"][1]["op"], "parallel");
    assert_eq!(actions[1]["then"][1]["branches"][1][0]["ms"], 128);
    assert_eq!(actions[1]["then"][3]["then"][0]["op"], "push");
    assert_eq!(
        wire["nodes"]["next"]["events"]["onTap"][3]["then"][0]["op"],
        "replace"
    );
}

#[test]
fn authored_script_markup_stays_data_and_disallowed_actions_skip() {
    let mut value = serde_json::to_value(source()).unwrap();
    value["children"][0]["children"][1]["events"]["onTap"] = serde_json::json!([
        {"fetch":{"url":"'/external'"}},
        {"set":{"$app.label":"'x</script><script>alert(1)</script>'"}},
        {"push":"$app.nextScreen"}
    ]);
    let doc = jian_ops_schema::load_str(&value.to_string()).unwrap().value;
    let html = Html.generate(&doc);
    assert!(!html.contains("x</script>"));
    assert_eq!(
        program(&html)["nodes"]["trigger"]["events"]["onTap"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

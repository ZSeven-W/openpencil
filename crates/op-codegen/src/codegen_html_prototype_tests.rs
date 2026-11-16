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
    assert!(html.contains("&quot;animation&quot;:&quot;reveal&quot;"));
    assert!(html.contains("&quot;path&quot;:&quot;/detail&quot;"));
    assert!(html.contains("role=\"button\" tabindex=\"0\""));
    assert!(html.contains("window.opPlayMotion?.(target, action.animation, action.mode)"));
    assert!(html.contains("\"duration\":320"));
    assert!(html.contains("\"outgoing_on_top\":true"));
}

#[test]
fn dynamic_routes_and_unsupported_tap_chains_are_not_executed() {
    let source = serde_json::to_value(source()).unwrap();
    let mut source = source;
    source["children"][0]["children"][2]["events"]["onTap"] =
        serde_json::json!([{"push":"$app.nextScreen"}]);
    source["children"][0]["children"][1]["events"]["onTap"] = serde_json::json!([
        {"set":{"$app.isOpen":true}},
        {"play_motion":{"target":"card","animation":"reveal"}}
    ]);
    let doc = jian_ops_schema::load_str(&source.to_string())
        .unwrap()
        .value;
    let html = Html.generate(&doc);
    assert!(!html.contains("data-op-stage "));
    assert!(!html.contains("data-op-target="));
    assert!(!html.contains("id=\"op-screen-motion\""));
    assert!(!html.contains("$app.nextScreen"));
}

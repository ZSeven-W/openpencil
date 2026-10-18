use super::*;
use crate::plan::OrchestratorPlan;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest};
use futures::executor::block_on;
use op_editor_core::EditorState;
use serde_json::json;

// Exact successful 1018 model retry: the icon and circular backdrop are siblings.
const RETAINED_SCRIPT: &str = r###"const sec = I(null, {type:"frame", name:"购物车条", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,16,12,16]});

const bar = I(sec, {type:"frame", name:"cart-bar-pill", width:"fill_container", height:56, layout:"horizontal", alignItems:"center", fill:[{type:"solid", color:"#21140F"}], cornerRadius:28, padding:[0,6,0,0]});

// 购物车圆盘（带数量角标）
const discWrap = I(bar, {type:"frame", name:"cart-disc-wrap", width:"fit_content", height:"fit_content", layout:"none"});
I(discWrap, {type:"frame", name:"cart-disc", width:48, height:48, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:24, fill:[{type:"solid", color:"#FF5A1F"}], children:[]});
I(discWrap, {type:"icon_font", name:"cart-icon", iconFontName:"shopping-cart", iconFontFamily:"lucide", width:22, height:22, fill:[{type:"solid", color:"#FFFFFF"}]});
// center icon over disc: use none-layout stack order — children[0] topmost
// rebuild properly:
// (discWrap children: badge first, then disc+icon frame as bottom layer)

const badge = I(discWrap, {type:"frame", name:"cart-count-badge", width:18, height:18, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:9, fill:[{type:"solid", color:"#FFFFFF"}], x:32, y:-2});
I(badge, {type:"text", name:"cart-count", content:"3", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:11, fontWeight:700, fill:[{type:"solid", color:"#FF5A1F"}], width:"fit_content", textGrowth:"auto"});

const totalGroup = I(bar, {type:"frame", name:"total-group", width:"fill_container", height:"fit_content", layout:"horizontal", alignItems:"center", gap:16, padding:[0,12,0,12]});

const totalCol = I(totalGroup, {type:"frame", name:"total-spacer", width:"fit_content", height:"fit_content", layout:"vertical", gap:2, alignItems:"start"});
I(totalCol, {type:"text", name:"cart-total", content:"¥58", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:20, fontWeight:700, fill:[{type:"solid", color:"#FFFFFF"}], width:"fit_content", textGrowth:"auto", letterSpacing:-0.3});
I(totalCol, {type:"text", name:"cart-fee-note", content:"另需配送费", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:11, fontWeight:400, fill:[{type:"solid", color:"#D7C6B8"}], width:"fit_content", textGrowth:"auto"});

const checkoutButton = I(totalGroup, {type:"frame", name:"checkout-button", width:112, height:44, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:22, fill:[{type:"solid", color:"#FF5A1F"}], events:{onTap:[{toast:"去结算"}]}});
I(checkoutButton, {type:"text", name:"checkout-label", content:"去结算", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}], width:"fit_content", textGrowth:"auto"});"###;

fn circle(size: f64) -> Value {
    json!({"type":"frame","name":"Round backdrop","width":size,"height":size,
        "cornerRadius":size / 2.0,"fill":[{"type":"solid","color":"#FF5A1F"}],"children":[]})
}

fn decorated_parent(child: Value) -> Value {
    json!({"type":"frame","cornerRadius":28,"fill":[{"type":"solid","color":"#21140F"}],
        "stroke":{"thickness":1},"effects":[{"type":"shadow","blur":8}],"children":[child]})
}

fn named<'a>(value: &'a Value, name: &str) -> &'a Value {
    fn find<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
        if value.get("name").and_then(Value::as_str) == Some(name) {
            return Some(value);
        }
        value
            .get("children")?
            .as_array()?
            .iter()
            .find_map(|child| find(child, name))
    }
    find(value, name).unwrap_or_else(|| panic!("missing {name}"))
}

#[test]
fn compact_circle_backdrops_keep_uniform_and_per_corner_geometry() {
    for size in [16.0, 40.0, 48.0, 64.0, 80.0] {
        for radius in [
            json!(size / 2.0),
            json!(999),
            json!([size / 2.0, size / 2.0, size / 2.0, size / 2.0]),
        ] {
            let mut shape = circle(size);
            shape["cornerRadius"] = radius;
            let expected = shape.clone();
            let mut root = decorated_parent(shape);
            strip_nested_card_decoration(&mut root, DecoFlags::default());
            assert_eq!(root["children"][0], expected, "{size}");
            let once = root.clone();
            strip_nested_card_decoration(&mut root, DecoFlags::default());
            assert_eq!(root, once);
        }
    }
}

#[test]
fn ordinary_empty_frames_and_nested_content_do_not_gain_circle_protection() {
    for case in [
        "rectangle",
        "large",
        "missing size",
        "fluid size",
        "zero size",
        "negative size",
        "small radius",
        "partial corners",
        "content",
        "no fill",
        "empty fill",
        "transparent",
        "transparent shorthand",
        "fill opacity",
        "invalid color",
    ] {
        let mut shape = circle(48.0);
        match case {
            "rectangle" => shape["width"] = json!(64),
            "large" => {
                shape["width"] = json!(100);
                shape["height"] = json!(100);
                shape["cornerRadius"] = json!(50);
            }
            "missing size" => {
                shape.as_object_mut().unwrap().remove("width");
            }
            "fluid size" => shape["height"] = json!("fit_content"),
            "zero size" => {
                shape["width"] = json!(0);
                shape["height"] = json!(0);
            }
            "negative size" => {
                shape["width"] = json!(-48);
                shape["height"] = json!(-48);
            }
            "small radius" => shape["cornerRadius"] = json!(12),
            "partial corners" => shape["cornerRadius"] = json!([24, 24, 0, 24]),
            "content" => shape["children"] = json!([{"type":"text","content":"Nested content"}]),
            "no fill" => {
                shape.as_object_mut().unwrap().remove("fill");
            }
            "empty fill" => shape["fill"] = json!([]),
            "transparent" => shape["fill"][0]["color"] = json!("#FF5A1F00"),
            "transparent shorthand" => shape["fill"][0]["color"] = json!("#F000"),
            "fill opacity" => shape["fill"][0]["opacity"] = json!(0),
            "invalid color" => shape["fill"][0]["color"] = json!("unknown-color"),
            _ => unreachable!(),
        }
        let mut root = decorated_parent(shape);
        strip_nested_card_decoration(&mut root, DecoFlags::default());
        assert!(root["children"][0].get("cornerRadius").is_none(), "{case}");
    }
}

#[test]
fn circle_protection_does_not_keep_redundant_strokes_or_shadows() {
    let mut shape = circle(48.0);
    shape["fill"][0]["color"] = json!("$--primary");
    shape["stroke"] = json!({"thickness":1});
    shape["effects"] = json!([{"type":"shadow","blur":4}]);
    let mut root = decorated_parent(shape);
    strip_nested_card_decoration(&mut root, DecoFlags::default());
    let shape = &root["children"][0];
    assert_eq!(shape["cornerRadius"], 24.0);
    assert!(shape.get("stroke").is_none());
    assert!(shape.get("effects").is_none());
}

#[test]
fn retained_circle_and_sibling_icon_survive_subtask_cleanup_and_save_reload() {
    let error = crate::script_gen::parse_script(RETAINED_SCRIPT).unwrap_err();
    assert!(matches!(
        &error,
        crate::script_gen::ScriptGenError::RuntimeExpression(_)
    ));
    assert!(error.to_string().contains("events.onTap[0].toast"));
    // Preserve the exact response above; only quote its toast literal for the
    // independent geometry and save/reload regression below.
    let corrected = RETAINED_SCRIPT.replace(r#"toast:"去结算""#, r#"toast:"'去结算'""#);
    let (parsed, _) = crate::script_gen::parse_script(&corrected).unwrap();
    let parsed = serde_json::to_value(&parsed[0]).unwrap();
    assert_eq!(named(&parsed, "cart-disc")["cornerRadius"], 24.0);
    assert!(named(&parsed, "cart-disc")["children"]
        .as_array()
        .is_none_or(Vec::is_empty));
    assert!(named(&parsed, "cart-icon").get("x").is_none());
    assert!(named(&parsed, "cart-icon").get("y").is_none());
    let mut plan: OrchestratorPlan = serde_json::from_value(json!({
        "rootFrame":{"id":"page","name":"Detail","width":375,"height":812,"layout":"vertical",
            "fill":[{"type":"solid","color":"#FFFCF6"}]},
        "sharedContext":{},"subtasks":[{"id":"action","label":"购物车条",
            "elements":"底部购物车条，固定于底部，右侧去结算按钮","region":{"width":375,"height":68}}]
    })).unwrap();
    plan.subtasks[0].parent_frame_id = Some("page".into());
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(corrected)]);
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(serde_json::from_value(json!({
        "version":"1.0.0","children":[{
            "type":"frame","id":"page","name":"Detail","width":375,"height":812,
            "layout":"vertical","gap":16,"fill":[{"type":"solid","color":"#FFFCF6"}],
            "children":[
                {"type":"frame","id":"status","role":"status-bar","name":"Status Bar","width":"fill_container","height":62},
                {"type":"frame","id":"body","name":"Content","width":"fill_container","height":1000,"layout":"vertical","children":[
                    {"type":"text","id":"body-copy","content":"Menu list","fontSize":16,"height":1000}
                ]}
            ]
        }]
    })).unwrap());
    let result = block_on(crate::subagent::run_subtask(
        &plan.subtasks[0],
        &plan,
        &DesignRequest::default(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert!(result.error.is_none(), "{:?}", result.error);
    let inserted = serde_json::to_value(&sink.state.doc).unwrap();
    assert_eq!(named(&inserted, "cart-disc")["cornerRadius"], 24.0);
    let diagnostics = crate::geometry_validation::geometry_diagnostics(&sink.state);
    assert!(
        !diagnostics
            .iter()
            .any(|line| line.contains("cart-disc") && line.contains("empty decorated frame")),
        "{diagnostics:?}"
    );
    crate::cleanup::finalize_design_with_summary_and_policy(
        &mut sink,
        &plan,
        &["page"],
        &mut crate::repair_summary::RepairSummary::default(),
        crate::cleanup::CleanupPolicy {
            preserve_requested_root_height: true,
            roots_are_run_output: true,
            is_deck: false,
        },
    );
    let stored = serde_json::to_string(&sink.state.doc).unwrap();
    let loaded: jian_ops_schema::PenDocument = serde_json::from_str(&stored).unwrap();
    let loaded = serde_json::to_value(&loaded).unwrap();
    assert_eq!(named(&loaded, "cart-disc")["cornerRadius"], 24.0);
    assert_eq!(named(&loaded, "cart-disc")["width"], 48.0);
    assert_eq!(named(&loaded, "cart-disc")["height"], 48.0);
    // This change preserves shape only; the model's missing icon coordinates
    // remain visible to their separate geometry check.
    assert!(named(&loaded, "cart-icon").get("x").is_none());
    assert!(named(&loaded, "cart-icon").get("y").is_none());
}

//! Weak-model dialect repairs (`batch_program_dialect*.rs`). Every body below
//! is a line the executor DROPPED in the design-arena-v1 corpus
//! (`[program-gen] dropped line` in run stderr, 2026-09-26/27). Previews
//! the log truncated at 200 chars are completed with the smallest tail that
//! reproduces the logged error; the run each came from is named.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, NodeId};
use serde_json::Value;

use super::batch_program_test_support::{binding_id, call_operations, call_operations_best_effort};
use super::test_fixtures::sample;

const ROOT: &str = "b0=I(null, {\"type\":\"frame\",\"name\":\"screen\",\"width\":1440,\"height\":900,\"layout\":\"vertical\"})\n";

fn run(program: &str) -> (EditorState, Value) {
    let mut state = sample();
    let (envelope, command) = call_operations_best_effort(&state, &format!("{ROOT}{program}"));
    if let Some(command) = command {
        assert!(state.apply(command), "{envelope}");
    }
    (state, envelope)
}

fn json_of(state: &EditorState, envelope: &Value, binding: &str) -> Value {
    let id = binding_id(envelope, binding);
    let node: &PenNode =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&id))
            .unwrap_or_else(|| panic!("{binding} missing: {envelope}"));
    serde_json::to_value(node).expect("node json")
}

fn errors(envelope: &Value) -> Vec<String> {
    envelope["errors"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|e| e["error"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn warned(envelope: &Value, needle: &str) -> bool {
    envelope["warnings"].as_array().is_some_and(|list| {
        list.iter()
            .any(|w| w["warning"].as_str().is_some_and(|t| t.contains(needle)))
    })
}

#[test]
fn table_family_becomes_frames_and_the_table_keeps_its_rows() {
    // glm-5-3-flash-0926d arena-d01 run-2: one `table` line failed and 8 rows
    // x 6 cells cascaded (63 dropped lines, 54 of them `table-cell`).
    let program = r##"b12=I(b0, {"type":"table","name":"ops-table","layout":"vertical","width":"fill_container","height":"fit_content","gap":0,"clipContent":true})
b13=I(b12, {"type":"table-header","name":"table-head-row","layout":"horizontal","width":"fill_container","height":"fit_content","alignItems":"center","padding":[10,20],"fill":[{"type":"solid","color":"#F8FAFC"}]})
b14=I(b13, {"type":"table-cell","layout":"horizontal","width":130,"height":"fit_content","justifyContent":"start","alignItems":"center"})
b15=I(b14, {"type":"text","content":"Order","fontSize":12})
b26=I(b12, {"type":"table-row","name":"row-0","layout":"horizontal","width":"fill_container","height":"fit_content","alignItems":"center","padding":[12,20],"fill":[{"type":"solid","color":"#FFFFFF"}]})
b27=I(b26, {"type":"table-cell","layout":"horizontal","width":130,"height":"fit_content","justifyContent":"start","alignItems":"center","gap":6})
b28=I(b27, {"type":"text","content":"#10231","fontSize":13})"##;
    let (state, envelope) = run(program);
    assert!(errors(&envelope).is_empty(), "{envelope}");
    let table = json_of(&state, &envelope, "b12");
    assert_eq!(table["type"], "frame");
    assert_eq!(table["layout"], "vertical");
    let rows = table["children"].as_array().expect("rows");
    assert_eq!(rows.len(), 2, "header + row survive under the table");
    for row in rows {
        assert_eq!(row["type"], "frame");
        assert_eq!(row["layout"], "horizontal");
        let cell = &row["children"][0];
        assert_eq!(cell["type"], "frame");
        assert_eq!(cell["children"][0]["type"], "text");
    }
    assert!(warned(&envelope, "rewrote type \"table-cell\" as frame"));
}

#[test]
fn table_cell_with_inline_content_gets_a_text_child() {
    let (state, envelope) = run(
        "c=I(b0, {\"type\":\"table-cell\",\"width\":120,\"content\":\"Paid\",\"fontSize\":13,\"color\":\"#16A34A\"})",
    );
    assert!(errors(&envelope).is_empty(), "{envelope}");
    let cell = json_of(&state, &envelope, "c");
    assert_eq!(cell["type"], "frame");
    assert_eq!(cell["layout"], "horizontal");
    let text = &cell["children"][0];
    assert_eq!(
        (text["type"].as_str(), text["content"].as_str()),
        (Some("text"), Some("Paid"))
    );
    assert_eq!(text["fontSize"], 13.0);
    assert!(
        cell.get("fontSize").is_none(),
        "typography moved off the frame"
    );
}

#[test]
fn spacer_becomes_an_unpainted_frame_with_its_size() {
    // glm-5-3-flash-0926d arena-l02 run-2 (3x) and 0926b arena-d03 run-1.
    let (state, envelope) = run(concat!(
        "s1=I(b0, {\"type\":\"spacer\",\"width\":\"fill_container\",\"height\":28})\n",
        "s2=I(b0, {\"type\":\"spacer\",\"name\":\"flex-spacer\",\"width\":\"fill_container\",\"height\":\"fill_container\"})",
    ));
    assert!(errors(&envelope).is_empty(), "{envelope}");
    let spacer = json_of(&state, &envelope, "s1");
    assert_eq!(spacer["type"], "frame");
    assert_eq!(spacer["height"], 28.0);
    assert!(spacer.get("fill").is_none());
    assert_eq!(json_of(&state, &envelope, "s2")["type"], "frame");
}

#[test]
fn divider_becomes_a_one_pixel_rectangle() {
    // glm-5-3-flash-0926c arena-l02 run-2 (fill given) and deepseek-v4-pro
    // arena-d01 run-1; the stroke-only form is the same intent spelled with
    // a border.
    let (state, envelope) = run(concat!(
        "d1=I(b0, {\"type\":\"divider\",\"name\":\"Footer Divider\",\"width\":\"fill_container\",\"height\":1,\"fill\":[{\"type\":\"solid\",\"color\":\"#1C1C1F\"}]})\n",
        "d2=I(b0, {\"type\":\"divider\",\"name\":\"分隔线\",\"role\":\"divider\",\"width\":\"fill_container\",\"height\":1,\"layout\":\"none\",\"fill\":[{\"type\":\"solid\",\"color\":\"#1F2937\"}]})\n",
        "d3=I(b0, {\"type\":\"divider\",\"stroke\":{\"thickness\":1,\"color\":\"#E5E7EB\"}})\n",
        "d4=I(b0, {\"type\":\"divider\",\"orientation\":\"vertical\",\"color\":\"#E5E7EB\"})",
    ));
    assert!(errors(&envelope).is_empty(), "{envelope}");
    for binding in ["d1", "d2"] {
        assert_eq!(json_of(&state, &envelope, binding)["type"], "rectangle");
    }
    let from_stroke = json_of(&state, &envelope, "d3");
    assert_eq!(from_stroke["width"], "fill_container");
    assert_eq!(from_stroke["height"], 1.0);
    assert_eq!(from_stroke["fill"][0]["color"], "#E5E7EB");
    let vertical = json_of(&state, &envelope, "d4");
    assert_eq!(
        (vertical["width"].clone(), vertical["height"].clone()),
        (serde_json::json!(1.0), serde_json::json!("fill_container"))
    );
}

#[test]
fn controls_become_centred_frames_with_icon_and_label_children() {
    // glm-5-3-flash-0926c arena-d02 run-1 (icon-button), 0926d arena-d01
    // run-2 (button) and 0926b arena-d02 run-2 (badge); tails completed.
    let (state, envelope) = run(concat!(
        "ib=I(b0, {\"type\":\"icon-button\",\"name\":\"notifications-button\",\"width\":36,\"height\":36,\"layout\":\"horizontal\",\"justifyContent\":\"center\",\"alignItems\":\"center\",\"cornerRadius\":6,\"fill\":[{\"type\":\"solid\",\"color\":\"#F1F5F9\"}],\"iconFontName\":\"bell\",\"iconFontFamily\":\"lucide\"})\n",
        "bt=I(b0, {\"type\":\"button\",\"name\":\"export-btn\",\"layout\":\"horizontal\",\"padding\":[9,16],\"height\":38,\"cornerRadius\":6,\"gap\":8,\"alignItems\":\"center\",\"justifyContent\":\"center\",\"fill\":[{\"type\":\"solid\",\"color\":\"#0F172A\"}],\"content\":\"Export\",\"fontSize\":13,\"color\":\"#FFFFFF\"})\n",
        "bd=I(b0, {\"type\":\"badge\",\"name\":\"Count Badge\",\"content\":\"84\",\"fontFamily\":\"Inter\",\"fontSize\":10,\"fontWeight\":500,\"fill\":[{\"type\":\"solid\",\"color\":\"#1D4ED820\"}],\"padding\":[3,8],\"cornerRadius\":999})",
    ));
    assert!(errors(&envelope).is_empty(), "{envelope}");
    let icon_button = json_of(&state, &envelope, "ib");
    assert_eq!(icon_button["type"], "frame");
    assert_eq!(icon_button["children"][0]["type"], "icon_font");
    assert_eq!(icon_button["children"][0]["iconFontName"], "bell");
    let button = json_of(&state, &envelope, "bt");
    assert_eq!(button["children"][0]["content"], "Export");
    assert_eq!(button["children"][0]["fill"][0]["color"], "#FFFFFF");
    assert_eq!(button["fill"][0]["color"], "#0F172A", "background stays");
    let badge = json_of(&state, &envelope, "bd");
    assert_eq!(badge["type"], "frame");
    assert_eq!(badge["justifyContent"], "center");
    assert_eq!(badge["children"][0]["content"], "84");
}

#[test]
fn missing_type_is_inferred_only_from_a_telling_field() {
    // deepseek-v4-pro arena-l02 run-1: two headings without `type`; the
    // same runs' `I(bN, {})` bodies carry nothing to infer from.
    let (state, envelope) = run(concat!(
        "b3=I(b0, {\"content\":\"Pricing Tiers\",\"fontSize\":36,\"fontWeight\":600,\"fill\":[{\"type\":\"solid\",\"color\":\"#FAFAFA\"}],\"fontFamily\":\"Geist\",\"letterSpacing\":-1})\n",
        "b6=I(b0, {})",
    ));
    assert_eq!(json_of(&state, &envelope, "b3")["type"], "text");
    assert_eq!(
        errors(&envelope),
        vec!["invalid PenNode payload: missing field `type`".to_string()],
        "an empty body keeps its unchanged diagnostic"
    );
}

#[test]
fn field_dialects_are_read_as_their_schema_spelling() {
    // cornerRadius object: glm-5-3-flash-0926b arena-d01 run-1 (10 bars);
    // textAlignVertical center: deepseek-v4-pro arena-m03 run-1; leadingIcon
    // object: glm-5-3-flash-0926b arena-d02; events list: 0926b arena-m01
    // run-1; press trigger: space-bunny-alpha arena-m04 run-1; colourless
    // solid fill: the chip dots/labels of the same corpus.
    let (state, envelope) = run(concat!(
        "bar=I(b0, {\"type\":\"rectangle\",\"name\":\"本月柱\",\"width\":14,\"height\":132,\"cornerRadius\":{\"topLeft\":3,\"topRight\":3},\"fill\":[{\"type\":\"solid\",\"color\":\"#334155\"}]})\n",
        "lbl=I(b0, {\"type\":\"text\",\"name\":\"首页 Label\",\"content\":\"首页\",\"fontSize\":10,\"textAlign\":\"center\",\"textAlignVertical\":\"center\"})\n",
        "srch=I(b0, {\"type\":\"text_input\",\"name\":\"global-search\",\"placeholder\":\"Search\",\"value\":\"\",\"width\":320,\"height\":40,\"leadingIcon\":{\"iconFontName\":\"search\",\"width\":16,\"height\":16}})\n",
        "tab=I(b0, {\"type\":\"frame\",\"name\":\"tab-首页\",\"layout\":\"vertical\",\"events\":[]})\n",
        "cta=I(b0, {\"type\":\"frame\",\"name\":\"Sign Up\",\"layout\":\"horizontal\",\"animations\":[{\"trigger\":\"press\",\"keyframes\":{\"from\":{\"scale\":1},\"to\":{\"scale\":0.97}},\"durationMs\":120}]})\n",
        "dot=I(b0, {\"type\":\"ellipse\",\"name\":\"dot\",\"width\":6,\"height\":6,\"fill\":[{\"type\":\"solid\"}]})",
    ));
    assert!(errors(&envelope).is_empty(), "{envelope}");
    assert_eq!(
        json_of(&state, &envelope, "bar")["cornerRadius"],
        serde_json::json!([3.0, 3.0, 0.0, 0.0])
    );
    assert_eq!(
        json_of(&state, &envelope, "lbl")["textAlignVertical"],
        "middle"
    );
    assert_eq!(json_of(&state, &envelope, "srch")["leadingIcon"], "search");
    assert!(json_of(&state, &envelope, "tab").get("events").is_none());
    assert!(json_of(&state, &envelope, "cta")
        .get("animations")
        .is_none());
    assert!(json_of(&state, &envelope, "dot").get("fill").is_none());
    assert!(warned(&envelope, "unsupported trigger"));
}

#[test]
fn events_list_merges_into_one_handler_map() {
    let (state, envelope) = run(
        "t=I(b0, {\"type\":\"frame\",\"name\":\"tab-订单\",\"layout\":\"vertical\",\"events\":[{\"onTap\":[{\"push\":\"/orders\"}]}]})",
    );
    assert!(errors(&envelope).is_empty(), "{envelope}");
    assert!(json_of(&state, &envelope, "t")["events"]["onTap"].is_array());
}

#[test]
fn update_patches_get_the_field_repairs_too() {
    let (state, envelope) = run(concat!(
        "r=I(b0, {\"type\":\"rectangle\",\"width\":14,\"height\":90})\n",
        "U(r, {\"cornerRadius\":\"6px\"})",
    ));
    assert!(errors(&envelope).is_empty(), "{envelope}");
    assert_eq!(json_of(&state, &envelope, "r")["cornerRadius"], 6.0);
}

#[test]
fn null_children_entries_are_dropped_not_the_node() {
    // glm-5-3-flash-0926b arena-m02 run-2: an avatar frame whose inline
    // children held a `null`.
    let (state, envelope) = run(
        "av=I(b0, {\"type\":\"frame\",\"name\":\"头像\",\"width\":44,\"height\":44,\"children\":[null,{\"type\":\"text\",\"content\":\"林\"}]})",
    );
    assert!(errors(&envelope).is_empty(), "{envelope}");
    let avatar = json_of(&state, &envelope, "av");
    assert_eq!(avatar["children"].as_array().map(Vec::len), Some(1));
}

#[test]
fn field_salvage_is_best_effort_only_and_protects_identity() {
    // space-bunny-alpha arena-d02 run-1: the "scrollable users table" frame
    // failed with `invalid type: map, expected a sequence` in a field the log
    // truncated, and 205 lines cascaded. A map in a list-typed optional field
    // reproduces it.
    let body = "t=I(b0, {\"type\":\"frame\",\"name\":\"scrollable users table\",\"width\":\"fill_container\",\"height\":154,\"layout\":\"vertical\",\"clipContent\":true,\"slot\":{\"header\":\"b2\"}})\nh=I(t, {\"type\":\"frame\",\"name\":\"table-header\",\"layout\":\"horizontal\"})";

    let (state, envelope) = run(body);
    assert!(errors(&envelope).is_empty(), "{envelope}");
    assert!(warned(&envelope, "dropped field `slot`"), "{envelope}");
    let table = json_of(&state, &envelope, "t");
    assert_eq!(table["children"][0]["name"], "table-header");

    // The agent-facing transactional surface keeps rejecting it verbatim:
    // a model in the loop can fix its own payload.
    let (strict, command) = call_operations(&sample(), &format!("{ROOT}{body}"));
    assert!(command.is_none(), "{strict}");
    assert!(
        errors(&strict)[0].contains("invalid type: map, expected a sequence"),
        "{strict}"
    );

    // A value the schema rejects in a protected field is not salvaged.
    let (_, envelope) = run("x=I(b0, {\"type\":\"text\",\"content\":{\"zh\":\"林\"}})");
    assert_eq!(errors(&envelope).len(), 1, "{envelope}");
}

#[test]
fn genuinely_unknown_types_keep_the_unchanged_diagnostic() {
    // glm-5-3-flash-0926c arena-w01 run-1.
    let (_, envelope) = run("b8=I(b0, {\"type\":\"effects\"})");
    let errors = errors(&envelope);
    assert_eq!(errors.len(), 1, "{envelope}");
    assert!(
        errors[0].starts_with("invalid PenNode payload: unknown variant `effects`"),
        "{errors:?}"
    );
}

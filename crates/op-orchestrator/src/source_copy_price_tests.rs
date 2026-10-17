//! Product/price copy remains paired when a row includes a secondary note.
use super::*;
use serde_json::{json, Value};

fn row(name: &str, note: &str, price: &str) -> Value {
    json!({"type":"frame","layout":"horizontal","children":[
        {"type":"frame","layout":"vertical","children":[
            {"type":"text","content":name},
            {"type":"text","content":note}
        ]},
        {"type":"text","content":price}
    ]})
}

fn nodes(rows: Vec<Value>) -> Vec<PenNode> {
    crate::script_gen::parse_script(&format!(
        "I(null, {});",
        json!({"type":"frame","layout":"vertical","children":rows})
    ))
    .unwrap()
    .0
}

fn contract(lines: &str) -> SourceCopy {
    SourceCopy::from_brief(&format!("Keep all following text verbatim:\n{lines}"))
}

#[test]
fn a_product_note_does_not_erase_an_editable_same_row_price() {
    let nodes = nodes(vec![
        row("拿铁", "", "28元"),
        row("美式", "", "22元"),
        row("燕麦拿铁", "含植物奶", "32元"),
    ]);
    assert!(contract("拿铁 28元\n美式 22元\n燕麦拿铁 32元")
        .missing(&EditorState::new(), &nodes)
        .is_empty());
}

#[test]
fn annotated_prices_must_not_be_borrowed_from_other_rows_or_products() {
    let state = EditorState::new();
    let wrong = nodes(vec![
        row("燕麦拿铁", "含植物奶", "28元"),
        row("美式", "热饮", "32元"),
    ]);
    assert_eq!(
        contract("燕麦拿铁 32元").missing(&state, &wrong),
        ["燕麦拿铁 32元"]
    );
    let longer = nodes(vec![row("燕麦拿铁", "含植物奶", "32元")]);
    assert_eq!(
        contract("拿铁 32元").missing(&state, &longer),
        ["拿铁 32元"]
    );
}

#[test]
fn annotated_price_fields_are_consumed_once_and_hidden_prices_do_not_count() {
    let state = EditorState::new();
    let one = nodes(vec![row("燕麦拿铁", "含植物奶", "32元")]);
    assert_eq!(
        contract("燕麦拿铁 32元\n燕麦拿铁 32元").missing(&state, &one),
        ["燕麦拿铁 32元"]
    );
    let mut hidden = row("燕麦拿铁", "含植物奶", "32元");
    hidden["children"][1]["opacity"] = json!(0);
    assert_eq!(
        contract("燕麦拿铁 32元").missing(&state, &nodes(vec![hidden])),
        ["燕麦拿铁 32元"]
    );
}

#[test]
fn annotations_do_not_allow_rewriting_supplied_prose() {
    let prose = nodes(vec![row("到店自取，", "很快做好", "不含配送。")]);
    assert_eq!(
        contract("到店自取，不含配送。").missing(&EditorState::new(), &prose),
        ["到店自取，不含配送。"]
    );
}

#[test]
#[ignore = "set OPENPENCIL_PRICE_COPY_REPLAY to the retained generation script"]
fn replay_retained_annotated_drink_list() {
    let path = std::env::var("OPENPENCIL_PRICE_COPY_REPLAY").expect("retained script path");
    let (nodes, _) =
        crate::script_gen::parse_script(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(contract("拿铁 28元\n美式 22元\n燕麦拿铁 32元")
        .missing(&EditorState::new(), &nodes)
        .is_empty());
}
